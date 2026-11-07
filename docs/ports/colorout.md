# Port spec: output colour profile (colorout)

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/colorout.c` (`commit_params()`, `process()` through `_transform_cmatrix_linear` and `_process_fastpath_apply_tonecurves`), with `src/common/colorspaces.c` (`_colorspaces_create_srgb_profile`, `_create_lcms_profile`). The reference is darktable's CPU path.

Status: ported.

- **Code:** the kernel is `slang/colorout.slang` in `crates/op/colorout`, its setup `src/lib.rs`.
- **Coverage:** sRGB, linear Rec2020 and linear Rec709 as output; other profiles and ICC files return `NotPorted`. It reports the pipe's output profile through `Operation::output_profile` (decision 26), sRGB's for a type without a matrix.
- **Checks:** unit tests, the engine's tests and the CLI's exports; a golden check against darktable's sRGB export is planned.
- **Module:** the Output colour profile module owns both parameters (`docs/modules/output-colour-profile.md`), so `--param` sets neither. Its Export profile writes `type`, an edit choosing only sRGB until the display path converts from the edit's profile (decision 26), and it fixes `intent` at perceptual.

## Purpose

Takes the finished image from the working space, linear Rec2020, to the output's colour space and encoding: sRGB for the CLI's PNGs. Its settings belong to the Output colour profile module, darktable's (decision 2).

## Pipeline

- **darktable position:** last but gamma (order 70.0), after the tone mapper and the display-referred modules.
- **Domain:** display-referred; working RGB in, output RGB out, encoded with the output's tone curve.
- **Layer support:** background only.

## Parameters

darktable params version 5: `type` (profile type, default sRGB), `filename`, `intent`. Apertrust leaves out the file name. Both belong to the Output colour profile module, which shows `type` as Export profile and fixes `intent`, shown by darktable only under a preference that is off by default. darktable's export overrides the type with the export's profile; the CLI uses colorout's.

**The pipe's output profile** (decision 26): colorout implements `Operation::output_profile(params)`, the matrices to and from XYZ D50 of its `type`, which the planner puts into the image context for every operation's `prepare`, as darktable's `commit_params` publishes the pipe's output profile (`dt_ioppr_set_pipe_output_profile_info`). A type without a matrix reports sRGB's, which darktable puts in place of a profile without one, Lab's included, so the pipe always has a profile; here that is every type `prepare` refuses as not ported. In an export with a profile of its own, that profile reaches colorout as its `type` for that export only, as darktable's export sets `p->type = pipe->icc_type`; the viewport and stills keep the edit's, never the display's, which darktable's darkroom uses (principle 2).

## Setup on the CPU

The matrix is XYZ D50 → output RGB after working RGB → XYZ D50, both from lcms profiles built as darktable builds them: the profile's primaries with `D65xyY`, Bradford-adapted to D50. sRGB and the working space share their white, so for them the adaptation cancels.

## Per pixel (`process`)

`out = curve(M · in)`: for sRGB, darktable's parametric curve (2.4, 1/1.055, 0.055/1.055, 1/12.92, 0.04045) inverted, `12.92 x` up to 0.0031308 and `1.055 x^(1/2.4) − 0.055` above; negative values clip to 0, as darktable's table does. Linear profiles have no curve.

## Deliberate differences

- **The curve itself, not darktable's table:** darktable samples the inverted curve in a table of 65536 and interpolates below 1, and above 1 extrapolates with an exponential fitted to the table's top; Apertrust evaluates the curve, which differs by about 1e-5 below 1 and slightly above it, where 8-bit output clips anyway.
- **No Lab before colorout.**
- **Only sRGB and linear Rec2020 and Rec709.**
- **No derivatives:** colorout has no continuous parameters.
- **The viewport's output profile is the edit's, never the display's** (decision 26, principle 2): darktable's darkroom publishes the display's profile as the pipe's, so filmic rgb and AgX's "export profile" map against the display there. Apertrust's viewport and stills keep the edit's profile, so the screen computes what the export does, and the display colour path converts from it.
- **The output profile's XYZ D50 to RGB matrix is inverted in f64:** `xyz_d50_to_rgb` is the f64 inverse of the profile's RGB to XYZ D50, rounded to f32, where darktable inverts the f32 matrix with `mat3SSEinv` (`_colorspaces_get_matrix_from_profile`), so the two can differ in their last bits.

## Test plan

- **Unit tests:** sRGB keeps white and has the curve; Rec2020's red lies outside sRGB; linear Rec2020 out is the identity; what is not ported; each ported type's `output_profile` matrices invert each other, both ways, within 4 f32 epsilons (the largest error 1.2e-7); a type without a matrix, Lab, reports sRGB's.
- **In the engine:** every `prepare` sees colorout's profile, in the export and viewport pipes, and a plan without colorout sees `None`; changing `type` changes it (`crates/engine/tests/pipeline.rs`); every part of a tiled export, before its tiles and in a first round, sees the whole export's (`crates/engine/tests/tiles.rs`).
- **Planned:** a golden check against darktable's export with an sRGB profile.
