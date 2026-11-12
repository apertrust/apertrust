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

fn step(id: u64, entries: Vec<Entry>) -> Step {
    Step {
        id: StepId(id),
        clock: Clock::at(id, None, MachineId::UNKNOWN),
        source: StepSource::You,
        note: None,
        batch: None,
        entries,
    }
}

#[test]
fn a_step_holds_each_changed_module_s_whole_state_and_an_empty_entry_its_reset() {
    let before = brightened(0.5);
    let mut after = before.clone();
    let image = testing::image();
    (after.set_control(&Sample, "grain", ControlValue::Number(10.0), &image)).unwrap();
    (after.set_control(&Grain, "strength", ControlValue::Number(30.0), &image)).unwrap();
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

#[test]
fn the_fold_is_each_module_s_last_entry_up_to_the_end() {
    let (one, two) = (brightened(0.5), brightened(1.0));
    let mut log = Log::default();
    log.add(step(1, Entry::between(&Edit::default(), &one)));
    log.add(step(2, Entry::between(&one, &two)));
    log.add(step(3, Entry::between(&two, &Edit::default())));
    assert_eq!(log.fold(), Edit::default());
    log.set_end(2);
    assert_eq!(log.fold(), two);
    log.set_end(1);
    assert_eq!(log.fold(), one);
    log.set_end(0);
    assert_eq!(log.fold(), Edit::default());
}

#[test]
fn a_step_added_below_the_end_drops_the_steps_after_it() {
    let mut log = Log::default();
    for id in 1..=3 {
        log.add(step(
            id,
            Entry::between(&Edit::default(), &brightened(id as f32)),
        ));
    }
    log.set_end(1);
    let dropped = log.add(step(4, Vec::new()));
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

#[test]
fn a_version_1_edit_reads_as_one_step_with_the_same_id_every_time() {
    let edit = brightened(0.5);
    let hash = apertrust_library::index::hash(b"<x:xmpmeta/>");
    let modified = UNIX_EPOCH + Duration::from_millis(1_773_497_331_120);
    let log = Log::read(&edit, Some(hash), Some(modified));
    assert_eq!(log, Log::read(&edit, Some(hash), Some(modified)));
    assert_eq!((log.steps().len(), log.end()), (1, 1));
    let read = &log.steps()[0];
    let hex: String = hash[..8].iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(read.id.to_string(), hex);
    assert_eq!(
        read.clock,
        Clock {
            when: 1_773_497_331_120,
            counter: 0,
            machine: MachineId::UNKNOWN,
        }
    );
    assert_eq!((read.source, &read.note), (StepSource::You, &None));
    assert_eq!(log.fold(), edit);
    // A file holding nothing is no step.
    assert_eq!(
        Log::read(&Edit::default(), Some(hash), None),
        Log::default()
    );
}

#[test]
fn ids_are_random_and_none_clashes_within_its_log() {
    let mut log = Log::default();
    for _ in 0..100 {
        let id = log.new_id();
        assert!(log.steps().iter().all(|step| step.id != id));
        log.add(step(id.0, Vec::new()));
    }
    assert_eq!(StepId(0xa1b2c3d4e5f6).to_string(), "0000a1b2c3d4e5f6");
}

#[test]
fn a_step_is_described_control_by_control_before_and_after() {
    let image = testing::image();
    let one = brightened(0.5);
    let mut two = one.clone();
    (two.set_control(&Sample, "brightness", ControlValue::Number(1.0), &image)).unwrap();
    (two.set_control(&Grain, "strength", ControlValue::Number(30.0), &image)).unwrap();
    (two.set_param(OPERATIONS, MODULES, "exposure", "black", Value::Float(0.01))).unwrap();
    let mut log = Log::default();
    log.add(step(1, Entry::between(&Edit::default(), &one)));
    log.add(step(2, Entry::between(&one, &two)));
    log.add(step(3, Entry::between(&two, &Edit::default())));
    let mut described = Described::default();
    let steps = described.steps(&log, MODULES, OPERATIONS, &image);
    let brightness = |before: f32, after: f32, mark: [Option<Mark>; 2]| ControlChange {
        control: "brightness",
        before: ControlValue::Number(before),
        after: ControlValue::Number(after),
        mark,
    };
    let kept = Some(Mark::Kept);
    assert_eq!(
        steps[0].changes,
        [ModuleChange {
            module: "sample",
            enabled: None,
            controls: vec![brightness(0.0, 0.5, [None, kept])],
        }]
    );
    let black = base_black(&image);
    assert_eq!(
        steps[1].changes,
        [
            // Grain, switched, turned on.
            ModuleChange {
                module: "grain",
                enabled: Some([false, true]),
                controls: vec![ControlChange {
                    control: "strength",
                    before: ControlValue::Number(25.0),
                    after: ControlValue::Number(30.0),
                    mark: [None, kept],
                }],
            },
            ModuleChange {
                module: "sample",
                enabled: None,
                controls: vec![brightness(0.5, 1.0, [kept, kept])],
            },
            // An unowned parameter, under its operation.
            ModuleChange {
                module: "exposure",
                enabled: None,
                controls: vec![ControlChange {
                    control: "black",
                    before: ControlValue::Number(black),
                    after: ControlValue::Number(0.01),
                    mark: [None, None],
                }],
            },
        ]
    );
    assert_eq!(steps[2].changes[0].enabled, Some([true, false]));
    assert_eq!(
        steps[2].changes[1].controls,
        [brightness(1.0, 0.0, [kept, None])]
    );
    // Described once: kept while the steps before them stay.
    let again = described.steps(&log, MODULES, OPERATIONS, &image);
    assert!(steps.iter().zip(&again).all(|(a, b)| Arc::ptr_eq(a, b)));
    log.set_end(2);
    log.add(step(4, Entry::between(&two, &one)));
    let after = described.steps(&log, MODULES, OPERATIONS, &image);
    assert!(Arc::ptr_eq(&after[1], &steps[1]));
    assert_eq!(after[2].id, StepId(4));
    assert_eq!(
        after[2].changes[1].controls,
        [brightness(1.0, 0.5, [kept, kept])]
    );
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
