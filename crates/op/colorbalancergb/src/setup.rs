// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/colorbalancergb.c
// and src/common/darktable_ucs_22_helpers.h)
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

//! Color balance rgb: setup for the `colorbalancergb` kernels, ported from darktable's
//! `commit_params()`. See `docs/ports/colorbalancergb.md`.

#![allow(
    clippy::excessive_precision,
    reason = "constants are copied digit for digit from darktable"
)]

use std::borrow::Cow;
use std::f32::consts::PI;
use std::sync::OnceLock;

use apertrust_plugin::NotPorted;

use apertrust_color::{
    self as color, Mat3, WorkingProfile, XYZ_D50_TO_D65_CAT16, XYZ_D65_TO_D50_CAT16,
    XYZ_D65_TO_LMS_2006, ucs,
};

/// Hue offset between the conventional colour wheel and Filmlight Yrg, in degrees.
const ANGLE_SHIFT: f32 = -30.0;

/// The smallest value `prepare` divides by where darktable's ranges reach a division by
/// zero: a grey fulcrum or mask grey fulcrum of 0, a midtones luminance of -1, and the
/// midtones wheel, whose `1 + v` crosses 0 from a chroma of about 0.31 at some hues.
/// darktable divides by zero there; the kernel gets finite uniforms instead, exponents
/// of at most 10^6, which its `bounded_pow` keeps finite.
const SMALLEST_DIVISOR: f32 = 1e-6;

/// `d`, or ±[`SMALLEST_DIVISOR`] with its sign where it is closer to 0.
fn away_from_zero(d: f32) -> f32 {
    if d.abs() < SMALLEST_DIVISOR {
        SMALLEST_DIVISOR.copysign(d)
    } else {
        d
    }
}

/// Bytes of the kernel's `ColorBalanceParams`: thirteen `vec4<f32>`, then fifteen floats
/// and the saturation formula.
pub const PARAMS_SIZE: usize = 13 * 16 + 16 * 4;

/// darktable `dt_iop_colorbalancrgb_saturation_t`: the space brilliance, saturation and
/// the gamut mapping work in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaturationFormula {
    /// JzAzBz (2021), the formula of darktable's params before version 5.
    JzAzBz,
    /// darktable UCS (2022), darktable's default since params version 5.
    Ucs,
}

impl SaturationFormula {
    /// From darktable's value: 0 JzAzBz, 1 darktable UCS.
    pub fn from_darktable(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::JzAzBz),
            1 => Some(Self::Ucs),
            _ => None,
        }
    }

    /// darktable's value, which the kernel's `saturation_formula` takes.
    pub fn to_darktable(self) -> u32 {
        match self {
            Self::JzAzBz => 0,
            Self::Ucs => 1,
        }
    }
}

/// The module's settings: darktable params version 5, field for field. Hues and the
/// hue shift are in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub shadows_y: f32,
    pub shadows_c: f32,
    pub shadows_h: f32,
    pub midtones_y: f32,
    pub midtones_c: f32,
    pub midtones_h: f32,
    pub highlights_y: f32,
    pub highlights_c: f32,
    pub highlights_h: f32,
    pub global_y: f32,
    pub global_c: f32,
    pub global_h: f32,
    pub shadows_weight: f32,
    pub white_fulcrum: f32,
    pub highlights_weight: f32,
    pub chroma_shadows: f32,
    pub chroma_highlights: f32,
    pub chroma_global: f32,
    pub chroma_midtones: f32,
    pub saturation_global: f32,
    pub saturation_highlights: f32,
    pub saturation_midtones: f32,
    pub saturation_shadows: f32,
    pub hue_angle: f32,
    pub brilliance_global: f32,
    pub brilliance_highlights: f32,
    pub brilliance_midtones: f32,
    pub brilliance_shadows: f32,
    pub mask_grey_fulcrum: f32,
    pub vibrance: f32,
    pub grey_fulcrum: f32,
    pub contrast: f32,
    pub saturation_formula: SaturationFormula,
}

impl Default for Settings {
    /// darktable's defaults, its params' introspection `$DEFAULT`s: every field 0 but the
    /// two fall-offs, 1, and the two fulcrums, 0.1845, with the darktable UCS saturation
    /// formula. The module has no `reload_defaults`, so these are its scene-referred
    /// defaults too. (`legacy_params`' `default_v5` differs: its grey fulcrum is 0, which
    /// each upgrade overwrites.)
    fn default() -> Self {
        Self {
            shadows_y: 0.0,
            shadows_c: 0.0,
            shadows_h: 0.0,
            midtones_y: 0.0,
            midtones_c: 0.0,
            midtones_h: 0.0,
            highlights_y: 0.0,
            highlights_c: 0.0,
            highlights_h: 0.0,
            global_y: 0.0,
            global_c: 0.0,
            global_h: 0.0,
            shadows_weight: 1.0,
            white_fulcrum: 0.0,
            highlights_weight: 1.0,
            chroma_shadows: 0.0,
            chroma_highlights: 0.0,
            chroma_global: 0.0,
            chroma_midtones: 0.0,
            saturation_global: 0.0,
            saturation_highlights: 0.0,
            saturation_midtones: 0.0,
            saturation_shadows: 0.0,
            hue_angle: 0.0,
            brilliance_global: 0.0,
            brilliance_highlights: 0.0,
            brilliance_midtones: 0.0,
            brilliance_shadows: 0.0,
            mask_grey_fulcrum: 0.1845,
            vibrance: 0.0,
            grey_fulcrum: 0.1845,
            contrast: 0.0,
            saturation_formula: SaturationFormula::Ucs,
        }
    }
}

/// What the kernel needs besides the image: its uniform block after the pass header, and
/// the gamut LUT for binding 3.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub params: Vec<u8>,
    pub gamut_lut: Cow<'static, [f32]>,
}

impl Prepared {
    /// The gamut LUT as binding 3's little-endian bytes.
    pub fn table(&self) -> Vec<u8> {
        self.gamut_lut
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }
}

/// The working gamut's boundary per hue in this formula's space, the kernel's gamut LUT:
/// the largest JzAzBz saturation, or with darktable UCS the boundary's colourfulness
/// squared. darktable builds it again only when the working profile or the saturation
/// formula changes (`commit_params`' `lut_inited`). Here linear Rec2020's, Apertrust's
/// working space, is built once per process for each formula, so `prepare` stays cheap on
/// every slider drag; any other profile, which only tests use, builds its own on each
/// call.
pub fn gamut_lut(profile: &WorkingProfile, formula: SaturationFormula) -> Cow<'static, [f32]> {
    let build = || {
        let rgb_to_xyz_d65 = color::mul(&XYZ_D50_TO_D65_CAT16, &profile.rgb_to_xyz_d50);
        match formula {
            SaturationFormula::JzAzBz => color::jzazbz_gamut_lut(&rgb_to_xyz_d65),
            SaturationFormula::Ucs => ucs_gamut_lut(&rgb_to_xyz_d65),
        }
    };
    if *profile == WorkingProfile::linear_rec2020() {
        static JZAZBZ: OnceLock<Vec<f32>> = OnceLock::new();
        static UCS_GAMUT: OnceLock<Vec<f32>> = OnceLock::new();
        let table = match formula {
            SaturationFormula::JzAzBz => &JZAZBZ,
            SaturationFormula::Ucs => &UCS_GAMUT,
        };
        Cow::Borrowed(table.get_or_init(build))
    } else {
        Cow::Owned(build())
    }
}

/// darktable `dt_UCS_22_build_gamut_LUT`: the colourfulness squared, M², of the working
/// gamut's boundary in darktable UCS, per hue. It marches the triangle of the primaries
/// in CIE xy around D65 at 50 rays per entry, and averages the samples whose UCS hue
/// rounds to each entry; an entry no sample reaches is 0. `rgb_to_xyz_d65` maps working
/// RGB to XYZ D65. In f32, as darktable, but for the order of each entry's sum, which
/// darktable's threads share.
pub fn ucs_gamut_lut(rgb_to_xyz_d65: &Mat3) -> Vec<f32> {
    const SIZE: usize = color::GAMUT_LUT_SIZE;
    const SAMPLES: usize = 50 * SIZE;
    const TWO_PI: f32 = 2.0 * PI;
    let white = ucs::D65_XY;
    let primary = |rgb: [f32; 3]| {
        let [x, y, _] = ucs::d65_xyz_to_xyy(color::apply(rgb_to_xyz_d65, rgb));
        [x, y]
    };
    let [red, green, blue] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(primary);
    // The primaries' angles in xy around D65.
    let angle_of = |p: [f32; 2]| (p[1] - white[1]).atan2(p[0] - white[0]);
    let (h_red, h_green, h_blue) = (angle_of(red), angle_of(green), angle_of(blue));
    // darktable `Delta_H`: the difference of two angles, within [-π, π].
    let delta_h = |h_1: f32, h_2: f32| {
        let mut diff = h_1 - h_2;
        if diff < -PI {
            diff += TWO_PI;
        }
        if diff > PI {
            diff -= TWO_PI;
        }
        diff
    };
    let within = |t: f32| (0.0..=1.0).contains(&t);

    let mut sums = vec![0.0f32; SIZE];
    let mut counts = vec![0.0f32; SIZE];
    for i in 0..SAMPLES {
        let angle = -PI + i as f32 / SAMPLES as f32 * TWO_PI;
        let tan_angle = angle.tan();
        // Where the ray from D65 at this angle crosses the edge from `from` to `to`.
        let crossing = |from: [f32; 2], to: [f32; 2]| {
            let t = (white[1] - from[1] + tan_angle * (from[0] - white[0]))
                / (to[1] - from[1] + tan_angle * (from[0] - to[0]));
            [
                from[0] + t * (to[0] - from[0]),
                from[1] + t * (to[1] - from[1]),
            ]
        };
        let xy = if within(delta_h(angle, h_blue) / delta_h(h_red, h_blue)) {
            crossing(blue, red)
        } else if within(delta_h(angle, h_red) / delta_h(h_green, h_red)) {
            crossing(red, green)
        } else if within(delta_h(angle, h_green) / delta_h(h_blue, h_green)) {
            crossing(green, blue)
        } else {
            [0.0, 0.0]
        };
        let [u, v] = ucs::xy_to_uv_star_prime(xy);
        let hue = v.atan2(u);
        let index = ((SIZE - 1) as f32 * (hue + PI) / TWO_PI).round() as i32;
        let index = index.rem_euclid(SIZE as i32) as usize;
        sums[index] += u * u + v * v;
        counts[index] += 1.0;
    }
    sums.iter()
        .zip(&counts)
        .map(|(sum, count)| sum / count.max(1.0))
        .collect()
}

/// darktable `commit_params`, with the matrices and `L_white` that `process` computes.
/// Both saturation formulas are ported, so it always succeeds; the `Result` is the
/// operation interface's.
pub fn prepare(settings: &Settings, profile: &WorkingProfile) -> Result<Prepared, NotPorted> {
    let s = settings;

    let rgb_to_xyz_d65 = color::mul(&XYZ_D50_TO_D65_CAT16, &profile.rgb_to_xyz_d50);
    let input_matrix = color::mul(&XYZ_D65_TO_LMS_2006, &rgb_to_xyz_d65);
    let output_matrix = color::mul(&profile.xyz_d50_to_rgb, &XYZ_D65_TO_D50_CAT16);

    // Grading RGB of white, and of each colour wheel at Y = 1.
    let norm = color::ych_to_grading_rgb([1.0, 0.0, 1.0, 0.0]);
    let wheel = |chroma: f32, hue: f32| {
        let rgb = color::ych_to_grading_rgb(color::make_ych(
            1.0,
            chroma,
            color::deg_to_rad(hue + ANGLE_SHIFT),
        ));
        [0, 1, 2].map(|c| rgb[c] - norm[c])
    };
    let global = wheel(s.global_c, s.global_h);
    let global = [0, 1, 2].map(|c| global[c] + norm[c] * s.global_y);
    let shadows = wheel(s.shadows_c, s.shadows_h).map(|v| 1.0 + v + s.shadows_y);
    let highlights = wheel(s.highlights_c, s.highlights_h).map(|v| 1.0 + v + s.highlights_y);
    let midtones = wheel(s.midtones_c, s.midtones_h).map(|v| 1.0 / away_from_zero(1.0 + v));

    let shadows_weight = 2.0 + s.shadows_weight * 2.0;
    let highlights_weight = 2.0 + s.highlights_weight * 2.0;
    let (sw2, hw2) = (
        shadows_weight * shadows_weight,
        highlights_weight * highlights_weight,
    );
    let hue = color::deg_to_rad(s.hue_angle);

    let mut uniform = Uniform::default();
    for row in input_matrix.iter().chain(&output_matrix) {
        uniform.vec3(*row);
    }
    for v in [global, shadows, highlights, midtones] {
        uniform.vec3(v);
    }
    uniform.vec3([s.chroma_shadows, s.chroma_midtones, s.chroma_highlights]);
    uniform.vec3([
        s.saturation_shadows,
        s.saturation_midtones,
        s.saturation_highlights,
    ]);
    uniform.vec3([
        s.brilliance_shadows,
        s.brilliance_midtones,
        s.brilliance_highlights,
    ]);
    for v in [
        s.chroma_global,
        s.vibrance,
        1.0 + s.contrast,
        s.saturation_global,
        s.brilliance_global,
        hue.cos(),
        hue.sin(),
        shadows_weight,
        highlights_weight,
        sw2 * hw2 / (sw2 + hw2),
        s.mask_grey_fulcrum
            .max(SMALLEST_DIVISOR)
            .powf(0.4101205819200422),
        s.white_fulcrum.exp2(),
        s.grey_fulcrum.max(SMALLEST_DIVISOR),
        1.0 / (1.0 + s.midtones_y).max(SMALLEST_DIVISOR),
        // darktable UCS's L_white, the lightness of the white fulcrum; JzAzBz ignores it.
        ucs::y_to_l_star(s.white_fulcrum.exp2()),
    ] {
        uniform.f32(v);
    }
    uniform.u32(s.saturation_formula.to_darktable());

    debug_assert_eq!(uniform.0.len(), PARAMS_SIZE);
    Ok(Prepared {
        params: uniform.0,
        gamut_lut: gamut_lut(profile, s.saturation_formula),
    })
}

/// Little-endian `ColorBalanceParams`, written field by field in the kernels' order. It
/// sits at offset 48 of the uniform, after the pass header the engine writes.
#[derive(Default)]
struct Uniform(Vec<u8>);

impl Uniform {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    /// A `vec4<f32>` with the last component zero.
    fn vec3(&mut self, v: [f32; 3]) {
        debug_assert_eq!(self.0.len() % 16, 0, "vec4 must sit at a 16-byte offset");
        for c in [v[0], v[1], v[2], 0.0] {
            self.f32(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jzazbz_defaults() -> Settings {
        Settings {
            saturation_formula: SaturationFormula::JzAzBz,
            ..Settings::default()
        }
    }

    /// The uniform's words as floats, by index.
    fn floats(prepared: &Prepared) -> Vec<f32> {
        let (words, _) = prepared.params.as_chunks::<4>();
        words.iter().map(|word| f32::from_le_bytes(*word)).collect()
    }

    #[test]
    fn the_uniform_has_the_kernels_size() {
        let prepared = prepare(&jzazbz_defaults(), &WorkingProfile::linear_rec2020()).unwrap();
        assert_eq!(prepared.params.len(), PARAMS_SIZE);
        assert_eq!(PARAMS_SIZE % 16, 0);
        assert_eq!(prepared.gamut_lut.len(), color::GAMUT_LUT_SIZE);
        assert_eq!(prepared.table().len(), 4 * color::GAMUT_LUT_SIZE);
    }

    #[test]
    fn contrast_reaches_the_kernel_as_one_plus_contrast() {
        // `ColorBalanceParams`: thirteen vec4s, then chroma_global, vibrance, contrast.
        let at = 13 * 4 + 2;
        let profile = WorkingProfile::linear_rec2020();
        let neutral = prepare(&jzazbz_defaults(), &profile).unwrap();
        assert_eq!(floats(&neutral)[at], 1.0);
        let settings = Settings {
            contrast: 0.25,
            ..jzazbz_defaults()
        };
        assert_eq!(floats(&prepare(&settings, &profile).unwrap())[at], 1.25);
    }

    #[test]
    fn the_working_spaces_gamut_lut_is_built_once_for_each_formula() {
        // Built on the first call, then borrowed: the same table every time.
        let profile = WorkingProfile::linear_rec2020();
        for (settings, formula) in [
            (jzazbz_defaults(), SaturationFormula::JzAzBz),
            (Settings::default(), SaturationFormula::Ucs),
        ] {
            let first = gamut_lut(&profile, formula);
            let second = prepare(&settings, &profile).unwrap().gamut_lut;
            assert!(matches!(first, Cow::Borrowed(_)));
            assert!(matches!(second, Cow::Borrowed(_)));
            assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
        }
        let jzazbz = gamut_lut(&profile, SaturationFormula::JzAzBz);
        let ucs = gamut_lut(&profile, SaturationFormula::Ucs);
        assert_ne!(jzazbz, ucs);
    }

    #[test]
    fn the_formula_and_the_white_lightness_reach_the_kernel() {
        // The last two words: L_white, then the saturation formula.
        let profile = WorkingProfile::linear_rec2020();
        let words = |settings: &Settings| {
            let prepared = prepare(settings, &profile).unwrap();
            let (words, _) = prepared.params.as_chunks::<4>();
            let n = words.len();
            (
                f32::from_le_bytes(words[n - 2]),
                u32::from_le_bytes(words[n - 1]),
            )
        };
        let (l_white, formula) = words(&Settings::default());
        assert_eq!(formula, 1);
        assert_eq!(l_white, ucs::y_to_l_star(1.0));
        assert_eq!(words(&jzazbz_defaults()).1, 0);
        // darktable's L_white is the white fulcrum's lightness, 2^white_fulcrum.
        let brighter = Settings {
            white_fulcrum: 2.0,
            ..Settings::default()
        };
        assert_eq!(words(&brighter).0, ucs::y_to_l_star(4.0));
    }

    #[test]
    fn the_ucs_gamut_lut_reaches_every_hue_of_rec2020() {
        // Every entry gets samples of the primaries' triangle, so none is left at 0, and
        // the boundary's colourfulness is largest towards the primaries.
        let lut = gamut_lut(&WorkingProfile::linear_rec2020(), SaturationFormula::Ucs);
        assert_eq!(lut.len(), color::GAMUT_LUT_SIZE);
        assert!(lut.iter().all(|m2| m2.is_finite() && *m2 > 0.0), "{lut:?}");
        let (least, most) = lut
            .iter()
            .fold((f32::MAX, 0.0f32), |(lo, hi), &m2| (lo.min(m2), hi.max(m2)));
        assert!(most > 2.0 * least, "{least} to {most}");
    }

    #[test]
    fn divisions_by_zero_in_darktables_ranges_give_finite_uniforms() {
        let settings = Settings {
            grey_fulcrum: 0.0,
            mask_grey_fulcrum: 0.0,
            midtones_y: -1.0,
            ..jzazbz_defaults()
        };
        let prepared = prepare(&settings, &WorkingProfile::linear_rec2020()).unwrap();
        // The last word is the saturation formula, a whole number.
        let floats = floats(&prepared);
        let (_, uniform) = floats.split_last().unwrap();
        assert!(uniform.iter().all(|v| v.is_finite()), "{uniform:?}");
    }

    #[test]
    fn the_midtones_wheel_gives_bounded_exponents_at_every_hue() {
        // From a chroma of about 0.31, `1 + v` crosses 0 at some hues: chroma 0.4 at
        // 180.02° gave an exponent of -2.8e6, 0.6 at 27.34° one of -19831.
        let profile = WorkingProfile::linear_rec2020();
        // `ColorBalanceParams`' tenth vec4 is the midtones power.
        let midtones = 9 * 4..9 * 4 + 3;
        let mut crossed = false;
        for chroma in [0.31, 0.4, 0.6, 0.8, 1.0] {
            for step in 0..3600 {
                let hue = step as f32 / 10.0;
                let settings = Settings {
                    midtones_c: chroma,
                    midtones_h: hue,
                    ..jzazbz_defaults()
                };
                let floats = floats(&prepare(&settings, &profile).unwrap());
                for &exponent in &floats[midtones.clone()] {
                    assert!(
                        exponent.is_finite() && exponent.abs() <= 1.0 / SMALLEST_DIVISOR,
                        "chroma {chroma} at {hue}°: exponent {exponent}"
                    );
                    crossed |= exponent < 0.0;
                }
            }
        }
        assert!(crossed, "no hue crossed the pole");
        for (chroma, hue) in [(0.4, 180.02), (0.6, 27.34)] {
            let settings = Settings {
                midtones_c: chroma,
                midtones_h: hue,
                ..jzazbz_defaults()
            };
            let floats = floats(&prepare(&settings, &profile).unwrap());
            assert!(
                floats[midtones.clone()].iter().any(|v| *v < -1.0),
                "chroma {chroma} at {hue}°: {:?}",
                &floats[midtones.clone()]
            );
        }
    }

    #[test]
    fn a_working_space_other_than_rec2020_builds_its_own_gamut_lut() {
        let srgb_to_xyz = color::to_f32(color::RgbSpace::SRGB.rgb_to_xyz_d50());
        let srgb = WorkingProfile {
            rgb_to_xyz_d50: srgb_to_xyz,
            xyz_d50_to_rgb: color::inverse(&srgb_to_xyz),
        };
        for formula in [SaturationFormula::JzAzBz, SaturationFormula::Ucs] {
            let rec2020 = gamut_lut(&WorkingProfile::linear_rec2020(), formula);
            let own = gamut_lut(&srgb, formula);
            assert!(matches!(own, Cow::Owned(_)));
            assert_eq!(own.len(), rec2020.len());
            assert_ne!(own, rec2020);
            // A smaller gamut: a lower boundary at most hues.
            let lower = own
                .iter()
                .zip(rec2020.iter())
                .filter(|(s, r)| s < r)
                .count();
            assert!(
                lower > own.len() / 2,
                "{formula:?}: {lower} of {}",
                own.len()
            );
        }
    }
}
