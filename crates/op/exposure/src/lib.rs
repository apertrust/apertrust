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

//! Exposure, ported from darktable (`src/iop/exposure.c`): manual mode, with
//! the camera's exposure bias and the exposure a highlight-preserving mode held back
//! compensated, and automatic mode, darktable's deflicker, which puts a percentile of the
//! raw's histogram at a target level below the white point (`docs/ports/exposure.md`).
//! The kernel computes its scale from the black level and the exposure, so the
//! operation's setup only packs them; automatic mode's exposure comes from the raw's
//! histogram, which the engine counts on the GPU and hands over in the image context
//! (decision 87).

use apertrust_plugin::{
    Domain, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Value,
};

/// The id of what [`Operation::computed`] gives: the exposure automatic mode applies,
/// darktable's "computed EC".
pub const COMPUTED_EXPOSURE: &str = "computed_exposure";

/// Generated from `slang/exposure.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "exposure",
    wgsl: include_str!("../generated/exposure.wgsl"),
    derivatives: true,
};

pub struct Exposure;

static MANIFEST: Manifest = Manifest {
    id: "exposure",
    version: 1,
    slot: Slot {
        stage: Stage::CameraRgb,
        order: 21.0,
    },
    domain: Domain::Scene,
    layers: true,
    differentiable: true,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "exposure",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/exposure.c", "data/kernels/basic.cl"],
    }),
};

/// darktable's modes, `dt_iop_exposure_mode_t`: manual, and automatic, its deflicker.
const MODES: [&str; 2] = ["manual", "automatic"];
const MANUAL: u32 = 0;
const AUTOMATIC: u32 = 1;

/// How far below the white a black that reaches it is put: darktable's margin, which its
/// `gui_changed` keeps and the Exposure tool's Black level correction too (decision 30).
const WHITE_MARGIN: f32 = 0.01;

/// The deflicker's percentile and target level at darktable's defaults: they act only in
/// automatic mode.
const PERCENTILE: f32 = 50.0;
const TARGET_LEVEL: f32 = -4.0;

/// darktable `dt_iop_exposure_params_t`, in its order, with its ranges and its
/// scene-referred defaults for a colour raw (`reload_defaults`), highlight-preservation
/// compensation on, as darktable turns it on for a module's first instance. Version 2
/// adds the mode, the deflicker's settings and that compensation.
static SCHEMA: Schema = Schema {
    version: 2,
    params: &[
        Param {
            name: "mode",
            kind: Kind::Choice {
                default: MANUAL,
                choices: &MODES,
            },
        },
        Param {
            name: "black",
            kind: Kind::Float {
                // darktable's -0.000244140625, exactly -2^-12.
                default: -1.0 / 4096.0,
                min: -1.0,
                max: 1.0,
            },
        },
        Param {
            name: "exposure",
            kind: Kind::Float {
                default: 0.7,
                min: -18.0,
                max: 18.0,
            },
        },
        Param {
            name: "deflicker_percentile",
            kind: Kind::Float {
                default: PERCENTILE,
                min: 0.0,
                max: 100.0,
            },
        },
        Param {
            name: "deflicker_target_level",
            kind: Kind::Float {
                default: TARGET_LEVEL,
                min: -18.0,
                max: 18.0,
            },
        },
        Param {
            name: "compensate_exposure_bias",
            kind: Kind::Bool { default: true },
        },
        Param {
            name: "compensate_hilite_pres",
            kind: Kind::Bool { default: true },
        },
    ],
};

/// The exposure automatic mode applies on `input`, as darktable's deflicker computes it
/// (`_compute_correction`): the one that puts the percentile of the raw's histogram at the
/// target level below rawprepare's white point. `None` in manual mode, and where the
/// image context holds no histogram, an image that is not a raw, where automatic mode
/// renders as manual mode.
pub fn automatic_exposure(
    params: &ParamValues,
    image: &ImageContext,
    input: &ImageDesc,
) -> Option<f32> {
    if params.choice("mode") != AUTOMATIC {
        return None;
    }
    let histogram = image.raw_histogram.as_ref()?;
    Some(histogram.exposure(
        params.float("deflicker_percentile"),
        params.float("deflicker_target_level"),
        input.raw_black,
        input.raw_white,
    ))
}

/// darktable `_get_exposure_bias`: the camera's exposure compensation, trusted only
/// within ±5 EV.
fn exposure_bias(image: &ImageContext) -> f32 {
    image
        .camera
        .as_ref()
        .map_or(0.0, |camera| camera.exposure_bias.clamp(-5.0, 5.0))
}

/// darktable `_get_highlight_bias`: the exposure a highlight-preserving mode held back,
/// as the maker notes give it, where it is above 0, trusted only within −1 to +4 EV; 0
/// otherwise.
pub fn highlight_bias(image: &ImageContext) -> f32 {
    image.camera.as_ref().map_or(0.0, |camera| {
        if camera.highlight_preservation > 0.0 {
            camera.highlight_preservation.clamp(-1.0, 4.0)
        } else {
            0.0
        }
    })
}

/// The exposure the kernel applies, as darktable's `commit_params` computes it: the
/// parameter, less the camera's exposure bias when it is compensated, plus the exposure
/// a highlight-preserving mode held back when that is compensated.
pub fn effective_exposure(params: &ParamValues, image: &ImageContext) -> f32 {
    let mut exposure = params.float("exposure");
    if params.flag("compensate_exposure_bias") {
        exposure -= exposure_bias(image);
    }
    if params.flag("compensate_hilite_pres") {
        exposure += highlight_bias(image);
    }
    exposure
}

impl Operation for Exposure {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults`: +0.7 EV, a slightly lowered black and the
    /// camera's exposure bias compensated for a colour raw; no exposure change for a
    /// monochrome raw, and none at all for an image that is not a raw. Highlight
    /// preservation is compensated on every image, as darktable does for a module's
    /// first instance; the image context records no held-back exposure but a raw's.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        match &image.camera {
            Some(camera) if camera.cfa.is_some() => {}
            Some(_) => {
                values.set("exposure", Value::Float(0.0));
                values.set("black", Value::Float(0.0));
            }
            None => {
                values.set("exposure", Value::Float(0.0));
                values.set("black", Value::Float(0.0));
                values.set("compensate_exposure_bias", Value::Bool(false));
            }
        }
        values
    }

    /// Automatic mode meters the raw's histogram.
    fn reads_raw_histogram(&self, params: &ParamValues) -> bool {
        params.choice("mode") == AUTOMATIC
    }

    /// [`COMPUTED_EXPOSURE`]: the exposure automatic mode applies, where it computes one.
    fn computed(
        &self,
        id: &str,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Option<f32> {
        (id == COMPUTED_EXPOSURE)
            .then(|| automatic_exposure(params, image, input))
            .flatten()
    }

    /// The black and the exposure: in automatic mode the one its histogram gives, which
    /// replaces the compensated exposure as darktable's `_process_common_setup` does, or
    /// without a histogram manual mode's; there the black still applies, put 0.01 below the
    /// white that exposure renders where it reaches it, the tool's rule, so the kernel's
    /// `2^−e − b` stays positive whatever the photo gives, the tool leaving it within ±1
    /// alone. In manual mode the tool holds it so itself.
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let exposure = automatic_exposure(params, image, input)
            .unwrap_or_else(|| effective_exposure(params, image));
        let black = params.float("black");
        // The tool's rule, darktable's margin (decision 30): a black that reaches the white
        // goes 0.01 below it, any other is kept.
        let white = (-exposure).exp2();
        let black = if params.choice("mode") == AUTOMATIC && black >= white {
            white - WHITE_MARGIN
        } else {
            black
        };
        // `ExposureParams` in the kernel: black, then exposure.
        let bytes = [black, exposure]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        Ok(Prepared::per_pixel(input, bytes, Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::{Camera, Pattern};

    use super::*;

    fn raw(cfa: bool, exposure_bias: f32) -> ImageContext {
        let pattern = |values: Vec<u32>| Pattern {
            width: 2,
            height: 2,
            values,
        };
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 8,
                height: 8,
                cfa: cfa.then(|| pattern(vec![0, 1, 1, 2])),
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: 1.0,
                white_balance: None,
                xyz_to_camera: None,
                crop: [0, 0, 8, 8],
                declared: [0, 0, 8, 8],
                exposure_bias,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                highlight_preservation: 0.0,
                orientation: 0,
            }),
            ..ImageContext::full(8, 8)
        }
    }

    /// The kernel's two parameters for these settings on this image.
    fn kernel_params(image: &ImageContext, values: &ParamValues) -> [f32; 2] {
        let params = SCHEMA
            .resolve_over(&Exposure.defaults(image), values)
            .unwrap();
        let input = ImageDesc::rgba(8, 8);
        let prepared = Exposure.prepare(&params, image, &input).unwrap();
        let bytes = &prepared.passes[0].params;
        [0, 4].map(|at| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()))
    }

    #[test]
    fn defaults_follow_darktable_by_image() {
        let none = ParamValues::new();
        assert_eq!(kernel_params(&raw(true, 0.0), &none), [-1.0 / 4096.0, 0.7]);
        assert_eq!(kernel_params(&raw(false, 0.0), &none), [0.0, 0.0]);
        assert_eq!(kernel_params(&ImageContext::full(8, 8), &none), [0.0, 0.0]);
    }

    #[test]
    fn the_camera_exposure_bias_is_compensated_within_5_ev() {
        let none = ParamValues::new();
        assert_eq!(kernel_params(&raw(true, 1.0), &none)[1], 0.7 - 1.0);
        assert_eq!(kernel_params(&raw(true, -8.0), &none)[1], 0.7 + 5.0);
        let mut off = ParamValues::new();
        off.set("compensate_exposure_bias", Value::Bool(false));
        assert_eq!(kernel_params(&raw(true, 1.0), &off)[1], 0.7);
    }

    /// A colour raw whose highlight-preserving mode held back `held` EV, with this bias.
    fn preserving(exposure_bias: f32, held: f32) -> ImageContext {
        let mut image = raw(true, exposure_bias);
        image.camera.as_mut().unwrap().highlight_preservation = held;
        image
    }

    #[test]
    fn the_held_back_exposure_is_given_back_within_darktable_s_bounds() {
        let none = ParamValues::new();
        // On by default, as darktable's first instance has it.
        assert!(SCHEMA.defaults().flag("compensate_hilite_pres"));
        assert_eq!(kernel_params(&preserving(0.0, 0.5), &none)[1], 0.7 + 0.5);
        // After the camera's bias, as darktable's `commit_params` adds it.
        assert_eq!(
            kernel_params(&preserving(-2.0 / 3.0, 0.5), &none)[1],
            0.7 - (-2.0f32 / 3.0) + 0.5
        );
        // Above +4 EV it is +4; at 0 or below, none, as `_get_highlight_bias` trusts it.
        assert_eq!(kernel_params(&preserving(0.0, 6.0), &none)[1], 0.7 + 4.0);
        assert_eq!(kernel_params(&preserving(0.0, -0.5), &none)[1], 0.7);
        let mut off = ParamValues::new();
        off.set("compensate_hilite_pres", Value::Bool(false));
        assert_eq!(kernel_params(&preserving(0.0, 2.0), &off)[1], 0.7);
        // An image that is not a raw holds none.
        assert_eq!(highlight_bias(&ImageContext::full(8, 8)), 0.0);
    }

    #[test]
    fn parameters_written_at_their_defaults_render_as_none_are() {
        let image = raw(true, -0.67);
        let mut defaults = ParamValues::new();
        defaults.set("mode", Value::Choice(MANUAL));
        defaults.set("deflicker_percentile", Value::Float(PERCENTILE));
        defaults.set("deflicker_target_level", Value::Float(TARGET_LEVEL));
        defaults.set("compensate_hilite_pres", Value::Bool(true));
        assert_eq!(
            kernel_params(&image, &defaults),
            kernel_params(&image, &ParamValues::new())
        );
    }

    /// A colour raw with a camera bias of −1 EV and 0.5 EV held back whose histogram
    /// holds `count` photosites at each of `values`, and the input rawprepare leaves with
    /// black 512 and white 16896, 14 bits above it.
    fn metered(values: &[(usize, u32)]) -> (ImageContext, ImageDesc) {
        let mut image = preserving(-1.0, 0.5);
        let mut counts = vec![0; apertrust_plugin::RAW_HISTOGRAM_BINS];
        for &(value, count) in values {
            counts[value] += count;
        }
        image.raw_histogram = apertrust_plugin::RawHistogram::new(counts);
        let input = ImageDesc {
            raw_black: 512,
            raw_white: 512 + 16384,
            ..ImageDesc::rgba(8, 8)
        };
        (image, input)
    }

    /// The kernel's two parameters for `values` over the defaults on `input`.
    fn prepared(image: &ImageContext, input: &ImageDesc, values: &ParamValues) -> [f32; 2] {
        let params = SCHEMA
            .resolve_over(&Exposure.defaults(image), values)
            .unwrap();
        let prepared = Exposure.prepare(&params, image, input).unwrap();
        let bytes = &prepared.passes[0].params;
        [0, 4].map(|at| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()))
    }

    fn automatic(percentile: f32, target: f32) -> ParamValues {
        let mut values = ParamValues::new();
        values.set("mode", Value::Choice(AUTOMATIC));
        values.set("deflicker_percentile", Value::Float(percentile));
        values.set("deflicker_target_level", Value::Float(target));
        values
    }

    #[test]
    fn automatic_mode_puts_the_percentile_at_the_target_level() {
        // Half the photosites an eighth of the range above the black, −3 EV, half a
        // quarter, −2 EV.
        let (image, input) = metered(&[(512 + 2048, 50), (512 + 4096, 50)]);
        // The median is the first value reaching half, −3 EV, so −4 needs −1 EV. The
        // camera's bias and the held-back exposure are not added, as darktable replaces
        // the compensated exposure; the black still applies.
        assert_eq!(
            prepared(&image, &input, &automatic(50.0, -4.0)),
            [-1.0 / 4096.0, -1.0]
        );
        assert_eq!(prepared(&image, &input, &automatic(90.0, -4.0))[1], -2.0);
        assert_eq!(prepared(&image, &input, &automatic(90.0, -1.5))[1], 0.5);
        // Exposure itself changes nothing in automatic mode.
        let mut values = automatic(50.0, -4.0);
        values.set("exposure", Value::Float(3.0));
        assert_eq!(prepared(&image, &input, &values)[1], -1.0);
        // rawprepare's levels, as its output carries them, convert the value.
        let lifted = ImageDesc {
            raw_black: 512 + 1024,
            ..input
        };
        let ev = (2048f64 - 1024.0).log2() - (16384f64 - 1024.0).log2();
        assert_eq!(
            prepared(&image, &lifted, &automatic(50.0, -4.0))[1],
            (-4.0 - ev) as f32
        );
    }

    #[test]
    fn automatic_mode_holds_the_black_below_the_white_it_renders() {
        // Every photosite at 2⁻¹⁰ of the range: the median at −10 EV, so −4 needs +6 EV,
        // a white of 2⁻⁶.
        let (image, input) = metered(&[(512 + 16, 100)]);
        // Black level correction's +0.1 over the photo's black, which Exposure's +0.7 EV
        // keeps below its white of 0.616, reaches this one, so it goes 0.01 below it.
        let mut values = automatic(50.0, -4.0);
        values.set("black", Value::Float(0.1));
        let [black, exposure] = prepared(&image, &input, &values);
        assert_eq!(exposure, 6.0);
        assert_eq!(black, 2f32.powi(-6) - WHITE_MARGIN);
        assert!((-exposure).exp2() - black > 0.0);
        // A black already below it is kept, even within 0.01 of it: the margin is only
        // for one that reaches it, as the tool's.
        values.set("black", Value::Float(-0.001));
        assert_eq!(prepared(&image, &input, &values)[0], -0.001);
        values.set("black", Value::Float(2f32.powi(-6) - 0.005));
        assert_eq!(prepared(&image, &input, &values)[0], 2f32.powi(-6) - 0.005);
        // At the white exactly, and above it: 0.01 below.
        for black in [2f32.powi(-6), 0.5] {
            values.set("black", Value::Float(black));
            assert_eq!(
                prepared(&image, &input, &values)[0],
                2f32.powi(-6) - WHITE_MARGIN
            );
        }
        // Black 0 at +10 EV, a white of 2⁻¹⁰: kept, the scale 1024.
        let (image, input) = metered(&[(512 + 1, 100)]);
        let mut values = automatic(50.0, -4.0);
        values.set("black", Value::Float(0.0));
        assert_eq!(prepared(&image, &input, &values), [0.0, 10.0]);
        // Where the photo computes a lower exposure than manual mode's, a white above the
        // black manual mode's +8 EV would have pushed down: the black as set, unlowered.
        // Every photosite at a quarter of the range, −2 EV, so −3 needs −1 EV, a white of 2.
        let (image, input) = metered(&[(512 + 4096, 100)]);
        let mut values = automatic(50.0, -3.0);
        values.set("exposure", Value::Float(8.0));
        values.set("black", Value::Float(-1.0 / 4096.0 + 0.01));
        assert_eq!(
            prepared(&image, &input, &values),
            [-1.0 / 4096.0 + 0.01, -1.0]
        );
        // Without a histogram, at manual mode's +8 EV and its white, a black that reaches
        // it is held there.
        let mut plain = image.clone();
        plain.raw_histogram = None;
        let [black, exposure] = prepared(&plain, &input, &values);
        assert_eq!(black, (-exposure).exp2() - WHITE_MARGIN);
        // Manual mode's rule there: exposure 7, both compensations off, black 0 stays 0,
        // below the white of 2⁻⁷; 0.01, above it, goes 0.01 below it.
        values.set("exposure", Value::Float(7.0));
        values.set("compensate_exposure_bias", Value::Bool(false));
        values.set("compensate_hilite_pres", Value::Bool(false));
        values.set("black", Value::Float(0.0));
        assert_eq!(prepared(&plain, &input, &values), [0.0, 7.0]);
        values.set("black", Value::Float(0.01));
        assert_eq!(
            prepared(&plain, &input, &values),
            [2f32.powi(-7) - WHITE_MARGIN, 7.0]
        );
    }

    #[test]
    fn automatic_mode_without_a_histogram_renders_as_manual_mode() {
        let (mut image, input) = metered(&[(600, 1)]);
        image.raw_histogram = None;
        let manual = prepared(&image, &input, &ParamValues::new());
        assert_eq!(prepared(&image, &input, &automatic(30.0, -2.0)), manual);
        // An image that is not a raw has none.
        let plain = ImageContext::full(8, 8);
        assert_eq!(
            prepared(&plain, &input, &automatic(30.0, -2.0)),
            prepared(&plain, &input, &ParamValues::new())
        );
    }

    #[test]
    fn only_automatic_mode_reads_the_histogram_and_computes_an_exposure() {
        let (image, input) = metered(&[(512 + 1000, 10)]);
        let resolve = |values: &ParamValues| {
            SCHEMA
                .resolve_over(&Exposure.defaults(&image), values)
                .unwrap()
        };
        let manual = resolve(&ParamValues::new());
        assert!(!Exposure.reads_raw_histogram(&manual));
        let none = Exposure.computed(COMPUTED_EXPOSURE, &manual, &image, &input);
        assert_eq!(none, None);
        let auto = resolve(&automatic(50.0, -4.0));
        assert!(Exposure.reads_raw_histogram(&auto));
        // The readout is what the kernel gets.
        assert_eq!(
            Exposure.computed(COMPUTED_EXPOSURE, &auto, &image, &input),
            Some(prepared(&image, &input, &automatic(50.0, -4.0))[1])
        );
        assert_eq!(Exposure.computed("other", &auto, &image, &input), None);
    }

    #[test]
    fn the_effective_exposure_is_what_the_kernel_gets() {
        for (image, set, held) in [
            (raw(true, 1.0), None, None),
            (raw(true, -8.0), None, None),
            (raw(true, 1.0), Some(false), None),
            (preserving(-1.0, 0.66), None, None),
            (preserving(-1.0, 0.66), None, Some(false)),
            (ImageContext::full(8, 8), None, None),
        ] {
            let mut values = ParamValues::new();
            if let Some(flag) = set {
                values.set("compensate_exposure_bias", Value::Bool(flag));
            }
            if let Some(flag) = held {
                values.set("compensate_hilite_pres", Value::Bool(flag));
            }
            let params = SCHEMA
                .resolve_over(&Exposure.defaults(&image), &values)
                .unwrap();
            assert_eq!(
                effective_exposure(&params, &image),
                kernel_params(&image, &values)[1]
            );
        }
    }
}
