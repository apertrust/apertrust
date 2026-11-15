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

//! The exposure a camera holds back from the raw in a highlight-preserving mode, as
//! darktable estimates it from the maker notes (`_check_highlight_preservation` in
//! `src/common/exif.cc`, its `exif_highlight_preservation`), reading them through Exiv2
//! 0.28, over rawler's public parsers (`docs/ports/exposure.md`).
//!
//! darktable reads the first of these it finds: Canon's Auto Lighting Optimizer,
//! Fujifilm's dynamic range, Nikon's colour space (HLG) or else its Active D-Lighting,
//! Olympus's gradation, Pentax's dynamic range expansion. Pentax's never counts with
//! Exiv2 0.28, which darktable reads it through as an empty data area, and Sony's is
//! left to do in darktable, so neither is read here.

use std::io::{Read, Seek};

use rawler::bits::Endian;
use rawler::formats::bmff::Bmff;
use rawler::formats::tiff::ifd::OffsetMode;
use rawler::formats::tiff::reader::TiffReader;
use rawler::formats::tiff::{GenericTiffReader, IFD, Value};
use rawler::rawsource::RawSource;
use rawler::tags::ExifTag;

use crate::lens::raf_maker_note;

/// A highlight-preserving mode as the maker notes record it: the tag darktable reads, the
/// first of its list the raw holds, with the values Exiv2's `toLong` reads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Mode {
    /// Canon's Auto Lighting Optimizer, the third value of LightingOpt (0x4018), Exiv2's
    /// `CanonLiOp.AutoLightingOptimizer`: 0 standard, 1 low, 2 strong, 3 off.
    Canon(i64),
    /// Fujifilm's DevelopmentDynamicRange (0x1403), or AutoDynamicRange (0x140b) where it
    /// is missing: 100, 200 or 400 %.
    Fujifilm(i64),
    /// Nikon's ColorSpace (0x001e), 4 for HLG. Where it is recorded, as on every Nikon of
    /// the corpus, darktable reads no Active D-Lighting.
    NikonColorSpace(i64),
    /// Nikon's ActiveDLighting (0x0022): 0 off, 1 low, 3 normal, 5 high, 7 extra high, 8
    /// to 11 extra high 1 to 4, 65535 auto.
    NikonActiveDLighting(i64),
    /// Olympus's Gradation, CameraSettings' (0x2020, tag 0x050f), or RawDevelopment2's
    /// (0x2031, tag 0x0112) where it is missing: normal, auto, low key or high key in the
    /// first three values, a fourth 1 where auto overrode the user's choice.
    OlympusGradation(Vec<i64>),
}

/// darktable's estimate of the exposure `mode` holds back, in EV, as
/// `_check_highlight_preservation` sets `exif_highlight_preservation`: 0 where it
/// estimates none.
pub(crate) fn bias(mode: &Mode) -> f32 {
    match mode {
        Mode::Canon(state) => match state {
            0 => 0.50,
            1 => 0.33,
            2 => 0.66,
            _ => 0.0,
        },
        Mode::Fujifilm(range) => match range {
            200 => 1.0,
            400 => 2.0,
            _ => 0.0,
        },
        Mode::NikonColorSpace(space) => {
            if *space == 4 {
                2.0
            } else {
                0.0
            }
        }
        // darktable: low changes no exposure; auto's strength is unknown.
        Mode::NikonActiveDLighting(level) => match level {
            3 => 0.33,
            5 => 0.66,
            7 => 1.00,
            8 => 1.10,
            9 => 1.20,
            10 => 1.30,
            11 => 1.33,
            _ => 0.0,
        },
        Mode::OlympusGradation(values) => {
            let value = |i: usize| values.get(i).copied().unwrap_or(0);
            if value(3) == 1 {
                // Auto overriding the user's choice, reported as −0.3 EV.
                0.33
            } else if (value(0), value(1), value(2)) == (-1, -1, 1) {
                // Low key, reported as −1 EV, which 0.66 matches the camera's JPEG better.
                0.66
            } else {
                // High key's deliberate overexposure darktable leaves alone.
                0.0
            }
        }
    }
}

/// The exposure the raw's highlight-preserving mode holds back, in EV, as darktable
/// estimates it: 0 where its maker notes record none darktable reads.
pub(crate) fn read(source: &RawSource, make: &str) -> f32 {
    mode(source, make).map_or(0.0, |mode| bias(&mode))
}

/// The mode the raw's maker notes record, as darktable finds it; `None` where they record
/// none darktable reads, or do not parse.
pub(crate) fn mode(source: &RawSource, make: &str) -> Option<Mode> {
    if let Some(note) = raf_maker_note(source) {
        return fujifilm(&note);
    }
    if let Some(note) = cr3_maker_note(source) {
        return canon(&note);
    }
    let mut reader = source.reader();
    let tiff = GenericTiffReader::new(&mut reader, 0, 0, None, &[]).ok()?;
    let exif = tiff.find_first_ifd_with_tag(ExifTag::MakerNotes)?;
    let Value::Undefined(data) = &exif.get_entry(ExifTag::MakerNotes)?.value else {
        return None;
    };
    // Exiv2 tells the maker notes apart by the make and their header, as here. rawler's
    // `parse_makernote` indexes the note's first bytes unchecked, so a note too short for
    // what it reads of its format is none.
    if data.starts_with(b"Nikon\0\x02") {
        if data.len() < NIKON_LEAST {
            return None;
        }
        nikon(
            &exif
                .parse_makernote(&mut reader, OffsetMode::Absolute, &[])
                .ok()??,
        )
    } else if data.starts_with(b"OLYMPUS\0") || data.starts_with(b"OM SYSTEM\0") {
        olympus(&mut reader, exif, data)
    } else if make.starts_with("Canon") {
        if data.len() < HEADERLESS_LEAST {
            return None;
        }
        canon(
            &exif
                .parse_makernote(&mut reader, OffsetMode::Absolute, &[])
                .ok()??,
        )
    } else {
        None
    }
}

/// What rawler's `parse_makernote` reads of any note before it knows its format: its first
/// 9 bytes, compared with "SONY DSC ". A headerless note, Canon's, needs no more.
const HEADERLESS_LEAST: usize = 9;

/// What it reads of a Nikon note: the byte order after its 10-byte header.
const NIKON_LEAST: usize = 12;

/// A CR3's Canon maker note, the TIFF in its CMT3 box; `None` for a raw that is not one.
fn cr3_maker_note(source: &RawSource) -> Option<IFD> {
    if source.subview(4, 8).ok()? != b"ftypcrx " {
        return None;
    }
    let bmff = Bmff::new(&mut source.reader()).ok()?;
    let description = bmff.filebox.moov.cr3desc?;
    Some(description.cmt3.tiff.root_ifd().clone())
}

/// A tag's values as Exiv2's `toLong` reads each, where darktable's `FIND_EXIF_TAG` finds
/// the tag: present and holding at least one value.
fn longs(ifd: &IFD, tag: u16) -> Option<Vec<i64>> {
    integers(&ifd.get_entry(tag)?.value)
}

/// Every value as an integer; `None` for no values, or where any is no number rawler can
/// convert (bytes of an undefined type, text), which reads as unknown rather than as 0,
/// since 0 is a mode of its own (Canon's standard optimizer).
fn integers(value: &Value) -> Option<Vec<i64>> {
    let values = (0..value.count())
        .map(|i| value.get_i64(i).ok().flatten())
        .collect::<Option<Vec<i64>>>()?;
    (!values.is_empty()).then_some(values)
}

/// Canon's: LightingOpt's third value, where Exiv2 0.28 decodes LightingOpt into the
/// CanonLiOp array and darktable reads its AutoLightingOptimizer.
fn canon(note: &IFD) -> Option<Mode> {
    longs(note, 0x4018)?.get(2).copied().map(Mode::Canon)
}

/// Fujifilm's: DevelopmentDynamicRange, else AutoDynamicRange.
fn fujifilm(note: &IFD) -> Option<Mode> {
    longs(note, 0x1403)
        .or_else(|| longs(note, 0x140b))
        .map(|values| Mode::Fujifilm(values[0]))
}

/// Nikon's: ColorSpace, else ActiveDLighting.
fn nikon(note: &IFD) -> Option<Mode> {
    if let Some(values) = longs(note, 0x001e) {
        return Some(Mode::NikonColorSpace(values[0]));
    }
    longs(note, 0x0022).map(|values| Mode::NikonActiveDLighting(values[0]))
}

/// Olympus's and OM System's: the Gradation of CameraSettings, else of RawDevelopment2.
/// Their maker note is a TIFF directory after a header, "OLYMPUS\0" and a byte order in
/// 12 bytes or "OM SYSTEM" and one in 16, its offsets from the note's start, and each of
/// the two is a directory of its own: at the offset its tag holds where the tag is a
/// directory pointer, or at the tag's own data where it is of the undefined type.
fn olympus<R: Read + Seek>(reader: &mut R, exif: &IFD, data: &[u8]) -> Option<Mode> {
    let start = exif
        .base
        .checked_add(u32::try_from(exif.get_entry(ExifTag::MakerNotes)?.offset()?).ok()?)?;
    let header: usize = if data.starts_with(b"OLYMPUS\0") {
        12
    } else {
        16
    };
    let endian = match data.get(header - 4..header - 2)? {
        b"II" => Endian::Little,
        b"MM" => Endian::Big,
        _ => return None,
    };
    start.checked_add(header as u32)?;
    let note = IFD::new(reader, header as u32, start, 0, endian, &[]).ok()?;
    let mut gradation = |directory: u16, tag: u16| {
        let entry = note.get_entry(directory)?;
        let at = match entry.value {
            Value::Undefined(_) => u32::try_from(entry.offset()?).ok()?,
            _ => note
                .get_entry_raw_with_len(directory, reader, 4)
                .ok()??
                .get_force_u32(0),
        };
        start.checked_add(at)?;
        longs(&IFD::new(reader, at, start, 0, endian, &[]).ok()?, tag)
    };
    gradation(0x2020, 0x050f)
        .or_else(|| gradation(0x2031, 0x0112))
        .map(Mode::OlympusGradation)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn darktable_s_estimates_by_mode() {
        let canon = [(0, 0.5), (1, 0.33), (2, 0.66), (3, 0.0)];
        for (state, ev) in canon {
            assert_eq!(bias(&Mode::Canon(state)), ev, "Canon {state}");
        }
        for (range, ev) in [(100, 0.0), (200, 1.0), (400, 2.0)] {
            assert_eq!(bias(&Mode::Fujifilm(range)), ev, "Fujifilm DR{range}");
        }
        // HLG; sRGB and Adobe RGB, which leave Active D-Lighting unread.
        for (space, ev) in [(4, 2.0), (1, 0.0), (2, 0.0)] {
            assert_eq!(bias(&Mode::NikonColorSpace(space)), ev, "Nikon {space}");
        }
        let nikon = [
            (0, 0.0),
            (1, 0.0),
            (3, 0.33),
            (5, 0.66),
            (7, 1.0),
            (8, 1.1),
            (9, 1.2),
            (10, 1.3),
            (11, 1.33),
            (65535, 0.0),
        ];
        for (level, ev) in nikon {
            assert_eq!(bias(&Mode::NikonActiveDLighting(level)), ev, "ADL {level}");
        }
        let olympus: [(&[i64], f32); 6] = [
            // Normal, user-selected: the corpus's two.
            (&[0, -1, 1, 0], 0.0),
            // Auto overriding the user's choice.
            (&[0, -1, 1, 1], 0.33),
            // Low key, then high key, which is left alone.
            (&[-1, -1, 1, 0], 0.66),
            (&[-1, -1, 1], 0.66),
            (&[1, -1, 1, 0], 0.0),
            // Auto, chosen by the user.
            (&[-1, -1, 2, 0], 0.0),
        ];
        for (values, ev) in olympus {
            assert_eq!(
                bias(&Mode::OlympusGradation(values.to_vec())),
                ev,
                "{values:?}"
            );
        }
    }

    #[test]
    fn a_value_that_is_no_number_reads_as_unknown() {
        assert_eq!(integers(&Value::SLong(vec![0, 1, 0])), Some(vec![0, 1, 0]));
        assert_eq!(integers(&Value::Undefined(vec![0, 1, 0, 0])), None);
        assert_eq!(integers(&Value::Short(Vec::new())), None);
    }

    /// A little-endian TIFF whose first directory holds only a maker note of `note`.
    fn tiff_with_maker_note(note: &[u8]) -> Vec<u8> {
        let mut tiff = b"II*\0".to_vec();
        tiff.extend_from_slice(&8u32.to_le_bytes());
        tiff.extend_from_slice(&1u16.to_le_bytes());
        tiff.extend_from_slice(&0x927cu16.to_le_bytes());
        tiff.extend_from_slice(&7u16.to_le_bytes());
        tiff.extend_from_slice(&(note.len() as u32).to_le_bytes());
        if note.len() <= 4 {
            let mut inline = note.to_vec();
            inline.resize(4, 0);
            tiff.extend_from_slice(&inline);
        } else {
            tiff.extend_from_slice(&26u32.to_le_bytes());
        }
        tiff.extend_from_slice(&0u32.to_le_bytes());
        if note.len() > 4 {
            tiff.extend_from_slice(note);
        }
        tiff
    }

    #[test]
    fn a_broken_maker_note_reads_as_none_without_panicking() {
        let read = |note: &[u8], make: &str| {
            let tiff = tiff_with_maker_note(note);
            mode(&RawSource::new_from_slice(&tiff), make)
        };
        let padded = |head: &[u8]| {
            let mut note = head.to_vec();
            note.resize(64, 0);
            note
        };
        // A byte order that is neither, and a header cut short.
        assert_eq!(read(&padded(b"OLYMPUS\0XX\x03\0"), "OLYMPUS"), None);
        assert_eq!(
            read(&padded(b"OM SYSTEM\0\0\0ZZ\x04\0"), "OM Digital"),
            None
        );
        assert_eq!(read(b"OM SYSTEM\0\0\0II", "OM Digital"), None);
        // Shorter than rawler's `parse_makernote` reads, and empty.
        for note in [&b"Nikon\0\x02"[..], b"\x01\0", b""] {
            assert_eq!(read(note, "Canon"), None);
            assert_eq!(read(note, "NIKON"), None);
        }
    }

    #[test]
    fn a_short_canon_note_still_reads() {
        // The shortest Canon note darktable reads, 30 bytes: a directory of one
        // LightingOpt entry, then its three LONGs, the size and the optimizer standard,
        // +0.5 EV in darktable. The note starts at byte 26 of the TIFF, so its values at 44.
        let mut note = 1u16.to_le_bytes().to_vec();
        note.extend_from_slice(&0x4018u16.to_le_bytes());
        note.extend_from_slice(&9u16.to_le_bytes());
        note.extend_from_slice(&3u32.to_le_bytes());
        note.extend_from_slice(&44u32.to_le_bytes());
        note.extend_from_slice(&0u32.to_le_bytes());
        for value in [12i32, 0, 0] {
            note.extend_from_slice(&value.to_le_bytes());
        }
        assert_eq!(note.len(), 30);
        let tiff = tiff_with_maker_note(&note);
        let source = RawSource::new_from_slice(&tiff);
        assert_eq!(mode(&source, "Canon"), Some(Mode::Canon(0)));
        assert_eq!(read(&source, "Canon"), 0.5);
    }

    fn corpus_dir() -> PathBuf {
        std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
            PathBuf::from,
        )
    }

    /// Exiv2 0.28.9's reading of the tags darktable reads, `exiv2 -pa` on each raw of the
    /// corpus, as `toLong` gives them: every Canon CR2 and CR3 but the 5D Mark II's sRAW,
    /// whose Auto Lighting Optimizer sits in its custom functions, every Fujifilm RAF but
    /// the X-H2S's, every Nikon NEF and both Olympus ORFs. The others hold none darktable
    /// reads: a DNG's maker note Exiv2 does not read, Pentax's and Sony's, which darktable
    /// does not count, or none at all.
    fn exiv2(name: &str) -> Option<Mode> {
        Some(match name {
            "Canon/EOS 5D Mark II/09.canon.sraw1.cr2" => return None,
            "Canon/EOS 7D Mark II/capture000106.cr2" => Mode::Canon(3),
            _ if name.starts_with("Canon/") => Mode::Canon(0),
            "Fujifilm/X-H2S/IMG-20221005-0665.raf" => return None,
            _ if name.starts_with("Fujifilm/") => Mode::Fujifilm(100),
            _ if name.starts_with("Nikon/") => Mode::NikonColorSpace(1),
            _ if name.starts_with("Olympus/") || name.starts_with("OM System/") => {
                Mode::OlympusGradation(vec![0, -1, 1, 0])
            }
            _ => return None,
        })
    }

    #[test]
    #[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
    fn the_corpus_reads_as_exiv2_reads_it() {
        let dir = corpus_dir();
        let list = include_str!("../tests/corpus/files.sha256");
        for name in list
            .lines()
            .filter_map(|line| Some(line.split_once("  ")?.1))
        {
            let path = dir.join(name);
            let source = RawSource::new(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
            // The make the file names, which Exiv2 picks the maker note by.
            let make = name.split('/').next().unwrap();
            assert_eq!(mode(&source, make), exiv2(name), "{name}");
        }
    }
}
