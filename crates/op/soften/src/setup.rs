// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2011-2026  darktable developers (ported from src/iop/soften.c)
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

//! What darktable's `process` derives from the parameters, the image's size and the
//! pipe's scale before it filters: the factors of the saturation and the brightness, the
//! box mean's radius and the mix, and the passes over them. See `docs/ports/soften.md`.

use apertrust_color::box_filter::{BOX_ITERATIONS, iterated_radii};
use apertrust_plugin::{
    Binding, ImageDesc, NotPorted, Pass, PassKind, Prepared, Resource, Scratch,
};

/// darktable `dt_iop_soften_params_t` version 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The blur's size, in percent of the largest radius, 1% of the image's diagonal:
    /// the box mean's radius.
    pub size: f32,
    /// The blurred image's saturation, in percent.
    pub saturation: f32,
    /// The blurred image's brightness, in EV.
    pub brightness: f32,
    /// The mix of the blurred image into the input, in percent (darktable's `amount`).
    pub amount: f32,
}

impl Default for Settings {
    /// darktable's introspection defaults, which a new instance gets.
    fn default() -> Self {
        Self {
            size: 50.0,
            saturation: 100.0,
            brightness: 0.33,
            amount: 50.0,
        }
    }
}

/// darktable `mrad`, the largest radius, 1% of the image's diagonal at full resolution:
/// `dt_fast_hypotf(w, h) * 0.01f`, truncated, where `w` and `h` are `piece->iwidth *
/// piece->iscale` and `iheight`, the pipe's input at full resolution, for a raw its
/// readout before rawprepare's crop. darktable's release builds take
/// `dt_fast_hypotf`'s `-ffast-math` form, `sqrtf(x * x + y * y)`.
pub fn max_radius(full: [u32; 2]) -> u32 {
    let [w, h] = full.map(|side| side as f32);
    ((w * w + h * h).sqrt() * 0.01) as u32
}

/// darktable `rad`, the box mean's radius at full resolution:
/// `mrad * (fmin(100.0, size + 1.0f) / 100.0)`, truncated, where `size + 1` is an f32
/// sum and the rest double.
pub fn full_radius(size: f32, full: [u32; 2]) -> u32 {
    (f64::from(max_radius(full)) * (f64::from(size + 1.0).min(100.0) / 100.0)) as u32
}

/// Each box mean iteration's radius at `scale` pixels of the pipe per pixel at full
/// resolution: from darktable's full-resolution radius, radii `r` and `r + 1` whose blur
/// has the export's variance at that scale (decision 63, `iterated_radii`), where
/// darktable's `ceilf(rad * roi_in->scale / piece->iscale)` rounds up; every one
/// darktable's at scale 1. Each at most `mrad`, as darktable's `MIN(mrad, …)`.
pub fn radii(size: f32, full: [u32; 2], scale: f32) -> [u32; BOX_ITERATIONS as usize] {
    let cap = max_radius(full);
    iterated_radii(full_radius(size, full), scale).map(|radius| radius.min(cap))
}

/// darktable `saturation`, the factor of the HSL saturation: `d->saturation / 100.0`,
/// rounded to f32, which is the f32 quotient.
pub fn saturation_factor(saturation: f32) -> f32 {
    saturation / 100.0
}

/// darktable `brightness`, the factor of the HSL lightness: `1.0 / exp2f(-brightness)`,
/// rounded to f32, which is the f32 quotient.
pub fn brightness_factor(brightness: f32) -> f32 {
    1.0 / (-brightness).exp2()
}

/// darktable `amt`, the blurred image's weight in the mix: `d->amount / 100.0f`.
pub fn mix(amount: f32) -> f32 {
    amount / 100.0
}

/// The uniform block after the pass header, as `SoftenParams` in the kernel lays it out;
/// each pass fills only what it reads (decision 15).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Uniforms {
    image: [u32; 4],
    factors: [f32; 4],
}

/// Bytes of `SoftenParams`.
pub const PARAMS_SIZE: usize = 16 * 2;

impl Uniforms {
    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        for word in &self.image {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in &self.factors {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

/// A box pass's axis, `image.w` in the kernel.
const ROWS: u32 = 0;
const COLUMNS: u32 = 1;
/// The scratch images, each three planes, red, green and blue, of one float a pixel:
/// A holds the softened image and, after every pair of box passes, its blur; B the blur
/// along the rows between.
const A: usize = 0;
const B: usize = 3;
/// Scratch buffers in all: A's three planes and B's.
const SCRATCH: usize = 6;

const INPUT: (u32, Resource) = (0, Resource::Input);
const OUTPUT: (u32, Resource) = (1, Resource::Output);
const UNIFORMS: (u32, Resource) = (2, Resource::Uniforms);
/// The planes of the scratch image a pass reads, at bindings 10 to 12.
fn reads(image: usize) -> [(u32, Resource); 3] {
    [0, 1, 2].map(|plane| (10 + plane as u32, Resource::Scratch(image + plane)))
}
/// The planes of the scratch image a pass writes, at bindings 13 to 15.
fn writes(image: usize) -> [(u32, Resource); 3] {
    [0, 1, 2].map(|plane| (13 + plane as u32, Resource::Scratch(image + plane)))
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
/// scale (all of it but in a tile of an export), whose pipe starts from `full` pixels at
/// full resolution (darktable's `piece->iwidth` and `iheight` times `piece->iscale`),
/// at `scale` pixels of the pipe per pixel at full resolution (darktable's `process`).
pub fn prepare(
    settings: &Settings,
    input: &ImageDesc,
    whole: [u32; 2],
    full: [u32; 2],
    scale: f32,
) -> Result<Prepared, NotPorted> {
    let [width, height] = whole;
    let box_mean = |radius: u32, axis: u32, from: usize, to: usize| {
        let lines = lines(if axis == ROWS { height } else { width });
        let bindings: Vec<_> = [UNIFORMS]
            .into_iter()
            .chain(reads(from))
            .chain(writes(to))
            .collect();
        pass(
            "box_mean",
            PassKind::Reduction,
            Uniforms {
                image: [width, height, radius, axis],
                ..Uniforms::default()
            },
            lines,
            &bindings,
        )
    };
    let overexpose: Vec<_> = [INPUT, UNIFORMS].into_iter().chain(writes(A)).collect();
    let mut passes = vec![pass(
        "overexpose",
        PassKind::Reduction,
        Uniforms {
            image: [width, height, 0, 0],
            factors: [
                saturation_factor(settings.saturation),
                brightness_factor(settings.brightness),
                0.0,
                0.0,
            ],
        },
        None,
        &overexpose,
    )];
    // Each iteration's row pass and column pass take its radius (decision 63).
    for radius in radii(settings.size, full, scale) {
        passes.push(box_mean(radius, ROWS, A, B));
        passes.push(box_mean(radius, COLUMNS, B, A));
    }
    let main: Vec<_> = [INPUT, OUTPUT, UNIFORMS]
        .into_iter()
        .chain(reads(A))
        .collect();
    passes.push(pass(
        "main",
        PassKind::PerPixel,
        Uniforms {
            image: [width, height, 0, 0],
            factors: [0.0, 0.0, mix(settings.amount), 0.0],
        },
        None,
        &main,
    ));
    let plane = Scratch {
        bytes: u64::from(width) * u64::from(height) * 4,
    };
    Ok(Prepared {
        output: *input,
        passes,
        scratch: vec![plane; SCRATCH],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// mire1.cr2, the golden image: its readout, from which darktable measures `mrad`,
    /// and its developed size.
    const READOUT: [u32; 2] = [3944, 2622];
    const DEVELOPED: [u32; 2] = [3908, 2602];

    fn prepared(settings: &Settings, width: u32, height: u32) -> Prepared {
        let input = ImageDesc::rgba(width, height);
        prepare(settings, &input, [width, height], [width, height], 1.0).unwrap()
    }

    fn word(pass: &Pass, index: usize) -> u32 {
        u32::from_le_bytes(pass.params[4 * index..4 * index + 4].try_into().unwrap())
    }

    fn float(pass: &Pass, index: usize) -> f32 {
        f32::from_le_bytes(pass.params[4 * index..4 * index + 4].try_into().unwrap())
    }

    #[test]
    fn the_radius_is_darktables_at_full_resolution() {
        // mire1's readout: hypot(3944, 2622) = 4736.0, so mrad 47; its developed image
        // would give 46.
        assert_eq!(max_radius(READOUT), 47);
        assert_eq!(max_radius(DEVELOPED), 46);
        // Test 0051's size 33: 47 · 0.34 = 15.98.
        assert_eq!(full_radius(33.0, READOUT), 15);
        assert_eq!(full_radius(50.0, READOUT), 23);
        assert_eq!(full_radius(0.0, READOUT), 0);
        assert_eq!(full_radius(99.0, READOUT), 47);
        assert_eq!(full_radius(100.0, READOUT), 47);
        // 61 MP, 9504 × 6336: mrad 114, so size 0 blurs by 1.
        assert_eq!(max_radius([9504, 6336]), 114);
        assert_eq!(full_radius(0.0, [9504, 6336]), 1);
        assert_eq!(radii(33.0, READOUT, 1.0), [15; 8]);
    }

    #[test]
    fn the_radii_follow_the_pipes_scale() {
        // Decision 63 on test 0051's 15: mire1 fitted to 2400 wide, 9 in every pass; at
        // a half, two of 8 and six of 7, the export's variance exactly.
        assert_eq!(radii(33.0, READOUT, 2400.0 / 3908.0), [9; 8]);
        assert_eq!(radii(33.0, READOUT, 0.5), [8, 8, 7, 7, 7, 7, 7, 7]);
        assert_eq!(
            radii(33.0, READOUT, 488.0 / 3908.0),
            [2, 2, 2, 1, 1, 1, 1, 1]
        );
        assert_eq!(radii(0.0, READOUT, 0.1), [0; 8]);
    }

    #[test]
    fn each_iterations_row_and_column_passes_take_its_radius() {
        let input = ImageDesc::rgba(1954, 1301);
        let settings = Settings {
            size: 33.0,
            ..Settings::default()
        };
        let prepared = prepare(&settings, &input, [1954, 1301], READOUT, 0.5).unwrap();
        let taken: Vec<u32> = prepared.passes[1..17]
            .iter()
            .map(|pass| word(pass, 2))
            .collect();
        let expected: Vec<u32> = radii(33.0, READOUT, 0.5)
            .iter()
            .flat_map(|&radius| [radius, radius])
            .collect();
        assert_eq!(taken, expected);
        assert_eq!(taken[..6], [8, 8, 8, 8, 7, 7]);
    }

    #[test]
    fn the_factors_are_darktables() {
        assert_eq!(saturation_factor(100.0), 1.0);
        assert_eq!(saturation_factor(37.0), 0.37);
        // Test 0051's 0.5 EV.
        assert_eq!(brightness_factor(0.5), 1.0 / (-0.5_f32).exp2());
        assert!((brightness_factor(0.5) - 2f32.sqrt()).abs() < 1e-6);
        assert_eq!(brightness_factor(0.0), 1.0);
        assert_eq!(brightness_factor(-2.0), 0.25);
        assert_eq!(mix(75.0), 0.75);
        assert_eq!(mix(0.0), 0.0);
    }

    #[test]
    fn the_passes_are_the_softened_image_sixteen_box_means_and_main() {
        let prepared = prepared(&Settings::default(), 3908, 2602);
        let entry_points: Vec<&str> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(entry_points.len(), 18);
        assert_eq!(entry_points[0], "overexpose");
        assert!(entry_points[1..17].iter().all(|&e| e == "box_mean"));
        assert_eq!(entry_points[17], "main");
        assert_eq!(
            prepared.scratch,
            vec![
                Scratch {
                    bytes: 3908 * 2602 * 4
                };
                SCRATCH
            ]
        );
        // Rows, then columns, from A to B and back, each three planes.
        let (rows, columns) = (&prepared.passes[1], &prepared.passes[2]);
        assert_eq!(rows.size, Some([16, 163]));
        assert_eq!(columns.size, Some([16, 245]));
        assert_eq!((word(rows, 2), word(rows, 3)), (23, ROWS));
        assert_eq!(word(columns, 3), COLUMNS);
        let scratch = |pass: &Pass, binding: u32| {
            pass.bindings
                .iter()
                .find(|b| b.binding == binding)
                .map(|b| b.resource)
        };
        for plane in 0..3 {
            let (read, write) = (10 + plane as u32, 13 + plane as u32);
            assert_eq!(scratch(rows, read), Some(Resource::Scratch(A + plane)));
            assert_eq!(scratch(rows, write), Some(Resource::Scratch(B + plane)));
            assert_eq!(scratch(columns, read), Some(Resource::Scratch(B + plane)));
            assert_eq!(scratch(columns, write), Some(Resource::Scratch(A + plane)));
            assert_eq!(
                scratch(&prepared.passes[0], write),
                Some(Resource::Scratch(A + plane))
            );
            assert_eq!(
                scratch(&prepared.passes[17], read),
                Some(Resource::Scratch(A + plane))
            );
        }
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
            // The derivative entry points bind `main`'s but its output, and two more.
            assert!(storage <= 7, "{}", pass.entry_point);
            for binding in &pass.bindings {
                if let Resource::Scratch(index) = binding.resource {
                    assert!(index < prepared.scratch.len(), "{}", pass.entry_point);
                }
            }
            // Only `main` writes the output, and only it is per pixel; only it and
            // `overexpose` read the input.
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
        assert_eq!(bound, [0, 1, 2, 10, 11, 12]);
    }

    #[test]
    fn each_pass_holds_only_the_settings_it_reads() {
        // Decision 15: `overexpose` the saturation and the brightness, the box passes
        // the radius alone, `main` the mix; so a drag of the mix leaves every
        // whole-image pass's key unchanged.
        let base = Settings {
            size: 33.0,
            saturation: 100.0,
            brightness: 0.5,
            amount: 75.0,
        };
        let params = |settings: Settings| {
            prepared(&settings, 320, 200)
                .passes
                .iter()
                .map(|pass| pass.params.clone())
                .collect::<Vec<_>>()
        };
        let a = params(base);
        for moved in [
            Settings {
                saturation: 40.0,
                ..base
            },
            Settings {
                brightness: -1.0,
                ..base
            },
        ] {
            let b = params(moved);
            assert_ne!(a[0], b[0]);
            assert_eq!(a[1..], b[1..]);
        }
        let amount = params(Settings {
            amount: 20.0,
            ..base
        });
        assert_eq!(a[..17], amount[..17]);
        assert_ne!(a[17], amount[17]);
        // On 320 × 200, mrad is 3: size 100 gives a radius of 3, 33 one of 1.
        let size = params(Settings {
            size: 100.0,
            ..base
        });
        assert_eq!(a[0], size[0]);
        assert_ne!(a[1..17], size[1..17]);
        assert_eq!(a[17], size[17]);
    }

    #[test]
    fn a_tiles_passes_cover_the_whole_image() {
        // In a tile of an export, `overexpose` and `main` run over the tile and place it
        // in the whole image's scratch, which the box passes walk.
        let tile = ImageDesc::rgba(64, 48);
        let prepared = prepare(&Settings::default(), &tile, [300, 200], [300, 200], 1.0).unwrap();
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
        let prepared = prepare(
            &Settings {
                size: 33.0,
                saturation: 60.0,
                brightness: 0.5,
                amount: 75.0,
            },
            &ImageDesc::rgba(320, 200),
            [320, 200],
            READOUT,
            1.0,
        )
        .unwrap();
        let overexpose = &prepared.passes[0];
        assert_eq!((word(overexpose, 0), word(overexpose, 1)), (320, 200));
        assert_eq!(float(overexpose, 4), 0.6);
        assert_eq!(float(overexpose, 5), brightness_factor(0.5));
        assert_eq!(float(overexpose, 6), 0.0);
        let rows = &prepared.passes[1];
        assert_eq!(word(rows, 2), 15);
        let main = &prepared.passes[17];
        assert_eq!(float(main, 4), 0.0);
        assert_eq!(float(main, 6), 0.75);
    }
}
