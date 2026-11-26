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

//! darktable's ratings and labels, read from its own sidecars as darktable reads them
//! (`src/common/exif.cc:639-676`), the fixtures in `tests/fixtures/darktable`.

use std::path::Path;

use apertrust_library::Rating;
use apertrust_library::darktable::{self, DarktableFields};
use tempfile::TempDir;

/// The fixture `name` beside a photo in a folder of its own, read as the photo's.
fn read(name: &str) -> Option<DarktableFields> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/darktable")
        .join(name);
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_0001.CR2");
    std::fs::copy(fixture, darktable::path_for(&photo)).unwrap();
    darktable::read_fields(&photo).unwrap()
}

fn rating(name: &str) -> Rating {
    let read = read(name).unwrap_or_else(|| panic!("{name} reads"));
    assert!(read.fields.iter().all(|field| field.clock.is_none()));
    Rating::of(&read.fields)
}

#[test]
fn stars_and_labels_read_as_darktable_reads_them() {
    let cases = [
        // Red and blue.
        ("rating-3-labels.xmp", (3, false, 0b1001)),
        // −1 is a reject, with no stars beneath it.
        ("rejected.xmp", (0, true, 0)),
        // `xmp:Label` wins over darktable's own labels.
        ("label-and-colorlabels.xmp", (2, false, 0b100)),
        // Yellow and purple, whatever their order.
        ("labels-alone.xmp", (0, false, 0b1_0010)),
    ];
    for (name, (stars, rejected, labels)) in cases {
        assert_eq!(
            rating(name),
            Rating {
                stars,
                rejected,
                labels
            },
            "{name}"
        );
    }
}

#[test]
fn a_photo_with_no_darktable_sidecar_or_none_that_reads_has_no_fields() {
    let dir = TempDir::new().unwrap();
    let photo = dir.path().join("IMG_0001.CR2");
    assert_eq!(darktable::read_fields(&photo).unwrap(), None);
    std::fs::write(darktable::path_for(&photo), "not XMP").unwrap();
    assert_eq!(darktable::read_fields(&photo).unwrap(), None);
}

#[test]
fn a_label_darktable_does_not_know_and_a_rating_past_five_are_none() {
    let xml = |rating: &str, label: &str| {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF
            xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description
            xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="{rating}" xmp:Label="{label}"/>
            </rdf:RDF></x:xmpmeta>"#
        )
    };
    let read = darktable::parse(&xml("7", "Orange")).unwrap();
    assert_eq!(read.fields, []);
    let read = darktable::parse(&xml("5", "Purple")).unwrap();
    assert_eq!(
        Rating::of(&read.fields),
        Rating {
            stars: 5,
            rejected: false,
            labels: 0b1_0000
        }
    );
}

/// darktable's metadata fields, each read into its field as the photo's own with no clock,
/// its notes into Apertrust's and its version name as the copy's name.
#[test]
fn darktable_s_metadata_fields_read_as_the_photo_s_own() {
    use apertrust_library::fields::{FieldValue, value};

    let read = read("fields.xmp").expect("it reads");
    assert!(read.fields.iter().all(|field| field.clock.is_none()));
    let alt = |text: &str| FieldValue::LangAlt(vec![("x-default".to_owned(), text.to_owned())]);
    let list = |text: &str| FieldValue::List(vec![text.to_owned()]);
    let cases = [
        ("dc:title", alt("Morning market, Alfama")),
        (
            "dc:description",
            alt("Fish stalls before the crowd,\nthe light coming in low over the river."),
        ),
        ("dc:creator", list("Ana Sousa")),
        ("dc:publisher", list("Ana Sousa Studio")),
        ("dc:rights", alt("© 2026 Ana Sousa")),
        (
            "apertrust:notes",
            FieldValue::Text("Try a warmer crop".to_owned()),
        ),
        ("xmp:Rating", FieldValue::Stars(2)),
    ];
    for (field, expected) in &cases {
        assert_eq!(
            value(&read.fields, field).as_ref(),
            Some(expected),
            "{field}"
        );
    }
    assert_eq!(read.fields.len(), cases.len());
    assert_eq!(read.name, "Black and white");
    // darktable's own sidecar from its integration tests, as darktable wrote it.
    let xml = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/xmp/darktable-highlight-opposed.xmp"),
    )
    .unwrap();
    let written = darktable::parse(&xml).unwrap();
    assert_eq!(value(&written.fields, "dc:title"), Some(alt("t")));
    assert_eq!(value(&written.fields, "dc:creator"), Some(list("O")));
    assert_eq!(written.name, "");
}
