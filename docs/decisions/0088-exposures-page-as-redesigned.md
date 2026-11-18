# 0088. Exposure's page as redesigned

## Context

Exposure is the tool most edits touch first. The first pass gives each tool darktable's page ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md), [0029](0029-a-tool-is-darktables-page.md)), and darktable's exposure page has a Mode list; Compensate camera exposure and Highlight preservation mode; Exposure with darktable's picker; Percentile, Target level and the computed exposure in automatic mode ([0087](0087-the-raws-histogram-counted-once-for-an-operation-that-reads-it.md)); Black level correction; and the Area exposure mapping group, whose area mode (correction or measure), input and target patches and Lightness say neither what a pick will do nor what it did. Under decisions 29 and 30 its two camera rows would be drawn disabled on photos where they can never act, and Auto ([0028](0028-tools-bound-to-operations.md)) would be a second automatic way beside automatic mode.

The wireframe's board for the Exposure tool designs a page that explains itself. Taking it departs from darktable's page, a deviation from the first pass, and from decisions 29 and 30's rule that a control that cannot act is drawn disabled, never hidden. What an edit stores and how a photo renders do not change, so the migration's comparisons are untouched.

## Decision

**Exposure's page is the board's, an exception to [0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)'s first pass and [0029](0029-a-tool-is-darktables-page.md)'s darktable page.** Its controls, their ids, ranges, defaults and bindings, its operation and its processing stay darktable's: no tool version, operation, edit or golden case changes. Built in six steps, the last its check against the board ([tools/exposure.md](../tools/exposure.md#the-page-as-redesigned)). The page, top to bottom:

- **Mode** is a segmented switch, Manual | Automatic, in place of darktable's list. On an image that is not a raw the photo lists Manual alone (`ProcessingTool::options`, [0030](0030-controls-as-darktables.md)), and the switch draws Automatic disabled, its tooltip "Automatic needs a raw photo's histogram"; Manual stays selectable. Holding Manual there, the session reports Mode unavailable, a choice whose photo lists only the option it holds ([0030](0030-controls-as-darktables.md)), and the switch says so option by option rather than dimming whole: Manual drawn chosen as anywhere, Automatic disabled with its reason. An edit pasted there holding Automatic shows Automatic chosen and disabled, as an option the photo does not list, until Manual is chosen; it renders as manual mode meanwhile (decision 87). This is the one segmented choice listed per photo, an exception to decision 30's "a list, never segments".
- **In manual mode:**
  - **Compensate camera exposure −⅔ EV** and **Highlight preservation mode +1.0 EV**, labels with the photo's value, each hidden where the photo gives it nothing to do (below); first, above Exposure, as darktable's stack has them.
  - **Exposure**, its picker on its quad. The picker's tooltip and its hint over the photo say what a pick will do, by the Picker target: "Make an area middle grey", "Make an area L 92 %", "Match an area to IMG_2041 (L 62 %)".
  - **Picker target**, a choice under Exposure, in place of Area exposure mapping: Middle grey, the default; Lightness, its slider under it, 0 to 100 %, its track darktable's ramp of greys; Reference, the reference row under it. Each option has a tooltip saying what it does.
  - **Last pick**, after a pick, a one-line readout: "L 41 % → L 50 % · +0.55 EV", the two patches before the values.
- **In automatic mode:** Percentile as **Measure at**, Target level as **Place it at**, each tooltip starting with darktable's name, and decision 87's computed value as **This photo**. Measure at shows darktable's two decimals, "50.00%", as a slider's value does ([0030](0030-controls-as-darktables.md)), the median named in its tooltip; no word follows a slider's value.
- **Black level correction**, in both modes, a plain slider with darktable's four decimals ([0030](0030-controls-as-darktables.md)), its tooltip "For lifting clipped blacks, not for deeper blacks: use AgX's black relative exposure".

**The panel explains nothing in text.** Explanations are tooltips and results are one-line readouts. Only two warnings show inline, one line each in the warning colour, above Last pick. They are independent: a pick of clipped pixels that also falls short shows both, the clipped line first, since clipping is why it reads short.

- "The area has clipped pixels", its tooltip "Its lightness reads low: pick an area with no clipped highlights";
- "Exposure at +18 EV: the area is L 12 %, not 50 %", its tooltip saying what limited the pick ("Exposure reached its end of +18 EV"), with "Black level was moved to keep black below white" added only where the pick moved the black.

**Gone from the page:** the Area exposure mapping group, its area mode and patches, and darktable's measure mode.

**Two pickers.**

- **`exposure`** always sets the exposure, toward the target's L: 50 for Middle grey, the Lightness setting, or the reference's. darktable's arithmetic and its black margin are unchanged (E50), so Middle grey picks exactly what Lightness at 50 picks. That arithmetic leaves the black aside, as darktable's does, so the area lands on its target only while the black the operation holds is 0; a colour raw's own black or any Black level correction moves it off by what the black moves it (an area of 0.05 toward 0.184 with a black of 0.01 lands 0.27 EV short), and Last pick shows where it lands.
- **`reference`**, on the Reference row: an area of the exposure operation's input that writes no control. It measures the area by measure mode's arithmetic and hands its L back in `Shown::settings`; the app keeps it with the open photo's path.
- **The reference** stays until it is set again, across photos, folders and restarts. Its row shows its patch, "L 62 % · IMG_2041" or "· this photo", the name dimmed once the file is gone (checked off the interface's thread as a photo opens and as the setting changes), "none" with none set, "measuring…" while picking, its picker, and ⋯ with Clear Reference and Set Reference from an Area.
- **With Reference chosen and none set,** Exposure's picker is disabled, "Set a reference first, with the picker on the Reference row", and the session refuses its `On`.
- **Clear Reference while that picker is on,** or Reference chosen while it is on with none set: the app sends the settings as `Settings`, the session finds the picker no longer ready, applies nothing and turns it off, as the picker that is on turns off when it cannot go on; its button is then drawn disabled, its tooltip saying why, and the next `On` is refused with `NotReady`.

**Settings, none in an edit** ([0043](0043-settings-and-apertrusts-folders.md)), each `owner = "tool:exposure"`:

- `tools.exposure.target`, a choice of `apertrust_tools::exposure::PickerTarget` (`middle_grey`, `lightness`, `reference`), `middle_grey` by default;
- `tools.exposure.lightness`, as it is;
- `tools.exposure.reference_lightness`, a number, 0 to 100 %, one decimal, optional, and `tools.exposure.reference_photo`, text, optional, the source photo's path: the reference, two settings set and cleared together, since the schema has no record kind;
- `tools.exposure.area_mode` removed; an old key in a settings file stays an unknown key.

**Exposure's two camera rows are hidden where the photo gives them nothing to do,** an exception to [0029](0029-a-tool-is-darktables-page.md) and [0030](0030-controls-as-darktables.md)'s "drawn disabled, never hidden" for these two rows alone. Every other control a photo cannot use stays drawn disabled, Temperature and Tint without the camera's matrix among them; another tool's row hidden this way is a decision of its own.

- **`ProcessingTool::visible(control, image) -> bool`**, provided as true, reads the photo alone, never the values, so a row never comes or goes as the person edits. Exposure hides Compensate camera exposure where the camera recorded no bias (`camera_bias(image) == 0`) and Highlight preservation mode where the camera held nothing back (`highlight_bias(image) == 0`). A hidden control keeps its value and its binding; on that photo it changes nothing (E31, E33).
- **darktable hides only one of the two.** At the reference release it shows Compensate camera exposure on every photo, "compensate camera exposure (+0.0 EV)" where there is no bias, and hides only Highlight preservation mode, where `_get_highlight_bias` is 0 (`exposure.c`, `gui_update`). Hiding Compensate camera exposure, like writing its bias in thirds below, is part of this exception, not darktable's behaviour.
- **`ProcessingTool::available` stays** for everything else: a control that the values or the state disable is drawn disabled.
- **The session** computes it as the photo opens and again where its decode's context replaces the metadata's, beside `options`, under `catch_unwind`, and reports it as `ControlState::visible`, true on a panic, with a warning. The renderer skips the row, and screen readers find no node for it.

**Labels with the photo's value: `ProcessingTool::label_value(control, image) -> Option<LabelValue>`,** `None` by default, computed as the photo opens and again where its decode's context replaces the metadata's and reported as `ControlState::label_value`, `None` on a panic.

- `LabelValue { value: f32, unit: Unit, format: ValueFormat }`, plain data. `ValueFormat::Thirds` writes the camera's bias as the camera does, "−⅔", "+1⅓", "0", with one decimal where it is not a third or a half ("+0.7"); `ValueFormat::Decimals(n)` writes n decimals, signed ("+1.0").
- Exposure gives the values its operation takes off and gives back: `camera_bias(image)` in thirds, `highlight_bias(image)` with one decimal. darktable writes both with one decimal, `%+.1f` and `%.1f` unsigned; the thirds and the sign on Highlight preservation mode's value are the board's, part of this exception.
- The app draws `tool-<tool>-control-<control>-label-value`, with `$value` in the language's number format, after the label in `text_secondary`, "Compensate camera exposure −⅔ EV" (`ToggleRow::value`). Screen readers hear the label and the value.

**A picker's words by a setting: `Picker::words_by: Option<&'static str>`,** the choice setting whose option chooses the picker's tooltip and its hint over the photo, `tool-<tool>-picker-<picker>-promise-<option>` and `-hint-<option>`, the button's name its promise, with `-no-photo` and `-this-photo` for an option that shows a reference whose photo is not known or is the open one; the viewer bar reads "Exposure · Make an area middle grey". `ProcessingDefinition::check` holds it to a choice `Row::Setting` of the tool.

**Whether a picker can start: `ProcessingTool::picker_ready(picker, settings) -> bool`,** true by default, from the tool's settings alone. Exposure's `exposure` is not ready while the target is `reference` and `reference_lightness` has no value. The app draws a picker not ready disabled, its `-unavailable` tooltip saying why; the session refuses its `On` (`SessionError::Refused`, reason `NotReady`), so a script cannot pick toward no reference.

**Rows.**

- **`Row::WhenSetting { setting, is, rows }`:** rows shown while a choice setting holds one of `is`, as `Row::When` does for a control; `ProcessingDefinition::check` holds `setting` to a choice `Row::Setting` of the tool.
- **`Row::Reference { picker, lightness, photo }`:** the reference row, naming its picker, the number setting holding the reference's L and the text setting holding its photo's path; it joins `Row::pickers()`, and a tool's settings, which a picker's `settings` must name, are its `Row::Setting`s and the settings a `Row::Reference` names.
- **`Row::LastPick { picker }`:** the last pick's readout of a picker that writes controls, and the warning line above it.
- **The rows with no control** are `Row::Setting`, `Row::Action`, `Row::Patches`, `Row::Computed` ([0087](0087-the-raws-histogram-counted-once-for-an-operation-that-reads-it.md)), `Row::Reference` and `Row::LastPick`. `Row::Patches` stays for Colour calibration's Area colour mapping; Exposure no longer uses it.

**What a pick shows.**

- `Shown` gains `last: Option<LastPick { before: f32, after: f32, exposure: f32, patches: [Patch; 2] }>`, the area's L as it rendered before the pick and as it renders after, and the change of Exposure in EV, and `warnings: PickWarnings { clipped: bool, short: Option<Short { exposure, reached, target, black }> }`, each warning on its own, both possible at once; `black` is whether the pick moved the black.
- `Reading` gains `input_maximum: [f32; 3]`, what each channel of the operation's input reaches where the sensor saturated (`ImageDesc::maximum`), which the session fills in.
- Exposure's pick takes before from measure mode's arithmetic at the current exposure and after at the exposure written, the black the operation holds included; `short` where Exposure is held at ±18 EV and the area falls short of its target; `clipped` where some channel's maximum over the area reaches 0.99 × that channel's `input_maximum`.
- The app keeps each tool's `last` and `warnings` as it keeps its patch, cleared by the same commands. For a picker with a `Row::LastPick`, the live region says Last pick's words once, in place of `picker-said-control`.

**Auto is dropped from Exposure** (`auto: false`): automatic mode is its one automatic way. The Photo menu's Auto lists Rotate and perspective alone; Exposure's Auto row, E51, and its message `tool-exposure-auto` go. Decision 28's Auto, a definition's flag and a value's mark, stays for the tools that have it.

**Words and access.** British English is the source; American English, Ukrainian and Polish follow in the same change. Each picker is a toggle button named by its action; Esc ends a pick; Picker target is a choice row, a combo box to screen readers; Last pick is a polite live region.

**The design system** gains, each with gallery snapshots: `ToggleRow::value`; a `Segmented` option drawn disabled with its reason as the tooltip, and, while the photo lists only the option it holds, that option drawn as anywhere and the others disabled with their reasons; a tooltip per option in `Choice`'s list, the closed row's the chosen option's; `ReadoutRow::patches`, one patch before each value; a one-line warning row with the warning sign; and `ReferenceRow`.

**Tests.** E0, E3 and E14 unchanged prove the render did not change. New rows, their tolerances proposed: E52, Middle grey equals Lightness at 50, exact; E53, Reference matches across two photos with biases 0 and −0.67 EV, the area at the reference's L within 0.01 EV with Black level correction at 0 on both, as E50 holds; E54, the reference picker writes nothing, exact. E51 goes.

## Options considered

- **darktable's page as it is:** the area mode's two meanings for one button and the input and target patches say neither what a pick will do nor what it did; measure mode is the reference row's picker under another name.
- **The redesign after the migration:** it changes no stored value and no render, so nothing the migration compares waits for it, and Exposure is the tool most edits start with.
- **Picker target beside Area exposure mapping:** two places that choose one target.
- **The reference in the edit:** it would be lost on the next photo, the use it exists for, and would put another photo's measure into this one's edit.
- **One record setting for the reference:** the schema has no record kind; two settings set and cleared together need nothing new.
- **Rows the photo rules out drawn disabled,** as decisions 29 and 30 have them: a row that can never act on this photo invites a click that does nothing; darktable already hides Highlight preservation mode there.
- **A declarative condition on the image context** (`Row::WhenImage`): decision 30 rejected a condition language for availability, and the bounds that make a bias zero are the tool's code (`camera_bias`, `highlight_bias`).
- **Visibility from the values too:** rows that come and go as the person edits; `Row::When` already covers modes.
- **The label's value in each control's label message:** a message per photo value cannot be checked against the definition; one id with `$value` serves every tool and lets each language place it.
- **Last pick worked out by the app:** every client would need the tool's arithmetic; the pure `pick` gives it, tested without a GPU.
- **Clipped at a fixed 1.0:** the input's maximum differs by photo and by the raw stage's settings; the operation's input's own maximum, channel by channel, is the bound, since white balance lifts each channel's differently.
- **Readiness enforced by the app alone:** a script could pick toward no reference.
- **The whole Mode switch dimmed on an image that is not a raw:** an edit pasted there holding Automatic could not be set back to Manual.
- **One warning per pick** (`Option<PickWarning>`): a clipped area that also falls short would show only one of the two, and clipping is often why it falls short.
- **The pick corrected for Black level correction:** departs from darktable's arithmetic, which every pick has followed (E50); Last pick shows where the area lands instead.
- **Auto beside automatic mode:** two automatic ways that answer differently, and Auto's statistic is still unbuilt.

## Consequences

- **The docs** describe the redesign as built: [tools/exposure.md](../tools/exposure.md#the-page-as-redesigned) holds the page, its steps and its rows; [gui.md](../gui.md) the rows, the components and the Auto list without Exposure; [pickers.md](../pickers.md) the two pickers, the settings, `Shown`, `Reading` and `picker_ready`; [tools/README.md](../tools/README.md) rule 5's exception; [ports/exposure.md](../ports/exposure.md) closes its open question on hiding and labels; [engine.md](../engine.md) and [events.md](../events.md) give `ControlState::visible` and `label_value` and the refusal `NotReady`; [crates.md](../crates.md) lists the planned code; [roadmap.md](../roadmap.md) lists the six steps in Phase 2. Decisions 2, 29, 30 and 79 point here, and AGENTS.md's first-pass rule names the exception.
- **The code**, built: `ProcessingTool::visible` and `label_value` (`apertrust-tools`), `ControlState::visible` and `label_value` (the engine), `ToggleRow::value`, after the label with no brackets (`apertrust-ui`), the renderer skipping hidden rows, Exposure's `auto: false` and options by photo, and `Segmented`'s disabled options, `enabled` and `promise`; `Row::WhenSetting`, `Picker::words_by`, `PickerTarget`, `AreaMode` and the mapping removed (`apertrust-tools`), `Choice::option_tips` (`apertrust-ui`), the settings schema's `tools.exposure.target` and the four locales' words for it; `ProcessingTool::picker_ready`, `Row::Reference` and the `reference` picker (`apertrust-tools`), `NotReady` (the engine), `ReferenceRow` (`apertrust-ui`), the app's reference row, `ToolSettings` with optional and text settings and the photo-gone check, and the schema's two reference settings. Built too: `Row::LastPick`, `Shown::last` and `warnings` with `Short::black` (`apertrust-tools`), `Reading::input_maximum` (the engine), `ReadoutRow::text` and `patches` and `WarningRow` (`apertrust-ui`), the app's Last pick and its words in the live region, and the four locales' words for them.
- **Unchanged:** the edit, the tool's version, the operation and every golden case; Colour calibration's area mapping keeps `Row::Patches`.
- **Every other tool** keeps darktable's page until the migration ends; another redesign before then is a decision of its own.
