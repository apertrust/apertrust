# 0026. Colour profiles and colour management

## Context

How does Apertrust turn camera RGB into its working space and the working space into an output profile exactly as darktable does, which profile do operations that gamut-map against the output see, and how does the picture reach a display that may be wide-gamut or HDR? darktable builds its profiles with lcms2, a C library ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).

## Decision

**colorin** builds darktable's camera profile as darktable does: the inverse of the camera matrix ([0024](0024-the-cameras-data-in-the-image-context.md)) reduced to its primaries' chromaticities with darktable's D65 as white, and the linear Rec2020 working profile, each as lcms builds it, Bradford-adapted to D50, in `apertrust-color`. The two D65s differ in the fifth decimal, so the adaptations are reproduced, not cancelled. One matrix takes camera RGB to the working space, after temperature's late correction (the camera's D65 multipliers over its as-shot ones) when temperature asks for it.

**colorout** takes the working space to the output profile the same way (sRGB by default; linear Rec2020 and linear Rec709 ported), then applies sRGB's curve itself, where darktable interpolates a table. Neither has continuous parameters, so neither exports derivatives.

**The pipe's output profile is in the image context** as matrices: `ImageContext::output_profile`, an `OutputProfile` of `rgb_to_xyz_d50` and `xyz_d50_to_rgb`, f32 by rows, as darktable's `matrix_in` and `matrix_out`.

- colorout reports it through `Operation::output_profile(params)`, which the planner collects from every instance before any `prepare`; the last that answers gives the pipe's profile.
- A plan without colorout gets `None`, the working profile. A profile without a matrix gives sRGB's matrices, as darktable puts sRGB's in place of one.
- Operations that gamut-map against the output (filmic rgb's v6 and v7, AgX's "export profile") read it; neither `defaults(image)` nor a module's binding does.

**Which profile each pipe uses:**

- **An export:** the edit's profile, or the export's chosen one, which overrides colorout's `type` for that export only, never the edit (planned with the export dialog's profile and the CLI's). AgX's "export profile" then reads the edit's own profile, as darktable's does.
- **The viewport and stills:** the edit's output profile, never the display's, so the screen soft-proofs the export and the viewport computes what the export does. The display path converts colorout's output to the display. Until it converts from any profile, the Output colour profile module lets an edit choose only sRGB.

**Display colour, in two tiers behind one abstraction:**

1. **App-managed**, the baseline everywhere, X11 included: the engine converts to the display's ICC profile with moxcms and outputs 8 or 10 bits.
2. **System-managed** where the OS supports it (Phase 7): linear float16 tagged with its colour space, the compositor mapping it to the display, the only route to HDR and EDR headroom. FP16 scRGB on Windows; `CAMetalLayer` with rgba16Float in extended linear Display P3 on macOS; the Wayland colour-management protocol where the compositor supports it.

`apertrust-display`, an FFI crate, reaches the native layer where wgpu does not: today `tag_window` tags the Mac's Metal layer sRGB or Display P3 every frame, since wgpu leaves it untagged and macOS would show it unmanaged. Interface colours are authored in sRGB and converted when the surface is extended-linear.

**Colour management is moxcms**, pure Rust, validated against lcms2 in CI on a corpus of profiles; lcms2 is never compiled in or shipped.

**Golden cases with an output profile:** darktable-cli renders with `--icc-type` set to it, the plan appends colorout of that type, and both sides are decoded from that profile to linear Rec2020 for ΔE2000.

## Options considered

- Lab between colorin and colorout, as darktable has it: only rounds.
- A colour management library at run time for two matrices and a curve.
- lcms2 as the default or a shipped fallback.
- The engine reading colorout's parameters by id, or the client choosing the profile: either can disagree with colorout.
- The display profile in the viewport, as darktable's darkroom has it: differs between displays and from the export.
- The working profile everywhere: up to 10 ΔE2000 from darktable's sRGB export with filmic's gamut mapping.
- A single display path: no HDR, or no X11.

## Consequences

- The raw stage and colorin match darktable to a mean under 4e-7 on the raws whose decoders and matrices agree.
- Before an edit can choose another output profile, the display path converts from the edit's profile.
- An export writes darktable's sRGB profile as lcms2 saves it ([0056](0056-export-as-an-engine-job.md)).
