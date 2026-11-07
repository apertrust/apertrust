// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2017-2025  darktable developers (ported from src/iop/hazeremoval.c)
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

//! Haze removal: the `hazeremoval` kernel's passes and their uniforms, from darktable's
//! `process()`, which computes its windows and constants itself (the module has no
//! `commit_params`). See `docs/ports/hazeremoval.md`.

use apertrust_plugin::{
    Binding, Format, ImageDesc, NotPorted, Pass, PassKind, Prepared, Resource, Scratch,
};

/// darktable's `w1` at full resolution, `2 + ceil(4 × wscale)` with `wscale` 1: the
/// radius of the dark channel's and the transmission's boxes.
pub const DARK_RADIUS: u32 = 6;
/// darktable's `w2` at full resolution, `3 + ceil(6 × wscale)`: the guided filter's.
pub const GUIDE_RADIUS: u32 = 9;

/// darktable's quantiles: the haziest pixels are the dark channel's top 5%, and A0 the
/// mean of the brightest 5% of those.
const QUANTILE: f32 = 0.95;

/// The pixels a side each invocation of the counts and the ambient sums adds up, as the
/// kernel's `BLOCK`, and the kernel's workgroups a side. WebGPU zero-initialises a
/// workgroup's shared memory before it starts, which naga does from one invocation, so
/// the counts, with 8 KB of bins, run fewer, larger workgroups.
const BLOCK: u32 = 4;
const WORKGROUP: u32 = 16;
/// The selections' bins, as the kernel's `BINS`, and the words of its `state`.
const BINS: u64 = 2048;
const STATE_WORDS: u64 = 16;

/// Bytes of the kernel's `HazeParams`: twenty 32-bit words.
pub const PARAMS_SIZE: usize = 80;

/// darktable `dt_iop_hazeremoval_params_t` version 3, the fields a new instance uses.
/// `compatibility_mode` (off) and `adaptive` (on) are what darktable gives a new
/// instance and are not settings here (`docs/ports/hazeremoval.md`, Parameters).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// How much haze to remove, −1 to 1; negative adds haze.
    pub strength: f32,
    /// How far into the scene to remove it, 0 to 1.
    pub distance: f32,
}

impl Default for Settings {
    /// darktable's defaults, its introspection's `$DEFAULT`s.
    fn default() -> Self {
        Self {
            strength: 0.2,
            distance: 0.2,
        }
    }
}

/// darktable's `w1` and `w2` at a pipe's `scale`. darktable's adaptive windows,
/// `2 + ceil(4 × scale)` and `3 + ceil(6 × scale)`, follow its preview pipe; Apertrust's
/// are the full-resolution radii times the scale, rounded to the nearest integer, as
/// every pipe shows the export at its own scale (decision 15). At scale 1 they are
/// darktable's.
pub fn radii(scale: f32) -> [u32; 2] {
    let scaled = |radius: u32| ((radius as f32 * scale.clamp(0.0, 1.0)).round() as u32).min(radius);
    [scaled(DARK_RADIUS), scaled(GUIDE_RADIUS)]
}

/// darktable's `p = (size_t)(size * dark_channel_quantil)`: the rank, from 0, of the
/// dark channel's critical haze level among `pixels`, the product in f32.
pub fn rank(pixels: u64) -> u32 {
    (pixels as f32 * QUANTILE) as u32
}

/// darktable's `eps`: `sqrtf(0.025f)`, squared by the guided filter.
pub fn eps() -> f32 {
    let sqrt_eps = 0.025f32.sqrt();
    sqrt_eps * sqrt_eps
}

/// The kernel's `HazeParams` for one pass. The whole-image passes leave the strength
/// and distance zero, so their bytes depend on the strength's sign alone.
#[derive(Debug, Clone, Copy, Default)]
struct Uniforms {
    width: u32,
    height: u32,
    dark_radius: u32,
    guide_radius: u32,
    rank: u32,
    partials_across: u32,
    partials: u32,
    maximum: u32,
    normalise: u32,
    selection: u32,
    digit: u32,
    eps: f32,
    max_depth: f32,
    strength: f32,
    distance: f32,
}

impl Uniforms {
    fn bytes(&self) -> Vec<u8> {
        let words = [
            self.width,
            self.height,
            self.dark_radius,
            self.guide_radius,
            self.rank,
            self.partials_across,
            self.partials,
            self.maximum,
            self.normalise,
            self.selection,
            self.digit,
            self.eps.to_bits(),
            self.max_depth.to_bits(),
            0,
            0,
            0,
            self.strength.to_bits(),
            self.distance.to_bits(),
            0,
            0,
        ];
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }
}

/// The scratch buffers, by the kernel's bindings from 10: two planes of one float a
/// pixel, the guided filter's coefficients, the selections' bins and state, the ambient
/// sums' partial results, and the image, the input's RGB and smallest channel, which
/// every whole-image pass but the first reads in place of the input (decision 15).
const PLANE_A: (u32, Resource) = (10, Resource::Scratch(0));
const PLANE_B: (u32, Resource) = (11, Resource::Scratch(1));
const COEFFICIENTS: (u32, Resource) = (12, Resource::Scratch(2));
const SELECTION_BINS: (u32, Resource) = (13, Resource::Scratch(3));
const STATE: (u32, Resource) = (14, Resource::Scratch(4));
const PARTIALS: (u32, Resource) = (15, Resource::Scratch(5));
const IMAGE: (u32, Resource) = (16, Resource::Scratch(6));
/// In the viewport (decision 15): the full-resolution image its downscale read, bound in
/// the image's place, which no pass copies; and the filtered transmission's input
/// downscaled to the viewport's size, which `main` reads in plane A's place, the sixth
/// scratch buffer there, as no image is held.
const FULL_RESOLUTION: (u32, Resource) = (16, Resource::FullResolution);
const VIEWPORT_PLANE: (u32, Resource) = (17, Resource::Scratch(6));
const VIEWPORT_PLANE_FOR_MAIN: (u32, Resource) = (10, Resource::Scratch(6));
const INPUT: (u32, Resource) = (0, Resource::Input);
const OUTPUT: (u32, Resource) = (1, Resource::Output);
const UNIFORMS: (u32, Resource) = (2, Resource::Uniforms);

/// One workgroup's invocations, for the passes that combine.
const ONE_WORKGROUP: Option<[u32; 2]> = Some([WORKGROUP, WORKGROUP]);

/// The passes for `input`, part of an image `whole` pixels at the pipe's `scale` (all of
/// it but in a tile of an export): the whole-image passes that estimate A0, the depth
/// and the filtered transmission (decision 15), then `main`, the only one whose
/// uniforms hold the strength and distance.
///
/// The whole-image passes are one run (decision 15): the first, `image_copy`, the only
/// one that reads the input, writes it into scratch laid out as the whole image, so a
/// tile of an export's first round writes only its own pixels there; every later one
/// reads only scratch and covers the whole image. The scratch takes 40 bytes a pixel of
/// the whole image: the two planes, the coefficients and the image.
///
/// In the viewport, `full_resolution`, the size of the image the viewport's downscale
/// read, which the whole-image passes then bind as [`Resource::FullResolution`] in the
/// image's place, with no `image_copy` (decision 15): they estimate A0, the depth and
/// the filtered transmission from it, with darktable's windows at full resolution, as
/// an export does; `plane_downscale` downscales the transmission's input into the
/// viewport's size by the viewport's own box filter; and `main` runs at the viewport's
/// scale on `input`, reading that and the statistics as they are. The scratch then takes
/// 24 bytes a pixel of the full resolution, and 4 of the viewport.
pub fn prepare(
    settings: &Settings,
    input: &ImageDesc,
    whole: [u32; 2],
    scale: f32,
    full_resolution: Option<[u32; 2]>,
) -> Result<Prepared, NotPorted> {
    if input.format != Format::Rgba {
        return Err(NotPorted {
            operation: "hazeremoval",
            what: "images other than RGBA",
        });
    }
    // The image the whole-image passes estimate from: the full resolution, at scale 1,
    // or the whole image at the pipe's scale.
    let (estimated, estimate_scale) = match full_resolution {
        Some(full) => (full, 1.0),
        None => (whole, scale),
    };
    let [width, height] = estimated;
    let pixels = u64::from(width) * u64::from(height);
    // Every whole-image pass after the first runs over the whole image.
    let everywhere = Some(estimated);
    let [dark_radius, guide_radius] = radii(estimate_scale);
    let blocks = [width.div_ceil(BLOCK), height.div_ceil(BLOCK)];
    let partials_across = blocks[0].div_ceil(WORKGROUP);
    let partials = partials_across * blocks[1].div_ceil(WORKGROUP);
    let base = Uniforms {
        width,
        height,
        dark_radius,
        guide_radius,
        rank: rank(pixels),
        partials_across,
        partials,
        eps: eps(),
        max_depth: f32::MAX.ln() / 2.0,
        ..Uniforms::default()
    };

    let pass = |entry_point, kind, uniforms: Uniforms, size, bindings: &[(u32, Resource)]| Pass {
        entry_point,
        kind,
        params: uniforms.bytes(),
        tables: Vec::new(),
        bindings: [UNIFORMS]
            .iter()
            .chain(bindings)
            .map(|&(binding, resource)| Binding::new(binding, resource))
            .collect(),
        size,
        strips: None,
    };
    let whole_image = |entry_point, uniforms, size, bindings: &[(u32, Resource)]| {
        pass(entry_point, PassKind::Reduction, uniforms, size, bindings)
    };
    // What the whole-image passes read of the image: the copy, or the full-resolution
    // image itself.
    let image = if full_resolution.is_some() {
        FULL_RESOLUTION
    } else {
        IMAGE
    };
    let box_pass = |entry_point, maximum: bool, normalise: bool| {
        let uniforms = Uniforms {
            maximum: u32::from(maximum),
            normalise: u32::from(normalise),
            ..base
        };
        match entry_point {
            "image_rows" => {
                whole_image(entry_point, uniforms, everywhere, &[image, PLANE_A, STATE])
            }
            _ => whole_image(entry_point, uniforms, everywhere, &[PLANE_A, PLANE_B]),
        }
    };

    let mut passes = Vec::new();
    if full_resolution.is_none() {
        // The input, at its place in the whole image.
        passes.push(whole_image("image_copy", base, None, &[INPUT, IMAGE]));
    }
    passes.extend([
        // The dark channel, into plane B.
        box_pass("image_rows", false, false),
        box_pass("plane_cols", false, false),
        whole_image(
            "select_begin",
            base,
            ONE_WORKGROUP,
            &[SELECTION_BINS, STATE],
        ),
    ]);
    // darktable's two quick selects: the critical haze level, then the brightness among
    // the pixels at or above it.
    for selection in 0..2 {
        for digit in 0..3 {
            let uniforms = Uniforms {
                selection,
                digit,
                ..base
            };
            passes.push(whole_image(
                "select_count",
                uniforms,
                Some(blocks),
                &[image, PLANE_B, SELECTION_BINS, STATE],
            ));
            passes.push(whole_image(
                "select_scan",
                uniforms,
                ONE_WORKGROUP,
                &[SELECTION_BINS, STATE],
            ));
        }
    }
    // A0 and the depth.
    passes.push(whole_image(
        "ambient_sums",
        base,
        Some(blocks),
        &[image, PLANE_B, STATE, PARTIALS],
    ));
    passes.push(whole_image(
        "ambient_combine",
        base,
        ONE_WORKGROUP,
        &[STATE, PARTIALS],
    ));
    // The transmission's smallest channel over A0, eroded then dilated: darktable's box
    // maximum of 1 − strength × m, then its box minimum. A negative strength turns the
    // map around, and with it the order.
    let erode_first = settings.strength >= 0.0;
    passes.push(box_pass("image_rows", !erode_first, true));
    passes.push(box_pass("plane_cols", !erode_first, false));
    passes.push(box_pass("plane_rows", erode_first, false));
    passes.push(box_pass("plane_cols", erode_first, false));
    // The guided filter, into plane A: a workgroup per 16×16 tile.
    passes.push(whole_image(
        "guide_coefficients",
        base,
        everywhere,
        &[image, PLANE_B, COEFFICIENTS],
    ));
    passes.push(whole_image(
        "guide_result",
        base,
        everywhere,
        &[image, PLANE_A, COEFFICIENTS],
    ));
    // In the viewport, the transmission's input downscaled from the full resolution to
    // the viewport's size, which `main` reads, and `main` at the viewport's size.
    let viewport = [input.width, input.height];
    let transmission = match full_resolution {
        Some(_) => {
            passes.push(whole_image(
                "plane_downscale",
                base,
                Some(viewport),
                &[PLANE_A, VIEWPORT_PLANE],
            ));
            VIEWPORT_PLANE_FOR_MAIN
        }
        None => PLANE_A,
    };
    let [main_width, main_height] = if full_resolution.is_some() {
        viewport
    } else {
        whole
    };
    // The haze-free image.
    let dehaze = Uniforms {
        width: main_width,
        height: main_height,
        strength: settings.strength,
        distance: settings.distance,
        ..base
    };
    passes.push(pass(
        "main",
        PassKind::PerPixel,
        dehaze,
        None,
        &[INPUT, OUTPUT, transmission, STATE],
    ));

    let plane = |bytes: u64| Scratch {
        bytes: pixels * bytes,
    };
    let mut scratch = vec![
        plane(4),
        plane(4),
        plane(16),
        Scratch { bytes: BINS * 4 },
        Scratch {
            bytes: STATE_WORDS * 4,
        },
        Scratch {
            bytes: u64::from(partials) * 16,
        },
    ];
    scratch.push(match full_resolution {
        // The viewport's plane, its image the full-resolution one the plan holds.
        Some(_) => Scratch {
            bytes: u64::from(input.width) * u64::from(input.height) * 4,
        },
        None => plane(16),
    });
    Ok(Prepared {
        output: *input,
        passes,
        scratch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(bytes: &[u8], index: usize) -> u32 {
        u32::from_le_bytes(bytes[4 * index..4 * index + 4].try_into().unwrap())
    }

    fn float(bytes: &[u8], index: usize) -> f32 {
        f32::from_bits(word(bytes, index))
    }

    fn prepared(settings: Settings, width: u32, height: u32, scale: f32) -> Prepared {
        prepare(
            &settings,
            &ImageDesc::rgba(width, height),
            [width, height],
            scale,
            None,
        )
        .unwrap()
    }

    #[test]
    fn the_uniform_has_the_kernels_size() {
        let prepared = prepared(Settings::default(), 100, 60, 1.0);
        for pass in &prepared.passes {
            assert_eq!(pass.params.len(), PARAMS_SIZE, "{}", pass.entry_point);
        }
    }

    #[test]
    fn the_windows_are_darktables_at_full_resolution_and_scale_with_the_pipe() {
        assert_eq!(radii(1.0), [6, 9]);
        // A viewport at a quarter: 1.5 and 2.25, rounded.
        assert_eq!(radii(0.25), [2, 2]);
        assert_eq!(radii(0.5), [3, 5]);
        // Never wider than at full resolution.
        assert_eq!(radii(2.0), [6, 9]);
        let uniforms = &prepared(Settings::default(), 100, 60, 0.5).passes[0].params;
        assert_eq!([word(uniforms, 2), word(uniforms, 3)], [3, 5]);
    }

    #[test]
    fn the_rank_is_darktables_f32_product() {
        assert_eq!(rank(100 * 60), 5700);
        // 3908 × 2602 pixels, test 0026's: the product rounds in f32 as darktable's.
        assert_eq!(rank(3908 * 2602), (10_168_616.0f32 * 0.95f32) as u32);
        // Above 2^24 pixels the count itself rounds to an f32.
        let big = (1u64 << 25) + 1;
        assert_eq!(rank(big), ((1u64 << 25) as f32 * 0.95) as u32);
    }

    #[test]
    fn eps_is_darktables_regularisation() {
        assert!((eps() - 0.025).abs() < 1e-8);
        assert_eq!(eps(), 0.025f32.sqrt() * 0.025f32.sqrt());
    }

    #[test]
    fn only_main_holds_the_strength_and_distance() {
        let a = prepared(Settings::default(), 100, 60, 1.0);
        let b = prepared(
            Settings {
                strength: 0.9,
                distance: 0.8,
            },
            100,
            60,
            1.0,
        );
        let (main, whole) = a.passes.split_last().unwrap();
        let (main_b, whole_b) = b.passes.split_last().unwrap();
        assert_eq!(main.entry_point, "main");
        assert_eq!(main.kind, PassKind::PerPixel);
        // The whole-image passes' bytes, their key, are the same for any strength of the
        // same sign and any distance.
        for (pass, other) in whole.iter().zip(whole_b) {
            assert_eq!(pass.kind, PassKind::Reduction, "{}", pass.entry_point);
            assert_eq!(pass.params, other.params, "{}", pass.entry_point);
            assert_eq!([float(&pass.params, 16), float(&pass.params, 17)], [0.0; 2]);
        }
        assert_eq!(
            [float(&main.params, 16), float(&main.params, 17)],
            [0.2, 0.2]
        );
        assert_eq!(
            [float(&main_b.params, 16), float(&main_b.params, 17)],
            [0.9, 0.8]
        );
    }

    #[test]
    fn a_negative_strength_dilates_before_it_erodes() {
        let order = |strength| {
            let settings = Settings {
                strength,
                distance: 0.2,
            };
            let prepared = prepared(settings, 100, 60, 1.0);
            let passes = &prepared.passes;
            let n = passes.len();
            // The transmission's four box passes come before the guided filter's two
            // and main.
            passes[n - 7..n - 3]
                .iter()
                .map(|pass| {
                    (
                        pass.entry_point,
                        word(&pass.params, 7),
                        word(&pass.params, 8),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            order(0.2),
            [
                ("image_rows", 0, 1),
                ("plane_cols", 0, 0),
                ("plane_rows", 1, 0),
                ("plane_cols", 1, 0)
            ]
        );
        assert_eq!(
            order(-0.2),
            [
                ("image_rows", 1, 1),
                ("plane_cols", 1, 0),
                ("plane_rows", 0, 0),
                ("plane_cols", 0, 0)
            ]
        );
        // The dark channel is always a minimum, of the plain smallest channel.
        let first = &prepared(Settings::default(), 100, 60, 1.0).passes[1];
        assert_eq!((word(&first.params, 7), word(&first.params, 8)), (0, 0));
    }

    #[test]
    fn the_passes_and_scratch_cover_the_image() {
        let prepared = prepared(Settings::default(), 101, 61, 1.0);
        let entry_points: Vec<_> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(entry_points.len(), 25);
        assert_eq!(
            entry_points[..4],
            ["image_copy", "image_rows", "plane_cols", "select_begin"]
        );
        assert_eq!(
            entry_points[4..16]
                .iter()
                .filter(|&&e| e == "select_count")
                .count(),
            6
        );
        assert_eq!(
            entry_points[16..],
            [
                "ambient_sums",
                "ambient_combine",
                "image_rows",
                "plane_cols",
                "plane_rows",
                "plane_cols",
                "guide_coefficients",
                "guide_result",
                "main"
            ]
        );
        // The sums add up blocks of 4×4 pixels, one partial result per workgroup of
        // 64×64 pixels: two across, one down.
        let sums = &prepared.passes[16];
        assert_eq!(sums.size, Some([26, 16]));
        assert_eq!([word(&sums.params, 5), word(&sums.params, 6)], [2, 2]);
        let bytes: Vec<u64> = prepared.scratch.iter().map(|s| s.bytes).collect();
        let pixels = 101 * 61;
        assert_eq!(
            bytes,
            [
                pixels * 4,
                pixels * 4,
                pixels * 16,
                8192,
                64,
                32,
                pixels * 16
            ]
        );
        // Decision 15's estimate: 40 bytes a pixel, the bins, state and partial results
        // aside.
        let per_pixel: u64 = bytes.iter().filter(|&&b| b % pixels == 0).sum();
        assert_eq!(per_pixel, 40 * pixels);
        assert_eq!(prepared.output, ImageDesc::rgba(101, 61));
    }

    #[test]
    fn its_whole_image_passes_are_one_run() {
        // Decision 15: the first pass alone of the whole-image passes reads the input,
        // and every later one only scratch, so an export's first round of tiles runs the
        // first in each tile and the rest once; `main` reads the input in the last round.
        let prepared = prepared(Settings::default(), 101, 61, 1.0);
        let reads_input = |pass: &Pass| {
            pass.bindings
                .iter()
                .any(|binding| binding.resource == Resource::Input)
        };
        let readers: Vec<&str> = prepared
            .passes
            .iter()
            .filter(|pass| reads_input(pass))
            .map(|pass| pass.entry_point)
            .collect();
        assert_eq!(readers, ["image_copy", "main"]);
        let (main, whole) = prepared.passes.split_last().unwrap();
        assert_eq!(main.kind, PassKind::PerPixel);
        assert!(whole.iter().all(|pass| pass.kind == PassKind::Reduction));
    }

    #[test]
    fn a_tile_writes_its_own_pixels_and_the_rest_covers_the_whole_image() {
        // In a tile of an export, `image_copy` and `main` run over the tile, which the
        // pass header places in the whole image; every other pass, the uniforms' image
        // and the scratch are the whole image's.
        let tile = ImageDesc::rgba(64, 48);
        let prepared = prepare(&Settings::default(), &tile, [300, 200], 1.0, None).unwrap();
        assert_eq!(prepared.output, tile);
        let (first, rest) = prepared.passes.split_first().unwrap();
        let (main, rest) = rest.split_last().unwrap();
        assert_eq!((first.entry_point, first.size), ("image_copy", None));
        assert_eq!((main.entry_point, main.size), ("main", None));
        for pass in prepared.passes.iter() {
            assert_eq!(
                [word(&pass.params, 0), word(&pass.params, 1)],
                [300, 200],
                "{}",
                pass.entry_point
            );
        }
        for pass in rest {
            let size = pass.size.unwrap_or_else(|| panic!("{}", pass.entry_point));
            match pass.entry_point {
                "select_count" | "ambient_sums" => assert_eq!(size, [75, 50]),
                "select_begin" | "select_scan" | "ambient_combine" => assert_eq!(size, [16, 16]),
                _ => assert_eq!(size, [300, 200], "{}", pass.entry_point),
            }
        }
        assert_eq!(word(&first.params, 4), rank(300 * 200));
        let pixels = 300 * 200;
        let bytes: Vec<u64> = prepared.scratch.iter().map(|s| s.bytes).collect();
        assert_eq!(bytes[..3], [pixels * 4, pixels * 4, pixels * 16]);
        assert_eq!(bytes[6], pixels * 16);
    }

    #[test]
    fn in_the_viewport_it_estimates_from_the_full_resolution() {
        // Decision 15: a viewport of 100×60 from a full resolution of 400×240. No copy:
        // the whole-image passes bind the full-resolution image in the image's place and
        // run over it with darktable's windows; `plane_downscale` brings plane A to the
        // viewport's size, and `main` reads it there at the viewport's size.
        let settings = Settings {
            strength: 0.9,
            distance: 0.8,
        };
        let viewport = ImageDesc::rgba(100, 60);
        let prepared = prepare(&settings, &viewport, [100, 60], 0.25, Some([400, 240])).unwrap();
        let bound = |pass: &Pass, binding: u32| {
            pass.bindings
                .iter()
                .find(|b| b.binding == binding)
                .map(|b| b.resource)
        };
        let names: Vec<&str> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(names.len(), 25);
        assert!(!names.contains(&"image_copy"));
        let first = &prepared.passes[0];
        assert_eq!(
            (first.entry_point, first.size),
            ("image_rows", Some([400, 240]))
        );
        assert_eq!(bound(first, 16), Some(Resource::FullResolution));
        let words: Vec<u32> = (0..5).map(|i| word(&first.params, i)).collect();
        assert_eq!(words, [400, 240, 6, 9, rank(400 * 240)]);
        let (main, whole) = prepared.passes.split_last().unwrap();
        let downscale = whole.last().unwrap();
        assert_eq!(
            (downscale.entry_point, downscale.size),
            ("plane_downscale", Some([100, 60]))
        );
        assert_eq!(bound(downscale, 10), Some(Resource::Scratch(0)));
        assert_eq!(bound(downscale, 17), Some(Resource::Scratch(6)));
        assert_eq!((main.entry_point, main.size), ("main", None));
        assert_eq!([word(&main.params, 0), word(&main.params, 1)], [100, 60]);
        assert_eq!(bound(main, 0), Some(Resource::Input));
        assert_eq!(bound(main, 10), Some(Resource::Scratch(6)));
        // Only `main` reads the viewport's input; every image the estimate reads is the
        // full resolution's.
        for pass in whole {
            assert_ne!(
                bound(pass, 0),
                Some(Resource::Input),
                "{}",
                pass.entry_point
            );
            if let Some(image) = bound(pass, 16) {
                assert_eq!(image, Resource::FullResolution, "{}", pass.entry_point);
            }
        }
        let full = 400 * 240;
        let bytes: Vec<u64> = prepared.scratch.iter().map(|s| s.bytes).collect();
        assert_eq!(bytes.len(), 7);
        assert_eq!(bytes[..3], [full * 4, full * 4, full * 16]);
        assert_eq!(bytes[6], 100 * 60 * 4);
    }

    #[test]
    fn mosaics_are_not_ported() {
        let mosaic = ImageDesc::new(100, 60, Format::Mosaic);
        assert!(prepare(&Settings::default(), &mosaic, [100, 60], 1.0, None).is_err());
    }
}
