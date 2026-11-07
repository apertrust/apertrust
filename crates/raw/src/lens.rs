// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/common/exif.cc)
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

//! The corrections a raw records for its lens, as darktable reads them with Exiv2
//! (`_check_lens_correction_data` in `src/common/exif.cc`), read over rawler's public
//! parsers (decision 25). Fujifilm's so far; Sony's, a DNG's opcodes and Olympus's come
//! with their parts of lens correction's port (`docs/ports/lens.md`, Slices).

use rawler::decoders::raf::{self, FujiIFD, RafMakernotes};
use rawler::formats::tiff::ifd::OffsetMode;
use rawler::formats::tiff::{IFD, Value};
use rawler::rawsource::RawSource;
use rawler::tags::ExifTag;

/// A lens's corrections as the raw records them: darktable's `dt_image_correction_data_t`
/// by its `exif_correction_type`, each value as darktable stores it after its checks.
/// Lens correction computes every table from them.
#[derive(Debug, Clone, PartialEq)]
pub enum LensCorrection {
    Sony(SonyCorrection),
    Fujifilm(FujifilmCorrection),
    Dng(DngCorrection),
    Olympus(OlympusCorrection),
}

/// A Sony raw's DistortionCorrParams, ChromaticAberrationCorrParams and
/// VignettingCorrParams, darktable's `sony`: knot i lies at radius (i + 0.5) / (nc − 1)
/// of the half-diagonal.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SonyCorrection {
    /// The knots, at most 16; every array is 0 past them.
    pub nc: u32,
    /// The input radius over the output radius is 1 + distortion · 2⁻¹⁴.
    pub distortion: [i16; 16],
    /// Red's radius factor, 1 + ca_r · 2⁻²¹.
    pub ca_r: [i16; 16],
    /// Blue's radius factor, 1 + ca_b · 2⁻²¹.
    pub ca_b: [i16; 16],
    /// The brightness, 2^(0.5 − 2^(vignetting · 2⁻¹³ − 1)).
    pub vignetting: [i16; 16],
}

/// A RAF's GeometricDistortionParams, ChromaticAberrationParams and VignettingParams,
/// from its FujiIFD, darktable's `fuji`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FujifilmCorrection {
    /// The knots: 9 in the X-Trans IV and V layout, 11 in the X-Trans I to III one; every
    /// array is 0 past them.
    pub nc: u32,
    /// The knots' scale: 1.25 in the camera's 1.25× crop modes (CropMode 2 or 4), else 1.
    pub cropf: f32,
    /// The radii, normalised to the half-diagonal before `cropf`.
    pub knots: [f32; 11],
    /// The input radius over the output radius is 1 + distortion / 100.
    pub distortion: [f32; 11],
    /// Red's radius factor, 1 + ca_r.
    pub ca_r: [f32; 11],
    /// Blue's radius factor, 1 + ca_b.
    pub ca_b: [f32; 11],
    /// The brightness, in percent.
    pub vignetting: [f32; 11],
}

/// A DNG's OpcodeList3, its last WarpRectilinear and FixVignetteRadial, darktable's `dng`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DngCorrection {
    /// WarpRectilinear's planes, 1 or 3.
    pub planes: u32,
    /// Per plane, kr0 to kr3, kt0 and kt1: the input radius over the output radius is
    /// kr0 + kr1 r² + kr2 r⁴ + kr3 r⁶, r normalised to the half-diagonal.
    pub cwarp: [[f32; 6]; 3],
    /// WarpRectilinear's centre, which darktable does not read.
    pub centre_warp: [f32; 2],
    /// FixVignetteRadial's k0 to k4: the gain is 1 + k0 r² + k1 r⁴ + k2 r⁶ + k3 r⁸ + k4 r¹⁰.
    pub cvig: [f32; 5],
    /// FixVignetteRadial's centre, which darktable does not read.
    pub centre_vig: [f32; 2],
    pub has_warp: bool,
    pub has_vignette: bool,
}

/// An Olympus or OM System raw's maker note tags 0x150a and 0x150c, of its ImageProcessing
/// directory, darktable's `olympus`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OlympusCorrection {
    /// Whether `dist` corrects anything: one of its first three values is not 0.
    pub has_dist: bool,
    /// dk2, dk4, dk6 and drs, the corner's radius.
    pub dist: [f32; 4],
    /// Whether `ca` corrects anything: one of its values is not 0.
    pub has_ca: bool,
    /// car0, car2, car4, cab0, cab2 and cab4.
    pub ca: [f32; 6],
}

impl LensCorrection {
    /// darktable's name for the type.
    pub fn name(&self) -> &'static str {
        match self {
            LensCorrection::Sony(_) => "Sony",
            LensCorrection::Fujifilm(_) => "Fujifilm",
            LensCorrection::Dng(_) => "DNG",
            LensCorrection::Olympus(_) => "Olympus",
        }
    }
}

/// The corrections the raw records, as darktable finds them; `None` where it finds none.
/// darktable checks a DNG's opcodes, then Sony's, Fujifilm's and Olympus's tags, each
/// found replacing what came before, and a Fujifilm knot mismatch leaving none.
pub(crate) fn read(source: &RawSource) -> Option<LensCorrection> {
    fujifilm_tags(source).map(LensCorrection::Fujifilm)
}

/// A RAF header's big-endian pointer to its embedded JPEG, whose EXIF holds the maker note.
const JPEG_POINTER: u64 = 84;
/// A RAF header's big-endian pointer to its second TIFF structure, holding the FujiIFD.
const TIFF_POINTER: u64 = 100;
/// The JPEG's SOI and APP1 markers, APP1's length and "Exif\0\0", before its TIFF header.
const EXIF_HEADER: u32 = 12;

/// A RAF's corrections: the FujiIFD's three tags, where rawler's RAF decoder finds them,
/// and CropMode from the maker note.
fn fujifilm_tags(source: &RawSource) -> Option<FujifilmCorrection> {
    if !raf::is_raf(source) {
        return None;
    }
    let root = IFD::new_root_with_correction(
        &mut source.reader(),
        0,
        pointer(source, TIFF_POINTER)?,
        0,
        10,
        &[FujiIFD::FujiIFD.into()],
    )
    .ok()?;
    // rawler files the FujiIFD under its entry's type, not its tag, so the tags are looked
    // up below the root wherever it sits, as Exiv2 finds them by key.
    let values = |tag: FujiIFD| exiv2_floats(&root.get_entry_recursive(tag)?.value);
    fujifilm(
        &values(FujiIFD::GeometricDistortionParams)?,
        &values(FujiIFD::ChromaticAberrationParams)?,
        &values(FujiIFD::VignettingParams)?,
        || crop_mode(source),
    )
}

/// The maker note's CropMode, as Exiv2's `toLong` reads its first value.
fn crop_mode(source: &RawSource) -> Option<i64> {
    let mut reader = source.reader();
    let exif = IFD::new_root(
        &mut reader,
        pointer(source, JPEG_POINTER)?.checked_add(EXIF_HEADER)?,
    )
    .ok()?;
    let makernote = exif
        .find_first_ifd_with_tag(ExifTag::MakerNotes)?
        .parse_makernote(&mut reader, OffsetMode::Absolute, &[])
        .ok()??;
    makernote
        .get_entry(RafMakernotes::CropMode)?
        .value
        .get_i64(0)
        .ok()?
}

fn pointer(source: &RawSource, at: u64) -> Option<u32> {
    let bytes = source.subview(at, 4).ok()?;
    Some(u32::from_be_bytes(bytes.try_into().ok()?))
}

/// A tag's values as Exiv2's `toFloat` reads them, where darktable reads them: a
/// rational's numerator over its denominator in f32, 0 for a zero denominator, which
/// rawler's own conversion makes infinite; `None` for a tag Exiv2 would take as missing,
/// one without values, or one of values that are not numbers.
fn exiv2_floats(value: &Value) -> Option<Vec<f32>> {
    let ratio = |n: f32, d: f32| if d == 0.0 { 0.0 } else { n / d };
    let values: Option<Vec<f32>> = match value {
        Value::SRational(values) => Some(
            values
                .iter()
                .map(|r| ratio(r.n as f32, r.d as f32))
                .collect(),
        ),
        Value::Rational(values) => Some(
            values
                .iter()
                .map(|r| ratio(r.n as f32, r.d as f32))
                .collect(),
        ),
        _ => (0..value.count())
            .map(|i| value.get_f32(i).ok().flatten())
            .collect(),
    };
    values.filter(|values| !values.is_empty())
}

/// darktable's reading of Fujifilm's three tags, in one of two layouts: each tag holds a
/// value darktable skips, then the knots, then its values, the chromatic aberration tag
/// red's then blue's. The knots must agree between the tags.
fn fujifilm(
    d: &[f32],
    c: &[f32],
    v: &[f32],
    crop_mode: impl FnOnce() -> Option<i64>,
) -> Option<FujifilmCorrection> {
    let mut fuji = FujifilmCorrection::default();
    match (d.len(), c.len(), v.len()) {
        // X-Trans IV and V.
        (19, 29, 19) => {
            fuji.nc = 9;
            for i in 0..9 {
                let kd = d[i + 1];
                if kd != c[i + 1] || kd != v[i + 1] {
                    return None;
                }
                fuji.knots[i] = kd;
                fuji.distortion[i] = d[i + 10];
                fuji.ca_r[i] = c[i + 10];
                fuji.ca_b[i] = c[i + 19];
                fuji.vignetting[i] = v[i + 10];
            }
        }
        // X-Trans I to III: the chromatic aberration tag leaves out the first knot, 0.
        (23, 31, 23) => {
            fuji.nc = 11;
            for i in 0..11 {
                let kd = d[i + 1];
                let kc = if i == 0 { 0.0 } else { c[i] };
                if kd != kc || kd != v[i + 1] {
                    return None;
                }
                fuji.knots[i] = kd;
                fuji.distortion[i] = d[i + 12];
                if i > 0 {
                    fuji.ca_r[i] = c[i + 10];
                    fuji.ca_b[i] = c[i + 20];
                }
                fuji.vignetting[i] = v[i + 12];
            }
        }
        // The GFX 50S's 17, 25 and 17 among them, a layout darktable does not read.
        _ => return None,
    }
    // The 1.25× crop modes.
    fuji.cropf = if matches!(crop_mode(), Some(2 | 4)) {
        1.25
    } else {
        1.0
    };
    Some(fuji)
}

#[cfg(test)]
mod tests {
    use rawler::formats::tiff::SRational;

    use super::*;

    /// The X-T3's tags in `xtransIV.raf`, darktable's tests 0145 and 0146, as Exiv2 shows
    /// them: the knots in 1420ths, distortion and chromatic aberration in 65536ths,
    /// vignetting in 2048ths.
    fn x_t3() -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let knots = [502, 710, 870, 1004, 1123, 1230, 1328, 1420, 1506].map(|k| k as f32 / 1420.0);
        let over = |d: f32| move |n: i32| n as f32 / d;
        let distortion = [
            -90947, -181789, -267578, -345684, -414736, -469789, -506315, -518421, -530526,
        ]
        .map(over(65536.0));
        let ca = [2, 2, 2, 2, 2, 0, 0, -6, -6, 4, 4, 6, 8, 8, 12, 18, 24, 24].map(over(65536.0));
        let vignetting = [
            200014, 195174, 189615, 183157, 176751, 167906, 151037, 106023, 81680,
        ]
        .map(over(2048.0));
        let first = 3749.0 / 9.0;
        let tag = |values: &[f32], last: &[f32]| [&[first][..], &knots, values, last].concat();
        (
            tag(&distortion, &[]),
            tag(&ca, &[first]),
            tag(&vignetting, &[]),
        )
    }

    #[test]
    fn reads_the_x_trans_iv_layout_as_darktable() {
        let (d, c, v) = x_t3();
        let fuji = fujifilm(&d, &c, &v, || Some(0)).unwrap();
        assert_eq!(fuji.nc, 9);
        assert_eq!(fuji.cropf, 1.0);
        assert_eq!(fuji.knots[..9], d[1..10]);
        assert_eq!(fuji.knots[8], 1506.0 / 1420.0);
        assert_eq!(fuji.distortion[..9], d[10..19]);
        assert_eq!(fuji.ca_r[..9], c[10..19]);
        assert_eq!(fuji.ca_b[..9], c[19..28]);
        assert_eq!(fuji.vignetting[..9], v[10..19]);
        // Past the knots, nothing.
        assert_eq!(fuji.knots[9..], [0.0; 2]);
        assert_eq!(fuji.vignetting[9..], [0.0; 2]);
    }

    #[test]
    fn reads_the_x_trans_i_to_iii_layout_as_darktable() {
        // Eleven knots, the first 0, which the chromatic aberration tag leaves out.
        let knots: Vec<f32> = (0..11).map(|i| i as f32 / 10.0).collect();
        let values = |base: f32, n: usize| (0..n).map(move |i| base + i as f32);
        let d = [vec![99.0], knots.clone(), values(100.0, 11).collect()].concat();
        let c = [
            vec![99.0],
            knots[1..].to_vec(),
            values(200.0, 10).collect(),
            values(300.0, 10).collect(),
        ]
        .concat();
        let v = [vec![99.0], knots.clone(), values(400.0, 11).collect()].concat();
        assert_eq!((d.len(), c.len(), v.len()), (23, 31, 23));
        let fuji = fujifilm(&d, &c, &v, || None).unwrap();
        assert_eq!(fuji.nc, 11);
        assert_eq!(fuji.knots.to_vec(), knots);
        assert_eq!(
            fuji.distortion.to_vec(),
            values(100.0, 11).collect::<Vec<_>>()
        );
        assert_eq!(fuji.ca_r[0], 0.0);
        assert_eq!(fuji.ca_b[0], 0.0);
        assert_eq!(
            fuji.ca_r[1..].to_vec(),
            values(200.0, 10).collect::<Vec<_>>()
        );
        assert_eq!(
            fuji.ca_b[1..].to_vec(),
            values(300.0, 10).collect::<Vec<_>>()
        );
        assert_eq!(
            fuji.vignetting.to_vec(),
            values(400.0, 11).collect::<Vec<_>>()
        );

        // A first knot that is not 0 does not match the chromatic aberration's.
        let mut moved = d.clone();
        moved[1] = 0.01;
        let mut moved_v = v.clone();
        moved_v[1] = 0.01;
        assert_eq!(fujifilm(&moved, &c, &moved_v, || None), None);
    }

    #[test]
    fn knots_that_disagree_give_none() {
        let (d, c, v) = x_t3();
        for tag in 0..3 {
            let mut tags = [d.clone(), c.clone(), v.clone()];
            tags[tag][5] += 1e-6;
            assert_eq!(
                fujifilm(&tags[0], &tags[1], &tags[2], || None),
                None,
                "{tag}"
            );
        }
    }

    #[test]
    fn other_layouts_give_none() {
        let (d, c, v) = x_t3();
        // The GFX 50S's counts: eight knots.
        assert_eq!(fujifilm(&d[..17], &c[..25], &v[..17], || None), None);
        assert_eq!(fujifilm(&d, &c[..28], &v, || None), None);
        assert_eq!(fujifilm(&[], &[], &[], || None), None);
    }

    #[test]
    fn the_crop_modes_scale_the_knots_by_1_25() {
        let (d, c, v) = x_t3();
        let cropf = |mode| fujifilm(&d, &c, &v, || mode).unwrap().cropf;
        assert_eq!(cropf(Some(2)), 1.25);
        assert_eq!(cropf(Some(4)), 1.25);
        for mode in [None, Some(0), Some(1), Some(3), Some(5)] {
            assert_eq!(cropf(mode), 1.0, "{mode:?}");
        }
    }

    #[test]
    fn rationals_read_as_exiv2_reads_them() {
        let value = Value::SRational(vec![
            SRational::new(-90947, 65536),
            SRational::new(1506, 1420),
            SRational::new(5, 0),
        ]);
        assert_eq!(
            exiv2_floats(&value),
            Some(vec![-90947.0 / 65536.0, 1506.0 / 1420.0, 0.0])
        );
        assert_eq!(exiv2_floats(&Value::SRational(Vec::new())), None);
        assert_eq!(exiv2_floats(&Value::Ascii(Default::default())), None);
    }

    #[test]
    fn a_raw_that_is_not_a_raf_has_none() {
        assert_eq!(
            read(&RawSource::new_from_slice(b"not a raw file at all")),
            None
        );
        // A RAF's signature with nothing after it.
        assert_eq!(
            read(&RawSource::new_from_slice(b"FUJIFILMCCD-RAW 0201")),
            None
        );
    }
}
