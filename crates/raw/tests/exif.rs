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

//! Holds the EXIF tags rawler's metadata lacks, which `apertrust-raw` reads through
//! rawler's TIFF parser for the Metadata tool (decision 110), to what Exiv2 reads of them:
//! `corpus/exif-tags.txt` is Exiv2 0.28's dump of the same tags over every raw.pixls.us
//! sample, and lists each raw whose tags cannot be reached, with why.
//!
//! Needs the raw corpus (`tools/fetch-raw-corpus.sh`):
//! `cargo test --release -p apertrust-raw -- --ignored`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use apertrust_raw::{Exif, Resolution};

const LIST: &str = include_str!("corpus/files.sha256");
const EXIV2: &str = include_str!("corpus/exif-tags.txt");

fn corpus_dir() -> PathBuf {
    std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    )
}

/// A file's lines in the dump: each tag with its value as Exiv2 prints it, and why its
/// tags cannot be reached where they cannot.
#[derive(Default)]
struct Dumped {
    tags: BTreeMap<&'static str, &'static str>,
    unread: Option<&'static str>,
}

fn dumped() -> BTreeMap<&'static str, Dumped> {
    let mut files: BTreeMap<&str, Dumped> = BTreeMap::new();
    for line in EXIV2.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.splitn(4, " | ").collect();
        let file = files.entry(fields[0]).or_default();
        match fields[1] {
            "unread" => file.unread = Some(fields[2]),
            tag => {
                file.tags.insert(tag, fields.get(3).copied().unwrap_or(""));
            }
        }
    }
    files
}

/// A text as Exiv2 prints it, as the reader keeps it: trimmed, none where empty.
fn text(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// A number as Exiv2 prints it, a rational's numerator over its denominator as its
/// `toFloat` reads it, 0 for a zero denominator.
fn number(value: Option<&str>) -> Option<f32> {
    let value = value?.trim();
    Some(match value.split_once('/') {
        Some((n, d)) => {
            let (n, d) = (n.parse::<f64>().unwrap(), d.parse::<f64>().unwrap());
            if d == 0.0 { 0.0 } else { (n / d) as f32 }
        }
        None => value.parse().unwrap(),
    })
}

/// What the reader gives for the tags Exiv2 printed.
fn expected(tags: &BTreeMap<&str, &str>) -> Exif {
    let get = |tag: &str| tags.get(tag).copied();
    let short = |tag: &str| number(get(tag)).map(|n| n as u16);
    let resolution = match (number(get("XResolution")), number(get("YResolution"))) {
        (Some(x), Some(y)) => Some(Resolution {
            x,
            y,
            unit: short("ResolutionUnit").unwrap_or(2),
        }),
        _ => None,
    };
    Exif {
        make_written: text(get("Make")),
        model_written: text(get("Model")),
        software: text(get("Software")),
        image_description: text(get("ImageDescription")),
        resolution,
        digital_zoom: number(get("DigitalZoomRatio")),
        contrast: short("Contrast"),
        saturation: short("Saturation"),
        sharpness: short("Sharpness"),
        unique_image_id: text(get("ImageUniqueID")),
        focal_length_35mm: number(get("FocalLengthIn35mmFilm")).filter(|mm| *mm > 0.0),
        ..Exif::default()
    }
}

/// The fields of `exif` the dump holds.
fn read(exif: &Exif) -> Exif {
    Exif {
        make_written: exif.make_written.clone(),
        model_written: exif.model_written.clone(),
        software: exif.software.clone(),
        image_description: exif.image_description.clone(),
        resolution: exif.resolution,
        digital_zoom: exif.digital_zoom,
        contrast: exif.contrast,
        saturation: exif.saturation,
        sharpness: exif.sharpness,
        unique_image_id: exif.unique_image_id.clone(),
        focal_length_35mm: exif.focal_length_35mm,
        ..Exif::default()
    }
}

#[test]
#[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
fn the_corpus_reads_the_tags_exiv2_reads() {
    let dir = corpus_dir();
    let dumped = dumped();
    let names: Vec<&str> = LIST
        .lines()
        .filter_map(|line| line.split_once("  "))
        .map(|(_, name)| name)
        .collect();
    let mut failed = Vec::new();
    for name in names {
        let path = dir.join(name);
        assert!(
            path.exists(),
            "{} is missing: run tools/fetch-raw-corpus.sh",
            path.display()
        );
        let file = dumped
            .get(name)
            .unwrap_or_else(|| panic!("{name} is not in exif-tags.txt"));
        let Ok(exif) = apertrust_raw::read_exif_file(&path) else {
            if file.unread.is_none() {
                failed.push(format!("{name}: no EXIF, and not listed unread"));
            }
            continue;
        };
        if file.unread.is_some() {
            assert!(!exif.unread.is_empty(), "{name}: listed unread, but read");
            assert_eq!(read(&exif), Exif::default(), "{name}");
            continue;
        }
        assert!(exif.unread.is_empty(), "{name}: unread {:?}", exif.unread);
        let mut wanted = expected(&file.tags);
        // A CR3 with no ImageUniqueID has rawler's unique id in its place.
        if wanted.unique_image_id.is_none() && name.to_lowercase().ends_with(".cr3") {
            wanted.unique_image_id = exif.unique_image_id.clone();
        }
        if read(&exif) != wanted {
            failed.push(format!(
                "{name}:\n  read {:?}\n  Exiv2 {wanted:?}",
                read(&exif)
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
