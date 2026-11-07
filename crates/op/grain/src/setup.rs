// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2024  darktable developers (ported from src/iop/grain.c)
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

//! Film grain: setup for the `grain` kernel, ported from darktable's `commit_params()`,
//! which copies the parameters and fills the paper response's table, and the start of
//! `process()`. The kernel computes the table's entries itself, so their derivatives
//! reach the mid-tones bias. See `docs/ports/grain.md`.

use apertrust_color::{Mat3, WorkingProfile};
use apertrust_plugin::NotPorted;

/// darktable `GRAIN_SCALE_FACTOR`: the coarseness its slider shows as ISO is `scale`
/// times this.
pub const SCALE_FACTOR: f64 = 213.2;

/// darktable `_simplex_2d_noise`'s octave frequencies.
pub const FREQUENCY: [f64; 3] = [0.4910, 0.9441, 1.7280];

/// The noise's period along x and along y, in its own units: a step of 768 in x moves
/// darktable's skewed cell by (1024, 256, 256), whole turns of its 256-entry permutation,
/// and leaves the point's place in the cell as it was.
pub const PERIOD: f64 = 768.0;

/// darktable's permutation, Ken Perlin's (`permutation` in `grain.c`).
pub const PERMUTATION: [u32; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

/// darktable `perm` (`_simplex_noise_init`), the kernel's table at binding 3: the
/// permutation twice, 512 little-endian `u32`, so no index needs wrapping.
pub fn permutation_table() -> Vec<u8> {
    PERMUTATION
        .iter()
        .chain(&PERMUTATION)
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// Where the noise starts, in the image's shorter sides from its left edge, for every
/// image the engine renders: provisional, until the planned seed stored in the edit
/// (`docs/ports/grain.md`, Deliberate differences). darktable starts at
/// a hash of the file name ([`darktable_offset`]); 0 starts at the image's top-left
/// corner, where the noise's first octave is 0. The golden cases set darktable's offset
/// in [`Settings`]; the schema has no such parameter, darktable no such field.
pub const PROVISIONAL_OFFSET: u32 = 0;

/// The module's settings: darktable params version 2's continuous fields, and the
/// offset darktable derives from the file name.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// "coarseness": the grain's size, ISO / [`SCALE_FACTOR`].
    pub scale: f32,
    /// The grain's strength, in percent.
    pub strength: f32,
    /// "mid-tones bias", in percent: how much the paper response makes the grain fall
    /// off in shadows and highlights.
    pub midtones_bias: f32,
    /// Where the noise starts, in the image's shorter sides from its left edge:
    /// darktable's hash of the file name ([`darktable_offset`]) in the golden cases,
    /// [`PROVISIONAL_OFFSET`] in the engine.
    pub offset: u32,
}

impl Default for Settings {
    /// darktable's defaults, its introspection's (the module has no `init`), at the
    /// provisional offset.
    fn default() -> Self {
        Self {
            scale: (1600.0 / SCALE_FACTOR) as f32,
            strength: 25.0,
            midtones_bias: 100.0,
            offset: PROVISIONAL_OFFSET,
        }
    }
}

/// darktable's introspection ranges: `scale` from 20 to 6400 ISO, `strength` and
/// `midtones_bias` from 0 to 100.
pub fn scale_range() -> (f32, f32) {
    ((20.0 / SCALE_FACTOR) as f32, (6400.0 / SCALE_FACTOR) as f32)
}

/// darktable `_hash_string`: DJBX33X over the name's bytes from the last to the first,
/// each byte a signed `char` as on x86-64 and on Apple's arm64 (Linux on arm64 has
/// unsigned `char`, which changes the hash of a name with bytes above 127).
pub fn hash_string(name: &str) -> u32 {
    name.bytes().rev().fold(5381u32, |hash, byte| {
        (hash << 5).wrapping_add(hash) ^ (byte as i8 as i32 as u32)
    })
}

/// darktable `process()`'s offset: the file name's hash modulo 0.3 times the width of
/// the region the pipe renders (`roi_out->width`), at least 1. An export at full
/// resolution renders the whole image, so `width` is its width; darktable's previews
/// render other widths, and so other offsets (`docs/ports/grain.md`).
pub fn darktable_offset(file_name: &str, width: u32) -> u32 {
    let modulus = (f64::from(width) * 0.3).max(1.0) as u32;
    hash_string(file_name) % modulus
}

/// darktable's zoom, `(1.0 + 8 * scale / 100) / 800.0`, as its release build computes
/// it: `-ffast-math` makes `8 * scale / 100` the float product `scale * 0.08f`, the rest
/// is double. The offset magnifies the zoom's last bits (Precision in the spec).
pub fn zoom(scale: f32) -> f64 {
    (1.0 + f64::from(scale * 0.08)) / 800.0
}

/// darktable's offset term per octave, `offset · frequency / zoom`, which it adds to the
/// position in double, reduced by the noise's [`PERIOD`] in f64 for the kernel.
pub fn noise_offsets(offset: u32, scale: f32) -> [f32; 3] {
    let zoom = zoom(scale);
    FREQUENCY.map(|f| ((f64::from(offset) * f / zoom) % PERIOD) as f32)
}

/// One of the continuous parameters the derivative entry points differentiate against,
/// in darktable's units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    Strength,
    Scale,
    MidtonesBias,
}

impl Parameter {
    pub const ALL: [Self; 3] = [Self::Strength, Self::Scale, Self::MidtonesBias];

    /// The kernel's number for it.
    pub fn index(self) -> u32 {
        match self {
            Self::Strength => 0,
            Self::Scale => 1,
            Self::MidtonesBias => 2,
        }
    }

    /// Its name in the schema.
    pub fn name(self) -> &'static str {
        match self {
            Self::Strength => "strength",
            Self::Scale => "scale",
            Self::MidtonesBias => "midtones_bias",
        }
    }
}

/// The kernel's number for an empty gradient slot.
const NO_PARAMETER: u32 = 3;

/// What the kernel needs besides the image, its table and the pass header, which gives
/// the image's size at full resolution and the pipe's scale.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub strength: f32,
    pub scale: f32,
    pub midtones_bias: f32,
    /// darktable's offset term per octave, from [`noise_offsets`]. Held when a gradient
    /// slot moves the scale: the derivative is the grain's at a fixed offset (Gradients
    /// in the spec).
    pub offset: [f32; 3],
    /// Working RGB to XYZ D50 and back, darktable's conversions around a Lab module.
    pub rgb_to_xyz: Mat3,
    pub xyz_to_rgb: Mat3,
    /// The parameters the derivative entry points differentiate against, in their order;
    /// the fourth slot is empty. `main` ignores them.
    pub gradient: [Parameter; 3],
}

/// Bytes of `GrainParams`, the kernel's parameters after the pass header.
pub const PARAMS_SIZE: usize = 144;

/// The gradient slots `prepare` chooses.
pub const DEFAULT_GRADIENT: [Parameter; 3] = [
    Parameter::Strength,
    Parameter::Scale,
    Parameter::MidtonesBias,
];

impl Prepared {
    /// Little-endian `GrainParams`, in the kernel's field order.
    pub fn params(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(PARAMS_SIZE);
        let mut put = |word: [u8; 4]| bytes.extend_from_slice(&word);
        for v in [self.strength, self.scale, self.midtones_bias] {
            put(v.to_le_bytes());
        }
        put(0u32.to_le_bytes());
        for v in self.offset {
            put(v.to_le_bytes());
        }
        put(0f32.to_le_bytes());
        for row in self.rgb_to_xyz.iter().chain(&self.xyz_to_rgb) {
            for v in row.iter().chain(&[0.0]) {
                put(v.to_le_bytes());
            }
        }
        for parameter in &self.gradient {
            put(parameter.index().to_le_bytes());
        }
        put(NO_PARAMETER.to_le_bytes());
        debug_assert_eq!(bytes.len(), PARAMS_SIZE);
        bytes
    }

    /// The value a gradient slot stands for.
    pub fn value_mut(&mut self, parameter: Parameter) -> &mut f32 {
        match parameter {
            Parameter::Strength => &mut self.strength,
            Parameter::Scale => &mut self.scale,
            Parameter::MidtonesBias => &mut self.midtones_bias,
        }
    }
}

const OPERATION: &str = "grain";

/// darktable `commit_params()`, which copies the parameters, and the start of
/// `process()`: the offset's term per octave, and the working space's conversions to
/// and from Lab. The kernel evaluates darktable's paper-response table where it reads it.
pub fn prepare(settings: &Settings, profile: &WorkingProfile) -> Result<Prepared, NotPorted> {
    let refuse = |what| {
        Err(NotPorted {
            operation: OPERATION,
            what,
        })
    };
    let values = [settings.scale, settings.strength, settings.midtones_bias];
    if !values.iter().all(|v| v.is_finite()) {
        return refuse("a parameter that is not a finite number");
    }
    let (min_scale, max_scale) = scale_range();
    let in_range = (min_scale..=max_scale).contains(&settings.scale)
        && (0.0..=100.0).contains(&settings.strength)
        && (0.0..=100.0).contains(&settings.midtones_bias);
    if !in_range {
        return refuse("a parameter outside darktable's range");
    }
    Ok(Prepared {
        strength: settings.strength,
        scale: settings.scale,
        midtones_bias: settings.midtones_bias,
        offset: noise_offsets(settings.offset, settings.scale),
        rgb_to_xyz: profile.rgb_to_xyz_d50,
        xyz_to_rgb: profile.xyz_d50_to_rgb,
        gradient: DEFAULT_GRADIENT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 0059's settings (`crates/golden/cases/grain.xmp`) on mire1.cr2.
    fn test_0059() -> Settings {
        Settings {
            scale: f32::from_le_bytes([0x6c, 0x26, 0xf0, 0x41]),
            strength: 100.0,
            midtones_bias: 100.0,
            offset: darktable_offset("mire1.cr2", 3908),
        }
    }

    #[test]
    fn darktables_hash_of_the_test_image() {
        // Checked against darktable's render of test 0059: the noise this offset gives
        // reproduces it (docs/ports/grain.md, Results).
        assert_eq!(hash_string("mire1.cr2"), 2_559_687_434);
        assert_eq!(darktable_offset("mire1.cr2", 3908), 758);
        // The hash runs from the name's end, so names that differ there differ widely.
        assert_ne!(hash_string("frame0001.cr2"), hash_string("frame0002.cr2"));
        // A tiny region still has a modulus of 1.
        assert_eq!(darktable_offset("mire1.cr2", 3), 0);
        // A byte above 127 counts as a negative char: "é" is 0xc3 0xa9, hashed from the
        // end, each sign-extended.
        let step = |hash: u32, byte: u32| hash.wrapping_mul(33) ^ byte;
        assert_eq!(hash_string("é"), step(step(5381, 0xffff_ffa9), 0xffff_ffc3));
    }

    #[test]
    fn the_zoom_is_darktables_fast_math_product() {
        let scale = test_0059().scale;
        assert_eq!(zoom(scale), (1.0 + f64::from(scale * 0.08)) / 800.0);
        // Not the correctly rounded quotient, which differs in the last bit here and
        // moves darktable's noise by 0.02 of a cell at offset 758.
        let quotient = (1.0 + f64::from(8.0 * scale / 100.0)) / 800.0;
        assert_ne!(zoom(scale), quotient);
    }

    #[test]
    fn offsets_are_reduced_by_the_period() {
        let offsets = noise_offsets(758, test_0059().scale);
        for (k, &offset) in offsets.iter().enumerate() {
            assert!((0.0..=PERIOD as f32).contains(&offset), "{k}: {offset}");
            let whole = 758.0 * FREQUENCY[k] / zoom(test_0059().scale);
            let turns = ((whole - f64::from(offset)) / PERIOD).round();
            assert!((whole - turns * PERIOD - f64::from(offset)).abs() < 1e-4);
        }
        assert_eq!(noise_offsets(0, 7.5), [0.0; 3]);
    }

    #[test]
    fn the_table_is_darktables_perm() {
        let table = permutation_table();
        assert_eq!(table.len(), 512 * 4);
        let word = |i: usize| u32::from_le_bytes(table[4 * i..4 * i + 4].try_into().unwrap());
        for i in 0..512 {
            assert_eq!(word(i), PERMUTATION[i & 255]);
        }
        // A permutation of 0 to 255.
        let mut sorted = PERMUTATION;
        sorted.sort_unstable();
        assert!(sorted.iter().enumerate().all(|(i, &v)| v == i as u32));
    }

    #[test]
    fn values_outside_darktables_ranges_are_not_ported() {
        let profile = WorkingProfile::linear_rec2020();
        assert!(prepare(&test_0059(), &profile).is_ok());
        assert!(prepare(&Settings::default(), &profile).is_ok());
        let (min_scale, max_scale) = scale_range();
        let bad = [
            Settings {
                scale: f32::NAN,
                ..test_0059()
            },
            Settings {
                strength: f32::INFINITY,
                ..test_0059()
            },
            Settings {
                scale: min_scale * 0.99,
                ..test_0059()
            },
            Settings {
                scale: max_scale * 1.01,
                ..test_0059()
            },
            Settings {
                strength: -1.0,
                ..test_0059()
            },
            Settings {
                midtones_bias: 100.5,
                ..test_0059()
            },
        ];
        for settings in bad {
            assert!(prepare(&settings, &profile).is_err(), "{settings:?}");
        }
    }

    #[test]
    fn prepare_passes_darktables_parameters() {
        let settings = test_0059();
        let profile = WorkingProfile::linear_rec2020();
        let prepared = prepare(&settings, &profile).unwrap();
        assert_eq!(
            (prepared.strength, prepared.scale, prepared.midtones_bias),
            (settings.strength, settings.scale, settings.midtones_bias)
        );
        assert_eq!(prepared.offset, noise_offsets(758, settings.scale));
        assert_eq!(prepared.rgb_to_xyz, profile.rgb_to_xyz_d50);
        assert_eq!(prepared.xyz_to_rgb, profile.xyz_d50_to_rgb);
        assert_eq!(prepared.gradient, DEFAULT_GRADIENT);
    }

    #[test]
    fn layout_matches_the_kernel() {
        let mut prepared = prepare(&test_0059(), &WorkingProfile::linear_rec2020()).unwrap();
        prepared.gradient = [
            Parameter::MidtonesBias,
            Parameter::Strength,
            Parameter::Scale,
        ];
        let params = prepared.params();
        assert_eq!(params.len(), PARAMS_SIZE);
        let word = |i: usize| u32::from_le_bytes(params[4 * i..4 * i + 4].try_into().unwrap());
        let float = |i: usize| f32::from_bits(word(i));
        assert_eq!(
            [float(0), float(1), float(2), float(3)],
            [100.0, prepared.scale, 100.0, 0.0]
        );
        assert_eq!(
            [float(4), float(5), float(6), float(7)],
            [
                prepared.offset[0],
                prepared.offset[1],
                prepared.offset[2],
                0.0
            ]
        );
        for (r, row) in prepared
            .rgb_to_xyz
            .iter()
            .chain(&prepared.xyz_to_rgb)
            .enumerate()
        {
            let at = 8 + 4 * r;
            assert_eq!(
                [float(at), float(at + 1), float(at + 2), float(at + 3)],
                [row[0], row[1], row[2], 0.0]
            );
        }
        assert_eq!([word(32), word(33), word(34), word(35)], [2, 0, 1, 3]);
    }

    #[test]
    fn a_slot_moves_its_parameter() {
        let base = prepare(&test_0059(), &WorkingProfile::linear_rec2020()).unwrap();
        for parameter in Parameter::ALL {
            let mut moved = base.clone();
            *moved.value_mut(parameter) += 0.25;
            let changed: Vec<Parameter> = Parameter::ALL
                .into_iter()
                .filter(|&p| moved.clone().value_mut(p) != base.clone().value_mut(p))
                .collect();
            assert_eq!(changed, [parameter]);
            // The offsets stay: the derivative is taken at a fixed offset.
            assert_eq!(moved.offset, base.offset);
        }
        let indices: Vec<u32> = Parameter::ALL.iter().map(|p| p.index()).collect();
        assert_eq!(indices, [0, 1, 2]);
    }
}
