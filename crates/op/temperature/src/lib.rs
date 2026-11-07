// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/temperature.c)
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

//! White balance (darktable's temperature), ported from darktable
//! (`src/iop/temperature.c`): multiplies each photosite of the mosaic by its filter
//! colour's coefficient, the camera's as-shot white balance by default.
//! `docs/ports/temperature.md` is its spec.
//!
//! The preset is kept for the operations after demosaic: "as shot to reference",
//! darktable's scene-referred default, tells input colour to correct towards the
//! camera's D65 reference later. darktable's temperature and tint controls, its
//! colour picker and its camera presets belong to modules, not to this operation.

use apertrust_plugin::{
    Camera, Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation,
    Param, ParamValues, Prepared, Provenance, Schema, Slot, Stage, Strips, Value,
};

/// Generated from `slang/temperature.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "temperature",
    wgsl: include_str!("../generated/temperature.wgsl"),
    derivatives: false,
};

pub struct Temperature;

static MANIFEST: Manifest = Manifest {
    id: "temperature",
    version: 1,
    slot: Slot {
        stage: Stage::Raw,
        order: 3.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "temperature",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/temperature.c", "data/kernels/basic.cl"],
    }),
};

const fn coefficient(name: &'static str) -> Param {
    Param {
        name,
        kind: Kind::Float {
            default: 1.0,
            min: 0.0,
            max: 8.0,
        },
    }
}

/// darktable's presets, in its order.
pub const PRESETS: &[&str] = &["as_shot", "spot", "user", "d65", "d65_late"];
const AS_SHOT: u32 = 0;
const D65_LATE: u32 = 4;

/// darktable `dt_iop_temperature_params_t`, with its ranges. Without a camera, the
/// defaults are darktable's for an image that is not a raw: every coefficient 1, as
/// shot; [`Temperature::defaults`] gives a raw's own.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        coefficient("red"),
        coefficient("green"),
        coefficient("blue"),
        // The fourth colour of a four-colour sensor, unused on red, green and blue ones.
        coefficient("various"),
        Param {
            name: "preset",
            kind: Kind::Choice {
                default: AS_SHOT,
                choices: PRESETS,
            },
        },
    ],
};

/// The largest colour pattern the kernel holds: X-Trans's 6×6.
const MAX_PATTERN: usize = 36;

/// darktable `_find_coeffs`, normalised: the as-shot multipliers, else daylight's from
/// the camera's matrix, else darktable's last resort (`apertrust-color`).
fn as_shot(camera: &Camera) -> [f64; 4] {
    apertrust_color::as_shot_multipliers(
        camera.white_balance.as_deref(),
        camera.xyz_to_camera.as_ref(),
    )
}

impl Operation for Temperature {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's scene-referred `reload_defaults`: a colour raw gets its as-shot
    /// coefficients; a monochrome raw and an image that is not a raw keep every
    /// coefficient at 1. With a colour matrix the preset is "as shot to reference", so
    /// colorin returns the image to the camera's D65 reference and color calibration
    /// white balances it (decision 27); without one it is "as shot".
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        let Some(camera) = image.camera.as_ref().filter(|camera| camera.cfa.is_some()) else {
            return values;
        };
        let coefficients = as_shot(camera);
        for (name, coefficient) in ["red", "green", "blue", "various"].iter().zip(coefficients) {
            // Within the schema's range, so a strange camera cannot stop planning.
            let value = if coefficient.is_finite() {
                coefficient.clamp(0.0, 8.0)
            } else {
                1.0
            };
            values.set(name, Value::Float(value as f32));
        }
        let preset = if camera.xyz_to_camera.is_some() {
            D65_LATE
        } else {
            AS_SHOT
        };
        values.set("preset", Value::Choice(preset));
        values
    }

    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let not_ported = |what| NotPorted {
            operation: "temperature",
            what,
        };
        if input.format != Format::Mosaic {
            return Err(not_ported("images other than a prepared mosaic"));
        }
        let cfa = image
            .camera
            .as_ref()
            .and_then(|camera| camera.cfa.as_ref())
            .ok_or(not_ported("monochrome raws"))?;
        let size = (cfa.width * cfa.height) as usize;
        if size == 0 || size > MAX_PATTERN || cfa.values.len() != size {
            return Err(not_ported("colour patterns larger than X-Trans's"));
        }
        let coefficients = [
            params.float("red"),
            params.float("green"),
            params.float("blue"),
        ];
        // `TemperatureParams` in the kernel: the pattern's size, the mosaic's corner on
        // the sensor, then a coefficient per position of the pattern.
        let mut bytes = Vec::with_capacity(16 + 4 * MAX_PATTERN);
        let [x, y] = input.sensor_origin;
        for word in [cfa.width, cfa.height, x, y] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for position in 0..MAX_PATTERN {
            let coefficient = match cfa.values.get(position) {
                Some(&color) => *coefficients
                    .get(color as usize)
                    .ok_or(not_ported("colour filters other than red, green and blue"))?,
                None => 0.0,
            };
            bytes.extend_from_slice(&coefficient.to_le_bytes());
        }
        let mut prepared = Prepared::per_pixel(input, bytes, Vec::new());
        prepared.passes[0].strips = Some(Strips::PER_PIXEL);
        // darktable `_publish_chroma`: what later operations learn about the image.
        let output = &mut prepared.output;
        output.maximum = [0, 1, 2].map(|c| input.maximum[c] * coefficients[c]);
        output.white_balance = coefficients;
        output.late_correction = params.choice("preset") == D65_LATE;
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::Pattern;

    use super::*;

    fn camera(white_balance: Option<Vec<f32>>, matrix: Option<[[f32; 3]; 3]>) -> ImageContext {
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 12,
                height: 10,
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
                white: 4095.0,
                white_balance,
                xyz_to_camera: matrix,
                crop: [1, 1, 10, 8],
                declared: [1, 1, 10, 8],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(10, 8)
        }
    }

    const MATRIX: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 0.5]];

    fn resolved(image: &ImageContext) -> ParamValues {
        SCHEMA
            .resolve_over(&Temperature.defaults(image), &ParamValues::new())
            .unwrap()
    }

    fn rgb(values: &ParamValues) -> [f32; 3] {
        [
            values.float("red"),
            values.float("green"),
            values.float("blue"),
        ]
    }

    #[test]
    fn defaults_are_the_as_shot_coefficients_normalised_to_green() {
        let image = camera(Some(vec![2048.0, 1024.0, 1536.0]), Some(MATRIX));
        let values = resolved(&image);
        assert_eq!(rgb(&values), [2.0, 1.0, 1.5]);
        // As shot to reference, for color calibration to white balance (decision 27).
        assert_eq!(values.choice("preset"), D65_LATE);
        // Without a matrix there is no reference to return to: as shot.
        let values = resolved(&camera(Some(vec![2048.0, 1024.0, 1536.0]), None));
        assert_eq!(rgb(&values), [2.0, 1.0, 1.5]);
        assert_eq!(values.choice("preset"), AS_SHOT);
    }

    #[test]
    fn without_as_shot_coefficients_daylight_comes_from_the_matrix() {
        let values = resolved(&camera(None, Some(MATRIX)));
        // One over each row of the matrix times sRGB's D65 white, then over green's.
        let white = [0.950456, 1.0, 1.088754];
        let mul = [
            1.0 / white[0],
            1.0 / (2.0 * white[1]),
            1.0 / (0.5 * white[2]),
        ];
        let expected = [mul[0] / mul[1], 1.0, mul[2] / mul[1]];
        for (got, want) in rgb(&values).iter().zip(expected) {
            assert!((f64::from(*got) - want).abs() < 1e-5, "{got} != {want}");
        }
        // Nothing at all: darktable's last resort.
        assert_eq!(rgb(&resolved(&camera(None, None))), [2.0, 1.0, 1.5]);
    }

    #[test]
    fn an_image_that_is_not_a_raw_keeps_every_coefficient_at_one() {
        let values = resolved(&ImageContext::full(10, 8));
        assert_eq!(rgb(&values), [1.0; 3]);
        assert_eq!(values.choice("preset"), AS_SHOT);
    }

    #[test]
    fn each_position_of_the_pattern_gets_its_colours_coefficient() {
        let image = camera(Some(vec![2.0, 1.0, 1.5]), Some(MATRIX));
        let input = ImageDesc {
            sensor_origin: [1, 1],
            ..ImageDesc::new(10, 8, Format::Mosaic)
        };
        let prepared = Temperature
            .prepare(&resolved(&image), &image, &input)
            .unwrap();
        // The default, "as shot to reference", asks colorin for the late correction.
        assert_eq!(
            prepared.output,
            ImageDesc {
                maximum: [2.0, 1.0, 1.5],
                white_balance: [2.0, 1.0, 1.5],
                late_correction: true,
                ..input
            }
        );
        // "As shot" does not.
        let mut as_shot = ParamValues::new();
        as_shot.set("preset", Value::Choice(AS_SHOT));
        let params = SCHEMA
            .resolve_over(&Temperature.defaults(&image), &as_shot)
            .unwrap();
        let output = Temperature.prepare(&params, &image, &input).unwrap().output;
        assert!(!output.late_correction);
        let bytes = &prepared.passes[0].params;
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!([word(0), word(4), word(8), word(12)], [2, 2, 1, 1]);
        assert_eq!([16, 20, 24, 28].map(float), [2.0, 1.0, 1.0, 1.5]);
        assert_eq!(bytes.len(), 16 + 4 * MAX_PATTERN);
    }

    #[test]
    fn other_inputs_and_filters_are_not_ported() {
        let image = camera(Some(vec![2.0, 1.0, 1.5]), None);
        let params = resolved(&image);
        let mosaic16 = ImageDesc::new(10, 8, Format::Mosaic16);
        assert!(Temperature.prepare(&params, &image, &mosaic16).is_err());
        let mut cmyg = image.clone();
        if let Some(camera) = cmyg.camera.as_mut() {
            camera.cfa.as_mut().unwrap().values = vec![3, 1, 1, 3];
        }
        let mosaic = ImageDesc::new(10, 8, Format::Mosaic);
        assert!(Temperature.prepare(&params, &cmyg, &mosaic).is_err());
    }
}
