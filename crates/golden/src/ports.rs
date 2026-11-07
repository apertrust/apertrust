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

//! darktable parameters translated into the inputs of Apertrust's kernels.
//!
//! Each function reads a module's params struct from the case's sidecar and returns
//! what the kernel needs besides the image: its uniform block after the pass header, and
//! its read-only tables, as little-endian bytes.

use anyhow::{Context, Result, bail, ensure};
use apertrust_color::{RgbNorm, RgbSpace, WorkingProfile};
use apertrust_operations::agx::{self, BasePrimaries};
use apertrust_operations::bilat;
use apertrust_operations::bloom;
use apertrust_operations::channelmixerrgb;
use apertrust_operations::colorbalancergb::{self, SaturationFormula, Settings};
use apertrust_operations::colorequal;
use apertrust_operations::filmicrgb;
use apertrust_operations::grain;
use apertrust_operations::hazeremoval;
use apertrust_operations::primaries;
use apertrust_operations::rgbcurve::{self, Interpolation, Mode, Norm, Slot, SlotKind};
use apertrust_operations::rgblevels::{self, Point};
use apertrust_operations::sigmoid::{self, ColorProcessing};
use apertrust_operations::soften;
use apertrust_operations::toneequal;
use apertrust_operations::vignette::{self, Parameter};
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues};

use crate::cases::Case;
use crate::xmp::module_params;

#[derive(Debug, Clone, PartialEq)]
pub struct KernelInputs {
    pub params: Vec<u8>,
    pub tables: Vec<Vec<u8>>,
}

/// The kernel inputs for the case's module, or `None` if it has no port yet.
pub fn kernel_inputs(case: &Case) -> Result<Option<KernelInputs>> {
    match case.operation {
        "exposure" => exposure(case.xmp).map(Some),
        "colorbalancergb" => color_balance_inputs(&color_balance_settings(case.xmp)?).map(Some),
        "agx" => Ok(Some(agx_inputs(&agx_prepare(case.xmp)?))),
        "sigmoid" => Ok(Some(sigmoid_inputs(&sigmoid_prepare(case.xmp)?))),
        "rgbcurve" => Ok(Some(rgbcurve_inputs(&rgbcurve_prepare(case)?))),
        "rgblevels" => Ok(Some(rgblevels_inputs(&rgblevels_prepare(case)?))),
        "vignette" => Ok(Some(vignette_inputs(&vignette_prepare(case)?))),
        "grain" => Ok(Some(grain_inputs(&grain_prepare(case)?))),
        "channelmixerrgb" => Ok(Some(channelmixerrgb_inputs(&channelmixerrgb_prepare(
            case,
        )?))),
        "primaries" => primaries_inputs(&primaries_settings(case.xmp)?).map(Some),
        _ => Ok(None),
    }
}

/// The schema values of a case whose operation runs as a one-instance engine plan
/// (decision 15): the tone equalizer's, whose mask filters take whole-image passes and
/// scratch buffers, local contrast's, whose pyramid and grid do, haze removal's, whose
/// ambient light, depth and transmission do, bloom's and soften's, whose box means do, and
/// filmic rgb's, whose highlight reconstruction does. `None` for the cases the harness
/// dispatches as one kernel.
pub fn plan_values(case: &Case) -> Result<Option<ParamValues>> {
    match case.operation {
        "toneequal" => Ok(Some(toneequal::values(&toneequal_settings(case.xmp)?))),
        "bilat" => Ok(Some(bilat::values(&bilat_settings(case.xmp)?))),
        "hazeremoval" => Ok(Some(hazeremoval::values(&hazeremoval_settings(case.xmp)?))),
        "bloom" => Ok(Some(bloom::values(&bloom_settings(case.xmp)?))),
        "filmicrgb" => Ok(Some(filmicrgb::values(&filmicrgb_settings(case.xmp)?))),
        "colorequal" => Ok(Some(colorequal::values(&colorequal_settings(case.xmp)?))),
        "soften" => Ok(Some(soften::values(&soften_settings(case.xmp)?))),
        "flip" => Ok(Some(flip_values(case.xmp)?)),
        _ => Ok(None),
    }
}

/// A grain, vignetting, tone equalizer, haze removal, colour equalizer, colour balance
/// rgb, local contrast, bloom or soften case's module as the operation's parameter
/// values, from its sidecar, for the whole pipeline (decisions 15 and 56, and the
/// timing of `apertrust-golden export`), which adds the module to both sides; any other
/// case is refused. Grain's settings leave out its offset, which is
/// no parameter: the harness gives the operation darktable's for the render's width.
pub fn pipeline_values(case: &Case) -> Result<ParamValues> {
    use apertrust_plugin::Value;
    let mut values = ParamValues::new();
    match case.operation {
        "grain" => {
            let settings = grain_settings(case)?;
            for (name, value) in [
                ("scale", settings.scale),
                ("strength", settings.strength),
                ("midtones_bias", settings.midtones_bias),
            ] {
                values.set(name, Value::Float(value));
            }
        }
        "vignette" => {
            let settings = vignette_settings(case.xmp)?;
            ensure!(
                settings.dithering == vignette::Dithering::Off,
                "{}: the operation does not port dithering",
                case.name
            );
            for (name, value) in [
                ("scale", settings.scale),
                ("falloff_scale", settings.falloff_scale),
                ("brightness", settings.brightness),
                ("saturation", settings.saturation),
                ("center_x", settings.center[0]),
                ("center_y", settings.center[1]),
                ("whratio", settings.whratio),
                ("shape", settings.shape),
            ] {
                values.set(name, Value::Float(value));
            }
            values.set("autoratio", Value::Bool(settings.autoratio));
            values.set("unbound", Value::Bool(settings.unbound));
            values.set("dithering", Value::Choice(0));
        }
        "toneequal" | "hazeremoval" | "colorequal" | "bloom" | "soften" | "bilat" => {
            return plan_values(case)?.context("a plan case has its values");
        }
        "colorbalancergb" => {
            return Ok(colorbalancergb::values(&color_balance_settings(case.xmp)?));
        }
        other => bail!("the whole pipeline does not add {other} cases"),
    }
    Ok(values)
}

/// The image context of a case's plan over its input, `width` × `height`: for the tone
/// equalizer with its raw's camera metadata from rawler, since it measures its radius on
/// the raw's readout, as darktable's `piece->iwidth` is the pipe's input, the raw before
/// its crop; haze removal reads only the pipe's scale, 1 here.
pub fn plan_context(case: &Case, width: u32, height: u32) -> Result<ImageContext> {
    if case.operation == "hazeremoval" {
        return Ok(ImageContext::full(width, height));
    }
    let path = crate::render::images_dir()?.join(case.image);
    let raw = apertrust_raw::decode_file(&path)
        .with_context(|| format!("decoding {}", path.display()))?;
    Ok(ImageContext {
        camera: apertrust_engine::raw::image_context(&raw).camera,
        ..ImageContext::full(width, height)
    })
}

/// [`plan_values`] with gradient parameter `index` moved by `delta`, for finite
/// differences, in the units its gradient is taken in.
pub fn perturbed_values(case: &Case, index: usize, delta: f32) -> Result<ParamValues> {
    match case.operation {
        "toneequal" => toneequal_perturbed(case.xmp, index, delta),
        "bilat" => bilat_perturbed(case.xmp, index, delta),
        "hazeremoval" => {
            // The schema's units, the kernel's.
            let mut settings = hazeremoval_settings(case.xmp)?;
            match index {
                0 => settings.strength += delta,
                1 => settings.distance += delta,
                _ => bail!("no hazeremoval gradient parameter {index}"),
            }
            Ok(hazeremoval::values(&settings))
        }
        "bloom" => bloom_perturbed(case.xmp, index, delta),
        "filmicrgb" => filmicrgb_perturbed(case.xmp, index, delta),
        "colorequal" => colorequal_perturbed(case.xmp, index, delta),
        "soften" => {
            // The schema's units: the amount in percent, which the kernel's derivative
            // entry points report.
            let mut settings = soften_settings(case.xmp)?;
            match index {
                0 => settings.amount += delta,
                _ => bail!("no soften gradient parameter {index}"),
            }
            Ok(soften::values(&settings))
        }
        operation => bail!("{operation} does not run as a plan"),
    }
}

/// A case's own step for the gradient check's finite differences, where the harness's
/// 1e-3 is too small: the tone equalizer's filtered cases, whose smallest totals the f32
/// rounding of two renders outweighs at 1e-3 EV, and which agree within 1.3e-3 at 3e-2 EV
/// (docs/ports/toneequal.md, Gradients).
pub fn gradient_step(case: &Case) -> Option<f32> {
    match case.operation {
        "toneequal" => toneequal_settings(case.xmp)
            .ok()
            .filter(|settings| settings.details != toneequal::Filter::None)
            .map(|_| 3e-2),
        // The curves' nodes move each pixel's hue, saturation or brightness by about 1e-4
        // of a pixel's value per unit, which f32 renders resolve from a step of 1e-2
        // (docs/ports/colorequal.md, Gradients).
        "colorequal" => Some(1e-2),
        _ => None,
    }
}

/// The parameters a module's gradient entry point differentiates against, in the order
/// of its gradient output (see each Slang kernel's `<module>_backward`).
pub fn gradient_parameters(case: &Case) -> &'static [&'static str] {
    match case.operation {
        "hazeremoval" => &["strength", "distance"],
        "exposure" => &["black", "exposure"],
        "colorbalancergb" => &[
            "chroma_global",
            "contrast",
            "saturation_global",
            "brilliance_global",
        ],
        "agx" => &["look_slope", "look_power", "look_saturation", "hue_mix"],
        "sigmoid" => sigmoid_gradient(case),
        "rgbcurve" => rgbcurve_gradient(case).map_or(&[], |gradient| &gradient.names),
        "rgblevels" => rgblevels_gradient(case).map_or(&[], |gradient| gradient.names),
        "vignette" => vignette_gradient(case).names,
        "grain" => grain_gradient(case).names,
        "channelmixerrgb" => channelmixerrgb_gradient(case),
        "toneequal" => &TONEEQUAL_GRADIENT,
        "primaries" => &PRIMARIES_GRADIENT,
        "bilat" => bilat_gradient(case),
        "bloom" => &BLOOM_GRADIENT,
        "filmicrgb" => filmicrgb_gradient(case),
        "colorequal" => colorequal_gradient(case),
        "soften" => &SOFTEN_GRADIENT,
        _ => &[],
    }
}

/// Why the gradient check's loss, the sum of the output's RGB, cannot see this gradient
/// parameter on the synthetic sweep, when it cannot: `gradient --synthetic` then skips it
/// and says so. The case's own image still checks it, and the port's GPU tests check it
/// per pixel, as for vignette's saturation, which no case names (the golden-tests skill).
pub fn sweep_blind(case: &Case, parameter: &str) -> Option<&'static str> {
    match case.operation {
        // A primary's hue or purity, not the tint's (docs/ports/primaries.md, Gradients).
        "primaries" if !parameter.starts_with("achromatic_tint") => Some(
            "grey maps to the white whatever the primaries, and the sweep is grey on average, \
             so the loss's derivative cancels",
        ),
        _ => None,
    }
}

/// The kernel inputs with gradient parameter `index` moved by `delta`, for finite
/// differences. Each parameter is moved in the units its gradient is taken in.
pub fn perturbed_inputs(case: &Case, index: usize, delta: f32) -> Result<KernelInputs> {
    match case.operation {
        "exposure" => {
            // The params are (black, exposure) as f32, in gradient order.
            let mut inputs = exposure(case.xmp)?;
            let at = index * 4;
            ensure!(
                at + 4 <= inputs.params.len(),
                "no exposure parameter {index}"
            );
            let value = f32::from_le_bytes(inputs.params[at..at + 4].try_into()?) + delta;
            inputs.params[at..at + 4].copy_from_slice(&value.to_le_bytes());
            Ok(inputs)
        }
        "colorbalancergb" => {
            let mut settings = color_balance_settings(case.xmp)?;
            match index {
                0 => settings.chroma_global += delta,
                // The kernel gets 1 + contrast, so its derivative equals the setting's.
                1 => settings.contrast += delta,
                2 => settings.saturation_global += delta,
                3 => settings.brilliance_global += delta,
                _ => bail!("no colorbalancergb gradient parameter {index}"),
            }
            color_balance_inputs(&settings)
        }
        "agx" => {
            // Moved in the kernel's own units: the power, not darktable's brightness.
            let mut prepared = agx_prepare(case.xmp)?;
            let look = &mut prepared.look;
            match index {
                0 => look.slope += delta,
                1 => look.power += delta,
                2 => look.saturation += delta,
                3 => look.hue_mix += delta,
                _ => bail!("no agx gradient parameter {index}"),
            }
            Ok(agx_inputs(&prepared))
        }
        "sigmoid" => {
            // Moved in the kernel's own units: the powers the setup fits to darktable's
            // contrast and skew, the target white and black as fractions, the hue
            // preservation as a fraction.
            let mut prepared = sigmoid_prepare(case.xmp)?;
            let curve = &mut prepared.curve;
            match (index, prepared.color_processing) {
                (0, _) => curve.film_power += delta,
                (1, _) => curve.paper_power += delta,
                (2, _) => curve.white_target += delta,
                (3, ColorProcessing::PerChannel) => prepared.hue_preservation += delta,
                (3, ColorProcessing::RgbRatio) => curve.black_target += delta,
                _ => bail!("no sigmoid gradient parameter {index}"),
            }
            Ok(sigmoid_inputs(&prepared))
        }
        "rgbcurve" => {
            // Moved in the kernel's own units: a knot's linear x, y or tangent, or a
            // curve's end value or power, leaving the others as the setup made them.
            let mut prepared = rgbcurve_prepare(case)?;
            let slot = *prepared
                .gradient
                .get(index)
                .with_context(|| format!("no rgbcurve gradient parameter {index}"))?;
            *prepared.value_mut(slot) += delta;
            Ok(rgbcurve_inputs(&prepared))
        }
        "rgblevels" => {
            // The kernel takes the levels themselves, darktable's units.
            let mut prepared = rgblevels_prepare(case)?;
            let slot = *prepared
                .gradient
                .get(index)
                .with_context(|| format!("no rgblevels gradient parameter {index}"))?;
            *prepared.value_mut(slot) += delta;
            Ok(rgblevels_inputs(&prepared))
        }
        "vignette" => {
            // The kernel takes darktable's parameters, in darktable's units.
            let mut prepared = vignette_prepare(case)?;
            let parameter = *prepared
                .gradient
                .get(index)
                .with_context(|| format!("no vignette gradient parameter {index}"))?;
            *prepared.value_mut(parameter) += delta;
            Ok(vignette_inputs(&prepared))
        }
        "grain" => {
            // The kernel takes darktable's parameters, in darktable's units; the noise's
            // offsets stay where the setup put them, so the scale's derivative is the
            // grain's at a fixed offset (docs/ports/grain.md, Gradients).
            let mut prepared = grain_prepare(case)?;
            let parameter = *prepared
                .gradient
                .get(index)
                .with_context(|| format!("no grain gradient parameter {index}"))?;
            *prepared.value_mut(parameter) += delta;
            Ok(grain_inputs(&prepared))
        }
        "channelmixerrgb" => {
            // The kernel takes the illuminant's chromaticity, which every illuminant
            // resolves to on the CPU.
            let mut prepared = channelmixerrgb_prepare(case)?;
            match index {
                0 => prepared.x += delta,
                1 => prepared.y += delta,
                _ => bail!("no channelmixerrgb gradient parameter {index}"),
            }
            Ok(channelmixerrgb_inputs(&prepared))
        }
        "primaries" => {
            // The kernel takes darktable's settings, in darktable's units; the matrix
            // `main` applies is built again from them, in f64.
            let mut settings = primaries_settings(case.xmp)?;
            let parameter = *primaries::DEFAULT_GRADIENT
                .get(index)
                .with_context(|| format!("no primaries gradient parameter {index}"))?;
            *settings.get_mut(parameter) += delta;
            primaries_inputs(&settings)
        }
        operation => bail!("{operation} has no gradient port"),
    }
}

fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// darktable `dt_iop_exposure_params_t`, versions 5 and 6: mode (i32), black, exposure,
/// deflicker percentile and target level (f32), then in version 6 a flag to compensate
/// the camera's exposure bias (i32).
fn exposure(xmp: &str) -> Result<KernelInputs> {
    let params = module_params(xmp, "exposure")?;
    let expected = match params.version {
        5 => 20,
        6 => 24,
        v => bail!("exposure params version {v} is not supported"),
    };
    ensure!(
        params.bytes.len() == expected,
        "exposure v{} params are {} bytes, expected {expected}",
        params.version,
        params.bytes.len()
    );
    let word = words(&params.bytes);
    ensure!(
        i32::from_le_bytes(word[0]) == 0,
        "exposure deflicker mode is not ported"
    );
    if params.version == 6 && i32::from_le_bytes(word[5]) != 0 {
        bail!("exposure bias compensation needs EXIF data and is not ported");
    }
    // Kernel order: black, exposure.
    Ok(KernelInputs {
        params: [word[1], word[2]].concat(),
        tables: Vec::new(),
    })
}

/// darktable `dt_iop_colorbalancergb_params_t`, versions 1 to 5. Each version appends
/// fields; darktable's upgrade fills the missing ones as below, picks the JzAzBz
/// saturation formula for every version before 5, and divides version 1's global
/// saturation by 100.
fn color_balance_settings(xmp: &str) -> Result<Settings> {
    let params = module_params(xmp, "colorbalancergb")?;
    let expected_words = match params.version {
        1 => 24,
        2 => 28,
        3 => 29,
        4 => 32,
        5 => 33,
        v => bail!("colorbalancergb params version {v} is not supported"),
    };
    let word = words(&params.bytes);
    ensure!(
        word.len() == expected_words && params.bytes.len() % 4 == 0,
        "colorbalancergb v{} params are {} bytes, expected {}",
        params.version,
        params.bytes.len(),
        expected_words * 4
    );
    let f = |i: usize, default: f32| word.get(i).map_or(default, |w| f32::from_le_bytes(*w));

    Ok(Settings {
        shadows_y: f(0, 0.0),
        shadows_c: f(1, 0.0),
        shadows_h: f(2, 0.0),
        midtones_y: f(3, 0.0),
        midtones_c: f(4, 0.0),
        midtones_h: f(5, 0.0),
        highlights_y: f(6, 0.0),
        highlights_c: f(7, 0.0),
        highlights_h: f(8, 0.0),
        global_y: f(9, 0.0),
        global_c: f(10, 0.0),
        global_h: f(11, 0.0),
        shadows_weight: f(12, 1.0),
        white_fulcrum: f(13, 0.0),
        highlights_weight: f(14, 1.0),
        chroma_shadows: f(15, 0.0),
        chroma_highlights: f(16, 0.0),
        chroma_global: f(17, 0.0),
        chroma_midtones: f(18, 0.0),
        saturation_global: if params.version == 1 {
            f(19, 0.0) / 100.0
        } else {
            f(19, 0.0)
        },
        saturation_highlights: f(20, 0.0),
        saturation_midtones: f(21, 0.0),
        saturation_shadows: f(22, 0.0),
        hue_angle: f(23, 0.0),
        brilliance_global: f(24, 0.0),
        brilliance_highlights: f(25, 0.0),
        brilliance_midtones: f(26, 0.0),
        brilliance_shadows: f(27, 0.0),
        mask_grey_fulcrum: f(28, 0.1845),
        vibrance: f(29, 0.0),
        grey_fulcrum: f(30, 0.1845),
        contrast: f(31, 0.0),
        saturation_formula: match word.get(32).map(|w| i32::from_le_bytes(*w)) {
            None | Some(0) => SaturationFormula::JzAzBz,
            Some(1) => SaturationFormula::Ucs,
            Some(v) => bail!("unknown colorbalancergb saturation formula {v}"),
        },
    })
}

/// darktable `dt_iop_agx_params_t`, version 7: 36 four-byte fields. Versions 1 to 6
/// were unreleased, and darktable's `legacy_params` replaces them with the
/// scene-referred defaults whatever they hold. darktable cannot read version 8.
fn agx_settings(xmp: &str) -> Result<agx::Settings> {
    let params = module_params(xmp, "agx")?;
    match params.version {
        1..=6 => return Ok(agx::Settings::scene_referred_default()),
        7 => {}
        v => bail!("agx params version {v} is not supported: the reference release reads up to 7"),
    }
    agx_settings_v7(&params.bytes)
}

fn agx_settings_v7(bytes: &[u8]) -> Result<agx::Settings> {
    ensure!(
        bytes.len() == 144,
        "agx v7 params are {} bytes, expected 144",
        bytes.len()
    );
    let word = words(bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    let flag = |i: usize| match int(i) {
        0 => Ok(false),
        1 => Ok(true),
        v => bail!("agx boolean field {i} is {v}"),
    };
    let base = int(19);
    Ok(agx::Settings {
        look_lift: f(0),
        look_slope: f(1),
        look_brightness: f(2),
        look_saturation: f(3),
        look_original_hue_mix_ratio: f(4),
        range_black_relative_ev: f(5),
        range_white_relative_ev: f(6),
        // Field 7, dynamic range scaling, only drives darktable's pickers.
        curve_pivot_x: f(8),
        curve_pivot_y_linear_output: f(9),
        curve_contrast_around_pivot: f(10),
        curve_linear_ratio_below_pivot: f(11),
        curve_linear_ratio_above_pivot: f(12),
        curve_toe_power: f(13),
        curve_shoulder_power: f(14),
        curve_gamma: f(15),
        auto_gamma: flag(16)?,
        curve_target_display_black_ratio: f(17),
        curve_target_display_white_ratio: f(18),
        base_primaries: BasePrimaries::from_darktable(base)
            .with_context(|| format!("unknown agx base primaries {base}"))?,
        disable_primaries_adjustments: flag(20)?,
        inset: [f(21), f(23), f(25)],
        rotation: [f(22), f(24), f(26)],
        master_outset_ratio: f(27),
        master_unrotation_ratio: f(28),
        outset: [f(29), f(31), f(33)],
        unrotation: [f(30), f(32), f(34)],
        completely_reverse_primaries: flag(35)?,
    })
}

/// darktable's default `dynamic_range_scaling`, AgX v7's field 7.
const AGX_DYNAMIC_RANGE_SCALING: f32 = 0.1;

/// darktable `dt_iop_agx_params_t` version 7 for these settings, the inverse of
/// `agx_settings_v7`. Field 7, dynamic range scaling, gets darktable's default, since
/// only darktable's pickers read it.
pub fn agx_params_v7(settings: &agx::Settings) -> Vec<u8> {
    // Every field named, so a new one cannot be left out.
    let agx::Settings {
        look_lift,
        look_slope,
        look_brightness,
        look_saturation,
        look_original_hue_mix_ratio,
        range_black_relative_ev,
        range_white_relative_ev,
        curve_pivot_x,
        curve_pivot_y_linear_output,
        curve_contrast_around_pivot,
        curve_linear_ratio_below_pivot,
        curve_linear_ratio_above_pivot,
        curve_toe_power,
        curve_shoulder_power,
        curve_gamma,
        auto_gamma,
        curve_target_display_black_ratio,
        curve_target_display_white_ratio,
        base_primaries,
        disable_primaries_adjustments,
        inset,
        rotation,
        master_outset_ratio,
        master_unrotation_ratio,
        outset,
        unrotation,
        completely_reverse_primaries,
    } = *settings;
    let f = f32::to_le_bytes;
    let flag = |value: bool| i32::from(value).to_le_bytes();
    [
        f(look_lift),
        f(look_slope),
        f(look_brightness),
        f(look_saturation),
        f(look_original_hue_mix_ratio),
        f(range_black_relative_ev),
        f(range_white_relative_ev),
        f(AGX_DYNAMIC_RANGE_SCALING),
        f(curve_pivot_x),
        f(curve_pivot_y_linear_output),
        f(curve_contrast_around_pivot),
        f(curve_linear_ratio_below_pivot),
        f(curve_linear_ratio_above_pivot),
        f(curve_toe_power),
        f(curve_shoulder_power),
        f(curve_gamma),
        flag(auto_gamma),
        f(curve_target_display_black_ratio),
        f(curve_target_display_white_ratio),
        base_primaries.to_darktable().to_le_bytes(),
        flag(disable_primaries_adjustments),
        f(inset[0]),
        f(rotation[0]),
        f(inset[1]),
        f(rotation[1]),
        f(inset[2]),
        f(rotation[2]),
        f(master_outset_ratio),
        f(master_unrotation_ratio),
        f(outset[0]),
        f(unrotation[0]),
        f(outset[1]),
        f(unrotation[1]),
        f(outset[2]),
        f(unrotation[2]),
        flag(completely_reverse_primaries),
    ]
    .concat()
}

fn agx_prepare(xmp: &str) -> Result<agx::Prepared> {
    // The references are rendered in darktable's default working space.
    Ok(agx::prepare(
        &agx_settings(xmp)?,
        &RgbSpace::LINEAR_REC2020,
    )?)
}

fn agx_inputs(prepared: &agx::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: Vec::new(),
    }
}

/// darktable `dt_iop_sigmoid_params_t`, versions 1 to 3, four-byte fields: version 1
/// has the curve, the colour processing and the hue preservation (6 fields), version 2
/// adds the insets, rotations and purity (13), version 3 the base primaries (14).
/// darktable's `legacy_params` copies an older version into zeroed version 3 params, so
/// the primaries are the working profile's, unmodified. darktable cannot read
/// version 4.
fn sigmoid_settings(xmp: &str) -> Result<sigmoid::Settings> {
    let params = module_params(xmp, "sigmoid")?;
    let fields = match params.version {
        1 => 6,
        2 => 13,
        3 => 14,
        v => bail!(
            "sigmoid params version {v} is not supported: the reference release reads up to 3"
        ),
    };
    ensure!(
        params.bytes.len() == 4 * fields,
        "sigmoid v{} params are {} bytes, expected {}",
        params.version,
        params.bytes.len(),
        4 * fields
    );
    let mut word = words(&params.bytes);
    word.resize(14, [0; 4]);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    Ok(sigmoid::Settings {
        middle_grey_contrast: f(0),
        contrast_skewness: f(1),
        display_white_target: f(2),
        display_black_target: f(3),
        color_processing: ColorProcessing::from_darktable(int(4))
            .with_context(|| format!("unknown sigmoid colour processing {}", int(4)))?,
        hue_preservation: f(5),
        inset: [f(6), f(8), f(10)],
        rotation: [f(7), f(9), f(11)],
        purity: f(12),
        base_primaries: sigmoid::BasePrimaries::from_darktable(int(13))
            .with_context(|| format!("unknown sigmoid base primaries {}", int(13)))?,
    })
}

fn sigmoid_prepare(xmp: &str) -> Result<sigmoid::Prepared> {
    // The references are rendered in darktable's default working space.
    Ok(sigmoid::prepare(
        &sigmoid_settings(xmp)?,
        &RgbSpace::LINEAR_REC2020,
    ))
}

fn sigmoid_inputs(prepared: &sigmoid::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: Vec::new(),
    }
}

/// The kernel's gradient slots: its powers and target white, then what each colour
/// processing reads besides, the hue preservation or the target black (`sigmoid.slang`).
/// With RGB ratio the check's loss, R + G + B, cannot see the target black: it acts only
/// in the chroma compression, which keeps the channels' mean. Its slot is checked with
/// unequal channel weights in `tests/sigmoid.rs` instead.
fn sigmoid_gradient(case: &Case) -> &'static [&'static str] {
    const PER_CHANNEL: [&str; 4] = [
        "film_power",
        "paper_power",
        "white_target",
        "hue_preservation",
    ];
    const RGB_RATIO: [&str; 3] = ["film_power", "paper_power", "white_target"];
    match sigmoid_settings(case.xmp).map(|settings| settings.color_processing) {
        Ok(ColorProcessing::PerChannel) => &PER_CHANNEL,
        Ok(ColorProcessing::RgbRatio) => &RGB_RATIO,
        Err(_) => &[],
    }
}

/// darktable `dt_iop_rgbcurve_params_t`, version 1, the only one: 516 bytes, the nodes
/// `curve_nodes[3][20]` as (x, y) f32 pairs, then `curve_num_nodes[3]`,
/// `curve_type[3]`, `curve_autoscale`, `compensate_middle_grey` and `preserve_colors`,
/// all i32.
fn rgbcurve_settings(xmp: &str) -> Result<rgbcurve::Settings> {
    let params = module_params(xmp, "rgbcurve")?;
    ensure!(
        params.version == 1,
        "rgbcurve params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 516,
        "rgbcurve v1 params are {} bytes, expected 516",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    let max_nodes = rgbcurve::MAX_NODES;
    let curve = |c: usize| -> Result<rgbcurve::Curve> {
        let count = usize::try_from(int(120 + c))
            .ok()
            .filter(|n| (2..=max_nodes).contains(n))
            .with_context(|| format!("rgbcurve curve {c} has {} nodes", int(120 + c)))?;
        let kind = int(123 + c);
        Ok(rgbcurve::Curve {
            nodes: (0..count)
                .map(|k| {
                    let at = 2 * (c * max_nodes + k);
                    rgbcurve::Node {
                        x: f(at),
                        y: f(at + 1),
                    }
                })
                .collect(),
            interpolation: Interpolation::from_darktable(kind)
                .with_context(|| format!("unknown rgbcurve curve type {kind}"))?,
        })
    };
    let mode = int(126);
    let preserve = int(128);
    Ok(rgbcurve::Settings {
        curves: [curve(0)?, curve(1)?, curve(2)?],
        mode: Mode::from_darktable(mode)
            .with_context(|| format!("unknown rgbcurve autoscale {mode}"))?,
        compensate_middle_grey: match int(127) {
            0 => false,
            1 => true,
            v => bail!("rgbcurve compensate_middle_grey is {v}"),
        },
        preserve_colors: Norm::from_darktable(preserve)
            .with_context(|| format!("unknown rgbcurve preserve colors {preserve}"))?,
    })
}

/// The gradient slots an rgbcurve case checks, and their names: a knot's x, y or tangent
/// ("R4 y" is the red curve's knot 4), or a curve's end value or power. Each is a value
/// the case's pixels reach: a knot inside their range, and an end the brightest of them
/// pass.
struct RgbCurveGradient {
    case: &'static str,
    names: [&'static str; 4],
    slots: [Slot; 4],
}

const fn slot(curve: usize, node: usize, kind: SlotKind) -> Slot {
    Slot { curve, node, kind }
}

/// For the linked cases: the red curve's knots 2 and 3, linear values 0.18 and 0.28,
/// uncompensated; compensated, knots 4 and 5 at 0.12 and 0.27.
const LINKED: [Slot; 4] = [
    slot(0, 2, SlotKind::Y),
    slot(0, 3, SlotKind::Tangent),
    slot(0, 3, SlotKind::X),
    slot(0, 0, SlotKind::EndPower),
];
const LINKED_NAMES: [&str; 4] = ["R2 y", "R3 tangent", "R3 x", "R end power"];
const LINKED_COMPENSATED: [Slot; 4] = [
    slot(0, 4, SlotKind::Y),
    slot(0, 5, SlotKind::Tangent),
    slot(0, 4, SlotKind::X),
    slot(0, 0, SlotKind::EndPower),
];
const LINKED_COMPENSATED_NAMES: [&str; 4] = ["R4 y", "R5 tangent", "R4 x", "R end power"];

static RGBCURVE_GRADIENTS: &[RgbCurveGradient] = &[
    RgbCurveGradient {
        case: "rgbcurve",
        names: ["R4 y", "G2 tangent", "B2 x", "G end power"],
        slots: [
            slot(0, 4, SlotKind::Y),
            slot(1, 2, SlotKind::Tangent),
            slot(2, 2, SlotKind::X),
            slot(1, 0, SlotKind::EndPower),
        ],
    },
    RgbCurveGradient {
        case: "rgbcurve-independent",
        names: ["R2 y", "G1 tangent", "B1 x", "B end value"],
        slots: [
            slot(0, 2, SlotKind::Y),
            slot(1, 1, SlotKind::Tangent),
            slot(2, 1, SlotKind::X),
            slot(2, 0, SlotKind::EndValue),
        ],
    },
    RgbCurveGradient {
        case: "rgbcurve-luminance",
        names: LINKED_COMPENSATED_NAMES,
        slots: LINKED_COMPENSATED,
    },
    RgbCurveGradient {
        case: "rgbcurve-none",
        names: LINKED_NAMES,
        slots: LINKED,
    },
    RgbCurveGradient {
        case: "rgbcurve-max",
        names: LINKED_COMPENSATED_NAMES,
        slots: LINKED_COMPENSATED,
    },
    RgbCurveGradient {
        case: "rgbcurve-average",
        names: LINKED_NAMES,
        slots: LINKED,
    },
    RgbCurveGradient {
        case: "rgbcurve-sum",
        names: LINKED_COMPENSATED_NAMES,
        slots: LINKED_COMPENSATED,
    },
    RgbCurveGradient {
        case: "rgbcurve-norm",
        names: LINKED_NAMES,
        slots: LINKED,
    },
    RgbCurveGradient {
        case: "rgbcurve-power",
        names: LINKED_COMPENSATED_NAMES,
        slots: LINKED_COMPENSATED,
    },
];

fn rgbcurve_gradient(case: &Case) -> Option<&'static RgbCurveGradient> {
    RGBCURVE_GRADIENTS
        .iter()
        .find(|gradient| gradient.case == case.name)
}

fn rgbcurve_prepare(case: &Case) -> Result<rgbcurve::Prepared> {
    // The references are rendered in darktable's default working space.
    let mut prepared = rgbcurve::prepare(
        &rgbcurve_settings(case.xmp)?,
        &WorkingProfile::linear_rec2020(),
    )?;
    if let Some(gradient) = rgbcurve_gradient(case) {
        prepared.gradient = gradient.slots;
    }
    Ok(prepared)
}

fn rgbcurve_inputs(prepared: &rgbcurve::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: vec![prepared.knots()],
    }
}

/// darktable `dt_iop_rgblevels_params_t`, version 1, the only one: 44 bytes,
/// `autoscale` and `preserve_colors` (i32), then `levels[3][3]`, each channel's black,
/// grey and white points (f32).
fn rgblevels_settings(xmp: &str) -> Result<rgblevels::Settings> {
    let params = module_params(xmp, "rgblevels")?;
    ensure!(
        params.version == 1,
        "rgblevels params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 44,
        "rgblevels v1 params are {} bytes, expected 44",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    let levels = |c: usize| rgblevels::Levels {
        black: f(2 + 3 * c),
        grey: f(3 + 3 * c),
        white: f(4 + 3 * c),
    };
    Ok(rgblevels::Settings {
        mode: rgblevels::Mode::from_darktable(int(0))
            .with_context(|| format!("unknown rgblevels autoscale {}", int(0)))?,
        preserve_colors: RgbNorm::from_darktable(int(1))
            .with_context(|| format!("unknown rgblevels preserve colors {}", int(1)))?,
        levels: [levels(0), levels(1), levels(2)],
    })
}

/// The levels an rgblevels case's gradient check moves, and their names.
struct RgbLevelsGradient {
    names: &'static [&'static str],
    slots: [rgblevels::Slot; 4],
}

const fn level(channel: usize, point: Point) -> rgblevels::Slot {
    rgblevels::Slot { channel, point }
}

/// Linked: the levels' black, grey and white. The fourth slot, green's grey, stands for
/// nothing and is not checked.
const RGBLEVELS_LINKED: RgbLevelsGradient = RgbLevelsGradient {
    names: &["black", "grey", "white"],
    slots: rgblevels::DEFAULT_GRADIENT,
};

/// Independent: every point and every channel at least once.
const RGBLEVELS_INDEPENDENT: RgbLevelsGradient = RgbLevelsGradient {
    names: &["R black", "G grey", "B white", "R white"],
    slots: [
        level(0, Point::Black),
        level(1, Point::Grey),
        level(2, Point::White),
        level(0, Point::White),
    ],
};

fn rgblevels_gradient(case: &Case) -> Option<&'static RgbLevelsGradient> {
    match rgblevels_settings(case.xmp).ok()?.mode {
        rgblevels::Mode::Linked => Some(&RGBLEVELS_LINKED),
        rgblevels::Mode::Independent => Some(&RGBLEVELS_INDEPENDENT),
    }
}

fn rgblevels_prepare(case: &Case) -> Result<rgblevels::Prepared> {
    // The references are rendered in darktable's default working space.
    let mut prepared = rgblevels::prepare(
        &rgblevels_settings(case.xmp)?,
        &WorkingProfile::linear_rec2020(),
    )?;
    if let Some(gradient) = rgblevels_gradient(case) {
        prepared.gradient = gradient.slots;
    }
    Ok(prepared)
}

fn rgblevels_inputs(prepared: &rgblevels::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: Vec::new(),
    }
}

/// darktable `dt_iop_vignette_params_t`, version 4: 44 bytes, `scale`,
/// `falloff_scale`, `brightness`, `saturation` and `center` x and y (f32), `autoratio`
/// (gboolean), `whratio` and `shape` (f32), `dithering` and `unbound` (i32).
fn vignette_settings(xmp: &str) -> Result<vignette::Settings> {
    let params = module_params(xmp, "vignette")?;
    ensure!(
        params.version == 4,
        "vignette params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 44,
        "vignette v4 params are {} bytes, expected 44",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    Ok(vignette::Settings {
        scale: f(0),
        falloff_scale: f(1),
        brightness: f(2),
        saturation: f(3),
        center: [f(4), f(5)],
        autoratio: int(6) != 0,
        whratio: f(7),
        shape: f(8),
        dithering: vignette::Dithering::from_darktable(int(9))
            .with_context(|| format!("unknown vignette dithering {}", int(9)))?,
        unbound: int(10) != 0,
    })
}

/// The parameters a vignette case's gradient check moves, and their names.
struct VignetteGradient {
    names: &'static [&'static str],
    slots: [Parameter; 4],
}

/// Test 0045: the brightness, the radii and the ratio.
///
/// No case checks the saturation here: the check's loss is the output's R + G + B, and
/// the saturation step keeps the channels' mean, so that loss's derivative with respect
/// to it is 0 but where clipping breaks the mean, and the check compares rounding noise
/// (docs/ports/vignette.md, Gradients). `crates/golden/tests/vignette.rs` checks it with
/// unequal channel weights.
const VIGNETTE_TEST_0045: VignetteGradient = VignetteGradient {
    names: &["brightness", "scale", "falloff_scale", "whratio"],
    slots: [
        Parameter::Brightness,
        Parameter::Scale,
        Parameter::FalloffScale,
        Parameter::Whratio,
    ],
};

/// The automatic ratio: the centre, the shape and the inner radius.
const VIGNETTE_AUTORATIO: VignetteGradient = VignetteGradient {
    names: &["center_x", "center_y", "shape", "scale"],
    slots: [
        Parameter::CenterX,
        Parameter::CenterY,
        Parameter::Shape,
        Parameter::Scale,
    ],
};

/// A ratio below 1: the ratio, the shape, the centre's y and the inner radius. Not the
/// centre's x: at shape 0.37 the vignette's sides are nearly straight columns, which a
/// step of the centre's x moves by 0.18 of the synthetic sweep's pixels, so a whole
/// column of the sweep often straddles the fall-off's kink, while the two sides'
/// contributions cancel in the total (docs/ports/vignette.md, Gradients).
const VIGNETTE_NARROW: VignetteGradient = VignetteGradient {
    names: &["whratio", "shape", "center_y", "scale"],
    slots: [
        Parameter::Whratio,
        Parameter::Shape,
        Parameter::CenterY,
        Parameter::Scale,
    ],
};

/// The star, a shape above 2, whose cusps on the centre's axes the centre would move
/// across pixels: the shape, the ratio and the radii.
const VIGNETTE_STAR: VignetteGradient = VignetteGradient {
    names: &["shape", "whratio", "scale", "falloff_scale"],
    slots: [
        Parameter::Shape,
        Parameter::Whratio,
        Parameter::Scale,
        Parameter::FalloffScale,
    ],
};

/// Clipped: the brightness and the fall-off's width, which leave the inner radius in
/// place. Moving the inner radius, the centre, the ratio or the shape moves the jump
/// darktable's gate makes for a value outside [0, 1] across pixels. The last two slots
/// are not checked.
const VIGNETTE_CLIPPED: VignetteGradient = VignetteGradient {
    names: &["brightness", "falloff_scale"],
    slots: [
        Parameter::Brightness,
        Parameter::FalloffScale,
        Parameter::Scale,
        Parameter::Saturation,
    ],
};

/// A white vignette, unclipped: the brightness, the radii and the shape. Not the centre:
/// a float64 emulation of the check gives its y 1.4e-2 on the synthetic sweep
/// (docs/ports/vignette.md, Gradients).
const VIGNETTE_WHITE: VignetteGradient = VignetteGradient {
    names: &["brightness", "scale", "falloff_scale", "shape"],
    slots: [
        Parameter::Brightness,
        Parameter::Scale,
        Parameter::FalloffScale,
        Parameter::Shape,
    ],
};

fn vignette_gradient(case: &Case) -> &'static VignetteGradient {
    match case.name {
        "vignette-autoratio" => &VIGNETTE_AUTORATIO,
        "vignette-narrow" => &VIGNETTE_NARROW,
        "vignette-star" => &VIGNETTE_STAR,
        "vignette-clip" => &VIGNETTE_CLIPPED,
        "vignette-white" => &VIGNETTE_WHITE,
        _ => &VIGNETTE_TEST_0045,
    }
}

fn vignette_prepare(case: &Case) -> Result<vignette::Prepared> {
    let mut prepared = vignette::prepare(&vignette_settings(case.xmp)?)?;
    prepared.gradient = vignette_gradient(case).slots;
    Ok(prepared)
}

fn vignette_inputs(prepared: &vignette::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: Vec::new(),
    }
}

/// darktable `dt_iop_grain_params_t`: version 1, 12 bytes, `channel` (i32), `scale`
/// and `strength` (f32); version 2 adds `midtones_bias` (f32). `legacy_params` gives
/// version 1 a mid-tones bias of 0, "the same results as the old version". darktable's
/// `process()` ignores the channel, so it is only checked to be one of darktable's four.
fn grain_settings(case: &Case) -> Result<grain::Settings> {
    let params = module_params(case.xmp, "grain")?;
    let expected = match params.version {
        1 => 12,
        2 => 16,
        v => bail!("grain params version {v} is not supported"),
    };
    ensure!(
        params.bytes.len() == expected,
        "grain v{} params are {} bytes, expected {expected}",
        params.version,
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let channel = i32::from_le_bytes(word[0]);
    ensure!((0..4).contains(&channel), "unknown grain channel {channel}");
    Ok(grain::Settings {
        scale: f(1),
        strength: f(2),
        midtones_bias: if params.version == 1 { 0.0 } else { f(3) },
        offset: grain::darktable_offset(case.image, grain_render_width(case.image)?),
    })
}

/// The width darktable renders a case's image at, its `roi_out->width` in an export at
/// full resolution, whose 0.3 times is the modulus of darktable's offset. A wrong width
/// gives an unrelated grain, which the case's ΔE shows at once.
fn grain_render_width(image: &str) -> Result<u32> {
    match image {
        "mire1.cr2" => Ok(3908),
        "hlrecovery.arw" => Ok(5492),
        _ => bail!("no darktable render width recorded for {image}"),
    }
}

/// The parameters a grain case's gradient check moves, and their names.
struct GrainGradient {
    names: &'static [&'static str],
    slots: [grain::Parameter; 3],
}

/// Every case checks the strength alone. The other two slots are not checked here
/// (docs/ports/grain.md, Gradients), and `crates/golden/tests/grain.rs` checks both:
///
/// - the mid-tones bias: a step of 1e-3 moves the paper response's table entries by
///   about 2e-6 L, below their f32 rounding, about 1e-5 L, which every pixel reading an
///   entry shares, so the forward pass's finite differences miss by 23% to 410% at the
///   cases' settings (a float32 emulation); on test 0059 the GPU's reverse mode,
///   -5.23353, equals float64 finite differences of darktable's whole formula, -5.23354;
/// - the scale: the loss's derivative is what is left of per-pixel terms of both signs,
///   2.4e4 times larger in total on test 0059, and the noise's jumps where it crosses
///   from one simplex to the next put errors of a few percent into its finite
///   differences, which a smaller step does not reduce (on test 0059, 2.4e-2 at a step
///   of 1e-3 and 1.2e-3 at 1e-2; on the synthetic sweep, the other way round).
const GRAIN_STRENGTH: GrainGradient = GrainGradient {
    names: &["strength"],
    slots: [
        grain::Parameter::Strength,
        grain::Parameter::MidtonesBias,
        grain::Parameter::Scale,
    ],
};

/// 100 ISO: none. Its grain is white noise at the synthetic sweep's pixels, so the
/// strength's derivative there, 1.0194 in float64 (the GPU's reverse mode 1.0201), is
/// what is left of per-pixel terms of both signs, and the f32 rounding of the sweep's
/// brightest levels, up to 4, puts 2.9e-2 into its finite differences at a step of
/// 1e-3, falling to 2.6e-3 at 3e-2 (docs/ports/grain.md, Gradients). On its own image
/// it passes at 1.1e-4.
const GRAIN_NONE: GrainGradient = GrainGradient {
    names: &[],
    ..GRAIN_STRENGTH
};

fn grain_gradient(case: &Case) -> &'static GrainGradient {
    match case.name {
        "grain-fine" => &GRAIN_NONE,
        _ => &GRAIN_STRENGTH,
    }
}

fn grain_prepare(case: &Case) -> Result<grain::Prepared> {
    // The references are rendered in darktable's default working space.
    let mut prepared = grain::prepare(&grain_settings(case)?, &WorkingProfile::linear_rec2020())?;
    prepared.gradient = grain_gradient(case).slots;
    Ok(prepared)
}

fn grain_inputs(prepared: &grain::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: vec![grain::permutation_table()],
    }
}

/// Through the operation's `prepare`, as the engine runs it, from the settings as
/// parameter values checked against its schema. Its working space, linear Rec2020, is
/// darktable's default, the references'.
fn color_balance_inputs(settings: &Settings) -> Result<KernelInputs> {
    let operation = colorbalancergb::ColorBalanceRgb;
    let params = operation
        .schema()
        .resolve(&colorbalancergb::values(settings))?;
    // A per-pixel operation: the image's size reaches neither its uniforms nor its table.
    let prepared = operation.prepare(&params, &ImageContext::full(1, 1), &ImageDesc::rgba(1, 1))?;
    let [pass] = <[_; 1]>::try_from(prepared.passes)
        .map_err(|passes| anyhow::anyhow!("colorbalancergb prepared {} passes", passes.len()))?;
    Ok(KernelInputs {
        params: pass.params,
        tables: pass.tables,
    })
}

/// darktable `dt_iop_channelmixer_rgb_params_t`, version 3, which every test sidecar
/// holds: 160 bytes, the arrays `red`, `green`, `blue`, `saturation`, `lightness` and
/// `grey` of four f32 each (the fourth unused), then the six normalisations (gboolean),
/// `illuminant`, `illum_fluo`, `illum_led` and `adaptation` (enums), `x`, `y`,
/// `temperature` and `gamut` (f32), `clip` (gboolean) and `version` (enum). Versions 1
/// and 2 would need `legacy_params`' migration, which no case needs (the spec's
/// Parameter versions). Public for the Colour calibration module's darktable match
/// (`crates/modules/tests/colour_calibration.rs`).
pub fn channelmixerrgb_settings(xmp: &str) -> Result<channelmixerrgb::Settings> {
    use apertrust_color::{Adaptation, Illuminant};
    let params = module_params(xmp, "channelmixerrgb")?;
    ensure!(
        params.version == 3,
        "channelmixerrgb params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 160,
        "channelmixerrgb v3 params are {} bytes, expected 160",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    let flag = |i: usize| match int(i) {
        0 => Ok(false),
        1 => Ok(true),
        v => bail!("channelmixerrgb boolean field {i} is {v}"),
    };
    let array = |at: usize| [f(at), f(at + 1), f(at + 2)];
    Ok(channelmixerrgb::Settings {
        red: array(0),
        green: array(4),
        blue: array(8),
        saturation: array(12),
        lightness: array(16),
        grey: array(20),
        normalize_r: flag(24)?,
        normalize_g: flag(25)?,
        normalize_b: flag(26)?,
        normalize_sat: flag(27)?,
        normalize_light: flag(28)?,
        normalize_grey: flag(29)?,
        illuminant: Illuminant::from_darktable(int(30))
            .with_context(|| format!("unknown channelmixerrgb illuminant {}", int(30)))?,
        illum_fluo: u32::try_from(int(31))?,
        illum_led: u32::try_from(int(32))?,
        adaptation: Adaptation::from_darktable(int(33))
            .with_context(|| format!("unknown channelmixerrgb adaptation {}", int(33)))?,
        x: f(34),
        y: f(35),
        temperature: f(36),
        gamut: f(37),
        clip: flag(38)?,
        version: channelmixerrgb::Version::from_darktable(int(39))
            .with_context(|| format!("unknown channelmixerrgb version {}", int(39)))?,
    })
}

/// darktable `dt_iop_channelmixer_rgb_params_t` version 3 for these settings, the inverse
/// of `channelmixerrgb_settings`: 40 four-byte fields in the struct's order, each array's
/// unused fourth element 0.
pub fn channel_mixer_params_v3(settings: &channelmixerrgb::Settings) -> Vec<u8> {
    // Every field named, so a new one cannot be left out.
    let channelmixerrgb::Settings {
        red,
        green,
        blue,
        saturation,
        lightness,
        grey,
        normalize_r,
        normalize_g,
        normalize_b,
        normalize_sat,
        normalize_light,
        normalize_grey,
        illuminant,
        illum_fluo,
        illum_led,
        adaptation,
        x,
        y,
        temperature,
        gamut,
        clip,
        version,
    } = *settings;
    let version = channelmixerrgb::Version::ALL
        .iter()
        .position(|&v| v == version)
        .expect("every version is in ALL") as i32;
    let mut words: Vec<[u8; 4]> = Vec::with_capacity(40);
    for array in [red, green, blue, saturation, lightness, grey] {
        words.extend(array.into_iter().chain([0.0]).map(f32::to_le_bytes));
    }
    let flags = [
        normalize_r,
        normalize_g,
        normalize_b,
        normalize_sat,
        normalize_light,
        normalize_grey,
    ];
    words.extend(flags.map(|flag| i32::from(flag).to_le_bytes()));
    words.extend(
        [
            illuminant.to_darktable(),
            illum_fluo as i32,
            illum_led as i32,
            adaptation.to_darktable(),
        ]
        .map(i32::to_le_bytes),
    );
    words.extend([x, y, temperature, gamut].map(f32::to_le_bytes));
    words.extend([i32::from(clip), version].map(i32::to_le_bytes));
    words.concat()
}

/// What the camera's illuminant needs, from rawler's decode of the case's raw, as
/// `prepare` gets it from the image context. The case must be at the camera's reference
/// as darktable's default leaves it: temperature (params version 4) at "as shot to
/// reference" with the as-shot multipliers, which colorin's late correction turns into
/// the camera's D65 multipliers.
fn channelmixerrgb_camera(case: &Case) -> Result<channelmixerrgb::CameraWhite> {
    let path = crate::render::images_dir()?.join(case.image);
    let raw = apertrust_raw::decode_file(&path)
        .with_context(|| format!("decoding {}", path.display()))?;
    let image = apertrust_engine::raw::image_context(&raw);
    let camera = image.camera.context("the case's image is no raw")?;
    let xyz_to_camera = camera
        .xyz_to_camera
        .context("rawler has no matrix for the case's camera")?;
    let as_shot = match camera.white_balance.as_deref() {
        Some([r, g, b, ..]) => [*r, *g, *b],
        _ => bail!("rawler has no as-shot white balance for {}", case.image),
    };
    let temperature = module_params(case.xmp, "temperature")?;
    ensure!(
        temperature.version == 4 && temperature.bytes.len() == 20,
        "the camera case needs temperature params version 4"
    );
    let word = words(&temperature.bytes);
    let coefficients = [0, 1, 2].map(|c| f32::from_le_bytes(word[c]));
    ensure!(
        i32::from_le_bytes(word[4]) == 4
            && (0..3).all(|c| (coefficients[c] - as_shot[c]).abs() < 1e-5),
        "the camera case needs temperature at \"as shot to reference\" with the as-shot \
         multipliers {as_shot:?}, not {coefficients:?}"
    );
    let applied = apertrust_color::daylight_multipliers(&xyz_to_camera).map(|v| v as f32);
    Ok(channelmixerrgb::CameraWhite {
        xyz_to_camera,
        as_shot,
        applied,
    })
}

fn channelmixerrgb_prepare(case: &Case) -> Result<channelmixerrgb::Prepared> {
    let settings = channelmixerrgb_settings(case.xmp)?;
    let camera = if settings.illuminant == apertrust_color::Illuminant::Camera {
        let camera = channelmixerrgb_camera(case)?;
        ensure!(
            camera.illuminant_xy().is_some(),
            "no camera illuminant for {}",
            case.image
        );
        Some(camera)
    } else {
        None
    };
    // The references are rendered in darktable's default working space.
    Ok(channelmixerrgb::prepare(
        &settings,
        &WorkingProfile::linear_rec2020(),
        camera.as_ref(),
    )?)
}

fn channelmixerrgb_inputs(prepared: &channelmixerrgb::Prepared) -> KernelInputs {
    KernelInputs {
        params: prepared.params(),
        tables: Vec::new(),
    }
}

/// The illuminant's x and y on every case but the bypass, where no adaptation runs and
/// the illuminant reaches nothing: both derivatives are 0, which the check would
/// compare with finite differences of 0.
fn channelmixerrgb_gradient(case: &Case) -> &'static [&'static str] {
    match case.name {
        "channelmixerrgb-bypass" => &[],
        _ => &["x", "y"],
    }
}

/// darktable `dt_iop_toneequalizer_params_t` version 2, 72 bytes: the nine bands' gains,
/// -8 EV to 0 EV, then blending, smoothing, feathering, quantization, contrast_boost
/// and exposure_boost (f32), details, method and iterations (i32). Every integration
/// test stores version 2; version 1, older than darktable's repository, is not
/// supported.
fn toneequal_settings(xmp: &str) -> Result<toneequal::Settings> {
    let params = module_params(xmp, "toneequal")?;
    ensure!(
        params.version == 2,
        "toneequal params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 72,
        "toneequal v2 params are {} bytes, expected 72",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let int = |i: usize| i32::from_le_bytes(word[i]);
    Ok(toneequal::Settings {
        bands: std::array::from_fn(f),
        blending: f(9),
        smoothing: f(10),
        feathering: f(11),
        quantization: f(12),
        contrast_boost: f(13),
        exposure_boost: f(14),
        details: toneequal::Filter::from_darktable(int(15))
            .with_context(|| format!("unknown toneequal filter {}", int(15)))?,
        method: toneequal::Estimator::from_darktable(int(16))
            .with_context(|| format!("unknown toneequal estimator {}", int(16)))?,
        iterations: int(17),
    })
}

/// Orientation's one parameter from darktable's params version 2, its
/// `dt_image_orientation_t` as an `int32`: the schema's choice is darktable's value plus
/// one, auto (-1) the first. Version 1 is not supported: darktable merges it with the
/// image's orientation as it loads it (`legacy_params`).
fn flip_values(xmp: &str) -> Result<ParamValues> {
    let params = module_params(xmp, "flip")?;
    ensure!(
        params.version == 2,
        "flip params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 4,
        "flip v2 params are {} bytes, expected 4",
        params.bytes.len()
    );
    let orientation = i32::from_le_bytes(words(&params.bytes)[0]);
    ensure!(
        (-1..=7).contains(&orientation),
        "flip orientation {orientation} is not darktable's"
    );
    let mut values = ParamValues::new();
    values.set(
        "orientation",
        apertrust_plugin::Value::Choice((orientation + 1) as u32),
    );
    Ok(values)
}

/// darktable `dt_iop_hazeremoval_params_t`, versions 1 to 3: strength and distance (f32),
/// then from version 2 `compatibility_mode` and from version 3 `adaptive` (gboolean).
/// `legacy_params` gives version 1 the compatibility mode and fixed windows, version 2
/// fixed windows. Neither is a setting of the port (`docs/ports/hazeremoval.md`,
/// Deliberate differences): fixed windows are darktable's adaptive ones at full
/// resolution, where the harness renders, and the compatibility mode's quick select,
/// which mis-partitions, gives way to the exact selection a new instance uses, which
/// moves test 0026's A0 by about 2% (a mean ΔE2000 of 0.02 between darktable's two).
fn hazeremoval_settings(xmp: &str) -> Result<hazeremoval::Settings> {
    let params = module_params(xmp, "hazeremoval")?;
    let size = match params.version {
        1 => 8,
        2 => 12,
        3 => 16,
        version => bail!("hazeremoval params version {version} is not supported"),
    };
    ensure!(
        params.bytes.len() == size,
        "hazeremoval v{} params are {} bytes, expected {size}",
        params.version,
        params.bytes.len()
    );
    let word = words(&params.bytes);
    Ok(hazeremoval::Settings {
        strength: f32::from_le_bytes(word[0]),
        distance: f32::from_le_bytes(word[1]),
    })
}

/// The tone equalizer's gradient parameters: the band gains along the default
/// directions, each a band alone, in EV, named by darktable's field and the band's EV
/// since the names below -2 EV are not the bands' labels.
const TONEEQUAL_GRADIENT: [&str; 4] = [
    "deep_blacks -6 EV",
    "shadows -4 EV",
    "highlights -2 EV",
    "speculars 0 EV",
];

/// The schema values with the band gains moved by `delta` EV along gradient direction
/// `index`, the curve fitted again: finite differences of the whole setup.
fn toneequal_perturbed(xmp: &str, index: usize, delta: f32) -> Result<ParamValues> {
    let mut settings = toneequal_settings(xmp)?;
    let direction = toneequal::DEFAULT_DIRECTIONS
        .get(index)
        .with_context(|| format!("no toneequal gradient parameter {index}"))?;
    for (band, step) in settings.bands.iter_mut().zip(direction) {
        *band += delta * step;
    }
    Ok(toneequal::values(&settings))
}

/// darktable `dt_iop_primaries_params_t`, version 1, the only one: 32 bytes, eight f32,
/// `achromatic_tint_hue`, `achromatic_tint_purity`, then each primary's hue and purity,
/// red, green and blue; hues in radians.
fn primaries_settings(xmp: &str) -> Result<primaries::Settings> {
    let params = module_params(xmp, "primaries")?;
    ensure!(
        params.version == 1,
        "primaries params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 32,
        "primaries v1 params are {} bytes, expected 32",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let mut settings = primaries::Settings::default();
    for (parameter, word) in primaries::Parameter::ALL.into_iter().zip(word) {
        *settings.get_mut(parameter) = f32::from_le_bytes(word);
    }
    Ok(settings)
}

/// The settings `prepare` gives the gradient slots, which the case checks.
const PRIMARIES_GRADIENT: [&str; 4] = [
    primaries::DEFAULT_GRADIENT[0].name(),
    primaries::DEFAULT_GRADIENT[1].name(),
    primaries::DEFAULT_GRADIENT[2].name(),
    primaries::DEFAULT_GRADIENT[3].name(),
];

/// Through the operation's `prepare`, as the engine runs it, from the settings as
/// parameter values checked against its schema. Its working space, linear Rec2020, is
/// darktable's default, the references'.
fn primaries_inputs(settings: &primaries::Settings) -> Result<KernelInputs> {
    let operation = primaries::Primaries;
    let params = operation.schema().resolve(&primaries::values(settings))?;
    // A per-pixel operation: the image's size reaches neither its uniform nor a table.
    let prepared = operation.prepare(&params, &ImageContext::full(1, 1), &ImageDesc::rgba(1, 1))?;
    let [pass] = <[_; 1]>::try_from(prepared.passes)
        .map_err(|passes| anyhow::anyhow!("primaries prepared {} passes", passes.len()))?;
    Ok(KernelInputs {
        params: pass.params,
        tables: pass.tables,
    })
}

/// darktable's local contrast params, version 3, or an older version migrated as its
/// `legacy_params` does: version 1 has no mode or mid-tone range, version 2 no mid-tone
/// range, which both take as 0.2, and version 1 the bilateral grid.
fn bilat_from_params(version: u32, bytes: &[u8]) -> Result<bilat::Settings> {
    let expected = match version {
        1 => 12,
        2 => 16,
        3 => 20,
        v => bail!("bilat params version {v} is not supported"),
    };
    ensure!(
        bytes.len() == expected,
        "bilat v{version} params are {} bytes, expected {expected}",
        bytes.len()
    );
    let word = words(bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    // Version 1 starts at sigma_r; the others with the mode.
    let (mode, first) = if version == 1 {
        (bilat::Mode::BilateralGrid, 0)
    } else {
        let index = u32::from_le_bytes(word[0]);
        let mode = bilat::Mode::from_index(index)
            .with_context(|| format!("unknown bilat mode {index}"))?;
        (mode, 1)
    };
    Ok(bilat::Settings {
        mode,
        sigma_r: f(first),
        sigma_s: f(first + 1),
        detail: f(first + 2),
        midtone: if version == 3 { f(4) } else { 0.2 },
    })
}

fn bilat_settings(xmp: &str) -> Result<bilat::Settings> {
    let params = module_params(xmp, "bilat")?;
    bilat_from_params(params.version, &params.bytes)
}

/// A local contrast case's values with gradient parameter `index` moved by `delta`, in
/// the schema's units, which are the kernel's: detail, shadows (`sigma_s`), highlights
/// (`sigma_r`), the mid-tone range.
fn bilat_perturbed(xmp: &str, index: usize, delta: f32) -> Result<ParamValues> {
    let mut settings = bilat_settings(xmp)?;
    match index {
        0 => settings.detail += delta,
        1 => settings.sigma_s += delta,
        2 => settings.sigma_r += delta,
        3 => settings.midtone += delta,
        _ => bail!("no bilat gradient parameter {index}"),
    }
    Ok(bilat::values(&settings))
}

/// The parameters a local contrast case's gradient check moves (docs/ports/bilat.md,
/// Gradients): the bilateral grid's detail, which its grid does not depend on. The local
/// laplacian filter's parameters all feed its pyramid, which decision 15's derivatives
/// of `main` hold fixed, so they differ from finite differences of whole renders by
/// every level but the finest, and none is named.
fn bilat_gradient(case: &Case) -> &'static [&'static str] {
    match bilat_settings(case.xmp).map(|settings| settings.mode) {
        Ok(bilat::Mode::BilateralGrid) => &["detail"],
        Ok(bilat::Mode::LocalLaplacian) | Err(_) => &[],
    }
}

/// darktable's bloom params, version 1, its only one: size, threshold and strength.
fn bloom_settings(xmp: &str) -> Result<bloom::Settings> {
    let params = module_params(xmp, "bloom")?;
    ensure!(
        params.version == 1,
        "bloom params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 12,
        "bloom v1 params are {} bytes, expected 12",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    Ok(bloom::Settings {
        size: f(0),
        threshold: f(1),
        strength: f(2),
    })
}

/// The parameters a bloom case's gradient check moves (docs/ports/bloom.md, Gradients):
/// none, since a slot is exact or not declared (decision 11). `main` carries the strength's derivative for the pixels past the threshold, but
/// finite differences of whole renders also see the pixels whose scaled lightness
/// crosses it, which on test 0050 make the strength's derivative 3.8 times `main`'s, at
/// every step from 1e-3 to 0.1; on the synthetic sweep a step of 1e-3 happens to cross
/// none, and larger ones put the gap at 15%. The threshold's derivative is all
/// crossings, and the size's radius a whole number. `crates/golden/tests/bloom.rs`
/// checks the strength's derivative where no pixel crosses, on an image with none near
/// the threshold and with the threshold moved in proportion to the lights.
const BLOOM_GRADIENT: [&str; 0] = [];

/// A bloom case's values with gradient parameter `index` moved by `delta`, in the
/// schema's units, which are the kernel's: the strength, slot 0 of its derivative entry
/// points, which no case names.
fn bloom_perturbed(xmp: &str, index: usize, delta: f32) -> Result<ParamValues> {
    let mut settings = bloom_settings(xmp)?;
    match index {
        0 => settings.strength += delta,
        _ => bail!("no bloom gradient parameter {index}"),
    }
    Ok(bloom::values(&settings))
}

/// darktable `dt_iop_colorequal_params_t` from a case's sidecar, migrated to version 4
/// as its `legacy_params` does (docs/ports/colorequal.md, Parameter versions).
fn colorequal_settings(xmp: &str) -> Result<colorequal::Settings> {
    let params = module_params(xmp, "colorequal")?;
    colorequal_from_params(params.version, &params.bytes)
}

/// darktable's color equalizer params, version 4 (32 four-byte fields: threshold,
/// smoothing_hue, contrast, white_level, chroma_size, param_size, use_filter as an int,
/// the saturation, hue and brightness nodes red to magenta, hue_shift) or version 1, the
/// same without hue_shift (124 bytes), which `legacy_params` takes through versions 2
/// and 3: hue_shift 0; the threshold 0.024, the saturation and brightness nodes brought
/// to 3/4 of their departure from 1; then the threshold raised by 0.1 and the contrast
/// `-5 · max(0, threshold - 0.024)`, 0. Versions 2 and 3 are refused: darktable's
/// migrations from them copy all but the last field, leaving hue_shift uninitialised.
fn colorequal_from_params(version: u32, bytes: &[u8]) -> Result<colorequal::Settings> {
    use colorequal::NODES;
    let expected = match version {
        1 => 124,
        4 => 128,
        v => bail!("colorequal params version {v} is not supported"),
    };
    ensure!(
        bytes.len() == expected,
        "colorequal v{version} params are {} bytes, expected {expected}",
        bytes.len()
    );
    let word = words(bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    let nodes = |first: usize| -> [f32; NODES] { std::array::from_fn(|k| f(first + k)) };
    let mut settings = colorequal::Settings {
        threshold: f(0),
        smoothing_hue: f(1),
        contrast: f(2),
        white_level: f(3),
        chroma_size: f(4),
        param_size: f(5),
        use_filter: i32::from_le_bytes(word[6]) != 0,
        saturation: nodes(7),
        hue: nodes(15),
        brightness: nodes(23),
        hue_shift: if version == 4 { f(31) } else { 0.0 },
    };
    if version == 1 {
        // Version 2 to 3.
        settings.threshold = 0.024;
        let soften = |v: f32| 1.0 + 0.75 * (v - 1.0);
        settings.saturation = settings.saturation.map(soften);
        settings.brightness = settings.brightness.map(soften);
        // Version 3 to 4.
        let old = settings.threshold;
        settings.threshold = old + 0.1;
        settings.contrast = -5.0 * 0.0f32.max(old - 0.024);
    }
    Ok(settings)
}

/// The parameters a colour equalizer case's gradient check moves, in the order of its
/// derivative entry points' slots (docs/ports/colorequal.md, Gradients): the hue,
/// saturation and brightness curves' red nodes, exact without the guided filter. The
/// fourth slot, the white level, is exact too, but its whole-render total on test 0158
/// is what is left of per-pixel terms of both signs, which the near-grey pixels crossing
/// darktable's neutral threshold (chroma above `NORM_MIN`) outweigh in finite
/// differences at every step; the synthetic sweep resolves it at 1e-2, and
/// `crates/golden/tests/colorequal.rs` checks it on an image away from that threshold.
/// With the guided filter, every slot feeds the filter's whole-image passes as well as
/// `main`, so none is exact and the case names none (decision 11).
fn colorequal_gradient(case: &Case) -> &'static [&'static str] {
    match colorequal_settings(case.xmp) {
        Ok(settings) if !settings.use_filter => &COLOREQUAL_GRADIENT[..3],
        _ => &[],
    }
}

const COLOREQUAL_GRADIENT: [&str; 4] = ["hue_red", "sat_red", "bright_red", "white_level"];

/// A colour equalizer case's values with gradient parameter `index` moved by `delta`, in
/// the schema's units, which are the kernel's: EV, degrees and the curves' values.
fn colorequal_perturbed(xmp: &str, index: usize, delta: f32) -> Result<ParamValues> {
    let mut settings = colorequal_settings(xmp)?;
    match index {
        0 => settings.hue[0] += delta,
        1 => settings.saturation[0] += delta,
        2 => settings.brightness[0] += delta,
        3 => settings.white_level += delta,
        _ => bail!("no colorequal gradient parameter {index}"),
    }
    Ok(colorequal::values(&settings))
}

/// darktable `dt_iop_filmicrgb_params_t`, every version, migrated to version 6 as its
/// `legacy_params` does (docs/ports/filmicrgb.md, Parameter versions).
fn filmicrgb_settings(xmp: &str) -> Result<filmicrgb::Settings> {
    let params = module_params(xmp, "filmicrgb")?;
    filmicrgb_from_params(params.version, &params.bytes)
}

fn filmicrgb_from_params(version: u32, bytes: &[u8]) -> Result<filmicrgb::Settings> {
    use filmicrgb::{ColorScience, CurveType, Noise, Norm, Settings, SplineVersion};
    // Each version's size: 12 floats and an int, then 17, 18 or 18 floats and 7, 8, 10,
    // 10 or 11 four-byte fields.
    let size = match version {
        1 => 52,
        2 => 96,
        3 => 104,
        4 | 5 => 112,
        6 => 116,
        _ => bail!("filmicrgb params version {version} is not supported"),
    };
    ensure!(
        bytes.len() == size,
        "filmicrgb params version {version} has {} bytes, not {size}",
        bytes.len()
    );
    let f = |i: usize| f32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
    let n = |i: usize| i32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
    let norm = |i: usize| {
        u32::try_from(n(i))
            .ok()
            .and_then(Norm::from_index)
            .with_context(|| format!("preserve_color {}", n(i)))
    };
    let science = |i: usize| {
        u32::try_from(n(i))
            .ok()
            .and_then(ColorScience::from_index)
            .with_context(|| format!("colour science {}", n(i)))
    };
    let curve = |i: usize| {
        u32::try_from(n(i))
            .ok()
            .and_then(CurveType::from_index)
            .with_context(|| format!("curve {}", n(i)))
    };
    let noise = |i: usize| {
        u32::try_from(n(i))
            .ok()
            .and_then(Noise::from_index)
            .with_context(|| format!("noise distribution {}", n(i)))
    };
    let iterations = |i: usize| u32::try_from(n(i)).context("negative iterations");
    // Versions 2 to 6 share their first 17 floats, version 3 on adding the noise level.
    let reconstruction = |s: &mut Settings| {
        s.reconstruct_threshold = f(3);
        s.reconstruct_feather = f(4);
        s.reconstruct_bloom_vs_details = f(5);
        s.reconstruct_grey_vs_color = f(6);
        s.reconstruct_structure_vs_texture = f(7);
    };
    let look = |s: &mut Settings, first: usize| {
        s.security_factor = f(first);
        s.grey_point_target = f(first + 1);
        s.black_point_target = f(first + 2);
        s.white_point_target = f(first + 3);
        s.output_power = f(first + 4);
        s.latitude = f(first + 5);
        s.contrast = f(first + 6);
        s.saturation = f(first + 7);
        s.balance = f(first + 8);
    };
    let mut s = Settings {
        grey_point_source: f(0),
        black_point_source: f(1),
        white_point_source: f(2),
        ..Settings::default()
    };
    match version {
        1 => {
            look(&mut s, 3);
            s.preserve_color = norm(12)?;
            s.shadows = CurveType::Hard;
            s.highlights = CurveType::Soft;
            // Beyond any white, so a no-op for old edits.
            s.reconstruct_threshold = 6.0;
            s.reconstruct_bloom_vs_details = 100.0;
            s.reconstruct_grey_vs_color = 100.0;
            s.reconstruct_structure_vs_texture = 0.0;
            s.reconstruct_feather = 3.0;
            s.version = ColorScience::V3;
            s.auto_hardness = true;
            s.custom_grey = true;
            s.high_quality_reconstruction = 0;
            s.noise_distribution = Noise::Gaussian;
            s.noise_level = 0.0;
            s.spline_version = SplineVersion::V1;
            s.compensate_icc_black = false;
            filmicrgb_spline_v3(&mut s);
            s.enable_highlight_reconstruction = true;
        }
        2 => {
            reconstruction(&mut s);
            look(&mut s, 8);
            s.preserve_color = norm(17)?;
            s.version = science(18)?;
            s.auto_hardness = n(19) != 0;
            s.custom_grey = n(20) != 0;
            s.high_quality_reconstruction = iterations(21)?;
            s.shadows = curve(22)?;
            s.highlights = curve(23)?;
            s.noise_distribution = Noise::Gaussian;
            s.noise_level = 0.0;
            s.spline_version = SplineVersion::V1;
            s.compensate_icc_black = false;
            s.enable_highlight_reconstruction = true;
            filmicrgb_spline_v3(&mut s);
        }
        3 => {
            reconstruction(&mut s);
            look(&mut s, 8);
            s.preserve_color = norm(18)?;
            s.version = science(19)?;
            s.auto_hardness = n(20) != 0;
            s.custom_grey = n(21) != 0;
            s.high_quality_reconstruction = iterations(22)?;
            s.shadows = curve(24)?;
            s.highlights = curve(25)?;
            // darktable resets the noise rather than copying it.
            s.noise_level = 0.2;
            s.noise_distribution = Noise::Gaussian;
            s.spline_version = SplineVersion::V1;
            s.compensate_icc_black = false;
            s.enable_highlight_reconstruction = true;
            filmicrgb_spline_v3(&mut s);
        }
        _ => {
            reconstruction(&mut s);
            look(&mut s, 8);
            s.noise_level = f(17);
            s.preserve_color = norm(18)?;
            s.version = science(19)?;
            s.auto_hardness = n(20) != 0;
            s.custom_grey = n(21) != 0;
            s.high_quality_reconstruction = iterations(22)?;
            s.noise_distribution = noise(23)?;
            s.shadows = curve(24)?;
            s.highlights = curve(25)?;
            s.compensate_icc_black = n(26) != 0;
            s.spline_version = match (version, n(27)) {
                // Version 4 holds the spline's year.
                (4, 2019) => SplineVersion::V1,
                (4, 2020) => SplineVersion::V2,
                (4, 2021) => SplineVersion::V3,
                (4, year) => bail!("filmicrgb internal version {year}, which darktable refuses"),
                (_, index) => u32::try_from(index)
                    .ok()
                    .and_then(SplineVersion::from_index)
                    .with_context(|| format!("spline version {index}"))?,
            };
            s.enable_highlight_reconstruction = version < 6 || n(28) != 0;
            if version == 4 {
                filmicrgb_spline_v3(&mut s);
            }
        }
    }
    Ok(s)
}

/// darktable `convert_to_spline_v3`: the settings that give spline v3 the old spline's
/// contrast, latitude and balance, in its f32 arithmetic.
fn filmicrgb_spline_v3(n: &mut filmicrgb::Settings) {
    use filmicrgb::{Real, SplineVersion};
    if n.spline_version == SplineVersion::V3 {
        return;
    }
    let spline = filmicrgb::compute_spline(n);
    let grey_log = spline.x[2];
    let toe_log = spline.x[1].min(grey_log);
    let shoulder_log = spline.x[3].max(grey_log);
    let black_display = spline.y[0];
    let grey_display = spline.y[2];
    let white_display = spline.y[4];
    let margin = 0.01 * (white_display - black_display);
    let hardness = n.output_power;
    let mut contrast =
        (spline.y[3].max(grey_display) - spline.y[1].min(grey_display)) / (shoulder_log - toe_log);
    // The toe and shoulder kept within the margin; darktable moves their log values
    // along the slope too, which nothing reads after.
    let toe_display = spline.y[1].min(grey_display).max(black_display + margin);
    let shoulder_display = {
        let y = spline.y[3].max(grey_display);
        if y > white_display - margin {
            white_display - margin
        } else {
            y
        }
    };
    // What spline v3 will make of the contrast, undone.
    contrast *= 8.0 / (n.white_point_source - n.black_point_source);
    contrast *= hardness * Real::powf(grey_display, hardness - 1.0);
    let latitude = ((shoulder_display - toe_display)
        / ((white_display - black_display) - 2.0 * margin))
        .clamp(0.0, 0.99);
    let toe_reference = latitude * (black_display + margin) + (1.0 - latitude) * grey_display;
    let shoulder_reference = latitude * (white_display - margin) + (1.0 - latitude) * grey_display;
    let balance = if shoulder_display < shoulder_reference {
        0.5 * (1.0
            - (shoulder_display - grey_display).max(0.0)
                / (shoulder_reference - grey_display).max(1e-5))
    } else {
        -0.5 * (1.0
            - (grey_display - toe_display).max(0.0) / (grey_display - toe_reference).max(1e-5))
    };
    if n.spline_version == SplineVersion::V1 {
        // Spline v3 takes the targets through the power, spline v1 did not.
        n.black_point_target = Real::powf(black_display, hardness) * 100.0;
        n.white_point_target = Real::powf(white_display, hardness) * 100.0;
    }
    n.latitude = latitude * 100.0;
    n.contrast = contrast;
    n.balance = balance * 100.0;
    n.spline_version = SplineVersion::V3;
}

/// The filmic rgb case's values with gradient direction `index` moved by `delta` in the
/// direction's units: the percentages as fractions (docs/ports/filmicrgb.md, Gradients).
fn filmicrgb_perturbed(xmp: &str, index: usize, delta: f32) -> Result<ParamValues> {
    use filmicrgb::Direction;
    let mut settings = filmicrgb_settings(xmp)?;
    let direction = filmicrgb::DIRECTIONS
        .get(index)
        .with_context(|| format!("no filmicrgb gradient parameter {index}"))?;
    let step = delta * direction.scale();
    match direction {
        Direction::Contrast => settings.contrast += step,
        Direction::Latitude => settings.latitude += step,
        Direction::OutputPower => settings.output_power += step,
        Direction::Saturation => settings.saturation += step,
    }
    Ok(filmicrgb::values(&settings))
}

/// The derivative entry points' four directions, by their parameters.
const FILMICRGB_GRADIENT: [&str; 4] = ["contrast", "output_power", "latitude", "saturation"];

/// The parameters a filmic rgb case's gradient check moves, a prefix of the kernel's
/// slots, the same on the case and the sweep (docs/ports/filmicrgb.md, Gradients): all
/// four, but
/// - the saturation of v6 (2022) at 0, where `filmic_desaturate_v4` switches branch on
///   its sign, a kink finite differences straddle;
/// - the saturation of v3 (2019) with a norm, which desaturates each pixel's ratios
///   towards their luminance: the sweep's hues, grey on average, cancel it in the loss
///   (the case's own image sees it, within 1.2e-5);
/// - the latitude and saturation of test 0037, whose totals on the chart are small
///   enough that the 1% of its pixels on the ratios' clamps at 0, saturated oranges
///   with blue at or below 0, decide them: finite differences straddle those kinks, to
///   46% at a step of 1e-3 and about 1.3% at 3e-3 and 1e-2, while forward and reverse
///   mode agree per pixel within 1.4e-6 and the same settings pass on the sweep.
fn filmicrgb_gradient(case: &Case) -> &'static [&'static str] {
    use filmicrgb::{ColorScience, Norm};
    if case.name == "filmicrgb-reconstruction" {
        return &FILMICRGB_GRADIENT[..2];
    }
    match filmicrgb_settings(case.xmp) {
        Ok(settings)
            if (settings.version == ColorScience::V6 && settings.saturation == 0.0)
                || (settings.version == ColorScience::V3
                    && settings.preserve_color != Norm::None) =>
        {
            &FILMICRGB_GRADIENT[..3]
        }
        Ok(_) => &FILMICRGB_GRADIENT,
        Err(_) => &[],
    }
}

/// darktable's soften params, version 1, its only one: size, saturation, brightness and
/// amount (the mix).
fn soften_settings(xmp: &str) -> Result<soften::Settings> {
    let params = module_params(xmp, "soften")?;
    ensure!(
        params.version == 1,
        "soften params version {} is not supported",
        params.version
    );
    ensure!(
        params.bytes.len() == 16,
        "soften v1 params are {} bytes, expected 16",
        params.bytes.len()
    );
    let word = words(&params.bytes);
    let f = |i: usize| f32::from_le_bytes(word[i]);
    Ok(soften::Settings {
        size: f(0),
        saturation: f(1),
        brightness: f(2),
        amount: f(3),
    })
}

/// The parameters a soften case's gradient check moves (docs/ports/soften.md,
/// Gradients): the amount, which only `main` reads, so its derivative is the whole
/// render's (decision 11). The saturation and the brightness feed the whole-image
/// passes, whose result decision 15's derivatives of `main` hold fixed, and the size's
/// radius is a whole number, so neither is named.
const SOFTEN_GRADIENT: [&str; 1] = ["amount"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cases;

    #[test]
    fn tests_0158_and_0159_read_as_version_1_migrated() {
        let guided = cases::find("colorequal").unwrap();
        let unguided = cases::find("colorequal-no-filter").unwrap();
        for case in [guided, unguided] {
            assert_eq!(module_params(case.xmp, "colorequal").unwrap().version, 1);
        }
        let soften = |v: f32| 1.0 + 0.75 * (v - 1.0);
        let settings = colorequal_settings(guided.xmp).unwrap();
        assert!(settings.use_filter);
        assert_eq!((settings.chroma_size, settings.param_size), (2.0, 25.0));
        assert_eq!(settings.white_level, 1.0);
        assert_eq!(settings.threshold, 0.024f32 + 0.1);
        assert_eq!((settings.contrast, settings.hue_shift), (0.0, 0.0));
        assert_eq!(settings.saturation[0], soften(1.3297));
        assert_eq!(settings.brightness[7], soften(0.6816));
        assert_eq!(settings.hue[0], 42.749985);
        assert!(gradient_parameters(guided).is_empty());

        // The second of 0158's two entries.
        let settings = colorequal_settings(unguided.xmp).unwrap();
        assert!(!settings.use_filter);
        assert_eq!((settings.chroma_size, settings.param_size), (5.0, 5.0));
        assert_eq!(settings.white_level, f32::from_bits(0x3f66_6668));
        assert_eq!(settings.hue[1], -101.69);
        assert_eq!(settings.saturation[2], soften(0.4813));
        assert_eq!(gradient_parameters(unguided), &COLOREQUAL_GRADIENT[..3]);
        let values = plan_values(unguided).unwrap().unwrap();
        for (index, name) in COLOREQUAL_GRADIENT.iter().enumerate() {
            let moved = perturbed_values(unguided, index, 0.125).unwrap();
            assert_eq!(moved.float(name) - values.float(name), 0.125, "{name}");
        }
        assert!(perturbed_values(unguided, 4, 1e-3).is_err());
        assert!(colorequal_from_params(2, &[0; 128]).is_err());
    }

    #[test]
    fn test_0051_reads_as_its_soften() {
        let case = cases::find("soften").unwrap();
        assert_eq!(module_params(case.xmp, "soften").unwrap().version, 1);
        assert_eq!(
            soften_settings(case.xmp).unwrap(),
            soften::Settings {
                size: 33.0,
                saturation: 100.0,
                brightness: 0.5,
                amount: 75.0,
            }
        );
        assert_eq!(gradient_parameters(case), ["amount"]);
        let values = plan_values(case).unwrap().unwrap();
        let moved = perturbed_values(case, 0, 1e-3).unwrap();
        assert_ne!(moved, values);
        assert_eq!(moved.float("amount"), 75.0 + 1e-3);
        assert!(perturbed_values(case, 1, 1e-3).is_err());
    }

    #[test]
    fn test_0050_reads_as_its_bloom() {
        let case = cases::find("bloom").unwrap();
        assert_eq!(module_params(case.xmp, "bloom").unwrap().version, 1);
        assert_eq!(
            bloom_settings(case.xmp).unwrap(),
            bloom::Settings {
                size: 40.0,
                threshold: 85.0,
                strength: 70.0,
            }
        );
        assert!(gradient_parameters(case).is_empty());
        let values = plan_values(case).unwrap().unwrap();
        assert_ne!(perturbed_values(case, 0, 1e-3).unwrap(), values);
        assert!(perturbed_values(case, 1, 1e-3).is_err());
    }

    #[test]
    fn test_0002_reads_as_darktables_local_laplacian_defaults() {
        let case = cases::find("bilat").unwrap();
        assert_eq!(module_params(case.xmp, "bilat").unwrap().version, 3);
        assert_eq!(
            bilat_settings(case.xmp).unwrap(),
            bilat::Settings::default()
        );
        assert!(gradient_parameters(case).is_empty());
        let edited = cases::find("bilat-bilateral").unwrap();
        let settings = bilat_settings(edited.xmp).unwrap();
        assert_eq!(settings.mode, bilat::Mode::BilateralGrid);
        assert_eq!((settings.sigma_s, settings.sigma_r), (50.0, 20.0));
        assert_eq!(settings.detail, 1.0);
        assert_eq!(gradient_parameters(edited), ["detail"]);
        let values = plan_values(edited).unwrap().unwrap();
        assert_ne!(perturbed_values(edited, 0, 1e-3).unwrap(), values);
    }

    #[test]
    fn bilats_old_versions_migrate_as_darktables_legacy_params() {
        let bytes =
            |values: &[f32]| -> Vec<u8> { values.iter().flat_map(|v| v.to_le_bytes()).collect() };
        let v1 = bilat_from_params(1, &bytes(&[20.0, 50.0, 0.25])).unwrap();
        assert_eq!(
            v1,
            bilat::Settings {
                mode: bilat::Mode::BilateralGrid,
                sigma_r: 20.0,
                sigma_s: 50.0,
                detail: 0.25,
                midtone: 0.2,
            }
        );
        let mut v2 = 1_u32.to_le_bytes().to_vec();
        v2.extend(bytes(&[0.5, 0.6, 0.7]));
        assert_eq!(
            bilat_from_params(2, &v2).unwrap(),
            bilat::Settings {
                mode: bilat::Mode::LocalLaplacian,
                sigma_r: 0.5,
                sigma_s: 0.6,
                detail: 0.7,
                midtone: 0.2,
            }
        );
        assert!(bilat_from_params(3, &v2).is_err());
        assert!(bilat_from_params(4, &[]).is_err());
    }

    #[test]
    fn darktable_defaults_match_the_scene_referred_default() {
        // Test 0186's AgX entry holds darktable's scene-referred defaults.
        let case = cases::find("agx-highlights").unwrap();
        assert_eq!(
            agx_settings(case.xmp).unwrap(),
            agx::Settings::scene_referred_default()
        );
    }

    #[test]
    fn unreleased_versions_become_the_defaults() {
        let case = cases::find("agx-default").unwrap();
        assert_eq!(module_params(case.xmp, "agx").unwrap().version, 4);
        assert_eq!(
            agx_settings(case.xmp).unwrap(),
            agx::Settings::scene_referred_default()
        );
    }

    #[test]
    fn version_8_is_refused() {
        let case = cases::find("agx-highlights").unwrap();
        let xmp = case
            .xmp
            .replace("darktable:modversion=\"7\"", "darktable:modversion=\"8\"");
        assert!(agx_settings(&xmp).is_err());
    }

    #[test]
    fn agx_v7_params_round_trip() {
        // darktable's own bytes for its defaults, test 0186's entry, come back unchanged.
        let case = cases::find("agx-highlights").unwrap();
        let darktable = module_params(case.xmp, "agx").unwrap();
        assert_eq!(darktable.version, 7);
        assert_eq!(
            agx_params_v7(&agx_settings_v7(&darktable.bytes).unwrap()),
            darktable.bytes
        );
        // Settings off their defaults in every kind of field come back as they were.
        let moved = agx::Settings {
            look_slope: 1.25,
            auto_gamma: true,
            base_primaries: BasePrimaries::DisplayP3,
            inset: [0.1, 0.2, 0.3],
            unrotation: [-0.01, 0.02, -0.03],
            completely_reverse_primaries: true,
            ..agx::Settings::scene_referred_default()
        };
        let bytes = agx_params_v7(&moved);
        assert_eq!(bytes.len(), 144);
        assert_eq!(agx_settings_v7(&bytes).unwrap(), moved);
    }

    #[test]
    fn rgbcurve_test_0060_reads_as_darktable_stores_it() {
        let settings = rgbcurve_settings(cases::find("rgbcurve").unwrap().xmp).unwrap();
        assert_eq!(settings.mode, Mode::Independent);
        assert!(settings.compensate_middle_grey);
        assert_eq!(settings.preserve_colors, Norm::Luminance);
        let counts = settings.curves.each_ref().map(|curve| curve.nodes.len());
        assert_eq!(counts, [9, 5, 5]);
        for curve in &settings.curves {
            assert_eq!(curve.interpolation, Interpolation::MonotoneHermite);
        }
        let first = settings.curves[0].nodes[0];
        assert!(
            (first.x - 0.056114).abs() < 1e-6 && first.y == 0.0,
            "{first:?}"
        );
        assert_eq!(
            settings.curves[2].nodes[4],
            rgbcurve::Node { x: 1.0, y: 1.0 }
        );
    }

    #[test]
    fn every_rgbcurve_case_has_kernel_inputs_and_gradient_slots() {
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "rgbcurve")
        {
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params.len(), rgbcurve::PARAMS_SIZE, "{}", case.name);
            assert_eq!(gradient_parameters(case).len(), 4, "{}", case.name);
            // Every slot names a knot the case's curve has.
            let prepared = rgbcurve_prepare(case).unwrap();
            for slot in prepared.gradient {
                assert!(
                    slot.node < prepared.curves[slot.curve].knots.len(),
                    "{}",
                    case.name
                );
            }
        }
    }

    #[test]
    fn rgbcurve_perturbs_one_kernel_value() {
        let case = cases::find("rgbcurve").unwrap();
        let base = kernel_inputs(case).unwrap().unwrap();
        for index in 0..4 {
            let moved = perturbed_inputs(case, index, 1e-3).unwrap();
            assert_ne!(moved, base, "slot {index}");
        }
    }

    #[test]
    fn rgblevels_test_0054_reads_as_darktable_stores_it() {
        let settings = rgblevels_settings(cases::find("rgblevels").unwrap().xmp).unwrap();
        assert_eq!(settings.mode, rgblevels::Mode::Linked);
        assert_eq!(settings.preserve_colors, RgbNorm::Luminance);
        let red = settings.levels[0];
        assert!(
            (red.black - 0.011636).abs() < 1e-6
                && (red.grey - 0.355420).abs() < 1e-6
                && (red.white - 0.6208815).abs() < 1e-6,
            "{red:?}"
        );
        // Test 0055 has two entries, linked then independent; the second is active.
        let independent =
            rgblevels_settings(cases::find("rgblevels-independent").unwrap().xmp).unwrap();
        assert_eq!(independent.mode, rgblevels::Mode::Independent);
        assert_eq!(independent.levels, settings.levels);
    }

    #[test]
    fn every_rgblevels_case_has_kernel_inputs_and_its_gradient() {
        let mut norms = Vec::new();
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "rgblevels")
        {
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params.len(), rgblevels::PARAMS_SIZE, "{}", case.name);
            let settings = rgblevels_settings(case.xmp).unwrap();
            let expected = match settings.mode {
                rgblevels::Mode::Linked => 3,
                rgblevels::Mode::Independent => 4,
            };
            assert_eq!(gradient_parameters(case).len(), expected, "{}", case.name);
            for index in 0..expected {
                let moved = perturbed_inputs(case, index, 1e-3).unwrap();
                assert_ne!(moved, inputs, "{} slot {index}", case.name);
            }
            if settings.mode == rgblevels::Mode::Linked {
                norms.push(settings.preserve_colors);
            }
        }
        // Every way of preserving colours has a case.
        norms.sort_by_key(|norm| norm.to_darktable());
        assert_eq!(norms, RgbNorm::ALL);
    }

    #[test]
    fn vignette_test_0045_reads_as_darktable_stores_it() {
        let settings = vignette_settings(cases::find("vignette").unwrap().xmp).unwrap();
        assert_eq!(
            settings,
            vignette::Settings {
                scale: 69.4,
                falloff_scale: 32.08,
                brightness: -0.968,
                saturation: -0.82,
                center: [-0.116, 0.13],
                autoratio: false,
                whratio: 1.172,
                shape: 1.49,
                // Test 0045 dithers at 8 bits; the case has dithering off.
                dithering: vignette::Dithering::Off,
                unbound: true,
            }
        );
        let clip = vignette_settings(cases::find("vignette-clip").unwrap().xmp).unwrap();
        assert!(!clip.unbound && clip.brightness > 0.0);
    }

    #[test]
    fn every_vignette_case_has_kernel_inputs_and_its_gradient() {
        let mut checked = Vec::new();
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "vignette")
        {
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params.len(), vignette::PARAMS_SIZE, "{}", case.name);
            let gradient = vignette_gradient(case);
            assert_eq!(gradient_parameters(case), gradient.names, "{}", case.name);
            for (index, name) in gradient.names.iter().enumerate() {
                assert_eq!(gradient.slots[index].name(), *name, "{}", case.name);
                let moved = perturbed_inputs(case, index, 1e-3).unwrap();
                assert_ne!(moved, inputs, "{} slot {index}", case.name);
                checked.push(gradient.slots[index]);
            }
        }
        // Every continuous parameter's gradient is checked on some case, but the
        // saturation's, which the check's loss cannot see.
        for parameter in Parameter::ALL {
            assert_eq!(
                checked.contains(&parameter),
                parameter != Parameter::Saturation,
                "{parameter:?}"
            );
        }
    }

    #[test]
    fn sigmoid_cases_read_as_darktable_stores_them() {
        let settings = |name: &str| sigmoid_settings(cases::find(name).unwrap().xmp).unwrap();
        let version = |name: &str| {
            module_params(cases::find(name).unwrap().xmp, "sigmoid")
                .unwrap()
                .version
        };
        // Test 0097: version 1, migrated with the primaries unmodified.
        let test_0097 = settings("sigmoid");
        assert_eq!(version("sigmoid"), 1);
        assert_eq!(
            test_0097,
            sigmoid::Settings {
                middle_grey_contrast: f32::from_bits(0x3fe4_dd30),
                contrast_skewness: f32::from_bits(0x3eb8_51ec),
                hue_preservation: f32::from_bits(0x424a_7ae2),
                ..sigmoid::Settings::default()
            }
        );
        // Test 0151 has two entries; the second, with no hue preservation, is active.
        assert_eq!(
            settings("sigmoid-no-hue-preservation"),
            sigmoid::Settings {
                hue_preservation: 0.0,
                ..test_0097
            }
        );
        // Test 0182 stores darktable's defaults as version 3.
        assert_eq!(version("sigmoid-default"), 3);
        assert_eq!(settings("sigmoid-default"), sigmoid::Settings::default());
        let ratio = settings("sigmoid-rgb-ratio");
        assert_eq!(ratio.color_processing, ColorProcessing::RgbRatio);
        assert_eq!(ratio.base_primaries, sigmoid::BasePrimaries::Srgb);
        let primaries = settings("sigmoid-primaries");
        assert_eq!(primaries.color_processing, ColorProcessing::PerChannel);
        assert_eq!(primaries.base_primaries, sigmoid::BasePrimaries::Srgb);
        assert!((primaries.purity - 0.4).abs() < 1e-6 && primaries.inset[2] == 0.2);
    }

    #[test]
    fn sigmoid_version_4_is_refused() {
        // darktable master's version 4 adds `white_target_from_output`; the reference release replaces it
        // with its defaults (tests 0183 and 0185).
        let case = cases::find("sigmoid-default").unwrap();
        let entry = |version: u32, params: &str| {
            format!("darktable:modversion=\"{version}\"\n      darktable:params=\"{params}\"")
        };
        let version_3 = "0000c03f000000000000c8426c09793c000000000000c842\
                         0000000000000000000000000000000000000000000000000000000000000000";
        let xmp = case.xmp.replace(
            &entry(3, version_3),
            &entry(4, &format!("{version_3}01000000")),
        );
        assert_ne!(xmp, case.xmp);
        assert!(sigmoid_settings(&xmp).is_err());
    }

    #[test]
    fn every_sigmoid_case_has_kernel_inputs_and_its_gradient() {
        let mut processings = Vec::new();
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "sigmoid")
        {
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params.len(), sigmoid::PARAMS_SIZE, "{}", case.name);
            let settings = sigmoid_settings(case.xmp).unwrap();
            // The RGB ratio's target black has a slot, but the check's loss cannot see it.
            let named = match settings.color_processing {
                ColorProcessing::PerChannel => 4,
                ColorProcessing::RgbRatio => 3,
            };
            assert_eq!(gradient_parameters(case).len(), named, "{}", case.name);
            for index in 0..4 {
                let moved = perturbed_inputs(case, index, 1e-3).unwrap();
                assert_ne!(moved, inputs, "{} slot {index}", case.name);
            }
            // Through the operation, from schema values, as the engine would run it.
            let operation = sigmoid::Sigmoid;
            let params = operation
                .schema()
                .resolve(&sigmoid::values(&settings))
                .unwrap();
            let prepared = operation
                .prepare(&params, &ImageContext::full(1, 1), &ImageDesc::rgba(1, 1))
                .unwrap();
            assert_eq!(prepared.passes[0].params, inputs.params, "{}", case.name);
            processings.push(settings.color_processing);
        }
        // Both colour processings have a case.
        assert!(processings.contains(&ColorProcessing::PerChannel));
        assert!(processings.contains(&ColorProcessing::RgbRatio));
    }

    #[test]
    fn haze_removal_reads_every_params_version() {
        // Tests 0026 (version 1) and 0166 (version 2), as darktable stores them.
        let values = |name: &str| {
            let case = cases::find(name).unwrap();
            hazeremoval_settings(case.xmp).unwrap()
        };
        let v1 = module_params(cases::find("hazeremoval").unwrap().xmp, "hazeremoval").unwrap();
        assert_eq!(v1.version, 1);
        assert_eq!(
            values("hazeremoval"),
            hazeremoval::Settings {
                strength: 0.2,
                distance: 0.2
            }
        );
        assert_eq!(
            values("hazeremoval-v2"),
            hazeremoval::Settings {
                strength: 0.9,
                distance: 0.8
            }
        );
        // Version 3, with both flags, reads the same sliders.
        let case = cases::find("hazeremoval").unwrap();
        let v3: Vec<u8> = [
            0.4f32.to_le_bytes(),
            0.6f32.to_le_bytes(),
            [0; 4],
            [1, 0, 0, 0],
        ]
        .concat();
        let xmp = crate::xmp::set_params_and_version(case.xmp, "hazeremoval", 3, &v3).unwrap();
        let settings = hazeremoval_settings(&xmp).unwrap();
        assert_eq!((settings.strength, settings.distance), (0.4, 0.6));
        let v4 = crate::xmp::set_params_and_version(case.xmp, "hazeremoval", 4, &v3).unwrap();
        assert!(hazeremoval_settings(&v4).is_err());
        // Both cases run as one-instance plans, with the kernel's two gradient slots.
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "hazeremoval")
        {
            assert!(plan_values(case).unwrap().is_some());
            assert!(kernel_inputs(case).unwrap().is_none());
            assert_eq!(gradient_parameters(case), ["strength", "distance"]);
        }
    }

    #[test]
    fn every_agx_case_has_kernel_inputs() {
        for case in cases::CASES.iter().filter(|case| case.operation == "agx") {
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params.len(), agx::PARAMS_SIZE, "{}", case.name);
        }
    }

    #[test]
    fn grain_test_0059_reads_as_darktable_stores_it() {
        let case = cases::find("grain").unwrap();
        assert_eq!(module_params(case.xmp, "grain").unwrap().version, 2);
        let settings = grain_settings(case).unwrap();
        // 6400 ISO, darktable's coarsest grain, one ulp below the f32 of 6400 / 213.2.
        assert_eq!(settings.scale.to_bits(), 0x41f0_266c);
        assert_eq!(settings.scale.next_up(), grain::scale_range().1);
        assert_eq!((settings.strength, settings.midtones_bias), (100.0, 100.0));
        assert_eq!(settings.offset, 758);
    }

    #[test]
    fn grain_version_1_has_no_midtones_bias() {
        let case = cases::find("grain-v1").unwrap();
        assert_eq!(module_params(case.xmp, "grain").unwrap().version, 1);
        let settings = grain_settings(case).unwrap();
        assert_eq!(settings.midtones_bias, 0.0);
        assert_eq!(settings.strength, 64.5);
        assert!((f64::from(settings.scale) * grain::SCALE_FACTOR - 1150.0).abs() < 1e-3);
    }

    #[test]
    fn grain_channels_are_read_and_ignored() {
        // darktable's process() applies every channel to lightness, so the channel only
        // has to be one of its four.
        let case = cases::find("grain").unwrap();
        let channel = |value: &str| {
            let xmp = case.xmp.replace(
                "darktable:params=\"020000006c26f0410000c842",
                &format!("darktable:params=\"{value}6c26f0410000c842"),
            );
            grain_settings(&Case {
                xmp: Box::leak(xmp.into_boxed_str()),
                ..*case
            })
        };
        let lightness = channel("02000000").unwrap();
        for hue_to_rgb in ["00000000", "01000000", "03000000"] {
            assert_eq!(channel(hue_to_rgb).unwrap(), lightness);
        }
        assert!(channel("04000000").is_err());
    }

    #[test]
    fn every_grain_case_has_kernel_inputs_and_its_gradient() {
        for case in cases::CASES.iter().filter(|case| case.operation == "grain") {
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params.len(), grain::PARAMS_SIZE, "{}", case.name);
            assert_eq!(inputs.tables, [grain::permutation_table()], "{}", case.name);
            let gradient = grain_gradient(case);
            assert_eq!(gradient_parameters(case), gradient.names, "{}", case.name);
            let checked: &[&str] = if case.name == "grain-fine" {
                &[]
            } else {
                &["strength"]
            };
            assert_eq!(gradient.names, checked, "{}", case.name);
            for (index, name) in gradient.names.iter().enumerate() {
                assert_eq!(gradient.slots[index].name(), *name, "{}", case.name);
                let moved = perturbed_inputs(case, index, 1e-3).unwrap();
                assert_ne!(moved, inputs, "{} slot {index}", case.name);
            }
        }
    }

    #[test]
    fn colorbalancergb_inputs_come_through_the_operation_as_from_the_setup() {
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "colorbalancergb")
        {
            let settings = color_balance_settings(case.xmp).unwrap();
            let setup =
                colorbalancergb::prepare(&settings, &WorkingProfile::linear_rec2020()).unwrap();
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(inputs.params, setup.params, "{}", case.name);
            assert_eq!(inputs.tables, [setup.table()], "{}", case.name);
            for index in 0..gradient_parameters(case).len() {
                assert_ne!(
                    perturbed_inputs(case, index, 1e-3).unwrap(),
                    inputs,
                    "{} slot {index}",
                    case.name
                );
            }
        }
    }

    #[test]
    fn each_colorbalancergb_case_keeps_its_saturation_formula() {
        // Test 0083 is params version 4, which darktable's legacy_params, and so ports.rs,
        // gives JzAzBz whatever the schema's default; test 0093 is version 5 with
        // darktable UCS, the default.
        for (name, version, formula) in [
            ("colorbalancergb", 4, SaturationFormula::JzAzBz),
            ("colorbalancergb-ucs", 5, SaturationFormula::Ucs),
        ] {
            let case = cases::find(name).unwrap();
            let params = module_params(case.xmp, "colorbalancergb").unwrap();
            assert_eq!(params.version, version, "{name}");
            let settings = color_balance_settings(case.xmp).unwrap();
            assert_eq!(settings.saturation_formula, formula, "{name}");
        }
        let ucs = color_balance_settings(cases::find("colorbalancergb-ucs").unwrap().xmp).unwrap();
        let near = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(near(ucs.chroma_global, 0.3) && near(ucs.saturation_global, 0.3));
        assert!(near(ucs.saturation_shadows, 0.5) && near(ucs.brilliance_highlights, 0.1));
        assert!(near(ucs.vibrance, 0.2) && ucs.contrast == 0.0);
    }

    #[test]
    fn colorbalancergb_version_1_divides_global_saturation_by_100() {
        // Test 0083's version 4 fields, cut to version 1's 24, as darktable reads them.
        let case = cases::find("colorbalancergb").unwrap();
        let v4 = module_params(case.xmp, "colorbalancergb").unwrap();
        assert_eq!(v4.version, 4);
        let xmp =
            crate::xmp::set_params_and_version(case.xmp, "colorbalancergb", 1, &v4.bytes[..24 * 4])
                .unwrap();
        let (old, new) = (
            color_balance_settings(case.xmp).unwrap(),
            color_balance_settings(&xmp).unwrap(),
        );
        assert_eq!(new.saturation_global, old.saturation_global / 100.0);
        assert_eq!(new.hue_angle, old.hue_angle);
        assert_eq!(new.brilliance_global, 0.0);
        assert_eq!(new.contrast, 0.0);
        assert_eq!(new.grey_fulcrum, 0.1845);
        assert_eq!(new.saturation_formula, SaturationFormula::JzAzBz);
    }

    #[test]
    fn channelmixerrgb_test_0085_reads_as_darktable_stores_it() {
        use apertrust_color::{Adaptation, Illuminant};
        let case = cases::find("channelmixerrgb").unwrap();
        // Two entries of one instance; the second is active.
        assert_eq!(
            crate::xmp::all_params(case.xmp, "channelmixerrgb")
                .unwrap()
                .len(),
            2
        );
        let s = channelmixerrgb_settings(case.xmp).unwrap();
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|c| (a[c] - b[c]).abs() < 1e-6);
        assert!(near(s.red, [-0.969, 0.476, 0.0]), "{:?}", s.red);
        assert!(near(s.green, [0.0, -0.479, 0.0]) && near(s.blue, [0.0, 0.0, 1.0]));
        assert!(near(s.saturation, [0.215, -0.953, -0.544]));
        assert!(near(s.lightness, [0.184, -0.305, 0.138]));
        assert_eq!(s.grey, [0.0; 3]);
        assert!(s.normalize_r && s.normalize_g && s.normalize_b);
        assert!(!s.normalize_sat && !s.normalize_light && !s.normalize_grey);
        assert_eq!(
            (s.illuminant, s.adaptation, s.version),
            (
                Illuminant::Custom,
                Adaptation::Cat16,
                channelmixerrgb::Version::V3
            )
        );
        assert!((s.x - 0.3098125).abs() < 1e-6 && (s.y - 0.3276207).abs() < 1e-6);
        assert_eq!((s.gamut, s.clip), (1.0, true));
    }

    #[test]
    fn every_channelmixerrgb_case_has_kernel_inputs_and_its_gradient() {
        use apertrust_color::{Adaptation, Illuminant};
        let mut adaptations = Vec::new();
        for case in cases::CASES
            .iter()
            .filter(|case| case.operation == "channelmixerrgb")
        {
            let settings = channelmixerrgb_settings(case.xmp).unwrap();
            adaptations.push(settings.adaptation);
            // The camera's illuminant needs the raw, which `--ignored` tests read.
            if settings.illuminant == Illuminant::Camera {
                continue;
            }
            let inputs = kernel_inputs(case).unwrap().unwrap();
            assert_eq!(
                inputs.params.len(),
                channelmixerrgb::PARAMS_SIZE,
                "{}",
                case.name
            );
            let names = gradient_parameters(case);
            let expected: &[&str] = if settings.adaptation == Adaptation::Rgb {
                &[]
            } else {
                &["x", "y"]
            };
            assert_eq!(names, expected, "{}", case.name);
            for index in 0..names.len() {
                let moved = perturbed_inputs(case, index, 1e-3).unwrap();
                assert_ne!(moved, inputs, "{} slot {index}", case.name);
            }
        }
        // Every adaptation has a case.
        for adaptation in Adaptation::ALL {
            assert!(adaptations.contains(&adaptation), "{adaptation:?}");
        }
    }

    #[test]
    fn channelmixerrgb_v3_params_round_trip() {
        use apertrust_color::{Adaptation, Illuminant};
        // Every case's settings, then each field a value of its own, so a field written
        // in another's place shows.
        let mut all: Vec<_> = cases::CASES
            .iter()
            .filter(|case| case.operation == "channelmixerrgb")
            .map(|case| channelmixerrgb_settings(case.xmp).unwrap())
            .collect();
        let at = |i: usize| [0, 1, 2].map(|c| (3 * i + c) as f32 / 64.0);
        all.push(channelmixerrgb::Settings {
            red: at(0),
            green: at(1),
            blue: at(2),
            saturation: at(3),
            lightness: at(4),
            grey: at(5),
            normalize_r: true,
            normalize_g: false,
            normalize_b: true,
            normalize_sat: false,
            normalize_light: true,
            normalize_grey: false,
            illuminant: Illuminant::Led,
            illum_fluo: 7,
            illum_led: 3,
            adaptation: Adaptation::Xyz,
            x: 0.31,
            y: 0.32,
            temperature: 4321.0,
            gamut: 2.5,
            clip: false,
            version: channelmixerrgb::Version::V2,
        });
        let case = cases::find("channelmixerrgb").unwrap();
        for settings in all {
            let bytes = channel_mixer_params_v3(&settings);
            assert_eq!(bytes.len(), 160);
            let xmp =
                crate::xmp::set_params_and_version(case.xmp, "channelmixerrgb", 3, &bytes).unwrap();
            assert_eq!(channelmixerrgb_settings(&xmp).unwrap(), settings);
        }
    }

    #[test]
    #[ignore = "needs the darktable integration-test images (APERTRUST_TEST_IMAGES)"]
    fn the_camera_case_finds_darktables_illuminant() {
        // From rawler's decode of mire1-xtrans.raf, the chromaticity darktable's
        // defaults stored for it in test 0165.
        let case = cases::find("channelmixerrgb-camera").unwrap();
        let prepared = channelmixerrgb_prepare(case).unwrap();
        assert!(
            (prepared.x - 0.3892037).abs() < 2e-7 && (prepared.y - 0.41662326).abs() < 2e-7,
            "{} {}",
            prepared.x,
            prepared.y
        );
        assert_eq!(prepared.adaptation, apertrust_color::Adaptation::Cat16);
    }

    #[test]
    fn every_toneequal_case_is_darktables_test_in_its_own_order() {
        use toneequal::{Estimator, Filter};
        // The bands, -8 EV to 0 EV, as the spec records them: tests 0131, 0133 and 0134;
        // 0080; 0129.
        let max = [
            -1.283, -0.788, 0.445, 1.148, 1.321, 0.994, 0.211, -0.449, -0.755,
        ];
        let eigf = [
            0.4504, 1.2911, 1.8415, 1.5761, 0.9981, 0.5974, -0.2831, -0.9871, -1.1983,
        ];
        let average = [
            0.4496, 1.2919, 1.8429, 1.5910, 1.0840, 0.7308, -0.2076, -1.0943, -1.3645,
        ];
        let sqrt2 = std::f32::consts::SQRT_2;
        // Name, bands, filter, estimator, iterations, and blending, feathering, contrast
        // and exposure compensation, quantisation and smoothing; the first three and
        // quantisation unused, and not checked, without a filter.
        let expected = [
            (
                "toneequal-max",
                max,
                Filter::None,
                Estimator::Value,
                1,
                [5.0, 1.0, 0.0, 0.13, 0.0, sqrt2],
            ),
            (
                "toneequal-geomean",
                max,
                Filter::None,
                Estimator::GeometricMean,
                1,
                [5.0, 1.0, 0.0, 0.15, 0.0, sqrt2],
            ),
            (
                "toneequal-smoothing",
                max,
                Filter::None,
                Estimator::Value,
                1,
                [5.0, 1.0, 0.0, 0.13, 0.0, 1.2],
            ),
            (
                "toneequal-eigf",
                eigf,
                Filter::Eigf,
                Estimator::EuclideanNorm,
                1,
                [5.0, 1.0, 0.79, -0.51, 0.0, sqrt2],
            ),
            (
                "toneequal-average",
                average,
                Filter::AveragedEigf,
                Estimator::Average,
                1,
                [3.0, 3.0, 0.79, 0.13, 0.0, sqrt2],
            ),
            (
                "toneequal-power",
                max,
                Filter::Eigf,
                Estimator::PowerNorm,
                1,
                [8.0, 3.0, 0.27, 0.15, 0.0, sqrt2],
            ),
            (
                "toneequal-eigf-iterations",
                eigf,
                Filter::Eigf,
                Estimator::EuclideanNorm,
                5,
                [2.0, 20.0, 0.0, -1.57, 0.0, sqrt2],
            ),
        ];
        let all: Vec<&str> = cases::CASES
            .iter()
            .filter(|case| case.operation == "toneequal")
            .map(|case| case.name)
            .collect();
        assert_eq!(all, expected.map(|e| e.0));
        for (name, bands, filter, method, iterations, numbers) in expected {
            let case = cases::find(name).unwrap();
            // darktable runs the module where Apertrust does, before colorin (decision
            // 22): the tests' own legacy order, with colorin's input profile linear
            // Rec2020, an identity, so the module sees camera RGB on both sides.
            assert!(
                case.xmp.contains("darktable:iop_order_version=\"1\"")
                    && !case.xmp.contains("darktable:iop_order_list"),
                "{name}"
            );
            // colorin's first field, the input profile: DT_COLORSPACE_LIN_REC2020. Test
            // 0080 stores version 6, version 7's layout, which legacy_params copies.
            let colorin = module_params(case.xmp, "colorin").unwrap();
            assert!(matches!(colorin.version, 6 | 7), "{name}");
            assert_eq!(colorin.bytes[..4], 4_i32.to_le_bytes(), "{name}");
            let settings = toneequal_settings(case.xmp).unwrap();
            assert_eq!(
                (settings.details, settings.method, settings.iterations),
                (filter, method, iterations),
                "{name}"
            );
            let ours = [
                settings.blending,
                settings.feathering,
                settings.contrast_boost,
                settings.exposure_boost,
                settings.quantization,
                settings.smoothing,
            ];
            // Without a filter only the mask exposure and the smoothing are read.
            let read = |index: usize| filter != Filter::None || index >= 3;
            for (index, (ours, theirs)) in ours.into_iter().zip(numbers).enumerate() {
                if !read(index) {
                    continue;
                }
                assert!(
                    (ours - theirs).abs() < 1e-5 * theirs.abs().max(1.0),
                    "{name}: {ours} against {theirs}"
                );
            }
            for (ours, theirs) in settings.bands.iter().zip(bands) {
                assert!((ours - theirs).abs() < 5e-4, "{name}: {:?}", settings.bands);
            }
            // Every case runs as a one-instance plan, its values the settings'.
            assert!(kernel_inputs(case).unwrap().is_none(), "{name}");
            let values = plan_values(case).unwrap().unwrap();
            let schema = toneequal::ToneEqual.schema();
            assert_eq!(
                toneequal::settings(&schema.resolve(&values).unwrap()),
                settings,
                "{name}"
            );
            assert_eq!(gradient_parameters(case).len(), 4, "{name}");
            for index in 0..4 {
                let moved = perturbed_values(case, index, 1e-3).unwrap();
                assert_ne!(moved, values, "{name} direction {index}");
            }
        }
    }

    #[test]
    fn primaries_test_0152_reads_as_darktable_stores_it() {
        let case = cases::find("primaries").unwrap();
        let settings = primaries_settings(case.xmp).unwrap();
        // The sidecar's hues in degrees, as darktable's sliders show them.
        let degrees = [
            settings.achromatic_tint_hue,
            settings.red_hue,
            settings.green_hue,
            settings.blue_hue,
        ]
        .map(|v| (v.to_degrees() * 10.0).round() / 10.0);
        assert_eq!(degrees, [33.2, 9.5, -8.2, 13.1]);
        let purities = [
            settings.achromatic_tint_purity,
            settings.red_purity,
            settings.green_purity,
            settings.blue_purity,
        ];
        assert_eq!(purities, [0.134, 0.843, 1.235, 0.82]);
    }

    #[test]
    fn primaries_inputs_come_through_the_operation_as_from_the_setup() {
        let case = cases::find("primaries").unwrap();
        let settings = primaries_settings(case.xmp).unwrap();
        let setup = primaries::prepare(&settings, &RgbSpace::LINEAR_REC2020);
        let inputs = kernel_inputs(case).unwrap().unwrap();
        assert_eq!(inputs.params, setup.params());
        assert!(inputs.tables.is_empty());
        assert_eq!(gradient_parameters(case).len(), 4);
        for index in 0..4 {
            assert_ne!(
                perturbed_inputs(case, index, 1e-3).unwrap(),
                inputs,
                "slot {index}"
            );
        }
    }

    #[test]
    fn only_the_primaries_hues_and_purities_are_sweep_blind() {
        // The synthetic sweep skips them; the case checks all four slots.
        let mut blind = Vec::new();
        for case in cases::CASES {
            for name in gradient_parameters(case) {
                if let Some(reason) = sweep_blind(case, name) {
                    assert!(!reason.is_empty());
                    blind.push((case.name, *name));
                }
            }
        }
        assert_eq!(
            blind,
            [("primaries", "red_hue"), ("primaries", "green_purity")]
        );
    }

    fn filmic(name: &str) -> filmicrgb::Settings {
        filmicrgb_settings(cases::find(name).unwrap().xmp).unwrap()
    }

    #[test]
    fn filmicrgb_cases_read_at_their_versions() {
        use filmicrgb::{ColorScience, Norm, SplineVersion};
        let version = |name: &str| {
            module_params(cases::find(name).unwrap().xmp, "filmicrgb")
                .unwrap()
                .version
        };
        assert_eq!(version("filmicrgb"), 1);
        assert_eq!(version("filmicrgb-reconstruction"), 4);
        assert_eq!(version("filmicrgb-v6"), 5);
        for name in [
            "filmicrgb-v7",
            "filmicrgb-v3-nochroma",
            "filmicrgb-v4-nochroma",
            "filmicrgb-v5-nochroma",
            "filmicrgb-v6-euclidean",
        ] {
            assert_eq!(version(name), 6, "{name}");
        }
        let science = |name: &str| filmic(name).version;
        assert_eq!(science("filmicrgb"), ColorScience::V3);
        assert_eq!(science("filmicrgb-reconstruction"), ColorScience::V4);
        assert_eq!(science("filmicrgb-v6"), ColorScience::V6);
        assert_eq!(science("filmicrgb-v7"), ColorScience::V7);
        assert_eq!(science("filmicrgb-v3-nochroma"), ColorScience::V3);
        assert_eq!(science("filmicrgb-v4-nochroma"), ColorScience::V4);
        assert_eq!(science("filmicrgb-v5-nochroma"), ColorScience::V5);
        assert_eq!(science("filmicrgb-v6-euclidean"), ColorScience::V6);
        assert_eq!(
            filmic("filmicrgb-v6-euclidean").preserve_color,
            Norm::Euclidean
        );
        assert_eq!(filmic("filmicrgb-v5-nochroma").preserve_color, Norm::None);
        // Every migration ends in spline v3, version 5's already there.
        for case in cases::CASES.iter().filter(|c| c.operation == "filmicrgb") {
            assert_eq!(
                filmicrgb_settings(case.xmp).unwrap().spline_version,
                SplineVersion::V3,
                "{}",
                case.name
            );
        }
        // Version 5 gains highlight reconstruction on; version 6 says it.
        assert!(filmic("filmicrgb-v6").enable_highlight_reconstruction);
        assert!(!filmic("filmicrgb-v5-nochroma").enable_highlight_reconstruction);
        assert!(filmic("filmicrgb-reconstruction").enable_highlight_reconstruction);
    }

    #[test]
    fn filmicrgb_migrates_version_1_as_darktable_did_for_test_0139() {
        use filmicrgb::{ColorScience, CurveType, Noise, Norm, SplineVersion};
        // Test 0139's first filmicrgb entry is test 0014's version 1 settings as
        // darktable migrated them to version 6.
        let s = filmic("filmicrgb");
        let close = |a: f32, b: f32| (a - b).abs() <= 4.0 * f32::EPSILON * b.abs().max(1.0);
        assert!(close(s.latitude, 59.693_874), "{}", s.latitude);
        assert!(close(s.contrast, 1.087_291_2), "{}", s.contrast);
        assert!(close(s.balance, 29.234_432), "{}", s.balance);
        assert_eq!((s.black_point_target, s.white_point_target), (0.0, 100.0));
        assert_eq!(s.spline_version, SplineVersion::V3);
        assert_eq!(s.version, ColorScience::V3);
        assert_eq!(s.preserve_color, Norm::PowerNorm);
        assert_eq!(
            (s.shadows, s.highlights),
            (CurveType::Hard, CurveType::Soft)
        );
        assert_eq!(s.reconstruct_threshold, 6.0);
        assert_eq!(
            (s.noise_level, s.noise_distribution),
            (0.0, Noise::Gaussian)
        );
        assert_eq!(s.high_quality_reconstruction, 0);
        assert!(s.custom_grey && s.enable_highlight_reconstruction);
    }

    #[test]
    fn filmicrgb_refuses_unknown_versions_and_years() {
        assert!(filmicrgb_from_params(7, &[0; 120]).is_err());
        assert!(filmicrgb_from_params(6, &[0; 112]).is_err());
        // Version 4 with an internal version darktable does not know.
        let mut bytes = vec![0u8; 112];
        bytes[0..4].copy_from_slice(&18.45f32.to_le_bytes());
        bytes[4..8].copy_from_slice(&(-8.0f32).to_le_bytes());
        bytes[8..12].copy_from_slice(&4.0f32.to_le_bytes());
        bytes[48..52].copy_from_slice(&4.0f32.to_le_bytes());
        bytes[108..112].copy_from_slice(&2018i32.to_le_bytes());
        assert!(filmicrgb_from_params(4, &bytes).is_err());
        bytes[108..112].copy_from_slice(&2021i32.to_le_bytes());
        assert!(filmicrgb_from_params(4, &bytes).is_ok());
    }

    #[test]
    fn filmicrgb_gradients_name_the_saturation_off_v6_at_zero() {
        let names = |name: &str| gradient_parameters(cases::find(name).unwrap());
        assert_eq!(names("filmicrgb-v7"), FILMICRGB_GRADIENT);
        assert_eq!(names("filmicrgb-v6"), &FILMICRGB_GRADIENT[..3]);
        assert_eq!(names("filmicrgb-v6-euclidean"), &FILMICRGB_GRADIENT[..3]);
        assert_eq!(names("filmicrgb-reconstruction"), &FILMICRGB_GRADIENT[..2]);
        // Each direction moves its own parameter, the percentages as fractions.
        let case = cases::find("filmicrgb-v7").unwrap();
        let base = plan_values(case).unwrap().unwrap();
        for (index, name) in FILMICRGB_GRADIENT.iter().enumerate() {
            let direction = filmicrgb::DIRECTIONS[index];
            assert_eq!(direction.parameter(), *name);
            let moved = perturbed_values(case, index, 0.125).unwrap();
            assert_eq!(
                moved.float(name),
                base.float(name) + 0.125 * direction.scale(),
                "{name}"
            );
        }
    }
}
