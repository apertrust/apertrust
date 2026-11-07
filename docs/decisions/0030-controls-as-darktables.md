# 0030. Controls as darktable's

## Context

A first-pass module shows darktable's controls ([0029](0029-a-module-is-darktables-page.md)). How far does each control follow darktable: its range and steps, whether it can act on this photo, which options a choice lists, and what happens when another control moves its range? A definition is static data, while darktable's widgets change with the photo and the module's values.

## Decision

**Every slider is darktable's, as its `gui_init` sets it:** the control's range is darktable's hard range in the units its slider shows (the params' `$MIN` and `$MAX`, times its factor, plus its offset); the row's track spans darktable's soft range (`Slider::soft`); its digits are the row's decimals and its step is darktable's. Each module's page records them, and `testing::slider_differences`, run per module, finds no difference. Widening a range raises no module version; narrowing one does.

**Exposure is darktable's slider:** the control is `exposure.exposure`, its default the photo's (+0.7 EV on a colour raw, 0 otherwise), over darktable's −18 to +18 EV with its track over −3 to +4 EV. Black level correction is an offset over the photo's own black, within darktable's ±1, its track over −0.1 to +0.1; a black that reaches the white the photo renders is put 0.01 below it, the margin darktable's `gui_changed` restores, so the kernel never divides by zero, and no control writes another.

**Availability: `Module::available(control, values, image)`**, provided as true, recomputed by the session after every change from the module's own values and the photo, and reported with each control (`ControlState::available`). A control that can change nothing on this photo (Use camera bias where no bias is recorded, Temperature and Tint without the camera's matrix) or that another control's value disables (vignetting's width/height ratio while automatic ratio is on; colour calibration's adaptation while the illuminant is as shot in camera) is drawn disabled and keeps its value; a pasted edit or a script never fails on it. Visibility stays data (`Row::When`); availability is the module's code.

**A choice lists darktable's options for the photo: `Module::options(control, image)`**, by default every option, gives a subset of the definition's, in its order (demosaic: Bayer's nine methods, X-Trans's seven, Bayer4's three, monochrome's one).

- The definition holds every option darktable lists on some photo, each with a permanent id; a stored choice indexes the definition's list, never the photo's, so an edit means the same on every photo. Options darktable lists on no photo are left out.
- The session computes the list once as the photo opens, under `catch_unwind`, and reports it (`ControlState::options`); a panic or an inconsistent list gives every option, with a warning. Another option is refused (`EditError::NotListed`), in the session and the CLI alike.
- An option the edit holds but the photo does not list (from another photo) is kept, shown last, chosen and disabled, "Not available for this photo", and rendered as darktable's darkroom renders it, through the operation's fallback or the binding's correction.
- A choice whose photo lists only the option it holds is unavailable. A choice listed per photo is a list, never segments.

**A range may follow other controls: `Module::range(control, values)`** gives a number's range, track and step for the module's values, by default the definition's (AgX's pivot over black to white relative exposure, its step by darktable's `dt_bauhaus_slider_get_step`).

- It reads only the module's values, never its own control's or the photo's. The session reports it (`ControlState::range`), recomputed after every change under `catch_unwind`.
- A command outside the current range is refused (`OutOfRange`). After every change a value its moved range leaves outside is clamped to the nearer end and stored in the same change, as darktable's bauhaus does; a moved soft range keeps the value and the track widens to show it.
- `check::clamp` is the one clamp the edit and the app share. Reading a saved edit clamps a value outside its range with a warning (`Edit::clamp_read`).

**A failing proposed bound** of a module's invariant table stays unchanged in an `#[ignore]`d test, named and explained, until the maintainer decides; CI checks the parts that hold.

## Options considered

- Narrower first-pass ranges, or a control's unit changed into exact stops: departs from darktable's slider and render.
- darktable's coupling of Exposure and Black level, one writing the other: the result depends on the order controls were set.
- The renderer special-casing a module's photo facts, or a declarative condition language: client code knowing a module's rules, or a language that cannot state darktable's.
- Every option listed, those the photo lacks disabled: 16 of demosaic's 17 disabled on a Bayer raw.
- Rewriting a stored option the photo lacks, or dropping it: a write the user did not make.
- Darktable's own storage of the pivot as a fraction of the range: moves its exposure with the range.
- Loosening a bound to what was measured: decides a tolerance without the maintainer.

## Consequences

- Exposure is module version 2: an edit's version 1 Exposure, which no release wrote, is left out with a warning.
- `check::sweep` holds every range finite, inside the definition's, with the default inside and one clamp pass enough.
- Open: a disabled control's tooltip saying why it is disabled.
