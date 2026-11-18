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

//! Exposure (`docs/tools/exposure.md`): darktable's exposure module as the user
//! edits with it, in the first pass (decision 2), on one untitled tab, its page the
//! board's, an exception to darktable's (decision 88).
//!
//! - **Mode:** a Manual | Automatic switch (decision 88), automatic darktable's
//!   deflicker: the exposure that puts a percentile of the raw's histogram at a level
//!   below the white point, worked out from each photo, so a series comes out matched.
//!   An image that is not a raw lists manual alone, and an edit holding automatic renders
//!   there as manual. Exposure has no Auto: automatic mode is its one automatic way.
//! - **In manual mode:** Exposure, darktable's slider, `exposure.exposure` itself, −18 to
//!   +18 EV, its default the photo's: +0.7 EV on a colour raw, 0 otherwise (decision 30),
//!   with its picker; Picker target, what the picker makes of the area, and Lightness
//!   under it while Lightness is chosen, settings no edit stores ([`crate::settings`]);
//!   Compensate camera exposure, whether the camera's exposure compensation is undone;
//!   and Highlight preservation mode, whether the exposure a camera's
//!   highlight-preserving mode held back is given back, on by default. The two camera
//!   rows are hidden where the photo gives them nothing to do, their labels showing what
//!   they take off and give back (decision 88).
//! - **In automatic mode:** Percentile and Target level, labelled Measure at and Place it
//!   at, and darktable's computed EC, the exposure the operation applies, a computed
//!   value (`Row::Computed`) labelled This photo (decision 88).
//! - **Black level correction,** darktable's offset over the photo's own black.
//! - **Its picker,** on Exposure's quad: darktable's `_auto_set_exposure`, which sets the
//!   exposure that maps the area to Picker target's lightness, its tooltip and its hint
//!   over the photo following Picker target (decision 88).
//!
//! It drives `exposure`, which every photo runs, and owns every parameter of it. Contrast
//! and its pivot, which it once bound through colour balance rgb, are the Colour balance
//! RGB tool's `contrast` and `grey_fulcrum` (decision 2), and Brightness that tool's
//! `midtones_y`.

use apertrust_base::settings::Choice;
use apertrust_color::{
    Mat3, PROFILE_D65, REC2020_PRIMARIES, apply, camera_to_xyz_d50, lab_to_xyz_d50,
    profile_rgb_to_xyz_d50, to_f32, xyz_to_lab_d50,
};
use apertrust_plugin::{ImageContext, Value};

use crate::about::{Fact, Facts, Purpose};
use crate::definition::{
    ChoiceStyle, Computed, Control, ControlKind, Picker, PickerKind, ProcessingDefinition, Quad,
    Reset, Row, SettingRow, Slider, Space, Tab, ToolDefinition, Track, Unit,
};
use crate::pick::{LastPick, Patch, PickKeys, PickWarnings, Picked, Reading, Short, Shown};
use crate::transfer::Transfer;
use crate::{
    Base, ControlValue, Controls, LabelValue, Owned, ProcessingTool, Setting, ToolValues,
    ValueFormat,
};

pub struct Exposure;

/// The operation it drives.
const EXPOSURE: &str = "exposure";

/// darktable's modes, `dt_iop_exposure_mode_t`, as the operation's choices.
const MODES: [&str; 2] = ["manual", "automatic"];
const AUTOMATIC: u32 = 1;

/// The computed value automatic mode shows, the exposure the operation applies: what
/// the exposure operation's `computed` gives by this id (decision 87).
const COMPUTED_EXPOSURE: &str = "computed_exposure";

/// The controls, in darktable's `gui_init` order; the page puts Exposure before the
/// camera's rows (decision 88). Exposure, Black level correction and Compensate camera
/// exposure keep their first ids; the others are darktable's parameter names.
static CONTROLS: [Control; 7] = [
    Control {
        id: "mode",
        kind: ControlKind::Choice {
            options: &MODES,
            default: 0,
            unported: &[],
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
        // On, as darktable turns it on for a module's first instance and the operation's
        // default is; it acts only on a photo whose camera held exposure back.
        kind: ControlKind::Bool { default: true },
        unit: None,
        unported: false,
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
        unported: false,
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
        unported: false,
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

/// Picker target, under Exposure in manual mode, in place of darktable's area exposure
/// mapping (decision 88): what Exposure's picker makes of the area picked, a setting no
/// edit stores, kept in settings as `tools.exposure.target`.
const TARGET: Row = Row::Setting(SettingRow {
    id: "target",
    kind: ControlKind::Choice {
        options: &TARGETS,
        default: 0,
        unported: &[],
    },
    decimals: 0,
    track: Track::Plain,
    key: None,
    words: None,
});

/// Lightness, under Picker target while it holds Lightness: the L the picked area is
/// given, darktable's `darkroom/modules/exposure/lightness` with its range, decimal and
/// default, its track the ramp of greys darktable paints, kept in settings as
/// `tools.exposure.lightness`.
const LIGHTNESS: Row = Row::WhenSetting {
    setting: "target",
    is: &["lightness"],
    rows: &[Row::Setting(SettingRow {
        id: "lightness",
        kind: ControlKind::Number {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: MIDDLE_GREY,
        },
        decimals: 1,
        track: Track::Lightness,
        key: None,
        words: None,
    })],
};

/// The reference, under Picker target while it holds Reference: an area's L measured by
/// its own picker on any photo, kept in settings with the photo's path, across photos,
/// folders and restarts, until set again or cleared (decision 88).
const REFERENCE: Row = Row::WhenSetting {
    setting: "target",
    is: &["reference"],
    rows: &[Row::Reference {
        picker: "reference",
        lightness: REFERENCE_LIGHTNESS,
        photo: "reference_photo",
    }],
};

/// The setting holding the reference's L, `tools.exposure.reference_lightness`.
const REFERENCE_LIGHTNESS: &str = "reference_lightness";

/// Picker target's options, in the board's order.
const TARGETS: [&str; 3] = ["middle_grey", "lightness", "reference"];

/// Middle grey's L, darktable's default target lightness.
const MIDDLE_GREY: f32 = 50.0;

/// What Exposure's picker makes of the area picked, the setting `tools.exposure.target`
/// (decision 88).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerTarget {
    /// L 50 %, darktable's default target.
    #[default]
    MiddleGrey,
    /// The L the setting `tools.exposure.lightness` holds.
    Lightness,
    /// The L of an area measured by the reference's picker, on this photo or another,
    /// `tools.exposure.reference_lightness`.
    Reference,
}

impl PickerTarget {
    /// The option a setting's value names, Middle grey where it names none.
    fn of(value: Option<&ControlValue>) -> Self {
        match value {
            Some(&ControlValue::Choice(at)) => Self::OPTIONS
                .get(at as usize)
                .map_or(Self::MiddleGrey, |(target, _)| *target),
            _ => Self::MiddleGrey,
        }
    }
}

impl Choice for PickerTarget {
    const OPTIONS: &'static [(Self, &'static str)] = &[
        (Self::MiddleGrey, TARGETS[0]),
        (Self::Lightness, TARGETS[1]),
        (Self::Reference, TARGETS[2]),
    ];
}

static DEFINITION: ProcessingDefinition = ProcessingDefinition {
    tool: ToolDefinition {
        id: "exposure",
        tabs: &[Tab {
            id: None,
            rows: &[
                // Manual | Automatic, the one segmented choice listed per photo (decision
                // 88).
                Row::Choice {
                    control: "mode",
                    style: ChoiceStyle::Segmented,
                },
                // The manual page (decision 88): the camera's rows first, as darktable's
                // stack has them, then Exposure with its picker and Picker target right
                // under it.
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
                        TARGET,
                        LIGHTNESS,
                        REFERENCE,
                        // What the last pick did, its warnings above it.
                        Row::LastPick { picker: "exposure" },
                    ],
                },
                Row::When {
                    control: "mode",
                    is: &["automatic"],
                    rows: &[
                        Row::Slider(Slider::new("deflicker_percentile")),
                        Row::Slider(Slider::new("deflicker_target_level")),
                        // darktable's "computed EC", `%.2f EV`.
                        Row::Computed(Computed {
                            id: COMPUTED_EXPOSURE,
                            operation: EXPOSURE,
                            unit: Some(Unit::Ev),
                            decimals: 2,
                        }),
                    ],
                },
                Row::Slider(Slider::new("black_level").decimals(4).soft(-0.1, 0.1)),
            ],
        }],
        before: &[],
        after: &[],
        reset: Reset::Tool,
        presets: true,
        menus: &[],
    },
    version: 3,
    layers: true,
    // Automatic mode is its one automatic way (decision 88).
    auto: false,
    switched: false,
    controls: &CONTROLS,
    preset_rows: &[],
    views: &[],
    pickers: &[PICKER, REFERENCE_PICKER],
    // darktable's `description()` in src/iop/exposure.c at the reference release.
    facts: Some(Facts {
        purpose: Purpose::CorrectiveAndCreative,
        input: Fact::LinearRgbSceneReferred,
        process: Fact::LinearRgb,
        output: Fact::LinearRgbSceneReferred,
    }),
};

/// darktable's picker on Exposure (`DT_COLOR_PICKER_AREA`): the exposure that maps the
/// area to Picker target's lightness, the black kept below the new white; its pick comes
/// with Picker target, Lightness and the reference, and its tooltip and hint over the
/// photo follow Picker target (decision 88).
const PICKER: Picker = Picker {
    id: "exposure",
    operation: EXPOSURE,
    kind: PickerKind::Area,
    denoise: false,
    output: false,
    space: Space::Operation,
    keep: false,
    writes: &["exposure", "black_level"],
    settings: &["target", "lightness", REFERENCE_LIGHTNESS],
    words_by: Some("target"),
};

/// The reference row's picker (decision 88): an area of the exposure operation's input,
/// as Exposure's, that writes no control. Its pick measures the area by darktable's
/// measure mode's arithmetic and hands its L back as the reference. It reads no setting: the
/// reference it hands back is never its input, so a change of it never picks again.
const REFERENCE_PICKER: Picker = Picker {
    id: "reference",
    operation: EXPOSURE,
    kind: PickerKind::Area,
    denoise: false,
    output: false,
    space: Space::Operation,
    keep: false,
    writes: &[],
    settings: &[],
    words_by: None,
};

/// Every parameter of `exposure`: the tool is darktable's and owns them all.
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

impl ProcessingTool for Exposure {
    fn definition(&self) -> &'static ProcessingDefinition {
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

    /// Compensate camera exposure does nothing on a photo that records no bias, and
    /// Highlight preservation mode nothing on one whose camera held no exposure back,
    /// where darktable hides it (`gui_update`). Neither depends on the tool's values. Mode
    /// stays available everywhere, its options listed per photo ([`Self::options`]), so
    /// an edit pasted onto an image that is not a raw holding automatic can be set back
    /// to manual (decision 88).
    fn available(&self, control: &str, _: &Controls, image: &ImageContext) -> bool {
        match control {
            "camera_bias" => camera_bias(image) != 0.0,
            "compensate_hilite_pres" => highlight_bias(image) != 0.0,
            _ => true,
        }
    }

    /// Mode lists manual alone on an image that is not a raw of one 16-bit channel, where
    /// darktable makes it insensitive: one with no camera, since the engine develops only
    /// 16-bit mosaics, and which has no raw histogram for automatic mode to measure. The
    /// switch draws automatic disabled there, with its reason (decision 88).
    fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
        match control {
            "mode" if image.camera.is_none() => vec![MODES[0]],
            "mode" => MODES.to_vec(),
            _ => match DEFINITION.control(control).map(|control| control.kind) {
                Some(ControlKind::Choice { options, .. }) => options.to_vec(),
                _ => Vec::new(),
            },
        }
    }

    /// Compensate camera exposure is hidden on a photo that records no bias, and
    /// Highlight preservation mode on one whose camera held no exposure back, as
    /// darktable hides the latter (`gui_update`); the two camera rows are the one
    /// exception to drawing a control that cannot act disabled (decision 88).
    fn visible(&self, control: &str, image: &ImageContext) -> bool {
        match control {
            "camera_bias" => camera_bias(image) != 0.0,
            "compensate_hilite_pres" => highlight_bias(image) != 0.0,
            _ => true,
        }
    }

    /// What the two camera rows take off and give back, the operation's values: the
    /// camera's bias in thirds, as the camera writes it, "−⅔ EV", and the exposure a
    /// highlight-preserving mode held back with one decimal, "+1.0 EV", where
    /// darktable writes both with one decimal, `%+.1f` and `%.1f` (decision 88). None
    /// where the row is hidden, the value 0.
    fn label_value(&self, control: &str, image: &ImageContext) -> Option<LabelValue> {
        let (value, format) = match control {
            "camera_bias" => (camera_bias(image), ValueFormat::Thirds),
            "compensate_hilite_pres" => (highlight_bias(image), ValueFormat::Decimals(1)),
            _ => return None,
        };
        (value != 0.0).then_some(LabelValue {
            value,
            unit: Unit::Ev,
            format,
        })
    }

    /// Mode, Percentile and Target level are the parameters themselves, written where set,
    /// as are `compensate_exposure_bias` and `compensate_hilite_pres`. Exposure is written
    /// as it is, as darktable's slider writes it; the black, only while Black level is set,
    /// since the photo's own black lies below any white, held below the white manual mode
    /// renders; in automatic mode within darktable's ±1 alone, the operation holding it
    /// below the white the photo's histogram gives.
    fn bind(&self, values: &ToolValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
        let mut settings = Vec::new();
        if let Some(mode) = values.choice("mode") {
            settings.push(Setting::new(EXPOSURE, "mode", Value::Choice(mode)));
        }
        for param in ["deflicker_percentile", "deflicker_target_level"] {
            if let Some(value) = values.number(param) {
                settings.push(Setting::new(EXPOSURE, param, Value::Float(value)));
            }
        }
        let mut flag = |control: &str, param: &'static str| match values.flag(control) {
            Some(on) => {
                settings.push(Setting::new(EXPOSURE, param, Value::Bool(on)));
                on
            }
            None => base.flag(EXPOSURE, param),
        };
        let compensated = flag("camera_bias", "compensate_exposure_bias");
        let given_back = flag("compensate_hilite_pres", "compensate_hilite_pres");
        let bias = taken_off(image, compensated, given_back);
        let exposure = values
            .number("exposure")
            .unwrap_or_else(|| base.float(EXPOSURE, "exposure"));
        if let Some(level) = values.number("black_level") {
            // In automatic mode the white is the one the photo's histogram gives, which the
            // operation holds the black below (decision 87): here only darktable's ±1.
            let automatic = values.choice("mode") == Some(AUTOMATIC);
            let white = if automatic {
                f32::INFINITY
            } else {
                rendered_white(exposure, bias)
            };
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
    ///
    /// Before version 3 Highlight preservation mode was not ported and always off, the
    /// one value an edit could hold; version 3 turns it on by default, as darktable does,
    /// so an older edit gets it off, as it rendered.
    fn migrate(&self, from: u32, mut values: ToolValues) -> ToolValues {
        if from < 2
            && let Some(stops) = values.remove("exposure")
        {
            values.set(STOPS, stops);
            values.set_mark("exposure", None);
        }
        if from < 3 && values.get("compensate_hilite_pres").is_none() {
            values.set("compensate_hilite_pres", ControlValue::Bool(false));
        }
        values
    }

    /// darktable's `color_picker_apply`, its `_auto_set_exposure` (`exposure.c:866-965`),
    /// on the area's mean at Exposure's input, read through the input profile's matrix to
    /// XYZ D50 ([`input_profile`]) and made grey of its lightness: the input patch and its
    /// L, then darktable's correction toward Picker target's lightness,
    /// `tools.exposure.target` (decision 88): L 50 for Middle grey, darktable's default,
    /// or `tools.exposure.lightness` for Lightness.
    ///
    /// - The exposure `−log2(Y / Y_target)`, `Y_target` the grey of the target lightness,
    ///   plus the camera's bias where Compensate camera exposure is on and less the
    ///   held-back exposure where Highlight preservation mode is, which the operation takes
    ///   off and gives back again, so the area renders at the target.
    /// - Nothing is written where it equals Exposure's value, as `_exposure_set_white`
    ///   returns; a black at or above the new white goes 0.01 below it (`:763-779`),
    ///   written as Black level correction over the photo's own black.
    /// - darktable bounds neither: the session holds them to their ranges, and leaves out
    ///   a target lightness of 0's, which is not finite, showing the patch still.
    /// - **Reference:** toward `tools.exposure.reference_lightness`; with none set, the
    ///   patch alone and nothing written, the picker not being ready
    ///   ([`ProcessingTool::picker_ready`]).
    ///
    /// The reference row's picker, `reference`, takes darktable's measure mode's
    /// arithmetic (`exposure.c:900-934`): no control, and the L the area renders at at the
    /// current exposure, held to 0 to 100, handed back as `reference_lightness`
    /// (`Shown::settings`). Nothing on a colour raw with no matrix, which colorin cannot
    /// render, as darktable returns with no input profile.
    fn pick(
        &self,
        picker: &str,
        reading: &Reading,
        values: &Controls,
        image: &ImageContext,
        settings: &[(&str, ControlValue)],
        _channel: Option<&str>,
        _last: Option<&Reading>,
        _keys: PickKeys,
    ) -> Option<Picked> {
        if picker != PICKER.id && picker != REFERENCE_PICKER.id {
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
            settings: Vec::new(),
            last: None,
            warnings: PickWarnings::default(),
        };
        let exposure = values.number("exposure");
        let bias = taken_off(
            image,
            values.flag("camera_bias"),
            values.flag("compensate_hilite_pres"),
        );
        let mut writes = Controls::new();
        if picker == REFERENCE_PICKER.id {
            // Measure mode's: the exposure the operation applies, `exposure2white(-expo)`
            // its factor.
            let white = exposure_to_white(-(exposure - bias));
            let out = xyz_to_lab_d50(grey.map(|c| c * white))[0];
            let out = ControlValue::Number(out.clamp(0.0, 100.0));
            shown.settings.push((REFERENCE_LIGHTNESS, out));
            return Some(Picked { writes, shown });
        }
        let setting = |id: &str| settings.iter().find(|(s, _)| *s == id).map(|(_, v)| v);
        let target = match PickerTarget::of(setting("target")) {
            PickerTarget::MiddleGrey => MIDDLE_GREY,
            PickerTarget::Lightness => match setting("lightness") {
                Some(&ControlValue::Number(target)) => target,
                _ => MIDDLE_GREY,
            },
            PickerTarget::Reference => match setting(REFERENCE_LIGHTNESS) {
                Some(&ControlValue::Number(target)) => target,
                _ => return Some(Picked { writes, shown }),
            },
        };
        let target_l = target;
        let target = lab_to_xyz_d50([target, 0.0, 0.0]);
        let expo = -white_to_exposure(grey[1] / target[1]) - bias;
        // `_exposure_set_white`.
        let white = exposure_to_white(-expo);
        let picked = white_to_exposure(white);
        let default = default_black(image);
        let level = values.number("black_level");
        if picked != exposure {
            writes.set("exposure", ControlValue::Number(picked));
            // The black the operation holds now, darktable's `p->black`.
            let held = BlackLevel::new(default, rendered_white(exposure, bias));
            if held.value(level) >= white {
                let level = white - WHITE_MARGIN - default;
                writes.set("black_level", ControlValue::Number(level));
            }
        }
        // What the pick did and what went wrong (decision 88): the area's L before and as
        // the session writes it, Exposure held to its range, the black the operation then
        // holds included.
        let [r, g, b, _] = reading.input.max;
        shown.warnings.clipped = ([r, g, b].iter())
            .zip(reading.input_maximum)
            .any(|(channel, maximum)| *channel >= CLIPPED * maximum);
        if picked.is_finite() {
            let (min, max) = EXPOSURE_RANGE;
            let after_exposure = picked.clamp(min, max);
            let after_level = match writes.get("black_level") {
                Some(&ControlValue::Number(level)) => level,
                _ => level,
            };
            let before = rendered_lightness(grey[1], exposure, level, bias, default);
            let after = rendered_lightness(grey[1], after_exposure, after_level, bias, default);
            shown.last = Some(LastPick {
                before,
                after,
                exposure: after_exposure - exposure,
                patches: [before, after].map(|l| Patch::of_lch([l, 0.0, 0.0])),
            });
            // Held at an end of the range, as the session holds it, and missing the target.
            let held = after_exposure == min || after_exposure == max;
            if held && (after - target_l).abs() > SHORT {
                shown.warnings.short = Some(Short {
                    exposure: after_exposure,
                    reached: after,
                    target: target_l,
                    black: writes.get("black_level").is_some(),
                });
            }
        }
        Some(Picked { writes, shown })
    }

    /// Exposure's picker is not ready while Picker target holds Reference and no reference
    /// is set (decision 88); the reference's own picker always is.
    fn picker_ready(&self, picker: &str, settings: &[(&str, ControlValue)]) -> bool {
        let setting = |id: &str| settings.iter().find(|(s, _)| *s == id).map(|(_, v)| v);
        picker != PICKER.id
            || PickerTarget::of(setting("target")) != PickerTarget::Reference
            || matches!(setting(REFERENCE_LIGHTNESS), Some(ControlValue::Number(_)))
    }
}

/// How near its maximum at the input a channel of the area reads as clipped
/// (decision 88).
const CLIPPED: f32 = 0.99;

/// How far from its target, in L, an area held by Exposure's range falls short.
const SHORT: f32 = 0.05;

/// Exposure's range, darktable's ±18 EV, which the session holds a pick to.
const EXPOSURE_RANGE: (f32, f32) = (-18.0, 18.0);

/// The L an area of luminance `y` at Exposure's input renders at with Exposure at
/// `exposure` and Black level correction at `level`: the operation's `(in − b) / (w − b)`,
/// `w` the white it renders at 1.0 ([`rendered_white`]) and `b` the black it holds
/// ([`BlackLevel`]), on the area's grey, from 0, held to 0 to 100 as measure mode holds it.
fn rendered_lightness(y: f32, exposure: f32, level: f32, bias: f32, default: f32) -> f32 {
    let white = rendered_white(exposure, bias);
    let black = BlackLevel::new(default, white).value(level);
    let out = ((y - black) / (white - black)).max(0.0);
    xyz_to_lab_d50([out; 3])[0].clamp(0.0, 100.0)
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
/// photo, which no tool changes, the camera's matrix on a colour raw (darktable's
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
/// repeated here and the tool's tests check the two agree.
pub fn default_exposure(image: &ImageContext) -> f32 {
    match &image.camera {
        Some(camera) if camera.cfa.is_some() => 0.7,
        _ => 0.0,
    }
}

/// The camera's exposure bias as the exposure operation compensates it: within ±5 EV,
/// as darktable's `_get_exposure_bias` trusts it. The image context carries it
/// unclamped, and this crate cannot call the operation's own, so the clamp is repeated
/// here; the tool's tests check the two agree.
pub fn camera_bias(image: &ImageContext) -> f32 {
    image.camera.as_ref().map_or(0.0, |camera| {
        camera.exposure_bias.clamp(-BIAS_LIMIT, BIAS_LIMIT)
    })
}

/// The exposure a highlight-preserving mode held back, as the exposure operation gives
/// it back: above 0 and within −1 to +4 EV, as darktable's `_get_highlight_bias` trusts
/// it, 0 otherwise. This crate cannot call the operation's own, so the rule is repeated
/// here; the tool's tests check the two agree.
pub fn highlight_bias(image: &ImageContext) -> f32 {
    image.camera.as_ref().map_or(0.0, |camera| {
        if camera.highlight_preservation > 0.0 {
            camera.highlight_preservation.clamp(-1.0, 4.0)
        } else {
            0.0
        }
    })
}

/// β, what the exposure operation takes off the exposure: the camera's bias where
/// Compensate camera exposure is on, less the held-back exposure where Highlight
/// preservation mode is.
pub fn taken_off(image: &ImageContext, camera: bool, highlights: bool) -> f32 {
    let bias = if camera { camera_bias(image) } else { 0.0 };
    if highlights {
        bias - highlight_bias(image)
    } else {
        bias
    }
}

/// w = 2^−(e − β): the input the exposure `e` renders at 1.0, with β what the operation
/// takes off it ([`taken_off`]), the white the exposure operation divides by.
pub fn rendered_white(exposure: f32, bias: f32) -> f32 {
    (-(f64::from(exposure) - f64::from(bias))).exp2() as f32
}

/// Black level's black: the photo's own, `b₀` (−2⁻¹² for a colour raw), plus the
/// control's value, inside the operation's range, darktable's ±1; one that reaches the
/// white the photo renders, `w`, is put 0.01 below it, darktable's margin, as darktable's
/// `gui_changed` puts it, so the kernel never divides by zero (decision 30). darktable
/// compares with the white of its slider's exposure; this one includes the camera's bias
/// and the held-back exposure, the white the kernel divides by. Rising with a slope of 1 below `w`, flat at ±1, and
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
        let mut values = ToolValues::new();
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
            preserving(-0.67, 0.5),
            preserving(0.0, 6.0),
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

    /// The board's page (decision 88), on one untitled tab: Mode; in manual mode Exposure,
    /// Picker target with Lightness under it while Lightness is chosen, and the camera's
    /// rows; in automatic mode its rows; then Black level correction.
    #[test]
    fn its_page_is_the_board_s_with_darktable_s_camera_rows_first() {
        assert_eq!(DEFINITION.tool.tabs.len(), 1);
        assert_eq!(DEFINITION.tool.tabs[0].id, None);
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
        // Every control has a row; the controls keep darktable's order.
        let mut controls: Vec<&str> = DEFINITION.controls.iter().map(|c| c.id).collect();
        let mut shown = page();
        controls.sort_unstable();
        shown.sort_unstable();
        assert_eq!(controls, shown);
        // The camera's rows first, as darktable's stack has them; Picker target and
        // Lightness, the picker's settings, which no edit stores, right under Exposure; no
        // area exposure mapping, area mode or patches.
        let manual: Vec<String> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::When {
                    is: ["manual"],
                    rows,
                    ..
                } => Some(*rows),
                _ => None,
            })
            .flatten()
            .map(|row| match row {
                Row::Setting(setting) => format!("setting {}", setting.id),
                Row::WhenSetting {
                    setting, is, rows, ..
                } => format!("{setting} is {is:?}: {} rows", rows.len()),
                Row::LastPick { picker } => format!("last pick of {picker}"),
                row => row.controls().join(" "),
            })
            .collect();
        assert_eq!(
            manual,
            [
                "camera_bias",
                "compensate_hilite_pres",
                "exposure",
                "setting target",
                "target is [\"lightness\"]: 1 rows",
                "target is [\"reference\"]: 1 rows",
                "last pick of exposure",
            ]
        );
        let settings: Vec<&str> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::Setting(setting) => Some(setting.id),
                _ => None,
            })
            .collect();
        assert_eq!(settings, ["target", "lightness"]);
        assert!(
            !DEFINITION
                .rows()
                .any(|row| matches!(row, Row::Group { .. } | Row::Patches { .. }))
        );
        // Lightness on darktable's ramp of greys.
        let lightness = DEFINITION.rows().find_map(|row| match row {
            Row::Setting(setting) if setting.id == "lightness" => Some(setting.track),
            _ => None,
        });
        assert_eq!(lightness, Some(Track::Lightness));
        // The reference row under Reference, its picker the reference's.
        let reference = DEFINITION.rows().find_map(|row| match row {
            Row::Reference {
                picker,
                lightness,
                photo,
            } => Some((*picker, *lightness, *photo)),
            _ => None,
        });
        assert_eq!(
            reference,
            Some(("reference", "reference_lightness", "reference_photo"))
        );
    }

    /// Picker target's options, the board's, `PickerTarget`'s, Middle grey the default,
    /// each one to be chosen.
    #[test]
    fn picker_target_lists_middle_grey_lightness_and_reference() {
        let Some(ControlKind::Choice {
            options,
            default,
            unported,
        }) = DEFINITION.rows().find_map(|row| match row {
            Row::Setting(setting) if setting.id == "target" => Some(setting.kind),
            _ => None,
        })
        else {
            panic!("no Picker target")
        };
        assert_eq!(options, ["middle_grey", "lightness", "reference"]);
        let ids: Vec<&str> = PickerTarget::OPTIONS.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, options);
        assert_eq!(
            PickerTarget::OPTIONS[default as usize].0,
            PickerTarget::default()
        );
        assert_eq!(PickerTarget::default(), PickerTarget::MiddleGrey);
        assert!(unported.is_empty());
        assert_eq!(DEFINITION.choice_setting("target"), Some(&TARGETS[..]));
        assert_eq!(PICKER.words_by, Some("target"));
    }

    /// Automatic mode's page as redesigned (decision 88): Mode a Manual | Automatic switch,
    /// no Auto beside it, and automatic's rows Percentile, Target level and the computed
    /// exposure, then Black level correction with darktable's four decimals.
    #[test]
    fn its_mode_is_a_switch_and_it_has_no_auto() {
        assert!(!DEFINITION.auto);
        let mode = DEFINITION
            .rows()
            .find(|row| row.controls() == ["mode"])
            .unwrap();
        assert!(matches!(
            mode,
            Row::Choice {
                style: ChoiceStyle::Segmented,
                ..
            }
        ));
        let automatic: Vec<&str> = DEFINITION
            .rows()
            .filter_map(|row| match row {
                Row::When {
                    is: ["automatic"],
                    rows,
                    ..
                } => Some(*rows),
                _ => None,
            })
            .flatten()
            .flat_map(|row| match row {
                Row::Computed(computed) => vec![computed.id],
                row => row.controls().to_vec(),
            })
            .collect();
        assert_eq!(
            automatic,
            [
                "deflicker_percentile",
                "deflicker_target_level",
                COMPUTED_EXPOSURE
            ]
        );
        let black = DEFINITION
            .rows()
            .find_map(|row| match row {
                Row::Slider(slider) if slider.control == "black_level" => Some(slider.decimals),
                _ => None,
            })
            .unwrap();
        assert_eq!(black, 4);
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = raw(-0.67);
        assert!(
            Exposure
                .bind(&ToolValues::new(), &image, &Base::new(OPERATIONS, &image))
                .is_empty()
        );
        // Every control written at its default is still nothing to set.
        let defaults = [
            ("mode", ControlValue::Choice(0)),
            ("camera_bias", ControlValue::Bool(true)),
            ("compensate_hilite_pres", ControlValue::Bool(true)),
            ("exposure", number(0.7)),
            ("deflicker_percentile", number(50.0)),
            ("deflicker_target_level", number(-4.0)),
            ("black_level", number(0.0)),
        ];
        assert!(bind(&defaults, &image).is_empty());
        // And each is the operation's default for a colour raw.
        let base = Base::new(OPERATIONS, &image);
        assert_eq!(base.get(EXPOSURE, "mode"), Some(Value::Choice(0)));
        assert!(base.flag(EXPOSURE, "compensate_hilite_pres"));
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

    /// A colour raw whose highlight-preserving mode held back `held` EV, with this bias.
    fn preserving(exposure_bias: f32, held: f32) -> ImageContext {
        let mut image = raw(exposure_bias);
        image.camera.as_mut().unwrap().highlight_preservation = held;
        image
    }

    #[test]
    fn highlight_preservation_mode_gives_back_what_the_camera_held_back() {
        // On on every photo, as darktable's first instance; it can act only where the
        // camera held exposure back, as darktable shows it only there.
        for image in [raw(0.0), preserving(0.0, 0.5), ImageContext::full(8, 8)] {
            assert!(Exposure.defaults(&image).flag("compensate_hilite_pres"));
        }
        assert!(!available("compensate_hilite_pres", &raw(-0.67)));
        assert!(!available("compensate_hilite_pres", &preserving(0.0, -0.5)));
        assert!(!available(
            "compensate_hilite_pres",
            &ImageContext::full(8, 8)
        ));
        assert!(available("compensate_hilite_pres", &preserving(0.0, 0.33)));
        // Written only where set, as the operation's parameter.
        let image = preserving(-0.67, 0.5);
        assert_eq!(
            bind(
                &[("compensate_hilite_pres", ControlValue::Bool(false))],
                &image
            ),
            [Setting::new(
                EXPOSURE,
                "compensate_hilite_pres",
                Value::Bool(false)
            )]
        );
        // What it gives back is the operation's, within −1 to +4 EV.
        let effective = |set: &[(&str, ControlValue)], image: &ImageContext| {
            let base = Base::new(OPERATIONS, image);
            let mut params = base.params(EXPOSURE).unwrap().clone();
            for setting in bind(set, image) {
                params.set(setting.param, setting.value);
            }
            apertrust_operations::exposure::effective_exposure(&params, image)
        };
        for held in [-0.5, 0.0, 0.33, 0.5, 2.0, 6.0] {
            let image = preserving(-0.67, held);
            assert_eq!(
                highlight_bias(&image),
                apertrust_operations::exposure::highlight_bias(&image)
            );
            for (camera, highlights) in [(true, true), (true, false), (false, true)] {
                let set = [
                    ("camera_bias", ControlValue::Bool(camera)),
                    ("compensate_hilite_pres", ControlValue::Bool(highlights)),
                ];
                let expected = 0.7 - taken_off(&image, camera, highlights);
                assert!((effective(&set, &image) - expected).abs() < 1e-6);
            }
        }
        assert_eq!(highlight_bias(&preserving(0.0, 6.0)), 4.0);
    }

    /// The label's value as English writes it.
    fn label(control: &str, image: &ImageContext) -> Option<String> {
        let number = |value: f64, decimals: usize, signed: bool| {
            let digits = format!("{:.decimals$}", value.abs());
            match (value < 0.0, signed) {
                (true, _) => format!("−{digits}"),
                (false, true) => format!("+{digits}"),
                (false, false) => digits,
            }
        };
        let value = Exposure.label_value(control, image)?;
        assert_eq!(value.unit, Unit::Ev, "{control}");
        Some(value.write(number))
    }

    #[test]
    fn compensate_camera_exposure_shows_only_with_a_bias_and_says_it_in_thirds() {
        let shown = |image: &ImageContext| Exposure.visible("camera_bias", image);
        assert!(!shown(&raw(0.0)));
        assert!(!shown(&ImageContext::full(8, 8)));
        for (bias, written) in [(-2.0 / 3.0, "−⅔"), (-0.67, "−⅔"), (4.0 / 3.0, "+1⅓")] {
            let image = raw(bias);
            assert!(shown(&image), "{bias}");
            assert_eq!(label("camera_bias", &image).as_deref(), Some(written));
            // The value the operation takes off.
            let value = Exposure.label_value("camera_bias", &image).unwrap();
            assert_eq!(value.value, camera_bias(&image));
        }
        // Beyond what darktable trusts, the bias it takes off.
        assert_eq!(label("camera_bias", &raw(-8.0)).as_deref(), Some("−5"));
    }

    #[test]
    fn highlight_preservation_mode_shows_only_where_the_camera_held_back_and_says_how_much() {
        let shown = |image: &ImageContext| Exposure.visible("compensate_hilite_pres", image);
        for image in [
            raw(-0.67),
            preserving(0.0, 0.0),
            preserving(0.0, -0.5),
            ImageContext::full(8, 8),
        ] {
            assert!(!shown(&image));
        }
        for (held, written) in [(0.5, "+0.5"), (1.0, "+1.0"), (2.0, "+2.0"), (6.0, "+4.0")] {
            let image = preserving(-0.67, held);
            assert!(shown(&image), "{held}");
            assert_eq!(
                label("compensate_hilite_pres", &image).as_deref(),
                Some(written)
            );
        }
        // In manual mode only, as its row follows Mode's.
        let manual = DEFINITION.rows().find_map(|row| match row {
            Row::When { is, rows, .. }
                if rows
                    .iter()
                    .any(|row| row.controls() == ["compensate_hilite_pres"]) =>
            {
                Some(*is)
            }
            _ => None,
        });
        assert_eq!(manual, Some(&["manual"][..]));
    }

    #[test]
    fn only_the_two_camera_rows_hide_or_carry_a_value() {
        let images = [raw(0.0), preserving(-0.67, 1.0), ImageContext::full(8, 8)];
        for control in DEFINITION.controls {
            let camera = ["camera_bias", "compensate_hilite_pres"].contains(&control.id);
            for image in &images {
                let shown = Exposure.visible(control.id, image);
                assert!(shown || camera, "{}", control.id);
                // A value only on a camera row, and only where it shows.
                assert_eq!(
                    Exposure.label_value(control.id, image).is_some(),
                    camera && shown,
                    "{}",
                    control.id
                );
            }
        }
        // Hidden or not, the controls keep their values and bindings: a bias of 0 binds
        // Compensate camera exposure off as it did.
        assert_eq!(
            bind(&[("camera_bias", ControlValue::Bool(false))], &raw(0.0)),
            [Setting::new(
                EXPOSURE,
                "compensate_exposure_bias",
                Value::Bool(false)
            )]
        );
    }

    #[test]
    fn automatic_mode_its_percentile_and_target_level_set_the_operation() {
        // Every control is ported: automatic, Percentile and Target level over their
        // ranges.
        assert!(DEFINITION.controls.iter().all(|control| !control.unported));
        let mode = Exposure.definition().control("mode").unwrap();
        assert!(mode.check(&ControlValue::Choice(1)).is_ok());
        for (control, value) in [
            ("deflicker_percentile", number(0.0)),
            ("deflicker_percentile", number(100.0)),
            ("deflicker_target_level", number(-18.0)),
            ("deflicker_target_level", number(18.0)),
        ] {
            let control = DEFINITION.control(control).unwrap();
            assert_eq!(control.check(&value), Ok(()));
        }
        // Mode lists automatic on a raw, as darktable's is live there, and manual alone
        // on another image, where it stays available so a pasted automatic can be set
        // back to manual (decision 88).
        assert_eq!(Exposure.options("mode", &raw(-0.67)), MODES);
        assert_eq!(
            Exposure.options("mode", &ImageContext::full(8, 8)),
            ["manual"]
        );
        assert!(available("mode", &raw(-0.67)));
        assert!(available("mode", &ImageContext::full(8, 8)));
        // Each is its parameter, written where set.
        let settings = bind(
            &[
                ("mode", ControlValue::Choice(1)),
                ("deflicker_percentile", number(90.0)),
                ("deflicker_target_level", number(-2.5)),
            ],
            &raw(-0.67),
        );
        assert_eq!(
            settings,
            [
                Setting::new(EXPOSURE, "mode", Value::Choice(1)),
                Setting::new(EXPOSURE, "deflicker_percentile", Value::Float(90.0)),
                Setting::new(EXPOSURE, "deflicker_target_level", Value::Float(-2.5)),
            ]
        );
    }

    #[test]
    fn automatic_mode_leaves_the_black_to_the_white_it_computes() {
        let image = raw(-0.67);
        let black = Base::new(OPERATIONS, &image).float(EXPOSURE, "black") + 0.01;
        let level = [("exposure", number(8.0)), ("black_level", number(0.01))];
        // Manual mode at +8 EV holds +0.01 below its white, 2^−8.67.
        let manual = bind(&level, &image);
        let white = rendered_white(8.0, -0.67);
        assert!(manual.contains(&Setting::new(EXPOSURE, "black", Value::Float(white - 0.01))));
        // Automatic mode keeps the offset over the photo's black, whatever Exposure's white:
        // the operation bounds it at the white its histogram gives.
        let mut set = level.to_vec();
        set.push(("mode", ControlValue::Choice(1)));
        let automatic = bind(&set, &image);
        assert!(
            automatic.contains(&Setting::new(EXPOSURE, "black", Value::Float(black))),
            "{automatic:?}"
        );
    }

    #[test]
    fn the_computed_exposure_shows_in_automatic_mode_only() {
        let mut values = DEFINITION.defaults();
        let computed = DEFINITION.computed(&values);
        let [(computed, shown)] = computed.as_slice() else {
            panic!("one computed value: {computed:?}");
        };
        assert_eq!(
            **computed,
            Computed {
                id: COMPUTED_EXPOSURE,
                operation: EXPOSURE,
                unit: Some(Unit::Ev),
                decimals: 2,
            }
        );
        assert!(!shown);
        values.set("mode", ControlValue::Choice(1));
        assert_eq!(DEFINITION.computed(&values), [(*computed, true)]);
        // The operation computes it by the same id.
        assert_eq!(
            COMPUTED_EXPOSURE,
            apertrust_operations::exposure::COMPUTED_EXPOSURE
        );
    }

    /// darktable's exposure sliders (`src/iop/exposure.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/tools/exposure.md`'s slider table.
    /// None shows a factor; Percentile's "%" format gives none, its range above ±10.
    const DARKTABLE: [DarktableSlider; 5] = [
        DarktableSlider::new("exposure", (-18.0, 18.0), 3).soft(-3.0, 4.0),
        DarktableSlider::new("deflicker_percentile", (0.0, 100.0), 2),
        DarktableSlider::new("deflicker_target_level", (-18.0, 18.0), 2),
        DarktableSlider::new("black_level", (-1.0, 1.0), 4).soft(-0.1, 0.1),
        // Lightness, darktable's area exposure mapping's, `dt_bauhaus_slider_new_with_range`,
        // under Picker target here (decision 88).
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
    /// reading Picker target, Lightness and the reference, which replace the area exposure
    /// mapping's settings (`:866-973`; decision 88). The reference row's picker is
    /// darktable's measure mode's, an area of the same input that writes nothing.
    #[test]
    fn its_picker_is_darktable_s() {
        let darktable = [
            DarktablePicker {
                id: "exposure",
                slider: "exposure",
                kind: PickerKind::Area,
                denoise: false,
                output: false,
                writes: &["exposure", "black_level"],
                settings: &["target", "lightness", "reference_lightness"],
            },
            // On the reference row, no slider's.
            DarktablePicker {
                id: "reference",
                slider: "",
                kind: PickerKind::Area,
                denoise: false,
                output: false,
                writes: &[],
                settings: &[],
            },
        ];
        assert_eq!(crate::testing::pickers_on_sliders(&DEFINITION), darktable);
        assert_eq!(DEFINITION.picker("reference").unwrap().operation, EXPOSURE);
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
        let mut saved = ToolValues::new();
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
        // Version 3's values are its own.
        assert_eq!(Exposure.migrate(3, saved.clone()), saved);
    }

    #[test]
    fn before_version_3_highlight_preservation_mode_was_off() {
        // Not ported then, it took only off, which an older edit renders with.
        let mut saved = ToolValues::new();
        saved.set("exposure", number(1.0));
        for from in [1, 2] {
            let migrated = Exposure.migrate(from, saved.clone());
            assert_eq!(migrated.flag("compensate_hilite_pres"), Some(false));
            assert_eq!(migrated.flag("camera_bias"), None);
        }
        assert_eq!(
            Exposure.migrate(2, saved.clone()).number("exposure"),
            Some(1.0)
        );
        // Version 3 keeps the default, on.
        assert_eq!(Exposure.migrate(3, saved.clone()), saved);
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
            input_maximum: [1.0; 3],
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

    /// Picker target holding `target`, by its option's id, and Lightness at `lightness`.
    fn target(target: &str, lightness: f32) -> [(&'static str, ControlValue); 2] {
        let at = TARGETS.iter().position(|id| *id == target).unwrap();
        [
            ("target", ControlValue::Choice(at as u32)),
            ("lightness", number(lightness)),
        ]
    }

    /// Picker target holding Lightness, at `lightness`.
    fn toward(lightness: f32) -> [(&'static str, ControlValue); 2] {
        target("lightness", lightness)
    }

    fn pick(
        image: &ImageContext,
        rgb: [f32; 3],
        values: &Controls,
        settings: &[(&str, ControlValue)],
    ) -> Picked {
        Exposure
            .pick(
                "exposure",
                &reading(rgb),
                values,
                image,
                settings,
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
        // and without a bias, compensated and not, and with exposure held back, given
        // back and not.
        let cases = [
            (ImageContext::full(8, 8), true, true),
            (raw(0.0), true, true),
            (raw(-0.67), true, true),
            (raw(-0.67), false, true),
            (raw(1.3), true, true),
            (preserving(-0.67, 0.5), true, true),
            (preserving(-0.67, 0.5), true, false),
            (preserving(0.0, 2.0), false, true),
        ];
        for (image, compensated, given_back) in cases {
            let camera = if compensated {
                f64::from(camera_bias(&image))
            } else {
                0.0
            };
            let held = if given_back {
                f64::from(highlight_bias(&image))
            } else {
                0.0
            };
            for rgb in [[0.18, 0.18, 0.18], [0.02, 0.05, 0.01], [0.7, 0.4, 0.9]] {
                for target in [18.0, 50.0, 72.5, 100.0] {
                    let set = [
                        ("camera_bias", ControlValue::Bool(compensated)),
                        ("compensate_hilite_pres", ControlValue::Bool(given_back)),
                    ];
                    let values = values(&image, &set);
                    let picked = pick(&image, rgb, &values, &toward(target));
                    // −log2(Y / Y_target), plus the bias the operation takes off again
                    // and less the exposure it gives back.
                    let expected =
                        -(luminance(&image, rgb) / y_of(f64::from(target))).log2() + camera - held;
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
        let picked = pick(&image, [0.18; 3], &values(&image, &[]), &toward(50.0));
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
        let picked = pick(&image, rgb, &values(&image, &[]), &toward(50.0));
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
        let first = pick(&image, [0.3, 0.2, 0.1], &values(&image, &[]), &toward(60.0));
        let picked = first.writes.number("exposure");
        let again = pick(
            &image,
            [0.3, 0.2, 0.1],
            &values(&image, &[("exposure", number(picked))]),
            &toward(60.0),
        );
        assert!(again.writes.is_empty());
        assert_eq!(again.shown.patch, first.shown.patch);
    }

    #[test]
    fn its_pick_puts_a_black_at_or_above_the_new_white_darktable_s_margin_below_it() {
        // A dark area: the white 2^−e is Y / Y_target, below a black of 0.5.
        let image = ImageContext::full(8, 8);
        let rgb = [0.001; 3];
        let set = [("black_level", number(0.5))];
        let picked = pick(&image, rgb, &values(&image, &set), &toward(50.0));
        let white = luminance(&image, rgb) / y_of(50.0);
        let level = picked.writes.number("black_level");
        assert!(
            (f64::from(level) - (white - 0.01)).abs() < 1e-6,
            "{level} for {white}"
        );
        // Over a colour raw's own black, −2⁻¹², as Black level correction is.
        let image = raw(0.0);
        let picked = pick(&image, rgb, &values(&image, &set), &toward(50.0));
        let white = 2f32.powf(-picked.writes.number("exposure"));
        assert_eq!(
            picked.writes.number("black_level"),
            white - 0.01 + 1.0 / 4096.0
        );
        // A black below the new white is left as it is.
        let picked = pick(&image, [0.5; 3], &values(&image, &set), &toward(50.0));
        assert_eq!(picked.writes.get("black_level"), None);
    }

    #[test]
    fn its_pick_has_darktable_s_ends() {
        let image = ImageContext::full(8, 8);
        // A target lightness of 0 maps to no exposure: not finite, which the session
        // leaves out, keeping the patch.
        let picked = pick(&image, [0.18; 3], &values(&image, &[]), &toward(0.0));
        assert!(!picked.writes.number("exposure").is_finite());
        // An area of black or below: the white floored at 1e-20, so log2 has a value,
        // past Exposure's +18 EV, which the session holds there.
        let picked = pick(&image, [0.0; 3], &values(&image, &[]), &toward(50.0));
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

    /// Middle grey picks exactly what Lightness at 50 % picks, whatever Lightness holds, on
    /// every photo and area E50's and the correction's tests use (decision 88).
    #[test]
    fn middle_grey_picks_exactly_what_lightness_at_50_picks() {
        let images = [
            ImageContext::full(8, 8),
            raw(0.0),
            raw(-0.67),
            preserving(-0.67, 0.5),
        ];
        for image in &images {
            for rgb in [[0.05; 3], [0.18; 3], [0.7, 0.4, 0.9], [0.001; 3]] {
                for set in [&[][..], &[("black_level", number(0.5))][..]] {
                    let values = values(image, set);
                    let lightness = pick(image, rgb, &values, &toward(50.0));
                    for held in [0.0, 50.0, 92.0] {
                        let grey = pick(image, rgb, &values, &target("middle_grey", held));
                        assert_eq!(grey, lightness, "{rgb:?} with Lightness at {held}");
                    }
                }
            }
        }
        // Middle grey is the default, as no setting at all picks.
        let image = raw(-0.67);
        let values = values(&image, &[]);
        assert_eq!(
            pick(&image, [0.3, 0.2, 0.1], &values, &[]),
            pick(&image, [0.3, 0.2, 0.1], &values, &toward(50.0))
        );
    }

    /// The reference row's picker's reading of `rgb` at `values`.
    fn measure(image: &ImageContext, rgb: [f32; 3], values: &Controls) -> Picked {
        Exposure
            .pick(
                "reference",
                &reading(rgb),
                values,
                image,
                &[],
                None,
                None,
                PickKeys::default(),
            )
            .unwrap()
    }

    /// Picker target at Reference, with the reference at `lightness`, or none.
    fn toward_reference(lightness: Option<f32>) -> Vec<(&'static str, ControlValue)> {
        let mut settings = target("reference", 50.0).to_vec();
        if let Some(lightness) = lightness {
            settings.push(("reference_lightness", number(lightness)));
        }
        settings
    }

    /// Toward Reference, Exposure's pick is the correction toward the reference's L,
    /// exactly Lightness's at that L; with no reference, the patch alone and nothing
    /// written. An option it does not list picks as Middle grey.
    #[test]
    fn toward_reference_its_pick_takes_the_reference_s_lightness() {
        let image = raw(-0.67);
        let values = values(&image, &[]);
        for rgb in [[0.18, 0.12, 0.1], [0.05; 3], [0.7, 0.4, 0.9]] {
            for l in [12.5, 50.0, 62.0, 92.0] {
                let reference = pick(&image, rgb, &values, &toward_reference(Some(l)));
                assert_eq!(reference, pick(&image, rgb, &values, &toward(l)), "{l}");
            }
            let none = pick(&image, rgb, &values, &toward_reference(None));
            assert!(none.writes.is_empty());
            let lightness = pick(&image, rgb, &values, &toward(50.0));
            assert_eq!(none.shown.patch, lightness.shown.patch);
            assert_eq!(none.shown.last, None);
            assert!(lightness.shown.settings.is_empty());
        }
        let unknown = [("target", ControlValue::Choice(7))];
        let rgb = [0.18, 0.12, 0.1];
        assert_eq!(
            pick(&image, rgb, &values, &unknown),
            pick(&image, rgb, &values, &toward(50.0))
        );
    }

    /// The reference row's picker takes darktable's measure mode's arithmetic: the L the
    /// area renders at at the current exposure, the bias the operation takes off and the
    /// exposure it gives back included, held to 0 to 100, handed back as the reference,
    /// and writes nothing (E54). Picked toward on the same photo, it asks for no change.
    #[test]
    fn the_reference_picker_measures_the_area_s_lightness_and_writes_nothing() {
        for (image, compensated, exposure) in [
            (ImageContext::full(8, 8), false, 1.0),
            (raw(-0.67), true, 0.7),
            (raw(-0.67), false, -2.0),
            (preserving(-0.67, 0.5), true, 0.7),
        ] {
            let set = [
                ("exposure", number(exposure)),
                ("camera_bias", ControlValue::Bool(compensated)),
            ];
            // Highlight preservation mode is on, by default: what it gives back adds.
            let bias = if compensated {
                f64::from(camera_bias(&image))
            } else {
                0.0
            } - f64::from(highlight_bias(&image));
            let rgb = [0.18, 0.12, 0.1];
            let values = values(&image, &set);
            let measured = measure(&image, rgb, &values);
            assert!(measured.writes.is_empty());
            let y = luminance(&image, rgb) * (f64::from(exposure) - bias).exp2();
            let expected = 116.0 * y.cbrt() - 16.0;
            let [("reference_lightness", ControlValue::Number(l))] = measured.shown.settings[..]
            else {
                panic!("{:?}", measured.shown.settings)
            };
            assert!((f64::from(l) - expected).abs() < 1e-3, "{l} for {expected}");
            // Its patch is the input's, as Exposure's picker shows it.
            let input = pick(&image, rgb, &values, &toward(50.0));
            assert_eq!(measured.shown.patch, input.shown.patch);
            // Toward it on the same photo, Exposure stays where it is.
            let back = pick(&image, rgb, &values, &toward_reference(Some(l)));
            let written = back.writes.get("exposure").map_or(exposure, |v| match v {
                ControlValue::Number(n) => *n,
                _ => unreachable!(),
            });
            assert!(
                (written - exposure).abs() < 1e-4,
                "{written} for {exposure}"
            );
        }
        // Held to 0 to 100, as darktable's slider holds it.
        let image = ImageContext::full(8, 8);
        let bright = measure(
            &image,
            [4.0; 3],
            &values(&image, &[("exposure", number(4.0))]),
        );
        assert_eq!(
            bright.shown.settings,
            [("reference_lightness", ControlValue::Number(100.0))]
        );
    }

    /// Last pick (decision 88): before is measure mode's L of the area at the current
    /// exposure, after the target within 0.1 L where no bound holds the exposure, the
    /// exposure the change written; with nothing written, before and after alike, no
    /// change. The patches are greys of the two Ls.
    #[test]
    fn its_last_pick_says_where_the_area_was_and_lands() {
        let images = [ImageContext::full(8, 8), raw(0.0), raw(-0.67)];
        for image in &images {
            for rgb in [[0.18; 3], [0.05; 3], [0.7, 0.4, 0.9], [0.3, 0.2, 0.1]] {
                for target in [30.0, 50.0, 75.0] {
                    let values = values(image, &[]);
                    let picked = pick(image, rgb, &values, &toward(target));
                    let last = picked.shown.last.expect("a last pick");
                    let measured = measure(image, rgb, &values);
                    let [(_, ControlValue::Number(l))] = measured.shown.settings[..] else {
                        unreachable!()
                    };
                    // The photo's own black, −2⁻¹² on a colour raw, moves it a little.
                    assert!((last.before - l).abs() < 0.1, "{rgb:?}: {last:?} for {l}");
                    assert!(
                        (last.after - target).abs() < 0.1,
                        "{rgb:?}: {last:?} to {target}"
                    );
                    let written = picked.writes.number("exposure");
                    let change = written - values.number("exposure");
                    assert!((last.exposure - change).abs() < 1e-6, "{last:?}");
                    assert_eq!(last.patches[0], Patch::of_lch([last.before, 0.0, 0.0]));
                    assert_eq!(last.patches[1], Patch::of_lch([last.after, 0.0, 0.0]));
                    assert_eq!(picked.shown.warnings, PickWarnings::default());
                }
            }
        }
        // On an image with no black of its own, before is measure mode's exactly.
        let image = ImageContext::full(8, 8);
        let set = values(&image, &[("exposure", number(1.0))]);
        let last = pick(&image, [0.3, 0.2, 0.1], &set, &toward(50.0))
            .shown
            .last
            .unwrap();
        let measured = measure(&image, [0.3, 0.2, 0.1], &set);
        assert_eq!(
            measured.shown.settings,
            [("reference_lightness", ControlValue::Number(last.before))]
        );
        // Picked again where it landed: nothing written, nothing changed.
        let image = raw(-0.67);
        let first = pick(&image, [0.3, 0.2, 0.1], &values(&image, &[]), &toward(60.0));
        let there = values(
            &image,
            &[("exposure", first.writes.get("exposure").unwrap().clone())],
        );
        let again = pick(&image, [0.3, 0.2, 0.1], &there, &toward(60.0));
        let last = again.shown.last.unwrap();
        assert!(again.writes.is_empty());
        assert_eq!((last.exposure, last.before), (0.0, last.after));
    }

    /// Short (decision 88): where Exposure is held at an end of its ±18 EV and the area
    /// falls short of its target, with the L it reaches and whether the black moved; not
    /// where the area reaches its target.
    #[test]
    fn a_pick_held_at_exposure_s_end_is_short() {
        let image = ImageContext::full(8, 8);
        let values = values(&image, &[]);
        // Too dark for +18 EV: held there, the area short of 50.
        let dark = pick(&image, [1e-7; 3], &values, &toward(50.0));
        let short = dark.shown.warnings.short.expect("short");
        assert_eq!(
            (short.exposure, short.target, short.black),
            (18.0, 50.0, false)
        );
        let last = dark.shown.last.unwrap();
        assert_eq!(short.reached, last.after);
        assert!(short.reached < 50.0 - 1.0, "{short:?}");
        assert!((last.exposure - 18.0).abs() < 1e-6);
        // Too bright for −18 EV.
        let bright = pick(&image, [1e6; 3], &values, &toward(50.0));
        let short = bright.shown.warnings.short.expect("short");
        assert_eq!(short.exposure, -18.0);
        assert!(short.reached > 50.0 + 1.0, "{short:?}");
        // With a black held at or above the new white, the pick moves it too.
        let blacked = values_with_black(&image, 0.5);
        let moved = pick(&image, [1e-7; 3], &blacked, &toward(50.0));
        assert!(moved.writes.get("black_level").is_some());
        assert!(moved.shown.warnings.short.unwrap().black);
        // Within the range, none.
        let within = pick(&image, [0.18; 3], &values, &toward(50.0));
        assert_eq!(within.shown.warnings.short, None);
    }

    /// Exposure's controls at `image`'s defaults with Black level correction at `level`.
    fn values_with_black(image: &ImageContext, level: f32) -> Controls {
        values(image, &[("black_level", number(level))])
    }

    /// Clipped (decision 88): where any channel's maximum over the area reaches 0.99 × that
    /// channel's maximum at the input, and not below it: after white balance each has its
    /// own, so a green clipping alone at 1.0 is clipped beside a red that reaches 2.1.
    #[test]
    fn a_pick_of_clipped_pixels_says_so() {
        let image = raw(-0.67);
        let values = values(&image, &[]);
        let clipped = |maximum: [f32; 3], max: [f32; 3]| {
            let [r, g, b] = max;
            let mut reading = reading([0.2, 0.2, 0.2]);
            reading.input.max = [r, g, b, 1.0];
            reading.input_maximum = maximum;
            Exposure
                .pick(
                    "exposure",
                    &reading,
                    &values,
                    &image,
                    &toward(50.0),
                    None,
                    None,
                    PickKeys::default(),
                )
                .unwrap()
                .shown
                .warnings
                .clipped
        };
        let balanced = [2.1, 1.0, 1.6];
        // Each channel alone at 0.99 of its own maximum, and beyond it.
        assert!(clipped(balanced, [0.5, 0.99, 0.5]));
        assert!(clipped(balanced, [2.079, 0.5, 0.5]));
        assert!(clipped(balanced, [0.5, 0.5, 1.584]));
        assert!(clipped(balanced, [2.5, 0.5, 0.5]));
        // None at 0.98 of its own, though green's is above red's 0.98 × 2.1's share.
        assert!(!clipped(balanced, [2.058, 0.98, 1.568]));
        assert!(!clipped([1.0; 3], [0.98, 0.98, 0.98]));
        assert!(clipped([1.0; 3], [0.99, 0.2, 0.2]));
        // Clipped and short at once, each on its own.
        let mut reading = reading([1e-7; 3]);
        reading.input.max = [1.0, 0.0, 0.0, 1.0];
        let both = Exposure
            .pick(
                "exposure",
                &reading,
                &values,
                &image,
                &toward(50.0),
                None,
                None,
                PickKeys::default(),
            )
            .unwrap();
        assert!(both.shown.warnings.clipped && both.shown.warnings.short.is_some());
    }

    /// Exposure's range as the pick holds it is the control's.
    #[test]
    fn the_pick_holds_exposure_to_its_range() {
        let ControlKind::Number { min, max, .. } = DEFINITION.control("exposure").unwrap().kind
        else {
            unreachable!()
        };
        assert_eq!(EXPOSURE_RANGE, (min, max));
    }

    /// Exposure's picker is not ready toward Reference with no reference set; ready
    /// otherwise, as the reference's own picker always is.
    #[test]
    fn exposure_s_picker_waits_for_a_reference() {
        let ready =
            |picker, settings: &[(&str, ControlValue)]| Exposure.picker_ready(picker, settings);
        assert!(!ready("exposure", &toward_reference(None)));
        assert!(ready("exposure", &toward_reference(Some(62.0))));
        for option in ["middle_grey", "lightness"] {
            assert!(ready("exposure", &target(option, 50.0)));
        }
        assert!(ready("exposure", &[]));
        assert!(ready("reference", &toward_reference(None)));
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
