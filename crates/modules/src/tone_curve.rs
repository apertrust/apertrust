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

//! Tone curve (`docs/modules/tone-curve.md`): curves drawn over the photo's tones, before
//! the tone mapper at darktable's place (decision 22), on lightness axes with middle grey
//! at 0.5.
//!
//! - **Luma:** up to [`MAX_POINTS`] points between the curve's ends, (0, 0) and (1, 1),
//!   which stay where they are; none by default, the straight curve. Each pixel's
//!   luminance passes through the curve, and its colour is kept by scaling its three
//!   channels alike.
//! - **Advanced:** Preserve colours, what the Luma curve acts on in each pixel: luminance,
//!   as by default, or max, average, sum, norm or basic power, RGB curve's other norms.
//! - **RGB, Red, Green and Blue** have no rows, so the panel draws their tabs disabled;
//!   they give way to RGB curve's tabs when the module becomes RGB curve (decision 2).
//!
//! It drives one `rgbcurve` instance, linked, monotone Hermite and middle grey
//! compensated, planned only while the Luma curve is not straight: a
//! straight curve sets nothing, so it adds no operation and leaves the frame byte for byte
//! as it was. It becomes RGB curve's linked curve (decision 2).

use apertrust_color::WorkingProfile;
use apertrust_color::curve::{self, Interpolation, Knot};
use apertrust_plugin::{ImageContext, Value};

use crate::definition::{
    ChoiceStyle, Control, ControlKind, Definition, MAX_POINTS, Row, Tab, Widget,
};
use crate::{Base, ControlValue, Controls, Module, ModuleValues, Owned, Setting};

pub struct ToneCurve;

/// The operation it drives.
const RGB_CURVE: &str = "rgbcurve";

/// The Luma curve's points, and Preserve colours.
pub const LUMA: &str = "luma_points";
pub const PRESERVE: &str = "preserve_colour";

/// The curve's ends, which hold still: the module adds them to the points it stores, which
/// lie between them, so a saved edit keeps its meaning whatever the handles come to set.
pub const ENDS: [[f32; 2]; 2] = [[0.0, 0.0], [1.0, 1.0]];

/// RGB curve's nodes per curve: the points and the two ends.
const NODES_PER_CURVE: usize = MAX_POINTS + 2;

/// Preserve colours' options, luminance first as the default, by RGB curve's names for its
/// norms (`docs/ports/rgbcurve.md`, Parameters). RGB curve's `none` is the RGB tab's.
pub const NORMS: [&str; 6] = [
    "luminance",
    "max_rgb",
    "average_rgb",
    "sum_rgb",
    "norm_rgb",
    "basic_power",
];

/// Each of [`NORMS`] as RGB curve's `preserve_colors` choice.
const NORM_CHOICES: [u32; 6] = [1, 2, 3, 4, 5, 6];

/// Preserve colours' default, [`NORMS`]'s luminance.
pub const LUMINANCE: u32 = 0;

/// RGB curve's `curve_autoscale` linked and `curve_type` monotone Hermite.
const LINKED: u32 = 0;
const MONOTONE_HERMITE: u32 = 2;

static DEFINITION: Definition = Definition {
    id: "tone_curve",
    version: 1,
    layers: true,
    auto: false,
    switched: false,
    controls: &[
        Control {
            id: LUMA,
            kind: ControlKind::Points { default: &[] },
            unit: None,
            unported: false,
        },
        Control {
            id: PRESERVE,
            kind: ControlKind::Choice {
                options: &NORMS,
                default: LUMINANCE,
                unported: &[],
            },
            unit: None,
            unported: false,
        },
    ],
    tabs: &[
        Tab {
            id: Some("luma"),
            rows: &[Row::Custom {
                widget: Widget::Curve,
                controls: &[LUMA],
            }],
        },
        // To come when the module becomes RGB curve (decision 2).
        Tab {
            id: Some("rgb"),
            rows: &[],
        },
        Tab {
            id: Some("red"),
            rows: &[],
        },
        Tab {
            id: Some("green"),
            rows: &[],
        },
        Tab {
            id: Some("blue"),
            rows: &[],
        },
        Tab {
            id: Some("advanced"),
            rows: &[Row::Choice {
                control: PRESERVE,
                style: ChoiceStyle::List,
            }],
        },
    ],
    before: &[],
    after: &[],
    presets: &[],
    views: &[],
};

/// RGB curve's node parameters, `curve_nodes_<curve>_<node>_x` and `_y`, by curve and
/// node.
macro_rules! nodes {
    ($($curve:literal)*; $nodes:tt) => {
        [$(nodes!(@curve $curve; $nodes)),*]
    };
    (@curve $curve:literal; [$($node:literal)*]) => {
        [$([
            concat!("curve_nodes_", $curve, "_", $node, "_x"),
            concat!("curve_nodes_", $curve, "_", $node, "_y"),
        ]),*]
    };
}

const NODES: [[[&str; 2]; NODES_PER_CURVE]; 3] =
    nodes!(0 1 2; [0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19]);

/// RGB curve's parameters other than its nodes.
const SETTINGS: [&str; 9] = [
    "curve_num_nodes_0",
    "curve_num_nodes_1",
    "curve_num_nodes_2",
    "curve_type_0",
    "curve_type_1",
    "curve_type_2",
    "curve_autoscale",
    "compensate_middle_grey",
    "preserve_colors",
];

const OWNED: usize = 3 * NODES_PER_CURVE * 2 + SETTINGS.len();

/// Every RGB curve parameter: the module owns all of them, and no other
/// module sets them.
static OWNS: [Owned; OWNED] = owns();

const fn owns() -> [Owned; OWNED] {
    let mut owns = [Owned::new(RGB_CURVE, SETTINGS[0]); OWNED];
    let nodes = 3 * NODES_PER_CURVE * 2;
    let mut i = 0;
    while i < nodes {
        owns[i] = Owned::new(
            RGB_CURVE,
            NODES[i / (2 * NODES_PER_CURVE)][i / 2 % NODES_PER_CURVE][i % 2],
        );
        i += 1;
    }
    while i < OWNED {
        owns[i] = Owned::new(RGB_CURVE, SETTINGS[i - nodes]);
        i += 1;
    }
    owns
}

impl Module for ToneCurve {
    fn definition(&self) -> &'static Definition {
        &DEFINITION
    }

    fn owns(&self) -> &'static [Owned] {
        &OWNS
    }

    /// The Luma curve's instance while the curve is not straight, and nothing while it
    /// is, whatever Preserve colours holds.
    fn bind(&self, values: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
        match values.points(LUMA) {
            Some(points) if !straight(points) => {
                luma(points, values.choice(PRESERVE).unwrap_or(LUMINANCE))
            }
            _ => Vec::new(),
        }
    }

    /// The Luma curve as RGB curve renders it, on lightness axes ([`LumaCurve`]).
    fn curve(&self, control: &str, values: &Controls) -> Option<Box<dyn Fn(f32) -> f32>> {
        let Some(ControlValue::Points(points)) = values.get(control).filter(|_| control == LUMA)
        else {
            return None;
        };
        if straight(points) {
            // Exactly the diagonal, so a point added on it keeps the curve straight.
            return Some(Box::new(|x| x));
        }
        let curve = LumaCurve::new(points)?;
        Some(Box::new(move |x| curve.at(x)))
    }
}

/// Whether the points between the ends draw the identity: every one on the diagonal, as
/// the ends are. Such a curve changes nothing, so it plans no operation.
pub fn straight(points: &[[f32; 2]]) -> bool {
    points.iter().all(|[x, y]| x == y)
}

/// The curve's nodes: the ends with the points between them.
pub fn nodes(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let [start, end] = ENDS;
    std::iter::once(start)
        .chain(points.iter().copied())
        .chain(std::iter::once(end))
        .collect()
}

/// The Luma curve's instance through the ends and `points`, preserving colours by
/// Preserve colours' option `preserve`: RGB curve linked, its first curve through those
/// nodes, monotone Hermite, on lightness axes. It is what [`ToneCurve::bind`] gives for a
/// curve that is not straight; the invariant tests plan it straight too, as the first
/// step (row T1).
pub fn luma(points: &[[f32; 2]], preserve: u32) -> Vec<Setting> {
    let nodes = nodes(points);
    let mut settings = vec![
        Setting::new(RGB_CURVE, "curve_autoscale", Value::Choice(LINKED)),
        Setting::new(RGB_CURVE, "curve_type_0", Value::Choice(MONOTONE_HERMITE)),
        Setting::new(RGB_CURVE, "compensate_middle_grey", Value::Bool(true)),
        Setting::new(
            RGB_CURVE,
            "preserve_colors",
            Value::Choice(NORM_CHOICES[preserve as usize]),
        ),
        Setting::new(
            RGB_CURVE,
            "curve_num_nodes_0",
            Value::Int(nodes.len() as i64),
        ),
    ];
    for ([x_param, y_param], &[x, y]) in NODES[0].iter().zip(&nodes) {
        settings.push(Setting::new(RGB_CURVE, x_param, Value::Float(x)));
        settings.push(Setting::new(RGB_CURVE, y_param, Value::Float(y)));
    }
    settings
}

/// The Luma curve as RGB curve renders it (`docs/ports/rgbcurve.md`, Setup and Per
/// pixel), to draw it and to test against: the nodes made linear, darktable's monotone
/// Hermite tangent at each (`monotone_hermite_set`), cubic Hermite segments between them,
/// clamped to 0 to 1, and past the last node darktable's power law fitted to the curve,
/// by the curve maths RGB curve's setup uses (`apertrust_color::curve`). The module's
/// tests hold the knots it builds to those the setup builds from the binding.
///
/// It is the curve a grey's luminance goes through. With Preserve colours on sum or norm,
/// a grey's measure is 3 or √3 times its value, so greys do not render as it draws
/// (`docs/modules/tone-curve.md`, T17).
#[derive(Debug, Clone, PartialEq)]
pub struct LumaCurve {
    /// Each point in linear values with the curve's slope there.
    knots: Vec<Knot>,
    /// Past the last point, `end_value · (v / x_last)^end_power`.
    end_value: f32,
    end_power: f32,
    profile: WorkingProfile,
}

impl LumaCurve {
    /// The curve through the ends and `points`, the points between them. `None` where RGB
    /// curve cannot interpolate them: more than [`MAX_POINTS`], or x not increasing.
    pub fn new(points: &[[f32; 2]]) -> Option<Self> {
        let profile = WorkingProfile::linear_rec2020();
        let nodes = nodes(points);
        let linear = |c: usize| -> Vec<f32> {
            nodes
                .iter()
                .map(|node| profile.uncompensate_middle_grey(node[c]))
                .collect()
        };
        let (x, y) = (linear(0), linear(1));
        let tangents = curve::tangents(Interpolation::MonotoneHermite, &x, &y)?;
        let knots: Vec<Knot> = (0..x.len())
            .map(|i| Knot {
                x: x[i],
                y: y[i],
                tangent: tangents[i],
            })
            .collect();
        let (end_value, end_power) = curve::extrapolation(&knots);
        Some(Self {
            knots,
            end_value,
            end_power,
            profile,
        })
    }

    /// The output lightness for the input lightness `x`: the lightness of the grey the
    /// curve makes of the grey whose lightness is `x`. What the panel draws.
    pub fn at(&self, x: f32) -> f32 {
        let v = self.profile.uncompensate_middle_grey(x);
        self.profile.compensate_middle_grey(self.linear(v))
    }

    /// The curve on linear values, as the kernel's `curve_at` evaluates it.
    pub fn linear(&self, v: f32) -> f32 {
        let last = self.knots[self.knots.len() - 1].x;
        if v >= last {
            self.end_value * (v / last).powf(self.end_power)
        } else {
            curve::evaluate(&self.knots, v)
        }
    }
}

#[cfg(test)]
mod tests {
    use apertrust_operations::OPERATIONS;
    use apertrust_operations::rgbcurve::{self, Mode, Norm};
    use apertrust_plugin::ParamValues;

    use super::*;
    use crate::check;

    fn bind(set: &[(&str, ControlValue)]) -> Vec<Setting> {
        let image = crate::testing::image();
        let mut values = ModuleValues::new();
        for (control, value) in set {
            values.set(control, value.clone());
        }
        let values = values.off_defaults(&ToneCurve.defaults(&image));
        ToneCurve.bind(&values, &image, &Base::new(OPERATIONS, &image))
    }

    fn points(points: &[[f32; 2]]) -> ControlValue {
        ControlValue::Points(points.to_vec())
    }

    /// RGB curve's settings for what the binding sets, every other parameter at its
    /// default.
    fn operation_settings(settings: &[Setting]) -> rgbcurve::Settings {
        let operation = apertrust_operations::operation(RGB_CURVE).unwrap();
        let mut params = ParamValues::new();
        for setting in settings {
            params.set(setting.param, setting.value);
        }
        rgbcurve::settings(&operation.schema().resolve(&params).unwrap())
    }

    /// darktable's "contrast | medium (linear)" preset's nodes between its ends
    /// (`rgbcurve.c`, `init_presets`).
    const MEDIUM: [[f32; 2]; 5] = [
        [0.08, 0.06],
        [0.17, 0.14],
        [0.5, 0.5],
        [0.83, 0.86],
        [0.92, 0.94],
    ];

    #[test]
    fn the_definition_holds_and_its_binding_sweeps_inside_the_schema() {
        ToneCurve.definition().check().unwrap();
        let (accepted, rejected) = check::accept(&[&ToneCurve], OPERATIONS);
        assert!(rejected.is_empty(), "{rejected:?}");
        assert_eq!(accepted.len(), 1);
        let problems = check::sweep(&ToneCurve, OPERATIONS, &crate::testing::image());
        assert!(problems.is_empty(), "{problems:?}");
        // It owns every RGB curve parameter, each once.
        let schema = apertrust_operations::operation(RGB_CURVE).unwrap().schema();
        assert_eq!(OWNS.len(), schema.params.len());
    }

    #[test]
    fn it_has_no_slider_as_darktable_s_rgb_curve_has_none() {
        // darktable's rgb curve draws its curve, lists and a toggle (`src/iop/rgbcurve.c`,
        // `gui_init`): `docs/modules/tone-curve.md`'s slider table is empty.
        let differences =
            crate::testing::slider_differences(&ToneCurve, &DEFINITION.defaults(), &[]);
        assert!(differences.is_empty(), "{differences:#?}");
    }

    #[test]
    fn a_straight_curve_sets_nothing_whatever_preserve_colours_holds() {
        assert!(bind(&[]).is_empty());
        for curve in [
            &[][..],
            &[[0.5, 0.5]],
            &[[0.1, 0.1], [0.25, 0.25], [0.9, 0.9]],
        ] {
            for preserve in 0..NORMS.len() as u32 {
                let set = [
                    (LUMA, points(curve)),
                    (PRESERVE, ControlValue::Choice(preserve)),
                ];
                assert!(
                    bind(&set).is_empty(),
                    "{curve:?}, {}",
                    NORMS[preserve as usize]
                );
            }
        }
        // Preserve colours alone sets nothing.
        assert!(bind(&[(PRESERVE, ControlValue::Choice(2))]).is_empty());
        assert!(!straight(&[[0.25, 0.25], [0.5, 0.5000001]]));
    }

    #[test]
    fn the_luma_curve_sets_a_linked_instance_through_its_ends_and_points() {
        let one = [[0.25, 0.35]];
        let settings = bind(&[(LUMA, points(&one))]);
        let resolved = operation_settings(&settings);
        assert_eq!(resolved.mode, Mode::Linked);
        assert!(resolved.compensate_middle_grey);
        assert_eq!(resolved.preserve_colors, Norm::Luminance);
        let nodes: Vec<[f32; 2]> = resolved.curves[0]
            .nodes
            .iter()
            .map(|node| [node.x, node.y])
            .collect();
        assert_eq!(nodes, [[0.0, 0.0], [0.25, 0.35], [1.0, 1.0]]);
        assert_eq!(
            resolved.curves[0].interpolation,
            Interpolation::MonotoneHermite
        );
        for setting in &settings {
            check::setting(&ToneCurve, OPERATIONS, setting).unwrap();
        }
    }

    #[test]
    fn each_preserve_colours_option_is_rgb_curve_s_norm_of_its_name() {
        let schema = apertrust_operations::operation(RGB_CURVE).unwrap().schema();
        let Some(apertrust_plugin::Kind::Choice { choices, .. }) =
            schema.param("preserve_colors").map(|param| param.kind)
        else {
            panic!("preserve_colors is a choice");
        };
        for (option, choice) in NORMS.iter().zip(NORM_CHOICES) {
            assert_eq!(choices[choice as usize], *option);
        }
        assert_eq!(NORMS[LUMINANCE as usize], "luminance");
        let set = |preserve| {
            let settings = bind(&[
                (LUMA, points(&MEDIUM)),
                (PRESERVE, ControlValue::Choice(preserve)),
            ]);
            operation_settings(&settings).preserve_colors
        };
        let norms: Vec<Norm> = (0..NORMS.len() as u32).map(set).collect();
        assert_eq!(
            norms,
            [
                Norm::Luminance,
                Norm::Max,
                Norm::Average,
                Norm::Sum,
                Norm::Norm,
                Norm::Power
            ]
        );
    }

    #[test]
    fn the_drawn_curve_is_the_operation_s() {
        // The curve maths is RGB curve's own; this holds the knots the panel builds, and
        // so their linear values and interpolation, to those the binding gives the setup.
        let profile = WorkingProfile::linear_rec2020();
        for curve in [
            &MEDIUM[..],
            &[[0.25, 0.35]],
            &[[0.1, 0.05], [0.6, 0.8], [0.9, 0.95]],
            &[[0.3, 1.0], [0.7, 0.0]],
        ] {
            let drawn = LumaCurve::new(curve).unwrap();
            let settings = operation_settings(&luma(curve, LUMINANCE));
            let prepared = rgbcurve::prepare(&settings, &profile).unwrap();
            let kernel = &prepared.curves[0];
            assert_eq!(drawn.knots, kernel.knots, "{curve:?}");
            assert_eq!(
                (drawn.end_value, drawn.end_power),
                (kernel.end_value, kernel.end_power),
                "{curve:?}"
            );
            // Drawn on lightness, through each point and the ends.
            for [x, y] in nodes(curve) {
                assert!((drawn.at(x) - y).abs() < 1e-5, "{curve:?}: ({x}, {y})");
            }
        }
        assert!(LumaCurve::new(&[[0.0, 0.5]]).is_none());
        assert!(LumaCurve::new(&[[0.5, 0.0], [0.4, 1.0]]).is_none());
    }

    #[test]
    fn a_straight_curve_draws_the_diagonal_exactly() {
        let mut values = ToneCurve.definition().defaults();
        let drawn = ToneCurve.curve(LUMA, &values).unwrap();
        for x in [0.0, 0.1, 0.333, 0.5, 1.0] {
            assert_eq!(drawn(x), x);
        }
        values.set(LUMA, points(&MEDIUM));
        let drawn = ToneCurve.curve(LUMA, &values).unwrap();
        assert!((drawn(0.08) - 0.06).abs() < 1e-5);
        assert!(ToneCurve.curve(PRESERVE, &values).is_none());
    }
}
