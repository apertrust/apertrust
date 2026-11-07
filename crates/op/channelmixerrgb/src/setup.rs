// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/iop/channelmixerrgb.c)
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

//! Color calibration: setup for the `channelmixerrgb` kernel, ported from darktable's
//! `commit_params()` and the part of `process()` that runs once per image. See
//! `docs/ports/channelmixerrgb.md`, Setup on the CPU.

use apertrust_color::illuminants::{self, D50_XY, FLUORESCENT, LED};
use apertrust_color::{Adaptation, Illuminant, Mat3, WorkingProfile, daylight_multipliers, mul};
use apertrust_plugin::NotPorted;

/// darktable `dt_iop_channelmixer_rgb_version_t`: the colourfulness algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// "version 1 (2020)", for old edits.
    V1,
    /// "version 2 (2021)", for old edits.
    V2,
    /// "version 3 (Apr 2021)", every new edit's.
    V3,
}

impl Version {
    pub const ALL: [Self; 3] = [Self::V1, Self::V2, Self::V3];

    pub fn from_darktable(value: i32) -> Option<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }
}

/// The module's settings: darktable params version 3, field for field, without the
/// arrays' unused fourth elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// The mixer's rows: output channel `i` is `row_i · input`, in the adaptation space.
    pub red: [f32; 3],
    pub green: [f32; 3],
    pub blue: [f32; 3],
    /// Colourfulness, brightness and the grey weights, per channel of the adaptation
    /// space.
    pub saturation: [f32; 3],
    pub lightness: [f32; 3],
    pub grey: [f32; 3],
    pub normalize_r: bool,
    pub normalize_g: bool,
    pub normalize_b: bool,
    pub normalize_sat: bool,
    pub normalize_light: bool,
    pub normalize_grey: bool,
    pub illuminant: Illuminant,
    /// F1 to F12 as 0 to 11.
    pub illum_fluo: u32,
    /// B1 to B5, BH1, RGB1, V1, V2 as 0 to 8.
    pub illum_led: u32,
    pub adaptation: Adaptation,
    /// The custom illuminant, CIE 1931 2°, relative to the pipeline's D50.
    pub x: f32,
    pub y: f32,
    /// For the daylight and Planckian illuminants, in K.
    pub temperature: f32,
    /// Gamut compression; 0 is none.
    pub gamut: f32,
    /// Clip negative RGB from gamut.
    pub clip: bool,
    pub version: Version,
}

impl Settings {
    /// darktable's defaults before it looks at the image: `init`'s identity mixer and
    /// the introspection defaults (daylight at 5003 K, CAT16, gamut 1, clipping).
    pub const DEFAULT: Self = Self {
        red: [1.0, 0.0, 0.0],
        green: [0.0, 1.0, 0.0],
        blue: [0.0, 0.0, 1.0],
        saturation: [0.0; 3],
        lightness: [0.0; 3],
        grey: [0.0; 3],
        normalize_r: false,
        normalize_g: false,
        normalize_b: false,
        normalize_sat: false,
        normalize_light: false,
        normalize_grey: false,
        illuminant: Illuminant::D,
        illum_fluo: 2,
        illum_led: 4,
        adaptation: Adaptation::Cat16,
        x: 0.333,
        y: 0.333,
        temperature: 5003.0,
        gamut: 1.0,
        clip: true,
        version: Version::V3,
    };
}

/// What the camera's illuminant needs (Setup, 7): the camera's XYZ to camera matrix
/// under D65 and its as-shot multipliers, and the multipliers the image carries when it
/// reaches this operation, the image description's `white_balance`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraWhite {
    pub xyz_to_camera: Mat3,
    pub as_shot: [f32; 3],
    pub applied: [f32; 3],
}

impl CameraWhite {
    /// darktable `_get_white_balance_coeff`: 1 when the image is at the camera's D65
    /// reference (darktable's `_dev_is_D65_chroma`, each multiplier within 1e-5);
    /// otherwise the camera's D65 multipliers over the applied ones, when some applied
    /// multiplier exceeds 1, else 1.
    ///
    /// darktable compares temperature's coefficients, before colorin's late correction;
    /// after colorin, the image description holds them with the correction applied, which
    /// gives darktable's result in every state darktable reaches (the spec's Setup, 7).
    pub fn custom_wb(&self) -> [f32; 3] {
        let d65 = daylight_multipliers(&self.xyz_to_camera).map(|v| v as f32);
        let at_reference = (0..3).all(|c| (self.applied[c] - d65[c]).abs() < 1e-5);
        let valid = d65.iter().all(|&v| v > 0.0);
        let changed = self.applied.iter().any(|&v| v > 1.0);
        if at_reference || !valid || !changed {
            [1.0; 3]
        } else {
            [0, 1, 2].map(|c| d65[c] / self.applied[c])
        }
    }

    /// darktable `find_temperature_from_raw_coeffs`: the light the as-shot multipliers
    /// neutralise, as the pipeline sees it.
    pub fn illuminant_xy(&self) -> Option<[f32; 2]> {
        let custom = self.custom_wb();
        let wb = [0, 1, 2].map(|c| self.as_shot[c] * custom[c]);
        illuminants::camera_illuminant_xy(&self.xyz_to_camera, wb)
    }
}

/// The largest x + y `prepare` passes on: beyond 1 the illuminant's Z would be negative
/// (the spec's Design choices, 3). A pair past it is scaled back onto it.
pub const MAX_XY_SUM: f32 = 0.9999;

/// What a negative luminance-weighted distance in gamut mapping gives at the
/// compression, as darktable's `powf` does: possible only with clipping off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegativeDistance {
    /// Not a whole number: `powf` gives NaN, and darktable's clamps then return D50.
    Neutral,
    /// An odd whole number: a negative power.
    Odd,
    /// An even whole number: a positive power.
    Even,
}

/// What the kernel needs besides the image: `ChannelMixerParams`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prepared {
    pub to_lms: Mat3,
    pub mix_to_xyz: Mat3,
    pub cone: Mat3,
    pub to_space: Mat3,
    pub from_space: Mat3,
    pub to_rgb: Mat3,
    pub d50: [f32; 3],
    pub saturation: [f32; 3],
    pub lightness: [f32; 3],
    pub grey: [f32; 3],
    /// The illuminant's chromaticity, which the kernel converts and differentiates.
    pub x: f32,
    pub y: f32,
    pub compression: f32,
    pub clip: bool,
    pub adaptation: Adaptation,
    pub apply_grey: bool,
    pub negative_distance: NegativeDistance,
}

/// The kernel's uniform block after the pass header: 24 `float4`s.
pub const PARAMS_SIZE: usize = 384;

impl Prepared {
    /// Little-endian `ChannelMixerParams`, field by field in the kernel's order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut put = |value: [u8; 4]| bytes.extend_from_slice(&value);
        let mut vec3 = |v: [f32; 3]| {
            for c in [v[0], v[1], v[2], 0.0] {
                put(c.to_le_bytes());
            }
        };
        for matrix in [
            &self.to_lms,
            &self.mix_to_xyz,
            &self.cone,
            &self.to_space,
            &self.from_space,
            &self.to_rgb,
        ] {
            for row in matrix {
                vec3(*row);
            }
        }
        for v in [self.d50, self.saturation, self.lightness, self.grey] {
            vec3(v);
        }
        let clip_min = if self.clip { 0.0 } else { -f32::MAX };
        for v in [self.x, self.y, self.compression, clip_min] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let negative = match self.negative_distance {
            NegativeDistance::Neutral => 0u32,
            NegativeDistance::Odd => 1,
            NegativeDistance::Even => 2,
        };
        for v in [
            self.adaptation.to_darktable() as u32,
            u32::from(self.apply_grey),
            negative,
            0,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }
}

fn not_ported(what: &'static str) -> NotPorted {
    NotPorted {
        operation: "channelmixerrgb",
        what,
    }
}

/// darktable's normalisation of a mixer row: by its sum, unless that is 0, where
/// darktable divides by zero.
fn normalised(row: [f32; 3], normalize: bool) -> [f32; 3] {
    if normalize && normalisable(row) {
        let sum = row[0] + row[1] + row[2];
        row.map(|v| v / sum)
    } else {
        row
    }
}

/// The smallest sum of three weights, as a fraction of their sizes added up, that
/// normalisation divides by: about eight times `f32`'s rounding of the sum, so a smaller
/// one is rounding noise around 0.
pub const NORMALISE_MIN: f32 = 1e-6;

/// Whether darktable's normalisation of these weights, a division by their sum, is
/// done: darktable divides whenever the sum is not 0, by zero for the mixer's rows and
/// not for the grey weights. Apertrust leaves weights whose sum is within
/// [`NORMALISE_MIN`] of their sizes unnormalised, as darktable does grey weights that
/// sum to 0, so the kernel's gains stay within 1e6.
fn normalisable(weights: [f32; 3]) -> bool {
    let sum = weights[0] + weights[1] + weights[2];
    let sizes = weights[0].abs() + weights[1].abs() + weights[2].abs();
    sum.abs() > NORMALISE_MIN * sizes
}

/// The illuminant's chromaticity for these settings: darktable `illuminant_to_xy` from
/// `commit_params`, falling back to the stored x and y; the camera's comes from
/// `camera`, or is unknown without it.
pub fn illuminant_xy(settings: &Settings, camera: Option<&CameraWhite>) -> [f32; 2] {
    let camera_xy = camera.and_then(CameraWhite::illuminant_xy);
    illuminants::illuminant_to_xy(
        settings.illuminant,
        settings.temperature,
        settings.illum_fluo as usize,
        settings.illum_led as usize,
        camera_xy,
    )
    .unwrap_or([settings.x, settings.y])
}

/// darktable `commit_params`: the settings, in the working space `profile`, as the
/// kernel's inputs. `camera` is what the camera illuminant needs, `None` for an image
/// without a camera's white balance, where it falls back to the stored x and y as in
/// darktable.
pub fn prepare(
    settings: &Settings,
    profile: &WorkingProfile,
    camera: Option<&CameraWhite>,
) -> Result<Prepared, NotPorted> {
    let s = settings;
    if s.version != Version::V3 {
        return Err(not_ported(
            "the colourfulness of versions 1 and 2, for old darktable edits",
        ));
    }
    if matches!(
        s.illuminant,
        Illuminant::DetectSurfaces | Illuminant::DetectEdges
    ) {
        return Err(not_ported(
            "illuminant detection, which darktable compiles out",
        ));
    }
    if s.illum_fluo as usize >= FLUORESCENT.len() || s.illum_led as usize >= LED.len() {
        return Err(not_ported(
            "fluorescent or LED illuminants past darktable's",
        ));
    }

    let mix = [
        normalised(s.red, s.normalize_r),
        normalised(s.green, s.normalize_g),
        normalised(s.blue, s.normalize_b),
    ];
    let mean = |v: [f32; 3]| (v[0] + v[1] + v[2]) / 3.0;
    let sat_offset = if s.normalize_sat {
        mean(s.saturation)
    } else {
        0.0
    };
    let light_offset = if s.normalize_light {
        mean(s.lightness)
    } else {
        0.0
    };
    let grey_norm = if s.normalize_grey && normalisable(s.grey) {
        s.grey[0] + s.grey[1] + s.grey[2]
    } else {
        1.0
    };

    // darktable forces CAT16 for the camera's illuminant, through
    // `_check_if_close_to_daylight`.
    let adaptation = if s.illuminant == Illuminant::Camera {
        Adaptation::Cat16
    } else {
        s.adaptation
    };

    let [mut x, mut y] = illuminant_xy(s, camera);
    if x + y > MAX_XY_SUM {
        let scale = MAX_XY_SUM / (x + y);
        (x, y) = (x * scale, y * scale);
    }

    let rgb_to_xyz = profile.rgb_to_xyz_d50;
    let xyz_to_rgb = profile.xyz_d50_to_rgb;
    let cone = adaptation.xyz_to_lms();
    // darktable `_loop_switch`'s matrices, and where its second and third switches go.
    let (to_lms, mix_to_xyz, to_space, from_space) = match adaptation {
        Adaptation::LinearBradford | Adaptation::Cat16 => (
            mul(&cone, &rgb_to_xyz),
            mul(&adaptation.lms_to_xyz(), &mix),
            cone,
            adaptation.lms_to_xyz(),
        ),
        Adaptation::FullBradford => (
            rgb_to_xyz,
            mul(&adaptation.lms_to_xyz(), &mix),
            cone,
            adaptation.lms_to_xyz(),
        ),
        Adaptation::Xyz => (rgb_to_xyz, mix, cone, cone),
        Adaptation::Rgb => (
            apertrust_color::adaptation::IDENTITY,
            mul(&rgb_to_xyz, &mix),
            xyz_to_rgb,
            rgb_to_xyz,
        ),
    };

    let compression = if s.gamut == 0.0 { 0.0 } else { 1.0 / s.gamut };
    let negative_distance = if compression.fract() != 0.0 {
        NegativeDistance::Neutral
    } else if (compression / 2.0).fract() == 0.0 {
        NegativeDistance::Even
    } else {
        NegativeDistance::Odd
    };

    Ok(Prepared {
        to_lms,
        mix_to_xyz,
        cone,
        to_space,
        from_space,
        to_rgb: xyz_to_rgb,
        d50: adaptation.d50(),
        saturation: s.saturation.map(|v| -v + sat_offset),
        lightness: s.lightness.map(|v| v - light_offset),
        grey: s.grey.map(|v| v / grey_norm),
        x,
        y,
        compression,
        clip: s.clip,
        adaptation,
        apply_grey: s.grey.iter().any(|&v| v != 0.0),
        negative_distance,
    })
}

/// What darktable's `reload_defaults` looks at in the image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DefaultsFor<'a> {
    /// An image that is not a raw: darktable's introspection defaults, with the module
    /// off.
    NotRaw,
    /// A monochrome raw: darktable's plain channel mixer.
    MonochromeRaw,
    /// A colour raw (darktable's `dt_image_is_matrix_correction_supported`), with what
    /// its camera's illuminant needs, if it has a matrix and as-shot multipliers.
    ColourRaw(Option<&'a CameraWhite>),
}

/// The settings an unedited image starts from, as darktable's `reload_defaults` gives them
/// (the spec's Defaults; decision 27). A colour raw stores the camera's chromaticity in x
/// and y, the one for an image at the camera's reference, as temperature's "as shot to
/// reference" leaves it, or darktable's 0.333, 0.333 without a camera illuminant; then
/// `_check_if_close_to_daylight` describes it: daylight or Planckian at its correlated
/// colour temperature where either lies within 0.005 of it in darktable's UCS, which the
/// render then reads instead of x and y, custom otherwise, under CAT16. A monochrome raw
/// gets darktable's plain channel mixer, the pipeline's illuminant without adaptation; any
/// other image the introspection defaults.
pub fn image_defaults(image: DefaultsFor) -> Settings {
    let mut settings = Settings::DEFAULT;
    let camera = match image {
        DefaultsFor::NotRaw => return settings,
        DefaultsFor::MonochromeRaw => {
            settings.illuminant = Illuminant::Pipe;
            settings.adaptation = Adaptation::Rgb;
            return settings;
        }
        DefaultsFor::ColourRaw(camera) => camera,
    };
    // darktable's `find_temperature_from_raw_coeffs` into x and y, then
    // `_check_if_close_to_daylight` on them, whether the camera gave a light or not.
    let at_reference = camera.and_then(|camera| {
        CameraWhite {
            applied: daylight_multipliers(&camera.xyz_to_camera).map(|v| v as f32),
            ..*camera
        }
        .illuminant_xy()
    });
    if let Some([x, y]) = at_reference {
        (settings.x, settings.y) = (x, y);
    }
    let (temperature, model) = illuminants::closest_cct_model(settings.x, settings.y);
    let (min, max) = illuminants::TEMPERATURE_RANGE;
    settings.illuminant = model;
    settings.temperature = temperature.clamp(min, max);
    settings.adaptation = Adaptation::Cat16;
    settings
}

/// darktable's D50 chromaticity, the `pipe` illuminant.
pub const PIPE_XY: [f32; 2] = D50_XY;

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_color::adaptation::{
        BRADFORD_LMS_TO_XYZ, CAT16_LMS_TO_XYZ, IDENTITY, XYZ_TO_BRADFORD_LMS, XYZ_TO_CAT16_LMS,
    };
    use apertrust_color::illuminants::{cct_to_xy_blackbody, cct_to_xy_daylight};

    fn profile() -> WorkingProfile {
        WorkingProfile::linear_rec2020()
    }

    fn prepared(settings: &Settings) -> Prepared {
        prepare(settings, &profile(), None).unwrap()
    }

    /// The Fujifilm X-T4 of test 0165: darktable's matrix and the as-shot multipliers.
    fn x_t4(applied: [f32; 3]) -> CameraWhite {
        CameraWhite {
            xyz_to_camera: [
                [1.3426, -0.6334, -0.1177],
                [-0.4244, 1.2136, 0.2371],
                [-0.058, 0.1303, 0.598],
            ],
            as_shot: [1.9139073, 1.0, 2.1887417],
            applied,
        }
    }

    fn d65(camera: &CameraWhite) -> [f32; 3] {
        daylight_multipliers(&camera.xyz_to_camera).map(|v| v as f32)
    }

    #[test]
    fn params_have_the_kernels_layout() {
        let p = prepared(&Settings::DEFAULT);
        let bytes = p.params();
        assert_eq!(bytes.len(), PARAMS_SIZE);
        let float = |i: usize| f32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
        let word = |i: usize| u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
        // d50 is the 19th float4; x, y, compression and the clip floor follow the grey
        // weights, then the adaptation, grey and negative-distance flags.
        assert_eq!([float(72), float(73), float(74)], Adaptation::Cat16.d50());
        assert_eq!([float(88), float(89)], [p.x, p.y]);
        assert_eq!([float(90), float(91)], [1.0, 0.0]);
        assert_eq!([word(92), word(93), word(94)], [1, 0, 1]);
        let unclipped = Prepared { clip: false, ..p }.params();
        assert_eq!(
            f32::from_le_bytes(unclipped[364..368].try_into().unwrap()),
            -f32::MAX
        );
    }

    #[test]
    fn illuminants_resolve_as_darktables_commit_params() {
        let with = |illuminant, temperature| Settings {
            illuminant,
            temperature,
            x: 0.31,
            y: 0.32,
            ..Settings::DEFAULT
        };
        let xy = |settings: &Settings| {
            let p = prepared(settings);
            [p.x, p.y]
        };
        assert_eq!(xy(&with(Illuminant::Pipe, 5000.0)), PIPE_XY);
        assert_eq!(xy(&with(Illuminant::D, 5003.0)), cct_to_xy_daylight(5003.0));
        // The spec's values: daylight at 5003 K, the Planckian at 3927 K, and daylight
        // below 4000 K is the Planckian.
        let d = xy(&with(Illuminant::D, 5003.0));
        assert!((d[0] - 0.34565).abs() < 1e-5 && (d[1] - 0.35860).abs() < 1e-5);
        let bb = xy(&with(Illuminant::BlackBody, 3927.0));
        assert!((bb[0] - 0.38382).abs() < 1e-5 && (bb[1] - 0.37878).abs() < 1e-5);
        assert_eq!(
            xy(&with(Illuminant::D, 3927.0)),
            cct_to_xy_blackbody(3927.0)
        );
        assert_eq!(xy(&with(Illuminant::Custom, 5000.0)), [0.31, 0.32]);
        // The camera's without a camera is the stored x and y, as in darktable.
        assert_eq!(xy(&with(Illuminant::Camera, 5000.0)), [0.31, 0.32]);
        let fluorescent = Settings {
            illuminant: Illuminant::F,
            illum_fluo: 11,
            ..Settings::DEFAULT
        };
        assert_eq!(xy(&fluorescent), FLUORESCENT[11]);
        let led = Settings {
            illuminant: Illuminant::Led,
            illum_led: 0,
            ..Settings::DEFAULT
        };
        assert_eq!(xy(&led), LED[0]);
    }

    #[test]
    fn the_camera_illuminant_follows_the_image_description() {
        // At the camera's reference ("as shot to reference" after colorin's late
        // correction, or temperature at the D65 multipliers): the as-shot illuminant,
        // darktable's own result for test 0165.
        let reference = x_t4(d65(&x_t4([1.0; 3])));
        assert_eq!(reference.custom_wb(), [1.0; 3]);
        let [x, y] = reference.illuminant_xy().unwrap();
        assert!((x - 0.3892037).abs() < 2e-7 && (y - 0.41662326).abs() < 2e-7);
        // Balanced as shot without the correction: darktable's D65 over the applied
        // multipliers, which brings the illuminant to D50 as seen through the camera's
        // own balance.
        let as_shot = x_t4([1.9139073, 1.0, 2.1887417]);
        let custom = as_shot.custom_wb();
        let daylight = d65(&as_shot);
        for c in 0..3 {
            assert!((custom[c] - daylight[c] / as_shot.as_shot[c]).abs() < 1e-6);
        }
        let [x, y] = as_shot.illuminant_xy().unwrap();
        let pipe = PIPE_XY;
        assert!(
            (x - pipe[0]).abs() < 2e-3 && (y - pipe[1]).abs() < 2e-3,
            "{x} {y}"
        );
        // Without temperature (every multiplier 1): no correction, darktable's guard.
        assert_eq!(x_t4([1.0; 3]).custom_wb(), [1.0; 3]);
        // The camera forces CAT16, whatever the adaptation set.
        let settings = Settings {
            illuminant: Illuminant::Camera,
            adaptation: Adaptation::Xyz,
            ..Settings::DEFAULT
        };
        let p = prepare(&settings, &profile(), Some(&reference)).unwrap();
        assert_eq!(p.adaptation, Adaptation::Cat16);
        assert!((p.x - 0.3892037).abs() < 2e-7);
    }

    #[test]
    fn the_matrices_follow_darktables_loop_switch() {
        let profile = profile();
        let rgb_to_xyz = profile.rgb_to_xyz_d50;
        let mixer = Settings {
            red: [0.9, 0.1, 0.0],
            green: [0.0, 1.2, -0.2],
            blue: [0.1, 0.0, 0.9],
            ..Settings::DEFAULT
        };
        let mix = [mixer.red, mixer.green, mixer.blue];
        let at = |adaptation| {
            prepared(&Settings {
                adaptation,
                ..mixer
            })
        };
        let cat16 = at(Adaptation::Cat16);
        assert_eq!(cat16.to_lms, mul(&XYZ_TO_CAT16_LMS, &rgb_to_xyz));
        assert_eq!(cat16.mix_to_xyz, mul(&CAT16_LMS_TO_XYZ, &mix));
        assert_eq!(
            (cat16.cone, cat16.to_space, cat16.from_space),
            (XYZ_TO_CAT16_LMS, XYZ_TO_CAT16_LMS, CAT16_LMS_TO_XYZ)
        );
        let linear = at(Adaptation::LinearBradford);
        assert_eq!(linear.to_lms, mul(&XYZ_TO_BRADFORD_LMS, &rgb_to_xyz));
        assert_eq!(linear.mix_to_xyz, mul(&BRADFORD_LMS_TO_XYZ, &mix));
        let full = at(Adaptation::FullBradford);
        assert_eq!(full.to_lms, rgb_to_xyz);
        assert_eq!(full.cone, XYZ_TO_BRADFORD_LMS);
        assert_eq!(full.from_space, BRADFORD_LMS_TO_XYZ);
        let xyz = at(Adaptation::Xyz);
        assert_eq!((xyz.to_lms, xyz.mix_to_xyz), (rgb_to_xyz, mix));
        assert_eq!((xyz.cone, xyz.to_space), (IDENTITY, IDENTITY));
        let none = at(Adaptation::Rgb);
        assert_eq!(none.to_lms, IDENTITY);
        assert_eq!(none.mix_to_xyz, mul(&rgb_to_xyz, &mix));
        assert_eq!(
            (none.to_space, none.from_space),
            (profile.xyz_d50_to_rgb, rgb_to_xyz)
        );
        assert_eq!(none.d50, [1.0; 3]);
        for p in [cat16, linear, full, xyz, none] {
            assert_eq!(p.to_rgb, profile.xyz_d50_to_rgb);
        }
    }

    #[test]
    fn normalisation_follows_darktable_and_guards_a_zero_sum() {
        let settings = Settings {
            red: [-0.969, 0.476, 0.0],
            green: [0.5, -0.5, 0.0],
            blue: [0.0, 0.0, 2.0],
            saturation: [0.3, 0.6, 0.0],
            lightness: [0.3, 0.0, 0.0],
            normalize_r: true,
            normalize_g: true,
            normalize_b: true,
            normalize_sat: true,
            normalize_light: true,
            adaptation: Adaptation::Xyz,
            ..Settings::DEFAULT
        };
        let p = prepared(&settings);
        let red_sum = -0.969f32 + 0.476;
        assert_eq!(p.mix_to_xyz[0], [-0.969 / red_sum, 0.476 / red_sum, 0.0]);
        // A zero sum is left unnormalised, where darktable divides by zero.
        assert_eq!(p.mix_to_xyz[1], [0.5, -0.5, 0.0]);
        assert_eq!(p.mix_to_xyz[2], [0.0, 0.0, 1.0]);
        // Colourfulness changes sign; both lose their mean.
        assert_eq!(p.saturation, [-0.3 + 0.3, -0.6 + 0.3, 0.3]);
        assert_eq!(p.lightness, [0.3 - 0.1, -0.1, -0.1]);
    }

    #[test]
    fn grey_output_and_its_normalisation() {
        assert!(!prepared(&Settings::DEFAULT).apply_grey);
        // darktable's "monochrome | ILFORD HP5+" preset.
        let hp5 = Settings {
            grey: [0.25304098, 0.25958747, 0.48737156],
            normalize_grey: true,
            illuminant: Illuminant::Pipe,
            adaptation: Adaptation::Xyz,
            gamut: 0.0,
            ..Settings::DEFAULT
        };
        let p = prepared(&hp5);
        assert!(p.apply_grey);
        let sum: f32 = hp5.grey.iter().sum();
        assert_eq!(p.grey, hp5.grey.map(|v| v / sum));
        assert_eq!(p.compression, 0.0);
        // Weights that sum to 0 are left as they are, as in darktable.
        let zero = Settings {
            grey: [0.5, -0.5, 0.0],
            ..hp5
        };
        let p = prepared(&zero);
        assert!(p.apply_grey);
        assert_eq!(p.grey, [0.5, -0.5, 0.0]);
    }

    #[test]
    fn negative_distances_follow_powf() {
        let at = |gamut| {
            prepared(&Settings {
                gamut,
                ..Settings::DEFAULT
            })
        };
        assert_eq!(at(1.0).compression, 1.0);
        assert_eq!(at(1.0).negative_distance, NegativeDistance::Odd);
        assert_eq!(at(0.5).negative_distance, NegativeDistance::Even);
        assert_eq!(at(4.0).compression, 0.25);
        assert_eq!(at(4.0).negative_distance, NegativeDistance::Neutral);
    }

    #[test]
    fn x_plus_y_stays_below_1() {
        let p = prepared(&Settings {
            illuminant: Illuminant::Custom,
            x: 0.6,
            y: 0.6,
            ..Settings::DEFAULT
        });
        assert!((p.x + p.y - MAX_XY_SUM).abs() < 1e-6 && (p.x - p.y).abs() < 1e-7);
    }

    #[test]
    fn what_is_not_ported() {
        for version in [Version::V1, Version::V2] {
            let settings = Settings {
                version,
                ..Settings::DEFAULT
            };
            assert!(prepare(&settings, &profile(), None).is_err());
        }
        for illuminant in [Illuminant::DetectSurfaces, Illuminant::DetectEdges] {
            let settings = Settings {
                illuminant,
                ..Settings::DEFAULT
            };
            assert!(prepare(&settings, &profile(), None).is_err());
        }
    }

    #[test]
    fn defaults_describe_the_cameras_light_as_darktable_does() {
        // darktable's own defaults for test 0165, the X-T4: the camera's light, too far
        // from both loci to snap, so custom.
        let camera = x_t4([1.0; 3]);
        let settings = image_defaults(DefaultsFor::ColourRaw(Some(&camera)));
        assert_eq!(settings.illuminant, Illuminant::Custom);
        assert_eq!(settings.adaptation, Adaptation::Cat16);
        assert!((settings.x - 0.3892037).abs() < 2e-7 && (settings.y - 0.41662326).abs() < 2e-7);
        assert!((settings.temperature - 4005.0747).abs() < 0.01);
        // And for test 0174, mire1.cr2's Canon EOS 40D (darktable's matrix and the
        // as-shot multipliers): the camera's light in x and y, snapped to daylight at
        // 3927.06 K, below 4000 K the Planckian, which the render reads.
        let canon = CameraWhite {
            xyz_to_camera: [
                [0.6071, -0.0747, -0.0856],
                [-0.7653, 1.5365, 0.2441],
                [-0.2025, 0.2553, 0.7315],
            ],
            as_shot: [2.115_234_4, 1.0, 1.398_437_5],
            applied: [1.0; 3],
        };
        let settings = image_defaults(DefaultsFor::ColourRaw(Some(&canon)));
        assert_eq!(
            (settings.illuminant, settings.adaptation),
            (Illuminant::D, Adaptation::Cat16)
        );
        assert!((settings.x - 0.381_967_5).abs() < 2e-7 && (settings.y - 0.3699879).abs() < 2e-7);
        assert!((settings.temperature - 3927.0596).abs() < 0.01);
        assert_eq!(
            illuminant_xy(&settings, None),
            cct_to_xy_blackbody(settings.temperature)
        );
        let monochrome = image_defaults(DefaultsFor::MonochromeRaw);
        assert_eq!(
            (monochrome.illuminant, monochrome.adaptation),
            (Illuminant::Pipe, Adaptation::Rgb)
        );
        assert_eq!(image_defaults(DefaultsFor::NotRaw), Settings::DEFAULT);
    }

    #[test]
    fn a_raw_without_a_camera_illuminant_gets_darktables_defaults() {
        // darktable's reload_defaults runs _check_if_close_to_daylight on its stored
        // 0.333, 0.333 when the camera gives no illuminant, as on the camera's light: too
        // far from both loci, so custom, at its correlated colour temperature, under CAT16.
        let settings = image_defaults(DefaultsFor::ColourRaw(None));
        let (temperature, model) = illuminants::closest_cct_model(0.333, 0.333);
        assert_eq!(model, Illuminant::Custom);
        assert!((temperature - 5491.0).abs() < 1.0, "{temperature}");
        assert_eq!(
            (
                settings.illuminant,
                settings.adaptation,
                settings.x,
                settings.y
            ),
            (Illuminant::Custom, Adaptation::Cat16, 0.333, 0.333)
        );
        assert_eq!(settings.temperature, temperature);
        // Multipliers that are not normal give no camera illuminant either.
        let unrecorded = CameraWhite {
            as_shot: [0.0, 1.0, 1.0],
            ..x_t4([1.0; 3])
        };
        assert_eq!(
            image_defaults(DefaultsFor::ColourRaw(Some(&unrecorded))),
            settings
        );
    }

    #[test]
    fn weights_summing_to_rounding_noise_stay_unnormalised() {
        // (1, -1, 1e-30) sums to 1e-30, which darktable divides by, giving gains of
        // 1e30; a sum within 1e-6 of the weights' sizes is left alone, as a zero sum.
        let settings = Settings {
            red: [1.0, -1.0, 1e-30],
            green: [2.0, -1.9999995, 0.0],
            blue: [0.5, -0.5, 1e-5],
            grey: [1.0, -1.0, 1e-30],
            normalize_r: true,
            normalize_g: true,
            normalize_b: true,
            normalize_grey: true,
            adaptation: Adaptation::Xyz,
            ..Settings::DEFAULT
        };
        let p = prepared(&settings);
        assert_eq!(p.mix_to_xyz[0], settings.red);
        assert_eq!(p.mix_to_xyz[1], settings.green);
        assert_eq!(p.grey, settings.grey);
        // 1e-5 of the sizes is a sum normalisation divides by.
        let sum = 0.5f32 + -0.5 + 1e-5;
        assert_eq!(p.mix_to_xyz[2], settings.blue.map(|v| v / sum));
    }
}
