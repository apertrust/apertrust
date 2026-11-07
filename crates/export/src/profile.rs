// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/common/colorspaces.c)
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

//! darktable's sRGB output profile, which every export embeds (decision 56): the profile
//! `_colorspaces_create_srgb_profile` builds with lcms2 for darktable's "sRGB (web-safe)"
//! output (`src/common/colorspaces.c`), written as lcms2 saves it, an ICC version 2.4
//! profile with its tags in lcms2's order, each padded to four bytes. Its white is
//! darktable's D65 and its primaries sRGB's, Bradford-adapted to D50; its curve is the
//! parametric {2.4, 1/1.055, 0.055/1.055, 1/12.92, 0.04045} as lcms2 keeps it in a version
//! 2 profile, 4096 16-bit samples in one tag the three channels share; its description
//! and model are "sRGB", its copyright "Public Domain", and its device maker "Apertrust"
//! where darktable's says "darktable"; its header names no CMM, platform or creator where
//! darktable's names lcms2 and the platform darktable was built on. `tests/profile.rs`
//! holds it to darktable-cli's byte for byte, but for those, the maker and the creation
//! time.
//!
//! Built once a process, as darktable builds its profiles once (`dt_colorspaces_init`), so
//! every export of one run carries the same creation time.
//!
//! Written here, not by moxcms, the colour management library (decision 5): its encoder
//! writes only version 4, its texts multi-localised, with no chromaticity tag and its tags
//! not padded (0.8.1 and 0.9.1).

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use apertrust_color::{D50_XYZ, RgbSpace, SRGB_PRIMARIES};

/// The profile's description, which names it in a PNG's iCCP chunk, as darktable names it.
pub const DESCRIPTION: &str = "sRGB";

/// The profile's device maker, where darktable's says "darktable" (decision 56).
const MAKER: &str = "Apertrust";

/// The size of an ICC profile's header.
const HEADER: usize = 128;

/// The profile, as an export embeds it: built the first time it is asked for, and the
/// same bytes for the rest of the process.
pub fn srgb() -> &'static [u8] {
    static PROFILE: OnceLock<Vec<u8>> = OnceLock::new();
    PROFILE.get_or_init(|| build(SystemTime::now()))
}

/// The profile, stamped with `created`.
fn build(created: SystemTime) -> Vec<u8> {
    let space = RgbSpace::SRGB;
    let colorants = space.rgb_to_xyz_d50();
    let colorant = |c: usize| xyz([colorants[0][c], colorants[1][c], colorants[2][c]]);

    // Each tag's data once, in the order lcms2 writes it: `cmsCreateRGBProfile`'s tags,
    // the colorants red, blue and green, then the copyright, description, model and maker
    // darktable's `_create_lcms_profile` sets, its copyright and description in the
    // places `cmsCreateRGBProfile` gave them.
    let data = [
        text_description(DESCRIPTION),
        text("Public Domain"),
        xyz(D50_XYZ),
        s15_fixed16_array(space.adaptation_to_d50().as_flattened()),
        colorant(0),
        colorant(2),
        colorant(1),
        curve(&srgb_curve()),
        chromaticity(SRGB_PRIMARIES),
        text_description("sRGB"),
        text_description(MAKER),
    ];
    // Each tag's signature and its data; lcms2 links the green and blue curves to the red.
    let tags: [(&[u8; 4], usize); 13] = [
        (b"desc", 0),
        (b"cprt", 1),
        (b"wtpt", 2),
        (b"chad", 3),
        (b"rXYZ", 4),
        (b"bXYZ", 5),
        (b"gXYZ", 6),
        (b"rTRC", 7),
        (b"gTRC", 7),
        (b"bTRC", 7),
        (b"chrm", 8),
        (b"dmdd", 9),
        (b"dmnd", 10),
    ];

    let mut offsets = Vec::with_capacity(data.len());
    let mut size = HEADER + 4 + 12 * tags.len();
    for tag in &data {
        offsets.push(size);
        size += tag.len().next_multiple_of(4);
    }

    let mut profile = Vec::with_capacity(size);
    header(&mut profile, size, date_time(created));
    put_u32(&mut profile, tags.len());
    for (signature, index) in tags {
        profile.extend(signature);
        put_u32(&mut profile, offsets[index]);
        put_u32(&mut profile, data[index].len());
    }
    for tag in &data {
        profile.extend(tag);
        profile.resize(profile.len().next_multiple_of(4), 0);
    }
    profile
}

/// lcms2's header for a profile `cmsCreateRGBProfile` makes and darktable's
/// `_create_lcms_profile` sets to version 2.4 and marks embedded: a display profile from
/// RGB to XYZ, perceptual, its illuminant D50. Its preferred CMM, its platform and its
/// creator are none, zero, where darktable's name lcms2 as the CMM and the creator and,
/// as the platform, the one lcms2 was built for (`APPL` on macOS and Linux, `MSFT` on
/// Windows): Apertrust uses no lcms, and the building machine's platform says nothing of
/// the reader's (decision 56).
fn header(profile: &mut Vec<u8>, size: usize, created: [u16; 6]) {
    put_u32(profile, size);
    // No preferred CMM.
    profile.extend([0; 4]);
    profile.extend([2, 0x40, 0, 0]);
    profile.extend(b"mntrRGB XYZ ");
    for part in created {
        profile.extend(part.to_be_bytes());
    }
    profile.extend(b"acsp");
    // No platform.
    profile.extend([0; 4]);
    // Flags: embedded in a file.
    profile.extend([0, 0, 0, 1]);
    // No device maker or model, no attributes; the perceptual intent.
    profile.resize(68, 0);
    for v in D50_XYZ {
        profile.extend(s15_fixed16(v).to_be_bytes());
    }
    // No creator, no profile ID, then the reserved bytes.
    profile.resize(HEADER, 0);
}

/// `textDescriptionType`, as lcms2's `Type_Text_Description_Write` writes it: the text
/// in ASCII and in Unicode with their nulls, no ScriptCode, padded to four bytes within
/// the tag.
fn text_description(text: &str) -> Vec<u8> {
    let count = text.len() + 1;
    let mut tag = signature(b"desc");
    put_u32(&mut tag, count);
    tag.extend(text.as_bytes());
    tag.push(0);
    // Unicode's language code, none, and its count.
    put_u32(&mut tag, 0);
    put_u32(&mut tag, count);
    for unit in text.encode_utf16().chain([0]) {
        tag.extend(unit.to_be_bytes());
    }
    // ScriptCode's code and count, and its 67 bytes.
    tag.resize(tag.len() + 2 + 1 + 67, 0);
    tag.resize(tag.len().next_multiple_of(4), 0);
    tag
}

/// `textType`: the text in ASCII with its null.
fn text(text: &str) -> Vec<u8> {
    let mut tag = signature(b"text");
    tag.extend(text.as_bytes());
    tag.push(0);
    tag
}

/// `XYZType`: one XYZ number.
fn xyz(xyz: [f64; 3]) -> Vec<u8> {
    let mut tag = signature(b"XYZ ");
    for v in xyz {
        tag.extend(s15_fixed16(v).to_be_bytes());
    }
    tag
}

/// `s15Fixed16ArrayType`.
fn s15_fixed16_array(numbers: &[f64]) -> Vec<u8> {
    let mut tag = signature(b"sf32");
    for &v in numbers {
        tag.extend(s15_fixed16(v).to_be_bytes());
    }
    tag
}

/// `curveType`: 16-bit samples from 0 to 1.
fn curve(samples: &[u16]) -> Vec<u8> {
    let mut tag = signature(b"curv");
    put_u32(&mut tag, samples.len());
    for sample in samples {
        tag.extend(sample.to_be_bytes());
    }
    tag
}

/// `chromaticityType` with no colorant type given: each primary's x and y, as lcms2's
/// `Type_Chromaticity_Write` writes them.
fn chromaticity(primaries: [[f64; 2]; 3]) -> Vec<u8> {
    let mut tag = signature(b"chrm");
    tag.extend(3u16.to_be_bytes());
    tag.extend(0u16.to_be_bytes());
    for v in primaries.as_flattened() {
        tag.extend(s15_fixed16(*v).to_be_bytes());
    }
    tag
}

/// A tag's type signature and its four reserved bytes.
fn signature(kind: &[u8; 4]) -> Vec<u8> {
    let mut tag = kind.to_vec();
    tag.extend([0; 4]);
    tag
}

/// A count, a size or an offset, all far under 4 GiB in this profile.
fn put_u32(into: &mut Vec<u8>, value: usize) {
    into.extend((value as u32).to_be_bytes());
}

/// lcms2's `_cmsDoubleTo15Fixed16`.
fn s15_fixed16(v: f64) -> i32 {
    (v * 65536.0 + 0.5).floor() as i32
}

/// darktable's sRGB curve as lcms2 samples it: `cmsBuildParametricToneCurve`'s type 4,
/// `(a·x + b)^γ` from `d` and `c·x` below it, evaluated in f64 at 4096 points from 0 to 1
/// and rounded to 16 bits (`cmsBuildSegmentedToneCurve`, `_cmsQuickSaturateWord`).
fn srgb_curve() -> Vec<u16> {
    let (gamma, a, b, c, d) = (2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045);
    (0..4096u32)
        .map(|i| {
            let x = f64::from(i) / 4095.0;
            let y = if x >= d {
                let e = a * x + b;
                if e > 0.0 { e.powf(gamma) } else { 0.0 }
            } else {
                x * c
            };
            (y * 65535.0 + 0.5).floor().clamp(0.0, 65535.0) as u16
        })
        .collect()
}

/// `time` in UTC as the header's date and time, year, month, day, hours, minutes and
/// seconds, as lcms2 stamps a profile it creates (`gmtime`).
fn date_time(time: SystemTime) -> [u16; 6] {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, second) = (seconds / 86_400, seconds % 86_400);
    // Howard Hinnant's `civil_from_days`, for days from 1970 on.
    let z = days + 719_468;
    let (era, day_of_era) = (z / 146_097, z % 146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    [
        year,
        month,
        day,
        second / 3600,
        second % 3600 / 60,
        second % 60,
    ]
    .map(|part| u16::try_from(part).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn dates_are_utc_as_lcms2_stamps_them() {
        let at = |seconds| date_time(UNIX_EPOCH + Duration::from_secs(seconds));
        assert_eq!(at(0), [1970, 1, 1, 0, 0, 0]);
        assert_eq!(at(951_868_799), [2000, 2, 29, 23, 59, 59]);
        // darktable-cli's profile in `tests/darktable/srgb.icc` was made at 1793881447.
        let darktable = include_bytes!("../tests/darktable/srgb.icc");
        let (stamped, _) = darktable[24..36].as_chunks::<2>();
        let stamped: Vec<u16> = stamped.iter().map(|&b| u16::from_be_bytes(b)).collect();
        assert_eq!(at(1_793_881_447).to_vec(), stamped);
    }
}
