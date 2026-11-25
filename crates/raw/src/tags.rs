// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
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

//! The EXIF tags rawler's metadata leaves out, read through its public TIFF parser, as
//! `lens.rs` reads Fujifilm's FujiIFD: IFD0's Make, Model, Software, ImageDescription and
//! resolution, and the Exif IFD's DigitalZoomRatio, Contrast, Saturation, Sharpness,
//! ImageUniqueID, FocalLengthIn35mmFilm and what the crop factor is worked out from
//! ([`crate::cropfactors`]).
//!
//! The IFDs are found where Exiv2 finds them: at the file's TIFF header for the formats
//! built on TIFF (DNG, CR2, NEF, ARW, ORF, RW2, PEF, SRW, 3FR and the rest), with IFD0's
//! tags a Panasonic raw lacks from the JPEG it embeds, in a CR3's CMT1 and CMT2 boxes, and in
//! the EXIF of a RAF's embedded JPEG. Any other file, or one
//! whose structure does not parse, has none of them ([`read`] gives `None`).

use std::io::{Read, Seek};

use rawler::formats::tiff::reader::TiffReader;
use rawler::formats::tiff::{GenericTiffReader, IFD, Value};
use rawler::rawsource::RawSource;
use rawler::tags::ExifTag;

use crate::Resolution;
use crate::cropfactors::{Dimensions, FocalPlane};
use crate::highlight::{cr3_description, olympus_directory};
use crate::lens::{exiv2_floats, raf_exif};

/// IFD0's tags.
const MAKE: u16 = 0x010f;
const MODEL: u16 = 0x0110;
const IMAGE_DESCRIPTION: u16 = 0x010e;
const SOFTWARE: u16 = 0x0131;
const X_RESOLUTION: u16 = 0x011a;
const Y_RESOLUTION: u16 = 0x011b;
const RESOLUTION_UNIT: u16 = 0x0128;
const IMAGE_WIDTH: u16 = 0x0100;
const IMAGE_LENGTH: u16 = 0x0101;
const NEW_SUBFILE_TYPE: u16 = 0x00fe;
const SUB_IFDS: u16 = 0x014a;
/// A Panasonic raw's embedded JPEG, and where its EXIF's TIFF header sits in it: after its
/// SOI and APP1 markers, APP1's length and "Exif\0\0".
const JPG_FROM_RAW: u16 = 0x002e;
const JPEG_EXIF: u32 = 12;
/// The Exif IFD's.
const PIXEL_X_DIMENSION: u16 = 0xa002;
const PIXEL_Y_DIMENSION: u16 = 0xa003;
const FOCAL_PLANE_X_RESOLUTION: u16 = 0xa20e;
const FOCAL_PLANE_Y_RESOLUTION: u16 = 0xa20f;
const FOCAL_PLANE_RESOLUTION_UNIT: u16 = 0xa210;
const DIGITAL_ZOOM_RATIO: u16 = 0xa404;
const FOCAL_LENGTH_IN_35MM_FILM: u16 = 0xa405;
const CONTRAST: u16 = 0xa408;
const SATURATION: u16 = 0xa409;
const SHARPNESS: u16 = 0xa40a;
const IMAGE_UNIQUE_ID: u16 = 0xa420;
/// Olympus's Equipment directory in its maker note, and its FocalPlaneDiagonal.
const OLYMPUS_EQUIPMENT: u16 = 0x2010;
const FOCAL_PLANE_DIAGONAL: u16 = 0x0205;

/// The [`crate::Exif`] fields read here, by name, which [`crate::Exif::unread`] lists for a
/// file whose IFDs cannot be reached.
pub(crate) const FIELDS: &[&str] = &[
    "make_written",
    "model_written",
    "software",
    "image_description",
    "resolution",
    "digital_zoom",
    "contrast",
    "saturation",
    "sharpness",
    "unique_image_id",
    "focal_length_35mm",
    "crop_factor",
];

/// The tags rawler's metadata leaves out, each `None` where the file has no such tag, a
/// text trimmed and none where it is empty.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Tags {
    pub make: Option<String>,
    pub model: Option<String>,
    pub software: Option<String>,
    pub image_description: Option<String>,
    pub resolution: Option<Resolution>,
    pub digital_zoom: Option<f32>,
    pub contrast: Option<u16>,
    pub saturation: Option<u16>,
    pub sharpness: Option<u16>,
    pub unique_image_id: Option<String>,
    /// FocalLengthIn35mmFilm, in mm; none for 0, which EXIF reads as unknown.
    pub focal_length_35mm: Option<f32>,
    pub focal_plane: Option<FocalPlane>,
    pub dimensions: Dimensions,
    pub olympus_diagonal: Option<(u32, u32)>,
}

/// The tags of the raw `source`, where its IFDs can be reached; `None` otherwise.
pub(crate) fn read(source: &RawSource) -> Option<Tags> {
    if let Some(description) = cr3_description(source) {
        let root = description.cmt1.tiff.root_ifd();
        return Some(of(root, Some(description.cmt2.tiff.root_ifd())));
    }
    if let Some(root) = raf_exif(source) {
        return Some(of(&root, root.get_sub_ifd(ExifTag::ExifOffset)));
    }
    let mut reader = source.reader();
    let tiff = GenericTiffReader::new(&mut reader, 0, 0, None, &[]).ok()?;
    let root = tiff.root_ifd();
    let exif = root.get_sub_ifd(ExifTag::ExifOffset);
    let mut tags = of(root, exif);
    tags.olympus_diagonal = exif.and_then(|exif| olympus_diagonal(&mut reader, exif));
    // A Panasonic raw's IFD0 holds few of them: Exiv2 takes the rest of IFD0's from the
    // EXIF of the JPEG it embeds.
    if let Some(jpeg) = panasonic_jpeg(source, root) {
        tags = tags.or(of(&jpeg, None));
    }
    Some(tags)
}

/// The IFD0 of the JPEG a Panasonic raw embeds as its JpgFromRaw (0x002E), where its APP1
/// comes first, as Panasonic writes it.
fn panasonic_jpeg(source: &RawSource, root: &IFD) -> Option<IFD> {
    let at = u32::try_from(root.get_entry(JPG_FROM_RAW)?.offset()?).ok()?;
    let start = source.subview(u64::from(at), u64::from(JPEG_EXIF)).ok()?;
    if !starts_with_exif(start) {
        return None;
    }
    IFD::new_root(&mut source.reader(), at.checked_add(JPEG_EXIF)?).ok()
}

/// Whether `start`, a JPEG's first bytes, is SOI then APP1, its two-byte length, then
/// "Exif\0\0", so that its TIFF header follows.
fn starts_with_exif(start: &[u8]) -> bool {
    start.len() == JPEG_EXIF as usize
        && start[..4] == [0xff, 0xd8, 0xff, 0xe1]
        && &start[6..] == b"Exif\0\0"
}

impl Tags {
    /// These tags, each of IFD0's missing taken from `other`'s.
    fn or(self, other: Tags) -> Tags {
        Tags {
            make: self.make.or(other.make),
            model: self.model.or(other.model),
            software: self.software.or(other.software),
            image_description: self.image_description.or(other.image_description),
            resolution: self.resolution.or(other.resolution),
            ..self
        }
    }
}

/// The tags `root`, IFD0, and `exif`, its Exif IFD, hold.
pub(crate) fn of(root: &IFD, exif: Option<&IFD>) -> Tags {
    let exif_tag = |tag: u16| exif.and_then(|exif| exif.get_entry(tag)).map(|e| &e.value);
    let root_tag = |tag: u16| root.get_entry(tag).map(|e| &e.value);
    let whole = |value: Option<&Value>| value.and_then(|v| v.get_u32(0).ok().flatten());
    let resolution = match (
        number(root_tag(X_RESOLUTION)),
        number(root_tag(Y_RESOLUTION)),
    ) {
        (Some(x), Some(y)) => Some(Resolution {
            x,
            y,
            unit: short(root_tag(RESOLUTION_UNIT)).unwrap_or(2),
        }),
        _ => None,
    };
    let focal_plane = number(exif_tag(FOCAL_PLANE_X_RESOLUTION)).map(|x| FocalPlane {
        x,
        y: number(exif_tag(FOCAL_PLANE_Y_RESOLUTION)).unwrap_or(0.0),
        unit: whole(exif_tag(FOCAL_PLANE_RESOLUTION_UNIT)).unwrap_or(1),
    });
    let pair = |ifd: &IFD, width: u16, height: u16| {
        let get = |tag| whole(ifd.get_entry(tag).map(|e| &e.value)).unwrap_or(0);
        [get(width), get(height)]
    };
    // Exiv2's SubImage1: the first of IFD0's SubIFDs, a DNG's raw where it is the primary
    // image.
    let sub_image = (root.sub_ifds().get(&SUB_IFDS))
        .and_then(|subs| subs.first())
        .filter(|sub| whole(sub.get_entry(NEW_SUBFILE_TYPE).map(|e| &e.value)) == Some(0))
        .map(|sub| pair(sub, IMAGE_WIDTH, IMAGE_LENGTH));
    Tags {
        make: text(root_tag(MAKE)),
        model: text(root_tag(MODEL)),
        software: text(root_tag(SOFTWARE)),
        image_description: text(root_tag(IMAGE_DESCRIPTION)),
        resolution,
        digital_zoom: number(exif_tag(DIGITAL_ZOOM_RATIO)),
        contrast: short(exif_tag(CONTRAST)),
        saturation: short(exif_tag(SATURATION)),
        sharpness: short(exif_tag(SHARPNESS)),
        unique_image_id: text(exif_tag(IMAGE_UNIQUE_ID)),
        focal_length_35mm: number(exif_tag(FOCAL_LENGTH_IN_35MM_FILM)).filter(|mm| *mm > 0.0),
        focal_plane,
        dimensions: Dimensions {
            exif: exif.map_or([0, 0], |exif| {
                pair(exif, PIXEL_X_DIMENSION, PIXEL_Y_DIMENSION)
            }),
            sub_image,
            image: pair(root, IMAGE_WIDTH, IMAGE_LENGTH),
        },
        olympus_diagonal: None,
    }
}

/// A text tag's first string, trimmed; none where it is no text or empty.
fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Ascii(ascii) => {
            let text = ascii.strings().first()?.trim();
            (!text.is_empty()).then(|| text.to_owned())
        }
        _ => None,
    }
}

/// A tag's first value as Exiv2's `toFloat` reads it, a rational with a zero denominator 0.
fn number(value: Option<&Value>) -> Option<f32> {
    exiv2_floats(value?)?.first().copied()
}

/// A tag's first value as a whole number of 16 bits.
fn short(value: Option<&Value>) -> Option<u16> {
    value?.get_u16(0).ok().flatten()
}

/// Olympus's FocalPlaneDiagonal from the maker note the Exif IFD `exif` holds, as a
/// rational, where the note is an Olympus or OM System one.
fn olympus_diagonal<R: Read + Seek>(reader: &mut R, exif: &IFD) -> Option<(u32, u32)> {
    let Value::Undefined(data) = &exif.get_entry(ExifTag::MakerNotes)?.value else {
        return None;
    };
    if !(data.starts_with(b"OLYMPUS\0") || data.starts_with(b"OM SYSTEM\0")) {
        return None;
    }
    let equipment = olympus_directory(reader, exif, data, OLYMPUS_EQUIPMENT)?;
    match &equipment.get_entry(FOCAL_PLANE_DIAGONAL)?.value {
        Value::Rational(values) => values.first().map(|r| (r.n, r.d)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rawler::formats::tiff::{Entry, Rational, TiffAscii};

    use super::*;

    /// An IFD holding `entries`, each a tag and its value.
    pub(crate) fn ifd(entries: Vec<(u16, Value)>) -> IFD {
        let entries: BTreeMap<u16, Entry> = entries
            .into_iter()
            .map(|(tag, value)| {
                let entry = Entry {
                    tag,
                    value,
                    embedded: None,
                };
                (tag, entry)
            })
            .collect();
        IFD {
            entries,
            ..IFD::default()
        }
    }

    fn ascii(text: &str) -> Value {
        Value::Ascii(TiffAscii::new(text))
    }

    fn rational(n: u32, d: u32) -> Value {
        Value::Rational(vec![Rational::new(n, d)])
    }

    #[test]
    fn reads_ifd0_s_texts_trimmed_and_its_resolution() {
        let root = ifd(vec![
            (MAKE, ascii("OLYMPUS CORPORATION    ")),
            (MODEL, ascii("E-M1MarkIII     ")),
            (SOFTWARE, ascii("Version 1.0")),
            (IMAGE_DESCRIPTION, ascii("                               ")),
            (X_RESOLUTION, rational(300, 1)),
            (Y_RESOLUTION, rational(300, 1)),
        ]);
        let tags = of(&root, None);
        assert_eq!(tags.make.as_deref(), Some("OLYMPUS CORPORATION"));
        assert_eq!(tags.model.as_deref(), Some("E-M1MarkIII"));
        assert_eq!(tags.software.as_deref(), Some("Version 1.0"));
        // A description of spaces is none.
        assert_eq!(tags.image_description, None);
        // Inches where the unit is missing, as EXIF has it.
        assert_eq!(
            tags.resolution,
            Some(Resolution {
                x: 300.0,
                y: 300.0,
                unit: 2
            })
        );
        let centimetres = ifd(vec![
            (X_RESOLUTION, rational(118, 1)),
            (Y_RESOLUTION, rational(118, 1)),
            (RESOLUTION_UNIT, Value::Short(vec![3])),
        ]);
        assert_eq!(of(&centimetres, None).resolution.map(|r| r.unit), Some(3));
        // Half a resolution is none.
        let half = ifd(vec![(X_RESOLUTION, rational(72, 1))]);
        assert_eq!(of(&half, None).resolution, None);
    }

    #[test]
    fn reads_the_exif_ifd_s_tags() {
        let exif = ifd(vec![
            (DIGITAL_ZOOM_RATIO, rational(16, 16)),
            (CONTRAST, Value::Short(vec![1])),
            (SATURATION, Value::Short(vec![2])),
            (SHARPNESS, Value::Short(vec![0])),
            (IMAGE_UNIQUE_ID, ascii("4F2A0000000000000000000000000001")),
            (FOCAL_LENGTH_IN_35MM_FILM, Value::Short(vec![28])),
            (FOCAL_PLANE_X_RESOLUTION, rational(5_000_000, 1000)),
            (FOCAL_PLANE_RESOLUTION_UNIT, Value::Short(vec![3])),
            (PIXEL_X_DIMENSION, Value::Long(vec![6000])),
            (PIXEL_Y_DIMENSION, Value::Short(vec![4000])),
        ]);
        let tags = of(&ifd(Vec::new()), Some(&exif));
        assert_eq!(tags.digital_zoom, Some(1.0));
        assert_eq!(
            (tags.contrast, tags.saturation, tags.sharpness),
            (Some(1), Some(2), Some(0))
        );
        assert_eq!(
            tags.unique_image_id.as_deref(),
            Some("4F2A0000000000000000000000000001")
        );
        assert_eq!(tags.focal_length_35mm, Some(28.0));
        assert_eq!(
            tags.focal_plane,
            Some(FocalPlane {
                x: 5000.0,
                y: 0.0,
                unit: 3
            })
        );
        assert_eq!(tags.dimensions.exif, [6000, 4000]);
        // A zero denominator reads as 0, and a 35 mm equivalent of 0 as unknown.
        let zero = ifd(vec![
            (DIGITAL_ZOOM_RATIO, rational(0, 0)),
            (FOCAL_LENGTH_IN_35MM_FILM, Value::Short(vec![0])),
        ]);
        let tags = of(&ifd(Vec::new()), Some(&zero));
        assert_eq!(
            (tags.digital_zoom, tags.focal_length_35mm),
            (Some(0.0), None)
        );
        // A tag of another type than the standard's is none.
        let odd = ifd(vec![
            (CONTRAST, ascii("high")),
            (MAKE, Value::Short(vec![1])),
        ]);
        let tags = of(&odd, Some(&odd));
        assert_eq!((tags.contrast, tags.make), (None, None));
    }

    #[test]
    fn reads_the_first_sub_ifd_s_size_only_for_the_primary_image() {
        let mut root = ifd(vec![
            (IMAGE_WIDTH, Value::Long(vec![256])),
            (IMAGE_LENGTH, Value::Long(vec![171])),
        ]);
        let raw = ifd(vec![
            (NEW_SUBFILE_TYPE, Value::Long(vec![0])),
            (IMAGE_WIDTH, Value::Long(vec![6048])),
            (IMAGE_LENGTH, Value::Long(vec![4024])),
        ]);
        root.sub.insert(SUB_IFDS, vec![raw]);
        let dimensions = of(&root, None).dimensions;
        assert_eq!(dimensions.sub_image, Some([6048, 4024]));
        assert_eq!(dimensions.image, [256, 171]);
        let preview = ifd(vec![(NEW_SUBFILE_TYPE, Value::Long(vec![1]))]);
        root.sub.insert(SUB_IFDS, vec![preview]);
        assert_eq!(of(&root, None).dimensions.sub_image, None);
    }

    #[test]
    fn a_file_that_is_no_raw_has_none() {
        let source = RawSource::new_from_slice(b"not a raw file at all");
        assert_eq!(read(&source), None);
    }

    /// A Panasonic JPEG's EXIF is read only after SOI, APP1, its length and "Exif\0\0".
    #[test]
    fn an_embedded_jpeg_is_read_only_where_its_exif_comes_first() {
        let exif = *b"\xff\xd8\xff\xe1\x12\x34Exif\0\0";
        assert!(starts_with_exif(&exif));
        // JFIF's APP0 first, a wrong signature, a cut start.
        assert!(!starts_with_exif(b"\xff\xd8\xff\xe0\x00\x10JFIF\0\x01"));
        assert!(!starts_with_exif(b"\xff\xd8\xff\xe1\x12\x34Exif\0\x01"));
        assert!(!starts_with_exif(&exif[..8]));
    }
}
