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

//! Preset and style files (decision 106): the fixtures in `tests/fixtures/presets` read and
//! write back byte for byte, a folder's files listed, written, renamed and deleted, and
//! names made into file names.

use std::path::{Path, PathBuf};

use apertrust_library::presets::{
    self, FORMAT_VERSION, Kind, PresetError, SETTING_NAMESPACE, Stored, file_name,
};
use apertrust_library::sidecar::{Entry, Newer, Problem};

/// The build the fixtures record.
const BUILD: &str = "0.1.0+4~g1a2b3c4";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/presets")
}

fn tool(tool: &str, version: &str, controls: &[(&str, &str)]) -> Entry {
    Entry::Tool {
        tool: tool.to_owned(),
        version: Some(version.to_owned()),
        kept: Vec::new(),
        auto: Vec::new(),
        enabled: None,
        controls: controls
            .iter()
            .map(|(id, text)| ((*id).to_owned(), (*text).to_owned()))
            .collect(),
    }
}

/// A tool's preset with every field of a tool entry, and a description.
fn bright_interiors() -> Stored {
    let mut stored = Stored::new(Kind::Preset, "Bright interiors");
    stored.description = Some("Lifts a room \"lit\" by a window & lamps".to_owned());
    let mut entry = tool(
        "exposure",
        "3",
        &[("exposure", "1.2"), ("black_level", "-0.01")],
    );
    if let Entry::Tool {
        kept,
        auto,
        enabled,
        ..
    } = &mut entry
    {
        *kept = vec!["exposure".to_owned(), "black_level".to_owned()];
        *auto = vec!["compensate_camera".to_owned()];
        *enabled = Some("true".to_owned());
    }
    stored.entries.push(entry);
    stored
}

/// A utility's preset: Export's id and settings, no tool entry.
fn for_the_web() -> Stored {
    let mut stored = Stored::new(Kind::Preset, "For the web");
    stored.settings_tool = Some("export".to_owned());
    stored.settings = [
        ("export.format", "jpeg"),
        ("export.jpeg.quality", "85"),
        ("export.size.width", "2048"),
    ]
    .iter()
    .map(|(key, text)| ((*key).to_owned(), (*text).to_owned()))
    .collect();
    stored
}

/// A style of two tools, one turned off as it applies: an entry with no
/// `apertrust:enabled` and no values, as the `.apxmp` writes a switched tool off.
fn film_look() -> Stored {
    let mut stored = Stored::new(Kind::Style, "Film look");
    stored
        .entries
        .push(tool("grain", "2", &[("strength", "0.25")]));
    stored.entries.push(tool("vignette", "1", &[]));
    stored
}

fn read_fixture(name: &str) -> (Kind, String) {
    let path = fixtures().join(name);
    let kind = Kind::of(&path).unwrap();
    (kind, std::fs::read_to_string(path).unwrap())
}

#[test]
fn every_fixture_writes_back_byte_for_byte() {
    for (name, expected) in [
        ("bright-interiors.appreset", bright_interiors()),
        ("for-the-web.appreset", for_the_web()),
        ("film-look.apstyle", film_look()),
    ] {
        let (kind, text) = read_fixture(name);
        let read = presets::parse(&text, kind).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(read, expected, "{name}");
        assert_eq!(read.newer(), None, "{name}");
        assert_eq!(read.to_xml(BUILD), text, "{name} written back");
        assert_eq!(expected.to_xml(BUILD), text, "{name} written");
    }
}

#[test]
fn settings_are_in_their_own_namespace() {
    let (_, text) = read_fixture("for-the-web.appreset");
    assert!(text.contains(&format!("xmlns:setting=\"{SETTING_NAMESPACE}\"")));
    assert!(text.contains("setting:export.jpeg.quality=\"85\""));
    // A tool's preset declares no setting namespace.
    assert!(!bright_interiors().to_xml(BUILD).contains("xmlns:setting"));
}

#[test]
fn a_tool_entry_is_the_bytes_an_apxmp_writes() {
    let mut document = apertrust_library::sidecar::Document::new(BUILD);
    document.entries = bright_interiors().entries;
    let edit = document.to_xml();
    let (_, preset) = read_fixture("bright-interiors.appreset");
    let entry = |text: &str| {
        let start = text.find("<apertrust:tools>").unwrap();
        let end = text.find("</apertrust:tools>").unwrap();
        text[start..end].to_owned()
    };
    assert_eq!(entry(&preset), entry(&edit));
}

#[test]
fn a_newer_file_is_read_and_parked() {
    let text = bright_interiors()
        .to_xml(BUILD)
        .replace("apertrust:preset=\"1\"", "apertrust:preset=\"2\"");
    let read = presets::parse(&text, Kind::Preset).unwrap();
    assert_eq!(read.version, 2);
    assert_eq!(read.entries, bright_interiors().entries);
    assert_eq!(
        read.newer(),
        Some(Newer::FormatVersion {
            version: 2,
            has: FORMAT_VERSION
        })
    );
    assert_eq!(
        read.newer().unwrap().to_string(),
        "is version 2, newer than this build's 1"
    );
}

#[test]
fn an_unknown_field_parks_the_file() {
    let text = bright_interiors().to_xml(BUILD);
    for (unknown, place, field) in [
        (
            text.replace(
                "apertrust:name=\"Bright interiors\"",
                "apertrust:name=\"Bright interiors\"\n    apertrust:colour=\"red\"",
            ),
            "the file",
            "apertrust:colour",
        ),
        (
            text.replace(
                "apertrust:tool=\"exposure\"",
                "apertrust:tool=\"exposure\"\n        apertrust:mask=\"m1\"",
            ),
            "the file's tools",
            "apertrust:mask",
        ),
        // At the root, a field of another namespace too.
        (
            text.replace(
                "apertrust:preset=\"1\"",
                "xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n    xmp:Rating=\"3\"\n    apertrust:preset=\"1\"",
            ),
            "the file",
            "xmp:Rating",
        ),
    ] {
        let read = presets::parse(&unknown, Kind::Preset).unwrap();
        assert_eq!(read.entries, bright_interiors().entries, "{field}");
        let problem = Problem::UnknownField {
            place: place.to_owned(),
            field: field.to_owned(),
        };
        assert_eq!(read.newer(), Some(Newer::Field(problem.clone())));
        assert_eq!(read.problems, [problem]);
    }
    // In a utility's settings, a field other than its tool, once, and the settings.
    let (_, web) = read_fixture("for-the-web.appreset");
    for (unknown, field) in [
        (
            web.replace(
                "apertrust:tool=\"export\"",
                "apertrust:tool=\"export\"\n      apertrust:colour=\"red\"",
            ),
            "apertrust:colour",
        ),
        (
            web.replace(
                "apertrust:tool=\"export\"",
                "apertrust:tool=\"export\"\n      xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n      dc:source=\"x\"",
            ),
            "dc:source",
        ),
    ] {
        let read = presets::parse(&unknown, Kind::Preset).unwrap();
        assert_eq!(read.settings_tool.as_deref(), Some("export"));
        let problem = Problem::UnknownField {
            place: "the file's settings".to_owned(),
            field: field.to_owned(),
        };
        assert_eq!(read.problems, [problem], "{field}");
    }

    // Parked, it is neither replaced nor renamed, and its name is refused.
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path();
    let path = presets::write(dir, &bright_interiors(), BUILD).unwrap();
    let parked = text.replace(
        "apertrust:name=\"Bright interiors\"",
        "apertrust:name=\"Bright interiors\"\n    apertrust:colour=\"red\"",
    );
    std::fs::write(&path, &parked).unwrap();
    assert!(matches!(
        presets::rename(&path, "Other"),
        Err(PresetError::Newer(Newer::Field(_)))
    ));
    assert!(matches!(
        presets::write(dir, &bright_interiors(), BUILD),
        Err(PresetError::Newer(Newer::Field(_)))
    ));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), parked);
    assert_eq!(file_names(dir), ["Bright interiors.appreset"]);
    // Nor is a parked file's own values ever written.
    let mut newer = bright_interiors();
    newer.name = "Elsewhere".to_owned();
    newer.version = 2;
    assert!(matches!(
        presets::write(dir, &newer, BUILD),
        Err(PresetError::Newer(Newer::FormatVersion { version: 2, .. }))
    ));
}

#[test]
fn what_is_no_preset_is_refused() {
    let style = film_look().to_xml(BUILD);
    assert!(matches!(
        presets::parse(&style, Kind::Preset),
        Err(PresetError::NotAPreset)
    ));
    assert!(matches!(
        presets::parse("<x:xmpmeta", Kind::Style),
        Err(PresetError::Xml(_))
    ));
    let version = style.replace("apertrust:style=\"1\"", "apertrust:style=\"one\"");
    assert!(matches!(
        presets::parse(&version, Kind::Style),
        Err(PresetError::Version(v)) if v == "one"
    ));
    let nameless = style.replace("apertrust:name=\"Film look\"", "apertrust:name=\" \"");
    assert!(matches!(
        presets::parse(&nameless, Kind::Style),
        Err(PresetError::EmptyName)
    ));
}

#[test]
fn names_become_file_names() {
    assert_eq!(file_name("A/B: c?"), "A_B_ c_");
    assert_eq!(file_name(" lead."), "lead");
    assert_eq!(file_name("\\*\"<>|\ttab"), "_______tab");
    assert_eq!(file_name("..."), "_");
    assert_eq!(file_name(" . "), "_");
    // Over 200 bytes, cut at a character's boundary: each "é" is two bytes, so 200 bytes
    // hold 100 of them, and 199 bytes of "a" then one more "é" would end mid-character.
    assert_eq!(file_name(&"é".repeat(150)), "é".repeat(100));
    let cut = file_name(&format!("{}{}", "a".repeat(199), "é".repeat(5)));
    assert_eq!(cut, "a".repeat(199));
    // A cut that ends on a space or dot is trimmed again.
    assert_eq!(
        file_name(&format!("{} .x", "a".repeat(198))),
        "a".repeat(198)
    );
    // Windows' device names, in any case, alone or before a dot, take a `_` after them.
    for (name, file) in [
        ("CON", "CON_"),
        ("nul", "nul_"),
        ("Aux.txt", "Aux_.txt"),
        ("prn .old", "prn_ .old"),
        ("com1", "com1_"),
        ("LPT9.a.b", "LPT9_.a.b"),
        ("COM0", "COM0"),
        ("CONSOLE", "CONSOLE"),
        ("LPT10", "LPT10"),
        ("My CON", "My CON"),
    ] {
        assert_eq!(file_name(name), file, "{name}");
    }
}

#[test]
fn a_windows_hostile_name_writes_a_valid_file_name() {
    let folder = tempfile::tempdir().unwrap();
    let mut stored = bright_interiors();
    stored.name = "Room: <bright>/\"warm\"?*|.".to_owned();
    let path = presets::write(folder.path(), &stored, BUILD).unwrap();
    assert_eq!(
        path.file_name().unwrap(),
        "Room_ _bright___warm____.appreset"
    );
    let listed = presets::list(folder.path(), Kind::Preset);
    assert_eq!(listed.files.len(), 1);
    assert_eq!(listed.files[0].name, stored.name);
}

fn names(listed: &presets::Listed) -> Vec<&str> {
    listed.files.iter().map(|s| s.name.as_str()).collect()
}

fn file_names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn a_folder_s_files_are_listed_in_the_browser_s_order() {
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path();
    for name in ["Preset 10", "preset 9", "Bright interiors"] {
        let mut stored = bright_interiors();
        stored.name = name.to_owned();
        presets::write(dir, &stored, BUILD).unwrap();
    }
    presets::write(dir, &film_look(), BUILD).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a preset").unwrap();
    std::fs::write(dir.join(".hidden.appreset"), "not read").unwrap();
    std::fs::write(dir.join("Broken.appreset"), "<x:xmpmeta").unwrap();
    std::fs::create_dir(dir.join("Folder.appreset")).unwrap();
    let listed = presets::list(dir, Kind::Preset);
    assert_eq!(
        names(&listed),
        ["Bright interiors", "preset 9", "Preset 10"]
    );
    assert!(listed.files.iter().all(|s| s.path.parent() == Some(dir)));
    let [(broken, PresetError::Xml(_))] = listed.failed.as_slice() else {
        panic!("one broken file, not {:?}", listed.failed);
    };
    assert_eq!(broken, &dir.join("Broken.appreset"));
    assert_eq!(names(&presets::list(dir, Kind::Style)), ["Film look"]);
    // A folder that does not exist yet holds nothing.
    let missing = presets::list(&dir.join("missing"), Kind::Preset);
    assert!(missing.files.is_empty() && missing.failed.is_empty());
}

#[test]
fn a_name_s_file_is_replaced_where_it_is_once_its_first_file_name_is_free() {
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path();
    let mut first = bright_interiors();
    first.name = "A/B".to_owned();
    let a = presets::write(dir, &first, BUILD).unwrap();
    let mut second = bright_interiors();
    second.name = "A:B".to_owned();
    let b = presets::write(dir, &second, BUILD).unwrap();
    presets::delete(&a).unwrap();
    second.description = None;
    assert_eq!(presets::write(dir, &second, BUILD).unwrap(), b);
    assert_eq!(file_names(dir), ["A_B 2.appreset"]);
}

#[test]
fn write_names_rename_and_delete_a_folder_s_files() {
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path().join("presets/exposure");
    // The folder is made, and the file named by its name.
    let mut first = bright_interiors();
    first.name = "A/B".to_owned();
    let a = presets::write(&dir, &first, BUILD).unwrap();
    assert_eq!(a, dir.join("A_B.appreset"));
    // Another name with the same file name is numbered.
    let mut second = bright_interiors();
    second.name = "A:B".to_owned();
    let b = presets::write(&dir, &second, BUILD).unwrap();
    assert_eq!(b, dir.join("A_B 2.appreset"));
    // The same name again replaces its own file.
    second.description = None;
    assert_eq!(presets::write(&dir, &second, BUILD).unwrap(), b);
    let read = presets::list(&dir, Kind::Preset);
    assert_eq!(names(&read), ["A/B", "A:B"]);
    assert_eq!(read.files[1].description, None);
    // A file a newer Apertrust wrote is never replaced, nor its name taken by another.
    let newer = second
        .to_xml(BUILD)
        .replace("apertrust:preset=\"1\"", "apertrust:preset=\"2\"");
    std::fs::write(&b, &newer).unwrap();
    assert!(matches!(
        presets::write(&dir, &second, BUILD),
        Err(PresetError::Newer(Newer::FormatVersion { version: 2, .. }))
    ));
    assert_eq!(std::fs::read_to_string(&b).unwrap(), newer);
    // Nor renamed.
    assert!(matches!(
        presets::rename(&b, "Other"),
        Err(PresetError::Newer(Newer::FormatVersion { version: 2, .. }))
    ));
    assert_eq!(file_names(&dir), ["A_B 2.appreset", "A_B.appreset"]);
    std::fs::write(&b, second.to_xml(BUILD)).unwrap();

    // A name another file holds is refused, its spaces at both ends trimmed first; an
    // empty one too.
    assert!(matches!(
        presets::rename(&a, " A:B  "),
        Err(PresetError::NameTaken(name)) if name == "A:B"
    ));
    assert!(matches!(
        presets::rename(&a, "  "),
        Err(PresetError::EmptyName)
    ));
    // Renamed: its name rewritten, everything else kept, and its file renamed.
    let renamed = presets::rename(&a, "Warm rooms").unwrap();
    assert_eq!(renamed, dir.join("Warm rooms.appreset"));
    assert!(!a.exists());
    let read = presets::list(&dir, Kind::Preset);
    assert_eq!(names(&read), ["A:B", "Warm rooms"]);
    let mut expected = bright_interiors();
    expected.name = "Warm rooms".to_owned();
    expected.path = renamed.clone();
    assert_eq!(read.files[1], expected);
    // The first free file name is taken: `A_B.appreset` is free again.
    let numbered = presets::rename(&renamed, "A?B").unwrap();
    assert_eq!(numbered, dir.join("A_B.appreset"));
    // A change of case alone renames the file in place.
    let cased = presets::rename(&b, "a:b").unwrap();
    assert_eq!(cased, dir.join("a_b 2.appreset"));
    assert_eq!(file_names(&dir), ["A_B.appreset", "a_b 2.appreset"]);

    presets::delete(&cased).unwrap();
    presets::delete(&numbered).unwrap();
    assert!(file_names(&dir).is_empty());
    assert!(matches!(
        presets::delete(&numbered),
        Err(PresetError::Io(_))
    ));
}

#[test]
fn a_rename_onto_a_file_name_taken_is_numbered_and_keeps_the_file() {
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path();
    let mut first = bright_interiors();
    first.name = "A/B".to_owned();
    presets::write(dir, &first, BUILD).unwrap();
    let mut other = bright_interiors();
    other.name = "C".to_owned();
    let c = presets::write(dir, &other, BUILD).unwrap();
    let before = std::fs::read_to_string(&c).unwrap();
    let renamed = presets::rename(&c, " A:B ").unwrap();
    assert_eq!(renamed, dir.join("A_B 2.appreset"));
    // Only `apertrust:name` changed, to the name trimmed.
    assert_eq!(
        std::fs::read_to_string(&renamed).unwrap(),
        before.replace("apertrust:name=\"C\"", "apertrust:name=\"A:B\"")
    );
}

#[test]
fn a_name_is_trimmed_and_one_that_leaves_no_file_name_is_written_as_underscore() {
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path();
    let mut spaced = bright_interiors();
    spaced.name = "  Spaced ".to_owned();
    let path = presets::write(dir, &spaced, BUILD).unwrap();
    assert_eq!(path, dir.join("Spaced.appreset"));
    assert_eq!(names(&presets::list(dir, Kind::Preset)), ["Spaced"]);
    // The same name, trimmed, replaces it.
    spaced.name = "Spaced".to_owned();
    assert_eq!(presets::write(dir, &spaced, BUILD).unwrap(), path);
    let mut dots = bright_interiors();
    dots.name = "...".to_owned();
    assert_eq!(
        presets::write(dir, &dots, BUILD).unwrap(),
        dir.join("_.appreset")
    );
    dots.name = " ".to_owned();
    assert!(matches!(
        presets::write(dir, &dots, BUILD),
        Err(PresetError::EmptyName)
    ));
}

/// A name the file system takes for another's file, as APFS takes `Cafe\u{301}` and
/// `Caf\u{e9}` for one, is numbered rather than written over it.
#[test]
fn a_file_name_the_file_system_takes_for_another_is_numbered() {
    let folder = tempfile::tempdir().unwrap();
    let dir = folder.path();
    let mut decomposed = bright_interiors();
    decomposed.name = "Cafe\u{301}".to_owned();
    let first = presets::write(dir, &decomposed, BUILD).unwrap();
    let mut composed = bright_interiors();
    composed.name = "Caf\u{e9}".to_owned();
    let second = presets::write(dir, &composed, BUILD).unwrap();
    assert_ne!(first, second);
    let listed = presets::list(dir, Kind::Preset);
    assert_eq!(listed.files.len(), 2, "{:?}", names(&listed));
}

/// The built-in presets: Export's two, in their order, each an Export preset of settings
/// with nothing a write would drop, in the one layout, and its British English name.
#[test]
fn the_built_in_presets_are_export_s_two_written_in_the_one_layout() {
    let builtins = presets::builtins("export");
    let ids: Vec<&str> = builtins.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["for-sharing", "full-size-jpeg"]);
    let names: Vec<&str> = builtins.iter().map(|(_, s)| s.name.as_str()).collect();
    assert_eq!(names, ["For sharing", "Full size JPEG"]);
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/presets/export");
    for (id, stored) in &builtins {
        assert_eq!(stored.settings_tool.as_deref(), Some("export"), "{id}");
        assert!(stored.entries.is_empty(), "{id}");
        assert_eq!(stored.newer(), None, "{id}");
        let text = std::fs::read_to_string(data.join(format!("{id}.appreset"))).unwrap();
        assert_eq!(stored.to_xml("0.1.0"), text, "{id}");
    }
    let setting = |id: usize, key: &str| {
        let settings = &builtins[id].1.settings;
        settings
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(setting(0, "export.size.width").as_deref(), Some("2048"));
    assert_eq!(setting(1, "export.size.width").as_deref(), Some("0"));
    assert!(presets::builtins("exposure").is_empty());
}
