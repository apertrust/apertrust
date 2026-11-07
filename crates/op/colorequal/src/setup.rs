// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2022-2026  darktable developers (ported from src/iop/colorequal.c)
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

//! What darktable's `commit_params` and the start of `process` (`_prepare_process`)
//! derive from the parameters and the pipe's scale: the three hue curves as tables, the
//! working gamut's table, the saturation weights, the white and the filters' shifts,
//! sigmas and grids, and the passes over them. See `docs/ports/colorequal.md`.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

use std::f32::consts::PI;

use apertrust_color::gaussian::Deriche;
use apertrust_color::{self as color, Mat3, WorkingProfile, ucs};
use apertrust_plugin::{
    Binding, ImageDesc, NotPorted, Pass, PassKind, Prepared, Resource, Scratch,
};

/// darktable `NODES`: the nodes of each curve, red to magenta.
pub const NODES: usize = 8;
/// darktable `LUT_ELEM`: the entries of a hue-indexed table over [-π, π).
pub const LUT_SIZE: usize = color::GAMUT_LUT_SIZE;
/// darktable `SATSIZE`: the saturation weights' table holds `2 · SATSIZE + 1` entries.
pub const SATSIZE: usize = 4096;
/// darktable `SAT_EFFECT` and `BRIGHT_EFFECT`: how much a saturation or brightness
/// curve's departure from 1 changes the pixel.
pub const SAT_EFFECT: f32 = 2.0;
pub const BRIGHT_EFFECT: f32 = 8.0;
/// darktable `ANGLE_SHIFT`: red sits at 20° of hue in darktable UCS, so the nodes are
/// offset to put it at the curves' origin.
const ANGLE_SHIFT: f32 = 20.0;
/// darktable `chroma_feathering` and `param_feathering`, `powf(10, -5)` and
/// `powf(10, -6)`: the guided filters' regularisation.
pub const CHROMA_FEATHERING: f32 = 1e-5;
pub const PARAM_FEATHERING: f32 = 1e-6;

/// The module's settings: darktable params version 4, field for field. The node
/// arrays are darktable's eight fields each, red, orange, yellow, green, cyan, blue,
/// lavender and magenta; hues and the node placement in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub threshold: f32,
    pub smoothing_hue: f32,
    pub contrast: f32,
    pub white_level: f32,
    pub chroma_size: f32,
    pub param_size: f32,
    pub use_filter: bool,
    pub saturation: [f32; NODES],
    pub hue: [f32; NODES],
    pub brightness: [f32; NODES],
    pub hue_shift: f32,
}

impl Default for Settings {
    /// darktable's introspection defaults, which a new instance gets.
    fn default() -> Self {
        Self {
            threshold: 0.1,
            smoothing_hue: 1.0,
            contrast: 0.0,
            white_level: 1.0,
            chroma_size: 1.5,
            param_size: 1.0,
            use_filter: true,
            saturation: [1.0; NODES],
            hue: [0.0; NODES],
            brightness: [1.0; NODES],
            hue_shift: 0.0,
        }
    }
}

/// darktable `_get_hue_node`: the k-th node's hue in darktable UCS, in radians, with the
/// node placement.
pub fn hue_node(k: usize, hue_shift: f32) -> f32 {
    color::deg_to_rad(k as f32 * 360.0 / NODES as f32 + hue_shift + ANGLE_SHIFT)
}

/// darktable `pseudo_solve` (`src/iop/choleski.h`) for a square system: `A x = y` by the
/// normal equations `A'A x = A'y`, Choleski-Banachiewicz on their lower triangle, then
/// the triangular descent and ascent, in f32 and darktable's order. `None` where
/// darktable's solve fails (`A'A` not positive definite), where darktable leaves `y`
/// unsolved.
pub fn pseudo_solve(a: &[[f32; NODES]; NODES], y: &[f32; NODES]) -> Option<[f32; NODES]> {
    const N: usize = NODES;
    let mut a_square = [[0.0f32; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let mut sum = 0.0f32;
            for row in a {
                sum += row[i] * row[j];
            }
            a_square[i][j] = sum;
        }
    }
    let mut y_square = [0.0f32; N];
    for (i, value) in y_square.iter_mut().enumerate() {
        let mut sum = 0.0f32;
        for (row, &y) in a.iter().zip(y) {
            sum += row[i] * y;
        }
        *value = sum;
    }

    // _choleski_decompose
    if a_square[0][0] <= 0.0 {
        return None;
    }
    let mut l = [[0.0f32; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let mut sum = 0.0f32;
            for (a, b) in l[i][..j].iter().zip(&l[j][..j]) {
                sum += a * b;
            }
            if i == j {
                let temp = a_square[i][i] - sum;
                if temp < 0.0 {
                    return None;
                }
                l[i][j] = (a_square[i][i] - sum).sqrt();
            } else {
                let temp = l[j][j];
                if temp == 0.0 {
                    return None;
                }
                l[i][j] = (a_square[i][j] - sum) / temp;
            }
        }
    }
    // _triangular_descent: L b = A'y.
    let mut b = [0.0f32; N];
    for i in 0..N {
        let mut sum = y_square[i];
        for j in 0..i {
            sum -= l[i][j] * b[j];
        }
        if l[i][i] == 0.0 {
            return None;
        }
        b[i] = sum / l[i][i];
    }
    // _triangular_ascent: L' x = b.
    let mut x = [0.0f32; N];
    for i in (0..N).rev() {
        let mut sum = b[i];
        for j in (i + 1..N).rev() {
            sum -= l[j][i] * x[j];
        }
        if l[i][i] == 0.0 {
            return None;
        }
        x[i] = sum / l[i][i];
    }
    Some(x)
}

/// darktable `_cosine_coeffs`.
fn cosine_coeff(l: f32, c: f32) -> f32 {
    (-l * l / c).exp()
}

/// The radial basis of `_periodic_RBF_interpolate` at angular distance `distance`:
/// `exp(Σ_l exp(-l²/c) · cos(l · |distance|))` over its `m` terms.
fn basis(distance: f32, smoothing: f32, terms: i32) -> f32 {
    let mut sum = 0.0f32;
    for l in 0..terms {
        sum += cosine_coeff(l as f32, smoothing) * (l as f32 * distance.abs()).cos();
    }
    sum.exp()
}

/// darktable `_periodic_RBF_interpolate`: the curve through `nodes` at the nodes' hues,
/// a periodic interpolation by radial basis functions, sampled every 360/512 degrees from
/// -π, the table [`LUT_SIZE`] entries long, clipped at 0 with `clip`. In f32 as
/// darktable; where darktable's solve fails, the nodes themselves weigh the bases, as
/// darktable leaves them.
pub fn periodic_rbf(nodes: &[f32; NODES], smoothing: f32, hue_shift: f32, clip: bool) -> Vec<f32> {
    let terms = (3.0 * smoothing.sqrt()).ceil() as i32;
    let mut a = [[0.0f32; NODES]; NODES];
    for (i, row) in a.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = basis(
                hue_node(i, hue_shift) - hue_node(j, hue_shift),
                smoothing,
                terms,
            );
        }
    }
    let weights = pseudo_solve(&a, nodes).unwrap_or(*nodes);
    (0..LUT_SIZE)
        .map(|i| {
            let hue = color::deg_to_rad(i as f32 * 360.0 / LUT_SIZE as f32) - PI;
            let mut value = 0.0f32;
            for (k, &weight) in weights.iter().enumerate() {
                value += weight * basis(hue - hue_node(k, hue_shift), smoothing, terms);
            }
            if clip { value.max(0.0) } else { value }
        })
        .collect()
}

/// darktable `_init_satweights`: the logistic weight of a saturation, `2 · SATSIZE + 1`
/// entries for saturations -1 to 1, computed in f64 and stored as f32, steeper with the
/// contrast.
pub fn satweights(contrast: f32) -> Vec<f32> {
    let factor = -60.0 - 40.0 * f64::from(contrast);
    (-(SATSIZE as i32)..=SATSIZE as i32)
        .map(|i| {
            let val = 0.5 / SATSIZE as f64 * f64::from(i);
            (1.0 / (1.0 + (factor * val).exp())) as f32
        })
        .collect()
}

/// darktable `_get_satweight`: the weights' table interpolated at a saturation.
pub fn satweight(table: &[f32], sat: f32) -> f32 {
    let size = SATSIZE as f32;
    let isat = size * (1.0 + sat.clamp(-1.0, 1.0 - 1.0 / size));
    let base = isat.floor();
    let i = base as usize;
    table[i] + (isat - base) * (table[i + 1] - table[i])
}

/// The three curves as tables over hue, as `commit_params` builds them: saturation and
/// brightness clipped at 0, the hue's in radians.
#[derive(Debug, Clone, PartialEq)]
pub struct Curves {
    pub hue: Vec<f32>,
    pub saturation: Vec<f32>,
    pub brightness: Vec<f32>,
}

impl Curves {
    /// darktable `commit_params`' three `_periodic_RBF_interpolate` calls.
    pub fn new(settings: &Settings) -> Self {
        let s = settings;
        Self {
            saturation: periodic_rbf(&s.saturation, PI, s.hue_shift, true),
            hue: periodic_rbf(
                &s.hue.map(color::deg_to_rad),
                hue_smoothing(s),
                s.hue_shift,
                false,
            ),
            brightness: periodic_rbf(&s.brightness, PI, s.hue_shift, true),
        }
    }
}

/// The hue curve's smoothing: `1.f / p->smoothing_hue * M_PI_F`.
fn hue_smoothing(settings: &Settings) -> f32 {
    1.0 / settings.smoothing_hue * PI
}

/// The derivative of each curve's table with respect to its red node (node 0), the
/// first three gradient slots (docs/ports/colorequal.md, Gradients): the
/// curve is linear in its nodes, so this is the curve through a 1 at red and 0 elsewhere,
/// per degree for the hue's, and 0 where the saturation or brightness curve is clipped.
pub fn red_node_derivatives(settings: &Settings, curves: &Curves) -> [Vec<f32>; 3] {
    let s = settings;
    let mut unit = [0.0; NODES];
    unit[0] = 1.0;
    let per_degree = color::deg_to_rad(1.0);
    let hue = periodic_rbf(&unit, hue_smoothing(s), s.hue_shift, false)
        .iter()
        .map(|w| w * per_degree)
        .collect();
    let unclipped = |nodes: &[f32; NODES]| periodic_rbf(nodes, PI, s.hue_shift, false);
    let held = |nodes: &[f32; NODES]| -> Vec<f32> {
        unclipped(&unit)
            .iter()
            .zip(unclipped(nodes))
            .map(|(&w, value)| if value < 0.0 { 0.0 } else { w })
            .collect()
    };
    debug_assert_eq!(curves.hue.len(), LUT_SIZE);
    [hue, held(&s.saturation), held(&s.brightness)]
}

/// What `_prepare_process` derives from the parameters and the pipe's scale, `scale`
/// pixels of the pipe per pixel at full resolution, 1 in an export: darktable's
/// `roi_in->scale / piece->iscale`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Process {
    /// The white's lightness L*, `Y_to_dt_UCS_L_star(exp2f(white_level))`.
    pub white: f32,
    /// The saturation weights' shifts: `sat_shift`, darktable's `d->threshold`, and
    /// `bright_shift`, raised with the brightest node and the effect radius.
    pub sat_shift: f32,
    pub bright_shift: f32,
    /// `gradient_amp`, the saturation gradient's gain.
    pub gradient_amp: f32,
    /// The Gaussians' σ at the pipe's scale: the prefilter's (`hue_sigma`), the guide's
    /// (`par_sigma`), the saturation's and its gradient's.
    pub hue_sigma: f32,
    pub par_sigma: f32,
    pub sat_sigma: f32,
    pub scharr_sigma: f32,
}

impl Process {
    pub fn new(settings: &Settings, scale: f32) -> Self {
        let s = settings;
        // commit_params: the inflection point, 6% at the default threshold.
        let threshold = -0.015 + 0.3 * sqr(5.0 * s.threshold);
        let max_brightness = s.brightness.iter().fold(1.0f32, |m, &v| m.max(v));
        let max_brightness_shift = 0.01 * max_brightness;
        let corr_max_brightness_shift = max_brightness_shift * 5.0f32.min(s.param_size.sqrt());
        Self {
            white: ucs::y_to_l_star(s.white_level.exp2()),
            sat_shift: threshold,
            bright_shift: threshold + corr_max_brightness_shift,
            gradient_amp: 4.0 * max_brightness.sqrt() * sqr(scale),
            hue_sigma: 0.5 * s.chroma_size * scale,
            par_sigma: 0.5 * s.param_size * scale,
            sat_sigma: 0.5f32.max(scale),
            scharr_sigma: 0.5f32.max(scale),
        }
    }
}

fn sqr(x: f32) -> f32 {
    x * x
}

/// darktable `_get_scaling`: a guided filter's downsampling, 1 to 4.
fn scaling(sigma: f32) -> f32 {
    1.0f32.max(4.0f32.min((sigma - 1.5).floor()))
}

/// A guided filter's grid, as `_prefilter_chromaticity` and `_guide_with_chromaticity`
/// downsample for speed: by `_get_scaling(sigma)`, 1 to 4, the Gaussians' σ on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// `ds_width` and `ds_height`, `(int)(width / scaling)`, at least 1.
    pub size: [u32; 2],
    /// `gsigma`, `MAX(0.2f, sigma / scaling)`.
    pub sigma: f32,
    /// Whether the grid is smaller than the image, which darktable calls `resized`.
    pub resized: bool,
    /// Whether the filter blurs the image's own buffers in place, as darktable does
    /// where its grid is not resized: the prefilter then mixes towards its blur of the
    /// chromaticity, and the guide blurs the hue correction and applies to the blurred
    /// chromaticity.
    pub in_place: bool,
}

impl Grid {
    /// darktable's grid for σ `sigma` on an image `whole` pixels across and down,
    /// blurring in place where it is the image.
    pub fn new(sigma: f32, whole: [u32; 2]) -> Self {
        let scaling = scaling(sigma);
        let size = whole.map(|side| ((side as f32 / scaling) as u32).max(1));
        let resized = size != whole;
        Self {
            size,
            sigma: 0.2f32.max(sigma / scaling),
            resized,
            in_place: !resized,
        }
    }

    /// The grid at a pipe's scale, σ `sigma` there, darktable's at that scale, blurring
    /// in place as the export does, where σ is `full_sigma`: in place only where the
    /// export's grid is the image (docs/ports/colorequal.md, Neighbourhood and scale).
    /// At scale 1, darktable's.
    pub fn scaled(sigma: f32, whole: [u32; 2], full_sigma: f32) -> Self {
        Self {
            in_place: scaling(full_sigma) == 1.0,
            ..Self::new(sigma, whole)
        }
    }

    fn pixels(&self) -> u64 {
        u64::from(self.size[0]) * u64::from(self.size[1])
    }
}

/// Bytes of the kernel's `ColorEqualParams`.
pub const PARAMS_SIZE: usize = 13 * 16;

/// The uniform block after the pass header, as `ColorEqualParams` lays it out; each
/// pass fills only what it reads (decision 15).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Uniforms {
    /// The whole image at the pipe's scale, and the grid a grid or line pass covers.
    size: [u32; 4],
    /// A line pass's axis (0 the columns, 1 the rows), a grid's `resized`, whether
    /// `main` reads the guided corrections, and whether a filter blurs in place
    /// (`Grid::in_place`).
    mode: [u32; 4],
    deriche_a: [f32; 4],
    deriche_b: [f32; 4],
    /// `sat_shift`, `bright_shift`, a filter's feathering, `gradient_amp`.
    shifts: [f32; 4],
    /// The white's L*.
    white: [f32; 4],
    /// The gradient slots' values: the red node of the hue, saturation and brightness
    /// curves, and the white level.
    slots: [f32; 4],
    rgb_to_xyz: Mat3,
    xyz_to_rgb: Mat3,
}

impl Uniforms {
    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        for word in self.size.iter().chain(&self.mode) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for value in [
            self.deriche_a,
            self.deriche_b,
            self.shifts,
            self.white,
            self.slots,
        ]
        .iter()
        .flatten()
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for row in self.rgb_to_xyz.iter().chain(&self.xyz_to_rgb) {
            for value in row.iter().chain(&[0.0]) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    fn deriche(mut self, sigma: f32) -> Self {
        let d = Deriche::new(sigma);
        self.deriche_a = [d.a0, d.a1, d.a2, d.a3];
        self.deriche_b = [d.b1, d.b2, d.coefp, d.coefn];
        self
    }
}

/// The kernel's bindings, one per role a pass gives a scratch buffer (see
/// `slang/colorequal.slang`).
mod binding {
    pub const UV: u32 = 10;
    pub const GRID_UV: u32 = 11;
    pub const P1: u32 = 12;
    pub const P2: u32 = 13;
    pub const C1: u32 = 14;
    pub const C2: u32 = 15;
    pub const CORRECTIONS: u32 = 16;
    pub const WEIGHTS: u32 = 17;
    pub const SATURATION: u32 = 18;
    pub const LIGHTNESS: u32 = 19;
    pub const BRIGHTNESS: u32 = 20;
    pub const LINE2_IN: u32 = 21;
    pub const LINE2_OUT: u32 = 22;
    pub const LINE1_IN: u32 = 23;
    pub const LINE1_OUT: u32 = 24;
    pub const GRID_CORRECTIONS: u32 = 25;
    pub const GRID_BRIGHTNESS: u32 = 26;
}

/// The scratch buffers, by index in [`Prepared::scratch`] with the filter on.
mod scratch {
    /// The chromaticity U*′V*′ of the whole image, prefiltered in place: 8 bytes a pixel.
    pub const UV: usize = 0;
    /// The saturation, blurred in place: 4.
    pub const SATURATION: usize = 1;
    /// L*, then the saturation's gradient: 4.
    pub const LIGHTNESS: usize = 2;
    /// The hue and saturation corrections: 8.
    pub const CORRECTIONS: usize = 3;
    /// The brightness correction: 4.
    pub const BRIGHTNESS: usize = 4;
    /// The saturation weights, `sat_shift`'s and `bright_shift`'s: 8.
    pub const WEIGHTS: usize = 5;
    /// The Gaussians' columns, the largest grid's float2 or the image's float.
    pub const TEMP: usize = 6;
    /// The filters' grids: the chromaticity, the covariance's halves, then the
    /// coefficients (`a` in P1 and P2, `b` in C1); C1 and C2 also the guide's
    /// correlations.
    pub const GRID_UV: usize = 7;
    pub const P1: usize = 8;
    pub const P2: usize = 9;
    pub const C1: usize = 10;
    pub const C2: usize = 11;
    /// The guide's corrections on its grid, downsampled or copied, only when it does
    /// not blur in place.
    pub const GRID_CORRECTIONS: usize = 12;
    pub const GRID_BRIGHTNESS: usize = 13;
}

/// The tables' layout: each hue-indexed table [`LUT_SIZE`] long, `main`'s seven, the
/// corrections' three then the weights, the prefilter's the weights alone.
pub mod table {
    use super::LUT_SIZE;
    pub const HUE: usize = 0;
    pub const SATURATION: usize = LUT_SIZE;
    pub const BRIGHTNESS: usize = 2 * LUT_SIZE;
    pub const GAMUT: usize = 3 * LUT_SIZE;
    pub const RED_HUE: usize = 4 * LUT_SIZE;
    pub const RED_SATURATION: usize = 5 * LUT_SIZE;
    pub const RED_BRIGHTNESS: usize = 6 * LUT_SIZE;
    /// The saturation weights in the corrections' table.
    pub const WEIGHTS: usize = 3 * LUT_SIZE;
}

fn floats(values: &[&[f32]]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|part| part.iter())
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// A pass of the kernel.
fn pass(
    entry_point: &'static str,
    kind: PassKind,
    uniforms: Uniforms,
    tables: Vec<Vec<u8>>,
    size: Option<[u32; 2]>,
    bindings: &[(u32, Resource)],
) -> Pass {
    Pass {
        entry_point,
        kind,
        params: uniforms.to_bytes(),
        tables,
        bindings: bindings
            .iter()
            .map(|&(binding, resource)| Binding::new(binding, resource))
            .collect(),
        size,
        strips: None,
    }
}

/// A line pass's size: one invocation per line, in rows of 16.
fn lines(count: u32) -> Option<[u32; 2]> {
    Some([16, count.div_ceil(16)])
}

const INPUT: (u32, Resource) = (0, Resource::Input);
const OUTPUT: (u32, Resource) = (1, Resource::Output);
const UNIFORMS: (u32, Resource) = (2, Resource::Uniforms);
const TABLE: (u32, Resource) = (3, Resource::Table(0));

fn at(binding: u32, scratch: usize) -> (u32, Resource) {
    (binding, Resource::Scratch(scratch))
}

/// `dt_gaussian_mean_blur` of `buffer`, `grid` pixels, `channels` 1 or 2, at `sigma`, in
/// place: down its columns into the temporary buffer, then along its rows back.
fn blur(buffer: usize, grid: [u32; 2], channels: u32, sigma: f32, whole: [u32; 2]) -> [Pass; 2] {
    let (columns, rows, line_in, line_out) = match channels {
        1 => (
            "blur_columns1",
            "blur_rows1",
            binding::LINE1_IN,
            binding::LINE1_OUT,
        ),
        _ => (
            "blur_columns2",
            "blur_rows2",
            binding::LINE2_IN,
            binding::LINE2_OUT,
        ),
    };
    let uniforms = |axis: u32| {
        Uniforms {
            size: [whole[0], whole[1], grid[0], grid[1]],
            mode: [axis, 0, 0, 0],
            ..Uniforms::default()
        }
        .deriche(sigma)
    };
    [
        pass(
            columns,
            PassKind::Reduction,
            uniforms(0),
            Vec::new(),
            lines(grid[0]),
            &[UNIFORMS, at(line_in, buffer), at(line_out, scratch::TEMP)],
        ),
        pass(
            rows,
            PassKind::Reduction,
            uniforms(1),
            Vec::new(),
            lines(grid[1]),
            &[UNIFORMS, at(line_in, scratch::TEMP), at(line_out, buffer)],
        ),
    ]
}

/// What `prepare` builds the passes from: the settings' tables and values at the pipe's
/// scale, in the working space `profile`.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    pub curves: Curves,
    pub gamut: Vec<f32>,
    pub weights: Vec<f32>,
    pub process: Process,
    /// The prefilter's and the guide's grids.
    pub prefilter: Grid,
    pub guide: Grid,
    pub rgb_to_xyz_d65: Mat3,
    pub xyz_d65_to_rgb: Mat3,
}

impl Setup {
    /// darktable `commit_params` and `_prepare_process` for an image `whole` pixels at
    /// the pipe's scale, `scale` pixels per pixel at full resolution.
    pub fn new(settings: &Settings, whole: [u32; 2], scale: f32, profile: &WorkingProfile) -> Self {
        let rgb_to_xyz_d65 = color::mul(&color::XYZ_D50_TO_D65_CAT16, &profile.rgb_to_xyz_d50);
        let process = Process::new(settings, scale);
        let full = Process::new(settings, 1.0);
        Self {
            curves: Curves::new(settings),
            gamut: ucs::gamut_lut(&rgb_to_xyz_d65),
            weights: satweights(settings.contrast),
            process,
            prefilter: Grid::scaled(process.hue_sigma, whole, full.hue_sigma),
            guide: Grid::scaled(process.par_sigma, whole, full.par_sigma),
            rgb_to_xyz_d65,
            xyz_d65_to_rgb: color::mul(&profile.xyz_d50_to_rgb, &color::XYZ_D65_TO_D50_CAT16),
        }
    }
}

/// The passes for `settings` on `input`, part of an image `whole` pixels at the pipe's
/// scale (all of it but in a tile of an export), at `scale` pixels of the pipe per pixel
/// at full resolution, in the working space `profile` (darktable's `process`). Every
/// setting is ported, so it always succeeds; the `Result` is the operation interface's.
pub fn prepare(
    settings: &Settings,
    input: &ImageDesc,
    whole: [u32; 2],
    scale: f32,
    profile: &WorkingProfile,
) -> Result<Prepared, NotPorted> {
    let setup = Setup::new(settings, whole, scale, profile);
    let s = settings;
    let p = &setup.process;
    let [red_hue, red_saturation, red_brightness] = red_node_derivatives(s, &setup.curves);
    let main_table = floats(&[
        &setup.curves.hue,
        &setup.curves.saturation,
        &setup.curves.brightness,
        &setup.gamut,
        &red_hue,
        &red_saturation,
        &red_brightness,
    ]);
    let main_uniforms = Uniforms {
        size: [whole[0], whole[1], 0, 0],
        mode: [0, 0, u32::from(s.use_filter), 0],
        white: [p.white, 0.0, 0.0, 0.0],
        slots: [s.hue[0], s.saturation[0], s.brightness[0], s.white_level],
        rgb_to_xyz: setup.rgb_to_xyz_d65,
        xyz_to_rgb: setup.xyz_d65_to_rgb,
        ..Uniforms::default()
    };
    let main = |uv: usize, corrections: usize, brightness: usize| {
        pass(
            "main",
            PassKind::PerPixel,
            main_uniforms,
            vec![main_table.clone()],
            None,
            &[
                INPUT,
                OUTPUT,
                UNIFORMS,
                TABLE,
                at(binding::UV, uv),
                at(binding::CORRECTIONS, corrections),
                at(binding::BRIGHTNESS, brightness),
            ],
        )
    };

    if !s.use_filter {
        // Per pixel, as darktable without its guided filter: `main` computes the
        // corrections itself and binds three placeholders where the guided `main` reads
        // its scratch, so both end in the same entry point and the derivative entry
        // points serve both.
        return Ok(Prepared {
            output: *input,
            passes: vec![main(0, 1, 2)],
            scratch: vec![Scratch { bytes: 16 }; 3],
        });
    }

    let pixels = u64::from(whole[0]) * u64::from(whole[1]);
    let (prefilter, guide) = (setup.prefilter, setup.guide);
    let image_uniforms = Uniforms {
        size: [whole[0], whole[1], 0, 0],
        ..Uniforms::default()
    };
    let grid_uniforms = |grid: &Grid| Uniforms {
        size: [whole[0], whole[1], grid.size[0], grid.size[1]],
        mode: [0, u32::from(grid.resized), 0, u32::from(grid.in_place)],
        ..Uniforms::default()
    };

    let mut passes = vec![pass(
        "sample",
        PassKind::Reduction,
        Uniforms {
            rgb_to_xyz: setup.rgb_to_xyz_d65,
            ..image_uniforms
        },
        Vec::new(),
        None,
        &[
            INPUT,
            UNIFORMS,
            at(binding::UV, scratch::UV),
            at(binding::SATURATION, scratch::SATURATION),
            at(binding::LIGHTNESS, scratch::LIGHTNESS),
        ],
    )];
    passes.extend(blur(scratch::SATURATION, whole, 1, p.sat_sigma, whole));

    // STEP 2: _prefilter_chromaticity.
    let weights_table = floats(&[&setup.weights]);
    passes.push(pass(
        "prefilter_init",
        PassKind::Reduction,
        grid_uniforms(&prefilter),
        Vec::new(),
        Some(prefilter.size),
        &[
            UNIFORMS,
            at(binding::UV, scratch::UV),
            at(binding::GRID_UV, scratch::GRID_UV),
            at(binding::P1, scratch::P1),
            at(binding::P2, scratch::P2),
        ],
    ));
    for buffer in [scratch::GRID_UV, scratch::P1, scratch::P2] {
        passes.extend(blur(buffer, prefilter.size, 2, prefilter.sigma, whole));
    }
    passes.push(pass(
        "prefilter_coefficients",
        PassKind::Reduction,
        Uniforms {
            shifts: [0.0, 0.0, CHROMA_FEATHERING, 0.0],
            ..grid_uniforms(&prefilter)
        },
        Vec::new(),
        Some(prefilter.size),
        &[
            UNIFORMS,
            at(binding::GRID_UV, scratch::GRID_UV),
            at(binding::P1, scratch::P1),
            at(binding::P2, scratch::P2),
            at(binding::C1, scratch::C1),
        ],
    ));
    for buffer in [scratch::P1, scratch::P2, scratch::C1] {
        passes.extend(blur(buffer, prefilter.size, 2, prefilter.sigma, whole));
    }
    passes.push(pass(
        "prefilter_apply",
        PassKind::Reduction,
        Uniforms {
            shifts: [p.sat_shift, 0.0, 0.0, 0.0],
            ..grid_uniforms(&prefilter)
        },
        vec![weights_table],
        Some(whole),
        &[
            UNIFORMS,
            TABLE,
            at(binding::UV, scratch::UV),
            at(binding::GRID_UV, scratch::GRID_UV),
            at(binding::P1, scratch::P1),
            at(binding::P2, scratch::P2),
            at(binding::C1, scratch::C1),
            at(binding::SATURATION, scratch::SATURATION),
        ],
    ));

    // STEP 3: HSB, the saturation's gradient, the corrections and the weights.
    passes.push(pass(
        "corrections_pass",
        PassKind::Reduction,
        Uniforms {
            shifts: [p.sat_shift, p.bright_shift, 0.0, p.gradient_amp],
            white: [p.white, 0.0, 0.0, 0.0],
            ..image_uniforms
        },
        vec![floats(&[
            &setup.curves.hue,
            &setup.curves.saturation,
            &setup.curves.brightness,
            &setup.weights,
        ])],
        Some(whole),
        &[
            UNIFORMS,
            TABLE,
            at(binding::UV, scratch::UV),
            at(binding::SATURATION, scratch::SATURATION),
            at(binding::LIGHTNESS, scratch::LIGHTNESS),
            at(binding::CORRECTIONS, scratch::CORRECTIONS),
            at(binding::BRIGHTNESS, scratch::BRIGHTNESS),
            at(binding::WEIGHTS, scratch::WEIGHTS),
        ],
    ));
    passes.extend(blur(scratch::LIGHTNESS, whole, 1, p.scharr_sigma, whole));

    // STEP 4: _guide_with_chromaticity. Blurring in place, darktable blurs the
    // corrections themselves, the hue's included, which `main` then reads; otherwise
    // copies on the guide's grid.
    let (grid_corrections, grid_brightness) = if !guide.in_place {
        passes.push(pass(
            "guide_downsample",
            PassKind::Reduction,
            grid_uniforms(&guide),
            Vec::new(),
            Some(guide.size),
            &[
                UNIFORMS,
                at(binding::CORRECTIONS, scratch::CORRECTIONS),
                at(binding::BRIGHTNESS, scratch::BRIGHTNESS),
                at(binding::GRID_CORRECTIONS, scratch::GRID_CORRECTIONS),
                at(binding::GRID_BRIGHTNESS, scratch::GRID_BRIGHTNESS),
            ],
        ));
        (scratch::GRID_CORRECTIONS, scratch::GRID_BRIGHTNESS)
    } else {
        (scratch::CORRECTIONS, scratch::BRIGHTNESS)
    };
    passes.push(pass(
        "guide_init",
        PassKind::Reduction,
        grid_uniforms(&guide),
        Vec::new(),
        Some(guide.size),
        &[
            UNIFORMS,
            at(binding::UV, scratch::UV),
            at(binding::GRID_CORRECTIONS, grid_corrections),
            at(binding::GRID_BRIGHTNESS, grid_brightness),
            at(binding::GRID_UV, scratch::GRID_UV),
            at(binding::P1, scratch::P1),
            at(binding::P2, scratch::P2),
            at(binding::C1, scratch::C1),
            at(binding::C2, scratch::C2),
        ],
    ));
    for buffer in [
        scratch::GRID_UV,
        scratch::P1,
        scratch::P2,
        grid_corrections,
        scratch::C1,
        scratch::C2,
    ] {
        passes.extend(blur(buffer, guide.size, 2, guide.sigma, whole));
    }
    passes.extend(blur(
        grid_brightness,
        guide.size,
        1,
        0.1 * guide.sigma,
        whole,
    ));
    passes.push(pass(
        "guide_coefficients",
        PassKind::Reduction,
        Uniforms {
            shifts: [0.0, 0.0, PARAM_FEATHERING, 0.0],
            ..grid_uniforms(&guide)
        },
        Vec::new(),
        Some(guide.size),
        &[
            UNIFORMS,
            at(binding::GRID_UV, scratch::GRID_UV),
            at(binding::GRID_CORRECTIONS, grid_corrections),
            at(binding::GRID_BRIGHTNESS, grid_brightness),
            at(binding::P1, scratch::P1),
            at(binding::P2, scratch::P2),
            at(binding::C1, scratch::C1),
            at(binding::C2, scratch::C2),
        ],
    ));
    for buffer in [scratch::P1, scratch::P2, scratch::C1] {
        passes.extend(blur(buffer, guide.size, 2, guide.sigma, whole));
    }
    // The chromaticity the guide applies to: its blur, as darktable's in-place blur
    // leaves it, or the image's own, prefiltered.
    let base = if !guide.in_place {
        scratch::UV
    } else {
        scratch::GRID_UV
    };
    passes.push(pass(
        "guide_apply",
        PassKind::Reduction,
        grid_uniforms(&guide),
        Vec::new(),
        Some(whole),
        &[
            UNIFORMS,
            at(binding::GRID_UV, base),
            at(binding::P1, scratch::P1),
            at(binding::P2, scratch::P2),
            at(binding::C1, scratch::C1),
            at(binding::WEIGHTS, scratch::WEIGHTS),
            at(binding::LIGHTNESS, scratch::LIGHTNESS),
            at(binding::CORRECTIONS, scratch::CORRECTIONS),
            at(binding::BRIGHTNESS, scratch::BRIGHTNESS),
        ],
    ));

    // STEP 5.
    passes.push(main(scratch::UV, scratch::CORRECTIONS, scratch::BRIGHTNESS));

    let largest_grid = prefilter.pixels().max(guide.pixels());
    let mut buffers = vec![
        Scratch { bytes: 8 * pixels },
        Scratch { bytes: 4 * pixels },
        Scratch { bytes: 4 * pixels },
        Scratch { bytes: 8 * pixels },
        Scratch { bytes: 4 * pixels },
        Scratch { bytes: 8 * pixels },
        Scratch {
            bytes: (4 * pixels).max(8 * largest_grid),
        },
        Scratch {
            bytes: 8 * largest_grid,
        },
        Scratch {
            bytes: 8 * largest_grid,
        },
        Scratch {
            bytes: 8 * largest_grid,
        },
        Scratch {
            bytes: 8 * largest_grid,
        },
        Scratch {
            bytes: 8 * guide.pixels(),
        },
    ];
    if !guide.in_place {
        buffers.push(Scratch {
            bytes: 8 * guide.pixels(),
        });
        buffers.push(Scratch {
            bytes: 4 * guide.pixels(),
        });
    }
    Ok(Prepared {
        output: *input,
        passes,
        scratch: buffers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    #[test]
    fn the_default_curves_are_flat_but_for_the_rbfs_ripple() {
        // Nodes at 1 interpolate 1 at every node; between them a radial basis
        // interpolation ripples. Hue nodes at 0 give 0 exactly.
        let curves = Curves::new(&Settings::default());
        assert!(curves.hue.iter().all(|&v| v == 0.0));
        let ripple = curves
            .saturation
            .iter()
            .map(|v| (v - 1.0).abs())
            .fold(0.0, f32::max);
        // 1.5e-3 at darktable's smoothing, the curve dipping between the nodes.
        assert!(ripple > 1e-3 && ripple < 2e-3, "{ripple}");
        assert_eq!(curves.saturation, curves.brightness);
    }

    #[test]
    fn a_curve_passes_through_its_nodes() {
        let settings = Settings {
            saturation: [1.2, 0.8, 1.0, 1.5, 0.3, 1.0, 0.9, 1.1],
            ..Settings::default()
        };
        let curves = Curves::new(&settings);
        for k in 0..NODES {
            // The node's hue in the table's coordinate, -π to π, then the nearest
            // entries, interpolated.
            let mut hue = hue_node(k, 0.0);
            if hue >= PI {
                hue -= 2.0 * PI;
            }
            let x = LUT_SIZE as f32 * (hue + PI) / (2.0 * PI);
            let i = x.floor() as usize % LUT_SIZE;
            let j = (i + 1) % LUT_SIZE;
            let value = curves.saturation[i]
                + (x - x.floor()) * (curves.saturation[j] - curves.saturation[i]);
            assert!(
                (value - settings.saturation[k]).abs() < 2e-2,
                "{k}: {value}"
            );
        }
    }

    #[test]
    fn the_curves_are_linear_in_their_nodes() {
        let settings = Settings {
            hue: [10.0, -5.0, 0.0, 20.0, 0.0, -30.0, 0.0, 5.0],
            saturation: [1.3, 1.0, 0.7, 1.0, 1.2, 1.0, 0.9, 1.0],
            ..Settings::default()
        };
        let base = Curves::new(&settings);
        let [d_hue, d_saturation, _] = red_node_derivatives(&settings, &base);
        let step = 0.01;
        let mut moved = settings;
        moved.hue[0] += step;
        moved.saturation[0] += step;
        let moved = Curves::new(&moved);
        for i in 0..LUT_SIZE {
            let hue = (moved.hue[i] - base.hue[i]) / step;
            assert!((hue - d_hue[i]).abs() < 1e-4, "hue {i}: {hue} {}", d_hue[i]);
            let saturation = (moved.saturation[i] - base.saturation[i]) / step;
            assert!(
                (saturation - d_saturation[i]).abs() < 2e-3,
                "saturation {i}: {saturation} {}",
                d_saturation[i]
            );
        }
    }

    #[test]
    fn the_solve_solves() {
        let mut a = [[0.0f32; NODES]; NODES];
        for (i, row) in a.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = basis(hue_node(i, 3.0) - hue_node(j, 3.0), PI, 6);
            }
        }
        let y = [1.0, 2.0, -1.0, 0.5, 0.0, 3.0, 1.0, -2.0];
        let x = pseudo_solve(&a, &y).unwrap();
        for i in 0..NODES {
            let ax: f32 = (0..NODES).map(|j| a[i][j] * x[j]).sum();
            assert!((ax - y[i]).abs() < 1e-3, "{i}: {ax}");
        }
        let singular = [[1.0f32; NODES]; NODES];
        assert_eq!(pseudo_solve(&singular, &y), None);
    }

    #[test]
    fn the_weights_are_darktables_logistic() {
        let table = satweights(0.0);
        assert_eq!(table.len(), 2 * SATSIZE + 1);
        assert_eq!(table[SATSIZE], 0.5);
        // Saturation 0.1 past the shift: 1 / (1 + e^-3).
        assert!((satweight(&table, 0.1) - 1.0 / (1.0 + (-3.0f32).exp())).abs() < 1e-5);
        assert!(satweight(&table, -1.0) < 1e-6 && satweight(&table, 1.0) > 1.0 - 1e-6);
        // Steeper with contrast.
        assert!(satweight(&satweights(1.0), 0.05) > satweight(&table, 0.05));
    }

    #[test]
    fn the_process_values_are_darktables() {
        let process = Process::new(&Settings::default(), 1.0);
        // Inflection at 6%: -0.015 + 0.3 · 0.25.
        assert!((process.sat_shift - 0.06).abs() < 1e-7);
        assert!((process.bright_shift - 0.07).abs() < 1e-7);
        assert_eq!(process.gradient_amp, 4.0);
        assert_eq!(process.hue_sigma, 0.75);
        assert_eq!(process.par_sigma, 0.5);
        assert_eq!((process.sat_sigma, process.scharr_sigma), (1.0, 1.0));
        assert_eq!(process.white, ucs::y_to_l_star(2.0));
        let half = Process::new(&Settings::default(), 0.5);
        assert_eq!(
            (half.sat_sigma, half.gradient_amp, half.hue_sigma),
            (0.5, 1.0, 0.375)
        );
    }

    #[test]
    fn the_grids_downsample_as_darktable() {
        // Test 0159's effect radius of 25: σ 12.5, a fourth of the image.
        let grid = Grid::new(12.5, [3908, 2602]);
        assert_eq!(
            (grid.size, grid.sigma, grid.resized),
            ([977, 650], 3.125, true)
        );
        // Its hue analysis radius of 2: σ 1, the image itself, σ 1.
        let grid = Grid::new(1.0, [3908, 2602]);
        assert_eq!(
            (grid.size, grid.sigma, grid.resized),
            ([3908, 2602], 1.0, false)
        );
        // σ 4.4: floor(2.9) = 2.
        assert_eq!(Grid::new(4.4, [100, 50]).size, [50, 25]);
        assert_eq!(Grid::new(0.1, [100, 50]).sigma, 0.2);
    }

    #[test]
    fn a_smaller_view_blurs_in_place_only_where_the_export_does() {
        // Test 0159's effect radius of 25, σ 12.5: at scale 1, darktable's grid, a fourth
        // of the image, not in place.
        let export = Grid::scaled(12.5, [3908, 2602], 12.5);
        assert_eq!(export, Grid::new(12.5, [3908, 2602]));
        assert_eq!(
            (export.size, export.resized, export.in_place),
            ([977, 650], true, false)
        );
        // At a fourth, σ 3.125: darktable's grid is the image, where darktable's preview
        // blurs in place; the export's does not, so neither does the view.
        let view = Grid::scaled(3.125, [977, 650], 12.5);
        assert_eq!((view.size, view.sigma), ([977, 650], 3.125));
        assert!(!view.resized && !view.in_place);
        assert!(Grid::new(3.125, [977, 650]).in_place);
        // A σ under 3.5 at full resolution, where darktable's export blurs in place: in
        // place at every scale. At 3.5 itself the export downsamples by 2.
        for full in [0.5, 1.0, 3.49] {
            for scale in [1.0, 0.5, 0.25, 0.125] {
                let grid = Grid::scaled(full * scale, [400, 300], full);
                assert!(grid.in_place && !grid.resized, "σ {full} at {scale}");
                assert_eq!(grid, Grid::new(full * scale, [400, 300]));
            }
        }
        assert!(!Grid::scaled(3.5, [400, 300], 3.5).in_place);
        assert!(!Grid::scaled(1.75, [200, 150], 3.5).in_place);
        // The setup gives both filters theirs: test 0159's radii at a fourth.
        let settings = Settings {
            chroma_size: 2.0,
            param_size: 25.0,
            ..Settings::default()
        };
        let setup = Setup::new(&settings, [977, 650], 0.25, &profile());
        assert!(setup.prefilter.in_place && !setup.prefilter.resized);
        assert!(!setup.guide.in_place && !setup.guide.resized);
    }

    fn prepared(settings: &Settings, width: u32, height: u32) -> Prepared {
        let input = ImageDesc::rgba(width, height);
        prepare(settings, &input, [width, height], 1.0, &profile()).unwrap()
    }

    #[test]
    fn without_the_filter_one_pass_does_everything() {
        let settings = Settings {
            use_filter: false,
            ..Settings::default()
        };
        let prepared = prepared(&settings, 64, 48);
        assert_eq!(prepared.passes.len(), 1);
        let main = &prepared.passes[0];
        assert_eq!((main.entry_point, main.kind), ("main", PassKind::PerPixel));
        assert_eq!(main.params.len(), PARAMS_SIZE);
        assert_eq!(main.tables[0].len(), 4 * 7 * LUT_SIZE);
        assert_eq!(prepared.scratch.len(), 3);
    }

    #[test]
    fn with_the_filter_every_pass_binds_what_it_may() {
        for param_size in [1.0, 25.0] {
            let settings = Settings {
                param_size,
                ..Settings::default()
            };
            let filtered = prepared(&settings, 640, 480);
            let main = filtered.passes.last().unwrap();
            assert_eq!(main.entry_point, "main");
            for pass in &filtered.passes {
                assert_eq!(pass.params.len(), PARAMS_SIZE, "{}", pass.entry_point);
                let storage = pass
                    .bindings
                    .iter()
                    .filter(|b| b.resource != Resource::Uniforms)
                    .count();
                assert!(storage <= 8, "{}: {storage}", pass.entry_point);
                let has = |resource| pass.bindings.iter().any(|b| b.resource == resource);
                // Only `main` writes the output and is per pixel; only it and `sample`
                // read the input.
                assert_eq!(has(Resource::Output), pass.entry_point == "main");
                assert_eq!(pass.kind == PassKind::PerPixel, pass.entry_point == "main");
                assert_eq!(
                    has(Resource::Input),
                    matches!(pass.entry_point, "main" | "sample")
                );
                for binding in &pass.bindings {
                    if let Resource::Scratch(index) = binding.resource {
                        assert!(index < filtered.scratch.len(), "{}", pass.entry_point);
                    }
                    // No pass binds one buffer twice.
                    assert_eq!(
                        pass.bindings
                            .iter()
                            .filter(|b| b.resource == binding.resource)
                            .count(),
                        1,
                        "{}",
                        pass.entry_point
                    );
                }
            }
            // `main` binds the same in both modes, for the derivative entry points.
            let unfiltered = prepared(
                &Settings {
                    use_filter: false,
                    ..settings
                },
                640,
                480,
            );
            let bound = |pass: &Pass| pass.bindings.iter().map(|b| b.binding).collect::<Vec<_>>();
            assert_eq!(bound(main), bound(&unfiltered.passes[0]));
            assert_eq!(
                filtered.scratch.len(),
                if param_size > 7.0 { 14 } else { 12 }
            );
        }
    }

    #[test]
    fn only_the_guide_reads_the_effect_radius() {
        // The prefilter's passes hold neither the effect radius nor the curves.
        let base = Settings::default();
        let other = Settings {
            param_size: 3.0,
            hue: [10.0; NODES],
            ..base
        };
        let a = prepared(&base, 320, 200);
        let b = prepared(&other, 320, 200);
        let prefilter = |p: &Prepared| {
            p.passes
                .iter()
                .take_while(|pass| pass.entry_point != "corrections_pass")
                .map(|pass| (pass.params.clone(), pass.tables.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(prefilter(&a), prefilter(&b));
    }
}
