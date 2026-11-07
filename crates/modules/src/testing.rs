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

//! For the module layer's tests, in this crate and, with the `testing` feature, in
//! others: [`Sample`], a module [`crate::MODULES`] never holds, with
//! [`Following`], Sample with follow writes, a small synthetic photo's image context, and
//! darktable's sliders, which each module's slider table is held to
//! ([`slider_differences`]).

use apertrust_plugin::{Camera, ImageContext, Pattern, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Coordinate, Definition, PresetRow, Range, Row, Slider, Tab,
    ToolSetting, Unit, View, slider_step,
};
use crate::transfer::{Linear, Transfer};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

/// A module over one operation an unedited photo runs and one it does not, its page laid
/// out as darktable lays one out (decision 29):
///
/// - `brightness`, added to the photo's default exposure, so its 0 is the photo as first
///   shown (`exposure.exposure`): an offset over the photo's own value, where the
///   Exposure module writes darktable's slider's value as it is (decision 30);
/// - `grain`, in percent, which adds the grain operation while it is off 0
///   (`grain.strength`);
/// - `size`, fine, medium or coarse grain, which sets nothing while `grain` is at 0,
///   as Exposure's pivot sets nothing while Contrast is 0 (`grain.scale`);
/// - `method`, how the grain lies over the tones: evenly, or biased by `bias`; its third
///   option, film, as darktable lists one this module does not port, drawn disabled and
///   refused; and its fourth, camera, the grain as the camera's exposure bias leaves the
///   tones, which only a photo that records a bias lists, as darktable lists colour
///   calibration's as shot in camera only on a colour raw (decision 30). It sets nothing
///   itself, camera on any photo rendering as even;
/// - `bias`, in percent, the grain's midtones bias (`grain.midtones_bias`), whose row
///   shows only while `method` is biased, and which sets nothing otherwise nor while
///   `grain` is at 0;
/// - `coarseness`, coarse grain's size in percent of its own, from 50 to 200 %
///   (`grain.scale`), which acts only on coarse grain that is on: it is unavailable
///   unless `size` is coarse and `grain` off 0, as darktable makes vignetting's
///   width/height ratio insensitive while automatic ratio is on (decision 30), and sets
///   nothing then. Its range follows `bias`, as AgX's pivot's follows the input range
///   (decision 30): from 50 % to 100 % plus the bias, its track the same, so 50 to 200 %
///   at the defaults, and a move of the bias that leaves it outside clamps it;
/// - `look`, a preset row writing brightness, grain and size, as White balance's Mode
///   writes its light (decision 2): `untouched`, their defaults for the photo, so it
///   matches after a reset; `grainy`, coarse grain at 50 %; and `camera`, the brightness
///   the camera's exposure bias asks for, without grain, which only a photo that records
///   a bias can have;
/// - `pair`, a lossless view of grain and bias, as White balance's hue and chroma are a
///   second view of its light (decision 2): their `sum`, from 0 to 200 %, and their
///   `difference`, grain less bias, from −100 to 100 %. Its inverse halves their sum and
///   difference, exact in f32 but for a rounding; a pair of coordinates in range may still
///   give a grain or a bias outside its own, which the session refuses.
///
/// The look's row and brightness's are before every tab's, shown whichever is open. The
/// Grain tab holds grain, size and coarseness under a group label; the Shape tab, the
/// Midtones collapsible group with method and bias, then the pair's coordinates, the
/// sum's track over a soft range of 50 to 150 %, as darktable's sliders have one.
pub struct Sample;

static DEFINITION: Definition = Definition {
    id: "sample",
    version: 1,
    layers: true,
    auto: false,
    controls: &[
        Control {
            id: "brightness",
            kind: ControlKind::Number {
                min: -3.0,
                max: 4.0,
                step: 0.01,
                default: 0.0,
            },
            unit: Some(Unit::Ev),
            unported: false,
        },
        Control {
            id: "grain",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 0.0,
            },
            unit: Some(Unit::Percent),
            unported: false,
        },
        Control {
            id: "size",
            kind: ControlKind::Choice {
                options: &["fine", "medium", "coarse"],
                default: 1,
                unported: &[],
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "method",
            kind: ControlKind::Choice {
                options: &METHODS,
                default: 0,
                unported: &["film"],
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "bias",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 100.0,
            },
            unit: Some(Unit::Percent),
            unported: false,
        },
        Control {
            id: "coarseness",
            kind: ControlKind::Number {
                min: 50.0,
                max: 200.0,
                step: 1.0,
                default: 100.0,
            },
            unit: Some(Unit::Percent),
            unported: false,
        },
    ],
    tabs: &[
        Tab {
            id: Some("grain"),
            rows: &[
                Row::Label { id: "grain" },
                Row::Slider(Slider::new("grain").decimals(0)),
                Row::Choice {
                    control: "size",
                    style: ChoiceStyle::Segmented,
                },
                Row::Slider(Slider::new("coarseness").decimals(0)),
            ],
        },
        Tab {
            id: Some("shape"),
            rows: &[Row::Group {
                id: "midtones",
                rows: &[
                    Row::Choice {
                        control: "method",
                        style: ChoiceStyle::List,
                    },
                    Row::When {
                        control: "method",
                        is: &["biased"],
                        rows: &[Row::Slider(Slider::new("bias").decimals(0))],
                    },
                    Row::Slider(Slider::new("sum").decimals(0).soft(50.0, 150.0)),
                    Row::Slider(Slider::new("difference").decimals(0)),
                ],
            }],
        },
    ],
    before: &[
        Row::Preset {
            preset: "look",
            style: ChoiceStyle::List,
        },
        Row::Slider(Slider::new("brightness")),
    ],
    after: &[],
    presets: &[PresetRow {
        id: "look",
        options: &["untouched", "grainy", "camera"],
        controls: &["brightness", "grain", "size"],
    }],
    views: &[View {
        id: "pair",
        controls: &["grain", "bias"],
        coordinates: &[
            Coordinate {
                id: "sum",
                min: 0.0,
                max: 200.0,
                step: 1.0,
                unit: Some(Unit::Percent),
            },
            Coordinate {
                id: "difference",
                min: -100.0,
                max: 100.0,
                step: 1.0,
                unit: Some(Unit::Percent),
            },
        ],
    }],
};

static OWNS: [Owned; 4] = [
    Owned::new("exposure", "exposure"),
    Owned::new("grain", "strength"),
    Owned::new("grain", "scale"),
    Owned::new("grain", "midtones_bias"),
];

/// How much `size` scales grain's default scale, by option.
const SIZES: [f32; 3] = [0.5, 1.0, 2.0];

/// `method`'s options, camera last.
const METHODS: [&str; 4] = ["even", "biased", "film", "camera"];

/// `method`'s option that lets `bias` act.
const BIASED: u32 = 1;

/// `size`'s coarse option.
const COARSE: u32 = 2;

impl Module for Sample {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    fn bind(&self, values: &ModuleValues, _: &ImageContext, base: &Base) -> Vec<Setting> {
        let mut settings = Vec::new();
        if let Some(ev) = values.number("brightness") {
            let exposure = Linear::over(base.float("exposure", "exposure")).value(ev);
            settings.push(Setting::new("exposure", "exposure", Value::Float(exposure)));
        }
        if let Some(strength) = values.number("grain") {
            settings.push(Setting::new("grain", "strength", Value::Float(strength)));
            if let Some(size) = values.choice("size") {
                let mut scale = base.float("grain", "scale") * SIZES[size as usize];
                if let (COARSE, Some(coarseness)) = (size, values.number("coarseness")) {
                    scale *= coarseness / 100.0;
                }
                settings.push(Setting::new("grain", "scale", Value::Float(scale)));
            }
            if let (Some(BIASED), Some(bias)) = (values.choice("method"), values.number("bias")) {
                settings.push(Setting::new("grain", "midtones_bias", Value::Float(bias)));
            }
        }
        settings
    }

    /// Coarseness acts only on coarse grain that is on (decision 30).
    fn available(&self, control: &str, values: &Controls, _: &ImageContext) -> bool {
        control != "coarseness"
            || (values.choice("size") == COARSE && values.number("grain") != 0.0)
    }

    /// Coarseness reaches up to 100 % plus the bias, its track the same (decision 30).
    fn range(&self, control: &str, values: &Controls) -> Option<Range> {
        let range = DEFINITION.range(control)?;
        if control != "coarseness" {
            return Some(range);
        }
        let max = 100.0 + values.number("bias");
        Some(Range {
            max,
            soft: (range.min, max),
            ..range
        })
    }

    /// Every option, but camera only on a photo that records an exposure bias
    /// (decision 30).
    fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
        let mut options =
            DEFINITION
                .control(control)
                .map_or(Vec::new(), |control| match control.kind {
                    ControlKind::Choice { options, .. } => options.to_vec(),
                    _ => Vec::new(),
                });
        let bias = image
            .camera
            .as_ref()
            .map_or(0.0, |camera| camera.exposure_bias);
        if control == "method" && bias == 0.0 {
            options.retain(|&option| option != "camera");
        }
        options
    }

    fn preset(&self, preset: &str, option: &str, image: &ImageContext) -> Option<Controls> {
        let row = DEFINITION.preset(preset)?;
        let defaults = self.defaults(image);
        let mut values = Controls::new();
        for &control in row.controls {
            values.set(control, defaults.get(control)?.clone());
        }
        match option {
            "untouched" => {}
            "grainy" => {
                values.set("grain", ControlValue::Number(50.0));
                values.set("size", ControlValue::Choice(COARSE));
            }
            "camera" => {
                let bias = image.camera.as_ref()?.exposure_bias;
                if bias == 0.0 {
                    return None;
                }
                values.set("brightness", ControlValue::Number(bias.clamp(-3.0, 4.0)));
            }
            _ => return None,
        }
        Some(values)
    }

    fn view(&self, view: &str, values: &Controls, _: &ImageContext) -> Option<Vec<f32>> {
        (view == "pair").then(|| {
            let (grain, bias) = (values.number("grain"), values.number("bias"));
            vec![grain + bias, grain - bias]
        })
    }

    fn unview(&self, view: &str, coordinates: &[f32], _: &ImageContext) -> Option<Controls> {
        let (true, &[sum, difference]) = (view == "pair", coordinates) else {
            return None;
        };
        let mut values = Controls::new();
        values.set("grain", ControlValue::Number(0.5 * (sum + difference)));
        values.set("bias", ControlValue::Number(0.5 * (sum - difference)));
        Some(values)
    }
}

/// [`Sample`] with follow writes (decision 27), as Colour calibration's illuminant writes
/// its light: its grain writes what [`Follow`] says, right or wrong as a bug may make it,
/// for the tests that the session writes them in the same change and refuses a module's
/// wrong ones. Its page, binding, look and pair are Sample's.
pub struct Following(pub Follow);

/// What [`Following`]'s grain writes when it moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// Its size: coarse above 50 %, fine at or below, as darktable's GUI keeps one
    /// quantity described alike by two controls.
    Size,
    /// A bias of 150 %, outside the bias's range.
    OutOfRange,
    /// A contrast, which the module lacks.
    Unknown,
    /// Its own value, rewritten.
    Itself,
    /// Nothing: it panics.
    Panics,
}

impl Module for Following {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
        Sample.bind(values, image, base)
    }

    fn view(&self, view: &str, values: &Controls, image: &ImageContext) -> Option<Vec<f32>> {
        Sample.view(view, values, image)
    }

    fn unview(&self, view: &str, coordinates: &[f32], image: &ImageContext) -> Option<Controls> {
        Sample.unview(view, coordinates, image)
    }

    fn follow(&self, control: &str, values: &Controls, _: &ImageContext) -> Controls {
        let mut writes = Controls::new();
        if control != "grain" {
            return writes;
        }
        match self.0 {
            Follow::Size => {
                let coarse = values.number("grain") > 50.0;
                writes.set(
                    "size",
                    ControlValue::Choice(if coarse { COARSE } else { 0 }),
                );
            }
            Follow::OutOfRange => writes.set("bias", ControlValue::Number(150.0)),
            Follow::Unknown => writes.set("contrast", ControlValue::Number(1.0)),
            Follow::Itself => writes.set("grain", ControlValue::Number(1.0)),
            Follow::Panics => panic!("no follow writes for grain"),
        }
        writes
    }
}

/// A module whose binding misbehaves on purpose, for the tests that it is left out while
/// the rest renders: with `stray` on it sets a parameter it does not own, and with
/// `panic` on it panics, as `Base::float` does on a parameter it lacks. It owns nothing,
/// so it passes [`crate::check::accept`].
pub struct Faulty;

static FAULTY: Definition = Definition {
    id: "faulty",
    version: 1,
    layers: false,
    auto: false,
    controls: &[
        Control {
            id: "stray",
            kind: ControlKind::Bool { default: false },
            unit: None,
            unported: false,
        },
        Control {
            id: "panic",
            kind: ControlKind::Bool { default: false },
            unit: None,
            unported: false,
        },
    ],
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Toggle { control: "stray" },
            Row::Toggle { control: "panic" },
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

impl Module for Faulty {
    fn definition(&self) -> &'static Definition {
        &FAULTY
    }

    fn owns(&self) -> &'static [Owned] {
        &[]
    }

    fn bind(&self, values: &ModuleValues, _: &ImageContext, base: &Base) -> Vec<Setting> {
        let mut settings = Vec::new();
        if values.flag("stray") == Some(true) {
            settings.push(Setting::new("exposure", "black", Value::Float(0.01)));
        }
        if values.flag("panic") == Some(true) {
            settings.push(Setting::new(
                "exposure",
                "black",
                Value::Float(base.float("exposure", "no_such_parameter")),
            ));
        }
        settings
    }
}

/// [`Sample`] whose bias writes a coarseness of 190 % whenever it moves, which a bias under
/// 90 % leaves outside coarseness's range (decision 30): a follow write the change's own
/// clamp would rewrite, for the tests that such a change is refused whole, through a
/// control or through the pair. Its page, binding, ranges and pair are Sample's.
pub struct Pushing;

impl Module for Pushing {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
        Sample.bind(values, image, base)
    }

    fn range(&self, control: &str, values: &Controls) -> Option<Range> {
        Sample.range(control, values)
    }

    fn view(&self, view: &str, values: &Controls, image: &ImageContext) -> Option<Vec<f32>> {
        Sample.view(view, values, image)
    }

    fn unview(&self, view: &str, coordinates: &[f32], image: &ImageContext) -> Option<Controls> {
        Sample.unview(view, coordinates, image)
    }

    fn follow(&self, control: &str, _: &Controls, _: &ImageContext) -> Controls {
        let mut writes = Controls::new();
        if control == "bias" {
            writes.set("coarseness", ControlValue::Number(190.0));
        }
        writes
    }
}

/// A module whose defaults for the photo panic, as a bug in [`Module::defaults`] may make
/// them, for the tests that the edit and the session go on without it where they ask
/// every module, as each change's clamp does (decision 30). Its one control, `amount`,
/// is a number from 0 to 10 at 0; it binds nothing and owns nothing.
pub struct Defaultless;

static DEFAULTLESS: Definition = Definition {
    id: "defaultless",
    version: 1,
    layers: false,
    auto: false,
    controls: &[Control {
        id: "amount",
        kind: ControlKind::Number {
            min: 0.0,
            max: 10.0,
            step: 0.1,
            default: 0.0,
        },
        unit: None,
        unported: false,
    }],
    tabs: &[Tab {
        id: None,
        rows: &[Row::Slider(Slider::new("amount"))],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

impl Module for Defaultless {
    fn definition(&self) -> &'static Definition {
        &DEFAULTLESS
    }

    fn defaults(&self, _: &ImageContext) -> Controls {
        panic!("no defaults for this photo")
    }

    fn owns(&self) -> &'static [Owned] {
        &[]
    }

    fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        Vec::new()
    }
}

/// A module with two lossless views of the same two controls, as White balance shows its
/// light as temperature and tint and as hue and chroma (decision 2), for the tests that
/// editing one view moves the other. It binds nothing and owns nothing. Its controls are
/// `x` and `y`, from 0 to 100, at 20 and 60; its views:
///
/// - `pair`, their `sum`, from 0 to 200, and their `difference`, from −100 to 100, exact
///   but for a rounding, as Sample's pair;
/// - `mix`, `u` = x + 2y, from 0 to 300, and `v` = 2x − y, from −100 to 200, whose inverse
///   divides by 5, so its own view of what it wrote is off by a rounding where a value is
///   not a multiple of 5 apart.
///
/// Every control and coordinate has a slider row on its one page.
pub struct Twin;

static TWIN: Definition = Definition {
    id: "twin",
    version: 1,
    layers: false,
    auto: false,
    controls: &[
        Control {
            id: "x",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 20.0,
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "y",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 60.0,
            },
            unit: None,
            unported: false,
        },
    ],
    tabs: &[Tab {
        id: None,
        rows: &[
            Row::Slider(Slider::new("x")),
            Row::Slider(Slider::new("y")),
            Row::Slider(Slider::new("sum")),
            Row::Slider(Slider::new("difference")),
            Row::Slider(Slider::new("u")),
            Row::Slider(Slider::new("v")),
        ],
    }],
    before: &[],
    after: &[],
    presets: &[],
    views: &[
        View {
            id: "pair",
            controls: &["x", "y"],
            coordinates: &[
                Coordinate {
                    id: "sum",
                    min: 0.0,
                    max: 200.0,
                    step: 1.0,
                    unit: None,
                },
                Coordinate {
                    id: "difference",
                    min: -100.0,
                    max: 100.0,
                    step: 1.0,
                    unit: None,
                },
            ],
        },
        View {
            id: "mix",
            controls: &["x", "y"],
            coordinates: &[
                Coordinate {
                    id: "u",
                    min: 0.0,
                    max: 300.0,
                    step: 1.0,
                    unit: None,
                },
                Coordinate {
                    id: "v",
                    min: -100.0,
                    max: 200.0,
                    step: 1.0,
                    unit: None,
                },
            ],
        },
    ],
};

impl Module for Twin {
    fn definition(&self) -> &'static Definition {
        &TWIN
    }

    fn owns(&self) -> &'static [Owned] {
        &[]
    }

    fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        Vec::new()
    }

    fn view(&self, view: &str, values: &Controls, _: &ImageContext) -> Option<Vec<f32>> {
        let (x, y) = (values.number("x"), values.number("y"));
        match view {
            "pair" => Some(vec![x + y, x - y]),
            "mix" => Some(vec![x + 2.0 * y, 2.0 * x - y]),
            _ => None,
        }
    }

    fn unview(&self, view: &str, coordinates: &[f32], _: &ImageContext) -> Option<Controls> {
        let (x, y) = match (view, coordinates) {
            ("pair", &[sum, difference]) => (0.5 * (sum + difference), 0.5 * (sum - difference)),
            ("mix", &[u, v]) => ((u + 2.0 * v) / 5.0, (2.0 * u - v) / 5.0),
            _ => return None,
        };
        let mut values = Controls::new();
        values.set("x", ControlValue::Number(x));
        values.set("y", ControlValue::Number(y));
        Some(values)
    }
}

/// A small Bayer raw's context, 96×72 photosites with its declared image inside, as the
/// engine's viewport tests make one.
pub fn image() -> ImageContext {
    ImageContext {
        camera: Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width: 96,
            height: 72,
            cfa: Some(Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            }),
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 4000.0,
            white_balance: Some(vec![2.0, 1.0, 1.5]),
            xyz_to_camera: Some([
                [3.2406, -1.5372, -0.4986],
                [-0.9689, 1.8758, 0.0415],
                [0.0557, -0.2040, 1.0570],
            ]),
            crop: [2, 2, 92, 68],
            declared: [6, 4, 84, 62],
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            orientation: 0,
        }),
        ..ImageContext::full(84, 62)
    }
}

/// darktable's slider for one control or tool setting, as its params and `gui_init`
/// leave it, in the parameter's own units: a row of a module's slider table
/// (`docs/modules/README.md`, Sliders against darktable), which [`slider_differences`]
/// holds the module's definition to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DarktableSlider {
    /// The control's id, or the tool setting's.
    pub id: &'static str,
    /// Its hard range: the params' `$MIN` and `$MAX`, or what
    /// `dt_bauhaus_slider_new_with_range` gives.
    pub hard: (f32, f32),
    /// What `dt_bauhaus_slider_set_soft_range` gives, or `None` where `gui_init` sets none.
    pub soft: Option<(f32, f32)>,
    /// What it shows of a value, `value × factor + offset`: `dt_bauhaus_slider_set_factor`
    /// and `_offset`, or the factor of 100 that `dt_bauhaus_slider_set_format` gives a
    /// percentage whose hard range lies within ±10.
    pub factor: f32,
    pub offset: f32,
    /// The decimals it shows, as `gui_init` leaves them.
    pub digits: u8,
    /// The step it was given, in the parameter's units: 1 for a whole number, as
    /// `dt_bauhaus_slider_from_params` gives an int; `None` for a float, whose step
    /// `dt_bauhaus_slider_get_step` computes.
    pub own_step: Option<f32>,
}

impl DarktableSlider {
    /// A slider showing its value as it is, with no soft range.
    pub const fn new(id: &'static str, hard: (f32, f32), digits: u8) -> Self {
        Self {
            id,
            hard,
            soft: None,
            factor: 1.0,
            offset: 0.0,
            digits,
            own_step: None,
        }
    }

    /// A whole number's slider, with no decimals, stepping by 1.
    pub const fn whole(id: &'static str, hard: (f32, f32)) -> Self {
        let mut slider = Self::new(id, hard, 0);
        slider.own_step = Some(1.0);
        slider
    }

    pub const fn soft(mut self, min: f32, max: f32) -> Self {
        self.soft = Some((min, max));
        self
    }

    pub const fn shown_as(mut self, factor: f32, offset: f32) -> Self {
        self.factor = factor;
        self.offset = offset;
        self
    }

    /// A range as the slider shows it, in order.
    fn shown(&self, (min, max): (f32, f32)) -> (f32, f32) {
        let (a, b) = (
            min * self.factor + self.offset,
            max * self.factor + self.offset,
        );
        (a.min(b), a.max(b))
    }

    /// The range a value may hold, as the slider shows it.
    pub fn range(&self) -> (f32, f32) {
        self.shown(self.hard)
    }

    /// What its track spans at rest, as it shows it: its soft range, else its hard one.
    pub fn track(&self) -> (f32, f32) {
        self.shown(self.soft.unwrap_or(self.hard))
    }

    /// What a key steps it by, as it shows it: its own step, else
    /// `dt_bauhaus_slider_get_step` at rest, in f32 as darktable computes it, for a slider
    /// given no step of its own, as
    /// `dt_bauhaus_slider_from_params` gives a float: 1 where the span at rest reaches 100
    /// in the parameter's units, else a hundredth of it in the units shown, taken to 1 or
    /// 5 times a power of ten.
    pub fn step(&self) -> f32 {
        if let Some(step) = self.own_step {
            return step * self.factor.abs();
        }
        let (min, max) = self.soft.unwrap_or(self.hard);
        slider_step(min, max, self.factor)
    }
}

/// How a module's sliders differ from darktable's while its controls hold `values`, a
/// line each: every slider row and every number tool setting against its row of
/// `darktable`, by id, in the units shown: the range a value may hold, what the track
/// spans at rest, the decimals and the key step; then each of darktable's sliders that no
/// row shows. A control's range, track and step are the module's for `values`
/// ([`Module::range`], decision 30), the module's defaults for every module's own test,
/// and other values where its range moves, as AgX's pivot's does with the input range.
pub fn slider_differences(
    module: &dyn Module,
    values: &Controls,
    darktable: &[DarktableSlider],
) -> Vec<String> {
    let definition = module.definition();
    let close = |a: f32, b: f32| (a - b).abs() <= 1e-5 * a.abs().max(b.abs()).max(1.0);
    let same = |a: (f32, f32), b: (f32, f32)| close(a.0, b.0) && close(a.1, b.1);
    let mut differences = Vec::new();
    let mut shown = Vec::new();
    for row in definition.rows() {
        let (id, range, step, decimals, soft) = match *row {
            Row::Slider(slider) => {
                let id = slider.control;
                // A row's own soft range stands while the module's range is the
                // definition's.
                if let (Some(ours), Some(moved)) = (definition.range(id), module.range(id, values))
                    && moved != ours
                {
                    let soft = Some(moved.soft);
                    (
                        id,
                        (moved.min, moved.max),
                        moved.step,
                        slider.decimals,
                        soft,
                    )
                } else {
                    let (range, step) = match definition.control(id).map(|c| c.kind) {
                        Some(ControlKind::Number { min, max, step, .. }) => ((min, max), step),
                        _ => match definition.coordinate(id) {
                            Some((view, at)) => {
                                let c = &view.coordinates[at];
                                ((c.min, c.max), c.step)
                            }
                            None => continue,
                        },
                    };
                    (id, range, step, slider.decimals, slider.soft)
                }
            }
            Row::ToolSetting(ToolSetting {
                id,
                kind: ControlKind::Number { min, max, step, .. },
                decimals,
            }) => (id, (min, max), step, decimals, None),
            _ => continue,
        };
        shown.push(id);
        let Some(theirs) = darktable.iter().find(|slider| slider.id == id) else {
            differences.push(format!("{id}: not in darktable's table"));
            continue;
        };
        let pair = |(min, max): (f32, f32)| format!("{min} to {max}");
        if !same(range, theirs.range()) {
            differences.push(format!(
                "{id}: holds {}, darktable's {}",
                pair(range),
                pair(theirs.range())
            ));
        }
        let track = soft.unwrap_or(range);
        if !same(track, theirs.track()) {
            differences.push(format!(
                "{id}: its track spans {}, darktable's {}",
                pair(track),
                pair(theirs.track())
            ));
        }
        if decimals != theirs.digits {
            differences.push(format!(
                "{id}: shows {decimals} decimals, darktable's {}",
                theirs.digits
            ));
        }
        if !close(step, theirs.step()) {
            differences.push(format!(
                "{id}: steps by {step}, darktable's {}",
                theirs.step()
            ));
        }
    }
    for theirs in darktable {
        if !shown.contains(&theirs.id) {
            differences.push(format!("{}: no row shows darktable's slider", theirs.id));
        }
    }
    differences
}

/// The mosaic [`image`] describes: a colour gradient with a corner that clips.
pub fn mosaic() -> Vec<u16> {
    let (width, height) = (96, 72);
    (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            let c = [0, 1, 1, 2][((y % 2) * 2 + x % 2) as usize] as f32;
            let level =
                0.1 + 0.6 * x as f32 / width as f32 + 0.1 * c + 0.2 * y as f32 / height as f32;
            (level.min(1.0) * 4000.0).round() as u16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darktable_s_step_is_get_step_s_over_the_span_at_rest() {
        // In f32, as darktable computes it: 0.049999997 for 0.05.
        let step = |slider: DarktableSlider| (f64::from(slider.step()) * 1e6).round() / 1e6;
        // A hundredth of the span, taken to 1 or 5 times a power of ten: exposure's 0.04
        // is 0.05, a highlights chroma's 0.2 % is 0.1 %.
        let exposure = DarktableSlider::new("exposure", (-18.0, 18.0), 3);
        assert_eq!(step(exposure.soft(-3.0, 4.0)), 0.05);
        assert_eq!(step(exposure), 0.1);
        let chroma = DarktableSlider::new("chroma", (0.0, 1.0), 2).shown_as(100.0, 0.0);
        assert_eq!(step(chroma), 1.0);
        assert_eq!(step(chroma.soft(0.0, 0.2)), 0.1);
        assert_eq!(step(chroma.soft(0.0, 0.01)), 0.01);
        // 1 where the span reaches 100 in the parameter's units, as a hue's 360°.
        assert_eq!(step(DarktableSlider::new("hue", (0.0, 360.0), 2)), 1.0);
        assert_eq!(step(DarktableSlider::new("level", (0.0, 100.0), 1)), 1.0);
        // Ranges shown with a factor and an offset, in order.
        let purity = DarktableSlider::new("purity", (0.01, 5.0), 1)
            .shown_as(100.0, -100.0)
            .soft(0.5, 1.5);
        assert_eq!(purity.range(), (-99.0, 400.0));
        assert_eq!(purity.track(), (-50.0, 50.0));
        assert_eq!(step(purity), 1.0);
    }

    #[test]
    fn a_definition_s_sliders_are_held_to_darktable_s_each_difference_named() {
        // Sample's sliders, in the panel's order: brightness, before every tab's, −3 to 4
        // by 0.01 with two decimals; grain and bias, 0 to 100 by 1, with none, and
        // coarseness, 50 to 200 by 1; the pair's sum and difference, 0 to 200, its track
        // over 50 to 150, and ±100 by 1.
        let darktable = [
            DarktableSlider::new("brightness", (-3.0, 4.0), 2).soft(-1.0, 1.0),
            // A percentage shown as darktable's factor of 100 shows it.
            DarktableSlider::new("grain", (0.0, 1.0), 0).shown_as(100.0, 0.0),
            DarktableSlider::whole("coarseness", (50.0, 200.0)),
            DarktableSlider::new("bias", (0.0, 50.0), 1),
            DarktableSlider::new("sum", (0.0, 200.0), 0).soft(50.0, 150.0),
            DarktableSlider::new("difference", (-100.0, 100.0), 0),
            DarktableSlider::new("missing", (0.0, 1.0), 2),
        ];
        assert_eq!(
            slider_differences(&Sample, &DEFINITION.defaults(), &darktable),
            [
                "brightness: its track spans -3 to 4, darktable's -1 to 1",
                "bias: holds 0 to 100, darktable's 0 to 50",
                "bias: its track spans 0 to 100, darktable's 0 to 50",
                "bias: shows 0 decimals, darktable's 1",
                "bias: steps by 1, darktable's 0.5",
                "missing: no row shows darktable's slider",
            ]
        );
        assert_eq!(
            slider_differences(&Sample, &DEFINITION.defaults(), &[]),
            [
                "brightness: not in darktable's table",
                "grain: not in darktable's table",
                "coarseness: not in darktable's table",
                "bias: not in darktable's table",
                "sum: not in darktable's table",
                "difference: not in darktable's table",
            ]
        );
    }

    /// Every control of [`Sample`] at its default for `image`, but those the look's
    /// `option` writes, as the session reports them: what [`Module::matching`] reads.
    fn holding(image: &ImageContext, option: &Controls) -> Controls {
        let mut values = Sample.defaults(image);
        for control in ["brightness", "grain", "size"] {
            values.set(control, option.get(control).unwrap().clone());
        }
        values
    }

    #[test]
    fn the_look_matches_the_option_its_values_hold_and_custom_off_them() {
        let image = image();
        let look = |values: &Controls| Sample.matching("look", values, &image);
        assert_eq!(look(&Sample.defaults(&image)), Some("untouched"));
        let grainy = Sample.preset("look", "grainy", &image).unwrap();
        let mut values = holding(&image, &grainy);
        assert_eq!(look(&values), Some("grainy"));
        // A control the row does not write leaves the match as it is.
        values.set("method", ControlValue::Choice(BIASED));
        assert_eq!(look(&values), Some("grainy"));
        // One it writes, moved off the option's value by however little, is Custom.
        values.set("grain", ControlValue::Number(50.0_f32.next_up()));
        assert_eq!(look(&values), None);
        assert_eq!(Sample.matching("tone", &values, &image), None);
    }

    /// Grain and bias as [`Module::unview`] gives them for the pair's `coordinates`.
    fn unviewed(coordinates: [f32; 2]) -> [f32; 2] {
        let values = Sample.unview("pair", &coordinates, &image()).unwrap();
        ["grain", "bias"].map(|id| values.number(id))
    }

    /// The pair's coordinates for grain and bias, the other controls at their defaults.
    fn viewed([grain, bias]: [f32; 2]) -> [f32; 2] {
        let mut values = Sample.defaults(&image());
        values.set("grain", ControlValue::Number(grain));
        values.set("bias", ControlValue::Number(bias));
        let coordinates = Sample.view("pair", &values, &image()).unwrap();
        [coordinates[0], coordinates[1]]
    }

    /// Whether `a` and `b` differ by a few f32 units at most, at the scale of 200, the
    /// pair's largest coordinate.
    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        a.iter()
            .zip(&b)
            .all(|(a, b)| (a - b).abs() <= 4.0 * 200.0 * f32::EPSILON)
    }

    #[test]
    fn the_pair_round_trips_over_its_controls_and_its_coordinates_ranges() {
        let steps =
            |from: f32, to: f32| (0..=97).map(move |i| from + (to - from) * i as f32 / 97.0);
        // View then unview, over grain's and bias's ranges, and back, over the sum's and the
        // difference's, where grain or bias may leave theirs: a rounding apart at most.
        for grain in steps(0.0, 100.0) {
            for bias in steps(0.0, 100.0) {
                let values = [grain, bias];
                assert!(near(unviewed(viewed(values)), values), "{values:?}");
            }
        }
        for sum in steps(0.0, 200.0) {
            for difference in steps(-100.0, 100.0) {
                let coordinates = [sum, difference];
                assert!(
                    near(viewed(unviewed(coordinates)), coordinates),
                    "{coordinates:?}"
                );
            }
        }
        // Exact where every value is a whole number of percent, the defaults among them.
        assert_eq!(viewed([0.0, 100.0]), [100.0, -100.0]);
        assert_eq!(unviewed([100.0, -100.0]), [0.0, 100.0]);
        assert_eq!(unviewed(viewed([37.0, 64.0])), [37.0, 64.0]);
        // No other view.
        assert_eq!(
            Sample.view("tone", &Sample.defaults(&image()), &image()),
            None
        );
        assert_eq!(Sample.unview("pair", &[100.0], &image()), None);
    }

    #[test]
    fn the_twin_s_views_hold_together_and_invert_to_a_rounding() {
        TWIN.check().unwrap();
        let image = image();
        let defaults = Twin.defaults(&image);
        assert_eq!(
            Twin.view("pair", &defaults, &image),
            Some(vec![80.0, -40.0])
        );
        assert_eq!(
            Twin.view("mix", &defaults, &image),
            Some(vec![140.0, -20.0])
        );
        for (view, coordinates) in [("pair", [110.0, -40.3]), ("mix", [251.0, 17.0])] {
            let values = Twin.unview(view, &coordinates, &image).unwrap();
            let back = Twin.view(view, &values, &image).unwrap();
            for (a, b) in back.iter().zip(coordinates) {
                assert!(
                    (a - b).abs() <= 4.0 * 300.0 * f32::EPSILON,
                    "{view}: {back:?}"
                );
            }
        }
        assert_eq!(Twin.view("tone", &defaults, &image), None);
    }

    #[test]
    fn coarseness_is_available_only_on_coarse_grain_that_is_on_and_sets_nothing_otherwise() {
        let image = image();
        let available = |size: u32, grain: f32| {
            let mut values = Sample.defaults(&image);
            values.set("size", ControlValue::Choice(size));
            values.set("grain", ControlValue::Number(grain));
            values.set("coarseness", ControlValue::Number(150.0));
            let others = ["brightness", "grain", "size", "method", "bias"];
            assert!(others.iter().all(|c| Sample.available(c, &values, &image)));
            Sample.available("coarseness", &values, &image)
        };
        assert!(!available(0, 50.0));
        assert!(!available(1, 50.0));
        assert!(!available(COARSE, 0.0));
        assert!(available(COARSE, 1.0));
        assert!(available(COARSE, 50.0));
        // Where it is unavailable it changes nothing: grain's scale is size's alone, and
        // without grain nothing is set at all.
        let base = Base::new(apertrust_operations::OPERATIONS, &image);
        let bound = |grain: Option<f32>, size: u32, coarseness: Option<f32>| {
            let mut values = ModuleValues::new();
            if let Some(grain) = grain {
                values.set("grain", ControlValue::Number(grain));
            }
            values.set("size", ControlValue::Choice(size));
            if let Some(coarseness) = coarseness {
                values.set("coarseness", ControlValue::Number(coarseness));
            }
            Sample.bind(&values, &image, &base)
        };
        let scale = |size: u32, coarseness: Option<f32>| {
            let settings = bound(Some(50.0), size, coarseness);
            settings
                .iter()
                .find(|s| (s.operation, s.param) == ("grain", "scale"))
                .map(|s| s.value)
        };
        assert!(bound(None, COARSE, Some(150.0)).is_empty());
        assert_eq!(scale(0, Some(150.0)), scale(0, None));
        assert_eq!(scale(1, Some(150.0)), scale(1, None));
        let default = base.float("grain", "scale");
        assert_eq!(scale(COARSE, None), Some(Value::Float(default * 2.0)));
        assert_eq!(
            scale(COARSE, Some(150.0)),
            Some(Value::Float(default * 3.0))
        );
    }

    #[test]
    fn the_look_s_camera_option_needs_a_photo_that_records_a_bias() {
        let plain = image();
        assert_eq!(Sample.preset("look", "camera", &plain), None);
        assert_eq!(Sample.preset("look", "film", &plain), None);
        let mut biased = image();
        if let Some(camera) = &mut biased.camera {
            camera.exposure_bias = -0.7;
        }
        let camera = Sample.preset("look", "camera", &biased).unwrap();
        assert_eq!(camera.get("brightness"), Some(&ControlValue::Number(-0.7)));
        let values = holding(&biased, &camera);
        assert_eq!(Sample.matching("look", &values, &biased), Some("camera"));
    }

    #[test]
    fn the_method_lists_camera_only_on_a_photo_that_records_a_bias() {
        let plain = image();
        let mut biased = image();
        if let Some(camera) = &mut biased.camera {
            camera.exposure_bias = -0.7;
        }
        let method = DEFINITION.control("method").unwrap();
        let lists = |image: &ImageContext| Sample.options("method", image);
        assert_eq!(lists(&plain), ["even", "biased", "film"]);
        assert_eq!(lists(&biased), METHODS);
        assert_eq!(lists(&ImageContext::full(8, 8)), ["even", "biased", "film"]);
        for image in [plain, biased] {
            let default = Sample.defaults(&image).get("method").unwrap().clone();
            assert_eq!(crate::check::listed(method, &lists(&image), &default), None);
        }
        // Every other control lists as provided: each option of a choice, nothing else.
        assert_eq!(
            Sample.options("size", &image()),
            ["fine", "medium", "coarse"]
        );
        assert!(Sample.options("grain", &image()).is_empty());
    }

    #[test]
    fn a_list_holds_when_it_is_the_definition_s_in_order_with_the_default() {
        let method = DEFINITION.control("method").unwrap();
        let even = ControlValue::Choice(0);
        let listed = |list: &[&str]| crate::check::listed(method, list, &even);
        assert_eq!(listed(&["even", "film"]), None);
        assert!(listed(&[]).is_some());
        assert!(listed(&["even", "sepia"]).is_some());
        assert!(listed(&["even", "even"]).is_some());
        assert!(listed(&["film", "even"]).is_some());
        assert!(listed(&["biased"]).is_some());
        // A control that is no choice lists nothing, and that holds.
        let grain = DEFINITION.control("grain").unwrap();
        assert_eq!(
            crate::check::listed(grain, &[], &ControlValue::Number(0.0)),
            None
        );
    }
}
