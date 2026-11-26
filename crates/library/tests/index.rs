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

//! The folder index, on temporary folders with sidecars written through
//! `apertrust_library::sidecar`.

use std::fs;
use std::path::Path;

use apertrust_library::sidecar::{self, Document, Entry, Newer};
use apertrust_library::{Index, Options, Sidecar};
use tempfile::TempDir;

fn raw(dir: &Path, name: &str) {
    fs::write(dir.join(name), [0u8; 8]).unwrap();
}

fn document(exposure: &str) -> Document {
    let mut document = Document::new("test");
    document.entries.push(Entry::Operation {
        operation: "exposure".to_owned(),
        schema: Some(1),
        params: vec![("exposure".to_owned(), exposure.to_owned())],
    });
    document
}

/// A raw with a sidecar holding `document`.
fn edited(dir: &Path, name: &str, document: &Document) {
    raw(dir, name);
    sidecar::write_xml(&sidecar::path_for(&dir.join(name)), &document.to_xml()).unwrap();
}

fn build(dir: &Path) -> Index {
    Index::build(dir, Options::default(), &|| false).unwrap()
}

fn read<'a>(index: &'a Index, name: &str) -> &'a apertrust_library::Edit {
    let photo = index.photos.iter().find(|p| p.name == name).unwrap();
    match &photo.sidecar {
        Sidecar::Read(edit) => edit,
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn an_edited_photo_has_its_version_and_hash() {
    let dir = TempDir::new().unwrap();
    edited(dir.path(), "DSCF0001.RAF", &document("0.5"));
    raw(dir.path(), "DSCF0002.RAF");

    let index = build(dir.path());

    assert!(!index.cancelled);
    let edit = read(&index, "DSCF0001.RAF");
    assert_eq!(edit.version, 1);
    assert!(edit.edited);
    assert_eq!(edit.view_only, None);
    let bytes = fs::read(dir.path().join("DSCF0001.RAF.apxmp")).unwrap();
    assert_eq!(edit.file_hash, apertrust_library::index::hash(&bytes));
    // A photo with no sidecar has no edit and nothing to read.
    assert_eq!(index.photos[1].sidecar, Sidecar::None);
    assert_eq!(index.photos[1].file, None);
    assert!(index.photos[0].file.is_some());
    let counts = index.counts();
    assert_eq!(
        (counts.edited, counts.view_only, counts.unreadable),
        (1, 0, 0)
    );
}

#[test]
fn a_sidecar_without_entries_is_no_edit() {
    let dir = TempDir::new().unwrap();
    edited(dir.path(), "DSCF0001.RAF", &Document::new("test"));

    let index = build(dir.path());

    assert!(!read(&index, "DSCF0001.RAF").edited);
    assert_eq!(index.counts().edited, 0);
}

#[test]
fn a_newer_sidecar_is_counted_view_only() {
    let dir = TempDir::new().unwrap();
    let mut newer = document("0.5");
    newer.version = sidecar::COPIES_VERSION + 1;
    edited(dir.path(), "DSCF0001.RAF", &newer);
    edited(dir.path(), "DSCF0002.RAF", &document("0.5"));

    let index = build(dir.path());

    let edit = read(&index, "DSCF0001.RAF");
    assert_eq!(edit.version, sidecar::COPIES_VERSION + 1);
    assert_eq!(
        edit.view_only,
        Some(Newer::Version(sidecar::COPIES_VERSION + 1))
    );
    assert_eq!(read(&index, "DSCF0002.RAF").view_only, None);
    let counts = index.counts();
    assert_eq!(
        (counts.edited, counts.view_only, counts.unreadable),
        (2, 1, 0)
    );
}

#[test]
fn a_newer_sidecar_with_no_entries_is_edited_and_view_only() {
    let dir = TempDir::new().unwrap();
    let mut newer = Document::new("test");
    newer.version = sidecar::COPIES_VERSION + 1;
    edited(dir.path(), "DSCF0001.RAF", &newer);

    let index = build(dir.path());

    let edit = read(&index, "DSCF0001.RAF");
    assert!(edit.edited);
    assert!(edit.view_only.is_some());
    let counts = index.counts();
    assert_eq!(
        (counts.edited, counts.view_only, counts.unreadable),
        (1, 1, 0)
    );
}

/// Version 2, the engine's: a photo is edited where some entry is not empty at the end, its
/// log folded as text, and not at Original or where its last entries are empty.
#[test]
fn a_version_2_sidecar_is_edited_by_its_log_up_to_the_end() {
    let dir = TempDir::new().unwrap();
    let text = sidecar::Edits::parse(&document("0.5").to_xml(), 0).unwrap();
    let mut at_original = text.clone();
    at_original.copies[0].end.clear();
    let mut reset = text.clone();
    let mut empty = reset.copies[0].history[0].clone();
    empty.id = "00000000000000ff".to_owned();
    if let Entry::Operation { params, .. } = &mut empty.entries[0] {
        params.clear();
    }
    reset.copies[0].end = empty.id.clone();
    reset.copies[0].history.push(empty);
    for (name, edits) in [("A.RAF", &text), ("B.RAF", &at_original), ("C.RAF", &reset)] {
        raw(dir.path(), name);
        let path = sidecar::path_for(&dir.path().join(name));
        sidecar::write_xml(&path, &edits.to_xml()).unwrap();
    }

    let index = build(dir.path());

    let edited = |name| read(&index, name).edited;
    assert_eq!(
        (edited("A.RAF"), edited("B.RAF"), edited("C.RAF")),
        (true, false, false)
    );
    assert_eq!(read(&index, "A.RAF").version, sidecar::COPIES_VERSION);
    assert_eq!(read(&index, "A.RAF").view_only, None);
}

#[test]
fn a_corrupt_sidecar_is_unreadable_and_the_others_are_indexed() {
    let dir = TempDir::new().unwrap();
    edited(dir.path(), "DSCF0001.RAF", &document("0.5"));
    raw(dir.path(), "DSCF0002.RAF");
    fs::write(dir.path().join("DSCF0002.RAF.apxmp"), "this is not xml <<").unwrap();
    raw(dir.path(), "DSCF0003.RAF");
    // Not even text.
    fs::write(dir.path().join("DSCF0003.RAF.apxmp"), [0xff, 0xfe, 0x00]).unwrap();
    edited(dir.path(), "DSCF0004.RAF", &document("1.0"));

    let index = build(dir.path());

    assert!(!index.cancelled);
    assert_eq!(index.photos.len(), 4);
    assert!(read(&index, "DSCF0001.RAF").edited);
    assert!(read(&index, "DSCF0004.RAF").edited);
    for name in ["DSCF0002.RAF", "DSCF0003.RAF"] {
        let photo = index.photos.iter().find(|p| p.name == name).unwrap();
        assert!(matches!(photo.sidecar, Sidecar::Unreadable(_)), "{name}");
    }
    let counts = index.counts();
    assert_eq!(
        (counts.edited, counts.view_only, counts.unreadable),
        (2, 0, 2)
    );
}

#[test]
fn the_hash_changes_when_the_bytes_do() {
    let dir = TempDir::new().unwrap();
    edited(dir.path(), "DSCF0001.RAF", &document("0.5"));
    edited(dir.path(), "DSCF0002.RAF", &document("0.5"));
    let before = read(&build(dir.path()), "DSCF0001.RAF").file_hash;

    // The same bytes give the same hash, on another photo too.
    assert_eq!(read(&build(dir.path()), "DSCF0002.RAF").file_hash, before);

    edited(dir.path(), "DSCF0001.RAF", &document("0.6"));
    assert_ne!(read(&build(dir.path()), "DSCF0001.RAF").file_hash, before);
}

#[test]
fn a_cancelled_build_says_so() {
    let dir = TempDir::new().unwrap();
    edited(dir.path(), "DSCF0001.RAF", &document("0.5"));
    edited(dir.path(), "DSCF0002.RAF", &document("0.5"));

    let index = Index::build(dir.path(), Options::default(), &|| true).unwrap();

    assert!(index.cancelled);
    // Nothing was read, and nothing is passed off as read.
    assert!(
        index
            .photos
            .iter()
            .all(|p| !matches!(p.sidecar, Sidecar::Read(_)))
    );
    assert_eq!(index.counts().edited, 0);
}

#[test]
fn a_folder_that_cannot_be_listed_is_an_error() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing");

    assert!(Index::build(&missing, Options::default(), &|| false).is_err());
}

/// Each photo's rating: its `.apxmp`'s first copy's, else darktable's `.xmp`'s beside it,
/// which is not looked for where the photo has an `.apxmp`, else none (decision 110).
#[test]
fn a_rating_is_read_from_the_apxmp_else_from_darktable_s_xmp() {
    use apertrust_library::clock::{Clock, MachineId};
    use apertrust_library::fields::{self, FieldValue};
    use apertrust_library::{Rating, darktable};

    let dir = TempDir::new().unwrap();
    let darktable_xmp = |name: &str, rating: &str| {
        let xml = format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF
            xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description
            xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="{rating}"/>
            </rdf:RDF></x:xmpmeta>"#
        );
        fs::write(darktable::path_for(&dir.path().join(name)), xml).unwrap();
    };
    // Rated in Apertrust, with darktable's sidecar beside it too.
    raw(dir.path(), "A.RAF");
    let mut edits = sidecar::Edits::new("test");
    edits.copies.push(sidecar::VirtualCopy {
        id: sidecar::FIRST_COPY.to_owned(),
        number: "0".to_owned(),
        name: String::new(),
        name_clock: None,
        end: String::new(),
        clocks: Vec::new(),
        fields: Vec::new(),
        history: Vec::new(),
    });
    let clock = Clock::at(1, None, MachineId::UNKNOWN);
    let first = &mut edits.copies[0].fields;
    fields::set(first, fields::RATING, Some(FieldValue::Stars(4)), clock);
    fields::set(
        first,
        fields::REJECTED,
        Some(FieldValue::Marked(true)),
        clock,
    );
    sidecar::write_xml(
        &sidecar::path_for(&dir.path().join("A.RAF")),
        &edits.to_xml(),
    )
    .unwrap();
    darktable_xmp("A.RAF", "1");
    // Rated in darktable alone.
    raw(dir.path(), "B.RAF");
    darktable_xmp("B.RAF", "3");
    // Neither.
    raw(dir.path(), "C.RAF");

    let index = build(dir.path());

    let ratings: Vec<Rating> = index.photos.iter().map(|photo| photo.rating()).collect();
    let rated = |stars, rejected| Rating {
        stars,
        rejected,
        labels: 0,
    };
    assert_eq!(ratings, [rated(4, true), rated(3, false), rated(0, false)]);
    assert_eq!(read(&index, "A.RAF").rating, rated(4, true));
    assert_eq!(index.photos[0].darktable, None);
    // A rating does not make a photo edited.
    assert_eq!(index.counts().edited, 0);
}

/// Each photo's fields: its `.apxmp`'s first copy's, its name and how many copies it has,
/// else darktable's `.xmp`'s, its version name the copy's, else none (decision 110).
#[test]
fn fields_are_read_from_the_apxmp_else_from_darktable_s_xmp() {
    use apertrust_library::clock::{Clock, MachineId};
    use apertrust_library::darktable;
    use apertrust_library::fields::{self, COPY_NAME, FieldValue};

    let dir = TempDir::new().unwrap();
    let title = |text: &str| FieldValue::LangAlt(vec![("x-default".to_owned(), text.to_owned())]);
    // Titled in Apertrust, with two copies, the first named.
    raw(dir.path(), "A.RAF");
    let mut edits = sidecar::Edits::new("test");
    let copy = |id: &str, number: &str, name: &str| sidecar::VirtualCopy {
        id: id.to_owned(),
        number: number.to_owned(),
        name: name.to_owned(),
        name_clock: None,
        end: String::new(),
        clocks: Vec::new(),
        fields: Vec::new(),
        history: Vec::new(),
    };
    edits.copies.push(copy(sidecar::FIRST_COPY, "0", "Colour"));
    edits.copies.push(copy("1f2e3d4c", "1", "Black and white"));
    let clock = Clock::at(1, None, MachineId::UNKNOWN);
    let first = &mut edits.copies[0].fields;
    fields::set(first, "dc:title", Some(title("Tram 28")), clock);
    fields::set(first, fields::RATING, Some(FieldValue::Stars(4)), clock);
    // A field cleared holds no value.
    fields::set(first, "dc:description", None, clock);
    sidecar::write_xml(
        &sidecar::path_for(&dir.path().join("A.RAF")),
        &edits.to_xml(),
    )
    .unwrap();
    // darktable's alone.
    raw(dir.path(), "B.RAF");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/darktable/fields.xmp");
    fs::copy(fixture, darktable::path_for(&dir.path().join("B.RAF"))).unwrap();
    // Neither.
    raw(dir.path(), "C.RAF");

    let index = build(dir.path());

    let a = index.photos[0].fields();
    assert_eq!(a.values, [("dc:title".to_owned(), title("Tram 28"))]);
    assert_eq!((a.name.as_str(), a.copies), ("Colour", 2));
    assert_eq!(read(&index, "A.RAF").fields, a);
    let b = index.photos[1].fields();
    assert_eq!(b.value("dc:title"), Some(title("Morning market, Alfama")));
    assert_eq!(
        b.value("apertrust:notes"),
        Some(FieldValue::Text("Try a warmer crop".to_owned()))
    );
    assert_eq!(
        b.value(COPY_NAME),
        Some(FieldValue::Text("Black and white".to_owned()))
    );
    assert_eq!(*index.photos[2].fields(), Default::default());
}
