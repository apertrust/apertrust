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

//! What darktable's own sidecar, `<photo>.xmp`, says of a photo's fields (decisions 51 and
//! 110): read while the photo has no `.apxmp`, and copied into it the first time Apertrust
//! writes one, never written. Built: the rating, the reject and the colour labels, read as
//! darktable reads them (`src/common/exif.cc:639-676` at the reference release).

use std::io;
use std::path::{Path, PathBuf};

use crate::fields::{self, DARKTABLE_NAMESPACE, FieldEntry, FieldValue, XMP_NAMESPACE};
use crate::xmp::Xmp;

/// What darktable's sidecar's name adds to its photo's.
pub const SUFFIX: &str = ".xmp";

/// darktable's sidecar beside `photo`, its first version's: the photo's whole name, then
/// `.xmp`.
pub fn path_for(photo: &Path) -> PathBuf {
    let mut name = photo.as_os_str().to_owned();
    name.push(SUFFIX);
    PathBuf::from(name)
}

/// The fields a darktable sidecar holds, each with no clock: the clock of the write that
/// copies them into the `.apxmp` is theirs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DarktableFields {
    pub fields: Vec<FieldEntry>,
}

/// The fields of the darktable sidecar beside `photo`: `None` where there is none, or where
/// it is no XMP, which is told under `-d settings`; an error where it cannot be read.
pub fn read_fields(photo: &Path) -> io::Result<Option<DarktableFields>> {
    let path = path_for(photo);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(parse_bytes(&path, &bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// The fields darktable's sidecar at `path` holds as `bytes`, `None` where they are no XMP.
pub(crate) fn parse_bytes(path: &Path, bytes: &[u8]) -> Option<DarktableFields> {
    let parsed = std::str::from_utf8(bytes)
        .map_err(|error| error.to_string())
        .and_then(|xml| parse(xml).map_err(|error| error.to_string()));
    match parsed {
        Ok(read) => Some(read),
        Err(error) => {
            let name = crate::sidecar::file_name(path);
            tracing::debug!(target: "settings", "{name}: not darktable's XMP, left out: {error}");
            None
        }
    }
}

/// The fields of a darktable sidecar's XMP, as darktable reads them:
///
/// - `xmp:Rating`, 0 to 5 stars, or −1 for a reject, which keeps no stars; any other number
///   is no rating, as is none, where darktable gives its import's initial rating;
/// - `xmp:Label`, where set, one label by its name, Red to Purple, and nothing for another
///   name; else `darktable:colorlabels`, an `rdf:Seq` of darktable's numbers, 0 red to 4
///   purple, a number it does not know left out.
pub fn parse(xml: &str) -> Result<DarktableFields, crate::xmp::Error> {
    let xmp = Xmp::parse(xml)?;
    let mut read = Vec::new();
    let rating = (xmp.get(XMP_NAMESPACE, "Rating"))
        .and_then(|node| node.as_str())
        .and_then(|text| text.trim().parse::<i32>().ok());
    match rating {
        Some(-1) => read.push(FieldEntry::unclocked(
            fields::REJECTED,
            FieldValue::Marked(true),
        )),
        Some(stars @ 0..=5) => read.push(FieldEntry::unclocked(
            fields::RATING,
            FieldValue::Stars(stars as u8),
        )),
        _ => {}
    }
    const NAMES: [&str; 5] = ["Red", "Yellow", "Green", "Blue", "Purple"];
    let labels = match xmp.get(XMP_NAMESPACE, "Label") {
        Some(label) => (NAMES.iter())
            .position(|name| label.as_str() == Some(name))
            .map_or(0, |at| 1 << at),
        None => (xmp.get(DARKTABLE_NAMESPACE, "colorlabels"))
            .and_then(|node| node.items())
            .unwrap_or_default()
            .iter()
            .filter_map(|item| item.as_str()?.trim().parse::<u8>().ok())
            .filter(|label| *label < fields::LABELS)
            .fold(0, |bits, label| bits | 1 << label),
    };
    if labels != 0 {
        read.push(FieldEntry::unclocked(
            fields::COLOUR_LABELS,
            FieldValue::Labels(labels),
        ));
    }
    read.sort_by(|a, b| a.field.cmp(&b.field));
    Ok(DarktableFields { fields: read })
}
