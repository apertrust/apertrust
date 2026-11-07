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

//! Apertrust's modules (architecture decisions 4 and 28): what the user edits with. A
//! module is one editing task, as the modules README lists them (`docs/modules/README.md`),
//! with its controls, its tabs and maybe Auto; an operation is one of the engine's
//! algorithms. A module implements [`Module`]:
//!
//! - its [`Definition`], static data: the controls an edit stores apart from the rows a
//!   panel draws them in, and no text, since every word is a message id that follows
//!   from the ids ([`messages`]);
//! - its defaults for a photo, the definition's unless they come from the photo, and
//!   whether each control can change anything on it while the module's values stand;
//! - the operation parameters it owns, each owned by one module only ([`check`]);
//! - its binding: a pure function from its own values, the photo's image context and
//!   [`Base`], each operation's defaults for the photo, to the parameters it sets,
//!   through transfer functions that carry their derivatives ([`transfer`]);
//! - migrations from the values an earlier version saved.
//!
//! The engine's session runs the bindings: every client sends module values, and the
//! session turns them into operation parameters, so the app, the CLI and MCP see the same
//! modules. This crate depends on the operation interface and the shared colour maths
//! only, never on egui, wgpu, the engine or an operation crate; it names operations and
//! parameters by id.

pub mod agx;
pub mod check;
pub mod colour_balance_rgb;
pub mod colour_calibration;
mod definition;
pub mod demosaic;
pub mod exposure;
pub mod grain;
pub mod haze_removal;
pub mod highlight_reconstruction;
pub mod messages;
pub mod orientation;
pub mod output_colour_profile;
pub mod raw_black_white_point;
pub mod rgb_primaries;
pub mod soften;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tone_curve;
pub mod tone_equalizer;
pub mod transfer;
mod values;
pub mod vignetting;
pub mod white_balance;

use std::collections::BTreeMap;

use apertrust_plugin::{ImageContext, Operation, ParamValues, Value};

pub use definition::{
    Button, ButtonIcon, ChoiceStyle, Control, ControlError, ControlKind, Coordinate, Definition,
    DefinitionError, MAX_POINTS, MIN_GAP, PresetRow, Quad, Range, Row, Slider, Tab, ToolSetting,
    Track, Unit, View, Widget, slider_step,
};
pub use values::{ControlValue, Controls, Mark, ModuleValues};

/// The built-in modules, in the order the session reports them and an edit saves them:
/// Raw black/white point, Highlight reconstruction, Demosaic, AgX and Output colour
/// profile, in the Profile tab's order; then White balance, Orientation, Exposure, Colour
/// calibration, Tone equalizer, Colour balance RGB, Tone curve, which becomes RGB curve,
/// Haze removal, then RGB primaries, Grain, Soften and Vignetting, as the side panel's
/// Profile and Adjust tabs listed them until decision 38 gave it darktable's groups, whose
/// lists are the app's. The first pass's other modules follow (`docs/gui.md`, Build
/// order).
pub const MODULES: &[&dyn Module] = &[
    &raw_black_white_point::RawBlackWhitePoint,
    &highlight_reconstruction::HighlightReconstruction,
    &demosaic::Demosaic,
    &agx::Agx,
    &output_colour_profile::OutputColourProfile,
    &white_balance::WhiteBalance,
    &orientation::Orientation,
    &exposure::Exposure,
    &colour_calibration::ColourCalibration,
    &tone_equalizer::ToneEqualizer,
    &colour_balance_rgb::ColourBalanceRgb,
    &tone_curve::ToneCurve,
    &haze_removal::HazeRemoval,
    &rgb_primaries::RgbPrimaries,
    &grain::Grain,
    &soften::Soften,
    &vignetting::Vignetting,
];

/// The ids of the modules Apertrust has retired (decision 43): an older build wrote
/// their values, so an edit holding one loses them with a warning and still opens for
/// editing, where a module neither built in nor retired is a newer Apertrust's and opens
/// the photo for viewing only. A module removed or renamed joins the list, as the Tone
/// curve's `tone_curve` will once RGB curve replaces it (decision 2), and leaves it when
/// a module takes its id again. Empty until then.
pub const RETIRED: &[&str] = &[];

/// The contract between the engine's session and a module (decision 28).
pub trait Module: Send + Sync {
    fn definition(&self) -> &'static Definition;

    /// Each control's untouched value for the photo: the definition's, unless it comes
    /// from the photo, as White balance's as-shot temperature will.
    fn defaults(&self, _image: &ImageContext) -> Controls {
        self.definition().defaults()
    }

    /// Every operation parameter the module owns (decision 4). No other module owns
    /// one of them, and nothing else sets them.
    fn owns(&self) -> &'static [Owned];

    /// The parameters of [`Self::owns`] no control sets, which the module holds at the
    /// operation's value for the photo, as Raw black/white point holds the raw's crop
    /// margins and Colour calibration darktable's version 3 (rule 12 of
    /// `docs/modules/README.md`): a client asking to set one is told the module fixes it,
    /// not that it sets it. None, as provided.
    fn fixed(&self) -> &'static [Owned] {
        &[]
    }

    /// The operation parameters these values set: a pure function of the module's own
    /// values, the photo's image context at full resolution and `base`, never of another
    /// module, a pipe or any state, so the viewport and the export get the same
    /// parameters and the order controls were used in cannot matter.
    ///
    /// `values` holds only the controls off their defaults for the photo, and the session
    /// binds no module that has none, so a module at its defaults sets nothing and leaves
    /// the plan exactly as it was. It sets only parameters it owns, each within its
    /// schema: an operation an unedited photo does not run is planned while some binding
    /// sets one of its parameters, so a binding that needs one at its defaults sets one
    /// of them to its default explicitly. A control relative to the photo's default is
    /// written over `base`.
    fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting>;

    /// Whether the control can change anything on this photo while the module's controls
    /// hold `values`, every control, the edit's over the defaults: true unless the photo
    /// lacks what it acts on, as Exposure's Use camera bias does on a photo that records
    /// no bias (decision 30), or another of the module's controls holds a value under
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

    /// The options the choice `control` lists on this photo, by id, in the definition's
    /// order (decision 30): darktable's list for the photo, as demosaic lists its
    /// sensor's methods and colour calibration lists as shot in camera only on a colour
    /// raw. Never empty, and holding the photo's default ([`check::listed`]). Pure, and
    /// reading only the photo, never the module's values: the session asks once as the
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

    /// The range of the number control `control` while the module's controls hold
    /// `values`, every control, the edit's over the defaults (decision 30): what a value
    /// may hold, what the track spans at rest and what a key moves, as darktable's slider
    /// has them for those values. The definition's, as provided ([`Definition::range`]);
    /// `None` for a control that is no number. Override it where darktable's `gui_changed`
    /// moves a slider's range with other controls, as AgX's pivot spans black to white
    /// relative exposure: the definition's range is then the widest any values give, and
    /// its soft range and step those at the module's defaults. Pure, as [`Self::bind`] is,
    /// reading the module's values but never `control`'s own, nor the photo. The session
    /// reports it, refuses a command outside it, and after every change clamps a number a
    /// moved range leaves outside, written at its nearer end in the same change
    /// ([`check::clamp`]).
    fn range(&self, control: &str, _values: &Controls) -> Option<Range> {
        self.definition().range(control)
    }

    /// The curve a curve row draws through the points of `control`, or a band graph
    /// through its bands, named by its first, while the module's controls hold `values`:
    /// its output at an input, both on the row's axes, so the curve drawn is the curve the
    /// operation renders. `None`, as provided, joins the points with straight lines; a
    /// band graph draws them in the warning colour, as no curve fits its bands.
    fn curve(&self, _control: &str, _values: &Controls) -> Option<Box<dyn Fn(f32) -> f32>> {
        None
    }

    /// The width σ, in its input's units, of the Gaussians the curve [`Self::curve`] gives
    /// for `control` is a sum of, while the module's controls hold `values`: how far a drag
    /// on a band graph reaches, as darktable's tone equalizer moves the bands around the
    /// pointer by a Gaussian its curve smoothing widens. `None`, as provided, for a curve
    /// of no such width, where a band graph takes darktable's default, √2.
    fn curve_width(&self, _control: &str, _values: &Controls) -> Option<f32> {
        None
    }

    /// The values option `option` of the preset row `preset` writes on this photo, one
    /// for each control the row names, each within its range; `None` where this photo
    /// cannot have it, as As shot on a photo that records no white balance (decision 2).
    /// Pure, as [`Self::bind`] is: it reads only the image context, so an option may be
    /// the controls' defaults for the photo. A module that declares a preset row gives
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
    /// reads Custom rather than an option it is not. A module may compare in a precision
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

    /// The coordinates of the lossless view `view` while the module's controls hold
    /// `values`, every control, the edit's over the defaults: one for each of the view's
    /// coordinates, in the definition's order, and `None` for a view the module lacks
    /// (decision 2). Pure, as [`Self::bind`] is, and reading only the controls the view
    /// covers and the image context, never the module's other controls: the session holds
    /// a view's coordinates through a gesture for as long as those controls are unchanged,
    /// so coordinates that moved with another control would be reported stale. A
    /// coordinate may lie outside its range, where the values put it. A module that
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
    /// module lacks.
    fn unview(&self, _view: &str, _coordinates: &[f32], _image: &ImageContext) -> Option<Controls> {
        None
    }

    /// What darktable's button `action` of a [`Row::Buttons`] writes when the module's
    /// controls hold `values`, every control, the edit's over the defaults: each control
    /// it sets with its new value, which a client sends as `SetControl`, one change, as
    /// orientation's rotate composes the orientation shown with a quarter turn. Pure, as
    /// [`Self::bind`] is, so the panel computes it from the values it shows. `None`, as
    /// provided, for an action the module does not run.
    fn action(&self, _action: &str, _values: &Controls, _image: &ImageContext) -> Option<Controls> {
        None
    }

    /// The other controls darktable's GUI writes when `control` takes the value `values`
    /// now holds, `values` being every control, the edit's over the defaults (decision
    /// 27; `docs/modules/README.md`, rule 13): Colour calibration's `x`, `y` and
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

    /// Values saved by version `from`, brought up to this version. A module raises its
    /// version, and migrates, whenever a control's meaning or its binding changes, so an
    /// old edit keeps its look.
    fn migrate(&self, _from: u32, values: ModuleValues) -> ModuleValues {
        values
    }
}

/// One operation parameter's value: what a binding sets, and what the session reports
/// for the parameters no module owns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Setting {
    pub operation: &'static str,
    pub param: &'static str,
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

/// An operation parameter a module owns.
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
    /// [`check::accept`] holds to exist, so a missing one is a bug in the module.
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

    /// A retired id taken again belongs to the module that took it, which reads its
    /// values, so it leaves the list.
    #[test]
    fn no_retired_id_is_a_built_in_module_s() {
        for id in RETIRED {
            assert!(
                !MODULES.iter().any(|module| module.definition().id == *id),
                "{id} is retired and built in"
            );
        }
    }
}
