# Operation order

Every built operation's slot, in pipeline order: the stage it reads and its number (decisions 21 and 22), whether an unedited photo runs it, its place in darktable's v5.0 order (`src/common/iop_order.c`, `v50_order`), and the module that drives it, of the first pass's modules, darktable's (decision 2, [modules/README.md](../../docs/modules/README.md)). The rules for choosing a slot are in [engine.md, Order](../../docs/engine.md#order).

`crates/operations/tests/order.rs` reads this table and holds every built-in operation's manifest to it, so it changes with the manifests. It lives beside the test, in the crate, so that nothing under `docs/` is read by code and every edit to the documentation skips the code's checks (decision 70); an edit here runs them.

| Operation | Slot | On an unedited photo | darktable, v5.0 | Driven by |
| --- | --- | --- | --- | --- |
| rawprepare | raw 1 | yes | 1 | Raw black/white point |
| temperature | raw 3 | yes | 3 | White balance, darktable's white balance module (decision 27) |
| highlights | raw 4 | yes | 4 | Highlight reconstruction |
| demosaic | raw 8 | yes | 8 | Demosaic |
| hazeremoval | camera RGB 14 | no | 14.0, after lens correction and chromatic aberrations, before rotate and perspective | Haze removal |
| flip (orientation) | camera RGB 16 | yes | 16; at no rotation it runs no pass and leaves the plan, as darktable disables its piece | Orientation |
| exposure | camera RGB 21 | yes | 21 | Exposure |
| toneequal | camera RGB 24 | no | 24.0, before crop and colorin | Tone equalizer |
| colorin | camera RGB 28 | yes | 28 | Input colour profile |
| channelmixerrgb (colour calibration) | working space 28.5 | yes | 28.5, first of its group | Colour calibration, its CAT tab included (decision 27) |
| primaries | working space 28.9 | no, as in darktable | 28.5, last of its group | RGB primaries |
| colorequal | working space 41.2 | no | 41.2, after colorbalance, before color balance rgb | Colour equalizer |
| colorbalancergb | working space 41.5 | no, as in darktable (decision 32) | 41.5 | Colour balance RGB |
| rgbcurve | working space 42 | no | 42.0, before rgb levels and the tone mappers | RGB curve, one instance |
| rgblevels | working space 43 | no | 43.0, after rgb curve, before the tone mappers | RGB levels |
| sigmoid | working space 45.3 | no | 45.3, before agx | Sigmoid, once an edit can choose it as its tone mapper (decision 33) |
| agx | working space 45.5 | yes | 45.5 | AgX, while it is the edit's tone mapper (decision 33) |
| filmicrgb | working space 46 | no | 46.0, after agx | Filmic RGB, once an edit can choose it as its tone mapper (decision 33) |
| bilat (local contrast) | display 54 | no | 54 | Local contrast |
| bloom | display 61 | no | 61 | Bloom |
| grain | display 65 | no | 65 | Grain |
| soften | display 66 | no | 66 | Soften |
| vignette | display 68 | no | 68 | Vignetting |
| colorout | display 70 | yes | 70 | Output colour profile |
