// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/highlights.c and
// src/iop/hlreconstruct/opposed.c)
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

//! Highlight reconstruction (darktable's highlights), ported from darktable
//! (`src/iop/highlights.c`, `src/iop/hlreconstruct/opposed.c`): inpaint opposed,
//! darktable's default method. A clipped photosite takes the average of the colours
//! opposing its own around it, offset by a chrominance the whole mosaic gives, so
//! highlights keep their colour instead of turning magenta. The offset is Apertrust's
//! first reduction (decision 15). `docs/ports/highlights.md` is its spec.

use apertrust_plugin::{
    Binding, Domain, Format, ImageContext, ImageDesc, Kernel, Kind, Manifest, NotPorted, Operation,
    Param, ParamValues, Pass, PassKind, Pattern, Prepared, Provenance, Resource, Schema, Scratch,
    Slot, Stage, Strips, Value,
};

/// Generated from `slang/highlights.slang`.
pub const KERNEL: Kernel = Kernel {
    name: "highlights",
    wgsl: include_str!("../generated/highlights.wgsl"),
    derivatives: false,
};

pub struct Highlights;

static MANIFEST: Manifest = Manifest {
    id: "highlights",
    version: 1,
    slot: Slot {
        stage: Stage::Raw,
        order: 4.0,
    },
    domain: Domain::Scene,
    layers: false,
    differentiable: false,
    // The opposing averages' 3×3. The chrominance offset needs the whole mosaic, which
    // its reduction passes cover.
    radius: 1,
    default_enabled: true,
    provenance: Some(Provenance {
        darktable_module: "highlights",
        release: "5.6.2",
        commit: "20891f6c6f",
        sources: &[
            "src/iop/highlights.c",
            "src/iop/hlreconstruct/opposed.c",
            "src/iop/hlreconstruct/segbased.c",
        ],
    }),
};

/// darktable's methods, by their enum's value.
pub const METHODS: &[&str] = &["clip", "lch", "inpaint", "laplacian", "segments", "opposed"];
const OPPOSED: u32 = 5;

/// darktable `dt_iop_highlights_params_t` version 4, the fields inpaint opposed reads.
/// The other methods' parameters come with the methods (decision 51).
static SCHEMA: Schema = Schema {
    version: 1,
    params: &[
        Param {
            name: "mode",
            kind: Kind::Choice {
                default: OPPOSED,
                choices: METHODS,
            },
        },
        Param {
            name: "clip",
            kind: Kind::Float {
                default: 1.0,
                min: 0.0,
                max: 2.0,
            },
        },
    ],
};

/// darktable `highlights_clip_magics` for inpaint opposed: the clipping level sits
/// this far below the threshold.
const OPPOSED_CLIP: f32 = 0.987;

/// The kernel's workgroups, 16×16, and the photosites each invocation of the sums
/// pass adds up, 4×4.
const WORKGROUP: u32 = 16;
const BLOCK: u32 = 4;

/// The colour of the photosite at (col, row) of the mosaic: its sensor position's.
fn color(cfa: &Pattern<u32>, input: &ImageDesc, col: u32, row: u32) -> u32 {
    let x = (input.sensor_origin[0] + col) % cfa.width;
    let y = (input.sensor_origin[1] + row) % cfa.height;
    cfa.values[(y * cfa.width + x) as usize]
}

impl Operation for Highlights {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }

    fn schema(&self) -> &'static Schema {
        &SCHEMA
    }

    fn kernel(&self) -> &'static Kernel {
        &KERNEL
    }

    /// darktable's `reload_defaults`: the clipping threshold is the lower of 1 and a
    /// DNG's LinearResponseLimit, where the sensor stops responding linearly.
    fn defaults(&self, image: &ImageContext) -> ParamValues {
        let mut values = SCHEMA.defaults();
        if let Some(camera) = &image.camera {
            let limit = camera.linear_response_limit;
            if limit.is_finite() {
                values.set("clip", Value::Float(limit.clamp(0.0, 1.0)));
            }
        }
        values
    }

    /// darktable's `commit_params` and `_process_opposed` set-up: the clipping level of
    /// each colour, the late correction, and the masks' grid.
    fn prepare(
        &self,
        params: &ParamValues,
        image: &ImageContext,
        input: &ImageDesc,
    ) -> Result<Prepared, NotPorted> {
        let not_ported = |what| NotPorted {
            operation: "highlights",
            what,
        };
        if params.choice("mode") != OPPOSED {
            return Err(not_ported("methods other than inpaint opposed"));
        }
        if input.format != Format::Mosaic {
            return Err(not_ported("images other than a prepared mosaic"));
        }
        let camera = image.camera.as_ref();
        let cfa = camera
            .and_then(|camera| camera.cfa.as_ref())
            .ok_or(not_ported("monochrome raws"))?;
        let repeats = |size: u32| size > 0 && 6 % size == 0;
        if !repeats(cfa.width) || !repeats(cfa.height) {
            return Err(not_ported(
                "colour patterns other than Bayer's and X-Trans's",
            ));
        }
        let colors: Vec<u32> = (0..36)
            .map(|position| color(cfa, input, position % 6, position / 6))
            .collect();
        if colors.iter().any(|&c| c > 2) {
            return Err(not_ported("colour filters other than red, green and blue"));
        }
        let (width, height) = (input.width, input.height);
        // darktable's masks: a third of the mosaic, at least one superpixel.
        let (mask_width, mask_height) = (width / 3, height / 3);
        if mask_width == 0 || mask_height == 0 {
            return Err(not_ported("mosaics under 3×3 photosites"));
        }
        let mask_stride = mask_width.next_multiple_of(8);
        let mask_size = mask_stride * mask_height.next_multiple_of(8);
        let blocks = [width.div_ceil(BLOCK), height.div_ceil(BLOCK)];
        let partials_across = blocks[0].div_ceil(WORKGROUP);
        let partials = partials_across * blocks[1].div_ceil(WORKGROUP);

        let clip = params.float("clip") * OPPOSED_CLIP;
        let clips = [0, 1, 2].map(|c| clip * input.white_balance[c]);
        // The opposing averages weigh each colour by the late correction until colorin
        // applies it.
        let correction = camera
            .filter(|_| input.late_correction)
            .and_then(|camera| {
                let matrix = camera.xyz_to_camera.as_ref()?;
                Some(apertrust_color::late_correction(
                    camera.white_balance.as_deref(),
                    matrix,
                ))
            })
            .unwrap_or([1.0; 3]);

        // `HighlightsParams` in the kernel.
        let mut bytes = Vec::with_capacity(208);
        for word in [
            width,
            height,
            mask_width,
            mask_height,
            mask_size,
            mask_stride,
            partials_across,
            partials,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in clips
            .into_iter()
            .chain([0.0])
            .chain(correction)
            .chain([0.0])
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for color in &colors {
            bytes.extend_from_slice(&color.to_le_bytes());
        }

        let pass = |entry_point, kind, size, strips, bindings: &[(u32, Resource)]| Pass {
            entry_point,
            kind,
            params: bytes.clone(),
            tables: Vec::new(),
            bindings: [(2, Resource::Uniforms)]
                .iter()
                .chain(bindings)
                .map(|&(binding, resource)| Binding::new(binding, resource))
                .collect(),
            size,
            strips,
        };
        let input_binding = (0, Resource::Input);
        let output_binding = (1, Resource::Output);
        let [clipped, dilated, sums, chroma] =
            [0, 1, 2, 3].map(|i: usize| (10 + i as u32, Resource::Scratch(i)));
        // The mask runs over the grid's rows, a row of invocations to three rows of the
        // mosaic, so it can run in strips of them (decision 14); the dilation, which binds
        // only the masks, whole, over mask_size entries in rows of mask_stride, a row of
        // them about a row of the grid, so it can run in bands of as many rows as the
        // mask's strips, each entry depending only on the finished mask.
        let mask_rows = Some([mask_width, mask_size.div_ceil(mask_width)]);
        let mask_strips = Strips {
            rows: 3,
            granularity: 1,
            halo: 0,
        };
        // A row of the sums' invocations adds up four rows of the mosaic and their
        // neighbours' rows; its strips are whole rows of workgroups, which write their
        // partial results at their place in the whole image.
        let sum_strips = Strips {
            rows: BLOCK,
            granularity: WORKGROUP,
            halo: 1,
        };
        let inpaint_strips = Strips {
            halo: 1,
            ..Strips::PER_PIXEL
        };
        let passes = vec![
            pass(
                "opposed_mask",
                PassKind::Reduction,
                mask_rows,
                Some(mask_strips),
                &[input_binding, clipped],
            ),
            pass(
                "opposed_dilate",
                PassKind::Reduction,
                Some([mask_stride, mask_size / mask_stride]),
                Some(mask_strips),
                &[clipped, dilated],
            ),
            pass(
                "opposed_sums",
                PassKind::Reduction,
                Some(blocks),
                Some(sum_strips),
                &[input_binding, dilated, sums],
            ),
            // One workgroup.
            pass(
                "opposed_chroma",
                PassKind::Reduction,
                Some([WORKGROUP, WORKGROUP]),
                None,
                &[sums, chroma],
            ),
            pass(
                "opposed_inpaint",
                PassKind::Spatial { radius: 1 },
                None,
                Some(inpaint_strips),
                &[input_binding, output_binding, chroma],
            ),
        ];
        let scratch = [
            u64::from(mask_size) * 4,
            u64::from(mask_size) * 4,
            u64::from(partials) * 32,
            16,
        ]
        .map(|bytes| Scratch { bytes })
        .to_vec();
        // Inpaint opposed keeps the signal scene-referred: the saturation levels stay.
        Ok(Prepared {
            output: *input,
            passes,
            scratch,
        })
    }
}

#[cfg(test)]
mod tests {
    use apertrust_plugin::Camera;

    use super::*;

    const BAYER: [u32; 4] = [0, 1, 1, 2];

    fn raw(cfa: Pattern<u32>) -> ImageContext {
        ImageContext {
            camera: Some(Camera {
                make: String::new(),
                model: String::new(),
                alias: String::new(),
                width: 64,
                height: 48,
                cfa: Some(cfa),
                black: Pattern {
                    width: 1,
                    height: 1,
                    values: vec![0.0],
                },
                white: 4095.0,
                white_balance: Some(vec![2.0, 1.0, 1.5]),
                xyz_to_camera: Some([[0.5, 0.3, 0.2], [0.2, 0.7, 0.1], [0.0, 0.1, 0.9]]),
                crop: [0, 0, 64, 48],
                declared: [0, 0, 64, 48],
                exposure_bias: 0.0,
                iso: 0.0,
                linear_response_limit: 1.0,
                lens_correction: None,
                orientation: 0,
            }),
            ..ImageContext::full(64, 48)
        }
    }

    fn bayer() -> ImageContext {
        raw(Pattern {
            width: 2,
            height: 2,
            values: BAYER.to_vec(),
        })
    }

    fn mosaic(width: u32, height: u32) -> ImageDesc {
        ImageDesc {
            sensor_origin: [1, 0],
            white_balance: [2.0, 1.0, 1.5],
            maximum: [2.0, 1.0, 1.5],
            ..ImageDesc::new(width, height, Format::Mosaic)
        }
    }

    fn word(bytes: &[u8], index: usize) -> u32 {
        u32::from_le_bytes(bytes[4 * index..4 * index + 4].try_into().unwrap())
    }

    fn float(bytes: &[u8], index: usize) -> f32 {
        f32::from_le_bytes(bytes[4 * index..4 * index + 4].try_into().unwrap())
    }

    #[test]
    fn opposed_clips_each_colour_at_its_white_balanced_level() {
        let prepared = Highlights
            .prepare(&SCHEMA.defaults(), &bayer(), &mosaic(62, 47))
            .unwrap();
        let bytes = &prepared.passes[0].params;
        // `HighlightsParams`: 8 words, the clips and the correction, 9 `uint4`.
        assert_eq!(bytes.len(), 208);
        // The masks' grid, a third of the mosaic, rounded up to 8 each way.
        assert_eq!(
            [0, 1, 2, 3, 4, 5].map(|i| word(bytes, i)),
            [62, 47, 20, 15, 384, 24]
        );
        let clips = [8, 9, 10].map(|i| float(bytes, i));
        assert_eq!(clips, [2.0, 1.0, 1.5].map(|wb| 0.987 * wb));
        // No late correction without temperature's "as shot to reference".
        assert_eq!([12, 13, 14].map(|i| float(bytes, i)), [1.0; 3]);
        // The pattern from the mosaic's corner, one photosite into the sensor's.
        assert_eq!([16, 17, 22, 23].map(|i| word(bytes, i)), [1, 0, 2, 1]);
        assert_eq!(prepared.output, mosaic(62, 47));
        let entry_points: Vec<_> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(
            entry_points,
            [
                "opposed_mask",
                "opposed_dilate",
                "opposed_sums",
                "opposed_chroma",
                "opposed_inpaint"
            ]
        );
        assert!(
            prepared.passes[..4]
                .iter()
                .all(|pass| pass.kind == PassKind::Reduction)
        );
    }

    #[test]
    fn the_passes_over_the_mosaic_and_its_masks_can_run_in_row_strips() {
        let prepared = Highlights
            .prepare(&SCHEMA.defaults(), &bayer(), &mosaic(62, 47))
            .unwrap();
        let passes = &prepared.passes;
        // The mask runs over the grid's rows, 20 entries each, until mask_size, 384, is
        // covered; a row of it reads three rows of the mosaic.
        assert_eq!(passes[0].size, Some([20, 20]));
        assert_eq!(
            passes[0].strips,
            Some(Strips {
                rows: 3,
                granularity: 1,
                halo: 0
            })
        );
        // The dilation binds only the masks, and runs in bands of its rows as the mask
        // does; the combination, one workgroup, needs the partial results whole.
        assert_eq!(passes[1].strips, passes[0].strips);
        assert_eq!(passes[3].strips, None);
        // The sums' strips are whole rows of workgroups, 64 rows of the mosaic.
        assert_eq!(
            passes[2].strips,
            Some(Strips {
                rows: 4,
                granularity: 16,
                halo: 1
            })
        );
        assert_eq!(
            passes[4].strips,
            Some(Strips {
                rows: 1,
                granularity: 1,
                halo: 1
            })
        );
    }

    #[test]
    fn the_late_correction_weighs_the_averages() {
        let input = ImageDesc {
            late_correction: true,
            ..mosaic(62, 47)
        };
        let image = bayer();
        let prepared = Highlights
            .prepare(&SCHEMA.defaults(), &image, &input)
            .unwrap();
        let bytes = &prepared.passes[0].params;
        let camera = image.camera.as_ref().unwrap();
        let expected = apertrust_color::late_correction(
            camera.white_balance.as_deref(),
            camera.xyz_to_camera.as_ref().unwrap(),
        );
        assert_eq!([12, 13, 14].map(|i| float(bytes, i)), expected);
        assert_ne!(expected, [1.0; 3]);
    }

    #[test]
    fn the_threshold_defaults_to_the_linear_response_limit_below_1() {
        let mut image = bayer();
        let clip = |image: &ImageContext| Highlights.defaults(image).float("clip");
        assert_eq!(clip(&image), 1.0);
        image.camera.as_mut().unwrap().linear_response_limit = 0.9;
        assert_eq!(clip(&image), 0.9);
        image.camera.as_mut().unwrap().linear_response_limit = 1.2;
        assert_eq!(clip(&image), 1.0);
        assert_eq!(clip(&ImageContext::full(4, 4)), 1.0);
    }

    #[test]
    fn other_methods_and_images_are_not_ported() {
        let image = bayer();
        let mut clip = ParamValues::new();
        clip.set("mode", Value::Choice(0));
        let clip = SCHEMA.resolve(&clip).unwrap();
        assert!(Highlights.prepare(&clip, &image, &mosaic(62, 47)).is_err());
        let defaults = SCHEMA.defaults();
        let rgba = ImageDesc::rgba(62, 47);
        assert!(Highlights.prepare(&defaults, &image, &rgba).is_err());
        let monochrome = ImageContext::full(62, 47);
        assert!(
            Highlights
                .prepare(&defaults, &monochrome, &mosaic(62, 47))
                .is_err()
        );
        let four_colours = raw(Pattern {
            width: 2,
            height: 2,
            values: vec![0, 1, 3, 2],
        });
        assert!(
            Highlights
                .prepare(&defaults, &four_colours, &mosaic(62, 47))
                .is_err()
        );
    }
}
