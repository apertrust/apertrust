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

//! Tone equalizer (`docs/modules/tone-equalizer.md`): darktable's tone equalizer as
//! the user edits with it, in the first pass (decision 2): darktable's page, its tabs
//! Simple, Advanced and Masking, its controls in `gui_init`'s order under its labels.
//!
//! - **Simple:** the nine bands, −8 EV to +0 EV of the mask, each its exposure correction.
//! - **Advanced:** curve smoothing, then the mask's exposure and contrast compensations.
//! - **Masking:** the luminance estimator and the filter that preserves details, every
//!   darktable option, the two guided filters not ported; then, while a filter is chosen,
//!   its diffusion, diameter, edges refinement and quantisation, as darktable shows them.
//! - **Under every tab:** darktable's display of the exposure mask, a view mode that no
//!   edit stores, drawn disabled until the mask preview comes.
//!
//! Each control is one `toneequal` parameter as darktable's slider shows it: the bands, the
//! compensations and the mask settings as they are, and the curve's σ as darktable's
//! slider shows it, `s` with σ = √2^(1 + s). It is switched (decision 32), as darktable
//! leaves the tone equalizer off on a new photo: off, it renders nothing; any write of a
//! control turns it on, with darktable's defaults for the others, until its reset turns it
//! off, `toneequal` planned while it is on. Its first step, on at its defaults, brings
//! the operation's curve with every band at 0, darktable's ripple of −0.013 to +0.025 EV
//! at the default smoothing, as darktable's first step does.
//!
//! Every slider is darktable's: its hard range the control's, its track darktable's soft
//! range, its decimals and its step darktable's (`docs/modules/tone-equalizer.md`, Sliders
//! against darktable). The contrast compensation, on the Advanced tab, shows only with
//! the guided filter or EIGF, chosen on the Masking tab, as darktable shows it.
//!
//! The Advanced tab opens with darktable's graph of the bands, a second view of them
//! (decision 2): the design system's equalizer, drawing the curve the operation renders
//! through them ([`Module::curve`], decision 31), from the fit `apertrust-color` holds for
//! the operation's setup and the graph alike (decision 65). Left out until the renderer
//! can draw them: the wands on the two compensations, which need the mask's histogram.

use apertrust_color::tone_equalizer::Curve;
use apertrust_plugin::{ImageContext, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Definition, Row, Slider, Tab, ToolSetting, Unit, Widget,
};
use crate::transfer::Transfer;
use crate::values::{ControlValue, Controls};
use crate::{Base, Module, ModuleValues, Owned, Setting, switched_from};

pub struct ToneEqualizer;

/// The operation it drives.
const TONE_EQUALIZER: &str = "toneequal";

/// The nine bands, −8 EV to 0 EV of the mask, by darktable's parameter names, which are
/// not their labels below −2 EV: `shadows` is the −4 EV band (`docs/ports/toneequal.md`,
/// Parameters).
pub const BANDS: [&str; 9] = [
    "noise",
    "ultra_deep_blacks",
    "deep_blacks",
    "blacks",
    "shadows",
    "midtones",
    "highlights",
    "whites",
    "speculars",
];

/// Curve smoothing: darktable's slider for the curve's σ, `s` with σ = √2^(1 + s).
pub const SMOOTHING: &str = "smoothing";
/// The mask filter's passes, a whole number the slider shows without decimals.
pub const ITERATIONS: &str = "iterations";
/// The luminance estimator, its options the operation's choices.
pub const METHOD: &str = "method";
pub const METHODS: [&str; 7] = [
    "rgb_average",
    "hsl_lightness",
    "hsv_value",
    "rgb_sum",
    "rgb_euclidean_norm",
    "rgb_power_norm",
    "rgb_geometric_mean",
];
/// Preserve details, the mask's filter, its options the operation's choices.
pub const DETAILS: &str = "details";
pub const FILTERS: [&str; 5] = [
    "none",
    "averaged_guided_filter",
    "guided_filter",
    "averaged_eigf",
    "eigf",
];
/// The filters the operation does not port yet (`docs/ports/toneequal.md`, Planned).
const GUIDED_FILTERS: [&str; 2] = ["averaged_guided_filter", "guided_filter"];
/// Every filter: darktable shows the filter's settings with any of them, none without
/// (`show_guiding_controls`).
const FILTERED: [&str; 4] = [
    "averaged_guided_filter",
    "guided_filter",
    "averaged_eigf",
    "eigf",
];
/// The filters darktable shows the contrast compensation with, those it acts with: the
/// guided filter and EIGF, not their averaged forms (`show_guiding_controls`).
const CONTRASTED: [&str; 2] = ["guided_filter", "eigf"];

/// Curve smoothing's transfer function, darktable's `smoothing_callback`: σ = √2^(1 + s),
/// `powf(M_SQRT2_F, 1.0f + s)`, correctly rounded from f64 so every platform binds the
/// same σ, so `s` at 0 is darktable's default, √2, and the slider's ends are the schema's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSmoothing;

impl Transfer for CurveSmoothing {
    fn value(&self, s: f32) -> f32 {
        f64::from(std::f32::consts::SQRT_2).powf(f64::from(1.0 + s)) as f32
    }

    fn derivative(&self, s: f32) -> f32 {
        self.value(s) * std::f32::consts::LN_2 / 2.0
    }
}

/// A number control. `step` is darktable's: what its slider steps by over its soft range,
/// which a key steps by here.
const fn number(
    id: &'static str,
    (min, max): (f32, f32),
    step: f32,
    default: f32,
    unit: Option<Unit>,
) -> Control {
    Control {
        id,
        kind: ControlKind::Number {
            min,
            max,
            step,
            default,
        },
        unit,
        unported: false,
    }
}

/// A band's gain, −2 EV to +2 EV, darktable's default 0.
const fn band(id: &'static str) -> Control {
    number(id, (-2.0, 2.0), 0.01, 0.0, Some(Unit::Ev))
}

/// darktable's hard ranges are the controls', in the units its sliders show; its soft
/// ranges, the rows', lie within them. Every control is named after its parameter.
static CONTROLS: [Control; 18] = [
    band(BANDS[0]),
    band(BANDS[1]),
    band(BANDS[2]),
    band(BANDS[3]),
    band(BANDS[4]),
    band(BANDS[5]),
    band(BANDS[6]),
    band(BANDS[7]),
    band(BANDS[8]),
    // σ = √2^(1 + s), so 0.6306887 to 2.5227547, the schema's range.
    number(SMOOTHING, (-2.33, 1.67), 0.01, 0.0, None),
    number("exposure_boost", (-16.0, 16.0), 0.05, 0.0, Some(Unit::Ev)),
    number("contrast_boost", (-16.0, 16.0), 0.01, 0.0, Some(Unit::Ev)),
    Control {
        id: METHOD,
        kind: ControlKind::Choice {
            options: &METHODS,
            default: 4,
            unported: &[],
        },
        unit: None,
        unported: false,
    },
    Control {
        id: DETAILS,
        kind: ControlKind::Choice {
            options: &FILTERS,
            default: 4,
            unported: &GUIDED_FILTERS,
        },
        unit: None,
        unported: false,
    },
    number(ITERATIONS, (1.0, 20.0), 1.0, 1.0, None),
    number("blending", (0.01, 100.0), 0.5, 5.0, Some(Unit::Percent)),
    number("feathering", (0.01, 10000.0), 0.5, 1.0, None),
    number("quantization", (0.0, 2.0), 0.01, 0.0, Some(Unit::Ev)),
];

/// A slider row as darktable's: two decimals, filled.
const fn slider(control: &'static str) -> Row {
    Row::Slider(Slider::new(control))
}

/// A slider row whose track spans darktable's soft range, `min` to `max`.
const fn soft(control: &'static str, min: f32, max: f32) -> Row {
    Row::Slider(Slider::new(control).soft(min, max))
}

static DEFINITION: Definition = Definition {
    id: "toneequal",
    version: 2,
    layers: true,
    auto: false,
    switched: true,
    controls: &CONTROLS,
    tabs: &[
        Tab {
            id: Some("simple"),
            rows: &[
                slider(BANDS[0]),
                slider(BANDS[1]),
                slider(BANDS[2]),
                slider(BANDS[3]),
                slider(BANDS[4]),
                slider(BANDS[5]),
                slider(BANDS[6]),
                slider(BANDS[7]),
                slider(BANDS[8]),
            ],
        },
        Tab {
            id: Some("advanced"),
            rows: &[
                // darktable's graph of the bands, a second view of them (decision 2).
                Row::Custom {
                    widget: Widget::Equalizer,
                    controls: &BANDS,
                },
                soft(SMOOTHING, -1.0, 1.0),
                // darktable's wands on the two compensations come with the mask's
                // histogram.
                soft("exposure_boost", -4.0, 4.0),
                // Following the Masking tab's filter, as darktable's does.
                Row::When {
                    control: DETAILS,
                    is: &CONTRASTED,
                    rows: &[soft("contrast_boost", -2.0, 2.0)],
                },
            ],
        },
        Tab {
            id: Some("masking"),
            rows: &[
                Row::Choice {
                    control: METHOD,
                    style: ChoiceStyle::List,
                },
                Row::Choice {
                    control: DETAILS,
                    style: ChoiceStyle::List,
                },
                Row::When {
                    control: DETAILS,
                    is: &FILTERED,
                    rows: &[
                        Row::Slider(Slider::new(ITERATIONS).decimals(0).soft(1.0, 5.0)),
                        soft("blending", 1.0, 45.0),
                        soft("feathering", 0.1, 50.0),
                        slider("quantization"),
                    ],
                },
            ],
        },
    ],
    before: &[],
    // darktable's toggle under its notebook: the mask in place of the photo, a view mode
    // that no edit stores (`docs/modules/README.md`, rule 10).
    after: &[Row::ToolSetting(ToolSetting {
        id: "display_exposure_mask",
        kind: ControlKind::Bool { default: false },
        decimals: 0,
    })],
    presets: &[],
    views: &[],
};

/// Every parameter of `toneequal`, each by the control of its name: the module is
/// darktable's and owns them all.
static OWNS: [Owned; 18] = owns();

const fn owns() -> [Owned; 18] {
    let mut owns = [Owned::new(TONE_EQUALIZER, ""); 18];
    let mut i = 0;
    while i < CONTROLS.len() {
        owns[i] = Owned::new(TONE_EQUALIZER, CONTROLS[i].id);
        i += 1;
    }
    owns
}

/// The curve's σ for curve smoothing's `s`.
fn smoothing(s: f32) -> f32 {
    CurveSmoothing.value(s)
}

impl Module for ToneEqualizer {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// Each control off its default sets its parameter: a number as it is, curve
    /// smoothing as σ, the iterations as the nearest whole number, a choice by the same
    /// index.
    fn bind(&self, values: &ModuleValues, _image: &ImageContext, _base: &Base) -> Vec<Setting> {
        CONTROLS
            .iter()
            .filter_map(|control| {
                let id = control.id;
                let value = match id {
                    METHOD | DETAILS => Value::Choice(values.choice(id)?),
                    SMOOTHING => Value::Float(smoothing(values.number(id)?)),
                    ITERATIONS => Value::Int(values.number(id)?.round() as i64),
                    _ => Value::Float(values.number(id)?),
                };
                Some(Setting::new(TONE_EQUALIZER, id, value))
            })
            .collect()
    }

    /// Version 1 planned `toneequal` while a control was off its default, darktable's then
    /// as now, and bound every one that was: such an edit is on, its values as they were;
    /// any other rendered nothing, so it is off, its values dropped (decision 32).
    fn migrate(&self, from: u32, values: ModuleValues) -> ModuleValues {
        if from >= 2 {
            return values;
        }
        let off = values.off_defaults(&DEFINITION.defaults());
        let rendered = off.values().next().is_some();
        switched_from(values, rendered)
    }

    /// The bands' curve as `toneequal` renders it (decision 31), on the graph's axes: the
    /// correction in EV for the mask's exposure in EV, darktable's fit of the nine gains at
    /// the σ the binding sets, through the curve `apertrust-color` gives the operation's
    /// setup too: at the defaults, darktable's ripple, which the module on renders. The
    /// panel draws it flat while the module is off (decision 32), as nothing is corrected
    /// then. `None` where the fit fails.
    fn curve(&self, control: &str, values: &Controls) -> Option<Box<dyn Fn(f32) -> f32>> {
        if !BANDS.contains(&control) {
            return None;
        }
        let bands = BANDS.map(|band| value_of(values, band));
        let curve = Curve::new(&bands, sigma(values))?;
        Some(Box::new(move |exposure| {
            curve.ev(f64::from(exposure)) as f32
        }))
    }

    /// The curve's σ, which sets how far a drag on the graph reaches, as darktable's.
    fn curve_width(&self, control: &str, values: &Controls) -> Option<f32> {
        BANDS.contains(&control).then(|| sigma(values))
    }
}

/// The curve's σ while the controls hold `values`, as the binding sets it.
fn sigma(values: &Controls) -> f32 {
    smoothing(value_of(values, SMOOTHING))
}

/// A band's or curve smoothing's value in `values`; its default, 0, where they hold none.
fn value_of(values: &Controls, control: &str) -> f32 {
    match values.get(control) {
        Some(ControlValue::Number(value)) => *value,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::toneequal::{self, ToneEqual};
    use apertrust_plugin::{Kind, Operation};

    use super::*;
    use crate::testing::DarktableSlider;
    use crate::values::ControlValue;
    use crate::{check, transfer};

    fn bind(set: &[(&str, ControlValue)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&ToneEqualizer.defaults(&image));
        ToneEqualizer.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    fn value(settings: &[Setting], param: &str) -> Option<Value> {
        settings
            .iter()
            .find(|setting| setting.param == param)
            .map(|setting| setting.value)
    }

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        DEFINITION.check().unwrap();
        let (accepted, rejected) = check::accept(&[&ToneEqualizer], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        for image in [crate::testing::image(), ImageContext::full(8, 8)] {
            let problems = check::sweep(&ToneEqualizer, OPERATIONS, &image);
            assert!(problems.is_empty(), "{problems:?}");
        }
    }

    #[test]
    fn its_controls_are_darktable_s_in_gui_init_s_order() {
        // The graph is a second view of the bands, which their sliders show first.
        let shown: Vec<&str> = DEFINITION
            .rows()
            .filter(|row| !matches!(row, Row::Custom { .. }))
            .flat_map(|row| row.controls().iter().copied())
            .collect();
        let mut expected = BANDS.to_vec();
        expected.extend([
            SMOOTHING,
            "exposure_boost",
            "contrast_boost",
            METHOD,
            DETAILS,
            ITERATIONS,
            "blending",
            "feathering",
            "quantization",
        ]);
        assert_eq!(shown, expected);
        let controls: Vec<&str> = CONTROLS.iter().map(|c| c.id).collect();
        assert_eq!(controls, expected);
    }

    #[test]
    fn it_owns_every_parameter_of_the_operation_each_with_one_control() {
        let schema = ToneEqual.schema();
        let owned: Vec<&str> = OWNS.iter().map(|owned| owned.param).collect();
        let params: Vec<&str> = schema.params.iter().map(|param| param.name).collect();
        assert_eq!(owned.len(), params.len());
        for param in &params {
            assert!(owned.contains(param), "{param}");
        }
        assert_eq!(BANDS, toneequal::BANDS);
        for control in &CONTROLS {
            let param = schema.param(control.id).unwrap();
            match (control.kind, param.kind) {
                // Curve smoothing shows σ as darktable's slider does.
                (ControlKind::Number { .. }, _) if control.id == SMOOTHING => {}
                (
                    ControlKind::Number {
                        min, max, default, ..
                    },
                    Kind::Float {
                        min: low,
                        max: high,
                        default: untouched,
                    },
                ) => assert_eq!(
                    (min, max, default),
                    (low, high, untouched),
                    "{}",
                    control.id
                ),
                (
                    ControlKind::Number {
                        min, max, default, ..
                    },
                    Kind::Int {
                        min: low,
                        max: high,
                        default: untouched,
                    },
                ) => assert_eq!(
                    (min, max, default),
                    (low as f32, high as f32, untouched as f32),
                    "{}",
                    control.id
                ),
                (
                    ControlKind::Choice {
                        options, default, ..
                    },
                    Kind::Choice {
                        choices,
                        default: untouched,
                    },
                ) => assert_eq!((options, default), (choices, untouched), "{}", control.id),
                (kind, param) => panic!("{}: {kind:?} for {param:?}", control.id),
            }
        }
    }

    #[test]
    fn curve_smoothing_is_darktable_s_slider_for_sigma() {
        let range = -2.33..=1.67;
        transfer::check(&CurveSmoothing, range.clone()).unwrap();
        assert!(transfer::increasing(&CurveSmoothing, range));
        // At 0, darktable's default, √2; at the slider's ends, the schema's, exactly.
        assert_eq!(smoothing(0.0), std::f32::consts::SQRT_2);
        assert_eq!(smoothing(-2.33), toneequal::SMOOTHING_MIN);
        assert_eq!(smoothing(1.67), toneequal::SMOOTHING_MAX);
        // darktable's own round trip, `gui_update`'s log over log √2 less 1, gives σ back
        // within an f32 step.
        for sigma in [0.8_f32, 1.0, 1.2, 1.5, 2.0] {
            let s = sigma.ln() / std::f32::consts::SQRT_2.ln() - 1.0;
            let back = smoothing(s);
            assert!(
                (back - sigma).abs() <= 2.0 * f32::EPSILON * sigma,
                "{sigma}: {back}"
            );
        }
    }

    const fn band(id: &'static str) -> DarktableSlider {
        DarktableSlider::new(id, (-2.0, 2.0), 2)
    }

    /// darktable's tone equalizer sliders (`src/iop/toneequal.c`, the params and
    /// `gui_init`), in its parameters' units: `docs/modules/tone-equalizer.md`'s slider
    /// table. None shows a factor; Smoothing diameter's "%" format gives none, its range
    /// above ±10. Curve smoothing is a slider of no parameter, from
    /// `dt_bauhaus_slider_new_with_range`.
    const DARKTABLE: [DarktableSlider; 16] = [
        band("noise"),
        band("ultra_deep_blacks"),
        band("deep_blacks"),
        band("blacks"),
        band("shadows"),
        band("midtones"),
        band("highlights"),
        band("whites"),
        band("speculars"),
        DarktableSlider::new(SMOOTHING, (-2.33, 1.67), 2).soft(-1.0, 1.0),
        DarktableSlider::new("exposure_boost", (-16.0, 16.0), 2).soft(-4.0, 4.0),
        DarktableSlider::new("contrast_boost", (-16.0, 16.0), 2).soft(-2.0, 2.0),
        // `dt_bauhaus_slider_set_soft_max(g->iterations, 5)`.
        DarktableSlider::whole(ITERATIONS, (1.0, 20.0)).soft(1.0, 5.0),
        DarktableSlider::new("blending", (0.01, 100.0), 2).soft(1.0, 45.0),
        DarktableSlider::new("feathering", (0.01, 10000.0), 2).soft(0.1, 50.0),
        DarktableSlider::new("quantization", (0.0, 2.0), 2),
    ];

    #[test]
    fn its_sliders_are_darktable_s() {
        let differences =
            crate::testing::slider_differences(&ToneEqualizer, &DEFINITION.defaults(), &DARKTABLE);
        assert!(differences.is_empty(), "{differences:#?}");
    }

    #[test]
    fn at_its_defaults_it_sets_nothing() {
        let image = crate::testing::image();
        let base = Base::new(OPERATIONS, &image);
        assert!(
            ToneEqualizer
                .bind(&ModuleValues::new(), &image, &base)
                .is_empty()
        );
        let defaults: Vec<(&str, ControlValue)> =
            CONTROLS.iter().map(|c| (c.id, c.default())).collect();
        assert!(bind(&defaults).is_empty());
    }

    #[test]
    fn each_control_sets_its_parameter_as_darktable_s_slider_shows_it() {
        let number = ControlValue::Number;
        let settings = bind(&[
            ("shadows", number(0.75)),
            (SMOOTHING, number(1.0)),
            ("exposure_boost", number(-1.57)),
            ("blending", number(2.0)),
            (ITERATIONS, number(4.6)),
            (METHOD, ControlValue::Choice(5)),
            (DETAILS, ControlValue::Choice(0)),
        ]);
        assert_eq!(value(&settings, "shadows"), Some(Value::Float(0.75)));
        // √2², within a step of the platform's powf.
        let Some(Value::Float(sigma)) = value(&settings, SMOOTHING) else {
            panic!("curve smoothing sets σ");
        };
        assert!((sigma - 2.0).abs() <= 2.0 * f32::EPSILON, "{sigma}");
        assert_eq!(
            value(&settings, "exposure_boost"),
            Some(Value::Float(-1.57))
        );
        assert_eq!(value(&settings, "blending"), Some(Value::Float(2.0)));
        assert_eq!(value(&settings, ITERATIONS), Some(Value::Int(5)));
        assert_eq!(value(&settings, METHOD), Some(Value::Choice(5)));
        assert_eq!(value(&settings, DETAILS), Some(Value::Choice(0)));
        assert_eq!(settings.len(), 7);
    }

    /// Every control at its default, `set` over them, as the panel hands `Module::curve`
    /// its values.
    fn shown(set: &[(&str, ControlValue)]) -> Controls {
        let mut values = DEFINITION.defaults();
        for (control, value) in set {
            let id = CONTROLS.iter().find(|c| c.id == *control).unwrap().id;
            values.set(id, value.clone());
        }
        values
    }

    #[test]
    fn the_graph_draws_the_curve_the_operation_renders() {
        // Test 0131's bands at σ 1.2, as darktable's slider shows it, then at both ends of
        // the smoothing slider: the graph's curve is the operation's setup's, exactly.
        let bands = [
            -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
        ];
        let s = 1.2f32.ln() / std::f32::consts::SQRT_2.ln() - 1.0;
        for smoothing in [s, -2.33, 1.67, 0.0] {
            let mut set: Vec<(&str, ControlValue)> = BANDS
                .iter()
                .zip(bands)
                .map(|(&band, value)| (band, ControlValue::Number(value)))
                .collect();
            set.push((SMOOTHING, ControlValue::Number(smoothing)));
            let mut params = apertrust_plugin::ParamValues::new();
            for setting in bind(&set) {
                params.set(setting.param, setting.value);
            }
            let params = ToneEqual.schema().resolve(&params).unwrap();
            let prepared = toneequal::prepare(
                &toneequal::settings(&params),
                &toneequal::Geometry::full(8, 8),
            )
            .unwrap();
            let values = shown(&set);
            let curve = ToneEqualizer.curve(BANDS[0], &values).unwrap();
            for k in 0..=800 {
                let exposure = -8.0 + k as f32 / 100.0;
                let rendered = prepared.correction(f64::from(exposure)).log2() as f32;
                assert_eq!(curve(exposure), rendered, "s {smoothing}, {exposure} EV");
            }
            // A drag on the graph reaches as far as the curve is wide: the σ bound.
            let sigma = ToneEqualizer.curve_width(BANDS[4], &values).unwrap();
            assert_eq!(sigma, toneequal::settings(&params).smoothing);
        }
    }

    #[test]
    fn the_graph_at_the_defaults_is_darktable_s_ripple() {
        // On at its defaults the module renders darktable's zero bands' ripple, so the
        // curve is that ripple; the panel draws it flat while the module is off.
        let ripple = ToneEqualizer
            .curve(BANDS[0], &DEFINITION.defaults())
            .unwrap();
        let highest = (0..=800)
            .map(|k| ripple(-8.0 + k as f32 / 100.0))
            .fold(f32::MIN, f32::max);
        assert!((highest - 0.0253).abs() < 1e-3, "{highest}");
        let stepped = shown(&[("exposure_boost", ControlValue::Number(0.05))]);
        // Only the bands have a graph.
        assert!(ToneEqualizer.curve(SMOOTHING, &stepped).is_none());
        assert!(ToneEqualizer.curve_width(SMOOTHING, &stepped).is_none());
        assert_eq!(
            ToneEqualizer.curve_width(BANDS[0], &stepped),
            Some(std::f32::consts::SQRT_2)
        );
    }

    #[test]
    fn the_guided_filters_are_listed_and_refused() {
        let details = DEFINITION.control(DETAILS).unwrap();
        for (i, option) in FILTERS.iter().enumerate() {
            let refused = details.check(&ControlValue::Choice(i as u32)).is_err();
            assert_eq!(refused, GUIDED_FILTERS.contains(option), "{option}");
        }
    }
}
