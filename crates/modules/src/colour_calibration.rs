// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c)
// Copyright (C) 2017-2023  darktable developers (ported from
// src/common/colorspaces_inline_conversions.h)
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

//! Colour calibration (`docs/modules/colour-calibration.md`): darktable's color
//! calibration as its page has it, every tab, the CAT tab first (decision 27).
//!
//! - **CAT:** the adaptation, unavailable while the illuminant is as shot in camera, which
//!   forces CAT16 (decision 30), then, unless it is none, the illuminant and what its type
//!   reads, as darktable's `_update_illuminants` shows them: an F or an LED source, a
//!   temperature, or a custom light's hue and chroma, a lossless view of its `x` and `y`
//!   ([`hue_chroma`]); then gamut compression, Clip negative RGB from gamut, and the area
//!   colour mapping, darktable's settings for its picker, which no edit stores, drawn
//!   disabled until the picker comes. Choosing an illuminant, a source or a temperature
//!   writes the light's `x`, `y` and temperature, and editing hue or chroma its
//!   temperature, as darktable's `gui_changed` and `_illum_xy_callback` do
//!   ([`Module::follow`]).
//! - **R, G and B:** the mixer's output rows, each from Input R, G and B, −2 to 2, the
//!   identity by default, and Normalise channels, which divides the row by its sum.
//! - **Colourfulness, Brightness and Grey:** the same per input channel, 0 by default:
//!   colourfulness moves colours away from grey or towards it, brightness scales each
//!   pixel by its channels' weights, and any grey weight off 0 makes the photo grey.
//! - **Calibrate with a colour checker,** under every tab: darktable's settings for its
//!   colour checker, which no edit stores, drawn disabled until the tool comes.
//!
//! Each control is its darktable parameter as it is, so the module has no transfer
//! function. It drives `channelmixerrgb`, which every colour raw runs (decision 27), and
//! owns every parameter of it. Its defaults are the operation's on every photo, As shot
//! on a colour raw, darktable's: the camera's own light, as daylight or Planckian where
//! one lies close, as a custom illuminant otherwise: a control at its default sets
//! nothing, and the module adds no operation, so it has no first step.
//! darktable shows its saturation algorithm (`version`) only for an edit made with an
//! older one, which no Apertrust edit is, so the module owns it with no control and leaves
//! it at version 3 (`docs/modules/README.md`, rule 12).

use apertrust_color::Adaptation;
use apertrust_color::illuminants::{self, Illuminant, TEMPERATURE_RANGE};
use apertrust_plugin::{ImageContext, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Coordinate, Definition, Row, Slider, Tab, ToolSetting, Unit,
    View,
};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct ColourCalibration;

/// The operation it drives.
const CHANNEL_MIXER: &str = "channelmixerrgb";

/// The CAT tab's controls, in `gui_init`'s order, each the `channelmixerrgb` parameter of
/// its name. `x` and `y`, the stored light, have no rows of their own: hue and chroma show
/// them.
pub const CAT: [&str; 9] = [
    "adaptation",
    "illuminant",
    "illum_fluo",
    "illum_led",
    "temperature",
    "x",
    "y",
    "gamut",
    "clip",
];

/// Each mixer control and the `channelmixerrgb` parameter it sets, tab by tab: the three
/// input channels of a row, darktable's `<row>[0..2]`, then its Normalise channels.
pub const PARAMS: [(&str, &str); 24] = [
    ("red_r", "red_0"),
    ("red_g", "red_1"),
    ("red_b", "red_2"),
    ("red_normalise", "normalize_R"),
    ("green_r", "green_0"),
    ("green_g", "green_1"),
    ("green_b", "green_2"),
    ("green_normalise", "normalize_G"),
    ("blue_r", "blue_0"),
    ("blue_g", "blue_1"),
    ("blue_b", "blue_2"),
    ("blue_normalise", "normalize_B"),
    ("colourfulness_r", "saturation_0"),
    ("colourfulness_g", "saturation_1"),
    ("colourfulness_b", "saturation_2"),
    ("colourfulness_normalise", "normalize_sat"),
    ("brightness_r", "lightness_0"),
    ("brightness_g", "lightness_1"),
    ("brightness_b", "lightness_2"),
    ("brightness_normalise", "normalize_light"),
    ("grey_r", "grey_0"),
    ("grey_g", "grey_1"),
    ("grey_b", "grey_2"),
    ("grey_normalise", "normalize_grey"),
];

/// The colourfulness algorithm, owned and left at darktable's version 3.
const VERSION: &str = "version";

/// The lossless view of a custom light: its hue and chroma.
pub const HUE_CHROMA: &str = "hue_chroma";

/// darktable's adaptations (`dt_adaptation_t`), by their values.
pub const ADAPTATIONS: [&str; 5] = [
    "linear_bradford",
    "cat16",
    "non_linear_bradford",
    "xyz",
    "none",
];

/// Every adaptation but none, with which darktable hides the illuminant and everything
/// that describes it (`_update_illuminants`).
const ADAPTING: [&str; 4] = ["linear_bradford", "cat16", "non_linear_bradford", "xyz"];

/// darktable's illuminants as its combobox lists them, those `dt_illuminant_t` describes:
/// the detection modes, which darktable compiles out, are not among them.
pub const ILLUMINANTS: [&str; 9] = [
    "pipeline",
    "a",
    "d",
    "e",
    "f",
    "led",
    "planckian",
    "custom",
    "camera",
];

/// Each of [`ILLUMINANTS`] as darktable's `dt_illuminant_t`, the operation's choice by its
/// value.
pub const ILLUMINANT_VALUES: [Illuminant; 9] = [
    Illuminant::Pipe,
    Illuminant::A,
    Illuminant::D,
    Illuminant::E,
    Illuminant::F,
    Illuminant::Led,
    Illuminant::BlackBody,
    Illuminant::Custom,
    Illuminant::Camera,
];

/// darktable's fluorescent and LED lights (`dt_illuminant_fluo_t`, `dt_illuminant_led_t`),
/// by their values.
const FLUORESCENT: [&str; 12] = [
    "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
];
const LED: [&str; 9] = ["b1", "b2", "b3", "b4", "b5", "bh1", "rgb1", "v1", "v2"];

/// A choice, all of whose options are ported.
const fn choice(id: &'static str, options: &'static [&'static str], default: u32) -> Control {
    Control {
        id,
        kind: ControlKind::Choice {
            options,
            default,
            unported: &[],
        },
        unit: None,
        unported: false,
    }
}

const fn number(id: &'static str, (min, max): (f32, f32), step: f32, default: f32) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min,
            max,
            step,
            default,
        },
        unit: None,
        unported: false,
    }
}

/// One input channel's weight: darktable's `COLOR_MIN` to `COLOR_MAX`, the schema's range
/// for every row, shown with darktable's three digits.
const fn weight(id: &'static str, default: f32) -> Control {
    number(id, (-2.0, 2.0), 0.01, default)
}

const fn toggle(id: &'static str, default: bool) -> Control {
    Control {
        id,
        kind: ControlKind::Bool { default },
        unit: None,
        unported: false,
    }
}

const fn normalise(id: &'static str) -> Control {
    toggle(id, false)
}

/// The controls, the CAT tab's then the mixer's. The CAT tab's defaults are darktable's
/// before it looks at the image, daylight at 5003 K under CAT16; a photo's are the
/// operation's for it ([`as_shot`]). The ranges are the operation's, darktable's but for
/// `x` and `y`, which darktable leaves unbounded and the operation bounds at the reach of
/// darktable's hue and chroma (decision 27). The steps are darktable's sliders'; `x` and
/// `y`, which no row shows, step by a ten-thousandth.
static CONTROLS: [Control; 33] = [
    choice("adaptation", &ADAPTATIONS, 1),
    choice("illuminant", &ILLUMINANTS, 2),
    choice("illum_fluo", &FLUORESCENT, 2),
    choice("illum_led", &LED, 4),
    Control {
        unit: Some(Unit::Kelvin),
        ..number("temperature", TEMPERATURE_RANGE, 1.0, 5003.0)
    },
    number("x", (-0.1, 1.2), 0.0001, 0.333),
    number("y", (0.0001, 2.0), 0.0001, 0.333),
    number("gamut", (0.0, 12.0), 0.05, 1.0),
    toggle("clip", true),
    weight("red_r", 1.0),
    weight("red_g", 0.0),
    weight("red_b", 0.0),
    normalise("red_normalise"),
    weight("green_r", 0.0),
    weight("green_g", 1.0),
    weight("green_b", 0.0),
    normalise("green_normalise"),
    weight("blue_r", 0.0),
    weight("blue_g", 0.0),
    weight("blue_b", 1.0),
    normalise("blue_normalise"),
    weight("colourfulness_r", 0.0),
    weight("colourfulness_g", 0.0),
    weight("colourfulness_b", 0.0),
    normalise("colourfulness_normalise"),
    weight("brightness_r", 0.0),
    weight("brightness_g", 0.0),
    weight("brightness_b", 0.0),
    normalise("brightness_normalise"),
    weight("grey_r", 0.0),
    weight("grey_g", 0.0),
    weight("grey_b", 0.0),
    normalise("grey_normalise"),
];

/// A custom light's hue and chroma, as darktable's `illum_x` and `illum_y` show it: a hue
/// from 0 to 360°, and a chroma from 0 to 300 %, `dt_bauhaus_slider_set_hard_max`'s, each
/// stepping by darktable's 1.
static VIEWS: [View; 1] = [View {
    id: HUE_CHROMA,
    controls: &["x", "y"],
    coordinates: &[
        Coordinate {
            id: "hue",
            min: 0.0,
            max: 360.0,
            step: 1.0,
            unit: Some(Unit::Degrees),
        },
        Coordinate {
            id: "chroma",
            min: 0.0,
            max: 300.0,
            step: 1.0,
            unit: Some(Unit::Percent),
        },
    ],
}];

/// A choice's row, as a list.
const fn list(control: &'static str) -> Row {
    Row::Choice {
        control,
        style: ChoiceStyle::List,
    }
}

/// darktable's area colour mapping, under the CAT tab: what its picker does with the area
/// picked, whether the target allows for the channel mixing, and the colour it maps the
/// area to, darktable's settings (`darkroom/modules/channelmixerrgb/use_mixing`,
/// `lightness`, `hue` and `chroma` in its darktablerc, and an area mode `gui_update` resets
/// to correction), at their defaults there. The input's and the target's patches, the
/// input's L, h and c readout and the title's "(active)" come with the picker.
const MAPPING: Row = Row::Group {
    id: "area_colour_mapping",
    rows: &[
        Row::ToolSetting(ToolSetting {
            id: "area_mode",
            kind: ControlKind::Choice {
                options: &["correction", "measure"],
                default: 0,
                unported: &[],
            },
            decimals: 0,
        }),
        Row::ToolSetting(ToolSetting {
            id: "channel_mixing",
            kind: ControlKind::Bool { default: true },
            decimals: 0,
        }),
        Row::Label { id: "target" },
        Row::ToolSetting(ToolSetting {
            id: "target_lightness",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 50.0,
            },
            decimals: 1,
        }),
        Row::ToolSetting(ToolSetting {
            id: "target_hue",
            kind: ControlKind::Number {
                min: 0.0,
                max: 360.0,
                step: 1.0,
                default: 0.0,
            },
            decimals: 1,
        }),
        Row::ToolSetting(ToolSetting {
            id: "target_chroma",
            kind: ControlKind::Number {
                min: 0.0,
                max: 128.0,
                step: 1.0,
                default: 0.0,
            },
            decimals: 1,
        }),
    ],
};

/// The CAT tab, as darktable's `gui_init` builds it and `_update_illuminants` shows it:
/// the adaptation; with any but none, the illuminant, then the F source for F, the LED
/// source for LED, the temperature, its track over 3000 to 7000 K, for daylight and
/// Planckian, and hue and chroma, chroma's track over 0 to 100 %, for custom; then gamut
/// compression, its track over 0 to 4, clipping and the area colour mapping. A hue has no
/// direction, so its track has no fill, as darktable's `illum_x` has none. The CCT readout
/// with the illuminant's colour and the picker, darktable's row after the adaptation, waits
/// for a row that draws a text and a swatch the module computes.
static CAT_TAB: [Row; 5] = [
    list("adaptation"),
    Row::When {
        control: "adaptation",
        is: &ADAPTING,
        rows: &[
            list("illuminant"),
            Row::When {
                control: "illuminant",
                is: &["f"],
                rows: &[list("illum_fluo")],
            },
            Row::When {
                control: "illuminant",
                is: &["led"],
                rows: &[list("illum_led")],
            },
            Row::When {
                control: "illuminant",
                is: &["d", "planckian"],
                rows: &[Row::Slider(
                    Slider::new("temperature").decimals(0).soft(3000.0, 7000.0),
                )],
            },
            Row::When {
                control: "illuminant",
                is: &["custom"],
                rows: &[
                    Row::Slider(Slider::new("hue").decimals(1).no_fill()),
                    Row::Slider(Slider::new("chroma").decimals(1).soft(0.0, 100.0)),
                ],
            },
        ],
    },
    Row::Slider(Slider::new("gamut").decimals(2).soft(0.0, 4.0)),
    Row::Toggle { control: "clip" },
    MAPPING,
];

/// A mixer tab's rows, as darktable's notebook page draws them: Input R, G and B, with
/// darktable's three digits and the page's soft range, then Normalise channels.
const fn page([r, g, b, n]: [&'static str; 4], soft: Option<(f32, f32)>) -> [Row; 4] {
    [
        input(r, soft),
        input(g, soft),
        input(b, soft),
        Row::Toggle { control: n },
    ]
}

/// One input channel's row.
const fn input(control: &'static str, soft: Option<(f32, f32)>) -> Row {
    Row::Slider(Slider {
        soft,
        ..Slider::new(control).decimals(3)
    })
}

/// darktable's soft ranges: none on the R, G and B pages, −1 to 1 on colourfulness and
/// brightness, 0 to 1 on grey.
static RED: [Row; 4] = page(["red_r", "red_g", "red_b", "red_normalise"], None);
static GREEN: [Row; 4] = page(["green_r", "green_g", "green_b", "green_normalise"], None);
static BLUE: [Row; 4] = page(["blue_r", "blue_g", "blue_b", "blue_normalise"], None);
static COLOURFULNESS: [Row; 4] = page(
    [
        "colourfulness_r",
        "colourfulness_g",
        "colourfulness_b",
        "colourfulness_normalise",
    ],
    Some((-1.0, 1.0)),
);
static BRIGHTNESS: [Row; 4] = page(
    [
        "brightness_r",
        "brightness_g",
        "brightness_b",
        "brightness_normalise",
    ],
    Some((-1.0, 1.0)),
);
static GREY: [Row; 4] = page(
    ["grey_r", "grey_g", "grey_b", "grey_normalise"],
    Some((0.0, 1.0)),
);

/// darktable's colour checker charts (`gui_init`), in its order.
const CHARTS: [&str; 7] = [
    "xrite_24_2000",
    "xrite_24_2014",
    "spyder_24",
    "spyder_24_v2",
    "spyder_48",
    "spyder_48_v2",
    "spyder_photo",
];

/// What darktable's colour checker optimises for, in its order.
const OPTIMISATIONS: [&str; 8] = [
    "none",
    "neutral",
    "saturated",
    "skin",
    "foliage",
    "sky",
    "average_delta_e",
    "maximum_delta_e",
];

/// darktable's "calibrate with a color checker" section, under its notebook: the chart,
/// what the fit favours and how much of each patch is read, darktable's settings kept in
/// its darktablerc (`darkroom/modules/channelmixerrgb/colorchecker`, `optimization` and
/// `safety`), at their defaults there. Its ΔE readout and its Validate, Recompute and Accept
/// buttons, with the chart's frame on the photo, come with the tool.
static CHECKER: [Row; 1] = [Row::Group {
    id: "colour_checker",
    rows: &[
        Row::ToolSetting(ToolSetting {
            id: "chart",
            kind: ControlKind::Choice {
                options: &CHARTS,
                default: 0,
                unported: &[],
            },
            decimals: 0,
        }),
        Row::ToolSetting(ToolSetting {
            id: "optimise_for",
            kind: ControlKind::Choice {
                options: &OPTIMISATIONS,
                default: 0,
                unported: &[],
            },
            decimals: 0,
        }),
        Row::ToolSetting(ToolSetting {
            id: "patch_scale",
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.5,
            },
            decimals: 3,
        }),
    ],
}];

static DEFINITION: Definition = Definition {
    id: "channelmixerrgb",
    version: 1,
    // Background only until Phase 4's layers (`docs/modules/colour-calibration.md`).
    layers: false,
    auto: false,
    controls: &CONTROLS,
    tabs: &[
        Tab {
            id: Some("cat"),
            rows: &CAT_TAB,
        },
        Tab {
            id: Some("red"),
            rows: &RED,
        },
        Tab {
            id: Some("green"),
            rows: &GREEN,
        },
        Tab {
            id: Some("blue"),
            rows: &BLUE,
        },
        Tab {
            id: Some("colourfulness"),
            rows: &COLOURFULNESS,
        },
        Tab {
            id: Some("brightness"),
            rows: &BRIGHTNESS,
        },
        Tab {
            id: Some("grey"),
            rows: &GREY,
        },
    ],
    before: &[],
    after: &CHECKER,
    presets: &[],
    views: &VIEWS,
};

/// Every parameter of `channelmixerrgb`: the CAT tab's, the mixer's, and `version`.
static OWNS: [Owned; CAT.len() + PARAMS.len() + 1] = owns();

const fn owns() -> [Owned; CAT.len() + PARAMS.len() + 1] {
    let mut owns = [Owned::new(CHANNEL_MIXER, VERSION); CAT.len() + PARAMS.len() + 1];
    let mut i = 0;
    while i < CAT.len() {
        owns[i] = Owned::new(CHANNEL_MIXER, CAT[i]);
        i += 1;
    }
    while i < CAT.len() + PARAMS.len() {
        owns[i] = Owned::new(CHANNEL_MIXER, PARAMS[i - CAT.len()].1);
        i += 1;
    }
    owns
}

impl Module for ColourCalibration {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    /// The definition's, but the CAT tab's light and adaptation are the photo's: As shot.
    fn defaults(&self, image: &ImageContext) -> Controls {
        let mut defaults = DEFINITION.defaults();
        let shot = as_shot(image);
        let [x, y] = shot.xy;
        defaults.set(
            "adaptation",
            ControlValue::Choice(adaptation(shot.adaptation)),
        );
        defaults.set(
            "illuminant",
            ControlValue::Choice(illuminant(shot.illuminant)),
        );
        defaults.set("temperature", ControlValue::Number(shot.temperature));
        defaults.set("x", ControlValue::Number(x));
        defaults.set("y", ControlValue::Number(y));
        defaults
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// `version`, the last parameter the module owns, left at darktable's version 3.
    fn fixed(&self) -> &'static [Owned] {
        &OWNS[OWNS.len() - 1..]
    }

    /// The adaptation can change nothing while the illuminant is as shot in camera, which
    /// forces CAT16, so darktable makes it insensitive then (`gui_changed`'s
    /// `gtk_widget_set_sensitive(g->adaptation, p->illuminant != DT_ILLUMINANT_CAMERA)`,
    /// decision 30); it still takes a value, which the camera's light ignores.
    fn available(&self, control: &str, values: &Controls, _: &ImageContext) -> bool {
        control != "adaptation"
            || ILLUMINANT_VALUES.get(values.choice("illuminant") as usize)
                != Some(&Illuminant::Camera)
    }

    /// Each control off its default as its parameter, the value as it is, an illuminant by
    /// its darktable value: the operation already runs, so nothing else is needed, and
    /// nothing at all at the defaults.
    fn bind(&self, values: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        let cat = CAT.iter().map(|&control| (control, control));
        cat.chain(PARAMS)
            .filter_map(|(control, param)| {
                let value = match values.get(control)? {
                    ControlValue::Number(value) => Value::Float(*value),
                    ControlValue::Bool(on) => Value::Bool(*on),
                    ControlValue::Choice(index) if control == "illuminant" => {
                        let illuminant = ILLUMINANT_VALUES.get(*index as usize)?;
                        Value::Choice(illuminant.to_darktable() as u32)
                    }
                    ControlValue::Choice(index) => Value::Choice(*index),
                    ControlValue::Points(_) => return None,
                };
                Some(Setting::new(CHANNEL_MIXER, param, value))
            })
            .collect()
    }

    /// Hue and chroma of the light `x` and `y` hold; nothing else is read, the camera
    /// never, so the app shows a pending change as the session will.
    fn view(&self, view: &str, values: &Controls, _: &ImageContext) -> Option<Vec<f32>> {
        (view == HUE_CHROMA).then(|| {
            let light = [values.number("x"), values.number("y")].map(f64::from);
            hue_chroma(light).map(|c| c as f32).to_vec()
        })
    }

    /// The light hue and chroma give, as `x` and `y`: [`Self::view`]'s exact inverse, but
    /// for a rounding.
    fn unview(&self, view: &str, coordinates: &[f32], _: &ImageContext) -> Option<Controls> {
        let (HUE_CHROMA, &[hue, chroma]) = (view, coordinates) else {
            return None;
        };
        let [x, y] = from_hue_chroma([hue, chroma].map(f64::from));
        let mut values = Controls::new();
        values.set("x", ControlValue::Number(x as f32));
        values.set("y", ControlValue::Number(y as f32));
        Some(values)
    }

    /// darktable's `gui_changed` and `_illum_xy_callback`, which keep every control that
    /// describes the light describing the same one (decision 27):
    ///
    /// - choosing an illuminant, an F or LED source or a temperature writes `x` and `y` to
    ///   the light they describe (`illuminant_to_xy`), unless the illuminant is custom or
    ///   as shot in camera, and the temperature to the light's correlated colour
    ///   temperature (`_check_if_close_to_daylight`), unless it is daylight, Planckian or
    ///   as shot in camera;
    /// - choosing as shot in camera first writes `x` and `y` from the camera, where the
    ///   photo records its light, at its temperature, and the adaptation to CAT16, which
    ///   as shot in camera forces;
    /// - editing `x` or `y`, which hue and chroma do, writes the temperature (`xy_to_CCT`,
    ///   `CCT_reverse_lookup` below 3000 K).
    ///
    /// darktable's memory of the last daylight and Planckian temperatures of its session
    /// is not mirrored (decision 27): choosing daylight or Planckian takes the stored
    /// temperature, the last light's. darktable also reads the camera's light when as shot
    /// in camera is left; this has the values alone, which choosing it set to the camera's
    /// light, so leaving it keeps that light, as darktable's does, unless a script moved
    /// `x`, `y` or the adaptation meanwhile. A temperature is held to its control's range,
    /// as the operation's defaults hold one, where darktable stores the correlated colour
    /// temperature whatever it is.
    fn follow(&self, control: &str, values: &Controls, image: &ImageContext) -> Controls {
        let Some(chosen) = ILLUMINANT_VALUES.get(values.choice("illuminant") as usize) else {
            return Controls::new();
        };
        let mut xy = [values.number("x"), values.number("y")];
        let mut temperature = values.number("temperature");
        let mut writes = Controls::new();
        let mut write = |id: &'static str, value: ControlValue| {
            if id != control {
                writes.set(id, value);
            }
        };
        match control {
            "illuminant" | "illum_fluo" | "illum_led" | "temperature" => {
                if control == "illuminant" && *chosen == Illuminant::Camera {
                    if let Some(camera) = camera_light(image) {
                        xy = camera;
                        write("x", ControlValue::Number(xy[0]));
                        write("y", ControlValue::Number(xy[1]));
                    }
                    temperature = daylight_temperature(xy).unwrap_or(temperature);
                    write("temperature", ControlValue::Number(temperature));
                    write(
                        "adaptation",
                        ControlValue::Choice(adaptation(Adaptation::Cat16)),
                    );
                }
                if !matches!(chosen, Illuminant::Custom | Illuminant::Camera) {
                    let fluorescent = values.choice("illum_fluo") as usize;
                    let led = values.choice("illum_led") as usize;
                    if let Some(light) =
                        illuminants::illuminant_to_xy(*chosen, temperature, fluorescent, led, None)
                    {
                        xy = light;
                        write("x", ControlValue::Number(xy[0]));
                        write("y", ControlValue::Number(xy[1]));
                    }
                }
                let described = matches!(
                    chosen,
                    Illuminant::D | Illuminant::BlackBody | Illuminant::Camera
                );
                if let (false, Some(temperature)) = (described, daylight_temperature(xy)) {
                    write("temperature", ControlValue::Number(temperature));
                }
            }
            "x" | "y" => {
                if let Some(temperature) = light_temperature(xy) {
                    write("temperature", ControlValue::Number(temperature));
                }
            }
            _ => {}
        }
        writes
    }
}

/// An adaptation as its control's option, its darktable value.
fn adaptation(adaptation: Adaptation) -> u32 {
    adaptation.to_darktable() as u32
}

/// An illuminant as its control's option. The defaults and the follow writes give only
/// those darktable lists.
fn illuminant(illuminant: Illuminant) -> u32 {
    ILLUMINANT_VALUES
        .iter()
        .position(|&listed| listed == illuminant)
        .expect("an illuminant darktable lists") as u32
}

/// The CAT tab's parameters an unedited photo renders with.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AsShot {
    illuminant: Illuminant,
    xy: [f32; 2],
    temperature: f32,
    adaptation: Adaptation,
}

/// As shot: colour calibration's defaults for the photo (`docs/ports/channelmixerrgb.md`,
/// Defaults), darktable's `reload_defaults`. This crate cannot call the operation's
/// `image_defaults`, so this restates it; the module's tests hold the two equal, bit for
/// bit.
///
/// - A colour raw: in `x` and `y` the light its as-shot multipliers neutralise at the
///   camera's reference ([`camera_light`]), or darktable's stored 0.333, 0.333 where it
///   records no white balance under a camera matrix, described as
///   `_check_if_close_to_daylight` describes it: daylight or Planckian at its correlated
///   colour temperature where either lies within 0.005 of it in darktable's UCS, custom
///   otherwise, under CAT16.
/// - A monochrome raw: the pipeline's own illuminant without adaptation, darktable's plain
///   channel mixer.
/// - Any other image: darktable's defaults before it looks at the image, daylight at
///   5003 K under CAT16.
fn as_shot(image: &ImageContext) -> AsShot {
    let unedited = AsShot {
        illuminant: Illuminant::D,
        xy: [0.333, 0.333],
        temperature: 5003.0,
        adaptation: Adaptation::Cat16,
    };
    let Some(camera) = &image.camera else {
        return unedited;
    };
    if camera.cfa.is_none() {
        return AsShot {
            illuminant: Illuminant::Pipe,
            adaptation: Adaptation::Rgb,
            ..unedited
        };
    }
    let (min, max) = TEMPERATURE_RANGE;
    let xy = camera_light(image).unwrap_or(unedited.xy);
    let (temperature, illuminant) = illuminants::closest_cct_model(xy[0], xy[1]);
    AsShot {
        illuminant,
        xy,
        temperature: temperature.clamp(min, max),
        adaptation: Adaptation::Cat16,
    }
}

/// The light a colour raw's as-shot white balance neutralises, at the camera's reference,
/// where the raw stage leaves the image unless White balance moves it (decision 27):
/// darktable's `find_temperature_from_raw_coeffs` with no correction (`custom_wb` 1). None
/// on a photo without a camera matrix or normal multipliers, or that is no colour raw.
fn camera_light(image: &ImageContext) -> Option<[f32; 2]> {
    let camera = image.camera.as_ref()?;
    camera.cfa.as_ref()?;
    let matrix = camera.xyz_to_camera?;
    let &[r, g, b, ..] = camera.white_balance.as_deref()? else {
        return None;
    };
    illuminants::camera_illuminant_xy(&matrix, [r, g, b])
}

/// `_check_if_close_to_daylight`'s correlated colour temperature of a light, held to the
/// temperature's range; none where darktable's fit gives no number.
fn daylight_temperature([x, y]: [f32; 2]) -> Option<f32> {
    within(illuminants::closest_cct_model(x, y).0)
}

/// `_illum_xy_callback`'s: `xy_to_CCT`, or below 3000 K, where its fit fails,
/// `CCT_reverse_lookup`, held to the temperature's range.
fn light_temperature([x, y]: [f32; 2]) -> Option<f32> {
    let temperature = illuminants::xy_to_cct(x, y);
    within(if temperature < 3000.0 {
        illuminants::cct_reverse_lookup(x, y)
    } else {
        temperature
    })
}

/// A temperature held to the control's range, as the operation's defaults hold one; none
/// for NaN.
fn within(temperature: f32) -> Option<f32> {
    let (min, max) = TEMPERATURE_RANGE;
    (!temperature.is_nan()).then(|| temperature.clamp(min, max))
}

// The view's maths, in f64 so each inverts the other to f32's rounding.

/// darktable's D50 in CIE 1976 u′v′ (`dt_xyY_to_Luv`).
const D50_UV: [f64; 2] = [0.20915914598542354, 0.488075320769787];

/// u* and v* per unit of u′ and v′ at L* 100, which an illuminant at Y = 1 has: 13 L*.
const LUV: f64 = 1300.0;

/// Hue, in degrees from 0 to 360, and chroma of a light, as darktable's CAT tab shows a
/// custom illuminant (`dt_xyY_to_Lch` of x, y at Y = 1): CIE LCh(uv) around D50. A light
/// at D50 has no hue, which reads 0.
pub fn hue_chroma([x, y]: [f64; 2]) -> [f64; 2] {
    let denominator = -2.0 * x + 12.0 * y + 3.0;
    let u = LUV * (4.0 * x / denominator - D50_UV[0]);
    let v = LUV * (9.0 * y / denominator - D50_UV[1]);
    let hue = v.atan2(u).to_degrees();
    [if hue < 0.0 { hue + 360.0 } else { hue }, u.hypot(v)]
}

/// The light for a hue and a chroma: [`hue_chroma`]'s inverse (`dt_Lch_to_xyY`).
pub fn from_hue_chroma([hue, chroma]: [f64; 2]) -> [f64; 2] {
    let (sin, cos) = hue.to_radians().sin_cos();
    let u = chroma * cos / LUV + D50_UV[0];
    let v = chroma * sin / LUV + D50_UV[1];
    let denominator = 6.0 * u - 16.0 * v + 12.0;
    [9.0 * u / denominator, 4.0 * v / denominator]
}

#[cfg(test)]
mod tests {
    use apertrust_operations::{OPERATIONS, channelmixerrgb};
    use apertrust_plugin::{Camera, ParamValues};

    use super::*;
    use crate::check;
    use crate::testing::DarktableSlider;

    fn operation() -> &'static dyn apertrust_plugin::Operation {
        apertrust_operations::operation(CHANNEL_MIXER).unwrap()
    }

    /// The image contexts the operation's defaults tell apart: an image that is not a raw;
    /// colour raws with a white balance, under the sample's matrix, the Canon EOS 40D's of
    /// darktable's test 0174 (darktable's matrix and the as-shot multipliers), whose light
    /// darktable snaps to daylight, and another, with none, and with no matrix; and a
    /// monochrome raw.
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
                white_balance: Some(vec![2.115_234_4, 1.0, 1.398_437_5]),
                xyz_to_camera: Some([
                    [0.6071, -0.0747, -0.0856],
                    [-0.7653, 1.5365, 0.2441],
                    [-0.2025, 0.2553, 0.7315],
                ]),
                ..camera.clone()
            }),
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
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&ColourCalibration.defaults(image));
        ColourCalibration.bind(&values, image, &Base::new(OPERATIONS, image))
    }

    /// Every parameter of the operation's defaults for `image`, resolved.
    fn resolved(image: &ImageContext) -> ParamValues {
        operation()
            .schema()
            .resolve_over(&operation().defaults(image), &ParamValues::new())
            .unwrap()
    }

    /// A control's value as the parameter it sets holds it.
    fn as_param(control: &str, value: &ControlValue) -> Value {
        match value {
            ControlValue::Number(value) => Value::Float(*value),
            ControlValue::Bool(on) => Value::Bool(*on),
            ControlValue::Choice(index) if control == "illuminant" => {
                Value::Choice(ILLUMINANT_VALUES[*index as usize].to_darktable() as u32)
            }
            ControlValue::Choice(index) => Value::Choice(*index),
            other => panic!("{control}: {other:?}"),
        }
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        ColourCalibration.definition().check().unwrap();
        let (accepted, rejected) = check::accept(&[&ColourCalibration], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in images() {
            let problems = check::sweep(&ColourCalibration, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn each_control_is_its_parameter_with_its_range_and_darktable_s_default() {
        let schema = operation().schema();
        let params = CAT.iter().map(|&control| (control, control)).chain(PARAMS);
        assert_eq!(CONTROLS.len(), CAT.len() + PARAMS.len());
        for (control, (id, param)) in CONTROLS.iter().zip(params) {
            assert_eq!(control.id, id);
            let found = schema.param(param).unwrap();
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
                ) => assert_eq!((min, max, default), (lowest, highest, its), "{id}"),
                (ControlKind::Bool { default }, apertrust_plugin::Kind::Bool { default: its }) => {
                    assert_eq!(default, its, "{id}");
                }
                (
                    ControlKind::Choice {
                        options, default, ..
                    },
                    apertrust_plugin::Kind::Choice {
                        default: its,
                        choices,
                    },
                ) => {
                    let default = as_param(id, &ControlValue::Choice(default));
                    assert_eq!(default, Value::Choice(its), "{id}");
                    // Every option is one of the operation's, in its order: an illuminant
                    // by its darktable value, every other choice by its place.
                    let values: Vec<u32> = (0..options.len() as u32)
                        .map(|i| match as_param(id, &ControlValue::Choice(i)) {
                            Value::Choice(value) => value,
                            other => panic!("{other:?}"),
                        })
                        .collect();
                    assert!(values.windows(2).all(|pair| pair[0] < pair[1]), "{id}");
                    assert!(values.iter().all(|&v| (v as usize) < choices.len()), "{id}");
                    if id != "illuminant" {
                        assert_eq!(options.len(), choices.len(), "{id}");
                    }
                }
                other => panic!("{id}: {other:?}"),
            }
        }
        // The illuminants darktable lists: all but the detection modes.
        let listed: Vec<i32> = ILLUMINANT_VALUES.iter().map(|i| i.to_darktable()).collect();
        assert_eq!(listed, [0, 1, 2, 3, 4, 5, 6, 7, 10]);
    }

    /// darktable's color calibration sliders (`src/iop/channelmixerrgb.c`, the params
    /// and `gui_init`), in its parameters' units: `docs/modules/colour-calibration.md`'s
    /// slider table.
    const DARKTABLE: [DarktableSlider; 26] = [
        // The CAT tab. `temperature`, from params, `TEMP_MIN` to `TEMP_MAX`, its soft range
        // 3000 to 7000 K, no digits.
        DarktableSlider::new("temperature", (1667.0, 25000.0), 0).soft(3000.0, 7000.0),
        // `illum_x`, a hue: `dt_bauhaus_slider_new_with_range_and_feedback(…, 0,
        // ILLUM_X_MAX, 0, 0, 1, 0)`, formatted "°".
        DarktableSlider::new("hue", (0.0, 360.0), 1),
        // `illum_y`, a chroma: `dt_bauhaus_slider_new_with_range(…, 0, 100, 0, 0, 1)`,
        // formatted "%" with no factor, its hard maximum then raised to `ILLUM_Y_MAX`.
        DarktableSlider::new("chroma", (0.0, 300.0), 1).soft(0.0, 100.0),
        // `gamut`, from params, its soft maximum 4.
        DarktableSlider::new("gamut", (0.0, 12.0), 2).soft(0.0, 4.0),
        // The area colour mapping's target, `dt_bauhaus_slider_new_with_range`:
        // `LIGHTNESS_MAX`, `HUE_MAX` and `CHROMA_MAX`.
        DarktableSlider::new("target_lightness", (0.0, 100.0), 1),
        DarktableSlider::new("target_hue", (0.0, 360.0), 1),
        DarktableSlider::new("target_chroma", (0.0, 128.0), 1),
        // The mixer's tabs, `NOTEBOOK_PAGE`'s three digits.
        weight("red_r"),
        weight("red_g"),
        weight("red_b"),
        weight("green_r"),
        weight("green_g"),
        weight("green_b"),
        weight("blue_r"),
        weight("blue_g"),
        weight("blue_b"),
        weight("colourfulness_r").soft(-1.0, 1.0),
        weight("colourfulness_g").soft(-1.0, 1.0),
        weight("colourfulness_b").soft(-1.0, 1.0),
        weight("brightness_r").soft(-1.0, 1.0),
        weight("brightness_g").soft(-1.0, 1.0),
        weight("brightness_b").soft(-1.0, 1.0),
        weight("grey_r").soft(0.0, 1.0),
        weight("grey_g").soft(0.0, 1.0),
        weight("grey_b").soft(0.0, 1.0),
        // The colour checker's patch scale, `dt_bauhaus_slider_new_with_range`.
        DarktableSlider::new("patch_scale", (0.0, 1.0), 3),
    ];

    /// An input channel's weight, `COLOR_MIN` to `COLOR_MAX`, shown as it is with
    /// `NOTEBOOK_PAGE`'s three digits.
    const fn weight(id: &'static str) -> DarktableSlider {
        DarktableSlider::new(id, (-2.0, 2.0), 3)
    }

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences = crate::testing::slider_differences(
            &ColourCalibration,
            &DEFINITION.defaults(),
            &DARKTABLE,
        );
        assert!(differences.is_empty(), "{differences:#?}");
    }

    #[test]
    fn its_defaults_are_the_operation_s_on_every_photo() {
        // An unedited photo renders the operation's defaults, so the module's zero is
        // exactly what every photo runs: As shot's light and adaptation, the identity
        // mixer, every weight at 0, no normalisation, darktable's version 3.
        for image in images() {
            let resolved = resolved(&image);
            let controls = ColourCalibration.defaults(&image);
            let params = CAT.iter().map(|&control| (control, control)).chain(PARAMS);
            for (control, param) in params {
                let default = as_param(control, controls.get(control).unwrap());
                assert_eq!(resolved.get(param), Some(default), "{control}");
            }
            assert_eq!(resolved.get(VERSION), Some(Value::Choice(2)));
        }
    }

    #[test]
    fn as_shot_is_the_light_and_adaptation_the_operation_renders_an_unedited_photo_with() {
        for image in images() {
            let settings = channelmixerrgb::settings(&resolved(&image));
            let shot = as_shot(&image);
            // Bit for bit, the operation's own: its parameters and the light `prepare`
            // takes from them, a custom illuminant's x and y or its model's chromaticity.
            assert_eq!(
                (shot.illuminant, shot.xy, shot.temperature, shot.adaptation),
                (
                    settings.illuminant,
                    [settings.x, settings.y],
                    settings.temperature,
                    settings.adaptation
                ),
                "{:?}",
                image
                    .camera
                    .as_ref()
                    .map(|c| (&c.white_balance, c.cfa.is_some()))
            );
            let light = channelmixerrgb::illuminant_xy(&settings, None);
            let rendered = illuminants::illuminant_to_xy(
                shot.illuminant,
                shot.temperature,
                FLUORESCENT_DEFAULT,
                LED_DEFAULT,
                None,
            );
            assert_eq!(rendered.unwrap_or(shot.xy), light);
        }
        // On a colour raw with a white balance, the camera's own light in `x` and `y`, and
        // darktable's description of it: on the Canon EOS 40D of darktable's test 0174,
        // daylight at 3927.06 K, as darktable's defaults stored it there.
        for image in &images()[1..3] {
            assert_eq!(Some(as_shot(image).xy), camera_light(image));
        }
        let shot = as_shot(&images()[2]);
        assert_eq!(shot.illuminant, Illuminant::D);
        assert!((shot.xy[0] - 0.381_967_5).abs() < 2e-7 && (shot.xy[1] - 0.3699879).abs() < 2e-7);
        assert!((shot.temperature - 3927.0596).abs() < 0.01);
    }

    /// darktable's default F and LED sources, F3 and B5.
    const FLUORESCENT_DEFAULT: usize = 2;
    const LED_DEFAULT: usize = 4;

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        for image in images() {
            assert!(bind(&[], &image).is_empty());
            // Every control written at its default for the photo is still nothing to set.
            let defaults = ColourCalibration.defaults(&image);
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
        let defaults = ColourCalibration.defaults(&image);
        let params = CAT.iter().map(|&control| (control, control)).chain(PARAMS);
        for (control, param) in params {
            let value = match (control, defaults.get(control).unwrap()) {
                (_, ControlValue::Number(_)) if control == "temperature" => {
                    ControlValue::Number(6500.0)
                }
                (_, ControlValue::Number(_)) => ControlValue::Number(0.25),
                (_, ControlValue::Bool(on)) => ControlValue::Bool(!on),
                (_, ControlValue::Choice(index)) => ControlValue::Choice((index + 1) % 4),
                other => panic!("{other:?}"),
            };
            assert_eq!(
                bind(&[(control, value.clone())], &image),
                [Setting::new(
                    CHANNEL_MIXER,
                    param,
                    as_param(control, &value)
                )],
                "{control}"
            );
        }
        // As shot in camera, the last listed, is darktable's value 10.
        assert_eq!(
            bind(&[("illuminant", ControlValue::Choice(8))], &image),
            [Setting::new(CHANNEL_MIXER, "illuminant", Value::Choice(10))]
        );
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation() {
        let schema = operation().schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        for param in schema.params {
            assert!(owned.contains(&param.name), "{}", param.name);
        }
        assert_eq!(owned.len(), schema.params.len());
    }

    #[test]
    fn hue_and_chroma_are_darktable_s_lch_around_d50() {
        // D50 itself has no chroma, but for f32's rounding of its x and y, so no hue.
        let d50 = illuminants::D50_XY.map(f64::from);
        assert!(hue_chroma(d50)[1] < 1e-3);
        // darktable's warm light at 3000 K, pipe-relative: orange, a hue near 40°.
        let [hue, chroma] = hue_chroma([0.4369, 0.4041]);
        assert!(
            (20.0..60.0).contains(&hue) && chroma > 30.0,
            "{hue} {chroma}"
        );
        for hue in [0.0, 45.0, 123.4, 270.0, 359.5] {
            let back = hue_chroma(from_hue_chroma([hue, 37.5]));
            assert!((back[0] - hue).abs() < 1e-9 && (back[1] - 37.5).abs() < 1e-9);
        }
    }

    #[test]
    fn the_view_reaches_darktable_s_chroma_of_300_inside_x_and_y_s_ranges() {
        // Decision 27: every hue at chroma 300 reaches x −0.050 to 1.096 and y 0.112 to
        // 1.879, inside the controls' ranges.
        let (mut x, mut y) = ((f64::MAX, f64::MIN), (f64::MAX, f64::MIN));
        for i in 0..3600 {
            for chroma in [100.0, 200.0, 300.0] {
                let [a, b] = from_hue_chroma([f64::from(i) / 10.0, chroma]);
                (x, y) = ((x.0.min(a), x.1.max(a)), (y.0.min(b), y.1.max(b)));
            }
        }
        let rounded = |(low, high): (f64, f64)| [(low * 1e3).round(), (high * 1e3).round()];
        assert_eq!(rounded(x), [-50.0, 1096.0]);
        assert_eq!(rounded(y), [112.0, 1879.0]);
        let range = |id: &str| match DEFINITION.control(id).unwrap().kind {
            ControlKind::Number { min, max, .. } => (f64::from(min), f64::from(max)),
            other => panic!("{other:?}"),
        };
        assert!(range("x").0 < x.0 && x.1 < range("x").1);
        assert!(range("y").0 < y.0 && y.1 < range("y").1);
    }
}
