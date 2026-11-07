// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2012-2026  darktable developers (ported from src/iop/bilat.c,
// src/common/locallaplacian.c, src/common/bilateral.c and src/common/math.h)
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

//! What darktable's `process` derives from the parameters and the region before it
//! filters, for both modes: the local laplacian filter's pyramid geometry
//! (`local_laplacian_internal`) and the bilateral grid's (`dt_bilateral_grid_size`), and
//! the passes over them. See `docs/ports/bilat.md`.

use apertrust_color::WorkingProfile;
use apertrust_plugin::{
    Binding, ImageDesc, NotPorted, Pass, PassKind, Prepared, Resource, Scratch,
};

/// darktable `dt_iop_bilat_mode_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    BilateralGrid = 0,
    LocalLaplacian = 1,
}

impl Mode {
    /// The schema's choices, in darktable's order, from its descriptions.
    pub const NAMES: [&'static str; 2] = ["bilateral_grid", "local_laplacian"];

    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(Self::BilateralGrid),
            1 => Some(Self::LocalLaplacian),
            _ => None,
        }
    }
}

/// darktable `dt_iop_bilat_params_t` version 3.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub mode: Mode,
    /// Local laplacian: highlights; bilateral grid: the range sigma (contrast).
    pub sigma_r: f32,
    /// Local laplacian: shadows; bilateral grid: the spatial sigma (coarseness), in
    /// pixels at full resolution.
    pub sigma_s: f32,
    pub detail: f32,
    /// Local laplacian: the mid-tone range.
    pub midtone: f32,
}

impl Default for Settings {
    /// darktable's introspection defaults, which a new instance gets.
    fn default() -> Self {
        Self {
            mode: Mode::LocalLaplacian,
            sigma_r: 0.5,
            sigma_s: 0.5,
            detail: 0.25,
            midtone: 0.5,
        }
    }
}

/// darktable's `max_levels`.
const MAX_LEVELS: u32 = 30;
/// Planes of a pyramid level's scratch buffer: the input's Gaussian, the six remapped
/// images' and the assembled output.
pub const PLANES: u32 = 8;

/// darktable `dl`: a size halved `level` times, as the pyramid's levels are.
pub fn downsampled(size: u32, level: u32) -> u32 {
    (0..level).fold(size, |size, _| (size - 1) / 2 + 1)
}

/// The local laplacian filter's pyramid for an image (`local_laplacian_internal`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pyramid {
    /// darktable `num_levels`: one per halving of the shorter side, at most 30.
    pub levels: u32,
    /// darktable `max_supp`, the padding on every side: 2 to the last level.
    pub padding: u32,
    /// Every level's width and height, level 0 the padded image.
    pub sizes: Vec<[u32; 2]>,
}

impl Pyramid {
    /// The pyramid of a `width` × `height` image, or `None` below 4 pixels a side,
    /// where darktable's filter writes nothing (2) or reads out of bounds (3).
    pub fn of(width: u32, height: u32) -> Option<Self> {
        let shorter = width.min(height);
        if shorter < 4 {
            return None;
        }
        // 31 - clz(shorter): the index of its highest bit.
        let levels = MAX_LEVELS.min(31 - shorter.leading_zeros());
        let last = levels - 1;
        let padding = 1 << last;
        let (w, h) = (width + 2 * padding, height + 2 * padding);
        let sizes = (0..=last)
            .map(|level| [downsampled(w, level), downsampled(h, level)])
            .collect();
        Some(Self {
            levels,
            padding,
            sizes,
        })
    }

    /// The last level, the coarsest.
    pub fn last(&self) -> u32 {
        self.levels - 1
    }

    /// Floats in one plane of `level`.
    pub fn plane(&self, level: u32) -> u32 {
        let [w, h] = self.sizes[level as usize];
        w * h
    }

    /// Bytes of `level`'s scratch buffer, for levels 1 to the last.
    pub fn bytes(&self, level: u32) -> u64 {
        u64::from(self.plane(level)) * u64::from(PLANES) * 4
    }
}

/// darktable's bilateral grid's limits on its nodes.
const MAX_RES_S: i32 = 3000;
const MAX_RES_R: i32 = 50;
/// darktable's lightness range, `L_range`.
const L_RANGE: f32 = 100.0;

/// The bilateral grid for an image (`dt_bilateral_grid_size`, `L_range` 100), each
/// value in f32 as darktable computes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// Nodes along x, y and lightness.
    pub size: [u32; 3],
    /// The grid's own sigmas, after its sizes were clamped, and their reciprocals.
    pub sigma_s: f32,
    pub sigma_r: f32,
    pub sigma_s_inv: f32,
    pub sigma_r_inv: f32,
}

impl Grid {
    /// The grid of a `width` × `height` image at spatial sigma `sigma_s`, in its pixels,
    /// and range sigma `sigma_r`, in lightness.
    pub fn of(width: u32, height: u32, sigma_s: f32, sigma_r: f32) -> Self {
        // Smaller sigmas leave grid points no pixel reaches.
        let sigma_s = if sigma_s < 0.5 { 0.5 } else { sigma_s };
        let (width, height) = (width as f32, height as f32);
        // darktable's (int) of roundf; `as` saturates the infinity `sigma_r` 0 gives,
        // as darktable's arm64 build converts it (docs/ports/bilat.md, Precision).
        let nodes = |extent: f32, sigma: f32, max: i32| {
            ((extent / sigma).round() as i32).clamp(4, max) as f32
        };
        let x = nodes(width, sigma_s, MAX_RES_S);
        let y = nodes(height, sigma_s, MAX_RES_S);
        let z = nodes(L_RANGE, sigma_r, MAX_RES_R);
        let sigma_s = (height / y).max(width / x);
        let sigma_r = L_RANGE / z;
        let sigma_s_inv = 1.0 / sigma_s;
        let sigma_r_inv = 1.0 / sigma_r;
        let size = [
            (width * sigma_s_inv).ceil() as u32 + 1,
            (height * sigma_s_inv).ceil() as u32 + 1,
            (L_RANGE * sigma_r_inv).ceil() as u32 + 1,
        ];
        Self {
            size,
            sigma_s,
            sigma_r,
            sigma_s_inv,
            sigma_r_inv,
        }
    }

    /// Bytes of one of the grid's two buffers.
    pub fn bytes(&self) -> u64 {
        self.size.iter().map(|&n| u64::from(n)).product::<u64>() * 4
    }
}

/// The uniform block after the pass header, as `BilatParams` in the kernel lays it out;
/// each pass fills only what it reads (decision 15).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Uniforms {
    image: [u32; 4],
    fine: [u32; 4],
    coarse: [u32; 4],
    curve: [f32; 4],
    grid: [f32; 4],
    grid_size: [u32; 4],
    slice: [f32; 4],
    rgb_to_xyz: [[f32; 3]; 3],
    xyz_to_rgb: [[f32; 3]; 3],
}

/// Bytes of `BilatParams`.
pub const PARAMS_SIZE: usize = 16 * 13;

impl Uniforms {
    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        for word in self.image.iter().chain(&self.fine).chain(&self.coarse) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in self.curve.iter().chain(&self.grid) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for word in &self.grid_size {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in &self.slice {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for row in self.rgb_to_xyz.iter().chain(&self.xyz_to_rgb) {
            for value in row.iter().chain(&[0.0]) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    /// Both colour conversions, for `main`, which reads the input and writes the output.
    fn with_colour(self, profile: &WorkingProfile) -> Self {
        Self {
            xyz_to_rgb: profile.xyz_d50_to_rgb,
            ..self.with_lightness(profile)
        }
    }

    /// The conversion to Lab alone, for a whole-image pass that reads the input's
    /// lightness.
    fn with_lightness(self, profile: &WorkingProfile) -> Self {
        Self {
            rgb_to_xyz: profile.rgb_to_xyz_d50,
            ..self
        }
    }
}

/// The kernel's mode for images too small for the pyramid, which pass through.
const COPY: u32 = 2;

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

const INPUT: (u32, Resource) = (0, Resource::Input);
const OUTPUT: (u32, Resource) = (1, Resource::Output);
const UNIFORMS: (u32, Resource) = (2, Resource::Uniforms);
/// The finer level, or the grid a pass reads.
fn fine(scratch: usize) -> (u32, Resource) {
    (10, Resource::Scratch(scratch))
}
/// The coarser level, or the grid a pass writes and `main` reads.
fn coarse(scratch: usize) -> (u32, Resource) {
    (11, Resource::Scratch(scratch))
}

/// The passes for `settings` on `input`, at `scale` pixels of the pipe per pixel at full
/// resolution, in the working space `profile` (darktable's `process`).
pub fn prepare(
    settings: &Settings,
    input: &ImageDesc,
    scale: f32,
    profile: &WorkingProfile,
) -> Result<Prepared, NotPorted> {
    let (width, height) = (input.width, input.height);
    let image = [width, height, 0, settings.mode as u32];
    let passes_and_scratch = match settings.mode {
        Mode::LocalLaplacian => match Pyramid::of(width, height) {
            Some(pyramid) => local_laplacian(settings, image, &pyramid, profile),
            None => copy(image, profile),
        },
        Mode::BilateralGrid => {
            // darktable's sigma_s / max(iscale / roi_in->scale, 1): the pipe's scale,
            // never above 1.
            let grid = Grid::of(
                width,
                height,
                settings.sigma_s * scale.min(1.0),
                settings.sigma_r,
            );
            bilateral_grid(settings, image, &grid, profile)
        }
    };
    let (passes, scratch) = passes_and_scratch;
    Ok(Prepared {
        output: *input,
        passes,
        scratch: scratch.into_iter().map(|bytes| Scratch { bytes }).collect(),
    })
}

/// One pass of `main`, which copies each pixel through Lab, with a scratch buffer of 16
/// bytes for the binding it reads in the other modes.
fn copy(image: [u32; 4], profile: &WorkingProfile) -> (Vec<Pass>, Vec<u64>) {
    let uniforms = Uniforms {
        image: [image[0], image[1], 0, COPY],
        ..Uniforms::default()
    }
    .with_colour(profile);
    let main = pass(
        "main",
        PassKind::PerPixel,
        uniforms,
        None,
        &[INPUT, OUTPUT, UNIFORMS, coarse(0)],
    );
    (vec![main], vec![16])
}

/// The local laplacian filter's passes: the pyramids reduced level by level, the first
/// from the input, the assembly coarse to fine down to level 1, and `main` for level 0
/// and the write-back. Level `l`'s buffer is scratch `l - 1`.
fn local_laplacian(
    settings: &Settings,
    image: [u32; 4],
    pyramid: &Pyramid,
    profile: &WorkingProfile,
) -> (Vec<Pass>, Vec<u64>) {
    let last = pyramid.last();
    let level = |l: u32, is_last: bool| {
        let [w, h] = pyramid.sizes[l as usize];
        [w, h, w * h, u32::from(is_last)]
    };
    let curve = [
        settings.midtone,
        settings.sigma_s,
        settings.sigma_r,
        settings.detail,
    ];
    let image = [image[0], image[1], pyramid.padding, image[3]];
    let scratch = |l: u32| l as usize - 1;
    let mut passes = vec![pass(
        "ll_reduce_input",
        PassKind::Reduction,
        Uniforms {
            image,
            coarse: level(1, last == 1),
            curve,
            ..Uniforms::default()
        }
        .with_lightness(profile),
        Some(pyramid.sizes[1]),
        &[INPUT, UNIFORMS, coarse(scratch(1))],
    )];
    for l in 2..=last {
        passes.push(pass(
            "ll_reduce",
            PassKind::Reduction,
            Uniforms {
                fine: level(l - 1, false),
                coarse: level(l, l == last),
                ..Uniforms::default()
            },
            Some(pyramid.sizes[l as usize]),
            &[UNIFORMS, fine(scratch(l - 1)), coarse(scratch(l))],
        ));
    }
    for l in (1..last).rev() {
        passes.push(pass(
            "ll_assemble",
            PassKind::Reduction,
            Uniforms {
                fine: level(l, false),
                coarse: level(l + 1, false),
                ..Uniforms::default()
            },
            Some(pyramid.sizes[l as usize]),
            &[UNIFORMS, fine(scratch(l)), coarse(scratch(l + 1))],
        ));
    }
    passes.push(pass(
        "main",
        PassKind::PerPixel,
        Uniforms {
            image,
            fine: level(0, false),
            coarse: level(1, false),
            curve,
            ..Uniforms::default()
        }
        .with_colour(profile),
        None,
        &[INPUT, OUTPUT, UNIFORMS, coarse(scratch(1))],
    ));
    let scratch = (1..=last).map(|l| pyramid.bytes(l)).collect();
    (passes, scratch)
}

/// The bilateral grid's passes: the splat into grid A, the blurs along x (A to B), y (B
/// to A) and lightness (A to B), and `main`, which slices B.
fn bilateral_grid(
    settings: &Settings,
    image: [u32; 4],
    grid: &Grid,
    profile: &WorkingProfile,
) -> (Vec<Pass>, Vec<u64>) {
    let [x, y, z] = grid.size;
    let grid_values = [
        grid.sigma_s_inv,
        grid.sigma_r_inv,
        grid.sigma_s * grid.sigma_s,
        grid.sigma_r,
    ];
    let (a, b) = (0, 1);
    let mut passes = vec![pass(
        "bl_splat",
        PassKind::Reduction,
        Uniforms {
            image,
            grid: grid_values,
            grid_size: [x, y, z, 0],
            ..Uniforms::default()
        }
        .with_lightness(profile),
        Some([x, y]),
        &[INPUT, UNIFORMS, coarse(a)],
    )];
    for (axis, from, to) in [(0, a, b), (1, b, a), (2, a, b)] {
        passes.push(pass(
            "bl_blur",
            PassKind::Reduction,
            Uniforms {
                grid_size: [x, y, z, axis],
                ..Uniforms::default()
            },
            Some([x * z, y]),
            &[UNIFORMS, fine(from), coarse(to)],
        ));
    }
    passes.push(pass(
        "main",
        PassKind::PerPixel,
        Uniforms {
            image,
            grid: grid_values,
            grid_size: [x, y, z, 0],
            slice: [settings.detail, 0.0, 0.0, 0.0],
            ..Uniforms::default()
        }
        .with_colour(profile),
        None,
        &[INPUT, OUTPUT, UNIFORMS, coarse(b)],
    ));
    (passes, vec![grid.bytes(); 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mire1s_pyramid_is_darktables() {
        // 3908×2602: the shorter side's highest bit is 2^11, so 11 levels.
        let pyramid = Pyramid::of(3908, 2602).unwrap();
        assert_eq!(pyramid.levels, 11);
        assert_eq!(pyramid.padding, 1024);
        assert_eq!(pyramid.sizes[0], [5956, 4650]);
        assert_eq!(pyramid.sizes[1], [2978, 2325]);
        assert_eq!(pyramid.sizes[10], [6, 5]);
        assert_eq!(pyramid.sizes.len(), 11);
    }

    #[test]
    fn the_pyramid_has_a_level_per_halving() {
        assert_eq!(Pyramid::of(2400, 1600).unwrap().levels, 10);
        assert_eq!(Pyramid::of(4, 4).unwrap().levels, 2);
        assert_eq!(Pyramid::of(7, 100).unwrap().levels, 2);
        assert_eq!(Pyramid::of(8, 8).unwrap().levels, 3);
        assert_eq!(Pyramid::of(1 << 31, 1 << 31).unwrap().levels, 30);
        assert!(Pyramid::of(3, 100).is_none());
        assert!(Pyramid::of(100, 1).is_none());
    }

    #[test]
    fn halving_rounds_up() {
        assert_eq!(downsampled(5956, 1), 2978);
        assert_eq!(downsampled(4650, 1), 2325);
        assert_eq!(downsampled(2325, 1), 1163);
        assert_eq!(downsampled(7, 0), 7);
    }

    #[test]
    fn the_edited_cases_grid_is_darktables() {
        // darktable-cli -d dev on test 0002 edited to the bilateral grid, coarseness 50
        // and contrast 20: "created grid [79 53 6] with sigma (50.102566 50.000000)
        // (20.000000 20.000000)".
        let grid = Grid::of(3908, 2602, 50.0, 20.0);
        assert_eq!(grid.size, [79, 53, 6]);
        assert_eq!(format!("{:.6}", grid.sigma_s), "50.102566");
        assert_eq!(grid.sigma_r, 20.0);
    }

    #[test]
    fn the_grid_clamps_its_nodes() {
        // At most 3000 nodes across, so the sigma grows; at least 4.
        let fine = Grid::of(9504, 6336, 0.1, 1.0);
        assert_eq!(fine.sigma_s, 9504.0 / 3000.0);
        assert_eq!(fine.size[0], 3001);
        assert_eq!(fine.size[2], 51);
        let coarse = Grid::of(100, 100, 100.0, 100.0);
        assert_eq!(coarse.size, [5, 5, 5]);
        // sigma_r 0's infinity saturates to darktable's arm64 50 levels.
        assert_eq!(Grid::of(100, 100, 1.0, 0.0).size[2], 51);
    }

    fn profile() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    fn mode(mode: Mode) -> Settings {
        Settings {
            mode,
            ..Settings::default()
        }
    }

    #[test]
    fn the_local_laplacian_has_two_passes_a_level() {
        let input = ImageDesc::rgba(3908, 2602);
        let prepared = prepare(&mode(Mode::LocalLaplacian), &input, 1.0, &profile()).unwrap();
        let entry_points: Vec<&str> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(entry_points.len(), 20);
        assert_eq!(entry_points[0], "ll_reduce_input");
        assert!(entry_points[1..10].iter().all(|&e| e == "ll_reduce"));
        assert!(entry_points[10..19].iter().all(|&e| e == "ll_assemble"));
        assert_eq!(entry_points[19], "main");
        assert_eq!(prepared.scratch.len(), 10);
        assert_eq!(prepared.scratch[0].bytes, 2978 * 2325 * 8 * 4);
        let total: u64 = prepared.scratch.iter().map(|s| s.bytes).sum();
        assert!((290_000_000..300_000_000).contains(&total), "{total}");
        assert_eq!(prepared.output, input);
        // The assembly runs from the level under the last down to 1.
        assert_eq!(prepared.passes[10].size, Some([12, 10]));
        assert_eq!(prepared.passes[18].size, Some([2978, 2325]));
    }

    #[test]
    fn the_bilateral_grid_has_five_passes() {
        let input = ImageDesc::rgba(3908, 2602);
        let settings = Settings {
            mode: Mode::BilateralGrid,
            sigma_s: 50.0,
            sigma_r: 20.0,
            ..Settings::default()
        };
        let prepared = prepare(&settings, &input, 1.0, &profile()).unwrap();
        let entry_points: Vec<&str> = prepared.passes.iter().map(|p| p.entry_point).collect();
        assert_eq!(
            entry_points,
            ["bl_splat", "bl_blur", "bl_blur", "bl_blur", "main"]
        );
        assert_eq!(prepared.scratch.len(), 2);
        assert_eq!(prepared.scratch[0].bytes, 79 * 53 * 6 * 4);
        assert_eq!(prepared.passes[0].size, Some([79, 53]));
        assert_eq!(prepared.passes[1].size, Some([79 * 6, 53]));
    }

    #[test]
    fn the_bilateral_grid_scales_its_spatial_sigma() {
        let settings = Settings {
            mode: Mode::BilateralGrid,
            sigma_s: 50.0,
            sigma_r: 20.0,
            ..Settings::default()
        };
        let half = prepare(&settings, &ImageDesc::rgba(1954, 1301), 0.5, &profile()).unwrap();
        let full = prepare(&settings, &ImageDesc::rgba(3908, 2602), 1.0, &profile()).unwrap();
        // About the same number of nodes at half the size and half the sigma.
        assert_eq!(half.passes[0].size, full.passes[0].size);
    }

    #[test]
    fn every_pass_binds_what_it_reads_within_eight_storage_buffers() {
        let input = ImageDesc::rgba(640, 480);
        for mode in [Mode::LocalLaplacian, Mode::BilateralGrid] {
            let prepared = prepare(&self::mode(mode), &input, 1.0, &profile()).unwrap();
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
                // Only `main` writes the output, and only it is per pixel.
                let writes_output = pass.bindings.iter().any(|b| b.resource == Resource::Output);
                assert_eq!(writes_output, pass.entry_point == "main");
                assert_eq!(
                    pass.kind == PassKind::PerPixel,
                    pass.entry_point == "main",
                    "{}",
                    pass.entry_point
                );
            }
            // `main` binds the same resources in either mode, so the derivative entry
            // points serve both.
            let main = prepared.passes.last().unwrap();
            let bound: Vec<u32> = main.bindings.iter().map(|b| b.binding).collect();
            assert_eq!(bound, [0, 1, 2, 11]);
        }
    }

    #[test]
    fn the_bilateral_grids_detail_reaches_only_main() {
        let settings = |detail| Settings {
            mode: Mode::BilateralGrid,
            sigma_s: 20.0,
            sigma_r: 10.0,
            detail,
            ..Settings::default()
        };
        let input = ImageDesc::rgba(320, 200);
        let a = prepare(&settings(0.25), &input, 1.0, &profile()).unwrap();
        let b = prepare(&settings(2.0), &input, 1.0, &profile()).unwrap();
        let (main_a, main_b) = (a.passes.last().unwrap(), b.passes.last().unwrap());
        assert_ne!(main_a.params, main_b.params);
        let others = |p: &Prepared| {
            p.passes[..p.passes.len() - 1]
                .iter()
                .map(|pass| pass.params.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(others(&a), others(&b));
    }

    #[test]
    fn small_images_pass_through() {
        let prepared = prepare(
            &mode(Mode::LocalLaplacian),
            &ImageDesc::rgba(3, 200),
            1.0,
            &profile(),
        )
        .unwrap();
        assert_eq!(prepared.passes.len(), 1);
        assert_eq!(prepared.passes[0].entry_point, "main");
        assert_eq!(prepared.scratch.len(), 1);
        let mode_word = u32::from_le_bytes(prepared.passes[0].params[12..16].try_into().unwrap());
        assert_eq!(mode_word, COPY);
    }

    #[test]
    fn the_uniforms_are_laid_out_as_the_kernel_reads_them() {
        let uniforms = Uniforms {
            image: [1, 2, 3, 4],
            curve: [0.5, 0.25, 0.125, 2.0],
            slice: [7.0, 0.0, 0.0, 0.0],
            ..Uniforms::default()
        }
        .with_colour(&profile());
        let bytes = uniforms.to_bytes();
        assert_eq!(bytes.len(), PARAMS_SIZE);
        let float =
            |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 4);
        assert_eq!(float(48), 0.5);
        assert_eq!(float(60), 2.0);
        assert_eq!(float(96), 7.0);
        assert_eq!(float(112), profile().rgb_to_xyz_d50[0][0]);
        assert_eq!(float(124), 0.0);
        assert_eq!(float(160), profile().xyz_d50_to_rgb[0][0]);
    }
}
