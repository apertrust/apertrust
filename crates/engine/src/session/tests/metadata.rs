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

//! The open photo's Metadata fields and its copy's name (decision 110): `SetMetadata` and
//! `RenameCopy` set them beside the edit, never a step, Undo takes them back in the order
//! changes were made, and the write leaves the log as the file holds it.

use apertrust_library::fields::{self, FieldValue};
use apertrust_tools::colour_calibration::ColourCalibration;
use apertrust_tools::exposure::Exposure;
use apertrust_tools::white_balance::WhiteBalance;

use super::*;
use crate::session::history::MERGE_WITHIN;

/// Tools with default steps, so a photo opened is given steps a write could drop in.
const DEFAULTS: &[&dyn ProcessingTool] = &[&WhiteBalance, &Exposure, &ColourCalibration];

const TITLE: &str = "dc:title";
const CITY: &str = "photoshop:City";

fn set_field(field: &str, typed: Option<&str>) -> Command {
    Command::SetMetadata {
        fields: vec![(field.to_owned(), typed.map(str::to_owned))],
    }
}

fn alt(text: &str) -> FieldValue {
    FieldValue::LangAlt(vec![("x-default".to_owned(), text.to_owned())])
}

/// The turn's `Metadata` fact, if it sent one.
fn told(events: &[Event]) -> Option<&MetadataState> {
    events.iter().rev().find_map(|event| match event {
        Event::Metadata(state) => Some(state),
        _ => None,
    })
}

/// The value `field` holds in the fact `state`.
fn shown(state: &MetadataState, field: &str) -> Option<FieldValue> {
    (state.fields.iter())
        .find(|(kept, _)| kept == field)
        .and_then(|(_, value)| value.clone())
}

/// What Undo would do, as the turn's `History` says.
fn undo(events: &[Event]) -> Option<Move> {
    events.iter().rev().find_map(|event| match event {
        Event::History(history) => Some(history.undo),
        _ => None,
    })?
}

/// The photo's `.apxmp` as written, once the writer is done.
fn file(driven: &Driven, path: &Path) -> sidecar::Edits {
    driven.writes();
    let xml = std::fs::read_to_string(sidecar::path_for(path)).unwrap();
    sidecar::Edits::parse(&xml, 0).unwrap()
}

/// A title on a photo never edited writes `dc:title` with a clock in the copy's fields, the
/// vector advanced with it, and no step: the default steps stay unwritten and the photo
/// unedited. `Metadata` tells it, `History` names the Undo, and no `Values` comes.
#[test]
fn a_title_is_written_beside_the_log_and_no_step() {
    let path = scratch("metadata-title");
    let mut driven = Driven::new(DEFAULTS);
    let opened = driven.open(&path);
    let state = told(&opened).expect("told as the photo opens");
    assert_eq!(state.fields, []);
    assert_eq!(state.copy.count, 1);
    assert!(!state.darktable);
    let events = driven.run([set_field(TITLE, Some("  Morning market  "))]);
    assert_eq!(
        shown(told(&events).unwrap(), TITLE),
        Some(alt("Morning market"))
    );
    assert!(!events.iter().any(|e| matches!(e, Event::Values(_))));
    assert_eq!(
        undo(&events),
        Some(Move::Metadata {
            field: Some(TITLE),
            photos: 1
        })
    );
    let written = file(&driven, &path);
    let copy = &written.copies[0];
    assert!(copy.history.is_empty(), "no default step written");
    let [entry] = copy.fields.as_slice() else {
        panic!("{:?}", copy.fields);
    };
    assert_eq!(entry.value, Some(alt("Morning market")));
    assert!(copy.clocks.contains(&entry.clock.unwrap()));
    let bytes = std::fs::read(sidecar::path_for(&path)).unwrap();
    assert_eq!(library::edit_hash(&bytes), library::UNALTERED);
    // The same value again, a space after it, or a copy's name that is no change, commits
    // nothing, and is answered as saved, so a client waiting for its write stops.
    driven.writes();
    let file = sidecar::path_for(&path);
    for command in [
        set_field(TITLE, Some("Morning market ")),
        Command::RenameCopy {
            name: " ".to_owned(),
        },
    ] {
        let again = driven.run([command]);
        assert!(told(&again).is_none());
        assert_eq!(super::written(&driven.writes()), [(file.as_path(), true)]);
    }
    remove(&path);
}

/// Undo writes the value before back with a later clock, Redo the one after; commits of one
/// field within the merge time are one Undo, a clear among them.
#[test]
fn undo_puts_a_field_back_and_quick_commits_merge() {
    let path = scratch("metadata-undo");
    let mut driven = Driven::new(DEFAULTS);
    driven.open(&path);
    let at = Instant::now();
    driven.run_at(at, [set_field(TITLE, Some("A"))]);
    driven.run_at(at + Duration::from_secs(1), [set_field(TITLE, Some("B"))]);
    let later = at + MERGE_WITHIN + Duration::from_secs(2);
    driven.run_at(later, [set_field(TITLE, None)]);
    let clock = |driven: &Driven| file(driven, &path).copies[0].fields[0].clock;
    let before = clock(&driven);
    let undone = driven.run([Command::Undo]);
    assert_eq!(shown(told(&undone).unwrap(), TITLE), Some(alt("B")));
    assert!(clock(&driven) > before, "a later clock");
    // A and B were one change.
    let undone = driven.run([Command::Undo]);
    assert_eq!(shown(told(&undone).unwrap(), TITLE), None);
    assert_eq!(undo(&undone), None);
    let redone = driven.run([Command::Redo]);
    assert_eq!(shown(told(&redone).unwrap(), TITLE), Some(alt("B")));
    let copy = &file(&driven, &path).copies[0];
    assert_eq!(fields::value(&copy.fields, TITLE), Some(alt("B")));
    assert!(copy.history.is_empty());
    remove(&path);
}

/// A field and the edit's changes undo in the order they were made, and Reset and Discard
/// History leave the fields as they are.
#[test]
fn undo_takes_the_edit_and_a_field_back_in_order_and_reset_keeps_it() {
    let path = scratch("metadata-order");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set_field(CITY, Some("Lisbon"))]);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    for command in [Command::Reset, Command::DiscardHistory] {
        let events = driven.run([command]);
        assert!(told(&events).is_none(), "the fields as they were");
    }
    let held = |driven: &Driven| {
        let open = driven.engine.open.as_ref().unwrap();
        fields::value(open.fields(), CITY)
    };
    assert_eq!(held(&driven), Some(FieldValue::Text("Lisbon".to_owned())));
    // Discard History, Reset and the brightness, then the city.
    for _ in 0..3 {
        assert!(told(&driven.run([Command::Undo])).is_none());
    }
    let undone = driven.run([Command::Undo]);
    assert_eq!(shown(told(&undone).unwrap(), CITY), None);
    remove(&path);
}

/// A newer file taken in with another field set joins both: its field, and the session's
/// not yet written.
#[test]
fn a_newer_file_s_fields_join_the_session_s() {
    let path = scratch("metadata-newer");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set_field(TITLE, Some("Ours"))]);
    let mut elsewhere = file(&driven, &path);
    let last = elsewhere.copies[0].fields[0].clock.unwrap();
    let machine = "00000000".parse().unwrap();
    let clock = apertrust_library::clock::Clock::at(last.when + 60_000, None, machine);
    let city = Some(FieldValue::Text("Porto".to_owned()));
    fields::set(&mut elsewhere.copies[0].fields, CITY, city.clone(), clock);
    crate::history::advance(&mut elsewhere.copies[0].clocks, clock);
    std::fs::write(sidecar::path_for(&path), elsewhere.to_xml()).unwrap();
    driven.run([set_field("photoshop:Country", Some("Portugal"))]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    let state = told(&taken).expect("the join told");
    assert_eq!(shown(state, CITY), city);
    assert_eq!(shown(state, TITLE), Some(alt("Ours")));
    assert_eq!(
        shown(state, "photoshop:Country"),
        Some(FieldValue::Text("Portugal".to_owned()))
    );
    remove(&path);
}

/// A field none of `FIELDS` is, the rating's, or a value its field cannot hold refuses the
/// whole command, changing nothing; a photo open for viewing only refuses it too.
#[test]
fn unknown_fields_bad_values_and_view_only_photos_are_refused() {
    let path = scratch("metadata-refused");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let both = Command::SetMetadata {
        fields: vec![
            (TITLE.to_owned(), Some("Kept out".to_owned())),
            ("dc:subject".to_owned(), Some("harbour".to_owned())),
        ],
    };
    for (command, refused) in [
        (both, "UnknownField"),
        (set_field(fields::RATING, Some("3")), "UnknownField"),
        (
            set_field("photoshop:DateCreated", Some("4 July")),
            "FieldValue",
        ),
    ] {
        let events = driven.run([command]);
        let failed = format!("{:?}", failures(&events));
        assert!(failed.contains(refused), "{failed}");
        assert!(told(&events).is_none());
    }
    let open = driven.engine.open.as_ref().unwrap();
    assert!(open.fields().is_empty());
    remove(&path);
    let path = scratch("metadata-view-only");
    let newer =
        brightened_log("0.9.0").replace(r#"apertrust:version="2""#, r#"apertrust:version="3""#);
    std::fs::write(sidecar::path_for(&path), &newer).unwrap();
    driven.open(&path);
    for command in [
        set_field(TITLE, Some("A")),
        Command::RenameCopy {
            name: "B".to_owned(),
        },
    ] {
        let refused = driven.run([command]);
        assert!(
            matches!(failures(&refused)[..], [SessionError::ViewOnly(_)]),
            "{refused:?}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(sidecar::path_for(&path)).unwrap(),
        newer
    );
    remove(&path);
}

/// A photo with no `.apxmp` shows darktable's fields and version name as its own, which its
/// first write, of any change, copies in with that write's clock; the `.xmp` is never read
/// for them again nor written.
#[test]
fn darktable_s_fields_are_copied_in_at_the_first_write_then_never_read() {
    let path = scratch("metadata-darktable");
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF
        xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description
        xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:darktable="http://darktable.sf.net/"
        xmlns:acdsee="http://ns.acdsee.com/iptc/1.0/" acdsee:notes="Warmer"
        darktable:version_name="Black and white"><dc:title><rdf:Alt>
        <rdf:li xml:lang="x-default">darktable's title</rdf:li></rdf:Alt></dc:title>
        </rdf:Description></rdf:RDF></x:xmpmeta>"#;
    let darktable = apertrust_library::darktable::path_for(&path);
    std::fs::write(&darktable, xmp).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    let state = told(&opened).unwrap();
    assert!(state.darktable);
    assert_eq!(shown(state, TITLE), Some(alt("darktable's title")));
    assert_eq!(state.copy.name, "Black and white");
    let events = driven.run([set_field("dc:description", Some("Fish stalls"))]);
    assert!(!told(&events).unwrap().darktable);
    let written = file(&driven, &path);
    let copy = &written.copies[0];
    assert_eq!(
        fields::value(&copy.fields, TITLE),
        Some(alt("darktable's title"))
    );
    assert_eq!(
        fields::value(&copy.fields, "apertrust:notes"),
        Some(FieldValue::Text("Warmer".to_owned()))
    );
    assert!(copy.fields.iter().all(|entry| entry.clock.is_some()));
    assert_eq!(copy.name, "Black and white");
    assert!(copy.name_clock.is_some());
    assert_eq!(std::fs::read_to_string(&darktable).unwrap(), xmp);
    // darktable's file changed since is no longer read: the `.apxmp` is the photo's.
    std::fs::write(&darktable, xmp.replace("darktable's title", "Changed")).unwrap();
    let other = scratch("metadata-darktable-other");
    driven.open(&other);
    let reopened = driven.open(&path);
    assert_eq!(
        shown(told(&reopened).unwrap(), TITLE),
        Some(alt("darktable's title"))
    );
    remove(&path);
    remove(&other);
}

/// `RenameCopy` on a photo of two copies renames the open one, trimmed, one Undo, and
/// `Metadata` tells its name, place and count.
#[test]
fn rename_copy_renames_the_open_copy_of_two() {
    let path = scratch("metadata-rename");
    let mut two = sidecar::Edits::parse(&brightened_log("0.9.0"), 0).unwrap();
    let mut second = two.copies[0].clone();
    second.id = "1a2b3c4d".to_owned();
    second.number = "1".to_owned();
    second.name = "Warm".to_owned();
    two.copies.push(second);
    std::fs::write(sidecar::path_for(&path), two.to_xml()).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    let copy = &told(&opened).unwrap().copy;
    assert_eq!((copy.name.as_str(), copy.number, copy.count), ("", 1, 2));
    let events = driven.run([Command::RenameCopy {
        name: " Black and\nwhite ".to_owned(),
    }]);
    assert_eq!(told(&events).unwrap().copy.name, "Black and white");
    assert_eq!(undo(&events), Some(Move::CopyName));
    let written = file(&driven, &path);
    assert_eq!(written.copies[0].name, "Black and white");
    assert!(written.copies[0].name_clock.is_some());
    assert_eq!(written.copies[1].name, "Warm", "the other copy as it was");
    let undone = driven.run([Command::Undo]);
    assert_eq!(told(&undone).unwrap().copy.name, "");
    remove(&path);
}

/// A newer file that changes only the preserved file name, which another program wrote,
/// tells `Metadata` with it.
#[test]
fn a_newer_file_s_preserved_file_name_is_told() {
    use apertrust_library::xmp::{Name, Property};
    let path = scratch("metadata-preserved");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set_field(TITLE, Some("Ours"))]);
    let mut elsewhere = file(&driven, &path);
    let mm = "http://ns.adobe.com/xap/1.0/mm/";
    let name = Name::new("xmpMM", mm, "PreservedFileName");
    elsewhere.other.push(Property::simple(name, "IMG_0001.CR2"));
    std::fs::write(sidecar::path_for(&path), elsewhere.to_xml()).unwrap();
    driven.run([set_field(CITY, Some("Porto"))]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    let state = told(&taken).expect("the preserved file name told");
    assert_eq!(state.preserved_file_name.as_deref(), Some("IMG_0001.CR2"));
    remove(&path);
}
