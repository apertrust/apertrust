// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/exposure.c)
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

//! Exposure (`docs/modules/exposure.md`): darktable's exposure module as the user
//! edits with it, in the first pass (decision 2): darktable's page, its controls in
//! `gui_init`'s order under its labels, with no tabs.
//!
//! - **Mode:** manual, or automatic, darktable's deflicker, which needs the raw histogram
//!   and is not ported, so it is listed, drawn disabled and refused.
//! - **In manual mode:** Compensate camera exposure, whether the camera's exposure
//!   compensation is undone; Highlight preservation mode, not ported; and Exposure,
//!   darktable's slider, `exposure.exposure` itself, −18 to +18 EV, its default the
//!   photo's: +0.7 EV on a colour raw, 0 otherwise (decision 30).
//! - **In automatic mode:** Percentile and Target level, not ported. darktable's computed
//!   EC readout waits for a row that draws a readout.
//! - **Black level correction,** darktable's offset over the photo's own black.
//! - **Area exposure mapping,** a collapsible group of darktable's settings for its
//!   picker, which no edit stores, kept in settings ([`crate::tools`]), and the area's
//!   input and target patches with the input's L readout.
//! - **Its picker,** on Exposure's quad: darktable's `_auto_set_exposure`, which sets the
//!   exposure that maps the area to the target lightness, or measures the target the area
//!   gives.
//!
//! It drives `exposure`, which every photo runs, and owns every parameter of it. The
//! controls whose parameters are not ported are marked so (`Control::unported`): drawn
//! disabled, they take only their defaults, the only values the operation renders, as
//! Mode takes only manual. Contrast and its pivot, which it once bound through colour
//! balance rgb, are the Colour balance RGB module's `contrast` and `grey_fulcrum`
//! (decision 2), and Brightness that module's `midtones_y`.

use apertrust_base::settings::Choice;
use apertrust_color::{
    Mat3, PROFILE_D65, REC2020_PRIMARIES, apply, camera_to_xyz_d50, lab_to_xyz_d50,
    profile_rgb_to_xyz_d50, to_f32, xyz_to_lab_d50,
};
use apertrust_plugin::{ImageContext, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Definition, Picker, PickerKind, Quad, Row, Slider, Space,
    Tab, ToolSetting, Unit,
};
use crate::pick::{Patch, PickKeys, Picked, Reading, Shown};
use crate::transfer::Transfer;
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct Exposure;

/// The operation it drives.
const EXPOSURE: &str = "exposure";

/// darktable's modes, `dt_iop_exposure_mode_t`, as the operation's choices.
const MODES: [&str; 2] = ["manual", "automatic"];

/// The controls, in the page's order. Exposure, Black level correction and Compensate
/// camera exposure keep their first ids; the others are darktable's parameter names.
static CONTROLS: [Control; 7] = [
    Control {
        id: "mode",
        kind: ControlKind::Choice {
            options: &MODES,
            default: 0,
            unported: &["automatic"],
        },
        unit: None,
        unported: false,
    },
    Control {
        id: "camera_bias",
        // A raw's; an image that is not one has no bias to undo (`defaults`).
        kind: ControlKind::Bool { default: true },
        unit: None,
        unported: false,
    },
    Control {
        id: "compensate_hilite_pres",
        // Off, as the operation leaves it (decisions 21 and 62); darktable turns it on
        // for a new instance.
        kind: ControlKind::Bool { default: false },
        unit: None,
        unported: true,
    },
    Control {
        id: "exposure",
        // darktable's slider (decision 30): the parameter itself, over its range, ±18 EV,
        // the step darktable's over its soft range, −3 to +4 EV, which the track spans.
        // The default is the photo's ([`default_exposure`]); 0 is an image's that is not
        // a raw.
        kind: ControlKind::Number {
            min: -18.0,
            max: 18.0,
            step: 0.05,
            default: 0.0,
        },
        unit: Some(Unit::Ev),
        unported: false,
    },
    // darktable's ranges and defaults, and the steps its sliders take.
    Control {
        id: "deflicker_percentile",
        kind: ControlKind::Number {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: 50.0,
        },
        unit: Some(Unit::Percent),
        unported: true,
    },
    Control {
        id: "deflicker_target_level",
        kind: ControlKind::Number {
            min: -18.0,
            max: 18.0,
            step: 0.1,
            default: -4.0,
        },
        unit: Some(Unit::Ev),
        unported: true,
    },
    // An offset over the photo's own black (decision 22), over darktable's range, ±1, the
    // track over its soft range, ±0.1 (decision 30). [`BlackLevel`] keeps the black
    // inside darktable's ±1 and below the white the photo renders.
    Control {
        id: "black_level",
        kind: ControlKind::Number {
            min: -1.0,
            max: 1.0,
            step: 0.001,
            default: 0.0,
        },
        unit: None,
        unported: false,
    },
];

/// darktable's area exposure mapping, under the page: what its picker does with the area
/// picked and the lightness it maps it to, darktable's settings (an area mode it puts
/// back to correction at every `gui_update`, `exposure.c:742`, and
/// `darkroom/modules/exposure/lightness` in its darktablerc), at their defaults there,
/// both kept in settings (`tools.exposure.*`), the area mode's keeping Apertrust's; between
/// them the input's and the target's patches, with the input's L readout.
const MAPPING: Row = Row::Group {
    id: "area_exposure_mapping",
    rows: &[
        Row::ToolSetting(ToolSetting {
            id: "area_mode",
            kind: ControlKind::Choice {
                options: &AREA_MODES,
                default: 0,
                unported: &[],
            },
            decimals: 0,
        }),
        Row::Patches {
            picker: "exposure",
            target: &["lightness"],
        },
        Row::ToolSetting(ToolSetting {
            id: "lightness",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 50.0,
            },
            decimals: 1,
        }),
    ],
};

/// darktable's area modes, `dt_spot_mode_t`'s two that Exposure lists, in its order.
const AREA_MODES: [&str; 2] = ["correction", "measure"];

/// The area exposure mapping's mode, the tool setting `tools.exposure.area_mode`:
/// darktable's `dt_spot_mode_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AreaMode {
    /// The exposure that maps the area to the target lightness.
    #[default]
    Correction,
    /// The target lightness the area gives at the current exposure.
    Measure,
}

impl Choice for AreaMode {
    const OPTIONS: &'static [(Self, &'static str)] = &[
        (Self::Correction, AREA_MODES[0]),
        (Self::Measure, AREA_MODES[1]),
    ];
}

static DEFINITION: Definition = Definition {
    id: "exposure",
    version: 2,
    layers: true,
    auto: true,
    switched: false,
    controls: &CONTROLS,
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Choice {
                control: "mode",
                style: ChoiceStyle::List,
            },
            Row::When {
                control: "mode",
                is: &["manual"],
                rows: &[
                    Row::Toggle {
                        control: "camera_bias",
                    },
                    Row::Toggle {
                        control: "compensate_hilite_pres",
                    },
                    Row::Slider(
                        Slider::new("exposure")
                            .decimals(3)
                            .soft(-3.0, 4.0)
                            .quad(Quad::Picker("exposure")),
                    ),
                ],
            },
            Row::When {
                control: "mode",
                is: &["automatic"],
                rows: &[
                    Row::Slider(Slider::new("deflicker_percentile")),
                    Row::Slider(Slider::new("deflicker_target_level")),
                ],
            },
            Row::Slider(Slider::new("black_level").decimals(4).soft(-0.1, 0.1)),
            MAPPING,
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
    pickers: &[PICKER],
};

/// darktable's picker on Exposure (`DT_COLOR_PICKER_AREA`): the exposure that maps the
/// area to the target lightness, the black kept below the new white, or in measure mode
/// the target the area gives; its pick comes with the area exposure mapping's settings.
const PICKER: Picker = Picker {
    id: "exposure",
    operation: EXPOSURE,
    kind: PickerKind::Area,
    denoise: false,
    output: false,
    space: Space::Operation,
    keep: false,
    writes: &["exposure", "black_level"],
    tools: &["area_mode", "lightness"],
};

/// Every parameter of `exposure`: the module is darktable's and owns them all.
static OWNS: [Owned; 7] = [
    Owned::new(EXPOSURE, "mode"),
    Owned::new(EXPOSURE, "black"),
    Owned::new(EXPOSURE, "exposure"),
    Owned::new(EXPOSURE, "deflicker_percentile"),
    Owned::new(EXPOSURE, "deflicker_target_level"),
    Owned::new(EXPOSURE, "compensate_exposure_bias"),
    Owned::new(EXPOSURE, "compensate_hilite_pres"),
];

/// The exposure operation's limits: its schema's range for `black`, darktable's, and the
/// camera bias darktable's `_get_exposure_bias` trusts.
const BLACK_LIMIT: f32 = 1.0;
const BIAS_LIMIT: f32 = 5.0;

/// How far below the white the photo renders Black level puts a black that reaches it:
/// darktable's margin, which its `gui_changed` restores (decision 30).
const WHITE_MARGIN: f32 = 0.01;

/// darktable's floor under the white it takes the logarithm of (`white2exposure`).
const WHITE_FLOOR: f32 = 1e-20;

/// Where version 2's migration moves version 1's Exposure, which this version lacks, so
/// the reader leaves it out with a warning.
const STOPS: &str = "exposure_in_stops";

impl Module for Exposure {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    /// The definition's, but Exposure is the photo's, as the exposure operation's default
    /// is ([`default_exposure`]), and Compensate camera exposure is off for an image that
    /// is not a raw, as the operation's own default is (`Operation::defaults`).
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        defaults.set("camera_bias", ControlValue::Bool(image.camera.is_some()));
        defaults.set("exposure", ControlValue::Number(default_exposure(image)));
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Compensate camera exposure does nothing on a photo that records no bias, and Mode
    /// nothing on an image that is not a raw, where darktable makes it insensitive
    /// (`gui_update`); neither depends on the module's values. The controls not ported
    /// are unavailable everywhere, as the session reports every such control.
    fn available(&self, control: &str, _: &Controls, image: &ImageContext) -> bool {
        match control {
            "camera_bias" => camera_bias(image) != 0.0,
            "mode" => image.camera.is_some(),
            _ => true,
        }
    }

    /// Mode and the controls not ported take only their defaults, so this sets only
    /// `exposure`, `black` and `compensate_exposure_bias`. Exposure is written as it is,
    /// as darktable's slider writes it; the black, only while Black level is set, since
    /// the photo's own black lies below any white.
    fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
        let mut settings = Vec::new();
        let compensated = match values.flag("camera_bias") {
            Some(on) => {
                settings.push(Setting::new(
                    EXPOSURE,
                    "compensate_exposure_bias",
                    Value::Bool(on),
                ));
                on
            }
            None => base.flag(EXPOSURE, "compensate_exposure_bias"),
        };
        let bias = if compensated { camera_bias(image) } else { 0.0 };
        let exposure = values
            .number("exposure")
            .unwrap_or_else(|| base.float(EXPOSURE, "exposure"));
        if let Some(level) = values.number("black_level") {
            let white = rendered_white(exposure, bias);
            let black = BlackLevel::new(base.float(EXPOSURE, "black"), white).value(level);
            settings.push(Setting::new(EXPOSURE, "black", Value::Float(black)));
        }
        if values.number("exposure").is_some() {
            settings.push(Setting::new(EXPOSURE, "exposure", Value::Float(exposure)));
        }
        settings
    }

    /// Version 1's Exposure solved for exact stops through the photo's black, so its value
    /// means another exposure on every photo: it moves out of the way and is left out
    /// with a warning, its mark with it, Exposure back at the photo's default. No released
    /// build wrote it. Black level correction and Compensate camera exposure mean what
    /// they meant; Contrast and Pivot, an older version 1's, are left out as before.
    fn migrate(&self, from: u32, mut values: ModuleValues) -> ModuleValues {
        if from < 2
            && let Some(stops) = values.remove("exposure")
        {
            values.set(STOPS, stops);
            values.set_mark("exposure", None);
        }
        values
    }

    /// darktable's `color_picker_apply`, its `_auto_set_exposure` (`exposure.c:866-965`),
    /// on the area's mean at Exposure's input, read through the input profile's matrix to
    /// XYZ D50 ([`input_profile`]) and made grey of its lightness: the input patch and its
    /// L, then by the area mode, `tools.exposure.area_mode`:
    ///
    /// - **Correction:** the exposure `−log2(Y / Y_target)`, `Y_target` the grey of the
    ///   target lightness, plus the camera's bias where Compensate camera exposure is on,
    ///   which the operation takes off again, so the area renders at the target. Nothing is
    ///   written where it equals Exposure's value, as `_exposure_set_white` returns; a
    ///   black at or above the new white goes 0.01 below it (`:763-779`), written as Black
    ///   level correction over the photo's own black. darktable bounds neither: the session
    ///   holds them to their ranges, and leaves out a target lightness of 0's, which is
    ///   not finite, showing the patch still.
    /// - **Measure:** no control; the target lightness the area renders at at the current
    ///   exposure, held to the setting's 0 to 100 as darktable's slider holds it, which the
    ///   client keeps (`Shown::tools`).
    ///
    /// Highlight preservation mode, not ported, is never on, so its bias is never added.
    /// Nothing on a colour raw with no matrix, which colorin cannot render, as darktable
    /// returns with no input profile.
    fn pick(
        &self,
        picker: &str,
        reading: &Reading,
        values: &Controls,
        image: &ImageContext,
        tools: &[(&str, ControlValue)],
        _channel: Option<&str>,
        _last: Option<&Reading>,
        _keys: PickKeys,
    ) -> Option<Picked> {
        if picker != PICKER.id {
            return None;
        }
        let [r, g, b, _] = reading.input.mean;
        let xyz = apply(&input_profile(image)?, [r, g, b]);
        // darktable makes it grey to keep only its lightness.
        let lightness = xyz_to_lab_d50(xyz)[0];
        let grey = lab_to_xyz_d50([lightness, 0.0, 0.0]);
        let mut shown = Shown {
            patch: Some(Patch {
                srgb: apertrust_color::xyz_d50_to_srgb(grey).map(|c| c.clamp(0.0, 1.0)),
                lch: [lightness, 0.0, 0.0],
            }),
            tools: Vec::new(),
        };
        let tool = |id: &str| tools.iter().find(|(tool, _)| *tool == id).map(|(_, v)| v);
        let mode = match tool("area_mode") {
            Some(&ControlValue::Choice(1)) => AreaMode::Measure,
            _ => AreaMode::Correction,
        };
        let exposure = values.number("exposure");
        let bias = if values.flag("camera_bias") {
            camera_bias(image)
        } else {
            0.0
        };
        let mut writes = Controls::new();
        match mode {
            AreaMode::Measure => {
                // The exposure the operation applies; `exposure2white(-expo)` its factor.
                let white = exposure_to_white(-(exposure - bias));
                let out = xyz_to_lab_d50(grey.map(|c| c * white))[0];
                let out = ControlValue::Number(out.clamp(0.0, 100.0));
                shown.tools.push(("lightness", out));
            }
            AreaMode::Correction => {
                let target = match tool("lightness") {
                    Some(&ControlValue::Number(target)) => target,
                    _ => 50.0,
                };
                let target = lab_to_xyz_d50([target, 0.0, 0.0]);
                let expo = -white_to_exposure(grey[1] / target[1]) - bias;
                // `_exposure_set_white`.
                let white = exposure_to_white(-expo);
                let picked = white_to_exposure(white);
                if picked != exposure {
                    writes.set("exposure", ControlValue::Number(picked));
                    // The black the operation holds now, darktable's `p->black`.
                    let default = default_black(image);
                    let level = values.number("black_level");
                    let held = BlackLevel::new(default, rendered_white(exposure, bias));
                    if held.value(level) >= white {
                        let level = white - WHITE_MARGIN - default;
                        writes.set("black_level", ControlValue::Number(level));
                    }
                }
            }
        }
        Some(Picked { writes, shown })
    }
}

/// darktable's `exposure2white`: the input an exposure renders at 1.0, the camera's bias
/// aside.
fn exposure_to_white(exposure: f32) -> f32 {
    (-exposure).exp2()
}

/// darktable's `white2exposure`: the exposure that renders `white` at 1.0, the white
/// floored at 1e-20.
fn white_to_exposure(white: f32) -> f32 {
    -white.max(WHITE_FLOOR).log2()
}

/// The input profile's camera RGB to XYZ D50, darktable's `matrix_in` of the pipe's input
/// profile, through which Exposure's picker reads its area: colorin's default for the
/// photo, which no module changes, the camera's matrix on a colour raw (darktable's
/// standard matrix) and linear Rec2020 otherwise. This crate cannot call colorin, so its
/// rule is repeated here; a test holds the two to agree. `None` on a colour raw with no
/// matrix, which colorin cannot render.
pub fn input_profile(image: &ImageContext) -> Option<Mat3> {
    match &image.camera {
        Some(camera) if camera.cfa.is_some() => {
            Some(to_f32(camera_to_xyz_d50(camera.xyz_to_camera.as_ref()?)))
        }
        _ => Some(to_f32(profile_rgb_to_xyz_d50(
            PROFILE_D65,
            REC2020_PRIMARIES,
        ))),
    }
}

/// The photo's own black, the exposure operation's default `black`: darktable's −2⁻¹² on a
/// colour raw, 0 otherwise, as its `reload_defaults` sets it. This crate cannot call the
/// operation, so its rule is repeated here; a test holds the two to agree.
pub fn default_black(image: &ImageContext) -> f32 {
    match &image.camera {
        Some(camera) if camera.cfa.is_some() => -1.0 / 4096.0,
        _ => 0.0,
    }
}

/// darktable's default exposure for the photo, its `reload_defaults` in the scene-referred
/// workflow: +0.7 EV on a colour raw, 0 on a monochrome one or an image that is not a
/// raw. The exposure operation's own default; this crate cannot call it, so the rule is
/// repeated here and the module's tests check the two agree.
pub fn default_exposure(image: &ImageContext) -> f32 {
    match &image.camera {
        Some(camera) if camera.cfa.is_some() => 0.7,
        _ => 0.0,
    }
}

/// The camera's exposure bias as the exposure operation compensates it: within ±5 EV,
/// as darktable's `_get_exposure_bias` trusts it. The image context carries it
/// unclamped, and this crate cannot call the operation's own, so the clamp is repeated
/// here; the module's tests check the two agree.
pub fn camera_bias(image: &ImageContext) -> f32 {
    image.camera.as_ref().map_or(0.0, |camera| {
        camera.exposure_bias.clamp(-BIAS_LIMIT, BIAS_LIMIT)
    })
}

/// w = 2^−(e − β): the input the exposure `e` renders at 1.0, with the camera's bias `β`
/// compensated (0 when it is not), the white the exposure operation divides by.
pub fn rendered_white(exposure: f32, bias: f32) -> f32 {
    (-(f64::from(exposure) - f64::from(bias))).exp2() as f32
}

/// Black level's black: the photo's own, `b₀` (−2⁻¹² for a colour raw), plus the
/// control's value, inside the operation's range, darktable's ±1; one that reaches the
/// white the photo renders, `w`, is put 0.01 below it, darktable's margin, as darktable's
/// `gui_changed` puts it, so the kernel never divides by zero (decision 30). darktable
/// compares with the white of its slider's exposure; this one includes the camera's bias,
/// the white the kernel divides by. Rising with a slope of 1 below `w`, flat at ±1, and
/// falling there by darktable's margin: on a colour raw the control's bottom 2⁻¹² holds
/// the black at −1, darktable's lowest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackLevel {
    /// b₀, the photo's default black.
    pub default: f32,
    /// w, the input the photo renders at 1.0 ([`rendered_white`]).
    pub white: f32,
}

impl BlackLevel {
    pub fn new(default: f32, white: f32) -> Self {
        Self { default, white }
    }

    /// The black as set, before darktable's margin is restored.
    fn set(&self, level: f32) -> f32 {
        (self.default + level).clamp(-BLACK_LIMIT, BLACK_LIMIT)
    }
}

impl Transfer for BlackLevel {
    fn value(&self, level: f32) -> f32 {
        let black = self.set(level);
        if black >= self.white {
            self.white - WHITE_MARGIN
        } else {
            black
        }
    }

    fn derivative(&self, level: f32) -> f32 {
        let black = self.default + level;
        if black > -BLACK_LIMIT && black < BLACK_LIMIT && black < self.white {
            1.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_plugin::Camera;

    use super::*;
    use crate::check;
    use crate::testing::{DarktablePicker, DarktableSlider};
    use crate::transfer::{self, increasing};

    /// A colour raw's context, with this exposure bias.
    fn raw(exposure_bias: f32) -> ImageContext {
        ImageContext {
            camera: Some(Camera {
                exposure_bias,
                ..crate::testing::image().camera.unwrap()
            }),
            ..crate::testing::image()
        }
    }

    /// Whether `control` can change anything on `image`, every control at its default
    /// for it, as the session asks of an untouched photo; and the same with other values,
    /// since Exposure's availability reads the photo alone.
    fn available(control: &str, image: &ImageContext) -> bool {
        let defaults = Exposure.defaults(image);
        let answer = Exposure.available(control, &defaults, image);
        let mut moved = defaults;
        moved.set("camera_bias", ControlValue::Bool(false));
        moved.set("exposure", ControlValue::Number(2.0));
        assert_eq!(
            Exposure.available(control, &moved, image),
            answer,
            "{control}"
        );
        answer
    }

    /// What the binding sets for these values, as a session binds them.
    fn bind(set: &[(&str, ControlValue)], image: &ImageContext) -> Vec<Setting> {
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&Exposure.defaults(image));
        Exposure.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    fn number(value: f32) -> ControlValue {
        ControlValue::Number(value)
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        Exposure.definition().check().unwrap();
        let (accepted, rejected) = check::accept(&[&Exposure], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [
            raw(0.0),
            raw(-0.67),
            raw(-8.0),
            raw(5.0),
            ImageContext::full(8, 8),
        ] {
            let problems = check::sweep(&Exposure, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn it_owns_every_parameter_of_the_exposure_operation_and_no_other() {
        // Contrast and its pivot left for Colour balance RGB (decision 2).
        for control in ["contrast", "pivot"] {
            assert!(
                Exposure.definition().control(control).is_none(),
                "{control}"
            );
        }
        let schema = apertrust_operations::operation(EXPOSURE).unwrap().schema();
        let owned: Vec<&str> = Exposure.owns().iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned, params);
        assert!(
            Exposure
                .owns()
                .iter()
                .all(|owned| owned.operation == EXPOSURE)
        );
    }

    /// The controls each row shows, in the page's order, rows inside modes and groups
    /// included.
    fn page() -> Vec<&'static str> {
        DEFINITION
            .rows()
            .flat_map(|row| row.controls().iter().copied())
            .collect()
    }

    #[test]
    fn its_page_is_darktable_s_in_gui_init_s_order_on_one_untitled_tab() {
        assert_eq!(DEFINITION.tabs.len(), 1);
        assert_eq!(DEFINITION.tabs[0].id, None);
        assert_eq!(
            page(),
            [
                "mode",
                "camera_bias",
                "compensate_hilite_pres",
                "exposure",
                "deflicker_percentile",
                "deflicker_target_level",
                "black_level",
            ]
        );
        // The controls are listed in the same order.
        let controls: Vec<&str> = DEFINITION.controls.iter().map(|c| c.id).collect();
        assert_eq!(controls, page());
        // The area exposure mapping's settings are the picker's, which no edit stores.
        let settings: Vec<&str> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::ToolSetting(setting) => Some(setting.id),
                _ => None,
            })
            .collect();
        assert_eq!(settings, ["area_mode", "lightness"]);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = raw(-0.67);
        assert!(
            Exposure
                .bind(&ModuleValues::new(), &image, &Base::new(OPERATIONS, &image))
                .is_empty()
        );
        // Every control written at its default is still nothing to set.
        let defaults = [
            ("mode", ControlValue::Choice(0)),
            ("camera_bias", ControlValue::Bool(true)),
            ("compensate_hilite_pres", ControlValue::Bool(false)),
            ("exposure", number(0.7)),
            ("deflicker_percentile", number(50.0)),
            ("deflicker_target_level", number(-4.0)),
            ("black_level", number(0.0)),
        ];
        assert!(bind(&defaults, &image).is_empty());
        // And each is the operation's default for a colour raw.
        let base = Base::new(OPERATIONS, &image);
        assert_eq!(base.get(EXPOSURE, "mode"), Some(Value::Choice(0)));
        assert!(!base.flag(EXPOSURE, "compensate_hilite_pres"));
        assert_eq!(base.float(EXPOSURE, "deflicker_percentile"), 50.0);
        assert_eq!(base.float(EXPOSURE, "deflicker_target_level"), -4.0);
    }

    #[test]
    fn compensate_camera_exposure_is_off_by_default_only_for_an_image_that_is_not_a_raw() {
        let flag = |image: &ImageContext| Exposure.defaults(image).flag("camera_bias");
        assert!(flag(&raw(0.0)));
        assert!(!flag(&ImageContext::full(8, 8)));
        assert!(!available("camera_bias", &raw(0.0)));
        assert!(available("camera_bias", &raw(-0.67)));
        assert!(available("exposure", &raw(0.0)));
        assert!(available("black_level", &raw(0.0)));
    }

    #[test]
    fn what_is_not_ported_takes_only_its_default() {
        // Automatic, darktable's deflicker, is listed and refused.
        let mode = Exposure.definition().control("mode").unwrap();
        assert!(mode.check(&ControlValue::Choice(1)).is_err());
        assert!(mode.check(&ControlValue::Choice(0)).is_ok());
        // Mode is live on a raw, as darktable's is, and not on another image.
        assert!(available("mode", &raw(-0.67)));
        assert!(!available("mode", &ImageContext::full(8, 8)));
        // Highlight preservation mode, Percentile and Target level take their defaults
        // only, as automatic is refused.
        let unported: Vec<&str> = DEFINITION
            .controls
            .iter()
            .filter(|control| control.unported)
            .map(|control| control.id)
            .collect();
        assert_eq!(
            unported,
            [
                "compensate_hilite_pres",
                "deflicker_percentile",
                "deflicker_target_level"
            ]
        );
        for (control, value) in [
            ("compensate_hilite_pres", ControlValue::Bool(true)),
            ("deflicker_percentile", number(0.0)),
            ("deflicker_percentile", number(100.0)),
            ("deflicker_target_level", number(-18.0)),
            ("deflicker_target_level", number(18.0)),
        ] {
            let control = DEFINITION.control(control).unwrap();
            assert_eq!(
                control.check(&value),
                Err(crate::ControlError::NotPorted {
                    control: control.id
                })
            );
            assert_eq!(control.check(&control.default()), Ok(()));
        }
    }

    /// darktable's exposure sliders (`src/iop/exposure.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/modules/exposure.md`'s slider table.
    /// None shows a factor; Percentile's "%" format gives none, its range above ±10.
    const DARKTABLE: [DarktableSlider; 5] = [
        DarktableSlider::new("exposure", (-18.0, 18.0), 3).soft(-3.0, 4.0),
        DarktableSlider::new("deflicker_percentile", (0.0, 100.0), 2),
        DarktableSlider::new("deflicker_target_level", (-18.0, 18.0), 2),
        DarktableSlider::new("black_level", (-1.0, 1.0), 4).soft(-0.1, 0.1),
        // Area exposure mapping's lightness, `dt_bauhaus_slider_new_with_range`.
        DarktableSlider::new("lightness", (0.0, 100.0), 1),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        // Black level correction holds darktable's ±1 as an offset over the photo's black,
        // where darktable's slider shows the black itself.
        let differences =
            crate::testing::slider_differences(&Exposure, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
        // A tenth of Black level correction's step is darktable's finest, 0.0001.
        let black = DEFINITION.control("black_level").unwrap();
        let ControlKind::Number { step, .. } = black.kind else {
            unreachable!()
        };
        assert!((step / 10.0 - 1e-4).abs() < 1e-9);
        assert_eq!(black.check(&number(0.0001)), Ok(()));
    }

    /// darktable's picker on Exposure (`exposure.c:1211`): an area, writing the exposure
    /// and the black `_exposure_set_white` keeps below the new white (`:763-779`), and
    /// reading the area exposure mapping's settings (`:866-973`).
    #[test]
    fn its_picker_is_darktable_s() {
        let darktable = [DarktablePicker {
            id: "exposure",
            slider: "exposure",
            kind: PickerKind::Area,
            denoise: false,
            output: false,
            writes: &["exposure", "black_level"],
            tools: &["area_mode", "lightness"],
        }];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
    }

    #[test]
    fn exposure_is_the_parameter_its_default_the_photo_s() {
        // darktable's `reload_defaults`: +0.7 EV on a colour raw, 0 on a monochrome one
        // and on an image that is not a raw, the exposure operation's own.
        let mut monochrome = raw(0.0);
        monochrome.camera.as_mut().unwrap().cfa = None;
        for (image, expected) in [
            (raw(0.0), 0.7),
            (raw(-0.67), 0.7),
            (monochrome, 0.0),
            (ImageContext::full(8, 8), 0.0),
        ] {
            assert_eq!(Exposure.defaults(&image).number("exposure"), expected);
            let base = Base::new(OPERATIONS, &image);
            assert_eq!(base.float(EXPOSURE, "exposure"), expected);
        }
        // Any other value is written as it is, darktable's slider's value, the camera's
        // bias compensated by the operation, never by the binding.
        for image in [raw(0.0), raw(-0.67), raw(5.0), ImageContext::full(8, 8)] {
            for ev in [-18.0, -3.0, 0.0, 1.0, 1.3, 4.0, 18.0] {
                if ev == default_exposure(&image) {
                    continue;
                }
                assert_eq!(
                    bind(&[("exposure", number(ev))], &image),
                    [Setting::new(EXPOSURE, "exposure", Value::Float(ev))]
                );
            }
        }
    }

    #[test]
    fn the_rendered_white_is_the_one_the_operation_divides_by() {
        // 2^−(e − β): at the colour raw's default with no bias, 2^−0.7.
        assert_eq!(rendered_white(0.7, 0.0), (-0.7_f64).exp2() as f32);
        assert_eq!(rendered_white(0.7, -5.0), (-5.7_f64).exp2() as f32);
        assert_eq!(rendered_white(0.0, 0.0), 1.0);
    }

    #[test]
    fn black_level_puts_a_black_that_reaches_the_white_darktable_s_margin_below_it() {
        let default = -1.0 / 4096.0;
        // A bias of −5 EV: w = 2^−5.7, 0.0192, so a black at or above it goes to 0.0092.
        let white = rendered_white(0.7, -5.0);
        let black = BlackLevel::new(default, white);
        assert_eq!(black.value(0.1), white - 0.01);
        assert_eq!(black.derivative(0.1), 0.0);
        assert_eq!(black.value(0.0), default);
        assert_eq!(black.derivative(-0.1), 1.0);
        // Just below the white the black is as set, as darktable's is.
        let below = white - 0.005 - default;
        assert_eq!(black.value(below), default + below);
        assert_eq!(black.derivative(below), 1.0);
        // With no bias the white is at 0.6156: darktable renders every black below it.
        let white = rendered_white(0.7, 0.0);
        let free = BlackLevel::new(default, white);
        transfer::check(&free, -0.99..=0.6).unwrap();
        assert!(increasing(&free, -1.0..=0.6));
        assert_eq!(free.value(0.1), default + 0.1);
        assert_eq!(free.value(0.6), default + 0.6);
        assert_eq!(free.value(1.0), white - 0.01);
        assert_eq!(free.derivative(1.0), 0.0);
        // At the range's foot the black stops at the operation's −1, darktable's lowest.
        assert_eq!(free.value(-1.0), -1.0);
        assert_eq!(free.derivative(-1.0), 0.0);
        assert_eq!(free.derivative(-0.999), 1.0);
    }

    #[test]
    fn exposure_black_level_and_camera_bias_set_the_exposure_operation() {
        let image = raw(-0.67);
        let base = Base::new(OPERATIONS, &image);
        let black = base.float(EXPOSURE, "black") + 0.01;
        let settings = bind(
            &[
                ("exposure", number(1.0)),
                ("black_level", number(0.01)),
                ("camera_bias", ControlValue::Bool(false)),
            ],
            &image,
        );
        assert_eq!(
            settings,
            [
                Setting::new(EXPOSURE, "compensate_exposure_bias", Value::Bool(false)),
                Setting::new(EXPOSURE, "black", Value::Float(black)),
                Setting::new(EXPOSURE, "exposure", Value::Float(1.0)),
            ]
        );
        // The white Black level compares with is the one Exposure renders: at +8 EV with
        // the bias compensated, 2^−8.67, so a black of +0.01 goes 0.01 below it.
        let white = rendered_white(8.0, -0.67);
        let settings = bind(
            &[("exposure", number(8.0)), ("black_level", number(0.01))],
            &image,
        );
        assert_eq!(
            settings,
            [
                Setting::new(EXPOSURE, "black", Value::Float(white - 0.01)),
                Setting::new(EXPOSURE, "exposure", Value::Float(8.0)),
            ]
        );
        // The camera's bias, when compensated, is the operation's: within ±5 EV.
        assert_eq!(camera_bias(&raw(-8.0)), -5.0);
        assert_eq!(camera_bias(&ImageContext::full(8, 8)), 0.0);
    }

    #[test]
    fn version_1_s_exposure_is_left_out_and_the_rest_kept() {
        let mut saved = ModuleValues::new();
        saved.set("exposure", number(1.0));
        saved.set_mark("exposure", Some(crate::Mark::Kept));
        saved.set("black_level", number(0.01));
        saved.set("camera_bias", ControlValue::Bool(false));
        let migrated = Exposure.migrate(1, saved.clone());
        assert_eq!(migrated.get("exposure"), None);
        assert_eq!(migrated.mark("exposure"), None);
        assert_eq!(migrated.number(STOPS), Some(1.0));
        assert!(DEFINITION.control(STOPS).is_none());
        assert_eq!(migrated.number("black_level"), Some(0.01));
        assert_eq!(migrated.flag("camera_bias"), Some(false));
        // Version 2's values are its own.
        assert_eq!(Exposure.migrate(2, saved.clone()), saved);
    }

    // The picker: darktable's `_auto_set_exposure` (`exposure.c:866-965`) on readings
    // made for the test, against its arithmetic in f64.

    /// A reading whose mean is `rgb` at Exposure's input.
    fn reading(rgb: [f32; 3]) -> Reading {
        let [r, g, b] = rgb;
        let stats = crate::pick::Stats {
            mean: [r, g, b, 1.0],
            min: [r, g, b, 1.0],
            max: [r, g, b, 1.0],
        };
        Reading {
            input: stats,
            output: None,
        }
    }

    /// Exposure's controls at `image`'s defaults, with these set.
    fn values(image: &ImageContext, set: &[(&'static str, ControlValue)]) -> Controls {
        let mut values = Exposure.defaults(image);
        for (control, value) in set {
            values.set(control, value.clone());
        }
        values
    }

    fn correction(lightness: f32) -> [(&'static str, ControlValue); 2] {
        [
            ("area_mode", ControlValue::Choice(0)),
            ("lightness", number(lightness)),
        ]
    }

    fn pick(
        image: &ImageContext,
        rgb: [f32; 3],
        values: &Controls,
        tools: &[(&str, ControlValue)],
    ) -> Picked {
        Exposure
            .pick(
                "exposure",
                &reading(rgb),
                values,
                image,
                tools,
                None,
                None,
                PickKeys::default(),
            )
            .unwrap()
    }

    /// CIE L's Y for the D50 white, darktable's `lab_f_inv` above its knee: the grey a
    /// target lightness maps to.
    fn y_of(lightness: f64) -> f64 {
        ((lightness + 16.0) / 116.0).powi(3)
    }

    /// The area's luminance, Y of XYZ D50, through the photo's input profile, in f64.
    fn luminance(image: &ImageContext, rgb: [f32; 3]) -> f64 {
        let row = input_profile(image).unwrap()[1];
        (0..3).map(|c| f64::from(row[c]) * f64::from(rgb[c])).sum()
    }

    #[test]
    fn its_pick_maps_the_area_to_the_target_lightness() {
        // An image that is not a raw, read through linear Rec2020, and colour raws with
        // and without a bias, compensated and not.
        let cases = [
            (ImageContext::full(8, 8), true),
            (raw(0.0), true),
            (raw(-0.67), true),
            (raw(-0.67), false),
            (raw(1.3), true),
        ];
        for (image, compensated) in cases {
            let bias = if compensated {
                f64::from(camera_bias(&image))
            } else {
                0.0
            };
            for rgb in [[0.18, 0.18, 0.18], [0.02, 0.05, 0.01], [0.7, 0.4, 0.9]] {
                for target in [18.0, 50.0, 72.5, 100.0] {
                    let values =
                        values(&image, &[("camera_bias", ControlValue::Bool(compensated))]);
                    let picked = pick(&image, rgb, &values, &correction(target));
                    // −log2(Y / Y_target), plus the bias the operation takes off again.
                    let expected =
                        -(luminance(&image, rgb) / y_of(f64::from(target))).log2() + bias;
                    let written = picked.writes.number("exposure");
                    assert!(
                        (f64::from(written) - expected).abs() < 1e-4,
                        "{rgb:?} to L {target}: {written} for {expected}"
                    );
                    // Only Exposure: the black lies below every white here.
                    assert_eq!(picked.writes.iter().count(), 1);
                }
            }
        }
    }

    #[test]
    fn its_pick_follows_darktable_s_lightness_through_lab() {
        // The grey of 0.18: L 49.5, sRGB 0.461, shown with L, its C and h 0.
        let image = ImageContext::full(8, 8);
        let picked = pick(&image, [0.18; 3], &values(&image, &[]), &correction(50.0));
        let patch = picked.shown.patch.unwrap();
        let lightness = 116.0 * 0.18_f64.cbrt() - 16.0;
        assert!(
            (f64::from(patch.lch[0]) - lightness).abs() < 1e-3,
            "{patch:?}"
        );
        assert_eq!(&patch.lch[1..], [0.0, 0.0]);
        let srgb = 1.055 * 0.18_f64.powf(1.0 / 2.4) - 0.055;
        for c in patch.srgb {
            assert!((f64::from(c) - srgb).abs() < 1e-3, "{patch:?}");
        }
        // The default target is 50 %, darktable's.
        let default = pick(&image, [0.18; 3], &values(&image, &[]), &[]);
        assert_eq!(default.writes, picked.writes);
        // An area whose colour differs is made grey of its own lightness first: L's Y,
        // through Lab and back, is the area's.
        let rgb = [0.6, 0.1, 0.05];
        let picked = pick(&image, rgb, &values(&image, &[]), &correction(50.0));
        let expected = -(luminance(&image, rgb) / y_of(50.0)).log2();
        assert!((f64::from(picked.writes.number("exposure")) - expected).abs() < 1e-4);
        // A target patch is the grey of its lightness, as `_target_color_draw` paints it.
        let target = Patch::of_lch([50.0, 0.0, 0.0]);
        let srgb = 1.055 * y_of(50.0).powf(1.0 / 2.4) - 0.055;
        for c in target.srgb {
            assert!((f64::from(c) - srgb).abs() < 1e-3, "{target:?}");
        }
    }

    #[test]
    fn its_pick_writes_nothing_where_exposure_already_is_the_one_picked() {
        // `_exposure_set_white` returns where the exposure is the same.
        let image = raw(-0.67);
        let first = pick(
            &image,
            [0.3, 0.2, 0.1],
            &values(&image, &[]),
            &correction(60.0),
        );
        let picked = first.writes.number("exposure");
        let again = pick(
            &image,
            [0.3, 0.2, 0.1],
            &values(&image, &[("exposure", number(picked))]),
            &correction(60.0),
        );
        assert!(again.writes.is_empty());
        assert_eq!(again.shown, first.shown);
    }

    #[test]
    fn its_pick_puts_a_black_at_or_above_the_new_white_darktable_s_margin_below_it() {
        // A dark area: the white 2^−e is Y / Y_target, below a black of 0.5.
        let image = ImageContext::full(8, 8);
        let rgb = [0.001; 3];
        let set = [("black_level", number(0.5))];
        let picked = pick(&image, rgb, &values(&image, &set), &correction(50.0));
        let white = luminance(&image, rgb) / y_of(50.0);
        let level = picked.writes.number("black_level");
        assert!(
            (f64::from(level) - (white - 0.01)).abs() < 1e-6,
            "{level} for {white}"
        );
        // Over a colour raw's own black, −2⁻¹², as Black level correction is.
        let image = raw(0.0);
        let picked = pick(&image, rgb, &values(&image, &set), &correction(50.0));
        let white = 2f32.powf(-picked.writes.number("exposure"));
        assert_eq!(
            picked.writes.number("black_level"),
            white - 0.01 + 1.0 / 4096.0
        );
        // A black below the new white is left as it is.
        let picked = pick(&image, [0.5; 3], &values(&image, &set), &correction(50.0));
        assert_eq!(picked.writes.get("black_level"), None);
    }

    #[test]
    fn its_pick_has_darktable_s_ends() {
        let image = ImageContext::full(8, 8);
        // A target lightness of 0 maps to no exposure: not finite, which the session
        // leaves out, keeping the patch.
        let picked = pick(&image, [0.18; 3], &values(&image, &[]), &correction(0.0));
        assert!(!picked.writes.number("exposure").is_finite());
        // An area of black or below: the white floored at 1e-20, so log2 has a value,
        // past Exposure's +18 EV, which the session holds there.
        let picked = pick(&image, [0.0; 3], &values(&image, &[]), &correction(50.0));
        let exposure = picked.writes.number("exposure");
        assert!(exposure.is_finite() && exposure > 18.0, "{exposure}");
        // Another picker's reading, or a colour raw with no matrix, picks nothing.
        let none = |picker: &str, image: &ImageContext| {
            Exposure.pick(
                picker,
                &reading([0.18; 3]),
                &values(image, &[]),
                image,
                &[],
                None,
                None,
                PickKeys::default(),
            )
        };
        assert_eq!(none("black", &image), None);
        let mut matrixless = raw(0.0);
        matrixless.camera.as_mut().unwrap().xyz_to_camera = None;
        assert_eq!(none("exposure", &matrixless), None);
    }

    #[test]
    fn in_measure_mode_its_pick_gives_the_target_the_area_renders_at_and_writes_nothing() {
        let measure = [
            ("area_mode", ControlValue::Choice(1)),
            ("lightness", number(50.0)),
        ];
        for (image, compensated, exposure) in [
            (ImageContext::full(8, 8), false, 1.0),
            (raw(-0.67), true, 0.7),
            (raw(-0.67), false, -2.0),
        ] {
            let set = [
                ("exposure", number(exposure)),
                ("camera_bias", ControlValue::Bool(compensated)),
            ];
            let bias = if compensated {
                f64::from(camera_bias(&image))
            } else {
                0.0
            };
            let rgb = [0.18, 0.12, 0.1];
            let picked = pick(&image, rgb, &values(&image, &set), &measure);
            assert!(picked.writes.is_empty());
            let y = luminance(&image, rgb) * (f64::from(exposure) - bias).exp2();
            let expected = 116.0 * y.cbrt() - 16.0;
            let [("lightness", ControlValue::Number(lightness))] = picked.shown.tools[..] else {
                panic!("{:?}", picked.shown.tools)
            };
            assert!(
                (f64::from(lightness) - expected).abs() < 1e-3,
                "{lightness} for {expected}"
            );
            // The patch is the input's, as in correction.
            let input = pick(&image, rgb, &values(&image, &set), &correction(50.0));
            assert_eq!(picked.shown.patch, input.shown.patch);
        }
        // Held to the setting's 0 to 100, as darktable's slider holds it.
        let image = ImageContext::full(8, 8);
        let bright = pick(
            &image,
            [4.0; 3],
            &values(&image, &[("exposure", number(4.0))]),
            &measure,
        );
        assert_eq!(
            bright.shown.tools,
            [("lightness", ControlValue::Number(100.0))]
        );
    }

    #[test]
    fn its_input_profile_and_black_are_colorin_s_and_the_exposure_operation_s() {
        let mut monochrome = raw(0.0);
        monochrome.camera.as_mut().unwrap().cfa = None;
        for image in [raw(0.0), raw(-0.67), monochrome, ImageContext::full(8, 8)] {
            let base = Base::new(OPERATIONS, &image);
            assert_eq!(default_black(&image), base.float(EXPOSURE, "black"));
            // colorin's matrix, from the input profile to the working space, linear
            // Rec2020, at its defaults for the photo.
            let colorin = apertrust_operations::operation("colorin").unwrap();
            let params = colorin.defaults(&image);
            let input = apertrust_plugin::ImageDesc::rgba(8, 8);
            let prepared = colorin.prepare(&params, &image, &input).unwrap();
            let floats: Vec<f32> = prepared.passes[0]
                .params
                .chunks(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            let to_working: Mat3 = [0, 1, 2].map(|r| [0, 1, 2].map(|c| floats[r * 4 + c]));
            let working = to_f32(profile_rgb_to_xyz_d50(PROFILE_D65, REC2020_PRIMARIES));
            let through = apertrust_color::mul(&working, &to_working);
            let ours = input_profile(&image).unwrap();
            for (a, b) in through.iter().flatten().zip(ours.iter().flatten()) {
                assert!((a - b).abs() < 1e-5, "{through:?} for {ours:?}");
            }
        }
    }
}
