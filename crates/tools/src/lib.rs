// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Apertrust's tools (architecture decisions 4 and 28): what the user edits with. A
//! tool is one editing task, as the tools README lists them (`docs/tools/README.md`),
//! with its controls, its tabs and maybe Auto; an operation is one of the engine's
//! algorithms. A tool implements [`ProcessingTool`]:
//!
//! - its [`ProcessingDefinition`], static data: the controls an edit stores apart from the
//!   rows a panel draws them in, and no text, since every word is a message id that follows
//!   from the ids ([`messages`]);
//! - its defaults for a photo, the definition's unless they come from the photo, and
//!   whether each control can change anything on it while the tool's values stand;
//! - the operation parameters it owns, each owned by one tool only ([`check`]);
//! - its binding: a pure function from its own values, the photo's image context and
//!   [`Base`], each operation's defaults for the photo, to the parameters it sets,
//!   through transfer functions that carry their derivatives ([`transfer`]);
//! - its colour pickers, and what each makes of a reading of the photo ([`pick`]);
//! - migrations from the values an earlier version saved.
//!
//! The engine's session runs the bindings: every client sends tool values, and the
//! session turns them into operation parameters, so the app, the CLI and MCP see the same
//! tools. This crate depends on the operation interface, the shared colour maths and
//! the settings part of `apertrust-base`, for its settings' statics ([`settings`]),
//! only, never on egui, wgpu, the engine or an operation crate; it names operations and
//! parameters by id.

// The settings' generated statics name their choices' types by this crate's name.
extern crate self as apertrust_tools;

pub mod about;
pub mod agx;
pub mod check;
pub mod colour_balance_rgb;
pub mod colour_calibration;
pub mod colour_equalizer;
mod definition;
pub mod demosaic;
pub mod exposure;
pub mod grain;
pub mod haze_removal;
pub mod highlight_reconstruction;
pub mod input_colour_profile;
pub mod messages;
pub mod orientation;
pub mod output_colour_profile;
pub mod pick;
pub mod raw_black_white_point;
pub mod rgb_primaries;
pub mod settings;
pub mod soften;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tone_curve;
pub mod tone_equalizer;
pub mod transfer;
pub mod utility;
mod values;
pub mod vignetting;
pub mod white_balance;

use std::collections::BTreeMap;

use apertrust_base::wire::Id;
use apertrust_plugin::{ImageContext, Operation, ParamValues, Value};
use serde::{Deserialize, Serialize};

pub use definition::{
    ActionEnd, ButtonIcon, ChoiceStyle, Computed, Control, ControlError, ControlKind, Coordinate,
    CursorDef, CursorGroup, CursorGroupDef, Cut, DefinitionError, End, Face, Fact, FactEnd, Field,
    FieldRow, Fields, Gesture, Group, GroupKind, LineStart, MAX_POINTS, MIN_GAP, MOST_IN_SUBMENU,
    MOST_ON_TOOLBAR, MenuEntry, MenuItem, Picker, PickerKind, Place, PresetRow,
    ProcessingDefinition, Published, Range, ReadoutDef, Reset, Row, Section, Separator, SettingRow,
    SettingWords, Slider, Space, Surface, Tab, ToolCursor, ToolDefinition, ToolbarItem, Track,
    Unit, View, Widget, Word, check_menus, check_section, check_toolbar, slider_step,
};
use pick::{PickKeys, Picked, Reading, Readout};
pub use values::{ControlValue, Controls, Mark, PublishedValues, ToolValues};

/// The built-in tools, in the order the session reports them and an edit saves them:
/// Raw black/white point, Highlight reconstruction, Demosaic, AgX and Output colour
/// profile, in the Profile tab's order, Input colour profile among them after Demosaic;
/// then White balance, Orientation, Exposure, Colour calibration, Tone equalizer, Colour
/// balance RGB, Tone curve, which becomes RGB curve, Haze removal, then RGB primaries,
/// Grain, Soften and Vignetting, as the Tools panel's Profile and Adjust tabs listed them
/// in earlier builds; the order the Tools panel shows them in is the app's (decision 38).
/// The first pass's other tools follow (`docs/gui.md`, Build order).
pub const TOOLS: &[&dyn ProcessingTool] = &[
    &raw_black_white_point::RawBlackWhitePoint,
    &highlight_reconstruction::HighlightReconstruction,
    &demosaic::Demosaic,
    &input_colour_profile::InputColourProfile,
    &agx::Agx,
    &output_colour_profile::OutputColourProfile,
    &white_balance::WhiteBalance,
    &orientation::Orientation,
    &exposure::Exposure,
    &colour_calibration::ColourCalibration,
    &tone_equalizer::ToneEqualizer,
    &colour_balance_rgb::ColourBalanceRgb,
    &colour_equalizer::ColourEqualizer,
    &tone_curve::ToneCurve,
    &haze_removal::HazeRemoval,
    &rgb_primaries::RgbPrimaries,
    &grain::Grain,
    &soften::Soften,
    &vignetting::Vignetting,
];

/// The ids of the tools Apertrust has retired (decision 43): an older build wrote
/// their values, so an edit holding one loses them with a warning and still opens for
/// editing, where a tool neither built in nor retired is a newer Apertrust's and opens
/// the photo for viewing only. A tool removed or renamed joins the list, as the Tone
/// curve's `tone_curve` will once RGB curve replaces it (decision 2), and leaves it when
/// a tool takes its id again. Empty until then.
pub const RETIRED: &[&str] = &[];

/// What a hue curve graph paints behind its curve ([`ProcessingTool::curve_paint`]):
/// colours that are data, encoded sRGB, each channel from 0 to 1.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePaint {
    /// The strip under the graph, evenly spaced from its start to its end.
    pub strip: Vec<[f32; 3]>,
    /// The background, a grid's corners row by row from the top, `columns` a row, evenly
    /// spaced from the graph's start to its end and from its top to its foot.
    pub background: Vec<[f32; 3]>,
    pub columns: usize,
}

/// The contract between the engine's session and a tool (decision 28).
pub trait ProcessingTool: Send + Sync {
    fn definition(&self) -> &'static ProcessingDefinition;

    /// Each control's untouched value for the photo: the definition's, unless it comes
    /// from the photo, as White balance's as-shot temperature will.
    fn defaults(&self, _image: &ImageContext) -> Controls {
        self.definition().defaults()
    }

    /// Every operation parameter the tool owns (decision 4). No other tool owns
    /// one of them, and nothing else sets them.
    fn owns(&self) -> &'static [Owned];

    /// The parameters of [`Self::owns`] no control sets, which the tool holds at the
    /// operation's value for the photo, as Raw black/white point holds the raw's crop
    /// margins and Colour calibration darktable's version 3 (rule 12 of
    /// `docs/tools/README.md`): a client asking to set one is told the tool fixes it,
    /// not that it sets it. None, as provided.
    fn fixed(&self) -> &'static [Owned] {
        &[]
    }

    /// The operation parameters these values set: a pure function of the tool's own
    /// values, the photo's image context at full resolution and `base`, never of another
    /// tool, a pipe or any state, so the viewport and the export get the same
    /// parameters and the order controls were used in cannot matter.
    ///
    /// `values` holds only the controls off their defaults for the photo, and the session
    /// binds no tool that has none, so a tool at its defaults sets nothing and leaves
    /// the plan exactly as it was. It sets only parameters it owns, each within its
    /// schema: an operation an unedited photo does not run is planned while some binding
    /// sets one of its parameters, so a binding that needs one at its defaults sets one
    /// of them to its default explicitly. A control relative to the photo's default is
    /// written over `base`.
    fn bind(&self, values: &ToolValues, image: &ImageContext, base: &Base) -> Vec<Setting>;

    /// Whether the control can change anything on this photo while the tool's controls
    /// hold `values`, every control, the edit's over the defaults: true unless the photo
    /// lacks what it acts on, as Exposure's Use camera bias does on a photo that records
    /// no bias (decision 30), or another of the tool's controls holds a value under
    /// which it acts on nothing, as darktable makes vignetting's width/height ratio
    /// insensitive while automatic ratio is on (decision 30). A panel draws a control
    /// that cannot as disabled, as an action that cannot run is; it still takes a value
    /// and keeps it, so a pasted edit or a script never fails on it. Pure, as
    /// [`Self::bind`] is: the session asks again after each change, and the app on the
    /// values it shows before the session reports them, where they move its answer, on the
    /// image context the session's `Photo` carries (decision 27). The session also asks
    /// it with a lossless view's coordinate's id, as White balance's temperature and tint,
    /// which can change nothing without the camera's colour matrix; the coordinate is
    /// available where this holds and a control it moves is.
    fn available(&self, _control: &str, _values: &Controls, _image: &ImageContext) -> bool {
        true
    }

    /// Whether the row of `control` shows on this photo at all (decision 88): true, as
    /// provided. Pure, and reading only the photo's metadata, never the tool's values,
    /// so a row never comes or goes as the person edits: the session asks once as the
    /// photo opens and reports it. A hidden control keeps its value and its binding.
    /// Exposure alone overrides it, hiding its two camera rows where the photo gives them
    /// nothing to do; every other control a photo cannot use is drawn disabled
    /// ([`Self::available`]), and hiding another is a decision of its own.
    fn visible(&self, _control: &str, _image: &ImageContext) -> bool {
        true
    }

    /// A value of the photo's that the row of `control` shows after its label, as
    /// Exposure's "Compensate camera exposure −⅔ EV" (decision 88): `None`, as provided.
    /// Pure, and reading only the photo, as [`Self::visible`] is: the session asks once
    /// as the photo opens and reports it, and a client writes it through
    /// `tool-<tool>-control-<control>-label-value` ([`messages::label_value`]) where
    /// the row shows.
    fn label_value(&self, _control: &str, _image: &ImageContext) -> Option<LabelValue> {
        None
    }

    /// The options the choice `control` lists on this photo, by id, in the definition's
    /// order (decision 30): darktable's list for the photo, as demosaic lists its
    /// sensor's methods and colour calibration lists as shot in camera only on a colour
    /// raw. Never empty, and holding the photo's default ([`check::listed`]). Pure, and
    /// reading only the photo, never the tool's values: the session asks once as the
    /// photo opens, reports the list and refuses any other option, which an edit from
    /// another photo may still hold. Provided: every option; none for a control that is
    /// no choice.
    fn options(&self, control: &str, _image: &ImageContext) -> Vec<&'static str> {
        match self
            .definition()
            .control(control)
            .map(|control| control.kind)
        {
            Some(ControlKind::Choice { options, .. }) => options.to_vec(),
            _ => Vec::new(),
        }
    }

    /// The range of the number control `control` while the tool's controls hold
    /// `values`, every control, the edit's over the defaults (decision 30): what a value
    /// may hold, what the track spans at rest and what a key moves, as darktable's slider
    /// has them for those values. The definition's, as provided
    /// ([`ProcessingDefinition::range`]); `None` for a control that is no number. Override
    /// it where darktable's `gui_changed` moves a slider's range with other controls, as
    /// AgX's pivot spans black to white relative exposure: the definition's range is then
    /// the widest any values give, and its soft range and step those at the tool's
    /// defaults. Pure, as [`Self::bind`] is, reading the tool's values but never
    /// `control`'s own, nor the photo. The session reports it, refuses a command outside
    /// it, and after every change clamps a number a moved range leaves outside, written at
    /// its nearer end in the same change ([`check::clamp`]).
    fn range(&self, control: &str, _values: &Controls) -> Option<Range> {
        self.definition().range(control)
    }

    /// The curve a curve row draws through the points of `control`, or a band graph
    /// through its bands, named by its first, while the tool's controls hold `values`:
    /// its output at an input, both on the row's axes, so the curve drawn is the curve the
    /// operation renders. `None`, as provided, joins the points with straight lines; a
    /// band graph draws them in the warning colour, as no curve fits its bands.
    fn curve(&self, _control: &str, _values: &Controls) -> Option<Box<dyn Fn(f32) -> f32>> {
        None
    }

    /// The width σ, in its input's units, of the Gaussians the curve [`Self::curve`] gives
    /// for `control` is a sum of, while the tool's controls hold `values`: how far a drag
    /// on a band graph reaches, as darktable's tone equalizer moves the bands around the
    /// pointer by a Gaussian its curve smoothing widens. `None`, as provided, for a curve
    /// of no such width, where a band graph takes darktable's default, √2.
    fn curve_width(&self, _control: &str, _values: &Controls) -> Option<f32> {
        None
    }

    /// Where a hue curve graph's first node, `control`, sits across the graph while the
    /// tool's controls hold `values`, a share of its width from 0, the others evenly spaced
    /// after it around the hue circle: the colour equalizer's Node placement over 360°.
    /// The curve [`Self::curve`] gives runs from that node. `None`, as provided, for 0.
    fn curve_offset(&self, _control: &str, _values: &Controls) -> Option<f32> {
        None
    }

    /// What a hue curve graph paints for its curve of `control`, while the tool's controls
    /// hold `values`: the hue strip under it and its background, colours that are data.
    /// `None`, as provided, paints the graph's ground and no strip.
    fn curve_paint(&self, _control: &str, _values: &Controls) -> Option<CurvePaint> {
        None
    }

    /// The values option `option` of the preset row `preset` writes on this photo, one
    /// for each control the row names, each within its range; `None` where this photo
    /// cannot have it, as As shot on a photo that records no white balance (decision 2).
    /// Pure, as [`Self::bind`] is: it reads only the image context, so an option may be
    /// the controls' defaults for the photo. A tool that declares a preset row gives
    /// its options here. An option takes no follow writes ([`Self::follow`]), as
    /// darktable's presets write their parameters without `gui_changed`, so it writes
    /// itself every control a follow write would otherwise keep in step, as White
    /// balance's Settings writes `preset` with its coefficients.
    fn preset(&self, _preset: &str, _option: &str, _image: &ImageContext) -> Option<Controls> {
        None
    }

    /// The option of the preset row `preset` whose values `values` hold, `values` being
    /// every control, the edit's over the defaults; `None` for Custom. As provided, the
    /// first option whose [`Self::preset`] values equal the row's controls exactly, so an
    /// option given by the defaults matches after a reset, and a value off by a rounding
    /// reads Custom rather than an option it is not. A tool may compare in a precision
    /// of its own.
    fn matching(
        &self,
        preset: &str,
        values: &Controls,
        image: &ImageContext,
    ) -> Option<&'static str> {
        let row = self.definition().preset(preset)?;
        row.options.iter().copied().find(|option| {
            self.preset(preset, option, image).is_some_and(|set| {
                row.controls
                    .iter()
                    .all(|control| set.get(control) == values.get(control))
            })
        })
    }

    /// The coordinates of the lossless view `view` while the tool's controls hold
    /// `values`, every control, the edit's over the defaults: one for each of the view's
    /// coordinates, in the definition's order, and `None` for a view the tool lacks
    /// (decision 2). Pure, as [`Self::bind`] is, and reading only the controls the view
    /// covers and the image context, never the tool's other controls: the session holds
    /// a view's coordinates through a gesture for as long as those controls are unchanged,
    /// so coordinates that moved with another control would be reported stale. A
    /// coordinate may lie outside its range, where the values put it. A tool that
    /// declares a view gives its coordinates here.
    fn view(&self, _view: &str, _values: &Controls, _image: &ImageContext) -> Option<Vec<f32>> {
        None
    }

    /// The values of the controls the lossless view `view` covers that give `coordinates`,
    /// one for each of them: [`Self::view`]'s exact inverse, so a value read back through
    /// both moves by a rounding at most; for a view with fewer coordinates than controls,
    /// its inverse up to the scale the model drops, the values as darktable's GUI writes
    /// them, as White balance's coefficients normalised to green 1. A value may lie
    /// outside its control's range, which the session then refuses. `None` for a view the
    /// tool lacks.
    fn unview(&self, _view: &str, _coordinates: &[f32], _image: &ImageContext) -> Option<Controls> {
        None
    }

    /// What darktable's button `action`, an [`End::Action`] or a menu's item, writes when
    /// the tool's controls hold `values`, every control, the edit's over the defaults, and
    /// the facts it reads ([`ActionEnd::reads`]) are `read`: each control it sets with its
    /// new value, which a client sends as `SetControl`, one change, as orientation's rotate
    /// composes the orientation shown with a quarter turn. Pure, as [`Self::bind`] is, so
    /// the panel computes it from the values it shows and the facts the other tools publish
    /// for theirs. `None`, as provided, for an action the tool does not run.
    fn action(
        &self,
        _action: &str,
        _values: &Controls,
        _read: &PublishedValues,
        _image: &ImageContext,
    ) -> Option<Controls> {
        None
    }

    /// The fact `id` of [`ProcessingDefinition::publishes`] while the tool's controls hold
    /// `values`, every control, the edit's over the defaults, on the photo `image`, for
    /// another tool's action to read (decision 107): Exposure's `effective_exposure`. Pure,
    /// as [`Self::bind`] is, so wherever the action runs the fact comes from the same edit
    /// through the same function. `None`, as provided, for a fact the tool does not
    /// publish.
    fn publish(&self, _id: &str, _values: &Controls, _image: &ImageContext) -> Option<f32> {
        None
    }

    /// The other controls darktable's GUI writes when `control` takes the value `values`
    /// now holds, `values` being every control, the edit's over the defaults (decision
    /// 27; `docs/tools/README.md`, rule 13): Colour calibration's `x`, `y` and
    /// `temperature` when its illuminant changes, so every control describing the light
    /// describes the same one; White balance's `preset` user modified when a coefficient
    /// moves. Each within its range, and never `control` itself, nor another control the
    /// same command set. Pure, as [`Self::bind`] is: the session writes them in the same
    /// change, each kept, as the command's own, after a control or a coordinate is set or
    /// reset; never after a preset row's option, which writes every control it keeps in
    /// step itself ([`Self::preset`]). None, as provided.
    fn follow(&self, _control: &str, _values: &Controls, _image: &ImageContext) -> Controls {
        Controls::new()
    }

    /// The controls darktable's `gui_update` writes as it redraws the tool, while they hold
    /// `values`, every control, the edit's over the defaults: White balance's `preset` from
    /// image area turned into what its coefficients match, as `gui_update` writes
    /// `p->preset`. Each within its range. Pure, as [`Self::bind`] is: the session writes
    /// them into its edit wherever history's saved state is read again, as darktable
    /// redraws the tool: as the photo opens, after Undo, Redo or a move through history,
    /// and as a newer file is taken in. Never a history step of its own, it is saved with
    /// the tool's own next step; never after a change of the tool's own, as a pick, after
    /// which darktable shows the values as written. None, as provided.
    fn redrawn(&self, _values: &Controls, _image: &ImageContext) -> Controls {
        Controls::new()
    }

    /// What the tool remembers for the open photo after `moment`, darktable's memory of
    /// its panel that no edit stores and a preset row's option writes back ([`Self::recall`];
    /// decision 27): White balance's user-modified coefficients, darktable's `mod_coeff`.
    /// `memory` is what it remembered before, empty as the photo opens; `values` every
    /// control after the moment, the edit's over the defaults. Pure, as [`Self::bind`] is:
    /// the session keeps what it gives, one memory per tool for the open photo, never in
    /// the edit or its history, and forgets it as the photo closes. The memory as it was,
    /// as provided.
    fn remember(
        &self,
        _moment: Moment<'_>,
        memory: &Controls,
        _values: &Controls,
        _image: &ImageContext,
    ) -> Controls {
        memory.clone()
    }

    /// The values option `option` of the preset row `preset` writes from the tool's memory
    /// ([`Self::remember`]), for an option whose values darktable's panel remembers rather
    /// than [`Self::preset`] gives, as White balance's user modified writes `mod_coeff`
    /// back: one for each control the row names, each within its range, the option
    /// applied and offered as [`Self::preset`]'s are. `None`, as provided, for an option
    /// that does not recall.
    fn recall(
        &self,
        _preset: &str,
        _option: &str,
        _memory: &Controls,
        _image: &ImageContext,
    ) -> Option<Controls> {
        None
    }

    /// What the picker `picker` of [`ProcessingDefinition::pickers`] makes of `reading`,
    /// darktable's `color_picker_apply` (decision 79; `docs/pickers.md`, Declaring
    /// pickers): the controls it writes, each among the picker's `writes` and stored as
    /// darktable's rule gives it, with no other bound, and what the panel shows of the
    /// reading. `values` is every control, the edit's over the defaults; `settings` the
    /// settings its picker reads, as the client sent them; `channel` the channel tab the
    /// tool shows, as RGB levels' and RGB curve's; `last` the tool's last reading applied,
    /// one per tool and shared by its pickers, as darktable's `picked_color`; `keys` the
    /// keys held as the reading was asked for. Pure, as [`Self::bind`] is, so the tool's
    /// tests drive it with readings and no GPU: the session calls it as a reading arrives
    /// and applies what it gives as one change, with no follow writes ([`Self::follow`]).
    /// `None`, as provided, for a picker the tool does not run, and where a reading writes
    /// and shows nothing, as RGB levels' when it equals the last.
    #[allow(
        clippy::too_many_arguments,
        reason = "darktable's picker state, each part read by some tool's pick"
    )]
    fn pick(
        &self,
        _picker: &str,
        _reading: &Reading,
        _values: &Controls,
        _image: &ImageContext,
        _settings: &[(&str, ControlValue)],
        _channel: Option<&str>,
        _last: Option<&Reading>,
        _keys: PickKeys,
    ) -> Option<Picked> {
        None
    }

    /// Whether `picker` can start with the tool's settings as the client keeps them
    /// (decision 88): true, as provided. Pure, from the settings alone. A client draws a
    /// picker that is not ready disabled, its tooltip
    /// `tool-<tool>-picker-<picker>-unavailable` ([`messages::picker_unavailable`]), and
    /// the session refuses its `On` and turns it off where `Settings` leaves it not ready,
    /// so a script cannot pick toward nothing. Exposure's picker is not ready while Picker
    /// target holds Reference and no reference is set.
    fn picker_ready(&self, _picker: &str, _settings: &[(&str, ControlValue)]) -> bool {
        true
    }

    /// Whether the warning `id` at a slider's end ([`End::Warning`]) shows while the tool's
    /// controls hold `values`, every control, the edit's over the defaults, on the photo
    /// `image` (decision 107), as AgX's toe and shoulder power warn that the curve has lost
    /// its S shape. Pure, as [`Self::bind`] is: the session asks after every change and
    /// reports it with the tool's state, and the app's pending display follows it. False, as
    /// provided.
    fn warning(&self, _id: &str, _values: &Controls, _image: &ImageContext) -> bool {
        false
    }

    /// What the readout `readout` of a line of pickers ([`ReadoutDef`]) shows while the
    /// tool's controls hold `values`, every control, the edit's over the defaults: one of
    /// its texts with its number, and its swatch's colour, as Colour calibration's CCT and
    /// the illuminant's colour. Pure, as [`Self::view`] is: the session asks after every
    /// change and reports it with the tool's state. `None`, as provided, for a readout the
    /// tool lacks.
    fn readout(
        &self,
        _readout: &str,
        _values: &Controls,
        _image: &ImageContext,
    ) -> Option<Readout> {
        None
    }

    /// Whether the collapsible group `group`'s title says its rows act, as darktable's
    /// "area color mapping (active)", for the tool's settings as the client keeps them
    /// ([`messages::GROUP_ACTIVE`]): false, as provided. Pure, from the settings alone, as
    /// [`Self::picker_ready`] is.
    fn group_active(&self, _group: &str, _settings: &[(&str, ControlValue)]) -> bool {
        false
    }

    /// Values saved by version `from`, brought up to this version. A tool raises its
    /// version, and migrates, whenever a control's meaning or its binding changes, so an
    /// old edit keeps its look.
    fn migrate(&self, _from: u32, values: ToolValues) -> ToolValues {
        values
    }
}

/// A tool's values saved before it became switched (decision 32), as its migration
/// gives them back: on with every value, where they rendered something then, else off,
/// their values dropped, since they rendered nothing, and their marks kept. A tool that
/// becomes switched raises its version, and its [`ProcessingTool::migrate`] says whether an
/// older edit rendered, writing first, where darktable's defaults differ from the older
/// version's, those older defaults, so each edit binds what it bound.
pub fn switched_from(mut values: ToolValues, rendered: bool) -> ToolValues {
    if !rendered {
        let set: Vec<String> = values.values().map(|(id, _)| id.to_owned()).collect();
        for id in set {
            values.remove(&id);
        }
    }
    values.set_enabled(rendered);
    values
}

/// When darktable's panel updates its memory of a tool ([`ProcessingTool::remember`],
/// decision 27).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment<'a> {
    /// The photo opened, before it is first redrawn: darktable's `reload_defaults`.
    Opened,
    /// The tool redrawn, darktable's `gui_update`: wherever [`ProcessingTool::redrawn`]
    /// runs, and after a reset of the tool or of the edit that changed it, as darktable's
    /// reset runs `reload_defaults`, then `gui_update`.
    Redrawn,
    /// A control, or a lossless view's coordinate, moved by a command that set or reset
    /// it: darktable's `gui_changed` and its sliders' callbacks.
    Moved(&'a str),
}

/// A value of the photo's for a row's label ([`ProcessingTool::label_value`], decision 88):
/// plain data, which a client writes with its unit, `unit-<unit>-value`, in the language's
/// number format.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LabelValue {
    pub value: f32,
    pub unit: Unit,
    pub format: ValueFormat,
}

/// How a [`LabelValue`]'s number is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ValueFormat {
    /// As a camera writes its exposure bias, in thirds: "−⅔", "+1⅓", "+½", "0"; with
    /// one decimal, signed, where it is no third and no half: "+0.7", and whole where it
    /// rounds to a whole number at one decimal: "0", "+1".
    Thirds,
    /// With this many decimals, signed: "+1.0".
    Decimals(u8),
}

/// How close to a third or a half a value written in thirds must be to be written as
/// one: a camera's bias read from its metadata may be −0.67 as well as −⅔.
const THIRDS_TOLERANCE: f32 = 0.01;

impl LabelValue {
    /// The number as `format` writes it, its digits through `number`, the language's
    /// number format (`value`, decimals, signed), as `Translator::format_number` writes
    /// it; the unit is the client's to add.
    pub fn write(&self, number: impl Fn(f64, usize, bool) -> String) -> String {
        match self.format {
            ValueFormat::Decimals(decimals) => {
                number(f64::from(self.value), usize::from(decimals), true)
            }
            ValueFormat::Thirds => {
                let whole = self.value.abs().trunc();
                let part = self.value.abs() - whole;
                let near = |to: f32| (part - to).abs() <= THIRDS_TOLERANCE;
                // Rounded to the one decimal it would be written with, so a value that
                // rounds to a whole number is written as one, never "+1.0" or "+0.0".
                let tenths = (self.value.abs() * 10.0).round();
                let (whole, glyph) = if near(0.0) {
                    (whole, "")
                } else if near(1.0) {
                    (whole + 1.0, "")
                } else if near(1.0 / 3.0) {
                    (whole, "⅓")
                } else if near(0.5) {
                    (whole, "½")
                } else if near(2.0 / 3.0) {
                    (whole, "⅔")
                } else if tenths % 10.0 == 0.0 {
                    (tenths / 10.0, "")
                } else {
                    let rounded = (tenths / 10.0).copysign(self.value);
                    return number(f64::from(rounded), 1, true);
                };
                if whole == 0.0 && glyph.is_empty() {
                    return number(0.0, 0, false);
                }
                let sign = if self.value < 0.0 { "−" } else { "+" };
                let digits = if whole == 0.0 {
                    String::new()
                } else {
                    number(f64::from(whole), 0, false)
                };
                format!("{sign}{digits}{glyph}")
            }
        }
    }
}

/// One operation parameter's value: what a binding sets, and what the session reports
/// for the parameters no tool owns.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Setting {
    #[serde(deserialize_with = "apertrust_base::wire::id")]
    pub operation: Id,
    #[serde(deserialize_with = "apertrust_base::wire::id")]
    pub param: Id,
    pub value: Value,
}

impl Setting {
    pub const fn new(operation: &'static str, param: &'static str, value: Value) -> Self {
        Self {
            operation,
            param,
            value,
        }
    }
}

/// An operation parameter a tool owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owned {
    pub operation: &'static str,
    pub param: &'static str,
}

impl Owned {
    pub const fn new(operation: &'static str, param: &'static str) -> Self {
        Self { operation, param }
    }
}

/// Each operation's parameters for an unedited photo: exactly its
/// `Operation::defaults(image)`, every parameter resolved over its schema. A binding
/// writes a relative control over it; a layer, in Phase 4, may bind over a base of its
/// own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Base(BTreeMap<&'static str, ParamValues>);

impl Base {
    pub fn new(operations: &[&dyn Operation], image: &ImageContext) -> Self {
        let resolved = operations.iter().map(|operation| {
            let defaults = operation.defaults(image);
            let params = operation
                .schema()
                .resolve_over(&defaults, &ParamValues::new())
                .unwrap_or(defaults);
            (operation.manifest().id, params)
        });
        Self(resolved.collect())
    }

    /// The operation's parameters, if it is one of those given.
    pub fn params(&self, operation: &str) -> Option<&ParamValues> {
        self.0.get(operation)
    }

    pub fn get(&self, operation: &str, param: &str) -> Option<Value> {
        self.params(operation)?.get(param)
    }

    /// A number parameter's value. A binding reads only parameters it owns, which
    /// [`check::accept`] holds to exist, so a missing one is a bug in the tool.
    pub fn float(&self, operation: &str, param: &str) -> f32 {
        match self.get(operation, param) {
            Some(Value::Float(value)) => value,
            other => panic!("{operation}.{param}: expected a number, got {other:?}"),
        }
    }

    /// A boolean parameter's value; see [`Self::float`].
    pub fn flag(&self, operation: &str, param: &str) -> bool {
        match self.get(operation, param) {
            Some(Value::Bool(value)) => value,
            other => panic!("{operation}.{param}: expected true or false, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `value` as `format` writes it, its digits as English writes them, `−` for minus.
    fn written(value: f32, format: ValueFormat) -> String {
        let number = |value: f64, decimals: usize, signed: bool| {
            let digits = format!("{:.decimals$}", value.abs());
            match (value < 0.0, signed) {
                (true, _) => format!("−{digits}"),
                (false, true) => format!("+{digits}"),
                (false, false) => digits,
            }
        };
        let unit = Unit::Ev;
        LabelValue {
            value,
            unit,
            format,
        }
        .write(number)
    }

    #[test]
    fn thirds_are_written_as_a_camera_writes_its_bias() {
        let thirds = |value| written(value, ValueFormat::Thirds);
        assert_eq!(thirds(-2.0 / 3.0), "−⅔");
        assert_eq!(thirds(-0.67), "−⅔");
        assert_eq!(thirds(4.0 / 3.0), "+1⅓");
        assert_eq!(thirds(1.33), "+1⅓");
        assert_eq!(thirds(0.0), "0");
        assert_eq!(thirds(-0.5), "−½");
        assert_eq!(thirds(2.0), "+2");
        assert_eq!(thirds(-0.995), "−1");
        // No third and no half: one decimal, signed.
        assert_eq!(thirds(0.7), "+0.7");
        assert_eq!(thirds(-1.2), "−1.2");
        // Off a whole number by more than the tolerance but rounding to it at one
        // decimal: written whole, never "+0.0" for a negative value nor "+1.0" for 1.
        assert_eq!(thirds(-0.02), "0");
        assert_eq!(thirds(0.04), "0");
        assert_eq!(thirds(0.98), "+1");
        assert_eq!(thirds(1.03), "+1");
        assert_eq!(thirds(-1.96), "−2");
        assert_eq!(thirds(0.06), "+0.1");
        assert_eq!(thirds(-0.06), "−0.1");
    }

    #[test]
    fn decimals_are_written_signed() {
        assert_eq!(written(1.0, ValueFormat::Decimals(1)), "+1.0");
        assert_eq!(written(0.5, ValueFormat::Decimals(1)), "+0.5");
        assert_eq!(written(-0.25, ValueFormat::Decimals(2)), "−0.25");
    }

    /// A retired id taken again belongs to the tool that took it, which reads its
    /// values, so it leaves the list.
    #[test]
    fn no_retired_id_is_a_built_in_tool_s() {
        for id in RETIRED {
            assert!(
                !TOOLS.iter().any(|tool| tool.definition().tool.id == *id),
                "{id} is retired and built in"
            );
        }
    }
}
