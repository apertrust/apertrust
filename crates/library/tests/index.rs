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
    assert_eq!(edit.version, sidecar::VERSION);
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
    newer.version = sidecar::VERSION + 1;
    edited(dir.path(), "DSCF0001.RAF", &newer);
    edited(dir.path(), "DSCF0002.RAF", &document("0.5"));

    let index = build(dir.path());

    let edit = read(&index, "DSCF0001.RAF");
    assert_eq!(edit.version, sidecar::VERSION + 1);
    assert_eq!(edit.view_only, Some(Newer::Version(sidecar::VERSION + 1)));
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
    newer.version = sidecar::VERSION + 1;
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
