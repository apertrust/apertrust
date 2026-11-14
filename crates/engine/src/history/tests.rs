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

use std::time::Duration;

use apertrust_modules::grain::Grain;
use apertrust_modules::testing::{self, Sample};
use apertrust_operations::OPERATIONS;

use super::*;

const MODULES: &[&'static dyn Module] = &[&Sample, &Grain];

/// An edit with Sample's brightness at `brightness`, kept.
fn brightened(brightness: f32) -> Edit {
    let mut edit = Edit::default();
    let image = testing::image();
    (edit.set_control(
        &Sample,
        "brightness",
        ControlValue::Number(brightness),
        &image,
    ))
    .unwrap();
    edit
}

/// `edit` with Grain on at `strength`.
fn grained(edit: &Edit, strength: f32) -> Edit {
    let mut grained = edit.clone();
    let image = testing::image();
    (grained.set_control(&Grain, "strength", ControlValue::Number(strength), &image)).unwrap();
    grained
}

/// A step of source You with no note, holding `change`.
fn step(id: u64, change: Change) -> Step {
    Step {
        id: StepId(id),
        clock: Clock::at(id, None, MachineId::UNKNOWN),
        source: StepSource::You,
        note: None,
        batch: None,
        change,
    }
}

/// The change of the one module that differs from `before` to `after`.
fn changed(before: &Edit, after: &Edit) -> Change {
    match &Entry::between(before, after)[..] {
        [entry] => Change::Entry(entry.clone()),
        other => panic!("one entry, not {other:?}"),
    }
}

/// Adds the steps of the change from `before` to `after` that wrote `wrote`, their ids from
/// `first` on, sharing the batch `first` where there are several.
fn add(log: &mut Log, first: u64, before: &Edit, after: &Edit, wrote: &[Wrote]) {
    let changes = Change::between(before, after, wrote);
    let batch = (changes.len() > 1).then_some(BatchId(first));
    for (index, change) in changes.into_iter().enumerate() {
        log.add(Step {
            batch,
            ..step(first + index as u64, change)
        });
    }
}

#[test]
fn a_change_s_entries_hold_each_changed_module_s_whole_state_an_empty_one_its_reset() {
    let before = brightened(0.5);
    let mut after = before.clone();
    let image = testing::image();
    (after.set_control(&Sample, "grain", ControlValue::Number(10.0), &image)).unwrap();
    let after = grained(&after, 30.0);
    let entries = Entry::between(&before, &after);
    // Sample whole, its brightness with its grain; Grain on.
    assert_eq!(
        entries,
        [
            Entry::Module {
                module: "grain".to_owned(),
                values: after.module("grain").unwrap().clone(),
            },
            Entry::Module {
                module: "sample".to_owned(),
                values: after.module("sample").unwrap().clone(),
            },
        ]
    );
    // Back to nothing: an empty entry each, which folds to none.
    let reset = Entry::between(&after, &Edit::default());
    assert_eq!(reset.len(), 2);
    assert!(reset.iter().all(Entry::is_empty));
    // A module holding nothing and none at all are the same state: no entry.
    let mut untouched = Edit::default();
    untouched
        .modules
        .insert("sample".to_owned(), ModuleValues::new());
    assert!(Entry::between(&Edit::default(), &untouched).is_empty());
}

/// What each change is of, in order: a module, an operation's parameters, or a reset of
/// how many.
fn names(changes: &[Change]) -> Vec<String> {
    (changes.iter())
        .map(|change| match change {
            Change::Entry(Entry::Module { module, .. }) => module.clone(),
            Change::Entry(Entry::Operation { operation, .. }) => format!("{operation} params"),
            Change::Reset(entries) => format!("reset of {}", entries.len()),
        })
        .collect()
}

/// A change is a step for each module and operation it touched, in the order first written
/// after the last `Reset`, then those none wrote in `Entry::between`'s order; a `Reset`
/// is one step in its place, holding each empty entry written after it by none.
#[test]
fn a_change_is_a_step_of_each_module_in_the_order_written_and_reset_one() {
    let mut after = grained(&brightened(0.5), 30.0);
    (after.set_param(OPERATIONS, MODULES, "exposure", "black", Value::Float(0.01))).unwrap();
    let module = |id: &str| Wrote::Module(id.to_owned());
    let black = || Wrote::Operation("exposure".to_owned());
    let between = |before: &Edit, after: &Edit, wrote: &[Wrote]| {
        names(&Change::between(before, after, wrote))
    };
    let none = Edit::default();
    assert_eq!(
        between(
            &none,
            &after,
            &[module("grain"), black(), module("sample"), module("grain")]
        ),
        ["grain", "exposure params", "sample"]
    );
    assert_eq!(
        between(&none, &after, &[module("sample")]),
        ["sample", "grain", "exposure params"]
    );
    assert_eq!(
        between(&none, &after, &[]),
        ["grain", "sample", "exposure params"]
    );
    // A `Reset`, then Sample written: the reset of the rest, then Sample.
    let wrote = [module("grain"), Wrote::All, module("sample")];
    let changes = Change::between(&after, &brightened(1.0), &wrote);
    assert_eq!(names(&changes), ["reset of 2", "sample"]);
    assert!(changes[0].entries().iter().all(Entry::is_empty));
    assert_eq!(between(&after, &none, &[Wrote::All]), ["reset of 3"]);
    // Nothing differs: no step.
    assert!(between(&after, &after, &[Wrote::All]).is_empty());
}

#[test]
fn the_fold_is_each_module_s_last_entry_up_to_the_end() {
    let (one, two) = (brightened(0.5), brightened(1.0));
    let mut log = Log::default();
    log.add(step(1, changed(&Edit::default(), &one)));
    log.add(step(2, changed(&one, &two)));
    log.add(step(3, changed(&two, &Edit::default())));
    assert_eq!(log.fold(), Edit::default());
    log.set_end(2);
    assert_eq!(log.fold(), two);
    log.set_end(1);
    assert_eq!(log.fold(), one);
    log.set_end(0);
    assert_eq!(log.fold(), Edit::default());
}

/// A batch's steps follow one another: Undo's fallback moves the end back to the batch's
/// first step, from its end or from inside it, and Redo's forward to its end; the fold
/// inside a batch holds the steps up to there.
#[test]
fn the_fallback_moves_over_a_whole_batch() {
    let one = brightened(0.5);
    let both = grained(&brightened(1.0), 30.0);
    let three = grained(&brightened(2.0), 30.0);
    let mut log = Log::default();
    log.add(step(1, changed(&Edit::default(), &one)));
    // Grain, then Sample, in a batch.
    add(&mut log, 2, &one, &both, &[]);
    log.add(step(4, changed(&both, &three)));
    let batches: Vec<_> = log.steps().iter().map(|step| step.batch).collect();
    assert_eq!(batches, [None, Some(BatchId(2)), Some(BatchId(2)), None]);
    let starts: Vec<_> = (0..=4).map(|end| log.batch_start(end)).collect();
    assert_eq!(starts, [None, Some(0), Some(1), Some(1), Some(3)]);
    let ends: Vec<_> = (0..=4).map(|end| log.batch_end(end)).collect();
    assert_eq!(ends, [Some(1), Some(3), Some(3), Some(4), None]);
    assert_eq!(log.fold_to(3), both);
    assert_eq!(log.fold_to(2), grained(&one, 30.0));
}

/// Where the fallback moves the end from each end: Undo back over a batch whole and never
/// over a default step, Redo forward over a batch whole, the default steps among them.
#[test]
fn the_fallback_s_ends_stop_above_the_default_steps() {
    let mut log = Log::default();
    log.add_defaults(
        &["temperature", "exposure"],
        Clock::at(1, None, MachineId::UNKNOWN),
    );
    let one = brightened(0.5);
    log.add(step(1, changed(&Edit::default(), &one)));
    add(&mut log, 2, &one, &grained(&brightened(1.0), 30.0), &[]);
    let ends = |log: &Log| -> Vec<(Option<usize>, Option<usize>)> {
        (0..=log.steps().len())
            .map(|end| {
                let mut at = log.clone();
                at.set_end(end);
                (at.undo_end(), at.redo_end())
            })
            .collect()
    };
    assert_eq!(
        ends(&log),
        [
            (None, Some(1)),
            (None, Some(2)),
            (None, Some(3)),
            (Some(2), Some(5)),
            (Some(3), Some(5)),
            (Some(3), None),
        ]
    );
    // With no default step, back to Original.
    let mut log = Log::default();
    log.add(step(1, changed(&Edit::default(), &one)));
    assert_eq!(ends(&log), [(None, Some(1)), (Some(0), None)]);
}

#[test]
fn a_step_added_below_the_end_drops_the_steps_after_it() {
    let mut log = Log::default();
    let edited = |id: u64| changed(&Edit::default(), &brightened(id as f32));
    for id in 1..=3 {
        log.add(step(id, edited(id)));
    }
    log.set_end(1);
    let dropped = log.add(step(4, edited(4)));
    let ids = |steps: &[Step]| steps.iter().map(|s| s.id.0).collect::<Vec<_>>();
    assert_eq!(ids(&dropped), [2, 3]);
    assert_eq!(ids(log.steps()), [1, 4]);
    assert_eq!(log.end(), 2);
    // Put back as they were.
    let taken = log.replace_after(1, dropped, 1);
    assert_eq!(ids(&taken), [4]);
    assert_eq!(ids(log.steps()), [1, 2, 3]);
    assert_eq!(log.end(), 1);
}

/// A change of one module merged into the last step applied: the step keeps its id, its
/// entry takes the change's, and its clock is the merge's. None into a step of another
/// module or of a batch, nor with no step applied.
#[test]
fn a_merged_change_takes_the_step_s_place_keeping_its_id() {
    let sample = |edit: &Edit| Entry::Module {
        module: "sample".to_owned(),
        values: edit.module("sample").unwrap().clone(),
    };
    let clock = Clock::at(10, None, MachineId::UNKNOWN);
    let mut log = Log::default();
    assert!(!log.merge(&sample(&brightened(1.0)), clock));
    assert!(log.steps().is_empty());
    log.add(step(1, changed(&Edit::default(), &brightened(0.5))));
    assert!(log.merge(&sample(&brightened(0.75)), clock));
    let [merged] = log.steps() else {
        panic!("one step, not {}", log.steps().len());
    };
    assert_eq!((merged.id, merged.clock), (StepId(1), clock));
    assert_eq!(log.fold(), brightened(0.75));
    // Not into a step of another module.
    let grain = changed(&brightened(0.75), &grained(&brightened(0.75), 30.0));
    let Change::Entry(entry) = &grain else {
        panic!("{grain:?}");
    };
    assert!(!log.merge(entry, clock));
    // Below the end, the last step applied takes it, the steps after it untouched.
    log.add(step(2, grain.clone()));
    log.set_end(1);
    assert!(log.merge(&sample(&brightened(0.25)), clock));
    assert_eq!(log.fold_to(1), brightened(0.25));
    assert_eq!(log.steps()[1].change, grain);
    // Not into a step of a batch, here Grain then Sample.
    let mut log = Log::default();
    add(
        &mut log,
        1,
        &Edit::default(),
        &grained(&brightened(0.5), 30.0),
        &[],
    );
    assert_eq!(log.steps().len(), 2);
    assert!(!log.merge(&sample(&brightened(1.0)), clock));
}

/// A version 1 file reads as a step of each module and operation, modules first and each
/// in id order, sharing a batch, the ids from the hash of the file's bytes, so every read
/// gives the same steps (`docs/history.md`, In the sidecar).
#[test]
fn a_version_1_file_reads_as_a_step_of_each_module_with_the_same_ids_every_time() {
    let mut edit = grained(&brightened(0.5), 30.0);
    (edit.set_param(OPERATIONS, MODULES, "exposure", "black", Value::Float(0.01))).unwrap();
    let xml = crate::sidecar::to_xml(&edit, OPERATIONS, MODULES, "test");
    let hash = apertrust_library::index::hash(xml.as_bytes());
    let modified = UNIX_EPOCH + Duration::from_millis(1_773_497_331_120);
    let read = |xml: &str| {
        let loaded = crate::sidecar::from_xml_at(xml, Some(modified), OPERATIONS, MODULES, &[]);
        loaded.unwrap().log
    };
    let log = read(&xml);
    assert_eq!(log, read(&xml));
    assert_eq!((log.steps().len(), log.end()), (3, 3));
    let hex = |bytes: &[u8]| -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() };
    let first = log.steps()[0].id;
    assert_eq!(first.to_string(), hex(&hash[..8]));
    assert_eq!(log.steps()[1].id, StepId(first.0.wrapping_add(1)));
    // One batch, so one Undo takes the edit read back whole.
    let batch = log.steps()[0].batch.expect("a batch");
    assert_eq!(batch.to_string(), hex(&hash[8..16]));
    for read in log.steps() {
        assert_eq!(
            read.clock,
            Clock {
                when: 1_773_497_331_120,
                counter: 0,
                machine: MachineId::UNKNOWN,
            }
        );
        assert_eq!(
            (read.source, &read.note, read.batch),
            (StepSource::You, &None, Some(batch))
        );
    }
    let changes: Vec<Change> = (log.steps().iter()).map(|s| s.change.clone()).collect();
    assert_eq!(names(&changes), ["grain", "sample", "exposure params"]);
    assert_eq!(log.fold(), edit);
    // Other bytes, other ids.
    let other = read(&xml.replace("test", "other"));
    assert_ne!(other.steps()[0].id, first);
    // One module: one step, in no batch.
    let one = crate::sidecar::to_xml(&brightened(0.5), OPERATIONS, MODULES, "test");
    let one = read(&one);
    assert_eq!(one.steps().len(), 1);
    assert_eq!(one.steps()[0].batch, None);
    // A file holding nothing is no step.
    let none = crate::sidecar::to_xml(&Edit::default(), OPERATIONS, MODULES, "test");
    assert_eq!(read(&none), Log::default());
}

/// A completed change's steps: a step of each, sharing a new batch, a Reset's noted
/// `history-reset`, the first dropping the steps after the end, which it returns.
#[test]
fn a_change_is_added_as_a_step_of_each_sharing_a_batch() {
    let clock = Clock::at(7, None, MachineId::UNKNOWN);
    let mut log = Log::default();
    let one = Change::between(&Edit::default(), &brightened(0.5), &[]);
    assert!(
        log.add_change(one, StepSource::Auto, None, clock)
            .is_empty()
    );
    assert_eq!(log.steps()[0].batch, None);
    assert_eq!(log.steps()[0].source, StepSource::Auto);
    let two = Change::between(&brightened(0.5), &grained(&brightened(1.0), 30.0), &[]);
    let note = Note::Text("warmer".to_owned());
    assert!(
        log.add_change(two, StepSource::Assistant, Some(note.clone()), clock)
            .is_empty()
    );
    let [_, first, second] = log.steps() else {
        panic!("three steps: {:?}", log.steps());
    };
    assert!(first.batch.is_some() && first.batch == second.batch);
    assert_eq!(
        (&first.note, &second.note),
        (&Some(note.clone()), &Some(note))
    );
    assert_eq!(log.end(), 3);
    // Below the end, the steps after it dropped and returned; a Reset noted as one.
    log.set_end(1);
    let reset = Change::between(&brightened(0.5), &Edit::default(), &[Wrote::All]);
    let dropped = log.add_change(reset, StepSource::You, None, clock);
    assert_eq!(dropped.len(), 2);
    assert_eq!(log.steps()[1].note, Some(Note::reset()));
    assert_eq!(log.fold(), Edit::default());
}

/// A copy's clock vector keeps the latest clock of each machine; the latest clock read
/// ignores another machine's far past the wall clock, never this machine's own.
#[test]
fn the_clock_vector_keeps_each_machine_s_latest() {
    let machine = |id: &str| id.parse::<MachineId>().unwrap();
    let (a, b) = (machine("0000000a"), machine("0000000b"));
    let mut vector = Vec::new();
    advance(&mut vector, Clock::at(10, None, a));
    advance(&mut vector, Clock::at(20, None, b));
    advance(&mut vector, Clock::at(5, None, a));
    advance(&mut vector, Clock::at(30, None, a));
    assert_eq!(vector, [Clock::at(30, None, a), Clock::at(20, None, b)]);
    let far = Clock::at(apertrust_library::clock::MAX_WHEN, None, b);
    let near = Clock::at(1_000, None, b);
    assert_eq!(latest([&near, &far], a), Some(near));
    assert_eq!(latest([&near, &far], b), Some(far));
    assert_eq!(latest([], a), None);
}

/// A change not yet a step moves onto another log's edit: each module it changed at its
/// state, the rest the other's.
#[test]
fn a_change_not_yet_a_step_is_rebased() {
    let from = brightened(0.5);
    let edit = grained(&from, 30.0);
    let onto = brightened(1.0);
    assert_eq!(rebase(&edit, &from, &onto), grained(&brightened(1.0), 30.0));
    assert_eq!(rebase(&from, &from, &onto), onto);
}

#[test]
fn ids_are_random_and_none_clashes_within_its_log() {
    let mut log = Log::default();
    let change = changed(&Edit::default(), &brightened(1.0));
    for _ in 0..100 {
        let id = log.new_id();
        assert!(log.steps().iter().all(|step| step.id != id));
        let batch = log.new_batch();
        assert!(log.steps().iter().all(|step| step.batch != Some(batch)));
        log.add(Step {
            batch: Some(batch),
            ..step(id.0, change.clone())
        });
    }
    assert_eq!(StepId(0xa1b2c3d4e5f6).to_string(), "0000a1b2c3d4e5f6");
}

#[test]
fn a_step_is_described_control_by_control_before_and_after() {
    let image = testing::image();
    let one = brightened(0.5);
    let mut two = grained(&one, 30.0);
    (two.set_control(&Sample, "brightness", ControlValue::Number(1.0), &image)).unwrap();
    (two.set_param(OPERATIONS, MODULES, "exposure", "black", Value::Float(0.01))).unwrap();
    let mut log = Log::default();
    log.add(step(1, changed(&Edit::default(), &one)));
    // Grain, Sample and the parameter, in a batch.
    add(&mut log, 2, &one, &two, &[]);
    // Reset: one step of each.
    add(&mut log, 5, &two, &Edit::default(), &[Wrote::All]);
    let mut described = Described::default();
    let steps = described.steps(&log, MODULES, OPERATIONS, &image);
    let brightness = |before: f32, after: f32, mark: [Option<Mark>; 2]| ControlChange {
        control: "brightness",
        before: ControlValue::Number(before),
        after: ControlValue::Number(after),
        mark,
    };
    let kept = Some(Mark::Kept);
    let batches: Vec<_> = steps.iter().map(|step| step.batch).collect();
    let batch = Some(BatchId(2));
    assert_eq!(batches, [None, batch, batch, batch, None]);
    assert_eq!(
        steps[0].changes,
        [ModuleChange {
            module: "sample".into(),
            enabled: None,
            controls: vec![brightness(0.0, 0.5, [None, kept])],
        }]
    );
    let black = base_black(&image);
    // Grain, switched, turned on.
    assert_eq!(
        steps[1].changes,
        [ModuleChange {
            module: "grain".into(),
            enabled: Some([false, true]),
            controls: vec![ControlChange {
                control: "strength",
                before: ControlValue::Number(25.0),
                after: ControlValue::Number(30.0),
                mark: [None, kept],
            }],
        }]
    );
    assert_eq!(
        steps[2].changes,
        [ModuleChange {
            module: "sample".into(),
            enabled: None,
            controls: vec![brightness(0.5, 1.0, [kept, kept])],
        }]
    );
    // An unowned parameter, under its operation.
    assert_eq!(
        steps[3].changes,
        [ModuleChange {
            module: "exposure".into(),
            enabled: None,
            controls: vec![ControlChange {
                control: "black",
                before: ControlValue::Number(black),
                after: ControlValue::Number(0.01),
                mark: [None, None],
            }],
        }]
    );
    // The reset: each module and operation it reset.
    let reset = &steps[4].changes;
    assert_eq!(reset.len(), 3);
    assert_eq!(reset[0].enabled, Some([true, false]));
    assert_eq!(reset[1].controls, [brightness(1.0, 0.0, [kept, None])]);
    // Described once: kept while the steps before them stay.
    let again = described.steps(&log, MODULES, OPERATIONS, &image);
    assert!(steps.iter().zip(&again).all(|(a, b)| Arc::ptr_eq(a, b)));
    log.set_end(4);
    let mut three = two.clone();
    (three.set_control(&Sample, "brightness", ControlValue::Number(0.5), &image)).unwrap();
    log.add(step(6, changed(&two, &three)));
    let after = described.steps(&log, MODULES, OPERATIONS, &image);
    assert!(Arc::ptr_eq(&after[3], &steps[3]));
    assert_eq!(after[4].id, StepId(6));
    assert_eq!(
        after[4].changes[0].controls,
        [brightness(1.0, 0.5, [kept, kept])]
    );
}

#[test]
fn a_module_or_operation_this_build_does_not_know_is_listed_by_its_stored_name() {
    let image = testing::image();
    let mut values = ModuleValues::new();
    values.set("radius", ControlValue::Number(3.0));
    let mut params = ParamValues::new();
    params.set("strength", Value::Float(0.5));
    let mut log = Log::default();
    log.add(step(
        1,
        Change::Entry(Entry::Module {
            module: "lens_2030".to_owned(),
            values,
        }),
    ));
    log.add(step(
        2,
        Change::Entry(Entry::Operation {
            operation: "glow".to_owned(),
            params,
        }),
    ));
    log.add(step(3, changed(&Edit::default(), &brightened(0.5))));
    let steps = Described::default().steps(&log, MODULES, OPERATIONS, &image);
    // As a newer build wrote them: their names, and nothing this build could describe.
    let unknown = |name: &str| ModuleChange {
        module: Cow::Owned(name.to_owned()),
        enabled: None,
        controls: Vec::new(),
    };
    assert_eq!(steps[0].changes, [unknown("lens_2030")]);
    assert_eq!(steps[1].changes, [unknown("glow")]);
    assert!(matches!(
        steps[2].changes[0].module,
        Cow::Borrowed("sample")
    ));
}

/// The exposure operation's black level for the photo.
fn base_black(image: &ImageContext) -> f32 {
    match Base::new(OPERATIONS, image).get("exposure", "black") {
        Some(Value::Float(black)) => black,
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_default_modules_are_darktable_s_for_a_new_raw_in_its_history_order() {
    let lines = default_modules(apertrust_modules::MODULES, OPERATIONS);
    // darktable's modules with no on/off button first, then those with one, each in the
    // pipe's order.
    assert_eq!(
        lines[..6],
        [
            "rawprepare",
            "demosaic",
            "colorout",
            "temperature",
            "highlights",
            "flip"
        ]
    );
    // Then those darktable's workflow auto-applies, in the pipe's order; Tone curve, which
    // no preset applies, has none, nor has any switched module.
    let rest = &lines[6..];
    assert_eq!(rest, ["exposure", "channelmixerrgb", "agx"], "{lines:?}");
    let switched = |id: &&str| {
        apertrust_modules::MODULES
            .iter()
            .any(|m| m.definition().id == *id && m.definition().switched)
    };
    assert!(!lines.iter().any(switched), "{lines:?}");
}

/// The modules every photo runs, as the default steps hold them.
fn defaults() -> Vec<&'static str> {
    default_modules(apertrust_modules::MODULES, OPERATIONS)
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

#[test]
fn a_new_photo_s_log_is_given_a_default_step_of_each_module_every_photo_runs() {
    let defaults = defaults();
    let mut log = Log::default();
    log.add_defaults(&defaults, Clock::at(1, None, MachineId::UNKNOWN));
    // Steps 1 to n, source Default, each at its module's reset state, the end after them.
    assert_eq!(modules_of(&log), defaults);
    assert_eq!(
        (log.end(), log.defaults()),
        (defaults.len(), defaults.len())
    );
    assert!((log.steps().iter()).all(|step| {
        step.source == StepSource::Default
            && step.note.is_none()
            && step.batch.is_none()
            && step.change.entries().iter().all(Entry::is_empty)
    }));
    // The edit they fold to is the photo untouched, every module applied.
    assert_eq!(log.fold(), Edit::default());
}

/// A later build that runs one more module on every photo gives a log that has steps no
/// step of it: the steps keep their numbers, and the module runs at its defaults.
#[test]
fn a_later_build_s_new_default_module_adds_no_step() {
    let defaults = defaults();
    let (earlier, added) = defaults.split_at(defaults.len() - 1);
    let mut log = Log::default();
    log.add_defaults(earlier, Clock::at(1, None, MachineId::UNKNOWN));
    let ids: Vec<StepId> = log.steps().iter().map(|step| step.id).collect();
    log.add_defaults(&defaults, Clock::at(2, None, MachineId::UNKNOWN));
    let again: Vec<StepId> = log.steps().iter().map(|step| step.id).collect();
    assert_eq!(again, ids);
    // Applied wherever the end is, having no default step to lie before.
    for end in 0..=log.steps().len() {
        assert!(!log.unapplied(end).contains(added[0]), "{end}");
    }
    // A log of a person's steps, as an older build's version 2 file or a version 1 file
    // holding an edit reads, is given none either.
    let mut read = Log::default();
    add(&mut read, 7, &Edit::default(), &brightened(0.5), &[]);
    read.add_defaults(&defaults, Clock::at(3, None, MachineId::UNKNOWN));
    assert_eq!((read.steps().len(), read.defaults()), (1, 0));
}

/// Below its default step, a module darktable leaves off until its workflow applies it is
/// not applied; one darktable has on by default always is.
#[test]
fn the_end_within_the_default_steps_applies_the_first_k() {
    let defaults = defaults();
    let mut log = Log::default();
    log.add_defaults(&defaults, Clock::at(1, None, MachineId::UNKNOWN));
    let on = [
        "rawprepare",
        "demosaic",
        "colorout",
        "temperature",
        "highlights",
        "flip",
    ];
    for end in 0..=defaults.len() {
        let expected: BTreeSet<String> = (defaults[end..].iter())
            .filter(|module| !on.contains(module))
            .map(|module| (*module).to_owned())
            .collect();
        assert_eq!(log.fold_to(end).unapplied, expected, "{end}");
    }
    let original: BTreeSet<String> = ["agx", "channelmixerrgb", "exposure"]
        .map(str::to_owned)
        .into();
    assert_eq!(log.fold_to(0).unapplied, original);
    assert!(log.fold().unapplied.is_empty());
}

/// A change made with the end within the default steps keeps those after it, moved below
/// it, as darktable keeps an always-on module's item, so every module is applied again;
/// the steps after the end that are not default steps are dropped.
#[test]
fn a_change_within_the_default_steps_keeps_the_later_ones() {
    let mut log = Log::default();
    log.add_defaults(
        &["temperature", "exposure", "agx"],
        Clock::at(1, None, MachineId::UNKNOWN),
    );
    add(&mut log, 10, &Edit::default(), &brightened(0.5), &[]);
    log.set_end(1);
    let after: Vec<StepId> = log.steps()[1..].iter().map(|step| step.id).collect();
    let folded = log.fold();
    assert_eq!(
        folded.unapplied,
        ["agx", "exposure"].map(str::to_owned).into()
    );
    let back = log.add(step(11, changed(&folded, &grained(&folded, 5.0))));
    // Every step that was after the end, which Undo puts back.
    assert_eq!(back.iter().map(|step| step.id).collect::<Vec<_>>(), after);
    assert_eq!(
        modules_of(&log),
        ["temperature", "exposure", "agx", "grain"]
    );
    assert_eq!((log.end(), log.defaults()), (4, 3));
    assert!(log.fold().unapplied.is_empty());
}

/// The ids of `log`'s steps, and its end.
fn ids_and_end(log: &Log) -> (Vec<u64>, usize) {
    (
        log.steps().iter().map(|step| step.id.0).collect(),
        log.end(),
    )
}

/// Compress keeps, among the steps up to the end, each module's and operation's last step,
/// as it was; one whose entry is empty goes, as a module's reset and Reset do, and so does
/// every step after the end. The end is the last step kept, and the log renders as before.
#[test]
fn compress_keeps_each_module_s_last_entry_drops_empty_ones_and_every_step_after_the_end() {
    let one = brightened(0.5);
    let both = grained(&one, 10.0);
    let brighter = grained(&brightened(0.8), 10.0);
    let ungrained = brightened(0.8);
    let mut log = Log::default();
    log.add(step(1, changed(&Edit::default(), &one)));
    log.add(Step {
        note: Some(Note::Text("Grain".to_owned())),
        batch: Some(BatchId(9)),
        ..step(2, changed(&one, &both))
    });
    log.add(step(3, changed(&both, &brighter)));
    // Grain's reset: an empty entry, its module's last.
    log.add(step(4, changed(&brighter, &ungrained)));
    log.add(step(5, changed(&ungrained, &brightened(0.9))));
    log.set_end(4);
    let compressed = log.compressed();
    assert_eq!(ids_and_end(&compressed), (vec![3], 1));
    // Kept as it was: its id, clock, source, note, batch and entry.
    assert_eq!(compressed.steps()[0], log.steps()[2]);
    assert_eq!(compressed.fold(), log.fold());
    // A step that is its module's last keeps its note and batch.
    log.set_end(3);
    let kept = log.compressed();
    assert_eq!(ids_and_end(&kept), (vec![2, 3], 2));
    assert_eq!(
        (&kept.steps()[0].note, kept.steps()[0].batch),
        (&Some(Note::Text("Grain".to_owned())), Some(BatchId(9)))
    );
    assert_eq!(kept.fold(), log.fold());
    // Reset holds empty entries alone: it goes, and the steps before it it reset with it.
    let mut reset = Log::default();
    reset.add(step(1, changed(&Edit::default(), &one)));
    add(&mut reset, 2, &one, &Edit::default(), &[Wrote::All]);
    assert!(matches!(reset.steps()[1].change, Change::Reset(_)));
    assert_eq!(ids_and_end(&reset.compressed()), (vec![], 0));
    // At Original, nothing is kept; compressed again, a log is as it was.
    log.set_end(0);
    assert_eq!(log.compressed(), Log::default());
    assert_eq!(compressed.compressed(), compressed);
}

/// Compress keeps every default step, the end's or Original's photo needing them, though a
/// later step of its module follows, so the Defaults stay whole.
#[test]
fn compress_keeps_every_default_step() {
    let mut log = Log::default();
    log.add_defaults(&["sample", "grain"], Clock::at(1, None, MachineId::UNKNOWN));
    let defaults: Vec<u64> = log.steps().iter().map(|step| step.id.0).collect();
    let one = brightened(0.5);
    log.add(step(1, changed(&Edit::default(), &one)));
    let compressed = log.compressed();
    assert_eq!(
        ids_and_end(&compressed),
        (vec![defaults[0], defaults[1], 1], 3)
    );
    assert_eq!(compressed.defaults(), 2);
    assert_eq!(compressed.fold(), log.fold());
    // Sample reset after: its last step empty, no step of the person's kept for it.
    log.add(step(2, changed(&one, &Edit::default())));
    assert_eq!(ids_and_end(&log.compressed()), (defaults, 2));
}

/// A log of darktable's default steps, those darktable has off until its workflow applies
/// them among them, then the person's: Sample set, Grain set, Sample reset, and Reset.
fn defaults_then_steps() -> Log {
    let mut log = Log::default();
    log.add_defaults(
        &["temperature", "exposure", "agx"],
        Clock::at(1, None, MachineId::UNKNOWN),
    );
    let one = brightened(0.5);
    let both = grained(&one, 10.0);
    let grain = grained(&Edit::default(), 10.0);
    log.add(step(1, changed(&Edit::default(), &one)));
    log.add(step(2, changed(&one, &both)));
    log.add(step(3, changed(&both, &grain)));
    add(&mut log, 4, &grain, &Edit::default(), &[Wrote::All]);
    log
}

/// Compress and Truncate never change what the photo shows, at the end or at Original,
/// wherever the end is past Original, among the default steps too, where those after it
/// stay after it, their modules not applied; at end 0 both drop every step.
#[test]
fn compress_and_truncate_leave_the_photo_as_it_shows_at_the_end_and_at_original() {
    let log = defaults_then_steps();
    for end in 1..=log.steps().len() {
        let mut at = log.clone();
        at.set_end(end);
        for (name, rewritten) in [("compress", at.compressed()), ("truncate", at.truncated())] {
            assert_eq!(rewritten.fold(), at.fold_to(end), "{name} at {end}");
            assert_eq!(rewritten.fold_to(0), at.fold_to(0), "{name} at {end}");
            assert_eq!(rewritten.defaults(), 3, "{name} at {end}");
            // The person's steps after the end gone.
            let after = &rewritten.steps()[rewritten.end()..];
            assert!(after.iter().all(|step| step.source == StepSource::Default));
        }
    }
    let mut among = log.clone();
    among.set_end(1);
    let compressed = among.compressed();
    assert_eq!((compressed.steps().len(), compressed.end()), (3, 1));
    let unapplied: BTreeSet<String> = ["agx", "exposure"].map(str::to_owned).into();
    assert_eq!(compressed.fold().unapplied, unapplied);
    assert_eq!(among.truncated(), compressed);
    let mut original = log;
    original.set_end(0);
    assert_eq!(original.compressed(), Log::default());
    assert_eq!(original.truncated(), Log::default());
}

/// Truncate drops the steps after the end, every step up to it kept as it is; at Original,
/// all of them.
#[test]
fn truncate_drops_the_steps_after_the_end_alone() {
    let mut log = Log::default();
    let edited = |id: u64| changed(&Edit::default(), &brightened(id as f32));
    for id in 1..=4 {
        log.add(step(id, edited(id)));
    }
    log.set_end(2);
    let truncated = log.truncated();
    assert_eq!(ids_and_end(&truncated), (vec![1, 2], 2));
    assert_eq!(truncated.steps(), &log.steps()[..2]);
    assert_eq!(truncated.fold(), log.fold());
    assert_eq!(truncated.truncated(), truncated);
    log.set_end(0);
    assert_eq!(log.truncated(), Log::default());
}

/// An edit started afresh, as Discard History starts an unreadable history: a new photo's
/// default steps, then a step of each module and operation it holds, source You, noted
/// `history-discard`, sharing a batch, the end after the last, folding to the edit, with
/// Original the untouched photo; a module not applied in it keeps its default step after
/// the end, so it stays so. None for an edit holding nothing, with no default module.
#[test]
fn an_edit_started_afresh_is_the_default_steps_and_a_step_of_each_module_it_holds() {
    let mut edit = grained(&brightened(0.5), 10.0);
    let param = Value::Float(2.0);
    edit.params
        .entry("op".to_owned())
        .or_default()
        .set("p", param);
    let clock = Clock::at(5, None, MachineId::UNKNOWN);
    let defaults = ["temperature", "exposure"];
    let log = Log::afresh(&edit, &defaults, clock);
    assert_eq!(
        modules_of(&log),
        ["temperature", "exposure", "grain", "sample", "op"]
    );
    assert_eq!((log.end(), log.defaults()), (5, 2));
    let batch = log.steps()[2].batch;
    assert!(batch.is_some());
    assert!(log.steps()[2..].iter().all(|step| {
        step.source == StepSource::You
            && step.note == Some(Note::discard())
            && step.batch == batch
            && step.clock == clock
    }));
    assert_eq!(log.fold(), edit);
    let untouched: BTreeSet<String> = ["exposure".to_owned()].into();
    assert_eq!(log.fold_to(0).unapplied, untouched);
    // Exposure and AgX not applied in the edit as read: their default steps after the end.
    let unapplied = Edit {
        unapplied: ["agx", "exposure"].map(str::to_owned).into(),
        ..Edit::default()
    };
    let log = Log::afresh(&unapplied, &["temperature", "exposure", "agx"], clock);
    assert_eq!(modules_of(&log), ["temperature", "exposure", "agx"]);
    assert_eq!(log.end(), 1);
    assert_eq!(log.fold(), unapplied);
    assert_eq!(Log::afresh(&Edit::default(), &[], clock), Log::default());
}

/// Undo's fallback never moves the end back over a default step, wherever it lies: not
/// over one an edit started afresh leaves after the end once Redo applies it, nor, after
/// an edit there, over those the edit kept and moved below it.
#[test]
fn undo_s_fallback_stops_above_a_default_step_after_the_person_s() {
    let unapplied = Edit {
        unapplied: ["agx", "exposure"].map(str::to_owned).into(),
        ..brightened(0.5)
    };
    let clock = Clock::at(5, None, MachineId::UNKNOWN);
    let mut log = Log::afresh(&unapplied, &["temperature", "exposure", "agx"], clock);
    assert_eq!(
        modules_of(&log),
        ["temperature", "sample", "exposure", "agx"]
    );
    assert_eq!((log.end(), log.defaults()), (2, 1));
    // Redo applies Exposure's default step, which Undo never takes back.
    log.set_end(log.redo_end().unwrap());
    assert_eq!(log.end(), 3);
    assert_eq!(log.undo_end(), None);
    // An edit after it keeps AgX's default step, moved below it; Undo takes the edit back
    // and stops above AgX's.
    log.add(step(9, changed(&log.fold(), &brightened(1.0))));
    assert_eq!(
        modules_of(&log),
        ["temperature", "sample", "exposure", "agx", "sample"]
    );
    assert_eq!(log.undo_end(), Some(4));
    log.set_end(4);
    assert_eq!(log.undo_end(), None);
}
