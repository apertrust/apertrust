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

use std::borrow::Cow;
use std::path::PathBuf;

use apertrust_engine::history::{ControlChange, Entry, ModuleChange, StepId};
use apertrust_library::sidecar as text;

use super::*;
use crate::{EditArgs, FlagOrder, MODULES, OPERATIONS, SetFlags};

/// A folder of a test's own holding a photo, `IMG.ARW`, which is no raw, removed when the
/// test ends.
struct Photo(PathBuf);

impl Photo {
    fn new(name: &str) -> Photo {
        let folder = std::env::temp_dir().join(format!(
            "apertrust-cli-history-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("IMG.ARW"), b"not read").unwrap();
        Photo(folder)
    }

    fn path(&self) -> PathBuf {
        self.0.join("IMG.ARW")
    }

    fn sidecar(&self) -> PathBuf {
        sidecar::path_for(&self.path())
    }

    /// The sidecar's bytes.
    fn bytes(&self) -> Vec<u8> {
        std::fs::read(self.sidecar()).unwrap()
    }

    /// The log the sidecar holds, as the engine reads it.
    fn log(&self) -> Log {
        (crate::saved(&self.path(), OPERATIONS, MODULES).unwrap())
            .expect("a sidecar")
            .log
    }

    /// The sidecar as the text layer reads it.
    fn edits(&self) -> sidecar::Edits {
        text::read_edits(&self.sidecar()).unwrap()
    }

    /// `edit FILE --set …`, with `--note TEXT` where given.
    fn edit(&self, sets: &[&str], note: Option<&str>) {
        crate::edit(
            &self.path(),
            EditArgs {
                flags: &SetFlags {
                    sets: sets.iter().map(|set| (*set).to_owned()).collect(),
                    params: Vec::new(),
                },
                note,
                resets: &[],
                param_resets: &[],
                clear: false,
                order: FlagOrder::default(),
            },
            OPERATIONS,
            MODULES,
        )
        .unwrap();
    }

    /// The photo after three changes by `edit`: Exposure, which gives it its default steps
    /// first, Colour balance RGB and AgX as one, noted, and Exposure again; 13 steps, the
    /// end after the last.
    fn edited(name: &str) -> Photo {
        let photo = Photo::new(name);
        photo.edit(&["exposure.exposure=1"], None);
        let both = ["colorbalancergb.contrast=20", "agx.curve_pivot_x=-1"];
        photo.edit(&both, Some("Warmer look"));
        photo.edit(&["exposure.exposure=0.5"], None);
        photo
    }
}

impl Drop for Photo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// This machine's clock in the copy's clock vector.
fn mine(edits: &sidecar::Edits) -> Clock {
    let clocks = &edits.copies[0].clocks;
    *(clocks.iter())
        .find(|clock| clock.machine == MachineId::current())
        .expect("this machine's clock")
}

/// The modules and operations of `log`'s steps' entries, in order.
fn modules_of(log: &Log) -> Vec<&str> {
    (log.steps().iter())
        .flat_map(|step| step.change.entries())
        .map(|entry| match entry {
            Entry::Module { module, .. } => module.as_str(),
            Entry::Operation { operation, .. } => operation.as_str(),
        })
        .collect()
}

/// `edit` gives a photo with no history its default steps before its first change, as the
/// app's first change does, at a clock before the change's, so Original leaves out the
/// modules darktable's workflow applies; a later change adds none, and one that changes
/// nothing, a note alone among them, writes nothing.
#[test]
fn edit_gives_a_photo_with_no_history_its_default_steps_first() {
    let photo = Photo::new("defaults");
    photo.edit(&["exposure.exposure=1"], None);
    let log = photo.log();
    let defaults = default_modules(MODULES, OPERATIONS);
    let mut modules = defaults.clone();
    modules.push("exposure");
    assert_eq!(modules_of(&log), modules);
    assert_eq!((log.end(), log.defaults()), (10, 9));
    assert!(log.steps()[8].clock < log.steps()[9].clock);
    let unapplied: Vec<String> = log.unapplied(0).into_iter().collect();
    assert_eq!(unapplied, ["agx", "channelmixerrgb", "exposure"]);
    assert_eq!(mine(&photo.edits()), log.steps()[9].clock);
    photo.edit(&["exposure.exposure=2"], None);
    assert_eq!(photo.log().steps().len(), 11);
    let bytes = photo.bytes();
    photo.edit(&["exposure.exposure=2"], Some("Same again"));
    photo.edit(&[], Some("Nothing"));
    assert_eq!(photo.bytes(), bytes);
}

/// `edit --clear` with the end at Original moves no value, but applies the modules not
/// applied there, as the session's Reset does: one Reset step of each, the default steps
/// kept and moved below it, the person's after the end dropped.
#[test]
fn clear_at_original_applies_the_modules_not_applied() {
    let photo = Photo::edited("clear");
    make(&photo.path(), Move::GoTo(0), OPERATIONS, MODULES).unwrap();
    crate::edit(
        &photo.path(),
        EditArgs {
            flags: &SetFlags::default(),
            note: None,
            resets: &[],
            param_resets: &[],
            clear: true,
            order: FlagOrder::default(),
        },
        OPERATIONS,
        MODULES,
    )
    .unwrap();
    let log = photo.log();
    let defaults = default_modules(MODULES, OPERATIONS);
    assert_eq!((log.steps().len(), log.end()), (10, 10));
    assert_eq!(modules_of(&log)[..9], defaults[..]);
    let reset = &log.steps()[9];
    assert_eq!(reset.note, Some(Note::reset()));
    assert_eq!(
        modules_of(&Log::from_steps(vec![reset.clone()], 1)),
        ["agx", "channelmixerrgb", "exposure"]
    );
    assert!(log.unapplied(log.end()).is_empty());
}

/// `--note`'s words are each step's of the change, stored as written, and listed with them.
#[test]
fn edit_s_note_is_stored_as_written_and_listed() {
    let photo = Photo::edited("note");
    let edits = photo.edits();
    let noted: Vec<Option<&text::Note>> = (edits.copies[0].history.iter())
        .map(|step| step.note.as_ref())
        .collect();
    let warmer = text::Note::Text("Warmer look".to_owned());
    assert_eq!(noted[9..], [None, Some(&warmer), Some(&warmer), None]);
    let log = photo.log();
    let steps = Described::default().steps(&log, MODULES, OPERATIONS, &ImageContext::full(0, 0));
    let listed = listing("IMG.ARW", &steps, log.end());
    assert!(
        listed.contains("11–12  2 steps · \"Warmer look\"\n"),
        "{listed}"
    );
}

/// Each move writes the log the engine's own functions make of the log read, the copy's
/// clock vector advanced with a clock of its own; one that would change nothing writes
/// nothing.
#[test]
fn each_move_writes_what_the_engine_s_log_functions_make() {
    /// The log a move should leave of the log read, `None` where it changes nothing.
    type Expected = fn(&Log) -> Option<Log>;
    fn at(log: &Log, end: usize) -> Log {
        let mut moved = log.clone();
        moved.set_end(end);
        moved
    }
    let photo = Photo::edited("moves");
    let expect: [(Move, Expected); 10] = [
        (Move::Undo, |log| log.undo_end().map(|end| at(log, end))),
        // Over the batch whole, Colour balance RGB and AgX.
        (Move::Undo, |log| log.undo_end().map(|end| at(log, end))),
        (Move::Redo, |log| log.redo_end().map(|end| at(log, end))),
        (Move::GoTo(10), |log| Some(at(log, 10))),
        (Move::Truncate, |log| Some(log.truncated())),
        (Move::Undo, |log| log.undo_end().map(|end| at(log, end))),
        // Never over a default step.
        (Move::Undo, |_| None),
        (Move::GoTo(3), |log| Some(at(log, 3))),
        (Move::Compress, |log| Some(log.compressed())),
        (Move::Discard, |_| Some(Log::default())),
    ];
    for (chosen, expected) in expect {
        let (before, bytes, clock) = (photo.log(), photo.bytes(), mine(&photo.edits()));
        let made = make(&photo.path(), chosen, OPERATIONS, MODULES).unwrap();
        match expected(&before) {
            Some(expected) => {
                assert!(made.written, "{chosen:?}");
                assert_eq!(made.log, expected, "{chosen:?}");
                assert_eq!(photo.log(), expected, "{chosen:?}");
                // The text layer's: its steps by id, its end the last applied's id.
                let edits = photo.edits();
                let copy = &edits.copies[0];
                let ids: Vec<String> = (copy.history.iter()).map(|s| s.id.clone()).collect();
                let wanted: Vec<String> = (expected.steps().iter())
                    .map(|s| s.id.to_string())
                    .collect();
                assert_eq!(ids, wanted, "{chosen:?}");
                assert_eq!(copy.applied(), expected.end(), "{chosen:?}");
                assert!(mine(&edits) > clock, "{chosen:?}");
            }
            None => {
                assert!(!made.written, "{chosen:?}");
                assert_eq!(photo.bytes(), bytes, "{chosen:?}");
            }
        }
    }
    // Discard left no step: the end empty, nothing to discard, compress or undo.
    assert_eq!(photo.edits().copies[0].end, "");
    for chosen in [Move::Discard, Move::Compress, Move::Truncate, Move::Undo] {
        let made = make(&photo.path(), chosen, OPERATIONS, MODULES).unwrap();
        assert!(!made.written, "{chosen:?}");
    }
}

/// A move is refused, the file left as it is: on an edit a newer Apertrust wrote, Discard
/// too; on one part of whose history cannot be read, but for Discard, which starts its log
/// afresh from the edit as read; and to a step the log does not have.
#[test]
fn refused_moves_leave_the_file_as_it_is() {
    let all = [
        Move::Undo,
        Move::Redo,
        Move::GoTo(0),
        Move::Compress,
        Move::Truncate,
        Move::Discard,
    ];
    // A newer Apertrust's: a module this build lacks.
    let photo = Photo::edited("newer");
    let xml = String::from_utf8(photo.bytes()).unwrap().replacen(
        "apertrust:module=\"rawprepare\"",
        "apertrust:module=\"lens2\"",
        1,
    );
    assert!(xml.contains("lens2"));
    std::fs::write(photo.sidecar(), &xml).unwrap();
    for chosen in all {
        let refused = make(&photo.path(), chosen, OPERATIONS, MODULES).unwrap_err();
        let view_only = (refused.downcast_ref::<ViewOnly>()).expect("a newer Apertrust's");
        assert!(matches!(view_only.reason, Reason::Newer(_)), "{chosen:?}");
        assert_eq!(photo.bytes(), xml.as_bytes(), "{chosen:?}");
    }

    // One with a step of a source this build does not know.
    let photo = Photo::edited("problem");
    let xml = String::from_utf8(photo.bytes()).unwrap().replacen(
        "apertrust:source=\"you\"",
        "apertrust:source=\"robot\"",
        1,
    );
    std::fs::write(photo.sidecar(), &xml).unwrap();
    let read = crate::saved(&photo.path(), OPERATIONS, MODULES)
        .unwrap()
        .unwrap();
    assert!(matches!(
        read.view_only().map(|v| v.reason),
        Some(Reason::Unreadable(_))
    ));
    for chosen in &all[..5] {
        let refused = make(&photo.path(), *chosen, OPERATIONS, MODULES).unwrap_err();
        let said = format!("{refused:#}");
        assert!(said.contains("only --discard"), "{chosen:?}: {said}");
        assert_eq!(photo.bytes(), xml.as_bytes(), "{chosen:?}");
    }
    let made = make(&photo.path(), Move::Discard, OPERATIONS, MODULES).unwrap();
    assert!(made.written);
    // Afresh: the default steps, then a step of each module the edit holds as read, in a
    // batch, noted `history-discard`, the end the last; nothing left that cannot be read.
    let edits = photo.edits();
    assert!(edits.problems.is_empty(), "{:?}", edits.problems);
    let log = photo.log();
    assert_eq!(log, made.log);
    let defaults = default_modules(MODULES, OPERATIONS);
    assert_eq!(modules_of(&log)[..defaults.len()], defaults[..]);
    let fresh = &log.steps()[defaults.len()..];
    assert_eq!(
        modules_of(&Log::from_steps(fresh.to_vec(), 0)),
        ["agx", "colorbalancergb", "exposure"]
    );
    assert!(fresh.iter().all(|step| step.note == Some(Note::discard())));
    assert!(fresh[0].batch.is_some() && fresh.iter().all(|s| s.batch == fresh[0].batch));
    assert_eq!(log.end(), log.steps().len());
    assert_eq!(log.fold(), read.edit);

    // A step the log does not have.
    let photo = Photo::edited("unknown");
    let bytes = photo.bytes();
    let refused = make(&photo.path(), Move::GoTo(14), OPERATIONS, MODULES).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "no step 14: the history has 13 steps; 0 is Original"
    );
    assert_eq!(photo.bytes(), bytes);
}

/// `--undo` never moves the end back over a default step after the person's: one an edit
/// started afresh leaves after the end, which `--redo` applies, listed as a default step.
#[test]
fn undo_stops_above_a_default_step_redo_applied() {
    let photo = Photo::new("afresh");
    // An edit as read with AgX not applied: its end before AgX's default step, the last.
    let defaults = default_modules(MODULES, OPERATIONS);
    assert_eq!(defaults.last(), Some(&"agx"));
    let clock = Clock::now(None, MachineId::current());
    let mut read = Log::default();
    read.add_defaults(&defaults, clock);
    read.set_end(defaults.len() - 1);
    let mut edit = read.fold();
    let exposure = apertrust_engine::edit::module(MODULES, "exposure").unwrap();
    let number = ControlValue::Number(1.0);
    (edit.set_control(exposure, "exposure", number, &ImageContext::full(0, 0))).unwrap();
    let log = Log::afresh(&edit, &defaults, clock);
    let (steps, end) = (log.steps().len(), log.end());
    let file = sidecar::Edits::new("test");
    let modules = (OPERATIONS, MODULES);
    sidecar::write_log(&photo.sidecar(), &file, &log, &[clock], modules, "test").unwrap();
    let made = make(&photo.path(), Move::Redo, OPERATIONS, MODULES).unwrap();
    assert_eq!(made.log.end(), end + 1);
    assert_eq!(made.log.end(), steps);
    let made = make(&photo.path(), Move::Undo, OPERATIONS, MODULES).unwrap();
    assert_eq!(
        (made.written, made.said.as_str()),
        (false, "Nothing to undo")
    );
    let described =
        Described::default().steps(&made.log, MODULES, OPERATIONS, &ImageContext::full(0, 0));
    let listed = listing("IMG.ARW", &described, made.log.end());
    // The end's line, past the Defaults group, tagged as a default step.
    let line = (listed.lines())
        .find(|line| line.contains("agx · default"))
        .unwrap_or_default();
    assert!(line.starts_with('>'), "{listed}");
    assert!(
        line.contains(&format!(" {steps}  agx · default  ")),
        "{listed}"
    );
}

/// `--discard` on an unreadable history of default steps alone, here a clock of the copy's
/// vector that cannot be read: with the end among them, the fresh log is written as it
/// is, so the modules not applied stay so as the photo reopens; with every one applied, as
/// no step, as a new photo's.
#[test]
fn discard_keeps_default_steps_not_applied_in_an_unreadable_history() {
    let defaults = default_modules(MODULES, OPERATIONS);
    let exposure = defaults.iter().position(|id| *id == "exposure").unwrap();
    for end in [exposure, defaults.len()] {
        let photo = Photo::new(&format!("partial-{end}"));
        let clock = Clock::now(None, MachineId::current());
        let mut log = Log::default();
        log.add_defaults(&defaults, clock);
        log.set_end(end);
        let file = sidecar::Edits::new("test");
        let modules = (OPERATIONS, MODULES);
        sidecar::write_log(&photo.sidecar(), &file, &log, &[clock], modules, "test").unwrap();
        // The vector's clock, after the steps', with a counter that is no number.
        let xml = String::from_utf8(photo.bytes()).unwrap();
        let (steps, vector) = xml.split_at(xml.find("<apertrust:clocks>").unwrap());
        let vector = vector.replacen("apertrust:counter=\"", "apertrust:counter=\"x", 1);
        std::fs::write(photo.sidecar(), format!("{steps}{vector}")).unwrap();
        let read = crate::saved(&photo.path(), OPERATIONS, MODULES)
            .unwrap()
            .unwrap();
        assert!(matches!(
            read.view_only().map(|v| v.reason),
            Some(Reason::Unreadable(_))
        ));
        let made = make(&photo.path(), Move::Discard, OPERATIONS, MODULES).unwrap();
        assert!(made.written);
        assert!(photo.edits().problems.is_empty());
        let written = photo.log();
        assert_eq!(written, made.log);
        if end < defaults.len() {
            assert_eq!(modules_of(&written), defaults);
            assert_eq!(written.end(), end);
            assert_eq!(written.fold(), read.edit);
            let unapplied: Vec<String> = written.unapplied(end).into_iter().collect();
            assert_eq!(unapplied, ["agx", "channelmixerrgb", "exposure"]);
        } else {
            assert!(written.steps().is_empty());
        }
    }
}

/// A file another program changed between the read and the write is refused, never
/// written over.
#[test]
fn a_file_changed_after_it_was_read_is_not_written_over() {
    let photo = Photo::edited("changed");
    let path = photo.path();
    let prepared = prepare(&path, Move::Undo, OPERATIONS, MODULES).unwrap();
    // Another program's edit meanwhile.
    photo.edit(&["exposure.exposure=2"], None);
    let theirs = photo.bytes();
    let refused = prepared.write().unwrap_err();
    assert!(
        refused.to_string().contains("changed since it was read"),
        "{refused:#}"
    );
    assert_eq!(photo.bytes(), theirs);
    assert_eq!(photo.log().steps().len(), 14);
}

/// A step as a client is told it, at `minute` past 10:00 on 14 November 2026 UTC.
fn told(source: StepSource, minute: u64, changes: Vec<ModuleChange>) -> Arc<HistoryStep> {
    Arc::new(HistoryStep {
        id: StepId(minute),
        clock: Clock::at(
            1_794_650_400_000 + minute * 60_000,
            None,
            MachineId::UNKNOWN,
        ),
        source,
        note: None,
        batch: None,
        changes,
    })
}

/// What a step changed of one module: each of `controls`, a number from its first to its
/// second, or a choice where the two are none.
fn module(id: &'static str, controls: &[(&'static str, Option<(f32, f32)>)]) -> ModuleChange {
    ModuleChange {
        module: Cow::Borrowed(id),
        enabled: None,
        controls: (controls.iter())
            .map(|&(control, numbers)| {
                let (before, after) = match numbers {
                    Some((before, after)) => {
                        (ControlValue::Number(before), ControlValue::Number(after))
                    }
                    None => (ControlValue::Choice(0), ControlValue::Choice(1)),
                };
                ControlChange {
                    control,
                    before,
                    after,
                    mark: [None, None],
                }
            })
            .collect(),
    }
}

/// The listing of a small history, newest first: the end marked, the step after it
/// counted, a batch under its range with its source and note, the default steps under
/// theirs, what each step changed, a version 1 step's unknown time, and Original.
#[test]
fn the_listing_says_what_each_step_changed_newest_first() {
    let defaults = ["rawprepare", "demosaic", "exposure"]
        .map(|id| told(StepSource::Default, 0, vec![module(id, &[])]));
    let mut batch = [
        told(
            StepSource::Assistant,
            2,
            vec![module(
                "colorbalancergb",
                &[("contrast", Some((0.0, 0.2))), ("saturation", None)],
            )],
        ),
        told(
            StepSource::Assistant,
            2,
            vec![module("agx", &[("curve_pivot_x", Some((0.0, -1.0)))])],
        ),
    ];
    for step in &mut batch {
        let step = Arc::make_mut(step);
        step.batch = Some(apertrust_engine::history::BatchId(7));
        step.note = Some(Note::Text("Warmer look".to_owned()));
    }
    let mut reset = told(StepSource::You, 5, vec![module("exposure", &[])]);
    Arc::make_mut(&mut reset).note = Some(Note::reset());
    let mut unknown = told(
        StepSource::You,
        0,
        vec![module("sigmoid", &[("contrast", None)])],
    );
    Arc::make_mut(&mut unknown).clock = Clock::at(0, None, MachineId::UNKNOWN);
    let steps: Vec<Arc<HistoryStep>> = (defaults.into_iter())
        .chain([told(
            StepSource::You,
            1,
            vec![module("exposure", &[("exposure", Some((0.7, 1.2)))])],
        )])
        .chain(batch)
        .chain([
            unknown,
            told(StepSource::Auto, 4, vec![module("temperature", &[])]),
            reset,
        ])
        .collect();
    assert_eq!(
        listing("IMG.ARW", &steps, 8),
        "IMG.ARW: 9 steps, the end at step 8; times in UTC
    9  Reset                         2026-11-14 10:05
       1 undone: the next edit drops it
>   8  temperature · auto            2026-11-14 10:04
    7  sigmoid contrast              no date
  5–6  2 steps · assistant · \"Warmer look\"
    6    agx curve_pivot_x ↓         2026-11-14 10:02
    5    colorbalancergb 2 controls  2026-11-14 10:02
    4  exposure ↑                    2026-11-14 10:01
  1–3  defaults: 3 modules every photo runs
    3    exposure                    2026-11-14 10:00
    2    demosaic                    2026-11-14 10:00
    1    rawprepare                  2026-11-14 10:00
    0  Original
"
    );
    // Within the default steps, those after the end counted apart, as the next edit
    // applies them; with no step, Original alone, marked.
    let listed = listing("IMG.ARW", &steps, 2);
    assert!(
        listed.contains(
            "  1–3  defaults: 3 modules every photo runs\n    3    exposure                    \
             2026-11-14 10:00\n         6 undone: the next edit drops them; 1 default step \
             not applied: the next edit applies it\n>   2    demosaic"
        ),
        "{listed}"
    );
    assert_eq!(
        listing("IMG.ARW", &[], 0),
        "IMG.ARW: no history\n> 0  Original\n"
    );
}
