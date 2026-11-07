// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/bloom.c)
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

//! What darktable's `process` derives from the parameters and the pipe's scale before it
//! filters: the box mean's radius and the lights' scale, and the passes over them. See
//! `docs/ports/bloom.md`.

use apertrust_color::WorkingProfile;
use apertrust_color::box_filter::{BOX_ITERATIONS, iterated_radii};
use apertrust_plugin::{
    Binding, ImageDesc, NotPorted, Pass, PassKind, Prepared, Resource, Scratch,
};

/// darktable `dt_iop_bloom_params_t` version 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The bloom's size, in percent: the box mean's radius.
    pub size: f32,
    /// The scaled lightness a pixel must exceed to bloom.
    pub threshold: f32,
    /// The bloom's strength, in percent: the lights' scale.
    pub strength: f32,
}

impl Default for Settings {
    /// darktable's introspection defaults, which a new instance gets.
    fn default() -> Self {
        Self {
            size: 20.0,
            threshold: 90.0,
            strength: 25.0,
        }
    }
}

/// darktable's largest radius, `MIN(256.0f, _r)`.
const MAX_RADIUS: u32 = 256;

/// darktable `rad`, the box mean's radius at full resolution:
/// `256.0f * (fmin(100.0f, size + 1.0f) / 100.0f)`, truncated, where `fmin` is C's double
/// function, so `size + 1` is an f32 sum and the rest double.
pub fn full_radius(size: f32) -> u32 {
    (256.0 * (f64::from(size + 1.0).min(100.0) / 100.0)) as u32
}

/// Each box mean iteration's radius at `scale` pixels of the pipe per pixel at full
/// resolution: from darktable's full-resolution radius, radii `r` and `r + 1` whose blur
/// has the export's variance at that scale (decision 63, `iterated_radii`), where
/// darktable's `ceilf(rad * roi_in->scale / piece->iscale)` rounds up; every one
/// darktable's at scale 1. Each at most 256, as darktable's `MIN(256.0f, …)`, and the box
/// mean's half-width `hr = (2 · radius + 1) / 2` is the radius.
pub fn radii(size: f32, scale: f32) -> [u32; BOX_ITERATIONS as usize] {
    iterated_radii(full_radius(size), scale).map(|radius| radius.min(MAX_RADIUS))
}

/// darktable `scale`, the lights' scale:
/// `1.0f / exp2f(-1.0f * (fmin(100.0f, strength + 1.0f) / 100.0f))`, the exponent double,
/// then f32 for `exp2f`.
pub fn light_scale(strength: f32) -> f32 {
    let exponent = (-(f64::from(strength + 1.0).min(100.0) / 100.0)) as f32;
    1.0 / exponent.exp2()
}

/// The uniform block after the pass header, as `BloomParams` in the kernel lays it out;
/// each pass fills only what it reads (decision 15).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Uniforms {
    image: [u32; 4],
    lights: [f32; 4],
    rgb_to_xyz: [[f32; 3]; 3],
    xyz_to_rgb: [[f32; 3]; 3],
}

/// Bytes of `BloomParams`.
pub const PARAMS_SIZE: usize = 16 * 8;

impl Uniforms {
    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        for word in &self.image {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in &self.lights {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for row in self.rgb_to_xyz.iter().chain(&self.xyz_to_rgb) {
            for value in row.iter().chain(&[0.0]) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }
}

/// A box pass's axis, `image.w` in the kernel.
const ROWS: u32 = 0;
const COLUMNS: u32 = 1;
/// The scratch buffers: A holds the lights and, after every pair of box passes, their
/// blur; B the blur along the rows between.
const A: usize = 0;
const B: usize = 1;

const INPUT: (u32, Resource) = (0, Resource::Input);
const OUTPUT: (u32, Resource) = (1, Resource::Output);
const UNIFORMS: (u32, Resource) = (2, Resource::Uniforms);
/// The scratch buffer a pass reads.
fn reads(scratch: usize) -> (u32, Resource) {
    (10, Resource::Scratch(scratch))
}
/// The scratch buffer a pass writes.
fn writes(scratch: usize) -> (u32, Resource) {
    (11, Resource::Scratch(scratch))
}

/// A pass of the kernel with its uniforms and bindings, `size` invocations or the
/// output's.
fn pass(
    entry_point: &'static str,
    kind: PassKind,
    uniforms: Uniforms,
    size: Option<[u32; 2]>,
    bindings: &[(u32, Resource)],
) -> Pass {
    Pass {
        entry_point,
        kind,
        params: uniforms.to_bytes(),
        tables: Vec::new(),
        bindings: bindings
            .iter()
            .map(|&(binding, resource)| Binding::new(binding, resource))
            .collect(),
        size,
        strips: None,
    }
}

/// A box pass's size: one invocation per line, in rows of 16, as `box_mean` reads them.
fn lines(count: u32) -> Option<[u32; 2]> {
    Some([16, count.div_ceil(16)])
}

/// The passes for `settings` on `input`, part of an image `whole` pixels at the pipe's
/// scale (all of it but in a tile of an export), at `scale` pixels of the pipe per pixel
/// at full resolution, in the working space `profile` (darktable's `process`).
pub fn prepare(
    settings: &Settings,
    input: &ImageDesc,
    whole: [u32; 2],
    scale: f32,
    profile: &WorkingProfile,
) -> Result<Prepared, NotPorted> {
    let [width, height] = whole;
    let box_mean = |radius: u32, axis: u32, from: usize, to: usize| {
        let lines = lines(if axis == ROWS { height } else { width });
        pass(
            "box_mean",
            PassKind::Reduction,
            Uniforms {
                image: [width, height, radius, axis],
                ..Uniforms::default()
            },
            lines,
            &[UNIFORMS, reads(from), writes(to)],
        )
    };
    let mut passes = vec![pass(
        "threshold",
        PassKind::Reduction,
        Uniforms {
            image: [width, height, 0, 0],
            lights: [light_scale(settings.strength), settings.threshold, 0.0, 0.0],
            rgb_to_xyz: profile.rgb_to_xyz_d50,
            ..Uniforms::default()
        },
        None,
        &[INPUT, UNIFORMS, writes(A)],
    )];
    // Each iteration's row pass and column pass take its radius (decision 63).
    for radius in radii(settings.size, scale) {
        passes.push(box_mean(radius, ROWS, A, B));
        passes.push(box_mean(radius, COLUMNS, B, A));
    }
    passes.push(pass(
        "main",
        PassKind::PerPixel,
        Uniforms {
            image: [width, height, 0, 0],
            lights: [0.0, 0.0, settings.strength, 0.0],
            rgb_to_xyz: profile.rgb_to_xyz_d50,
            xyz_to_rgb: profile.xyz_d50_to_rgb,
        },
        None,
        &[INPUT, OUTPUT, UNIFORMS, reads(A)],
    ));
    let lights = Scratch {
        bytes: u64::from(width) * u64::from(height) * 4,
    };
    Ok(Prepared {
        output: *input,
        passes,
        scratch: vec![lights; 2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    fn prepared(settings: &Settings, width: u32, height: u32) -> Prepared {
        let input = ImageDesc::rgba(width, height);
        prepare(settings, &input, [width, height], 1.0, &profile()).unwrap()
    }

    fn word(pass: &Pass, index: usize) -> u32 {
        u32::from_le_bytes(pass.params[4 * index..4 * index + 4].try_into().unwrap())
    }

    fn float(pass: &Pass, index: usize) -> f32 {
        f32::from_le_bytes(pass.params[4 * index..4 * index + 4].try_into().unwrap())
    }

    #[test]
    fn the_radius_is_darktables_at_full_resolution() {
        assert_eq!(full_radius(20.0), 53);
        // Test 0050: 256 · 0.41 = 104.96.
        assert_eq!(full_radius(40.0), 104);
        assert_eq!(full_radius(0.0), 2);
        assert_eq!(full_radius(24.0), 64);
        assert_eq!(full_radius(99.0), 256);
        assert_eq!(full_radius(100.0), 256);
        assert_eq!(radii(40.0, 1.0), [104; 8]);
    }

    #[test]
    fn the_radii_follow_the_pipes_scale() {
        // Decision 63: test 0050's 104 with mire1 fitted to 2400 wide; the default 53 at
        // 1/8; the smallest, 2, at a half, six of 1 and two of 0, the export's variance
        // exactly.
        assert_eq!(
            radii(40.0, 2400.0 / 3908.0),
            [64, 64, 64, 64, 64, 63, 63, 63]
        );
        assert_eq!(radii(20.0, 488.0 / 3908.0), [7, 7, 6, 6, 6, 6, 6, 6]);
        assert_eq!(radii(0.0, 0.5), [1, 1, 1, 1, 1, 1, 0, 0]);
        assert_eq!(radii(0.0, 0.1), [0; 8]);
    }

    #[test]
    fn each_iterations_row_and_column_passes_take_its_radius() {
        let input = ImageDesc::rgba(1954, 1301);
        let settings = Settings {
            size: 40.0,
            ..Settings::default()
        };
        let prepared = prepare(&settings, &input, [1954, 1301], 0.5, &profile()).unwrap();
        let taken: Vec<u32> = prepared.passes[1..17]
            .iter()
            .map(|pass| word(pass, 2))
            .collect();
        let expected: Vec<u32> = radii(40.0, 0.5)
            .iter()
            .flat_map(|&radius| [radius, radius])
            .collect();
        assert_eq!(taken, expected);
        assert_eq!(taken[10..], [52, 52, 51, 51, 51, 51]);
    }

    #[test]
    fn the_lights_scale_is_darktables() {
        // Test 0050's strength 70: 2^0.71.
        assert_eq!(light_scale(70.0), 1.0 / (-0.71_f32).exp2());
        assert!((light_scale(70.0) - 1.635_804_3).abs() < 1e-6);
        assert_eq!(light_scale(99.0), 2.0);
        assert_eq!(light_scale(100.0), 2.0);
        assert!((light_scale(0.0) - 2f32.powf(0.01)).abs() < 1e-6);
    }

    #[test]
    fn the_passes_are_the_lights_sixteen_box_means_and_main() {
        let prepared = prepared(&Settings::default(), 3908, 2602);
        let entry_points: Vec<&str> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(entry_points.len(), 18);
        assert_eq!(entry_points[0], "threshold");
        assert!(entry_points[1..17].iter().all(|&e| e == "box_mean"));
        assert_eq!(entry_points[17], "main");
        assert_eq!(
            prepared.scratch,
            vec![
                Scratch {
                    bytes: 3908 * 2602 * 4
                };
                2
            ]
        );
        // Rows, then columns, from A to B and back.
        let (rows, columns) = (&prepared.passes[1], &prepared.passes[2]);
        assert_eq!(rows.size, Some([16, 163]));
        assert_eq!(columns.size, Some([16, 245]));
        assert_eq!((word(rows, 2), word(rows, 3)), (53, ROWS));
        assert_eq!(word(columns, 3), COLUMNS);
        let scratch = |pass: &Pass, binding: u32| {
            pass.bindings
                .iter()
                .find(|b| b.binding == binding)
                .map(|b| b.resource)
        };
        assert_eq!(scratch(rows, 10), Some(Resource::Scratch(A)));
        assert_eq!(scratch(rows, 11), Some(Resource::Scratch(B)));
        assert_eq!(scratch(columns, 10), Some(Resource::Scratch(B)));
        assert_eq!(scratch(columns, 11), Some(Resource::Scratch(A)));
        assert_eq!(scratch(&prepared.passes[0], 11), Some(Resource::Scratch(A)));
        assert_eq!(
            scratch(&prepared.passes[17], 10),
            Some(Resource::Scratch(A))
        );
    }

    #[test]
    fn every_pass_binds_what_it_reads_within_eight_storage_buffers() {
        let prepared = prepared(&Settings::default(), 640, 480);
        for pass in &prepared.passes {
            assert_eq!(pass.params.len(), PARAMS_SIZE);
            let storage = pass
                .bindings
                .iter()
                .filter(|b| b.resource != Resource::Uniforms)
                .count();
            assert!(storage <= 8, "{}", pass.entry_point);
            for binding in &pass.bindings {
                if let Resource::Scratch(index) = binding.resource {
                    assert!(index < prepared.scratch.len(), "{}", pass.entry_point);
                }
            }
            // Only `main` writes the output, and only it is per pixel; only it and
            // `threshold` read the input.
            let has = |resource| pass.bindings.iter().any(|b| b.resource == resource);
            assert_eq!(has(Resource::Output), pass.entry_point == "main");
            assert_eq!(
                pass.kind == PassKind::PerPixel,
                pass.entry_point == "main",
                "{}",
                pass.entry_point
            );
            assert_eq!(has(Resource::Input), pass.entry_point != "box_mean");
        }
        let main = prepared.passes.last().unwrap();
        let bound: Vec<u32> = main.bindings.iter().map(|b| b.binding).collect();
        assert_eq!(bound, [0, 1, 2, 10]);
    }

    #[test]
    fn each_pass_holds_only_the_settings_it_reads() {
        // Decision 15: the box passes the radius alone, `threshold` the threshold and the
        // lights' scale, `main` the strength.
        let base = Settings {
            size: 40.0,
            threshold: 85.0,
            strength: 70.0,
        };
        let params = |settings: Settings| {
            prepared(&settings, 320, 200)
                .passes
                .iter()
                .map(|pass| pass.params.clone())
                .collect::<Vec<_>>()
        };
        let a = params(base);
        let threshold = params(Settings {
            threshold: 60.0,
            ..base
        });
        let strength = params(Settings {
            strength: 10.0,
            ..base
        });
        let size = params(Settings { size: 5.0, ..base });
        assert_ne!(a[0], threshold[0]);
        assert_eq!(a[1..], threshold[1..]);
        assert_ne!(a[0], strength[0]);
        assert_eq!(a[1..17], strength[1..17]);
        assert_ne!(a[17], strength[17]);
        assert_eq!(a[0], size[0]);
        assert_ne!(a[1..17], size[1..17]);
        assert_eq!(a[17], size[17]);
    }

    #[test]
    fn a_tiles_passes_cover_the_whole_image() {
        // In a tile of an export, `threshold` and `main` run over the tile and place it
        // in the whole image's scratch, which the box passes walk.
        let tile = ImageDesc::rgba(64, 48);
        let prepared = prepare(&Settings::default(), &tile, [300, 200], 1.0, &profile()).unwrap();
        assert_eq!(prepared.output, tile);
        assert_eq!(prepared.scratch[0].bytes, 300 * 200 * 4);
        assert_eq!(prepared.passes[0].size, None);
        assert_eq!(
            (word(&prepared.passes[0], 0), word(&prepared.passes[0], 1)),
            (300, 200)
        );
        assert_eq!(prepared.passes[1].size, Some([16, 13]));
        assert_eq!(prepared.passes[2].size, Some([16, 19]));
        assert_eq!(word(&prepared.passes[17], 0), 300);
    }

    #[test]
    fn the_uniforms_are_laid_out_as_the_kernel_reads_them() {
        let prepared = prepared(
            &Settings {
                size: 40.0,
                threshold: 85.0,
                strength: 70.0,
            },
            320,
            200,
        );
        let threshold = &prepared.passes[0];
        assert_eq!((word(threshold, 0), word(threshold, 1)), (320, 200));
        assert_eq!(float(threshold, 4), light_scale(70.0));
        assert_eq!(float(threshold, 5), 85.0);
        assert_eq!(float(threshold, 8), profile().rgb_to_xyz_d50[0][0]);
        assert_eq!(float(threshold, 11), 0.0);
        let main = &prepared.passes[17];
        assert_eq!(float(main, 6), 70.0);
        assert_eq!(float(main, 20), profile().xyz_d50_to_rgb[0][0]);
        assert_eq!(float(main, 30), profile().xyz_d50_to_rgb[2][2]);
    }
}
