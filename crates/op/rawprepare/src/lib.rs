// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2015-2025  darktable developers (ported from src/iop/rawprepare.c)
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

//! Raw black and white points (darktable's rawprepare), ported from darktable
//! (`src/iop/rawprepare.c`): crops the sensor's 16-bit mosaic to the developed area
//! and scales it so black is 0 and the white point 1, with a black level for each
//! position of the 2×2 pattern of row and column parities on the sensor. The first
//! operation of the raw stage; `docs/ports/rawprepare.md` is its spec.
//!
//! Not ported yet: the embedded DNG gain maps of flat-field correction, which rawler
//! does not read, and raws that are not 16-bit mosaics, which the engine does not
//! upload.

use apertrust_plugin::{
    Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation, Param,
    ParamValues, Prepared, Provenance, Schema, Slot, Stage, Strips, Value,
};

/// Generated from `slang/rawprepare.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "rawprepare",
    wgsl: include_str!("../generated/rawprepare.wgsl"),
    derivatives: false,
};

pub struct Rawprepare;

static MANIFEST: Manifest = Manifest {
    id: "rawprepare",
    version: 1,
    slot: Slot {
        stage: Stage::Raw,
        order: 1.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    radius: 0,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "rawprepare",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &["src/iop/rawprepare.c", "data/kernels/basic.cl"],
    }),
};

const fn level(name: &'static str, default: i64) -> Param {
    Param {
        name,
        kind: Kind::Int {
            default,
            min: 0,
            max: u16::MAX as i64,
        },
    }
}

/// The black level parameters, by parity position: row parity × 2 + column parity on
/// the sensor, darktable's `raw_black_level_separate[0]` to `[3]`.
pub const BLACK: [&str; 4] = [
    "raw_black_level_separate_0",
    "raw_black_level_separate_1",
    "raw_black_level_separate_2",
    "raw_black_level_separate_3",
];

/// darktable `dt_iop_rawprepare_params_t`, with its ranges. Without a camera, the
/// defaults are darktable's passthrough preset, no crop and the full 16-bit range,
/// since its introspection defaults put the white point at 0; [`Rawprepare::defaults`]
/// gives a raw's own.
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        level("left", 0),
        level("top", 0),
        level("right", 0),
        level("bottom", 0),
        level(BLACK[0], 0),
        level(BLACK[1], 0),
        level(BLACK[2], 0),
        level(BLACK[3], 0),
        level("raw_white_point", u16::MAX as i64),
        Param {
            name: "flat_field",
            kind: Kind::Choice {
                default: 0,
                choices: &["off", "embedded"],
            },
        },
    ],
};

/// The least a black level's divisor, the white point less it, may be: one level, what a
/// white point one above the black gives, the levels being whole numbers.
const MIN_DIVISOR: f32 = 1.0;

/// A level as darktable's parameters hold it: a whole number in 16 bits.
fn whole(value: f32) -> i64 {
    value.round().clamp(0.0, f32::from(u16::MAX)) as i64
}

impl Operation for Rawprepare {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults`: the decoder's crop, black levels and white point.
    /// Gain maps are never on by default, since rawler does not read them.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        let Some(camera) = &image.camera else {
            return values;
        };
        let [x, y, width, height] = camera.crop;
        let margins = [
            ("left", x),
            ("top", y),
            ("right", camera.width.saturating_sub(x + width)),
            ("bottom", camera.height.saturating_sub(y + height)),
        ];
        for (name, margin) in margins {
            values.set(name, Value::Int(i64::from(margin)));
        }
        let black = &camera.black;
        if black.width > 0 && black.height > 0 && !black.values.is_empty() {
            for (position, name) in BLACK.iter().enumerate() {
                let (column, row) = (position as u32 & 1, position as u32 >> 1);
                let at = (row % black.height) * black.width + column % black.width;
                let value = black.values.get(at as usize).copied().unwrap_or(0.0);
                values.set(name, Value::Int(whole(value)));
            }
        }
        values.set("raw_white_point", Value::Int(whole(camera.white)));
        values
    }

    fn prepare(
        &self,
        params: &ParamValues,
        _image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let not_ported = |what| NotPorted {
            operation: "rawprepare",
            what,
        };
        if input.format != Format::Mosaic16 {
            return Err(not_ported("raws that are not 16-bit mosaics"));
        }
        // The kernel reads the whole readout, as the engine uploads it.
        if input.sensor_origin != [0, 0] {
            return Err(not_ported("a mosaic that is not the whole readout"));
        }
        if params.choice("flat_field") != 0 {
            return Err(not_ported("flat-field correction from embedded gain maps"));
        }
        // Margins fit in 16 bits, so these sums cannot overflow.
        let margin = |name| params.int(name) as u32;
        let (left, top) = (margin("left"), margin("top"));
        // A crop that leaves nothing gives an empty output, which planning refuses.
        let output = ImageDesc {
            width: input.width.saturating_sub(left + margin("right")),
            height: input.height.saturating_sub(top + margin("bottom")),
            format: Format::Mosaic,
            sensor_origin: [left, top],
            // darktable sets the processed maximum to 1 here.
            maximum: [1.0; 3],
            ..*input
        };
        // darktable `commit_params`: subtract black, divide by white less black. A white
        // point at or below a black level would divide by 0 or less, which darktable
        // does, writing infinities, NaN or an inverted mosaic for every later operation;
        // the divisor is kept at least one level, the least a white above the black gives,
        // so the output is finite and anything else is darktable's bit for bit.
        let white = params.int("raw_white_point") as f32;
        let sub = BLACK.map(|name| params.int(name) as f32);
        let div = sub.map(|black| (white - black).max(MIN_DIVISOR));
        // `RawprepareParams` in the kernel.
        let mut bytes = Vec::with_capacity(48);
        for word in [left, top, input.width, 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in sub.iter().chain(&div) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let mut prepared = Prepared::per_pixel(input, bytes, Vec::new());
        prepared.output = output;
        // One photosite each, read where the crop puts it on the sensor (decision 14).
        prepared.passes[0].strips = Some(Strips::PER_PIXEL);
        Ok(prepared)
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::{Camera, Pattern};

    use super::*;

    /// A 40×30 sensor whose developed area is 32×24 at (3, 2).
    fn raw(black: Pattern<f32>, white: f32) -> ImageContext {
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 40,
                height: 30,
                cfa: Some(Pattern {
                    width: 2,
                    height: 2,
                    values: vec![0, 1, 1, 2],
                }),
                black,
                white,
                white_balance: None,
                xyz_to_camera: None,
                crop: [3, 2, 32, 24],
                declared: [3, 2, 32, 24],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(32, 24)
        }
    }

    fn uniform(value: f32) -> Pattern<f32> {
        Pattern {
            width: 1,
            height: 1,
            values: vec![value],
        }
    }

    const SENSOR: ImageDesc = ImageDesc::new(40, 30, Format::Mosaic16);

    /// The kernel's parameters: the crop's corner and the sensor width, then black and
    /// white less black for each parity position.
    fn kernel_params(prepared: &Prepared) -> ([u32; 4], [f32; 4], [f32; 4]) {
        let bytes = &prepared.passes[0].params;
        let word = |at: usize| bytes[at..at + 4].try_into().unwrap();
        (
            [0, 4, 8, 12].map(|at| u32::from_le_bytes(word(at))),
            [16, 20, 24, 28].map(|at| f32::from_le_bytes(word(at))),
            [32, 36, 40, 44].map(|at| f32::from_le_bytes(word(at))),
        )
    }

    fn prepare(image: &ImageContext, values: &ParamValues) -> Result<Prepared, NotPorted> {
        let params = SCHEMA
            .resolve_over(&Rawprepare.defaults(image), values)
            .unwrap();
        Rawprepare.prepare(&params, image, &SENSOR)
    }

    #[test]
    fn defaults_crop_to_the_developed_area_with_the_camera_levels() {
        let image = raw(uniform(1023.4), 13600.0);
        let prepared = prepare(&image, &ParamValues::new()).unwrap();
        assert_eq!(
            prepared.output,
            ImageDesc {
                sensor_origin: [3, 2],
                ..ImageDesc::new(32, 24, Format::Mosaic)
            }
        );
        let (corner, sub, div) = kernel_params(&prepared);
        assert_eq!(corner, [3, 2, 40, 0]);
        assert_eq!(sub, [1023.0; 4]);
        assert_eq!(div, [12577.0; 4]);
    }

    #[test]
    fn black_levels_follow_the_parity_positions() {
        // A pattern of the rows' parity, as darktable measures a 40D's.
        let black = Pattern {
            width: 2,
            height: 2,
            values: vec![1030.0, 1030.0, 1026.0, 1026.0],
        };
        let (_, sub, _) =
            kernel_params(&prepare(&raw(black, 13600.0), &ParamValues::new()).unwrap());
        assert_eq!(sub, [1030.0, 1030.0, 1026.0, 1026.0]);
        // A 1×2 pattern repeats across the columns.
        let columns = Pattern {
            width: 2,
            height: 1,
            values: vec![500.0, 510.0],
        };
        let (_, sub, _) =
            kernel_params(&prepare(&raw(columns, 4095.0), &ParamValues::new()).unwrap());
        assert_eq!(sub, [500.0, 510.0, 500.0, 510.0]);
    }

    #[test]
    fn without_a_camera_the_defaults_are_the_passthrough_preset() {
        let image = ImageContext::full(40, 30);
        let prepared = prepare(&image, &ParamValues::new()).unwrap();
        assert_eq!(prepared.output.width, 40);
        let (_, sub, div) = kernel_params(&prepared);
        assert_eq!(sub, [0.0; 4]);
        assert_eq!(div, [65535.0; 4]);
    }

    /// A white point at a black level, or below it, keeps the divisor at one level, so
    /// every photosite from 0 to 65535 scales to a finite value; one above it is
    /// darktable's divisor, white less black, unchanged.
    #[test]
    fn a_white_point_at_or_below_a_black_level_gives_finite_output() {
        let image = raw(uniform(1024.0), 4095.0);
        for (white, divisor) in [
            (1024, 1.0),
            (1000, 1.0),
            (0, 1.0),
            (1025, 1.0),
            (4095, 3071.0),
        ] {
            let mut values = ParamValues::new();
            values.set("raw_white_point", Value::Int(white));
            let (_, sub, div) = kernel_params(&prepare(&image, &values).unwrap());
            assert_eq!(div, [divisor; 4], "white {white}");
            for raw in [0.0, 1024.0, 65535.0] {
                for (sub, div) in sub.iter().zip(div) {
                    assert!(((raw - sub) / div).is_finite(), "white {white}, raw {raw}");
                }
            }
        }
        // Per position: a black above the white point keeps its own divisor at one level.
        let black = Pattern {
            width: 2,
            height: 2,
            values: vec![1000.0, 1000.0, 2000.0, 2000.0],
        };
        let (_, sub, div) =
            kernel_params(&prepare(&raw(black, 1500.0), &ParamValues::new()).unwrap());
        assert_eq!(sub, [1000.0, 1000.0, 2000.0, 2000.0]);
        assert_eq!(div, [500.0, 500.0, 1.0, 1.0]);
    }

    #[test]
    fn a_crop_that_leaves_nothing_gives_an_empty_output() {
        let mut values = ParamValues::new();
        values.set("right", Value::Int(37));
        let prepared = prepare(&raw(uniform(0.0), 4095.0), &values).unwrap();
        assert_eq!(prepared.output.width, 0);
    }

    #[test]
    fn gain_maps_and_other_inputs_are_not_ported() {
        let image = raw(uniform(0.0), 4095.0);
        let mut values = ParamValues::new();
        values.set("flat_field", Value::Choice(1));
        assert!(prepare(&image, &values).is_err());
        let params = SCHEMA
            .resolve_over(&Rawprepare.defaults(&image), &ParamValues::new())
            .unwrap();
        let floats = ImageDesc {
            format: Format::Mosaic,
            ..SENSOR
        };
        assert!(Rawprepare.prepare(&params, &image, &floats).is_err());
    }

    #[test]
    fn levels_are_whole_16_bit_numbers() {
        assert_eq!(whole(28.9), 29);
        assert_eq!(whole(-3.0), 0);
        assert_eq!(whole(1e9), 65535);
    }
}
