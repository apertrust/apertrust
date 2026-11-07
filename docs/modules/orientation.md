# Orientation

Module `flip`, version 1. It works on the background only and has no Auto.

It is darktable's orientation in the first pass (decision 2), its page as darktable draws it: no tabs, sections or sliders, its transform label, a reset label of the stored orientation, with darktable's four buttons on the same line, rotate 90° anticlockwise, rotate 90° clockwise, flip horizontally and flip vertically (`gui_init`, `flip.c:596`), and under it darktable's guides widget, show guides with the global guide settings button at its end (`dt_guides_init_module_widget`, `src/gui/guides.c`). darktable's default group is base, so it is in the Base tab, after Exposure and before Demosaic, in darktable's order (decision 38).

It drives one operation, `flip` (camera RGB 16, [../ports/flip.md](../ports/flip.md)), which every photo runs, so it adds no operation. It owns its one parameter, darktable's stored orientation. Its default is darktable's, auto: the photo's own orientation from its EXIF data, so an untouched photo shows the right way up and the module sets nothing. A photo the right way up has no orientation in its plan at all (the port's Setup on the CPU).

**Status:** built, in `crates/modules/src/orientation.rs`. The app draws it in the Base tab from its definition (`docs/gui.md`, Module panels from definitions).

- Its rows are tests in `crates/modules/tests/orientation.rs` and the module's source; [Results](#results) gives what they measured.
- Every row is exact, so none has a proposed tolerance.
- Measures and test images are in [README.md](README.md).

## Controls

| Control | Parameter | Shown as | Default | Ported |
| --- | --- | --- | --- | --- |
| Transform (`orientation`) | `orientation` | a button row (`Row::Buttons`): the label, a double-click resetting it, and darktable's four buttons | Autodetect, darktable's `reload_defaults`, on every photo | every value |

**The stored orientation** is darktable's `dt_image_orientation_t` from -1 (`src/common/image.h`), its options the operation's choices of the same index: `auto` (autodetect), `none`, `flip_vertically`, `flip_horizontally`, `rotate_180`, `transpose`, `rotate_cw_90`, `rotate_ccw_90` and `transverse`, by darktable's `$DESCRIPTION`s. The binding writes it once off auto. darktable shows it only as the photo itself; so does the panel, with no list.

**darktable's buttons**, inline after the label, each an action (`Module::action`) that writes the orientation from the one shown, auto read as the photo's own, as darktable's do, a `SetControl` of the orientation, one change and one history step:

| Button | Icon | darktable | Writes |
| --- | --- | --- | --- |
| Rotate 90° anticlockwise (`rotate_ccw`) | darktable's circular arrow, anticlockwise | `do_rotate(self, 0)`, `[` | the columns shown flipped (FLIP_Y where the axes swap, else FLIP_X), then the axes swapped |
| Rotate 90° clockwise (`rotate_cw`) | its mirror image | `do_rotate(self, 1)`, `]` | the rows shown flipped, then the axes swapped |
| Flip horizontally (`flip_horizontally`) | two triangles across a vertical line | `_flip_h` | the columns shown flipped |
| Flip vertically (`flip_vertically`) | two triangles across a horizontal line | `_flip_v` | the rows shown flipped |

Each tooltip and screen-reader name is the button's full name, darktable's `rotate 90 degrees CCW` and `CW` and its flips in British English. darktable's buttons also tell crop to follow the turn (`_crop_callback`), which comes with crop.

**Show guides**, darktable's per-module toggle of the composition guides while the module has focus, kept in darktablerc and never in an edit, with the global guide settings button at its end, is drawn disabled, off, until the module's guides come: Apertrust's guides overlay (the central bar's Guides) is global, and the panel has no module focus yet.

**Commands.**

- The buttons send `SetControl { module: "flip", control: "orientation", value: … }`, the value their action computes; a script sends the same, and `apertrust-cli edit --set flip.orientation=rotate_cw_90` does too.
- `--param flip.orientation=…` is refused, since the module owns it.

**Presets:** darktable's auto (its auto-applied preset, the module's default), no rotation, flip horizontally, flip vertically, rotate by -90 degrees, rotate by 90 degrees and rotate by 180 degrees (`init_presets`), each one value, waiting for the header's presets menu. darktable's header shows "auto" after the module's name while the auto preset's orientation applies; Apertrust's section header shows no such label yet.

The words are darktable's label, button names and the guides widget's in British English, capitalised as the other modules' are; Ukrainian and Polish from darktable's `uk.po` and `pl.po` at the tag. The promise is darktable's module description, "flip or rotate image by step of 90 degrees"; the buttons' promises are Apertrust's drafts.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since every photo runs `flip`, and order, since the module has one control; then auto on a turned raw, the binding, layers, darktable's match and the buttons.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| F0 | Transform at its default | Autodetect, written or kept | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | The small Bayer raw and the sweep through the export and the viewport |
| F1 | Transform, moved and reset | Each of the seven turns, then reset by `ResetControl` and by `ResetModule`; No rotation | The frame | In the export, the untouched frame with darktable's buffer flip; in the viewport, not the untouched frame; once reset, the frame and its keys are no edit's; No rotation renders as the photo the right way up | Exact | The small Bayer raw and the sweep through the export and the viewport |
| F2 | Autodetect on a turned raw | The raw's EXIF orientation each of the seven turns | `bind`; the frame | `bind` returns nothing; the frame is the same raw the right way up with that orientation chosen | Exact | The small Bayer raw, through the export |
| F3 | The binding | Each of the nine options | `bind`'s settings | Autodetect writes nothing; every other option writes `flip.orientation` as the operation's choice of the same index, and nothing else | Exact | CPU, a colour raw's context and an image that is not a raw |
| F4 | On a layer | | The definition | Background only: never offered on a layer, as it turns the whole image | Exact | CPU |
| F5 | The darktable match | None | The definition's list; the operation's render | darktable's orientations in its enum's order; each orientation renders darktable's output bit for bit, the seven `flip-*` golden cases | Exact | CPU; mire1.cr2 |
| F6 | The buttons | Each of the four on each of the eight orientations, shown as itself and as auto on a photo whose EXIF gives it | `action`'s writes; the panel's change | darktable's `do_rotate`, `_flip_h` and `_flip_v` restated, one write of the orientation; four quarter turns or two mirrors come back; in the panel each button drawn and named, a click its action, a double-click on the label the orientation's reset | Exact | CPU; the panel in kittest |

F5's render is the port's: its golden cases are darktable-cli's renders of tests 0005 to 0007 and of test 0005 edited to the four other turns, each 0 ΔE2000. The whole-pipeline exit test runs auto on both sides, so the X100VI's raw, which its EXIF turns, is compared upright.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/orientation.rs` and the module's source; the rows run on CI's software GPUs too.

| Row | Result |
| --- | --- |
| F0 | Holds, exactly, through the export and the viewport |
| F1 | Holds, exactly, for all seven turns |
| F2 | Holds, exactly, for all seven turns |
| F3 | Holds for all nine options on both contexts |
| F4 | Holds |
| F5 | Holds: the options are darktable's nine in its order; every golden case 0 ΔE2000, mean and max |
| F6 | Holds for all four buttons on all eight orientations, as themselves and as auto; in the panel, the four buttons named and enabled, Rotate 90° clockwise reporting its action, which on auto writes rotate −90° (darktable's 5), and a double-click on Transform its reset |

## In the panel

- **No tabs:** the Transform row, its label and darktable's four buttons inline, then Show guides with the guide settings button at its end, drawn disabled.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

1. **Show guides** waits for the module's guides: an overlay while the module has focus, and the global guide settings the button opens.
2. **The presets** wait for the header's presets menu, and the header's "auto" label for a label in Apertrust's section header.
3. **Crop following a turn** (`_crop_callback`) comes with crop.

## Deliberate differences

None: the page is darktable's.

## Design choices

- **Control id:** `orientation`, darktable's params field and the operation's parameter; option ids the operation's choices, so the binding needs no table.
- **The default:** auto, darktable's, rather than the photo's orientation itself: the edit holds nothing until the user turns the photo, and the photo's own orientation stays the camera's.
- **The buttons:** a button row (`Row::Buttons`) and `Module::action`, the generic piece another module's buttons reuse: a pure function of the values shown and the photo, its writes sent as `SetControl`, so no new command and no stored state.
- **Rows:** F0 to F6; no first step, since every photo runs `flip`, and no order row, since there is one control.
- **Row ids:** F, for flip.
- **In `MODULES`:** after White balance, before Exposure.
