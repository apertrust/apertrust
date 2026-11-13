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

#[test]
fn a_version_1_edit_reads_as_a_step_of_each_module_with_the_same_ids_every_time() {
    let edit = grained(&brightened(0.5), 30.0);
    let hash = apertrust_library::index::hash(b"<x:xmpmeta/>");
    let modified = UNIX_EPOCH + Duration::from_millis(1_773_497_331_120);
    let log = Log::read(&edit, Some(hash), Some(modified));
    assert_eq!(log, Log::read(&edit, Some(hash), Some(modified)));
    assert_eq!((log.steps().len(), log.end()), (2, 2));
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
    assert_eq!(names(&[log.steps()[0].change.clone()]), ["grain"]);
    assert_eq!(log.fold(), edit);
    // One module: one step, in no batch.
    let one = Log::read(&brightened(0.5), Some(hash), None);
    assert_eq!(one.steps().len(), 1);
    assert_eq!((one.steps()[0].id, one.steps()[0].batch), (first, None));
    // A file holding nothing is no step.
    assert_eq!(
        Log::read(&Edit::default(), Some(hash), None),
        Log::default()
    );
}

/// The two readers of a version 1 file, the engine's from the typed edit and the text
/// layer's, give the same steps: the same ids, batch and modules in the same order, though
/// the file holds its modules in the definitions' order.
#[test]
fn both_version_1_readers_give_the_same_steps() {
    use apertrust_library::sidecar::{self as text, Edits};

    let mut edit = grained(&brightened(0.5), 30.0);
    (edit.set_param(OPERATIONS, MODULES, "exposure", "black", Value::Float(0.01))).unwrap();
    let xml = crate::sidecar::to_xml(&edit, OPERATIONS, MODULES, "test");
    let hash = apertrust_library::index::hash(xml.as_bytes());
    let ours: Vec<_> = (Log::read(&edit, Some(hash), None).steps().iter())
        .map(|step| {
            let name = match step.change.entries() {
                [Entry::Module { module, .. }] => module.clone(),
                [Entry::Operation { operation, .. }] => operation.clone(),
                other => panic!("one entry, not {other:?}"),
            };
            (step.id.to_string(), step.batch.map(|b| b.to_string()), name)
        })
        .collect();
    let read = Edits::parse(&xml, 0).unwrap();
    let theirs: Vec<_> = (read.copies[0].history.iter())
        .map(|step| {
            let name = match step.entries.as_slice() {
                [text::Entry::Module { module, .. }] => module.clone(),
                [text::Entry::Operation { operation, .. }] => operation.clone(),
                other => panic!("one entry, not {other:?}"),
            };
            (step.id.clone(), step.batch.clone(), name)
        })
        .collect();
    assert_eq!(ours.len(), 3);
    assert_eq!(ours, theirs);
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
fn the_default_lines_are_the_modules_every_photo_runs_in_darktable_s_history_order() {
    let lines = default_lines(apertrust_modules::MODULES, OPERATIONS);
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
    // Then every other module every photo runs, in the pipe's order; no switched one.
    let rest = &lines[6..];
    assert_eq!(
        rest,
        ["exposure", "channelmixerrgb", "tone_curve", "agx"],
        "{lines:?}"
    );
    let switched = |id: &&str| {
        apertrust_modules::MODULES
            .iter()
            .any(|m| m.definition().id == *id && m.definition().switched)
    };
    assert!(!lines.iter().any(switched), "{lines:?}");
    let every = apertrust_modules::MODULES.iter();
    let unswitched = every.filter(|m| !m.definition().switched).count();
    assert_eq!(lines.len(), unswitched);
}
