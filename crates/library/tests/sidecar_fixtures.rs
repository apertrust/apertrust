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
//! wrote is the same bytes after the text layer has read and written it. Version 1's are
//! in `tests/fixtures`, version 2's with a history in `tests/fixtures/xmp`, which ExifTool
//! and exiv2 read too.

use std::path::{Path, PathBuf};
use std::process::Command;

use apertrust_library::sidecar::{COPIES_VERSION, Document, Edits, Note, VirtualCopy};

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
        assert_eq!(document.version, 1, "{name}");
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

/// The version 2 fixture: one virtual copy with five steps, the end on the fourth, so the
/// last is greyed; each source, a note in words, a message with its arguments, a batch, a
/// switched module on and an unowned operation's entry; two machines' clocks.
fn history_fixture() -> (PathBuf, String) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/xmp/version-2-history.apxmp");
    let text = std::fs::read_to_string(&path).unwrap();
    (path, text)
}

#[test]
fn the_version_2_fixture_writes_back_byte_for_byte() {
    let (_, text) = history_fixture();
    let edits = Edits::parse(&text, 0).unwrap();
    assert_eq!(edits.to_xml(), text);
    assert_eq!(edits.version, COPIES_VERSION);
    assert_eq!(edits.build.as_deref(), Some("0.5.0+3~g1a2b3c4"));
    assert_eq!(edits.photo.as_deref(), Some("DSCF0001.RAF"));
    assert!(
        edits
            .fingerprint
            .as_deref()
            .is_some_and(|f| f.starts_with("30482117:"))
    );
    // This build's version, and nothing in it this build cannot read.
    assert_eq!(edits.newer(), None);
    assert!(edits.problems.is_empty(), "{:?}", edits.problems);
    assert!(edits.edited());
    let [copy] = edits.copies.as_slice() else {
        panic!("one copy, not {:?}", edits.copies);
    };
    assert_eq!((copy.id.as_str(), copy.number.as_str()), ("7c0e9a14", "0"));
    let sources: Vec<_> = copy.history.iter().map(|s| s.source.as_str()).collect();
    assert_eq!(sources, ["you", "auto", "assistant", "style", "you"]);
    assert_eq!(copy.end, copy.history[3].id);
    assert_eq!(
        copy.history[3].note,
        Some(Note::Message {
            id: "history-style".to_owned(),
            args: vec![
                ("style".to_owned(), "Film grain & fade".to_owned()),
                ("id".to_owned(), "film-grain-fade".to_owned()),
                ("modules".to_owned(), "2".to_owned()),
            ],
        })
    );
    assert_eq!(copy.history[2].batch.as_deref(), Some("0f1e2d3c4b5a6978"));
    assert!(edits.problems.is_empty(), "{:?}", edits.problems);
    // The clock vector holds each machine's latest clock, in the order of their machines:
    // the first machine's is past its last step, an Undo's.
    let machines: Vec<_> = copy.clocks.iter().map(|c| c.machine.to_string()).collect();
    assert_eq!(machines, ["0a1b2c3d", "5e1f22c9"]);
    for clock in &copy.clocks {
        let last = copy
            .history
            .iter()
            .filter_map(|s| s.clock)
            .filter(|c| c.machine == clock.machine);
        assert!(last.max().is_some_and(|last| last <= *clock), "{clock:?}");
    }
}

/// Runs `program` with `args` and the fixture: `None` where it is not installed, which
/// skips its check, else its standard output, after checking it ended well and said
/// nothing on its standard error.
fn read_with(program: &str, args: &[&str], fixture: &Path) -> Option<String> {
    let output = match Command::new(program).args(args).arg(fixture).output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("{program} is not installed: its check of the fixture is skipped");
            return None;
        }
        Err(error) => panic!("{program}: {error}"),
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{program}: {stderr}");
    assert!(stderr.trim().is_empty(), "{program}: {stderr}");
    Some(String::from_utf8(output.stdout).unwrap())
}

/// Other XMP readers read the version 2 fixture whole, without error or warning, where
/// they are installed, and skip it where not. Checked with ExifTool 13.55
/// (`exiftool -a -G1 -struct`), which reads each `apertrust:when` as a date and writes it
/// in its own form, and exiv2 0.28.9 (`exiv2 -px`), which lists every field. The style's
/// step has an argument named `id`, which ExifTool, naming a struct's fields by their
/// local names alone, keeps apart from the step's own id, as the arguments are a struct
/// of their own.
#[test]
fn exiftool_and_exiv2_read_the_version_2_fixture() {
    let (path, _) = history_fixture();
    if let Some(read) = read_with("exiftool", &["-a", "-G1", "-struct"], &path) {
        assert!(
            !read.contains("Warning") && !read.contains("Error"),
            "{read}"
        );
        for field in [
            "[XMP-apertrust] Fingerprint",
            "Id=7c0e9a14",
            "Id=5f4e3d2c1b0a9988",
            "Id=3c2b1a0f9e8d7c6b,Machine=0a1b2c3d,NoteArgs={Id=film-grain-fade,Modules=2,\
             Style=Film grain & fade},NoteMessage=history-style",
            "When=2026:03:14 14:02:11.120Z",
            "When=2026:03:15 09:31:40.000Z",
        ] {
            assert!(read.contains(field), "{field} in {read}");
        }
    }
    if let Some(read) = read_with("exiv2", &["-px"], &path) {
        for field in [
            "Xmp.apertrust.fingerprint",
            "Xmp.apertrust.copies[1]/apertrust:clocks[2]/apertrust:machine",
            "Xmp.apertrust.copies[1]/apertrust:history[5]/apertrust:id",
            "apertrust:history[4]/apertrust:noteArgs/arg:id",
            "apertrust:history[1]/apertrust:entries[2]/param:clip",
            "2026-03-14T14:02:11.120Z",
        ] {
            assert!(read.contains(field), "{field} in {read}");
        }
    }
}

/// A copy at Original, with no clock and no step, as the text layer writes it: its empty
/// arrays closed at once, ExifTool reads no text in them.
#[test]
fn exiftool_reads_no_text_in_a_copy_at_original() {
    let mut edits = Edits::new("0.5.0+3~g1a2b3c4");
    edits.photo = Some("DSCF0001.RAF".to_owned());
    edits.copies.push(VirtualCopy {
        id: "7c0e9a14".to_owned(),
        number: "0".to_owned(),
        name: String::new(),
        end: String::new(),
        clocks: Vec::new(),
        history: Vec::new(),
    });
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("DSCF0001.RAF.apxmp");
    std::fs::write(&path, edits.to_xml()).unwrap();
    if let Some(read) = read_with("exiftool", &["-j", "-a", "-G1"], &path) {
        assert!(!read.contains("Warning"), "{read}");
        assert!(!read.contains("\\n"), "whitespace read as text: {read}");
        for empty in ["CopiesClocks", "CopiesHistory"] {
            let field = format!(r#""XMP-apertrust:{empty}": """#);
            assert!(read.contains(&field), "{field} in {read}");
        }
        assert!(
            read.contains(r#""XMP-apertrust:CopiesId": "7c0e9a14""#),
            "{read}"
        );
    }
}
