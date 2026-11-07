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

//! Tone curve's invariants (`docs/modules/tone-curve.md`, Invariants) that apply to the
//! Luma curve, one test per row, each named after its row: T0, T1, T4 to T8, T11, T16
//! and T17, T4 and T5 for each Preserve colours option. The synthetic ramp and sweep run
//! on CI's software GPUs:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test tone_curve -- --nocapture
//! ```
//!
//! Synthetic images enter after demosaic, already in the working space. A row that reads
//! the module's output plans RGB curve alone on the image, so the curve sees the image as
//! it is, and compares it with the image; a row that reads the frame runs the pipeline
//! from exposure on, leaving colorin out, RGB curve before AgX as in darktable (decision
//! 22), and reads colorout's output before it is rounded to 8 bits.

mod common;

use apertrust_color::RgbNorm;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::{Instance, Pipe};
use apertrust_modules::tone_curve::{
    self, LUMA, LUMINANCE, LumaCurve, NORMS, PRESERVE, ToneCurve, nodes,
};
use apertrust_modules::{Base, ControlValue, MIN_GAP, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::rgbcurve;
use apertrust_plugin::{ImageContext, ParamValues};
use golden::synthetic;

use common::{
    BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, extended_ramp, id, jump,
    profile, raw_context, small_raw, sweep,
};

/// The ramp (`docs/modules/README.md`, Test images): neutral greys one per 1/16 EV from
/// 2⁻¹⁶ to 2⁴, then the greys whose lightness is each of `lightness`, as the curve's axis
/// reads them. One row.
fn ramp_with(lightness: &[f32]) -> (Picture, ImageContext) {
    let then: Vec<f32> = lightness
        .iter()
        .map(|&x| profile().uncompensate_middle_grey(x))
        .collect();
    common::ramp(&then, 0.0)
}

fn ramp() -> (Picture, ImageContext) {
    ramp_with(&[])
}

/// What a row reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Measure {
    /// RGB curve's output, planned alone on the image.
    Module,
    /// The output, colorout's.
    Frame,
}

impl common::Measure for Measure {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        match self {
            Measure::Module => instances.retain(|i| id(i) == RGB_CURVE),
            Measure::Frame => {
                instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))))
            }
        }
    }
}

const RGB_CURVE: &str = "rgbcurve";

impl Bench {
    /// The first step (row T1): RGB curve planned at the Luma curve's own defaults, the
    /// identity, linked, preserving colours by luminance, the binding's output with each
    /// default made explicit, over an unedited photo.
    fn first_step(&self, measure: Measure) -> Vec<Instance<'static>> {
        let mut params = ParamValues::new();
        for setting in tone_curve::luma(&[], LUMINANCE) {
            params.set(setting.param, setting.value);
        }
        let mut instances = edit::instances(&Edit::default(), OPERATIONS, MODULES, &self.image)
            .all()
            .unwrap();
        instances.push(Instance {
            operation: apertrust_operations::operation(RGB_CURVE).unwrap(),
            params,
        });
        self.keep(instances, measure)
    }

    /// The Luma curve through `points`, preserving colours by option `preserve`.
    fn curve(&mut self, points: &[[f32; 2]], preserve: u32, measure: Measure) -> Rendered {
        self.edit(&curve_edit(points, preserve), measure)
    }
}

/// An edit with the Luma curve through `points`, preserving colours by option `preserve`
/// when it is not the default.
fn curve_edit(points: &[[f32; 2]], preserve: u32) -> Edit {
    let mut edit = Edit::default();
    edit.set_control(
        &ToneCurve,
        LUMA,
        ControlValue::Points(points.to_vec()),
        &common::photo(),
    )
    .unwrap();
    if preserve != LUMINANCE {
        edit.set_control(
            &ToneCurve,
            PRESERVE,
            ControlValue::Choice(preserve),
            &common::photo(),
        )
        .unwrap();
    }
    edit
}

/// A working-space pixel's luminance, from the working space's luminance row, as the
/// curve's luminance norm reads it.
fn luminance(pixel: [f32; 4]) -> f32 {
    let row = profile().rgb_to_xyz_d50[1];
    row[0] * pixel[0] + row[1] * pixel[1] + row[2] * pixel[2]
}

/// Where a pixel's luminance sits on the curve's axis: its lightness, 0.5 at middle grey.
fn axis(pixel: [f32; 4]) -> f32 {
    profile().compensate_middle_grey(luminance(pixel))
}

/// darktable's presets the rows draw (`rgbcurve.c`, `init_presets`), in lightness as they
/// store them, with one point at (0.25, 0.35): the points between the ends, (0, 0) and
/// (1, 1), which every one of them has.
const CURVES: [(&str, &[[f32; 2]]); 4] = [
    (
        "contrast medium (linear)",
        &[
            [0.08, 0.06],
            [0.17, 0.14],
            [0.5, 0.5],
            [0.83, 0.86],
            [0.92, 0.94],
        ],
    ),
    (
        "contrast high (linear)",
        &[
            [0.08, 0.04],
            [0.17, 0.11],
            [0.5, 0.5],
            [0.83, 0.89],
            [0.92, 0.96],
        ],
    ),
    (
        "contrast compression",
        &[
            [0.003862, 0.007782],
            [0.076613, 0.156182],
            [0.169355, 0.290352],
            [0.774194, 0.773852],
        ],
    ),
    ("one point", &[[0.25, 0.35]]),
];

/// Preserve colours' option called `name`.
fn option(name: &str) -> u32 {
    NORMS.iter().position(|norm| *norm == name).unwrap() as u32
}

/// The options whose measure of a grey is its value, so greys render as the curve is
/// drawn, and the two whose measure is a multiple of it: sum, 3 v, and norm, √3 v.
const GREYS_AS_DRAWN: [&str; 4] = ["luminance", "max_rgb", "average_rgb", "basic_power"];
const GREYS_SCALED: [&str; 2] = ["sum_rgb", "norm_rgb"];

// T0: every tab at the identity.

/// Edits whose curves are all straight: the default written out, points added on the
/// diagonal, each with every Preserve colours option, and the controls kept untouched.
fn straight_edits() -> Vec<Edit> {
    let mut edits = Vec::new();
    for points in [
        &[][..],
        &[[0.5, 0.5]],
        &[[0.1, 0.1], [0.25, 0.25], [0.9, 0.9]],
    ] {
        for preserve in 0..NORMS.len() as u32 {
            edits.push(curve_edit(points, preserve));
        }
    }
    let mut kept = Edit::default();
    for control in [LUMA, PRESERVE] {
        kept.keep(&ToneCurve, control, true).unwrap();
    }
    edits.push(kept);
    edits
}

/// T0 on one pipeline, as the session renders: no edit, then RGB curve joining and
/// leaving the plan, then each straight edit, every frame as the first.
fn t0_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Measure::Frame);
    let joined = bench.first_step(Measure::Frame);
    bench.render(&joined);
    let base = Base::new(OPERATIONS, &bench.image);
    for edit in straight_edits() {
        if let Some(values) = edit.module("tone_curve") {
            let values = values.off_defaults(&ToneCurve.defaults(&bench.image));
            assert!(ToneCurve.bind(&values, &bench.image, &base).is_empty());
        }
        let straight = bench.edit(&edit, Measure::Frame);
        assert!(exact(&none, &straight), "T0 on {:?}: {edit:?}", bench.pipe);
    }
}

#[test]
fn t0_every_curve_straight_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(0.0);
        t0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
    for (picture, image) in [ramp(), sweep(0.0)] {
        t0_holds(&mut Bench::picture(&picture, image, Pipe::Export));
    }
}

// T1: the Luma curve's first step.

#[test]
fn t1_the_luma_curve_s_first_step_changes_no_pixel_beyond_the_curve_s_precision() {
    // The module's output against its input: within 1e-5, relative above 1.
    for (name, (picture, image)) in [
        ("the ramp", ramp()),
        ("the extended ramp", extended_ramp(0.0)),
        ("the sweep", sweep(0.0)),
    ] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let first = bench.first_step(Measure::Module);
        let output = bench.render(&first);
        let mut worst = 0.0_f32;
        for (before, after) in picture.pixels.iter().zip(&output.pixels) {
            for c in 0..3 {
                let error = (after[c] - before[c]).abs() / before[c].abs().max(1.0);
                worst = worst.max(error);
            }
        }
        println!("T1 on {name}, the module's output: at most {worst:.2e} off its input");
        assert!(worst <= 1e-5, "T1 on {name}: {worst:e}");
    }
    // The frame against the untouched frame: a max ΔE2000 of 0.01.
    let frames = |bench: &mut Bench| {
        let untouched = bench.edit(&Edit::default(), Measure::Frame);
        let first = bench.first_step(Measure::Frame);
        let first = bench.render(&first);
        jump(&untouched.pixels, &first.pixels, |_| true)
    };
    let (mosaic, image) = small_raw(0.0);
    let mut benches = vec![("the small raw", Bench::mosaic(&mosaic, image, Pipe::Export))];
    for (name, (picture, image)) in [("the ramp", ramp()), ("the sweep", sweep(0.0))] {
        benches.push((name, Bench::picture(&picture, image, Pipe::Export)));
    }
    for (name, mut bench) in benches {
        let Jump { max, mean, .. } = frames(&mut bench);
        println!("T1 on {name}, the frame: ΔE2000 mean {mean:.2e}, max {max:.2e}");
        assert!(max <= 0.01, "T1 on {name}: a max ΔE2000 of {max}");
    }
}

// T4 to T7 and T17: the Luma curve's points, on the module's output; T4, T5 and T17 for
// each Preserve colours option.

/// Each grey the curve's output: where its input sits on the axis, and where its output
/// does.
fn on_the_axis(input: &Picture, output: &Rendered) -> Vec<(f32, f32)> {
    input
        .pixels
        .iter()
        .zip(&output.pixels)
        .map(|(&before, &after)| (axis(before), axis(after)))
        .collect()
}

/// T4 with Preserve colours on `norm`, for each of T4's curves on the ramp, with greys at
/// each node's input: the most a grey renders off each node, and off the curve drawn
/// between them, in axis units.
fn rendered_as_drawn(norm: &str) -> Vec<(&'static str, f32, f32)> {
    let mut results = Vec::new();
    for (name, points) in CURVES {
        let nodes = nodes(points);
        let xs: Vec<f32> = nodes.iter().map(|node| node[0]).collect();
        let (picture, image) = ramp_with(&xs);
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        let output = bench.curve(points, option(norm), Measure::Module);
        let drawn = LumaCurve::new(points).unwrap();
        let measured = on_the_axis(&picture, &output);
        // Through each node, at the greys whose lightness is each node's input.
        let mut at_nodes = 0.0_f32;
        for (&(_, y), node) in measured[measured.len() - nodes.len()..].iter().zip(&nodes) {
            at_nodes = at_nodes.max((y - node[1]).abs());
        }
        // Between them as the curve drawn, wherever the ramp's greys fall.
        let mut between = 0.0_f32;
        for &(x, y) in &measured {
            if (0.0..=1.0).contains(&x) {
                between = between.max((y - drawn.at(x)).abs());
            }
        }
        println!(
            "T4 with {norm}, {name}: at most {at_nodes:.1e} off at the points, {between:.1e} between them"
        );
        results.push((name, at_nodes, between));
    }
    results
}

/// T4's bound, 2e-4 at the points and 1e-4 between them, with each of `norms`.
fn t4_holds(norms: &[&str]) {
    let mut failures = Vec::new();
    for norm in norms {
        for (name, at_nodes, between) in rendered_as_drawn(norm) {
            if at_nodes > 2e-4 || between > 1e-4 {
                failures.push(format!(
                    "{norm}, {name}: {at_nodes} at the points, {between} between"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "T4: {failures:#?}");
}

#[test]
fn t4_luma_points_render_as_drawn() {
    t4_holds(&GREYS_AS_DRAWN);
}

/// T4 with Preserve colours on sum or norm. Their measure of a grey v is 3v and √3·v, so
/// the grey renders C(3v)/3 and C(√3·v)/√3, not C(v), the curve the panel draws. Measured
/// on T4's curves, the worst of each: 0.073 to 0.19 of the axis off with sum, 0.044 to
/// 0.10 with norm, both largest on contrast high (docs/modules/tone-curve.md, Results).
#[test]
#[ignore = "fails its bound with Preserve colours on sum and norm, whose greys do not render as the curve drawn: the user decides (docs/modules/tone-curve.md, T4 and T17)"]
fn t4_luma_points_render_as_drawn_with_sum_and_norm() {
    t4_holds(&GREYS_SCALED);
}

#[test]
fn t5_luma_points_scale_each_pixel_s_channels_alike_whatever_the_norm() {
    let (picture, image) = sweep(0.0);
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let mut failures = Vec::new();
    for (preserve, norm) in NORMS.iter().enumerate() {
        for (name, points) in CURVES {
            let output = bench.curve(points, preserve as u32, Measure::Module);
            let mut worst = (0.0_f64, 0);
            for (i, (before, after)) in picture.pixels.iter().zip(&output.pixels).enumerate() {
                if luminance(*before) <= 1e-6 {
                    continue;
                }
                // A channel at 0 stays 0 whatever the factor.
                let factors: Vec<f64> = (0..3)
                    .filter(|&c| before[c] != 0.0)
                    .map(|c| f64::from(after[c]) / f64::from(before[c]))
                    .collect();
                for c in (0..3).filter(|&c| before[c] == 0.0) {
                    assert_eq!(after[c], 0.0, "T5, {name}: {}", synthetic::describe(i));
                }
                let (low, high) = factors
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), &f| {
                        (l.min(f), h.max(f))
                    });
                let spread = (high - low) / high.abs().max(low.abs());
                if spread > worst.0 {
                    worst = (spread, i);
                }
            }
            println!(
                "T5 with {norm}, {name}: a channel spread of at most {:.1e}, at {}",
                worst.0,
                synthetic::describe(worst.1)
            );
            if worst.0 > 1e-6 {
                failures.push(format!(
                    "{norm}, {name}: {:e} at {}",
                    worst.0,
                    synthetic::describe(worst.1)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "T5: {failures:#?}");
}

/// The norm RGB curve preserves colours by for Preserve colours' option `preserve`, as
/// the binding sets it.
fn norm_of(preserve: u32) -> RgbNorm {
    tone_curve::luma(&[], preserve)
        .into_iter()
        .find_map(|setting| match (setting.param, setting.value) {
            ("preserve_colors", apertrust_plugin::Value::Choice(choice)) => {
                RgbNorm::from_darktable(choice as i32)
            }
            _ => None,
        })
        .unwrap()
}

/// An option's measure of a pixel (`RgbNorm::of`), read in f64, so that rounding the
/// measure here, where luminance's terms nearly cancel on colours beyond the gamut, does
/// not count against the row.
fn measure(norm: RgbNorm, pixel: [f32; 4]) -> f64 {
    let [r, g, b] = [pixel[0], pixel[1], pixel[2]].map(f64::from);
    let row = profile().rgb_to_xyz_d50[1].map(f64::from);
    let squares = r * r + g * g + b * b;
    match norm {
        RgbNorm::Luminance => row[0] * r + row[1] * g + row[2] * b,
        RgbNorm::Max => r.max(g).max(b),
        RgbNorm::Average | RgbNorm::None => (r + g + b) / 3.0,
        RgbNorm::Sum => r + g + b,
        RgbNorm::Norm => squares.sqrt(),
        RgbNorm::Power if squares > 0.0 => (r * r * r + g * g * g + b * b * b) / squares,
        RgbNorm::Power => 0.0,
    }
}

/// T17's bound: an option's measure of the output within 1e-5 of the curve of its
/// measure of the input, relative, on pixels whose measure is over 1e-6.
#[test]
fn t17_the_curve_acts_on_the_measure_each_option_names() {
    let mut failures = Vec::new();
    for (image_name, (picture, image)) in [("the ramp", ramp()), ("the sweep", sweep(0.0))] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        for (preserve, option) in NORMS.iter().enumerate() {
            let norm = norm_of(preserve as u32);
            for (name, points) in CURVES {
                let output = bench.curve(points, preserve as u32, Measure::Module);
                let curve = LumaCurve::new(points).unwrap();
                let mut worst = (0.0_f64, 0);
                for (i, (before, after)) in picture.pixels.iter().zip(&output.pixels).enumerate() {
                    let input = measure(norm, *before);
                    if input <= 1e-6 {
                        continue;
                    }
                    let want = f64::from(curve.linear(input as f32));
                    let error = (measure(norm, *after) - want).abs() / want.abs().max(1e-6);
                    if error > worst.0 {
                        worst = (error, i);
                    }
                }
                let at = if image_name == "the sweep" {
                    synthetic::describe(worst.1)
                } else {
                    format!("grey {}", worst.1)
                };
                println!(
                    "T17 with {option}, {name}, on {image_name}: the measure at most {:.1e} off the curve, at {at}",
                    worst.0
                );
                if worst.0 > 1e-5 {
                    failures.push(format!(
                        "{option}, {name}, on {image_name}: {:e} at {at}",
                        worst.0
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "T17: {failures:#?}");
}

/// Curves whose points rise from left to right: T4's and a steep S.
fn rising() -> Vec<(&'static str, &'static [[f32; 2]])> {
    let mut curves = CURVES.to_vec();
    curves.push(("a steep S", &[[0.4, 0.1], [0.6, 0.9]]));
    curves
}

/// How far a rendered value may stray from where the curve puts it through the measure
/// alone: f32's rounding of the luminance read back and of its lightness.
const ROUNDING: f32 = 1e-6;

#[test]
fn t6_rising_points_keep_tones_in_order_with_nothing_overshooting() {
    let (picture, image) = ramp();
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    for (name, points) in rising() {
        let output = bench.curve(points, LUMINANCE, Measure::Module);
        // The ramp's greys rise, so their outputs must not fall.
        for (i, pair) in output.pixels.windows(2).enumerate() {
            assert!(
                luminance(pair[1]) >= luminance(pair[0]),
                "T6, {name}: grey {} renders darker than the grey before it",
                i + 1
            );
        }
        // Between two points, the ends among them, the curve stays between their heights.
        let nodes = nodes(points);
        let mut worst = 0.0_f32;
        for (x, y) in on_the_axis(&picture, &output) {
            let Some(segment) = nodes.windows(2).find(|w| (w[0][0]..=w[1][0]).contains(&x)) else {
                continue;
            };
            let (low, high) = (segment[0][1], segment[1][1]);
            worst = worst.max(low - y).max(y - high);
        }
        println!("T6, {name}: overshoots by at most {worst:.1e}");
        assert!(worst <= ROUNDING, "T6, {name}: overshoots by {worst}");
    }
}

#[test]
fn t7_a_point_added_on_the_identity_changes_nothing() {
    // On the identity a point is on the diagonal, so the curve stays straight: nothing
    // changes, byte for byte.
    let (picture, image) = ramp();
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let none = bench.edit(&Edit::default(), Measure::Frame);
    for x in [0.1, 0.5, 0.9] {
        let added = bench.curve(&[[x, x]], LUMINANCE, Measure::Frame);
        assert!(exact(&none, &added), "T7 on the identity at {x}");
    }
}

/// The most a point added on T4's curves, on the curve drawn, at x 0.1, 0.5 and 0.9 moves
/// the curve on the ramp, in axis units: each curve's name, the x and the move. Where a
/// curve already has a point closer than the gap, there is none to add.
fn points_added_on_curves() -> Vec<(&'static str, f32, f32)> {
    let (picture, image) = ramp();
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let mut moves = Vec::new();
    for (name, points) in CURVES {
        let before = bench.curve(points, LUMINANCE, Measure::Module);
        let drawn = LumaCurve::new(points).unwrap();
        for x in [0.1_f32, 0.5, 0.9] {
            if nodes(points).iter().any(|p| (p[0] - x).abs() < MIN_GAP) {
                continue;
            }
            let mut added = points.to_vec();
            added.insert(added.partition_point(|p| p[0] < x), [x, drawn.at(x)]);
            let after = bench.curve(&added, LUMINANCE, Measure::Module);
            let moved = before
                .pixels
                .iter()
                .zip(&after.pixels)
                .map(|(&b, &a)| (axis(a) - axis(b)).abs())
                .fold(0.0_f32, f32::max);
            println!("T7, {name}, a point at {x}: the curve moves at most {moved:.2e}");
            moves.push((name, x, moved));
        }
    }
    moves
}

/// The part of T7's proposed bound that holds: a point added at 0.1 moves each curve by
/// at most 0.01 of the axis (measured at most 0.0065, on the one-point curve).
#[test]
fn t7_a_point_added_in_the_shadows_moves_the_curve_within_its_proposed_bound() {
    let failures: Vec<_> = points_added_on_curves()
        .into_iter()
        .filter(|&(_, x, moved)| x == 0.1 && moved > 0.01)
        .collect();
    assert!(failures.is_empty(), "T7: {failures:?}");
}

/// The whole proposed bound, which the table asked to measure before confirming:
/// adding a point re-derives the monotone Hermite tangents of its neighbours, so the
/// segments beside it reshape. Measured: up to 0.062 of the axis on contrast compression
/// at 0.5, 0.050 and 0.046 on the one-point curve at 0.5 and 0.9, 0.0100023 on contrast
/// high at 0.9 (docs/modules/tone-curve.md, Results).
#[test]
#[ignore = "fails its proposed bound of 0.01, by up to 0.062 of the axis: the user decides (docs/modules/tone-curve.md, T7)"]
fn t7_adding_a_point_on_a_curve_moves_it_within_its_proposed_bound() {
    let failures: Vec<_> = points_added_on_curves()
        .into_iter()
        .filter(|&(_, _, moved)| moved > 0.01)
        .collect();
    assert!(failures.is_empty(), "T7: {failures:?}");
}

// T8: removing a point.

#[test]
fn t8_removing_a_point_and_putting_it_back_renders_as_before() {
    let (picture, image) = ramp();
    let mut bench = Bench::picture(&picture, image, Pipe::Export);
    let none = bench.edit(&Edit::default(), Measure::Frame);
    let one = [[0.25, 0.35]];
    let bent = bench.curve(&one, LUMINANCE, Measure::Frame);
    assert!(!exact(&none, &bent), "the point changes the frame");
    // Removing the only point leaves the identity, which renders as no edit.
    let mut edit = curve_edit(&one, LUMINANCE);
    edit.set_control(
        &ToneCurve,
        LUMA,
        ControlValue::Points(Vec::new()),
        &common::photo(),
    )
    .unwrap();
    assert!(exact(&none, &bench.edit(&edit, Measure::Frame)), "removed");
    // Putting it back, as undo will, renders as before; so do resetting the control and
    // the module, the curve's Shift+Delete and the header's reset.
    edit.set_control(
        &ToneCurve,
        LUMA,
        ControlValue::Points(one.to_vec()),
        &common::photo(),
    )
    .unwrap();
    assert!(exact(&bent, &bench.edit(&edit, Measure::Frame)), "put back");
    edit.reset_control(&ToneCurve, LUMA).unwrap();
    assert!(exact(&none, &bench.edit(&edit, Measure::Frame)), "reset");
    let mut edit = curve_edit(&one, 2);
    bench.edit(&edit, Measure::Frame);
    edit.reset_module(&ToneCurve);
    assert_eq!(edit, Edit::default());
    assert!(
        exact(&none, &bench.edit(&edit, Measure::Frame)),
        "module reset"
    );
}

// T11: points as commands.

#[test]
fn t11_points_the_operation_cannot_take_are_refused_and_never_bound() {
    let set = |points: Vec<[f32; 2]>| {
        Edit::default().set_control(
            &ToneCurve,
            LUMA,
            ControlValue::Points(points),
            &common::photo(),
        )
    };
    // The most points between the ends, 18, with RGB curve's 20 nodes.
    let eighteen: Vec<[f32; 2]> = (1..=18)
        .map(|k| {
            let x = k as f32 * 0.003;
            [x, x * 10.0]
        })
        .collect();
    let mut nineteen = eighteen.clone();
    nineteen.push([0.9, 0.9]);
    for refused in [
        nineteen,
        vec![[0.6, 0.5], [0.4, 0.7]],
        vec![[0.5, 0.5], [0.502, 0.6]],
        vec![[0.5, 1.2]],
        vec![[-0.1, 0.0]],
        vec![[0.5, f32::NAN]],
        // The ends are the curve's, and a point keeps the gap from them too.
        vec![[0.0, 0.0]],
        vec![[1.0, 1.0]],
        vec![[0.002, 0.5]],
        vec![[0.998, 0.5]],
    ] {
        assert!(set(refused.clone()).is_err(), "{refused:?}");
    }
    // What is taken binds to nodes RGB curve's setup takes.
    let image = raw_context(8, 8, 0.0);
    let base = Base::new(OPERATIONS, &image);
    let operation = apertrust_operations::operation(RGB_CURVE).unwrap();
    for (preserve, points) in [
        eighteen,
        vec![[0.5, 0.5], [0.503, 0.6]],
        vec![[0.2, 0.0], [0.3, 1.0]],
        CURVES[2].1.to_vec(),
        vec![[EDGE, 1.0], [BELOW_ONE, 0.0]],
    ]
    .into_iter()
    .enumerate()
    {
        let edit = curve_edit(&points, preserve as u32);
        let values = edit.module("tone_curve").unwrap();
        let settings = ToneCurve.bind(
            &values.off_defaults(&ToneCurve.defaults(&image)),
            &image,
            &base,
        );
        let mut params = ParamValues::new();
        for setting in &settings {
            params.set(setting.param, setting.value);
        }
        let resolved = operation.schema().resolve(&params).unwrap();
        rgbcurve::prepare(&rgbcurve::settings(&resolved), profile())
            .unwrap_or_else(|error| panic!("{points:?}: {error:?}"));
    }
}

// T16: extremes.

/// Just past the gap, exactly in f32: 41/16384. 0.0025 added to most x falls a hair
/// short of it, as 0.5025 does after 0.5.
const EDGE: f32 = 41.0 / 16384.0;
/// The gap's width below the end at 1, and above 0.5, exactly.
const BELOW_ONE: f32 = 1.0 - EDGE;
const PAST_HALF: f32 = 0.5 + EDGE;

#[test]
fn t16_extreme_curves_render_finite() {
    // Curves to 0 and to 1 between their ends, one that falls, points the gap apart, a
    // step from 0 to 1.
    let extremes: [&[[f32; 2]]; 5] = [
        &[[EDGE, 0.0], [BELOW_ONE, 0.0]],
        &[[EDGE, 1.0], [BELOW_ONE, 1.0]],
        &[[EDGE, 1.0], [BELOW_ONE, 0.0]],
        &[[0.5, 0.2], [PAST_HALF, 0.8]],
        &[[0.5, 0.0], [PAST_HALF, 1.0]],
    ];
    const {
        assert!(EDGE >= MIN_GAP && EDGE < 0.002_51);
        assert!(1.0 - BELOW_ONE >= MIN_GAP && PAST_HALF - 0.5 >= MIN_GAP);
    };
    for (name, (picture, image)) in [
        ("the sweep", sweep(0.0)),
        ("the extended ramp", extended_ramp(0.0)),
    ] {
        let mut bench = Bench::picture(&picture, image, Pipe::Export);
        for points in extremes {
            for preserve in 0..NORMS.len() as u32 {
                for measure in [Measure::Module, Measure::Frame] {
                    let output = bench.curve(points, preserve, measure);
                    let at = output
                        .pixels
                        .iter()
                        .position(|p| !p.iter().all(|v| v.is_finite()));
                    assert_eq!(
                        at, None,
                        "T16 on {name}, {points:?}, {}, {measure:?}",
                        NORMS[preserve as usize]
                    );
                }
            }
        }
    }
}
