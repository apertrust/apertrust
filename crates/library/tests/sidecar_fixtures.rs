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

//! Every committed `.apxmp` fixture reads and writes back byte for byte: a file the format
//! wrote is the same bytes after the text layer has read and written it.

use std::path::Path;

use apertrust_library::sidecar::{Document, VERSION};

fn fixtures() -> Vec<(String, String)> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut found: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "apxmp"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).unwrap())
        })
        .collect();
    found.sort();
    found
}

#[test]
fn every_fixture_writes_back_byte_for_byte() {
    let fixtures = fixtures();
    assert!(!fixtures.is_empty(), "no fixture in tests/fixtures");
    for (name, text) in fixtures {
        let document = Document::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(document.version, VERSION, "{name}");
        assert_eq!(document.newer(), None, "{name}");
        assert_eq!(document.to_xml(), text, "{name} written back");
    }
}

#[test]
fn the_version_1_fixture_holds_the_engine_s_edit() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/version-1.apxmp"),
    )
    .unwrap();
    let document = Document::parse(&text).unwrap();
    assert_eq!(document.build.as_deref(), Some("0.4.0+12~g1a2b3c4"));
    assert_eq!(document.entries.len(), 2);
}
