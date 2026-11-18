// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/temperature.c)
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

//! White balance (`docs/tools/white-balance.md`): darktable's white balance module
//! as its page has it, over the raw-stage `temperature` operation (decision 27), on one
//! untitled tab:
//!
//! - **Settings,** a preset row: as shot, from image area, user modified, camera reference
//!   and as shot to reference, each writing the coefficients and where they came from,
//!   `preset`, as darktable's `_preset_tune_callback` does. From image area turns the
//!   picker on, never applied; user modified writes back darktable's memory of the
//!   coefficients last set, `mod_coeff`, which the session keeps for the open photo
//!   ([`ProcessingTool::remember`] and [`ProcessingTool::recall`]). Its label is the option
//!   the coefficients hold, as
//!   `gui_update` names it within darktable's 1e-5, or from image area while `preset`
//!   holds it after a pick.
//! - **The picker,** `spot`, darktable's "from image area": an area of the mosaic at the
//!   operation's input, its pick setting the coefficients that make the area neutral and
//!   `preset` from image area ([`ProcessingTool::pick`]). Where darktable redraws the tool, as the
//!   photo opens and after Undo, Redo or a move through history, from image area becomes
//!   what the coefficients match, user modified for a picked balance ([`ProcessingTool::redrawn`]).
//! - **Temperature and Tint,** under the scene illuminant temp label: darktable's model of
//!   the coefficients through the camera's colour matrix
//!   (`apertrust_color::white_balance`), a lossless view of the three with two
//!   coordinates, editing either writing them normalised to green 1.
//! - **Channel coefficients,** a collapsible group, closed at first: Red, Green and Blue.
//!
//! Each coefficient is its parameter as it is, and `preset` is shown by Settings alone. The
//! fourth coefficient, `various`, a four-colour sensor's Emerald, which darktable shows on
//! such a sensor alone, is a control not ported, in no row: Settings writes it with the
//! other three, as darktable's `_temp_params_from_array` writes all four. The defaults are
//! the operation's for the photo: the as-shot coefficients, with "as shot to reference"
//! where the camera has a colour matrix. Moving a coefficient, temperature or tint writes
//! `preset` user modified, as darktable's `gui_changed` and `_temp_tint_callback` do
//! ([`ProcessingTool::follow`]), so colorin's late correction runs only with "as shot to
//! reference" and the as-shot coefficients. The button bar and the camera's presets with
//! Finetune come later (`docs/tools/white-balance.md`, Gaps).

use apertrust_color::white_balance::{Matrices, TEMPERATURE_RANGE, TINT_RANGE};
use apertrust_plugin::{Camera, ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    ChoiceStyle, Control, ControlKind, Coordinate, Picker, PickerKind, PresetRow,
    ProcessingDefinition, Reset, Row, Slider, Space, Tab, ToolDefinition, Unit, View,
};
use crate::pick::{PickKeys, Picked, Reading, Shown};
use crate::{Base, ControlValue, Controls, Moment, Owned, ProcessingTool, Setting, ToolValues};

pub struct WhiteBalance;

/// The operation it drives.
const OPERATION: &str = "temperature";

/// The coefficients, each the parameter of its name.
pub const COEFFICIENTS: [&str; 3] = ["red", "green", "blue"];

/// darktable's settings, `DT_IOP_TEMP_AS_SHOT` to `DT_IOP_TEMP_D65_LATE`, by their values:
/// as shot, from image area, user modified, camera reference, as shot to reference.
pub const SETTINGS: [&str; 5] = ["as_shot", "spot", "user", "d65", "d65_late"];
const AS_SHOT: u32 = 0;
const SPOT: u32 = 1;
const USER: u32 = 2;
const D65: u32 = 3;
const D65_LATE: u32 = 4;

/// The preset row and the view.
pub const SETTINGS_ROW: &str = "settings";
pub const TEMPERATURE_TINT: &str = "temperature_tint";

/// One channel's coefficient: darktable's 0 to 8, stepping by what
/// `dt_bauhaus_slider_get_step` gives over it, and 1 before the photo is known.
const fn coefficient(id: &'static str) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min: 0.0,
            max: 8.0,
            step: 0.1,
            default: 1.0,
        },
        unit: None,
        unported: false,
    }
}

/// The coefficients and their setting, darktable's parameters. A photo's defaults are
/// the operation's ([`ProcessingTool::defaults`]); before it, darktable's for an image that
/// is not a raw, every coefficient 1, as shot. `various`, the fourth, is not ported:
/// temperature takes no four-colour sensor, and on any other nothing reads it.
static CONTROLS: [Control; 5] = [
    coefficient("red"),
    coefficient("green"),
    coefficient("blue"),
    Control {
        unported: true,
        ..coefficient("various")
    },
    Control {
        id: "preset",
        kind: ControlKind::Choice {
            options: &SETTINGS,
            default: AS_SHOT,
            unported: &[],
        },
        unit: None,
        unported: false,
    },
];

/// Temperature and tint, darktable's `scale_k` and `scale_tint`: 1901 to 25000 K by 1 K,
/// and 0.135 to 2.326 by 0.01, as `dt_bauhaus_slider_get_step` gives them over their
/// ranges, a tint below 1 towards magenta and above towards green.
static VIEWS: [View; 1] = [View {
    id: TEMPERATURE_TINT,
    controls: &COEFFICIENTS,
    coordinates: &[
        Coordinate {
            id: "temperature",
            min: TEMPERATURE_RANGE.0 as f32,
            max: TEMPERATURE_RANGE.1 as f32,
            step: 1.0,
            unit: Some(Unit::Kelvin),
        },
        Coordinate {
            id: "tint",
            min: TINT_RANGE.0 as f32,
            max: TINT_RANGE.1 as f32,
            step: 0.01,
            unit: None,
        },
    ],
}];

/// A coefficient's row, with darktable's three digits.
const fn coefficient_row(control: &'static str) -> Row {
    Row::Slider(Slider::new(control).decimals(3))
}

/// darktable's page, `gui_init`'s order: Settings, the scene illuminant temp label with
/// temperature, no digits, and tint, three, then the channel coefficients' collapsible
/// group. The button bar above Settings and Finetune after it are planned.
static DEFINITION: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: "temperature",
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Preset {
                    preset: SETTINGS_ROW,
                    style: ChoiceStyle::List,
                },
                Row::Label {
                    id: "scene_illuminant_temp",
                },
                Row::Slider(Slider::new("temperature").decimals(0)),
                Row::Slider(Slider::new("tint").decimals(3)),
                Row::Group {
                    id: "channel_coefficients",
                    rows: &[
                        coefficient_row("red"),
                        coefficient_row("green"),
                        coefficient_row("blue"),
                    ],
                },
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::Tool,
        presets: true,
        open_at_first: false,
        menus: &[],
    },
    version: 1,
    // Background only, as the raw stage.
    layers: false,
    auto: false,
    switched: false,
    controls: &CONTROLS,
    preset_rows: &[PresetRow {
        id: SETTINGS_ROW,
        options: &SETTINGS,
        controls: &["red", "green", "blue", "various", "preset"],
        // From image area turns the picker on, its one place until the button bar comes.
        pickers: &[("spot", PICKER.id)],
    }],
    views: &VIEWS,
    pickers: &[PICKER],
    // darktable's `description()` in src/iop/temperature.c at the reference release.
    facts: Some(Facts {
        purpose: Purpose::Corrective,
        input: Fact::LinearRawSceneReferred,
        process: Fact::LinearRaw,
        output: Fact::LinearRawSceneReferred,
    }),
};

/// darktable's picker, "from image area": `dt_color_picker_new_with_cst(self,
/// DT_COLOR_PICKER_AREA, NULL, IOP_CS_NONE)` (`temperature.c:2088`), an area with no
/// denoise read in no space of its own, so the mosaic at the operation's input per CFA
/// colour; its pick writes the coefficients and `preset` ([`ProcessingTool::pick`]).
const PICKER: Picker = Picker {
    id: "spot",
    operation: OPERATION,
    kind: PickerKind::Area,
    denoise: false,
    output: false,
    space: Space::Operation,
    keep: false,
    writes: &["red", "green", "blue", "preset"],
    settings: &[],
    words_by: None,
};

/// darktable's bound on a picked coefficient, `color_picker_apply`'s own.
const PICKED_MAX: f32 = 8.0;

/// A mean at or below which darktable takes a channel as giving 1.
const PICKED_FLOOR: f32 = 0.001;

/// Every parameter of `temperature`: the four coefficients and `preset`.
static OWNS: [Owned; 5] = [
    Owned::new(OPERATION, "red"),
    Owned::new(OPERATION, "green"),
    Owned::new(OPERATION, "blue"),
    Owned::new(OPERATION, "various"),
    Owned::new(OPERATION, "preset"),
];

impl ProcessingTool for WhiteBalance {
    fn definition(&self) -> &'static ProcessingDefinition {
        &DEFINITION
    }

    /// The operation's defaults for the photo, darktable's scene-referred
    /// `reload_defaults`: on a colour raw the as-shot coefficients over green's, each
    /// held to the range, with "as shot to reference" where the camera has a colour
    /// matrix and "as shot" without; on any other image the definition's, every
    /// coefficient 1, as shot.
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        let Some(camera) = colour_raw(image) else {
            return defaults;
        };
        for (control, value) in FOUR.into_iter().zip(as_shot_coefficients(camera)) {
            defaults.set(control, ControlValue::Number(value));
        }
        let preset = if camera.xyz_to_camera.is_some() {
            D65_LATE
        } else {
            AS_SHOT
        };
        defaults.set("preset", ControlValue::Choice(preset));
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default as its parameter, the value as it is: the operation
    /// runs on every raw, so nothing else is needed, and nothing at all at the defaults.
    fn bind(&self, values: &ToolValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        CONTROLS
            .iter()
            .filter_map(|control| {
                let value = match values.get(control.id)? {
                    ControlValue::Number(value) => Value::Float(*value),
                    ControlValue::Choice(index) => Value::Choice(*index),
                    ControlValue::Bool(_) | ControlValue::Points(_) => return None,
                };
                Some(Setting::new(OPERATION, control.id, value))
            })
            .collect()
    }

    /// Temperature and tint can change nothing without the camera's colour matrix, which
    /// they read the coefficients through (decision 30); the coefficients still can.
    fn available(&self, control: &str, _: &Controls, image: &ImageContext) -> bool {
        !matches!(control, "temperature" | "tint") || matrix(image).is_some()
    }

    /// Settings' options as darktable's `_preset_tune_callback` writes them, all four
    /// coefficients (`_temp_params_from_array`) and `preset`: as shot, the as-shot
    /// coefficients; camera reference, the camera's D65 ones
    /// (`_calculate_bogus_daylight_wb`); as shot to reference, the as-shot coefficients,
    /// under which colorin applies the late correction. darktable offers all three on
    /// every photo, so does the row. From image area is never applied: it turns the
    /// picker on (`PresetRow::pickers`), whose pick writes the values; user modified
    /// writes the session's memory ([`ProcessingTool::recall`]).
    fn preset(&self, preset: &str, option: &str, image: &ImageContext) -> Option<Controls> {
        if preset != SETTINGS_ROW {
            return None;
        }
        let (coefficients, setting) = match option {
            "as_shot" => (as_shot(image), AS_SHOT),
            "d65" => (daylight(image), D65),
            "d65_late" => (as_shot(image), D65_LATE),
            _ => return None,
        };
        let mut values = Controls::new();
        for (control, value) in FOUR.into_iter().zip(coefficients) {
            values.set(control, ControlValue::Number(value));
        }
        values.set("preset", ControlValue::Choice(setting));
        Some(values)
    }

    /// From image area while `preset` holds it, as darktable's combobox shows it right
    /// after a pick (`temperature.c:1967`); otherwise darktable's `gui_update` label, within
    /// its `dt_dev_equal_chroma`, 1e-5 on each of the first three coefficients: as shot to
    /// reference with the as-shot coefficients and that setting, as shot with the as-shot
    /// coefficients, camera reference with the D65 ones, user modified otherwise.
    fn matching(
        &self,
        preset: &str,
        values: &Controls,
        image: &ImageContext,
    ) -> Option<&'static str> {
        if preset != SETTINGS_ROW {
            return None;
        }
        if values.choice("preset") == SPOT {
            return Some("spot");
        }
        Some(SETTINGS[label(values, image) as usize])
    }

    /// darktable's `gui_update` writing the label back into `p->preset`
    /// (`temperature.c:1336-1340`) where it holds from image area: what the coefficients
    /// alone match, as [`ProcessingTool::matching`] names it, a picked balance user modified. Any
    /// other setting is left
    /// as it is, as Settings' label only names it.
    fn redrawn(&self, values: &Controls, image: &ImageContext) -> Controls {
        let mut writes = Controls::new();
        if values.choice("preset") == SPOT {
            writes.set("preset", ControlValue::Choice(label(values, image)));
        }
        writes
    }

    /// darktable's `mod_coeff`, the four coefficients user modified writes back, and
    /// whether they are set yet (`mod_temp` other than `-FLT_MAX`):
    ///
    /// - as the photo opens, the defaults' four, not set (`reload_defaults`,
    ///   `temperature.c:1655`);
    /// - redrawn, the four the controls hold, set, where the label works out as user
    ///   modified or they are set already (`gui_update`, `:1344-1350`);
    /// - a coefficient, Emerald, temperature or tint moved, the four the move leaves, set
    ///   (`gui_changed`, `:1740`, and `_temp_tint_callback`, `:1720-1729`, whose
    ///   normalising to green 1 [`ProcessingTool::unview`] has done).
    fn remember(
        &self,
        moment: Moment<'_>,
        memory: &Controls,
        values: &Controls,
        image: &ImageContext,
    ) -> Controls {
        let set = memory.get(SET).is_some();
        match moment {
            Moment::Opened => four(&self.defaults(image), false),
            Moment::Redrawn if set || label(values, image) == USER => four(values, true),
            Moment::Moved(control)
                if FOUR.contains(&control) || matches!(control, "temperature" | "tint") =>
            {
                four(values, true)
            }
            Moment::Redrawn | Moment::Moved(_) => memory.clone(),
        }
    }

    /// User modified, darktable's `_preset_tune_callback` (`temperature.c:1824-1827`): the
    /// four coefficients the session remembers ([`ProcessingTool::remember`]) and `preset`
    /// user modified. `None` for a memory that holds no coefficients.
    fn recall(
        &self,
        preset: &str,
        option: &str,
        memory: &Controls,
        _: &ImageContext,
    ) -> Option<Controls> {
        if (preset, option) != (SETTINGS_ROW, "user") {
            return None;
        }
        let mut values = Controls::new();
        for control in FOUR {
            values.set(control, memory.get(control)?.clone());
        }
        values.set("preset", ControlValue::Choice(USER));
        Some(values)
    }

    /// darktable's `color_picker_apply` (`temperature.c:1948-1969`) on the area's mean at
    /// the operation's input, one channel per CFA colour, `m`: green normalises by `1 /
    /// m[1]`, or 1 where `m[1]` is 0.001 or below; each coefficient is `1 / m[c]`, or 1
    /// where `m[c]` is 0.001 or below, over that, held to darktable's own 0 to 8, then
    /// green is 1 and `preset` from image area. darktable writes Emerald too, which is not
    /// ported and stays as it is: nothing reads it on a three-colour sensor. Nothing where
    /// the area holds no red photosite, as darktable returns on an empty reading. It shows
    /// nothing, as darktable's white balance has no patch.
    fn pick(
        &self,
        picker: &str,
        reading: &Reading,
        _values: &Controls,
        _image: &ImageContext,
        _settings: &[(&str, ControlValue)],
        _channel: Option<&str>,
        _last: Option<&Reading>,
        _keys: PickKeys,
    ) -> Option<Picked> {
        let input = &reading.input;
        if picker != PICKER.id || input.max[0] < input.min[0] {
            return None;
        }
        let m = input.mean;
        let inverse = |mean: f32| {
            if mean > PICKED_FLOOR { 1.0 / mean } else { 1.0 }
        };
        let green = inverse(m[1]);
        let mut writes = Controls::new();
        for (c, control) in COEFFICIENTS.into_iter().enumerate() {
            // `fmaxf(0, fminf(8, …))`; a mean is never NaN, and a value that is not
            // finite the session would not write.
            let coefficient = (inverse(m[c]) / green).clamp(0.0, PICKED_MAX);
            writes.set(control, ControlValue::Number(coefficient));
        }
        writes.set("green", ControlValue::Number(1.0));
        writes.set("preset", ControlValue::Choice(SPOT));
        Some(Picked {
            writes,
            shown: Shown::default(),
        })
    }

    /// Temperature and tint of the coefficients through the camera's matrix, darktable's
    /// `_mul2temp` bisected to convergence; sRGB's matrix, darktable's for an image that is
    /// not a raw, where the photo has none, the coordinates then unavailable.
    fn view(&self, view: &str, values: &Controls, image: &ImageContext) -> Option<Vec<f32>> {
        (view == TEMPERATURE_TINT).then(|| {
            let coefficients = COEFFICIENTS.map(|control| f64::from(values.number(control)));
            matrices(image)
                .temperature_tint(coefficients)
                .map(|c| c as f32)
                .to_vec()
        })
    }

    /// The coefficients a temperature and a tint give, normalised to green 1 as
    /// `_temp_tint_callback` writes them: the view's inverse, but for the coefficients'
    /// scale, which the model drops.
    fn unview(&self, view: &str, coordinates: &[f32], image: &ImageContext) -> Option<Controls> {
        let (TEMPERATURE_TINT, &[t, tint]) = (view, coordinates) else {
            return None;
        };
        let coefficients = matrices(image).coefficients(f64::from(t), f64::from(tint));
        let mut values = Controls::new();
        for (control, value) in COEFFICIENTS.into_iter().zip(coefficients) {
            values.set(control, ControlValue::Number(value as f32));
        }
        Some(values)
    }

    /// darktable's `gui_changed` and `_temp_tint_callback`: moving a coefficient, which
    /// temperature and tint do too, sets the setting to user modified.
    fn follow(&self, control: &str, _: &Controls, _: &ImageContext) -> Controls {
        let mut writes = Controls::new();
        if COEFFICIENTS.contains(&control) {
            writes.set("preset", ControlValue::Choice(USER));
        }
        writes
    }
}

/// darktable's `gui_update` label (`temperature.c:1206-1340`), the setting by its value,
/// within its `dt_dev_equal_chroma`, 1e-5 on each of the first three coefficients, against
/// what the options write, as darktable compares the parameters with the `as_shot` and
/// `D65coeffs` it wrote: as shot to reference with the as-shot coefficients and that
/// setting; as shot with the as-shot coefficients; camera reference with the D65 ones;
/// user modified otherwise, which no option applies.
fn label(values: &Controls, image: &ImageContext) -> u32 {
    let coefficients = COEFFICIENTS.map(|control| values.number(control));
    let equal = |other: [f32; 4]| (0..3).all(|c| (coefficients[c] - other[c]).abs() < 1e-5);
    let as_shot = equal(as_shot(image));
    if as_shot && values.choice("preset") == D65_LATE {
        D65_LATE
    } else if as_shot {
        AS_SHOT
    } else if equal(daylight(image)) {
        D65
    } else {
        USER
    }
}

/// The camera of a colour raw, which darktable's white balance acts on.
fn colour_raw(image: &ImageContext) -> Option<&Camera> {
    image.camera.as_ref().filter(|camera| camera.cfa.is_some())
}

/// The four coefficients, `various` the fourth, as `_temp_params_from_array` writes them.
const FOUR: [&str; 4] = ["red", "green", "blue", "various"];

/// In White balance's memory, that its coefficients are set: darktable's `mod_temp` other
/// than `-FLT_MAX`.
const SET: &str = "set";

/// The four coefficients `values` holds, as White balance's memory keeps them, set or not.
fn four(values: &Controls, set: bool) -> Controls {
    let mut memory = Controls::new();
    for control in FOUR {
        if let Some(value) = values.get(control) {
            memory.set(control, value.clone());
        }
    }
    if set {
        memory.set(SET, ControlValue::Bool(true));
    }
    memory
}

/// The camera's XYZ to camera matrix, darktable's for its temperature and tint.
fn matrix(image: &ImageContext) -> Option<apertrust_color::Mat3> {
    image.camera.as_ref()?.xyz_to_camera
}

/// The matrices temperature and tint read the coefficients through: the camera's, or
/// sRGB's where the photo has none, as darktable assumes for an image that is not a raw.
fn matrices(image: &ImageContext) -> Matrices {
    matrix(image).map_or_else(Matrices::srgb, |m| Matrices::camera(&m))
}

/// darktable's `_find_coeffs` normalised to green, as `reload_defaults` keeps it in
/// `chr->as_shot`, held as the operation's defaults hold it: the camera's as-shot
/// multipliers, else daylight's from its matrix, else 2, 1, 1.5, the fourth the fourth
/// recorded or green's; each within the range, and 1 for one that is no number, as
/// darktable's fourth on a three-colour sensor, so a strange camera cannot stop planning.
fn as_shot_coefficients(camera: &Camera) -> [f32; 4] {
    let multipliers = apertrust_color::as_shot_multipliers(
        camera.white_balance.as_deref(),
        camera.xyz_to_camera.as_ref(),
    );
    multipliers.map(|c| {
        if c.is_finite() {
            c.clamp(0.0, 8.0) as f32
        } else {
            1.0
        }
    })
}

/// The as-shot coefficients `chr->as_shot` holds for `image`: every one 1 but on a
/// colour raw, as `reload_defaults` leaves them.
fn as_shot(image: &ImageContext) -> [f32; 4] {
    colour_raw(image).map_or([1.0; 4], as_shot_coefficients)
}

/// The camera's D65 coefficients, darktable's `chr->D65coeffs`, as
/// `_temp_params_from_array` stores them: from the colour matrix, the fourth 1, which
/// darktable's is not a number on a three-colour sensor; without a matrix every one 1,
/// `_calculate_bogus_daylight_wb`'s, darktable's camera Daylight preset being the one
/// fallback Apertrust lacks (`docs/tools/white-balance.md`, Gaps).
fn daylight(image: &ImageContext) -> [f32; 4] {
    match matrix(image) {
        Some(matrix) => {
            let [r, g, b] = apertrust_color::daylight_multipliers(&matrix);
            [r as f32, g as f32, b as f32, 1.0]
        }
        None => [1.0; 4],
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_plugin::ParamValues;

    use super::*;
    use crate::check;
    use crate::testing::{DarktablePicker, DarktableSlider};

    fn operation() -> &'static dyn apertrust_plugin::Operation {
        apertrust_operations::operation(OPERATION).unwrap()
    }

    /// The image contexts the operation's defaults tell apart: an image that is not a raw;
    /// colour raws with a white balance under the sample's matrix and another, with none,
    /// and with no matrix; and a monochrome raw.
    fn images() -> Vec<ImageContext> {
        let colour = crate::testing::image();
        let camera = colour.camera.clone().unwrap();
        let with = |camera: Camera| ImageContext {
            camera: Some(camera),
            ..colour.clone()
        };
        vec![
            ImageContext::full(8, 8),
            colour.clone(),
            with(Camera {
                white_balance: Some(vec![2.1, 1.0, 1.6]),
                xyz_to_camera: Some([[0.7, -0.2, -0.05], [-0.4, 1.2, 0.2], [-0.05, 0.15, 0.6]]),
                ..camera.clone()
            }),
            with(Camera {
                white_balance: None,
                ..camera.clone()
            }),
            with(Camera {
                xyz_to_camera: None,
                ..camera.clone()
            }),
            with(Camera {
                cfa: None,
                ..camera
            }),
        ]
    }

    fn bind(set: &[(&str, ControlValue)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ToolValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&WhiteBalance.defaults(image));
        WhiteBalance.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    /// A control's value as its parameter holds it.
    fn as_param(value: &ControlValue) -> Value {
        match value {
            ControlValue::Number(value) => Value::Float(*value),
            ControlValue::Choice(index) => Value::Choice(*index),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        WhiteBalance.definition().check().unwrap();
        let (accepted, rejected) = check::accept(&[&WhiteBalance], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&WhiteBalance, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn each_control_is_its_parameter_with_its_range_and_darktable_s_default() {
        let schema = operation().schema();
        for control in &CONTROLS {
            let found = schema.param(control.id).unwrap();
            match (control.kind, found.kind) {
                (
                    ControlKind::Number {
                        min, max, default, ..
                    },
                    apertrust_plugin::Kind::Float {
                        min: lowest,
                        max: highest,
                        default: its,
                    },
                ) => assert_eq!(
                    (min, max, default),
                    (lowest, highest, its),
                    "{}",
                    control.id
                ),
                (
                    ControlKind::Choice {
                        options, default, ..
                    },
                    apertrust_plugin::Kind::Choice {
                        default: its,
                        choices,
                    },
                ) => assert_eq!((options, default), (choices, its), "{}", control.id),
                other => panic!("{}: {other:?}", control.id),
            }
        }
    }

    #[test]
    fn its_defaults_are_the_operation_s_on_every_photo() {
        for image in images() {
            let resolved = operation()
                .schema()
                .resolve_over(&operation().defaults(&image), &ParamValues::new())
                .unwrap();
            let defaults = WhiteBalance.defaults(&image);
            for control in &CONTROLS {
                let default = as_param(defaults.get(control.id).unwrap());
                assert_eq!(resolved.get(control.id), Some(default), "{}", control.id);
            }
        }
    }

    /// darktable's white balance sliders (`src/iop/temperature.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/tools/white-balance.md`'s slider
    /// table.
    const DARKTABLE: [DarktableSlider; 5] = [
        // `scale_k`: `dt_bauhaus_slider_new_with_range_and_feedback(…, 1901, 25000, 0,
        // 5000, 0, …)`, formatted " K".
        DarktableSlider::new("temperature", (1901.0, 25000.0), 0),
        // `scale_tint`: 0.135 to 2.326, three digits.
        DarktableSlider::new("tint", (0.135, 2.326), 3),
        // `scale_r`, `scale_g`, `scale_b`, from params, `dt_bauhaus_slider_set_digits(…,
        // 3)`.
        DarktableSlider::new("red", (0.0, 8.0), 3),
        DarktableSlider::new("green", (0.0, 8.0), 3),
        DarktableSlider::new("blue", (0.0, 8.0), 3),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&WhiteBalance, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        for image in images() {
            assert!(bind(&[], &image).is_empty());
            let defaults = WhiteBalance.defaults(&image);
            let written: Vec<(&str, ControlValue)> = CONTROLS
                .iter()
                .map(|control| (control.id, defaults.get(control.id).unwrap().clone()))
                .collect();
            assert!(bind(&written, &image).is_empty());
        }
    }

    #[test]
    fn each_control_off_its_default_sets_its_parameter_alone() {
        let image = crate::testing::image();
        for (control, value) in [
            ("red", ControlValue::Number(2.5)),
            ("green", ControlValue::Number(1.1)),
            ("blue", ControlValue::Number(0.5)),
            ("preset", ControlValue::Choice(USER)),
        ] {
            assert_eq!(
                bind(&[(control, value.clone())], &image),
                [Setting::new(OPERATION, control, as_param(&value))],
                "{control}"
            );
        }
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation() {
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        for param in operation().schema().params {
            assert!(owned.contains(&param.name), "{}", param.name);
        }
        assert_eq!(owned.len(), operation().schema().params.len());
    }

    /// Settings' label compares with what the options and the defaults write, the as-shot
    /// coefficients held to the range, as darktable compares the parameters with the
    /// `as_shot` it wrote: a camera recording a red of 9 reads as shot to reference at its
    /// defaults, its red written as 8.
    #[test]
    fn the_label_reads_the_coefficients_as_written() {
        let mut image = crate::testing::image();
        image.camera.as_mut().unwrap().white_balance = Some(vec![9.0, 1.0, 1.5]);
        let defaults = WhiteBalance.defaults(&image);
        assert_eq!(defaults.get("red"), Some(&ControlValue::Number(8.0)));
        assert_eq!(
            WhiteBalance.matching(SETTINGS_ROW, &defaults, &image),
            Some("d65_late")
        );
        let as_shot = WhiteBalance
            .preset(SETTINGS_ROW, "as_shot", &image)
            .unwrap();
        assert_eq!(as_shot.get("red"), Some(&ControlValue::Number(8.0)));
    }

    /// darktable's picker, `dt_color_picker_new_with_cst(self, DT_COLOR_PICKER_AREA, NULL,
    /// IOP_CS_NONE)` (`temperature.c:2088`): an area, not denoised, the operation's input
    /// alone in no space of its own, so the mosaic, writing the coefficients and `preset`
    /// (`:1948-1969`) and reading no setting; Settings' from image area its one place.
    #[test]
    fn its_picker_is_darktable_s() {
        let darktable = [DarktablePicker {
            id: "spot",
            slider: "",
            kind: PickerKind::Area,
            denoise: false,
            output: false,
            writes: &["red", "green", "blue", "preset"],
            settings: &[],
        }];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
        let picker = DEFINITION.picker("spot").unwrap();
        assert_eq!(picker.operation, OPERATION);
        assert_eq!(picker.space, Space::Operation);
        assert!(!picker.keep);
        let row = DEFINITION.preset(SETTINGS_ROW).unwrap();
        assert_eq!(row.pickers, [("spot", "spot")]);
        assert!(DEFINITION.rows().all(|row| row.pickers().is_empty()));
    }

    // The pick: darktable's `color_picker_apply` (`temperature.c:1948-1969`) restated on
    // readings made for the test.

    /// A reading whose mean at the operation's input is `m`, one channel per CFA colour,
    /// the fourth empty as on a three-colour mosaic.
    fn reading(m: [f32; 3]) -> Reading {
        let [r, g, b] = m;
        Reading {
            input: crate::pick::Stats {
                mean: [r, g, b, 0.0],
                min: [r, g, b, f32::MAX],
                max: [r, g, b, -f32::MAX],
            },
            output: None,
            input_maximum: [1.0; 3],
        }
    }

    fn pick_by(picker: &str, m: [f32; 3]) -> Option<Picked> {
        let image = crate::testing::image();
        WhiteBalance.pick(
            picker,
            &reading(m),
            &WhiteBalance.defaults(&image),
            &image,
            &[],
            None,
            None,
            PickKeys::default(),
        )
    }

    /// The coefficients the pick of `m` writes, and its `preset`.
    fn picked(m: [f32; 3]) -> ([f32; 3], ControlValue) {
        let writes = pick_by("spot", m).unwrap().writes;
        (
            COEFFICIENTS.map(|control| writes.number(control)),
            writes.get("preset").unwrap().clone(),
        )
    }

    /// darktable's arithmetic, restated: `gnormal`, then each channel's inverse over it,
    /// held to 0 to 8, then green 1.
    fn darktable(m: [f32; 3]) -> [f32; 3] {
        let gnormal = if m[1] > 0.001 { 1.0 / m[1] } else { 1.0 };
        let mut coefficients = m.map(|mean| {
            let inverse: f32 = if mean > 0.001 { 1.0 / mean } else { 1.0 };
            (inverse / gnormal).clamp(0.0, 8.0)
        });
        coefficients[1] = 1.0;
        coefficients
    }

    #[test]
    fn a_pick_is_darktable_s_color_picker_apply() {
        let spot = ControlValue::Choice(SPOT);
        for m in [
            // 2, 1 and 1.333….
            [0.2, 0.4, 0.3],
            // Red below the floor: 1 over green's 1 / 0.4, so 0.4.
            [0.0005, 0.4, 0.3],
            // Green at and below the floor normalises by 1.
            [0.2, 0.001, 0.3],
            [0.2, 0.0002, 0.3],
            // A coefficient above 8 is held at 8.
            [0.01, 0.4, 0.3],
        ] {
            assert_eq!(picked(m), (darktable(m), spot.clone()), "{m:?}");
        }
        let (coefficients, _) = picked([0.2, 0.4, 0.3]);
        assert_eq!(coefficients[..2], [2.0, 1.0]);
        assert!(
            (coefficients[2] - 4.0 / 3.0).abs() < 1e-6,
            "{coefficients:?}"
        );
        assert_eq!(picked([0.0005, 0.4, 0.3]).0[0], 0.4);
        assert_eq!(picked([0.2, 0.001, 0.3]).0, [5.0, 1.0, 1.0 / 0.3]);
        assert_eq!(picked([0.01, 0.4, 0.3]).0[0], 8.0);
        // Emerald is not written, and nothing is shown.
        let all = pick_by("spot", [0.2, 0.4, 0.3]).unwrap();
        let written: Vec<&str> = all.writes.iter().map(|(control, _)| control).collect();
        assert_eq!(written, ["blue", "green", "preset", "red"]);
        assert_eq!(all.shown, Shown::default());
        // Another picker's id, or an area with no red photosite, gives nothing.
        assert_eq!(pick_by("exposure", [0.2, 0.4, 0.3]), None);
        let mut empty = reading([0.0, 0.4, 0.3]);
        (empty.input.min[0], empty.input.max[0]) = (f32::MAX, -f32::MAX);
        let image = crate::testing::image();
        let values = WhiteBalance.defaults(&image);
        let none = WhiteBalance.pick(
            "spot",
            &empty,
            &values,
            &image,
            &[],
            None,
            None,
            PickKeys::default(),
        );
        assert_eq!(none, None);
    }

    /// The coefficients a pick of `m` writes over the photo's defaults, `preset` with them.
    fn picked_values(image: &ImageContext, m: [f32; 3]) -> Controls {
        let mut values = WhiteBalance.defaults(image);
        for (control, value) in pick_by("spot", m).unwrap().writes.iter() {
            values.set(control, value.clone());
        }
        values
    }

    #[test]
    fn settings_reads_from_image_area_while_preset_holds_it_whatever_the_coefficients() {
        let image = crate::testing::image();
        let picked = picked_values(&image, [0.2, 0.4, 0.3]);
        assert_eq!(
            WhiteBalance.matching(SETTINGS_ROW, &picked, &image),
            Some("spot")
        );
        // The as-shot coefficients under from image area still read from image area.
        let mut as_shot = WhiteBalance.defaults(&image);
        as_shot.set("preset", ControlValue::Choice(SPOT));
        assert_eq!(
            WhiteBalance.matching(SETTINGS_ROW, &as_shot, &image),
            Some("spot")
        );
    }

    /// Redrawn, from image area becomes what the coefficients match: user modified for a
    /// picked balance, as shot for the as-shot coefficients, camera reference for the D65
    /// ones; any other setting is left as it is.
    #[test]
    fn redrawn_from_image_area_becomes_what_the_coefficients_match() {
        let image = crate::testing::image();
        let preset = |values: &Controls| WhiteBalance.redrawn(values, &image);
        let only = |option: u32| {
            let mut writes = Controls::new();
            writes.set("preset", ControlValue::Choice(option));
            writes
        };
        assert_eq!(preset(&picked_values(&image, [0.2, 0.4, 0.3])), only(USER));
        let mut as_shot = WhiteBalance.defaults(&image);
        as_shot.set("preset", ControlValue::Choice(SPOT));
        assert_eq!(preset(&as_shot), only(AS_SHOT));
        let mut daylight = as_shot.clone();
        for (control, value) in WhiteBalance
            .preset(SETTINGS_ROW, "d65", &image)
            .unwrap()
            .iter()
        {
            if control != "preset" {
                daylight.set(control, value.clone());
            }
        }
        assert_eq!(preset(&daylight), only(D65));
        for setting in [AS_SHOT, USER, D65, D65_LATE] {
            let mut values = picked_values(&image, [0.2, 0.4, 0.3]);
            values.set("preset", ControlValue::Choice(setting));
            assert!(preset(&values).is_empty(), "{setting}");
        }
    }

    /// darktable's `mod_coeff` (`temperature.c:1344-1350`, `:1655`, `:1720-1742`): the
    /// defaults' four as the photo opens, not set; redrawn, the four held where the label
    /// reads user modified or they are set; a coefficient, Emerald, temperature or tint
    /// moved, the four held, set; anything else leaves it. User modified writes it back
    /// (`:1824-1827`), and no other option recalls.
    #[test]
    fn user_modified_remembers_and_writes_back_darktable_s_mod_coeff() {
        let image = crate::testing::image();
        let defaults = WhiteBalance.defaults(&image);
        let opened = WhiteBalance.remember(Moment::Opened, &Controls::new(), &defaults, &image);
        assert_eq!(opened, four(&defaults, false));
        // Redrawn at the defaults, as shot to reference: not set, so left.
        let redrawn = WhiteBalance.remember(Moment::Redrawn, &opened, &defaults, &image);
        assert_eq!(redrawn, opened);
        // Red moved: the four, set.
        let mut moved = defaults.clone();
        moved.set("red", ControlValue::Number(2.5));
        moved.set("preset", ControlValue::Choice(USER));
        for control in ["red", "green", "blue", "various", "temperature", "tint"] {
            let after = WhiteBalance.remember(Moment::Moved(control), &opened, &moved, &image);
            assert_eq!(after, four(&moved, true), "{control}");
        }
        for control in ["preset", "finetune"] {
            let after = WhiteBalance.remember(Moment::Moved(control), &opened, &moved, &image);
            assert_eq!(after, opened, "{control}");
        }
        // Redrawn reading user modified, or set already: the four the controls hold.
        assert_eq!(
            WhiteBalance.remember(Moment::Redrawn, &opened, &moved, &image),
            four(&moved, true)
        );
        let set = four(&moved, true);
        assert_eq!(
            WhiteBalance.remember(Moment::Redrawn, &set, &defaults, &image),
            four(&defaults, true)
        );
        // User modified writes the four back with its setting.
        let recalled = WhiteBalance
            .recall(SETTINGS_ROW, "user", &set, &image)
            .unwrap();
        for control in FOUR {
            assert_eq!(recalled.get(control), moved.get(control), "{control}");
        }
        assert_eq!(recalled.get("preset"), Some(&ControlValue::Choice(USER)));
        assert_eq!(recalled.get(SET), None);
        for option in ["as_shot", "spot", "d65", "d65_late"] {
            assert_eq!(
                WhiteBalance.recall(SETTINGS_ROW, option, &set, &image),
                None
            );
        }
        assert_eq!(
            WhiteBalance.recall(SETTINGS_ROW, "user", &Controls::new(), &image),
            None
        );
    }

    #[test]
    fn a_coefficient_writes_user_modified_and_the_setting_writes_nothing() {
        let image = crate::testing::image();
        let values = WhiteBalance.defaults(&image);
        for control in COEFFICIENTS {
            let writes = WhiteBalance.follow(control, &values, &image);
            assert_eq!(writes.get("preset"), Some(&ControlValue::Choice(USER)));
            assert_eq!(writes.iter().count(), 1);
        }
        assert!(WhiteBalance.follow("preset", &values, &image).is_empty());
    }
}
