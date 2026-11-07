// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2020-2026  darktable developers (ported from src/iop/sigmoid.c)
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

//! Sigmoid: setup for the `sigmoid` kernel, ported from darktable's `commit_params()` and
//! `_calculate_adjusted_primaries()`. See `docs/ports/sigmoid.md`.
//!
//! The curve's fit follows the f32 arithmetic of darktable's macOS release build,
//! compiled by clang, as its `libsigmoid.so` computes it, because its finite differences
//! magnify the last bit of every value they take (Precision). Where darktable's f32 curve
//! then breaks down, its film response or paper exposure leaving f32's range, the setup
//! finds where in f32, for the kernel to render the breakdown as darktable does. The
//! matrices are computed in f64 and rounded once.

use apertrust_color::{self as color, Mat3, Mat3d, RgbSpace};

/// darktable `MIDDLE_GREY`: the grey the curve keeps, in the scene and on the display.
pub const MIDDLE_GREY: f32 = 0.1845;

/// The colour processing (darktable `dt_iop_sigmoid_methods_type_t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProcessing {
    /// The curve on each channel, over inset and rotated primaries, with the middle
    /// channel's hue restored as far as the hue preservation says: darktable's default.
    PerChannel,
    /// The curve on the channels' mean, which scales the pixel, then its chroma compressed
    /// into the display's range.
    RgbRatio,
}

impl ColorProcessing {
    pub fn from_darktable(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::PerChannel),
            1 => Some(Self::RgbRatio),
            _ => None,
        }
    }

    /// darktable's value, the inverse of [`Self::from_darktable`].
    pub fn to_darktable(self) -> i32 {
        match self {
            Self::PerChannel => 0,
            Self::RgbRatio => 1,
        }
    }
}

/// The space whose primaries the per-channel processing insets and rotates (darktable
/// `dt_iop_sigmoid_base_primaries_t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasePrimaries {
    WorkingProfile,
    Rec2020,
    DisplayP3,
    AdobeRgb,
    Srgb,
}

impl BasePrimaries {
    pub const ALL: [Self; 5] = [
        Self::WorkingProfile,
        Self::Rec2020,
        Self::DisplayP3,
        Self::AdobeRgb,
        Self::Srgb,
    ];

    pub fn from_darktable(value: i32) -> Option<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }

    /// darktable's value, the inverse of [`Self::from_darktable`].
    pub fn to_darktable(self) -> i32 {
        Self::ALL.iter().position(|&b| b == self).unwrap() as i32
    }
}

/// The module's settings: darktable params version 3, field for field. The targets and
/// the hue preservation are percentages and the rotations radians, as darktable stores
/// them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub middle_grey_contrast: f32,
    pub contrast_skewness: f32,
    pub display_white_target: f32,
    pub display_black_target: f32,
    pub color_processing: ColorProcessing,
    pub hue_preservation: f32,
    /// Red, green, blue.
    pub inset: [f32; 3],
    pub rotation: [f32; 3],
    pub purity: f32,
    pub base_primaries: BasePrimaries,
}

impl Default for Settings {
    /// darktable's defaults, its introspection's: what a new instance gets (sigmoid has
    /// no `reload_defaults`) and what its auto-applied "scene-referred default" preset
    /// stores.
    fn default() -> Self {
        Self {
            middle_grey_contrast: 1.5,
            contrast_skewness: 0.0,
            display_white_target: 100.0,
            display_black_target: 0.0152,
            color_processing: ColorProcessing::PerChannel,
            hue_preservation: 100.0,
            inset: [0.0; 3],
            rotation: [0.0; 3],
            purity: 0.0,
            base_primaries: BasePrimaries::WorkingProfile,
        }
    }
}

/// `powf` as f64 `pow` rounded once to f32. darktable calls its C library's `powf`, which
/// can be an ulp away from this (Apple's for about 0.6% of arguments) and differs between
/// platforms; the fit's finite differences magnify that ulp, so the platform's own `powf`
/// would make the curve differ between platforms. `f64::powf` calls the platform's `pow`
/// too, but its last bit decides the f32 result only near an f32 halfway point, about one
/// argument in 10^8.
fn powf(x: f32, y: f32) -> f32 {
    f64::from(x).powf(f64::from(y)) as f32
}

/// darktable's NaN check in `_generalized_loglogistic_sigmoid`: a NaN, which `inf / inf`
/// gives, becomes the curve's magnitude.
fn or_magnitude(value: f32, magnitude: f32) -> f32 {
    if value.is_nan() { magnitude } else { value }
}

// The fit's constants as darktable's release build (-ffast-math) folds them.

/// The points of the fit's central differences, `MIDDLE_GREY ± delta`, delta 1e-6.
const GREY_ABOVE: f32 = MIDDLE_GREY + 1e-6;
const GREY_BELOW: f32 = MIDDLE_GREY - 1e-6;
/// `1 / (2 delta)`: the build multiplies by it where the source divides by 2 and delta.
const HALF_INVERSE_DELTA: f32 = 500_000.0;
/// The build's `0.01f * target / MIDDLE_GREY`: `target * (0.01f * (1 / MIDDLE_GREY))`.
const PERCENT_OVER_GREY: f32 = 0.01 * (1.0 / MIDDLE_GREY);

/// Where darktable's f32 paper exposure P lies, which decides how its curve breaks down
/// at the ranges' corners (`docs/ports/sigmoid.md`, Precision).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperExposure {
    /// Finite and positive: the curve, but the target white where the film response F
    /// overflows (`inf / inf`, darktable's NaN check) and black where it underflows to 0.
    Finite,
    /// Overflowed: `F / inf` is 0, black, but the target white where F overflows too.
    Infinite,
    /// Underflowed to 0: `F / F` is 1, or `0 / 0` darktable's NaN check, so the target
    /// white everywhere.
    Zero,
}

impl PaperExposure {
    pub fn of(paper_exposure: f32) -> Self {
        if paper_exposure.is_infinite() {
            Self::Infinite
        } else if paper_exposure == 0.0 {
            Self::Zero
        } else {
            Self::Finite
        }
    }

    /// The kernel's value (`sigmoid.slang`).
    pub fn to_kernel(self) -> u32 {
        match self {
            Self::Finite => 0,
            Self::Infinite => 1,
            Self::Zero => 2,
        }
    }
}

/// The generalised log-logistic curve, `white · (F / (P + F))^q` with `F = (fog + x)^p`
/// (darktable `_generalized_loglogistic_sigmoid`): its coefficients, a pure function of
/// the settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    /// The display's white and black, as fractions: the curve's limits.
    pub white_target: f32,
    pub black_target: f32,
    /// darktable's `paper_exposure` P, in f32 as its build computes it: infinite or 0
    /// where the fit leaves f32's range.
    pub paper_exposure: f32,
    /// The log2 of the paper exposure from the same f32 terms, in f64: finite for every
    /// setting. The kernel takes this.
    pub log2_paper_exposure: f32,
    pub film_fog: f32,
    /// p, which darktable fits to the contrast.
    pub film_power: f32,
    /// q, `5^-skew`.
    pub paper_power: f32,
    /// The smallest film value `fog + x` whose film response `powf` overflows f32, or
    /// f32's largest value where none does, which no clamped input reaches.
    pub film_overflow: f32,
    /// The largest film value whose film response `powf` rounds to 0, or 0 where only 0
    /// does.
    pub film_underflow: f32,
}

impl Curve {
    /// darktable `commit_params`, the curve's part, in its build's f32 arithmetic: the fit
    /// makes the curve send 0 to the target black, middle grey to itself and infinity to
    /// the target white, with the slope at middle grey the contrast's, whatever the skew.
    pub fn new(s: &Settings) -> Self {
        // The reference: no skew, a display from 0 to 1 and no fog, at film power c, whose
        // slope at grey is the one to reach.
        let contrast = s.middle_grey_contrast;
        let reference_exposure = powf(MIDDLE_GREY, contrast) * (1.0 / MIDDLE_GREY - 1.0);
        let reference = |grey: f32| {
            let film = powf(grey, contrast);
            or_magnitude(film / (reference_exposure + film), 1.0)
        };
        let reference_slope = (reference(GREY_ABOVE) - reference(GREY_BELOW)) * HALF_INVERSE_DELTA;

        // The skewed curve at film power 1; its slope sets the film power.
        let paper_power = powf(5.0, -s.contrast_skewness);
        let white_target = 0.01 * s.display_white_target;
        let black_target = 0.01 * s.display_black_target;
        // White over grey to the power 1 / q, the white-grey relation plus 1.
        let white_over_grey = powf(
            s.display_white_target * PERCENT_OVER_GREY,
            1.0 / paper_power,
        );
        let skewed = |grey: f32| {
            // The build folds the paper exposure `MIDDLE_GREY · relation` into the sum
            // with `grey`: `MIDDLE_GREY · white_over_grey + (grey − MIDDLE_GREY)`.
            let ratio = grey / (MIDDLE_GREY * white_over_grey + (grey - MIDDLE_GREY));
            or_magnitude(powf(ratio, paper_power) * white_target, white_target)
        };
        let skewed_slope = (skewed(GREY_ABOVE) - skewed(GREY_BELOW)) * HALF_INVERSE_DELTA;
        let film_power = reference_slope / skewed_slope;

        let white_grey_relation = white_over_grey - 1.0;
        let white_black_relation = powf(black_target / white_target, -1.0 / paper_power) - 1.0;
        let inverse_film_power = 1.0 / film_power;
        let grey_term = powf(white_grey_relation, inverse_film_power);
        let film_fog =
            grey_term * MIDDLE_GREY / (powf(white_black_relation, inverse_film_power) - grey_term);
        let paper_exposure = powf(film_fog + MIDDLE_GREY, film_power) * white_grey_relation;
        let log2_paper_exposure = f64::from(film_power) * f64::from(film_fog + MIDDLE_GREY).log2()
            + f64::from(white_grey_relation).log2();

        // Where darktable's film response leaves f32's range, found in f32 here, so the
        // kernel compares film values and never tests for infinity.
        let response = |film: f32| powf(film, film_power);
        let film_overflow = first_positive(|film| response(film).is_infinite()).unwrap_or(f32::MAX);
        let film_underflow = first_positive(|film| response(film) > 0.0)
            .map_or(0.0, |film| film.next_down().max(0.0));

        Self {
            white_target,
            black_target,
            paper_exposure,
            log2_paper_exposure: log2_paper_exposure as f32,
            film_fog,
            film_power,
            paper_power,
            film_overflow,
            film_underflow,
        }
    }

    pub fn paper(&self) -> PaperExposure {
        PaperExposure::of(self.paper_exposure)
    }

    /// The curve as the kernel computes it: darktable's breakdown where its film response
    /// or paper exposure leaves f32's range ([`PaperExposure`]), and otherwise the log
    /// domain, where `F / (P + F)` is `1 / (1 + 2^u)` with `u = log2 P − p · log2(fog + x)`.
    pub fn eval(&self, x: f32) -> f32 {
        let film = self.film_fog + x.max(0.0);
        if film >= self.film_overflow || self.paper() == PaperExposure::Zero {
            return self.white_target;
        }
        if film <= self.film_underflow || self.paper() == PaperExposure::Infinite {
            return 0.0;
        }
        let u = self.log2_paper_exposure - self.film_power * film.log2();
        // log2(1 + 2^u), without overflowing 2^u.
        let softplus = if u > 0.0 {
            u + (1.0 + (-u).exp2()).log2()
        } else {
            (1.0 + u.exp2()).log2()
        };
        self.white_target * (-self.paper_power * softplus).exp2()
    }

    /// darktable `_generalized_loglogistic_sigmoid` in f32, as its CPU path computes it,
    /// for comparisons.
    pub fn darktable(&self, x: f32) -> f32 {
        let film = powf(self.film_fog + x.max(0.0), self.film_power);
        let paper = self.white_target * powf(film / (self.paper_exposure + film), self.paper_power);
        or_magnitude(paper, self.white_target)
    }
}

/// The smallest positive f32 for which `holds`, a test that holds from some value up, is
/// true, or `None`: a binary search over the bit patterns, which rise with the values.
fn first_positive(holds: impl Fn(f32) -> bool) -> Option<f32> {
    let (mut low, mut high) = (1u32, f32::MAX.to_bits());
    if !holds(f32::from_bits(high)) {
        return None;
    }
    while low < high {
        let middle = low + (high - low) / 2;
        if holds(f32::from_bits(middle)) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Some(f32::from_bits(low))
}

/// The matrices of darktable's `_calculate_adjusted_primaries`, in f64, as `M · v`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrices {
    pub pipe_to_base: Mat3d,
    pub base_to_rendering: Mat3d,
    /// 0 where the outbound matrix is singular, which darktable refuses to invert.
    pub rendering_to_pipe: Mat3d,
}

impl Matrices {
    /// `working` is the pipeline's working space. Only the per-channel processing uses
    /// them.
    pub fn new(s: &Settings, working: &RgbSpace) -> Self {
        // darktable `_get_base_profile`: the working profile, or one of darktable's
        // built-in profiles, which for Rec2020 is the working profile's own entry.
        let base = match s.base_primaries {
            BasePrimaries::WorkingProfile => *working,
            BasePrimaries::Rec2020 => RgbSpace::LINEAR_REC2020,
            BasePrimaries::DisplayP3 => RgbSpace::DISPLAY_P3,
            BasePrimaries::AdobeRgb => RgbSpace::ADOBE_RGB,
            BasePrimaries::Srgb => RgbSpace::SRGB,
        };

        let base_out = color::inverse_d(&base.rgb_to_xyz_d50());
        // darktable skips the conversion when the base profile is the working profile.
        let (pipe_to_base, base_to_pipe) = if base == *working {
            (color::IDENTITY_D, color::IDENTITY_D)
        } else {
            let pipe_to_base = color::mul_d(&base_out, &working.rgb_to_xyz_d50());
            (pipe_to_base, color::inverse_d(&pipe_to_base))
        };

        // Base RGB read as coordinates over the base primaries, each rotated about the
        // white and moved towards it to `scaling` of its distance to the gamut's edge.
        let chromaticities = base.chromaticities();
        let to_rendering = |scaling: [f64; 3]| -> Mat3d {
            let primaries = std::array::from_fn(|i| {
                color::rotate_and_scale_primary(
                    &chromaticities,
                    scaling[i],
                    f64::from(s.rotation[i]),
                    i,
                )
            });
            let custom_to_xyz =
                color::rgb_to_xyz_from_chromaticities(primaries, chromaticities.white);
            color::mul_d(&base_out, &custom_to_xyz)
        };
        // Inbound: the inset primaries. Outbound: the inverse of the same construction with
        // the insets reduced by the purity, which restores that much purity; the rotation
        // stays.
        let base_to_rendering = to_rendering(s.inset.map(|v| 1.0 - f64::from(v)));
        // darktable inverts with `mat3SSEinv`, which refuses a singular matrix and leaves
        // `rendering_to_base` uninitialised: the white on the line through two outbound
        // primaries, turned opposite each other, or the three on one line. The setup
        // renders black there (docs/ports/sigmoid.md, Precision).
        let rendering_to_base = color::checked_inverse_d(&to_rendering(
            s.inset.map(|v| 1.0 - f64::from(s.purity) * f64::from(v)),
        ))
        .unwrap_or([[0.0; 3]; 3]);
        let rendering_to_pipe = color::mul_d(&base_to_pipe, &rendering_to_base);

        Self {
            pipe_to_base,
            base_to_rendering,
            rendering_to_pipe,
        }
    }
}

/// Everything the kernel needs, kept apart so the gradient check can move the curve's
/// coefficients in the kernel's own units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prepared {
    pub color_processing: ColorProcessing,
    pub matrices: Matrices,
    pub curve: Curve,
    /// The hue preservation as a fraction, clamped to [0, 1] as darktable does.
    pub hue_preservation: f32,
}

/// The size of `SigmoidParams` in the kernel, after the 48-byte pass header.
pub const PARAMS_SIZE: usize = 192;

/// darktable `commit_params`, with the matrices `process()` builds.
pub fn prepare(settings: &Settings, working: &RgbSpace) -> Prepared {
    Prepared {
        color_processing: settings.color_processing,
        matrices: Matrices::new(settings, working),
        curve: Curve::new(settings),
        hue_preservation: (0.01 * settings.hue_preservation).clamp(0.0, 1.0),
    }
}

impl Prepared {
    /// Little-endian `SigmoidParams`, field by field in the kernel's order. It sits at
    /// offset 48 of the uniform, after the pass header the engine writes.
    pub fn params(&self) -> Vec<u8> {
        let mut out = Uniform::default();
        let m = &self.matrices;
        for matrix in [m.pipe_to_base, m.base_to_rendering, m.rendering_to_pipe] {
            let matrix: Mat3 = color::to_f32(matrix);
            for row in matrix {
                out.vec4([row[0], row[1], row[2], 0.0]);
            }
        }
        let c = &self.curve;
        out.vec4([
            c.white_target,
            c.black_target,
            c.log2_paper_exposure,
            c.film_fog,
        ]);
        out.f32(c.film_power);
        out.f32(c.paper_power);
        out.f32(self.hue_preservation);
        out.u32(self.color_processing.to_darktable() as u32);
        out.f32(c.film_overflow);
        out.f32(c.film_underflow);
        out.u32(c.paper().to_kernel());
        out.u32(0);
        debug_assert_eq!(out.0.len(), PARAMS_SIZE);
        out.0
    }
}

#[derive(Default)]
struct Uniform(Vec<u8>);

impl Uniform {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn vec4(&mut self, v: [f32; 4]) {
        debug_assert_eq!(self.0.len() % 16, 0, "vec4 must sit at a 16-byte offset");
        for c in v {
            self.f32(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Settings {
        Settings::default()
    }

    /// Test 0097's settings, as its sidecar stores them.
    fn test_0097() -> Settings {
        Settings {
            middle_grey_contrast: f32::from_bits(0x3fe4_dd30),
            contrast_skewness: f32::from_bits(0x3eb8_51ec),
            hue_preservation: f32::from_bits(0x424a_7ae2),
            ..defaults()
        }
    }

    /// The corners of darktable's ranges for contrast, skew and the targets.
    fn extremes() -> Vec<Settings> {
        let mut all = Vec::new();
        for contrast in [0.1, 10.0] {
            for skew in [-1.0, 1.0] {
                for white in [20.0, 1600.0] {
                    for black in [0.0, 15.0] {
                        all.push(Settings {
                            middle_grey_contrast: contrast,
                            contrast_skewness: skew,
                            display_white_target: white,
                            display_black_target: black,
                            ..defaults()
                        });
                    }
                }
            }
        }
        all
    }

    #[test]
    fn params_fill_the_kernel_struct() {
        let prepared = prepare(&defaults(), &RgbSpace::LINEAR_REC2020);
        assert_eq!(prepared.params().len(), PARAMS_SIZE);
    }

    #[test]
    fn choices_round_trip_through_darktable_values() {
        for value in 0..5 {
            let base = BasePrimaries::from_darktable(value).unwrap();
            assert_eq!(base.to_darktable(), value);
        }
        assert_eq!(BasePrimaries::from_darktable(5), None);
        for value in 0..2 {
            let processing = ColorProcessing::from_darktable(value).unwrap();
            assert_eq!(processing.to_darktable(), value);
        }
        assert_eq!(ColorProcessing::from_darktable(2), None);
    }

    #[test]
    fn the_fits_constants_are_the_builds() {
        // As darktable's libsigmoid.so holds them (Apple's arm64 build).
        assert_eq!(GREY_ABOVE.to_bits(), 0x3e3c_edd4);
        assert_eq!(GREY_BELOW.to_bits(), 0x3e3c_ed4e);
        assert_eq!((GREY_ABOVE - MIDDLE_GREY).to_bits(), 0x3586_0000);
        assert_eq!((GREY_BELOW - MIDDLE_GREY).to_bits(), 0xb586_0000);
        assert_eq!(PERCENT_OVER_GREY.to_bits(), 0x3d5e_0164);
        // Not the correctly rounded quotient.
        assert_ne!(PERCENT_OVER_GREY, 0.01f32 / MIDDLE_GREY);
        assert_eq!((1.0 / MIDDLE_GREY - 1.0f32).to_bits(), 0x408d_7116);
    }

    /// Settings a photo would use: the defaults, test 0097's and darktable's presets
    /// (`init_presets`), the primaries left out.
    fn realistic() -> Vec<Settings> {
        let preset = |contrast, skew| Settings {
            middle_grey_contrast: contrast,
            contrast_skewness: skew,
            ..defaults()
        };
        let mut all = vec![defaults(), test_0097()];
        // Neutral grey, ACES 100-nit like, Reinhard and smooth.
        all.extend([
            preset(1.22, 0.65),
            preset(1.6, -0.2),
            preset(1.0, 0.0),
            preset(1.5, -0.2),
        ]);
        // Targets and contrasts off their defaults.
        all.push(Settings {
            middle_grey_contrast: 2.5,
            contrast_skewness: 0.4,
            display_white_target: 160.0,
            display_black_target: 0.5,
            ..defaults()
        });
        all.push(Settings {
            middle_grey_contrast: 0.8,
            contrast_skewness: -0.5,
            display_white_target: 60.0,
            display_black_target: 0.0,
            ..defaults()
        });
        all
    }

    #[test]
    fn the_curve_meets_its_targets() {
        for settings in realistic() {
            let c = Curve::new(&settings);
            let near =
                |a: f32, b: f32, tolerance: f32| (a - b).abs() <= tolerance * b.abs().max(1e-3);
            assert!(near(c.eval(0.0), c.black_target, 1e-4), "{c:?}");
            assert!(near(c.eval(MIDDLE_GREY), MIDDLE_GREY, 1e-4), "{c:?}");
            // 2^40 over grey, far past any photo; low contrasts approach white slowly.
            assert!(near(c.eval(2e11), c.white_target, 1e-2), "{c:?}");
            // Rising, by steps f32 can tell apart below 0.99 of white.
            let mut previous = c.eval(0.0);
            for k in 1..=200 {
                let y = c.eval((k as f32 / 8.0 - 15.0).exp2());
                assert!(y > previous || y > 0.99 * c.white_target, "{k}: {c:?}");
                assert!(y >= previous, "{k}: {c:?}");
                previous = y;
            }
        }
    }

    #[test]
    fn the_fit_follows_darktables_finite_differences() {
        // At test 0097's settings darktable's f32 central differences give a film power
        // 0.5% under the exact slopes' c (1 − g) / (q (1 − 1 / X)); a contrast one ulp
        // lower gives 2.7083333, 1.0% under.
        let settings = test_0097();
        let c = Curve::new(&settings);
        assert_eq!(c.film_power, 2.722_222_3);
        let g = f64::from(MIDDLE_GREY);
        let q = f64::from(c.paper_power);
        let x = (f64::from(c.white_target) / g).powf(1.0 / q);
        let exact = f64::from(settings.middle_grey_contrast) * (1.0 - g) / (q * (1.0 - 1.0 / x));
        let gap = f64::from(c.film_power) / exact - 1.0;
        assert!((-0.006..-0.004).contains(&gap), "{gap}");
        let lower = Curve::new(&Settings {
            middle_grey_contrast: settings.middle_grey_contrast.next_down(),
            ..settings
        });
        assert_eq!(lower.film_power, 2.708_333_3);
    }

    #[test]
    fn the_log_domain_curve_is_darktables() {
        // At the ranges' corners the film power reaches 120, where f32's log2 of the film
        // response is only good to about 1e-4 in either form.
        for settings in realistic() {
            let c = Curve::new(&settings);
            for k in 0..=400 {
                // From 2^-30 to 2^20, and 0.
                let x = if k == 0 {
                    0.0
                } else {
                    (k as f32 / 8.0 - 30.0).exp2()
                };
                let (ours, theirs) = (c.eval(x), c.darktable(x));
                assert!(
                    (ours - theirs).abs() <= 2e-5 * theirs.abs() + 1e-12,
                    "{x}: {ours} != {theirs} for {c:?}"
                );
            }
        }
    }

    #[test]
    fn the_kernels_coefficients_are_finite_over_darktables_ranges() {
        for settings in extremes() {
            let c = Curve::new(&settings);
            for value in [
                c.white_target,
                c.black_target,
                c.log2_paper_exposure,
                c.film_fog,
                c.film_power,
                c.paper_power,
            ] {
                assert!(value.is_finite(), "{c:?} for {settings:?}");
            }
            assert!(c.film_power > 0.0 && c.film_fog >= 0.0, "{c:?}");
            assert!(
                c.film_overflow.is_finite() && c.film_underflow >= 0.0,
                "{c:?}"
            );
        }
    }

    /// Inputs from 0 and 2^-30 to 2^30.
    fn inputs() -> impl Iterator<Item = f32> {
        std::iter::once(0.0).chain((0..=480).map(|k| (k as f32 / 8.0 - 30.0).exp2()))
    }

    #[test]
    fn darktables_breakdown_at_the_ranges_corner_is_all_white() {
        // The fog is 14 and the film power 99: the paper exposure overflows f32, and so
        // does the film response at every input, so darktable's NaN check makes every
        // pixel the target white. The kernel's curve does the same.
        let corner = Settings {
            middle_grey_contrast: 10.0,
            contrast_skewness: -1.0,
            display_white_target: 20.0,
            display_black_target: 15.0,
            ..defaults()
        };
        let c = Curve::new(&corner);
        assert_eq!(c.paper(), PaperExposure::Infinite);
        assert!(c.film_overflow <= c.film_fog, "{c:?}");
        for x in inputs() {
            assert_eq!(c.darktable(x), c.white_target, "{x}");
            assert_eq!(c.eval(x), c.white_target, "{x}");
        }
    }

    #[test]
    fn darktables_other_breakdowns_are_followed() {
        // The paper exposure overflows but the film response does not, up to beyond
        // grey: black, and the target white where the film response overflows too.
        let infinite = Curve::new(&Settings {
            middle_grey_contrast: 9.0,
            contrast_skewness: 1.0,
            display_white_target: 1600.0,
            display_black_target: 15.0,
            ..defaults()
        });
        assert_eq!(infinite.paper(), PaperExposure::Infinite);
        assert_eq!(infinite.darktable(MIDDLE_GREY), 0.0);
        assert_eq!(infinite.darktable(1e30), infinite.white_target);
        // The paper exposure underflows to 0: the target white everywhere, black's 0 too.
        let zero = Curve::new(&Settings {
            middle_grey_contrast: 6.0,
            contrast_skewness: 1.0,
            display_white_target: 20.0,
            display_black_target: 0.0,
            ..defaults()
        });
        assert_eq!(zero.paper(), PaperExposure::Zero);
        assert_eq!(zero.film_fog, 0.0);
        for c in [infinite, zero] {
            for x in inputs() {
                assert_eq!(c.eval(x), c.darktable(x), "{x}: {c:?}");
            }
        }
    }

    #[test]
    fn the_kernels_curve_is_darktables_at_the_ranges_corners() {
        // Where the paper exposure is finite, the film response's overflow to the target
        // white and its underflow to black are exact; the log domain between is only good
        // to about 1e-4 at film powers up to 120. Where darktable's film response is
        // subnormal it keeps fewer bits, and a build that flushes subnormals to 0 renders
        // black there: those inputs depend on the build, and are left out.
        for settings in extremes() {
            let c = Curve::new(&settings);
            let subnormal = |x: f32| {
                let response = powf(c.film_fog + x, c.film_power);
                response > 0.0 && response < f32::MIN_POSITIVE
            };
            for x in inputs().filter(|&x| !subnormal(x)) {
                let (ours, theirs) = (c.eval(x), c.darktable(x));
                assert!(
                    (ours - theirs).abs() <= 2e-4 * theirs.abs() + 1e-9,
                    "{x}: {ours} != {theirs} for {c:?}"
                );
            }
        }
    }

    #[test]
    fn the_film_responses_limits_are_powfs() {
        for settings in extremes().into_iter().chain(realistic()) {
            let c = Curve::new(&settings);
            let response = |film: f32| powf(film, c.film_power);
            if c.film_overflow < f32::MAX {
                assert!(response(c.film_overflow).is_infinite(), "{c:?}");
                assert!(response(c.film_overflow.next_down()).is_finite(), "{c:?}");
            } else {
                assert!(response(f32::MAX).is_finite(), "{c:?}");
            }
            assert_eq!(response(c.film_underflow), 0.0, "{c:?}");
            assert!(response(c.film_underflow.next_up()) > 0.0, "{c:?}");
        }
        // At the defaults no photo comes near either: 4.1e25 and 9.7e-31.
        let c = Curve::new(&defaults());
        assert!(c.film_overflow > 1e25 && c.film_underflow < 1e-30, "{c:?}");
    }

    #[test]
    fn hue_preservation_is_a_clamped_fraction() {
        let at = |h| {
            prepare(
                &Settings {
                    hue_preservation: h,
                    ..defaults()
                },
                &RgbSpace::LINEAR_REC2020,
            )
            .hue_preservation
        };
        assert_eq!(at(100.0), 1.0);
        assert_eq!(at(50.0), 0.5);
        assert_eq!(at(0.0), 0.0);
        assert_eq!(at(150.0), 1.0);
    }

    fn identity(matrix: &Mat3d, tolerance: f64) -> bool {
        matrix.iter().enumerate().all(|(r, row)| {
            row.iter()
                .enumerate()
                .all(|(c, v)| (v - if r == c { 1.0 } else { 0.0 }).abs() < tolerance)
        })
    }

    fn round_trip(m: &Matrices) -> Mat3d {
        color::mul_d(
            &m.rendering_to_pipe,
            &color::mul_d(&m.base_to_rendering, &m.pipe_to_base),
        )
    }

    #[test]
    fn the_working_profile_and_rec2020_need_no_conversion() {
        for base in [BasePrimaries::WorkingProfile, BasePrimaries::Rec2020] {
            let settings = Settings {
                base_primaries: base,
                ..defaults()
            };
            let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020);
            assert_eq!(m.pipe_to_base, color::IDENTITY_D, "{base:?}");
            // Unmodified primaries cancel: the way out undoes the way in.
            assert!(identity(&round_trip(&m), 1e-9), "{base:?}: {m:?}");
        }
    }

    #[test]
    fn full_purity_undoes_the_inset() {
        // With purity 1 the way out is the inverse of the way in, whatever the insets,
        // rotations and base primaries.
        for base in BasePrimaries::ALL {
            let settings = Settings {
                inset: [0.1, 0.2, 0.3],
                rotation: [0.05, -0.1, 0.2],
                purity: 1.0,
                base_primaries: base,
                ..defaults()
            };
            let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020);
            assert!(identity(&round_trip(&m), 1e-9), "{base:?}: {m:?}");
            assert!(!identity(&m.base_to_rendering, 1e-3), "{base:?}: {m:?}");
        }
    }

    #[test]
    fn matrices_keep_white() {
        // Every matrix is built around the same D50 white, so grey stays grey.
        for base in BasePrimaries::ALL {
            let settings = Settings {
                inset: [0.12, 0.08, 0.2],
                rotation: [0.035, -0.026, -0.061],
                purity: 0.4,
                base_primaries: base,
                ..defaults()
            };
            let m = Matrices::new(&settings, &RgbSpace::LINEAR_REC2020);
            for matrix in [m.pipe_to_base, m.base_to_rendering, m.rendering_to_pipe] {
                for row in matrix {
                    assert!(
                        (row.iter().sum::<f64>() - 1.0).abs() < 1e-9,
                        "{base:?}: {m:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn singular_outbound_matrices_render_black() {
        // Green and blue turned opposite each other across the white put the white on the
        // line through them, whatever the insets and purity: within the rotations' ±0.4
        // over Display P3 (∓0.3834) and sRGB (∓0.3389). Past that, an outbound primary
        // close to the white can sit on the line through the other two (sRGB, recover
        // purity 100% and red attenuation 99%). The outbound matrix is singular either way;
        // darktable's mat3SSEinv refuses it and leaves the outbound matrix uninitialised,
        // and the setup renders black, where the plain inverse has entries over 1e7, or
        // NaN. With purity 1 or no insets the way out inverts the way in, so the plain
        // inverse is `base_to_pipe · base_to_rendering^-1`. A slider's value nearby (21.9°,
        // 19.4°, 17.5°) is inverted as before.
        let largest = |m: &Mat3d| m.iter().flatten().map(|v| v.abs()).fold(0.0, f64::max);
        let finite = |m: &Mat3d| m.iter().flatten().all(|v| v.is_finite());
        let plain_inverse = |m: &Matrices| {
            color::mul_d(
                &color::inverse_d(&m.pipe_to_base),
                &color::inverse_d(&m.base_to_rendering),
            )
        };
        let turn = |green: f32, blue: f32| [0.0, -green, blue];
        let degrees = |d: f32| d.to_radians();
        for (base, purity, inset, singular, ordinary) in [
            (
                BasePrimaries::DisplayP3,
                0.0,
                [0.0; 3],
                turn(f32::from_bits(0x3ec4_4850), f32::from_bits(0x3ec4_4850)),
                turn(degrees(21.9), degrees(21.9)),
            ),
            (
                BasePrimaries::Srgb,
                0.0,
                [0.0; 3],
                turn(f32::from_bits(0x3ead_827c), f32::from_bits(0x3ead_827c)),
                turn(degrees(19.4), degrees(19.4)),
            ),
            (
                BasePrimaries::Srgb,
                1.0,
                [0.99, 0.0, 0.0],
                turn(0.4, f32::from_bits(0x3e9d_2465)),
                turn(0.4, degrees(17.5)),
            ),
        ] {
            let at = |rotation| Settings {
                base_primaries: base,
                purity,
                inset,
                rotation,
                ..defaults()
            };
            let m = Matrices::new(&at(singular), &RgbSpace::LINEAR_REC2020);
            let plain = plain_inverse(&m);
            assert!(!finite(&plain) || largest(&plain) > 1e7, "{plain:?}");
            assert_eq!(m.rendering_to_pipe, [[0.0; 3]; 3], "{m:?}");
            // The kernel's last step, `rendering_to_pipe` times finite values, is 0.
            let params = prepare(&at(singular), &RgbSpace::LINEAR_REC2020).params();
            assert!(
                params
                    .chunks(4)
                    .all(|v| f32::from_le_bytes(v.try_into().unwrap()).is_finite()),
                "{m:?}"
            );

            let m = Matrices::new(&at(ordinary), &RgbSpace::LINEAR_REC2020);
            assert_eq!(m.rendering_to_pipe, plain_inverse(&m), "{m:?}");
            assert!(finite(&m.rendering_to_pipe), "{m:?}");
            assert!(largest(&m.rendering_to_pipe) > 1.0, "{m:?}");
        }
    }
}
