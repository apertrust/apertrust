// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c,
// src/iop/colorbalancergb.c, src/iop/agx.c, src/iop/toneequal.c and src/iop/vignette.c)
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

//! darktable's presets the built-in styles are made from, as `init_presets` sets them at
//! the reference release, in darktable's parameters, and each turned into the tool's
//! controls by the tool's own tables, so a test holds every built-in style to its preset
//! control by control.

use apertrust_tools::{ControlValue, ProcessingTool, TOOLS, agx, colour_balance_rgb};

/// A parameter's value as darktable's preset sets it.
#[derive(Clone, Copy, Debug)]
pub enum Dt {
    Number(f32),
    /// An enum's value, by the option's id the tool gives it.
    Choice(&'static str),
    Flag(bool),
}

/// One built-in style: its id, its tool's, and each parameter its darktable preset sets
/// off the module's defaults, or that the photo moves (Colour calibration's light), as
/// darktable's source computes it, in f32.
pub struct Preset {
    pub id: &'static str,
    pub tool: &'static str,
    pub params: Vec<(&'static str, Dt)>,
}

/// Colour calibration's monochrome presets: the light bypassed, the pipeline's D50 under
/// XYZ, no gamut compression, and the grey weights normalised (`channelmixerrgb.c`,
/// `init_presets`).
fn monochrome(id: &'static str, grey: [f32; 3]) -> Preset {
    use Dt::{Choice, Flag, Number};
    Preset {
        id,
        tool: "channelmixerrgb",
        params: vec![
            ("adaptation", Choice("xyz")),
            ("illuminant", Choice("pipeline")),
            ("temperature", Number(5003.0)),
            // `illuminant_to_xy(DT_ILLUMINANT_PIPE, …)`: `D50xyY`.
            ("x", Number(0.34567)),
            ("y", Number(0.35850)),
            ("gamut", Number(0.0)),
            ("grey_0", Number(grey[0])),
            ("grey_1", Number(grey[1])),
            ("grey_2", Number(grey[2])),
            ("normalize_grey", Flag(true)),
        ],
    }
}

/// Colour balance RGB's basic colourfulness: darktable UCS, its saturation graded
/// (`colorbalancergb.c`, `init_presets`).
fn colourfulness(id: &'static str, [global, shadows, highlights]: [f32; 3]) -> Preset {
    use Dt::Number;
    Preset {
        id,
        tool: "colorbalancergb",
        params: vec![
            ("saturation_global", Number(global)),
            ("saturation_shadows", Number(shadows)),
            ("saturation_highlights", Number(highlights)),
        ],
    }
}

/// AgX's blender-like punchy (`agx.c`, `_set_blenderlike_params` then `_make_punchy`):
/// Blender's curve, its contrast of 2.4 compensated for gamma 2.4 against 2.2, and a look
/// of power 1.35 and saturation 1.4.
fn punchy() -> Preset {
    use Dt::Number;
    let gamma = 2.4f32;
    let pivot_y = 0.18f32.powf(1.0 / gamma);
    // `_calculate_slope_gamma_compensation`, `_epsilon` 1e-6, `_default_gamma` 2.2.
    let at_default = 0.18f32.clamp(0.0, 1.0).powf(1.0 / 2.2);
    let compensation = (gamma * 1e-6f32.max(pivot_y).powf(gamma - 1.0))
        / (2.2 * 1e-6f32.max(at_default).powf(2.2 - 1.0));
    Preset {
        id: "punchy",
        tool: "agx",
        params: vec![
            ("curve_shoulder_power", Number(1.5)),
            ("curve_gamma", Number(gamma)),
            ("curve_contrast_around_pivot", Number(2.4 * compensation)),
            ("look_brightness", Number(1.0 / (1.35 * 1.35))),
            ("look_saturation", Number(1.4)),
        ],
    }
}

/// Tone equalizer's compress shadows/highlights, EIGF, medium (`toneequal.c`,
/// `init_presets` and `compress_shadows_highlight_preset_set_exposure_params` at 0.45).
fn compress_medium() -> Preset {
    use Dt::Number;
    let step = 0.45f32;
    Preset {
        id: "open-shadows-highlights",
        tool: "toneequal",
        params: vec![
            ("noise", Number(step)),
            ("ultra_deep_blacks", Number(5.0 / 3.0 * step)),
            ("deep_blacks", Number(5.0 / 3.0 * step)),
            ("blacks", Number(step)),
            ("midtones", Number(-step)),
            ("highlights", Number(-5.0 / 3.0 * step)),
            ("whites", Number(-5.0 / 3.0 * step)),
            ("speculars", Number(-step)),
            ("exposure_boost", Number(-1.57)),
            ("blending", Number(3.0)),
            ("feathering", Number(7.0)),
            ("iterations", Number(3.0)),
        ],
    }
}

/// Vignetting's lomo (`vignette.c`, `init_presets`).
fn lomo() -> Preset {
    use Dt::Number;
    Preset {
        id: "lomo",
        tool: "vignette",
        params: vec![
            ("scale", Number(40.0)),
            ("falloff_scale", Number(100.0)),
            ("brightness", Number(-1.0)),
            ("saturation", Number(0.5)),
        ],
    }
}

/// Every built-in style's preset, in the library's order.
pub fn presets() -> Vec<Preset> {
    vec![
        monochrome("bw-luminance", [0.0, 1.0, 0.0]),
        // ILFORD HP5+, and Fuji Acros 100.
        monochrome("bw-hp5", [0.25304098, 0.25958747, 0.48737156]),
        monochrome("bw-acros-100", [0.333, 0.313, 0.353]),
        colourfulness("natural-colour", [0.2, 0.30, -0.5]),
        colourfulness("vibrant-colour", [0.2, 0.5, -0.25]),
        punchy(),
        compress_medium(),
        lomo(),
    ]
}

/// The tool `id`.
pub fn tool(id: &str) -> &'static dyn ProcessingTool {
    *(TOOLS.iter())
        .find(|tool| tool.definition().tool.id == id)
        .expect("a tool")
}

/// darktable's parameter `param` of the tool `tool` as the tool's control shows it, by the
/// tool's own tables: Colour calibration's mixer by `PARAMS`, its light by name; Colour
/// balance RGB's and AgX's numbers by their `PARAMETERS`' scale; the tone equalizer's and
/// vignetting's by name, as they are.
pub fn control(tool_id: &str, param: &str, value: Dt) -> (&'static str, ControlValue) {
    let definition = tool(tool_id).definition();
    let (control, factor) = match tool_id {
        "channelmixerrgb" => {
            let mixer = (apertrust_tools::colour_calibration::PARAMS.iter())
                .find(|(_, p)| *p == param)
                .map(|(control, _)| *control);
            (mixer.unwrap_or(param), 1.0)
        }
        "colorbalancergb" => {
            let found = (colour_balance_rgb::PARAMETERS.iter())
                .find(|p| p.param == param)
                .expect("a parameter");
            (found.control, found.scale)
        }
        "agx" => {
            let found = (agx::PARAMETERS.iter())
                .find(|p| p.param == param)
                .expect("a parameter");
            (found.control, found.shown(1.0))
        }
        _ => (param, 1.0),
    };
    let found = definition.control(control).expect("a control");
    let value = match value {
        Dt::Number(number) => ControlValue::Number(number * factor),
        Dt::Flag(flag) => ControlValue::Bool(flag),
        Dt::Choice(option) => {
            let apertrust_tools::ControlKind::Choice { options, .. } = found.kind else {
                panic!("{control} is no choice");
            };
            let index = options
                .iter()
                .position(|o| *o == option)
                .expect("an option");
            ControlValue::Choice(index as u32)
        }
    };
    (found.id, value)
}
