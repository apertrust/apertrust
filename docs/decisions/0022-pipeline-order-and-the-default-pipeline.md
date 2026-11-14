# 0022. Pipeline order and the default pipeline

## Context

In what order do operations run, and which run on a photo nobody has edited? darktable lets users reorder modules and has several order lists; Apertrust keeps one fixed order ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)) and mirrors darktable's processing in the first pass ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), so that an unedited photo renders as darktable renders it.

## Decision

**darktable's v5.0 order throughout** (`src/common/iop_order.c`, `v50_order`). Every operation keeps darktable's place: the tone equalizer at camera RGB 24.0, after exposure and before input colour; RGB curve at 42 and RGB levels at 43, before the tone mapper; grain and vignette at 65 and 68, after it.

**Slot rules:**

- An operation keeps darktable's v5.0 number where it keeps darktable's place. darktable's place is its position in the list, which is its real order; where a number disagrees with the list (lut3d's 36, listed between 46.0 and 47.0), the operation takes a number between its list neighbours.
- No two operations share a slot, and the numbers rise with the stages, so the number alone gives the order.
- darktable's other operations at 28.5 take numbers after colour calibration in darktable's list order: RGB primaries is 28.9, the rest between 28.5 and 28.9.
- `crates/operations/tests/order.rs` holds every built-in operation to the table in [crates/operations/order.md](../../crates/operations/order.md) and `KERNELS` to the same order. A third-party plugin whose slot breaks a rule is refused at load.

**The default pipeline:** an unedited colour raw runs nine operations, each a manifest with `default_enabled`:

| Operation | Slot | Default mode |
| --- | --- | --- |
| rawprepare | raw 1 | the raw's own crop and levels |
| temperature | raw 3 | "as shot to reference" where the camera has a matrix, else "as shot" |
| highlights | raw 4 | inpaint opposed |
| demosaic | raw 8 | RCD on Bayer, Markesteijn 1-pass on X-Trans |
| exposure | camera RGB 21 | +0.7 EV, the camera's bias compensated |
| colorin | camera RGB 28 | the standard input matrix |
| channelmixerrgb (colour calibration) | working space 28.5 | As shot, CAT16 ([0027](0027-both-of-darktables-white-balances.md)) |
| agx | working space 45.5 | darktable's scene-referred default |
| colorout | display 70 | sRGB |

Every other operation runs only while a module sets it ([0028](0028-tools-bound-to-operations.md), [0032](0032-tools-off-until-first-touched.md)); colour balance rgb, the tone equalizer and the rest are off on a new photo, as in darktable. A test holds the enabled set to exactly these nine.

**The chain is linear** until layers make it a graph ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)). Per-pixel runs fuse between spatial and whole-image passes ([0008](0008-a-gpu-resident-fp32-pipeline-on-wgpu.md)); at darktable's place the tone equalizer's mask passes make one break, after exposure.

## Options considered

- The tone equalizer after colour calibration, so its mask sees the same space on every camera: a departure from darktable's render, and a White balance drag would rerun its mask.
- RGB curve and RGB levels after the tone mapper: the same settings would render unlike darktable's.
- darktable's numbers kept under a later stage: the planner would order them, but the number would misstate the order.
- User reordering, as darktable allows: excluded by the principles ([0001](0001-what-apertrust-is-the-principles.md)).
- Colour balance rgb on every photo: a pass on every photo and a gamut mapping darktable's unedited render lacks.

## Consequences

- The tone equalizer's golden cases use darktable's own order, input colour an identity.
- An image that is not a colour raw (a monochrome raw, later a JPEG or TIFF), for which colour calibration has no identity, needs the default set to depend on the image; settled when Apertrust first opens one.
