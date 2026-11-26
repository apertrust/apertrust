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

//! The open photo's rating (decision 110): `Rate` sets its fields beside the edit, never a
//! step, Undo takes it back in the order changes were made, and the write leaves the log
//! as the file holds it.

use apertrust_library::fields::{self, FieldValue};
use apertrust_tools::colour_calibration::ColourCalibration;
use apertrust_tools::exposure::Exposure;
use apertrust_tools::white_balance::WhiteBalance;

use super::*;
use crate::session::history::MERGE_WITHIN;

/// Tools with default steps, so a photo opened is given steps a write could drop in.
const DEFAULTS: &[&dyn ProcessingTool] = &[&WhiteBalance, &Exposure, &ColourCalibration];

fn rate(change: RatingChange) -> Command {
    Command::Rate {
        photos: Vec::new(),
        change,
    }
}

fn rated(stars: u8, rejected: bool, labels: u8) -> Rating {
    Rating {
        stars,
        rejected,
        labels,
    }
}

/// The rating the turn's `Rating` fact gives, checking a `Rated` tells it alike.
fn told(events: &[Event]) -> Option<Rating> {
    let fact = events.iter().find_map(|event| match event {
        Event::Rating(state) => Some(state.rating),
        _ => None,
    });
    let notice = events.iter().find_map(|event| match event {
        Event::Rated { rating, .. } => Some(*rating),
        _ => None,
    });
    if notice.is_some() {
        assert_eq!(fact, notice, "{events:?}");
    }
    fact
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

/// Three stars on a photo never edited write `xmp:Rating` with a clock in the copy's
/// fields, the vector advanced with it, and no step: the default steps stay unwritten, the
/// photo unedited, its thumbnail's version that of no edit. `Rating` and `Rated` say so,
/// and `History` names the Undo; no `Values`.
#[test]
fn stars_are_written_beside_the_log_and_no_step() {
    let path = scratch("rate-stars");
    let mut driven = Driven::new(DEFAULTS);
    let opened = driven.open(&path);
    assert_eq!(told(&opened), Some(Rating::default()));
    let events = driven.run([rate(RatingChange::Stars(3))]);
    assert_eq!(told(&events), Some(rated(3, false, 0)));
    assert!(!events.iter().any(|e| matches!(e, Event::Values(_))));
    assert_eq!(
        undo(&events),
        Some(Move::Field {
            change: RatingChange::Stars(3),
            photos: 1
        })
    );
    let written = file(&driven, &path);
    let copy = &written.copies[0];
    assert!(copy.history.is_empty(), "no default step written");
    let [entry] = copy.fields.as_slice() else {
        panic!("{:?}", copy.fields);
    };
    assert_eq!(entry.field, fields::RATING);
    assert_eq!(entry.value, Some(FieldValue::Stars(3)));
    assert!(copy.clocks.contains(&entry.clock.unwrap()));
    let bytes = std::fs::read(sidecar::path_for(&path)).unwrap();
    assert_eq!(library::edit_hash(&bytes), library::UNALTERED);
    let open = driven.engine.open.as_ref().unwrap();
    assert_eq!(open.thumbnail_hash, None);
    // The same stars again change nothing and write nothing.
    let again = driven.run([rate(RatingChange::Stars(3))]);
    assert_eq!(told(&again), None);
    remove(&path);
}

/// Undo writes the rating before back with a later clock, Redo the one after; a quick
/// repeat of one key is one Undo, a change after the merge time its own.
#[test]
fn undo_puts_the_rating_back_and_quick_repeats_merge() {
    let path = scratch("rate-undo");
    let mut driven = Driven::new(DEFAULTS);
    driven.open(&path);
    let at = Instant::now();
    driven.run_at(at, [rate(RatingChange::Stars(2))]);
    driven.run_at(at + Duration::from_secs(1), [rate(RatingChange::Stars(4))]);
    let later = at + MERGE_WITHIN + Duration::from_secs(2);
    driven.run_at(later, [rate(RatingChange::Rejected(true))]);
    let clock = |driven: &Driven| file(driven, &path).copies[0].fields[0].clock;
    let before = clock(&driven);
    let undone = driven.run([Command::Undo]);
    assert_eq!(told(&undone), Some(rated(4, false, 0)));
    assert!(clock(&driven) > before, "a later clock");
    let undone = driven.run([Command::Undo]);
    assert_eq!(told(&undone), Some(Rating::default()));
    let rating = |driven: &Driven| Rating::of(&file(driven, &path).copies[0].fields);
    assert_eq!(rating(&driven), Rating::default());
    // Nothing more: the photo opened with its default steps alone.
    assert_eq!(undo(&undone), None);
    let redone = driven.run([Command::Redo]);
    assert_eq!(told(&redone), Some(rated(4, false, 0)));
    assert_eq!(rating(&driven), rated(4, false, 0));
    assert!(file(&driven, &path).copies[0].history.is_empty());
    remove(&path);
}

/// Two changes of the stars apart, past the merge times: two Undos take them back to none,
/// each on the value the one after left, and two Redos put them back.
#[test]
fn two_changes_of_one_field_undo_and_redo_in_turn() {
    let path = scratch("rate-twice");
    let mut driven = Driven::new(DEFAULTS);
    driven.open(&path);
    let at = Instant::now();
    driven.run_at(at, [rate(RatingChange::Stars(3))]);
    let later = at + MERGE_WITHIN + Duration::from_secs(2);
    driven.run_at(later, [rate(RatingChange::Stars(4))]);
    assert_eq!(told(&driven.run([Command::Undo])), Some(rated(3, false, 0)));
    assert_eq!(told(&driven.run([Command::Undo])), Some(Rating::default()));
    assert_eq!(told(&driven.run([Command::Redo])), Some(rated(3, false, 0)));
    assert_eq!(told(&driven.run([Command::Redo])), Some(rated(4, false, 0)));
    assert_eq!(
        Rating::of(&file(&driven, &path).copies[0].fields),
        rated(4, false, 0)
    );
    remove(&path);
}

/// A newer file taken in that changes the open photo's rating tells it, `Rating` and
/// `Rated`, as a command does: its later stars win, the session's label kept.
#[test]
fn a_newer_file_s_rating_is_told() {
    let path = scratch("rate-newer");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([rate(RatingChange::Stars(2))]);
    let mut elsewhere = file(&driven, &path);
    let last = elsewhere.copies[0].fields[0].clock.unwrap();
    let machine = "00000000".parse().unwrap();
    let clock = apertrust_library::clock::Clock::at(last.when + 60_000, None, machine);
    let four = Some(FieldValue::Stars(4));
    fields::set(&mut elsewhere.copies[0].fields, fields::RATING, four, clock);
    crate::history::advance(&mut elsewhere.copies[0].clocks, clock);
    std::fs::write(sidecar::path_for(&path), elsewhere.to_xml()).unwrap();
    driven.run([rate(RatingChange::Label { label: 0, on: true })]);
    // The writer finds the file changed and writes nothing; then it is taken in.
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert_eq!(told(&taken), Some(rated(4, false, 0b1)));
    remove(&path);
}

/// A rating and the edit's changes undo in the order they were made, and Reset, Compress
/// and Discard History leave the rating as it is.
#[test]
fn undo_takes_the_edit_and_the_rating_back_in_order_and_reset_keeps_it() {
    let path = scratch("rate-order");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let thumbnail = |driven: &Driven| driven.engine.open.as_ref().unwrap().thumbnail_hash;
    let edited = thumbnail(&driven);
    driven.run([rate(RatingChange::Label { label: 2, on: true })]);
    driven.writes();
    // The rendered thumbnail's version hashes the edit alone.
    assert_eq!(thumbnail(&driven), edited);
    driven.run([set("brightness", ControlValue::Number(0.7), true)]);
    for command in [
        Command::Compress { up_to: None },
        Command::Reset,
        Command::DiscardHistory,
    ] {
        let events = driven.run([command]);
        assert_eq!(told(&events), None, "the rating as it was");
    }
    assert_eq!(
        driven.engine.open.as_ref().unwrap().rating(),
        rated(0, false, 0b100)
    );
    // Discard History, Reset, Compress and the second brightness, then the label.
    for _ in 0..4 {
        assert_eq!(told(&driven.run([Command::Undo])), None);
    }
    let undone = driven.run([Command::Undo]);
    assert_eq!(told(&undone), Some(Rating::default()));
    assert_eq!(brightness(&driven.run([Command::Undo])), 0.0);
    remove(&path);
}

fn brightness(events: &[Event]) -> f32 {
    match control(values(events), "sample", "brightness").value {
        ControlValue::Number(value) => value,
        ref other => panic!("{other:?}"),
    }
}

/// A photo open for viewing only refuses a rating, and so does a command naming a photo
/// other than the active one, until the batch.
#[test]
fn a_view_only_photo_and_another_photo_are_refused() {
    let path = scratch("rate-refused");
    let newer =
        brightened_log("0.9.0").replace(r#"apertrust:version="2""#, r#"apertrust:version="3""#);
    std::fs::write(sidecar::path_for(&path), &newer).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let refused = driven.run([rate(RatingChange::Stars(1))]);
    assert!(
        matches!(failures(&refused)[..], [SessionError::ViewOnly(_)]),
        "{refused:?}"
    );
    assert_eq!(
        std::fs::read_to_string(sidecar::path_for(&path)).unwrap(),
        newer
    );
    remove(&path);
    let path = scratch("rate-other");
    driven.open(&path);
    let other = Command::Rate {
        photos: vec![path.with_file_name("other.ARW")],
        change: RatingChange::Stars(1),
    };
    let refused = driven.run([other]);
    assert!(
        matches!(failures(&refused)[..], [SessionError::NotActive { .. }]),
        "{refused:?}"
    );
    // Naming the active photo is the active photo.
    let named = Command::Rate {
        photos: vec![path.clone()],
        change: RatingChange::Stars(1),
    };
    assert_eq!(told(&driven.run([named])), Some(rated(1, false, 0)));
    remove(&path);
}

/// A photo with no `.apxmp` shows darktable's rating and labels from its `.xmp`, which its
/// first write copies in, each with that write's clock; darktable's file stays as it was.
#[test]
fn darktable_s_rating_shows_and_is_copied_in_at_the_first_write() {
    let path = scratch("rate-darktable");
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF
        xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description
        xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:darktable="http://darktable.sf.net/"
        xmp:Rating="2"><darktable:colorlabels><rdf:Seq><rdf:li>0</rdf:li></rdf:Seq>
        </darktable:colorlabels></rdf:Description></rdf:RDF></x:xmpmeta>"#;
    let darktable = apertrust_library::darktable::path_for(&path);
    std::fs::write(&darktable, xmp).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    assert_eq!(told(&opened), Some(rated(2, false, 0b1)));
    let events = driven.run([rate(RatingChange::Label { label: 3, on: true })]);
    assert_eq!(told(&events), Some(rated(2, false, 0b1001)));
    let written = file(&driven, &path);
    let copy = &written.copies[0];
    assert_eq!(Rating::of(&copy.fields), rated(2, false, 0b1001));
    assert!(copy.fields.iter().all(|entry| entry.clock.is_some()));
    assert_eq!(std::fs::read_to_string(&darktable).unwrap(), xmp);
    remove(&path);
}
