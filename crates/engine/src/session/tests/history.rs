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

//! The session's history (decision 78): each completed change a step, Undo and Redo,
//! Reset a step, and the `History` fact.

use apertrust_modules::soften::Soften;

use super::*;

/// The `History` facts among `events`.
fn histories(events: &[Event]) -> Vec<&History> {
    (events.iter())
        .filter_map(|event| match event {
            Event::History(history) => Some(history),
            _ => None,
        })
        .collect()
}

/// The one `History` among `events`.
fn history(events: &[Event]) -> &History {
    match &histories(events)[..] {
        [history] => history,
        other => panic!("one History in {events:?}, not {}", other.len()),
    }
}

/// The kinds of `events` the session's order rules name, in order.
fn kinds(events: &[Event]) -> Vec<&'static str> {
    (events.iter())
        .filter_map(|event| match event {
            Event::Values(_) => Some("values"),
            Event::History(_) => Some("history"),
            Event::Frame(_) => Some("frame"),
            Event::Failed(_) => Some("failed"),
            _ => None,
        })
        .collect()
}

fn brightness(events: &[Event]) -> ControlValue {
    control(values(events), "sample", "brightness")
        .value
        .clone()
}

fn amount(value: f32, done: bool) -> Command {
    Command::SetControl {
        module: "soften".to_owned(),
        control: "amount".to_owned(),
        value: ControlValue::Number(value),
        done,
    }
}

/// The open photo's edit, as the session holds it.
fn edit(driven: &Driven) -> Edit {
    driven.engine.open.as_ref().unwrap().edit.clone()
}

/// The open photo's edit folded from its log, which is always the edit once its changes
/// are steps.
fn fold(driven: &Driven) -> Edit {
    driven.engine.open.as_ref().unwrap().history.log().fold()
}

#[test]
fn a_photo_opens_with_its_history_after_its_values_not_yet_live() {
    let path = scratch("history-open");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    let opened = driven.open(&path);
    // Its values not yet live, its history, then its values live and the first frame.
    assert_eq!(kinds(&opened), ["values", "history", "values", "frame"]);
    let history = history(&opened);
    assert_eq!((history.steps.len(), history.end), (0, 0));
    assert_eq!((history.undo, history.redo), (None, None));
    // The modules every photo runs; Soften, switched, is no Default line.
    assert_eq!(history.defaults, ["sample"]);
    remove(&path);
}

/// Each completed change is one step, and one `History` follows its batch's values, before
/// its frame; a gesture's `done: false` changes send none, and `through` never decreases.
#[test]
fn one_history_a_batch_that_changed_it_and_none_during_a_gesture() {
    let path = scratch("history-batches");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let mut throughs = Vec::new();
    for step in 1..=5 {
        let events = driven.run([set(
            "brightness",
            ControlValue::Number(0.1 * step as f32),
            false,
        )]);
        assert!(histories(&events).is_empty(), "{events:?}");
    }
    let events = driven.run([set("brightness", ControlValue::Number(0.6), true)]);
    assert_eq!(kinds(&events), ["values", "history", "frame"]);
    let released = history(&events);
    assert_eq!((released.steps.len(), released.end), (1, 1));
    assert_eq!(released.through, driven.engine.applied);
    throughs.push(released.through);
    let step = &released.steps[0];
    assert_eq!(
        (step.source, &step.note, step.batch),
        (StepSource::You, &None, None)
    );
    // The gesture is one step, from the default to its release.
    let change = &step.changes[0].controls[0];
    assert_eq!(
        (&change.before, &change.after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.6))
    );
    assert_eq!(released.undo, Some(Move::Step(step.id)));
    // Two completed changes in a batch: two steps, one fact.
    let events = driven.run([
        set("grain", ControlValue::Number(10.0), true),
        set("bias", ControlValue::Number(50.0), true),
    ]);
    assert_eq!(kinds(&events), ["values", "history", "frame"]);
    let both = history(&events);
    assert_eq!((both.steps.len(), both.end), (3, 3));
    // The steps it shares with the last fact are the same, not copies.
    assert!(Arc::ptr_eq(&both.steps[0], &released.steps[0]));
    throughs.push(both.through);
    // A change that changes nothing, a resize and a refusal make no step.
    let events = driven.run([
        set("grain", ControlValue::Number(10.0), true),
        set("nothing", ControlValue::Number(1.0), true),
    ]);
    assert_eq!(kinds(&events), ["failed", "values"]);
    let events = driven.run([Command::Undo, Command::Redo]);
    assert_eq!(kinds(&events), ["values", "history", "frame"]);
    throughs.push(history(&events).through);
    assert!(throughs.is_sorted(), "{throughs:?}");
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

#[test]
fn undo_and_redo_are_inverses_rendered_and_saved() {
    let path = scratch("history-undo");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let half = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let one = driven.run([set("brightness", ControlValue::Number(1.0), true)]);
    let second = history(&one).steps[1].id;
    driven.writes();
    let undone = driven.run([Command::Undo]);
    assert_eq!(kinds(&undone), ["values", "history", "frame"]);
    assert_eq!(brightness(&undone), ControlValue::Number(0.5));
    assert!(frame(&driven.gpu, &undone) == frame(&driven.gpu, &half));
    let after = history(&undone);
    // Undo takes the step out of the log; Redo would put it back.
    assert_eq!((after.steps.len(), after.end), (1, 1));
    assert_eq!(after.redo, Some(Move::Step(second)));
    // Saved as a completed change: the file holds the edit at the end.
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    let read = sidecar::read(&sidecar, OPERATIONS, &[&Sample], &[]).unwrap();
    assert_eq!(read.edit, edit(&driven));
    let redone = driven.run([Command::Redo]);
    assert_eq!(brightness(&redone), ControlValue::Number(1.0));
    assert!(frame(&driven.gpu, &redone) == frame(&driven.gpu, &one));
    let after = history(&redone);
    assert_eq!((after.steps.len(), after.end, after.redo), (2, 2, None));
    assert_eq!(after.steps[1].id, second);
    assert_eq!(fold(&driven), edit(&driven));
    // Back to Original, then nothing more to undo: the values, no history, no frame.
    driven.run([Command::Undo, Command::Undo]);
    assert_eq!(edit(&driven), Edit::default());
    let nothing = driven.run([Command::Undo]);
    assert_eq!(kinds(&nothing), ["values"]);
    remove(&path);
}

/// Reset is one step noted `history-reset`, turning a switched module off; its Undo brings
/// every module back, the switched one on.
#[test]
fn reset_is_a_step_and_its_undo_brings_each_module_back() {
    let path = scratch("history-reset");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    driven.open(&path);
    driven.run([amount(30.0, true)]);
    let edited = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let before = edit(&driven);
    let reset = driven.run([Command::Reset]);
    let soften = |events: &[Event]| values(events).modules[1].clone();
    assert!(!soften(&reset).enabled);
    assert_eq!(brightness(&reset), ControlValue::Number(0.0));
    let history = history(&reset);
    assert_eq!((history.steps.len(), history.end), (3, 3));
    let step = &history.steps[2];
    assert_eq!(step.note, Some(Note::reset()));
    // Each module the edit held, the switched one turned off.
    let modules: Vec<_> = step.changes.iter().map(|c| (c.module, c.enabled)).collect();
    assert_eq!(modules, [("sample", None), ("soften", Some([true, false]))]);
    let undone = driven.run([Command::Undo]);
    assert!(soften(&undone).enabled);
    assert_eq!(brightness(&undone), ControlValue::Number(0.5));
    assert_eq!(edit(&driven), before);
    assert!(frame(&driven.gpu, &undone) == frame(&driven.gpu, &edited));
    // A reset of an untouched edit changes nothing, so it is no step.
    driven.run([Command::Undo, Command::Undo, Command::Undo]);
    let nothing = driven.run([Command::Reset]);
    assert!(histories(&nothing).is_empty(), "{nothing:?}");
    remove(&path);
}

/// A release refused, as one out of range, ends its gesture as completed (decision 42):
/// the gesture is one step of its own, from before it to its last value taken.
#[test]
fn a_refused_release_leaves_its_gesture_a_step_of_its_own() {
    let path = scratch("history-refused-release");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    for value in [0.5, 0.8] {
        let events = driven.run([set("brightness", ControlValue::Number(value), false)]);
        assert!(histories(&events).is_empty(), "{events:?}");
    }
    let events = driven.run([set("brightness", ControlValue::Number(10.0), true)]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::Edit(EditError::Control { .. })]
        ),
        "{events:?}"
    );
    let released = history(&events);
    assert_eq!((released.steps.len(), released.end), (1, 1));
    let change = &released.steps[0].changes[0].controls[..];
    assert!(
        matches!(
            change,
            [ControlChange {
                control: "brightness",
                before: ControlValue::Number(0.0),
                after: ControlValue::Number(0.8),
                ..
            }]
        ),
        "{change:?}"
    );
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// A change not yet a step, a `Set` or a drag under way, is made one before Undo, which
/// then takes it back; Redo puts it back.
#[test]
fn undo_takes_back_a_pending_set_and_a_drag() {
    let path = scratch("history-pending");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let black = Command::Set {
        operation: "exposure".to_owned(),
        param: "black".to_owned(),
        value: Value::Float(0.01),
    };
    let set_black = driven.run([black]);
    assert!(histories(&set_black).is_empty(), "{set_black:?}");
    let black = |events: &[Event]| {
        (values(events).settings.iter())
            .find(|s| (s.operation, s.param) == ("exposure", "black"))
            .unwrap()
            .value
    };
    let undone = driven.run([Command::Undo]);
    assert_ne!(black(&undone), Value::Float(0.01));
    assert_eq!(brightness(&undone), ControlValue::Number(0.5));
    let after = history(&undone);
    assert_eq!(after.steps.len(), 1);
    assert!(after.redo.is_some());
    let redone = driven.run([Command::Redo]);
    assert_eq!(black(&redone), Value::Float(0.01));
    // A drag under way: Undo takes back the drag, and the gesture is over.
    driven.run([set("brightness", ControlValue::Number(1.0), false)]);
    let undone = driven.run([Command::Undo]);
    assert_eq!(brightness(&undone), ControlValue::Number(0.5));
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    let redone = driven.run([Command::Redo]);
    assert_eq!(brightness(&redone), ControlValue::Number(1.0));
    // A gesture another command ends is a step of its own.
    driven.run([set("grain", ControlValue::Number(20.0), false)]);
    let ended = driven.run([Command::ResetControl {
        module: "sample".to_owned(),
        control: "brightness".to_owned(),
    }]);
    let steps = &history(&ended).steps;
    let last: Vec<_> = (steps.iter().rev().take(2))
        .map(|step| step.changes[0].controls[0].control)
        .collect();
    assert_eq!(last, ["brightness", "grain"]);
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// After reopening, the edit read is one step: Undo past the done stack moves the end, and
/// Redo reverses it. An edit there drops the step read, which Undo brings back.
#[test]
fn after_reopening_undo_moves_the_end_and_an_edit_drops_the_later_steps() {
    let path = scratch("history-reopen");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let reopened = driven.open(&path);
    let read = history(&reopened);
    assert_eq!((read.steps.len(), read.end), (1, 1));
    let read = read.steps[0].id;
    let undone = driven.run([Command::Undo]);
    assert_eq!(brightness(&undone), ControlValue::Number(0.0));
    let after = history(&undone);
    assert_eq!((after.steps.len(), after.end), (1, 0));
    assert_eq!((after.undo, after.redo), (None, Some(Move::Step(read))));
    let redone = driven.run([Command::Redo]);
    assert_eq!(brightness(&redone), ControlValue::Number(0.5));
    assert_eq!(history(&redone).end, 1);
    // At Original, an edit drops the step read; Undo brings it back, the end at 0.
    driven.run([Command::Undo]);
    let edited = driven.run([set("brightness", ControlValue::Number(2.0), true)]);
    let after = history(&edited);
    assert_eq!((after.steps.len(), after.end), (1, 1));
    assert_ne!(after.steps[0].id, read);
    let undone = driven.run([Command::Undo]);
    let after = history(&undone);
    assert_eq!((after.steps[0].id, after.end), (read, 0));
    assert_eq!(brightness(&undone), ControlValue::Number(0.0));
    let redone = driven.run([Command::Redo]);
    assert_eq!(history(&redone).steps.len(), 1);
    assert_eq!(brightness(&redone), ControlValue::Number(2.0));
    remove(&path);
}

/// Several commands as one `Step`: one step of its source and note, which one Undo takes
/// back whole.
#[test]
fn a_step_is_one_step_of_its_source_and_note() {
    let path = scratch("history-step");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    driven.open(&path);
    let note = Note::Text("Brighter, and softer".to_owned());
    let events = driven.run([Command::Step {
        source: StepSource::Assistant,
        note: Some(note.clone()),
        commands: vec![
            set("brightness", ControlValue::Number(0.5), true),
            amount(30.0, true),
        ],
    }]);
    assert_eq!(kinds(&events), ["values", "history", "frame"]);
    let made = history(&events);
    assert_eq!((made.steps.len(), made.end), (1, 1));
    let step = &made.steps[0];
    assert_eq!(
        (step.source, &step.note),
        (StepSource::Assistant, &Some(note))
    );
    let modules: Vec<_> = step.changes.iter().map(|change| change.module).collect();
    assert_eq!(modules, ["sample", "soften"]);
    assert_eq!(fold(&driven), edit(&driven));
    driven.run([Command::Undo]);
    assert_eq!(edit(&driven), Edit::default());
    remove(&path);
}

/// One command of a `Step` refused refuses the whole, naming its index: nothing changes, and
/// no step is made. A command that is not a change to the edit's values is refused so.
#[test]
fn a_step_is_refused_whole_naming_its_index() {
    let path = scratch("history-step-refused");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let edited = edit(&driven);
    let step = |commands| Command::Step {
        source: StepSource::You,
        note: None,
        commands,
    };
    let events = driven.run([step(vec![
        set("brightness", ControlValue::Number(0.8), true),
        set("grain", ControlValue::Number(20.0), true),
        set("nothing", ControlValue::Number(1.0), true),
    ])]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::InStep { index: 2, error }]
                if matches!(**error, SessionError::Edit(EditError::UnknownControl { .. }))
        ),
        "{events:?}"
    );
    assert_eq!(kinds(&events), ["failed", "values"]);
    assert_eq!(brightness(&events), ControlValue::Number(0.5));
    assert_eq!(edit(&driven), edited);
    let events = driven.run([step(vec![
        Command::ResetModule {
            module: "sample".to_owned(),
        },
        Command::Undo,
    ])]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::InStep { index: 1, error }]
                if matches!(**error, SessionError::NotInStep)
        ),
        "{events:?}"
    );
    assert!(histories(&events).is_empty());
    assert_eq!(edit(&driven), edited);
    // Neither refusal wrote anything.
    assert!(driven.writes().is_empty());
    remove(&path);
}

/// The source `Default` is the session's alone: a client's `Step` of it is refused before
/// it touches the gesture, so a `Step` of a gesture's changes begins none, and no step of
/// `Default` follows.
#[test]
fn a_step_of_the_source_default_is_refused() {
    let path = scratch("history-step-default");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let events = driven.run([Command::Step {
        source: StepSource::Default,
        note: None,
        commands: vec![set("brightness", ControlValue::Number(0.5), false)],
    }]);
    assert!(
        matches!(failures(&events)[..], [SessionError::DefaultSource]),
        "{events:?}"
    );
    assert_eq!(brightness(&events), ControlValue::Number(0.0));
    assert!(histories(&events).is_empty());
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    let events = driven.run([set("grain", ControlValue::Number(10.0), true)]);
    let sources: Vec<_> = (history(&events).steps.iter())
        .map(|step| step.source)
        .collect();
    assert_eq!(sources, [StepSource::You]);
    remove(&path);
}

/// A `Step` holding nothing is refused before it touches the gesture: the drag under way
/// goes on, and its release is one step.
#[test]
fn an_empty_step_is_refused_and_the_gesture_goes_on() {
    let path = scratch("history-step-empty");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.4), false)]);
    let events = driven.run([Command::Step {
        source: StepSource::You,
        note: None,
        commands: Vec::new(),
    }]);
    assert!(
        matches!(failures(&events)[..], [SessionError::EmptyStep]),
        "{events:?}"
    );
    assert!(histories(&events).is_empty());
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_some());
    let events = driven.run([set("brightness", ControlValue::Number(0.6), true)]);
    let released = history(&events);
    assert_eq!(released.steps.len(), 1);
    let change = &released.steps[0].changes[0].controls[0];
    assert_eq!(
        (&change.before, &change.after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.6))
    );
    remove(&path);
}

/// A `Step` refused begins no gesture: one of a gesture's changes alone, refused at its
/// second, leaves no gesture and the edit as it was.
#[test]
fn a_refused_step_begins_no_gesture() {
    let path = scratch("history-step-no-gesture");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let events = driven.run([Command::Step {
        source: StepSource::Assistant,
        note: None,
        commands: vec![
            set("brightness", ControlValue::Number(0.5), false),
            set("nothing", ControlValue::Number(1.0), false),
        ],
    }]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::InStep { index: 1, .. }]
        ),
        "{events:?}"
    );
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    assert_eq!(edit(&driven), Edit::default());
    assert!(histories(&events).is_empty());
    remove(&path);
}

/// A `Step` of the gesture's own source going on with it, refused, leaves it under way: the
/// edit as the drag left it, no step, and its release one step from before the drag.
#[test]
fn a_refused_step_going_on_with_the_gesture_leaves_it_under_way() {
    let path = scratch("history-step-refused-going-on");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.4), false)]);
    let dragged = edit(&driven);
    let events = driven.run([Command::Step {
        source: StepSource::You,
        note: None,
        commands: vec![
            set("brightness", ControlValue::Number(0.5), false),
            set("nothing", ControlValue::Number(1.0), false),
        ],
    }]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::InStep { index: 1, .. }]
        ),
        "{events:?}"
    );
    assert!(histories(&events).is_empty(), "{events:?}");
    assert_eq!(edit(&driven), dragged);
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_some());
    let events = driven.run([set("brightness", ControlValue::Number(0.6), true)]);
    let released = history(&events);
    assert_eq!(released.steps.len(), 1);
    let change = &released.steps[0].changes[0].controls[0];
    assert_eq!(
        (&change.before, &change.after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.6))
    );
    remove(&path);
}

/// A `Step` of another source on the gesture's module, refused, still ends the gesture: the
/// drag a step of its own, and no gesture left under way.
#[test]
fn a_refused_step_of_another_source_ends_the_gesture() {
    let path = scratch("history-step-refused-other-source");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.4), false)]);
    let dragged = edit(&driven);
    let events = driven.run([Command::Step {
        source: StepSource::Auto,
        note: None,
        commands: vec![
            set("brightness", ControlValue::Number(0.5), false),
            set("nothing", ControlValue::Number(1.0), false),
        ],
    }]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::InStep { index: 1, .. }]
        ),
        "{events:?}"
    );
    let ended = history(&events);
    assert_eq!(ended.steps.len(), 1);
    let step = &ended.steps[0];
    let change = &step.changes[0].controls[0];
    assert_eq!(
        (step.source, &change.before, &change.after),
        (
            StepSource::You,
            &ControlValue::Number(0.0),
            &ControlValue::Number(0.4)
        )
    );
    assert_eq!(edit(&driven), dragged);
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    remove(&path);
}

/// A `Step` that would take in the gesture under way, refused, leaves the gesture a step of
/// its own, as a release refused does, and the edit as the gesture left it.
#[test]
fn a_refused_step_leaves_the_gesture_it_would_take_in_a_step_of_its_own() {
    let path = scratch("history-step-refused-release");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.4), false)]);
    let dragged = edit(&driven);
    let events = driven.run([Command::Step {
        source: StepSource::You,
        note: None,
        commands: vec![
            set("brightness", ControlValue::Number(0.5), true),
            set("nothing", ControlValue::Number(1.0), true),
        ],
    }]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::InStep { index: 1, .. }]
        ),
        "{events:?}"
    );
    let ended = history(&events);
    assert_eq!(ended.steps.len(), 1);
    let change = &ended.steps[0].changes[0].controls[0];
    assert_eq!(
        (&change.before, &change.after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.4))
    );
    assert_eq!(edit(&driven), dragged);
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    remove(&path);
}

/// One source per gesture: a gesture's change of another source or note on its module ends
/// the gesture as a step of its own and begins another, and a completed change of another
/// source or note does not take it in.
#[test]
fn a_gesture_has_one_source() {
    let path = scratch("history-gesture-source");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let brightness = |value, done| set("brightness", ControlValue::Number(value), done);
    let step = |source, note: Option<Note>, value, done| Command::Step {
        source,
        note,
        commands: vec![brightness(value, done)],
    };
    // Auto's drag, then the person's on the same control: two gestures.
    driven.run([step(StepSource::Auto, None, 0.2, false)]);
    driven.run([brightness(0.4, false)]);
    driven.run([brightness(0.6, true)]);
    // The person's drag, then an assistant's release of it: two steps.
    driven.run([brightness(0.8, false)]);
    let note = Note::Text("Brighter".to_owned());
    driven.run([step(StepSource::Assistant, Some(note.clone()), 1.0, true)]);
    // Auto's drag, then Auto's release with a note: two steps.
    driven.run([step(StepSource::Auto, None, 1.2, false)]);
    let events = driven.run([step(StepSource::Auto, Some(note.clone()), 1.4, true)]);
    let steps: Vec<_> = (history(&events).steps.iter())
        .map(|step| {
            let after = step.changes[0].controls[0].after.clone();
            (step.source, step.note.clone(), after)
        })
        .collect();
    let number = ControlValue::Number;
    assert_eq!(
        steps,
        [
            (StepSource::Auto, None, number(0.2)),
            (StepSource::You, None, number(0.6)),
            (StepSource::You, None, number(0.8)),
            (StepSource::Assistant, Some(note.clone()), number(1.0)),
            (StepSource::Auto, None, number(1.2)),
            (StepSource::Auto, Some(note), number(1.4)),
        ]
    );
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// A `Step` writing the module of the gesture under way takes it in: the drag and the
/// `Step` releasing it are one step. One that does not leaves the gesture a step of its
/// own first.
#[test]
fn a_step_takes_in_the_gesture_under_way_on_a_module_it_writes() {
    let path = scratch("history-step-gesture");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    driven.open(&path);
    for value in [0.2, 0.4] {
        driven.run([set("brightness", ControlValue::Number(value), false)]);
    }
    let you = |commands| Command::Step {
        source: StepSource::You,
        note: None,
        commands,
    };
    let events = driven.run([you(vec![
        set("brightness", ControlValue::Number(0.5), true),
        set("grain", ControlValue::Number(20.0), true),
    ])]);
    let released = history(&events);
    assert_eq!(released.steps.len(), 1);
    let controls = &released.steps[0].changes[0].controls;
    let names: Vec<_> = controls.iter().map(|change| change.control).collect();
    assert_eq!(names, ["brightness", "grain"]);
    assert_eq!(
        (&controls[0].before, &controls[0].after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.5))
    );
    assert!(driven.engine.open.as_ref().unwrap().gesture.is_none());
    // A gesture on Soften, then a `Step` on Sample alone: two steps.
    driven.run([amount(20.0, false)]);
    let events = driven.run([you(vec![set(
        "brightness",
        ControlValue::Number(0.7),
        true,
    )])]);
    let modules: Vec<_> = (history(&events).steps.iter())
        .map(|step| step.changes[0].module)
        .collect();
    assert_eq!(modules, ["sample", "soften", "sample"]);
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// A `Step` of `done: false` changes alone is a gesture's change of its source: it makes no
/// step, the next goes on with it, and the one ending it, or another command, makes the
/// gesture one step of that source.
#[test]
fn a_step_of_a_gesture_s_changes_alone_is_a_gesture_of_its_source() {
    let path = scratch("history-step-auto");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let auto = |value, done| Command::Step {
        source: StepSource::Auto,
        note: None,
        commands: vec![set("brightness", ControlValue::Number(value), done)],
    };
    for value in [0.2, 0.4] {
        let events = driven.run([auto(value, false)]);
        assert!(histories(&events).is_empty(), "{events:?}");
    }
    let events = driven.run([auto(0.6, true)]);
    let made = history(&events);
    assert_eq!(made.steps.len(), 1);
    let change = &made.steps[0].changes[0].controls[0];
    assert_eq!(
        (&change.before, &change.after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.6))
    );
    // Ended by another command, it is a step of its own, of its source.
    driven.run([auto(0.8, false)]);
    let events = driven.run([Command::Reset]);
    let sources: Vec<_> = (history(&events).steps.iter())
        .map(|step| step.source)
        .collect();
    assert_eq!(
        sources,
        [StepSource::Auto, StepSource::Auto, StepSource::You]
    );
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// `GoTo` moves the end just after a step, or to Original, rendering and saving the edit
/// there, and Undo takes it back. After it an edit drops the later steps, which Undo brings
/// back, then the end where `GoTo` found it. The end where it is changes nothing.
#[test]
fn go_to_moves_the_end_and_undo_brings_back_the_steps_an_edit_after_it_dropped() {
    let path = scratch("history-go-to");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let first = driven.run([set("brightness", ControlValue::Number(0.25), true)]);
    // Read now: a later frame takes its display buffer in turn.
    let first = frame(&driven.gpu, &first);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let last = driven.run([set("brightness", ControlValue::Number(0.75), true)]);
    let ids: Vec<StepId> = history(&last).steps.iter().map(|step| step.id).collect();
    driven.writes();
    let gone = driven.run([Command::GoTo { step: Some(ids[0]) }]);
    assert_eq!(kinds(&gone), ["values", "history", "frame"]);
    assert_eq!(brightness(&gone), ControlValue::Number(0.25));
    assert!(frame(&driven.gpu, &gone) == first);
    let after = history(&gone);
    assert_eq!((after.steps.len(), after.end), (3, 1));
    // Undo takes it back; with nothing undone, Redo would move the end forward a step.
    assert_eq!(
        (after.undo, after.redo),
        (Some(Move::GoTo), Some(Move::Step(ids[1])))
    );
    // Saved as a completed change: the file holds the edit at the end.
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    let read = sidecar::read(&sidecar, OPERATIONS, &[&Sample], &[]).unwrap();
    assert_eq!(read.edit, edit(&driven));
    let undone = driven.run([Command::Undo]);
    assert_eq!(brightness(&undone), ControlValue::Number(0.75));
    let after = history(&undone);
    assert_eq!((after.end, after.redo), (3, Some(Move::GoTo)));
    let redone = driven.run([Command::Redo]);
    assert_eq!(brightness(&redone), ControlValue::Number(0.25));
    assert_eq!(history(&redone).end, 1);
    // An edit there drops the later steps; Undo brings them back, then the end.
    let edited = driven.run([set("brightness", ControlValue::Number(1.0), true)]);
    let after = history(&edited);
    assert_eq!((after.steps.len(), after.end), (2, 2));
    let undone = driven.run([Command::Undo]);
    let after = history(&undone);
    let kept: Vec<StepId> = after.steps.iter().map(|step| step.id).collect();
    assert_eq!((kept, after.end), (ids.clone(), 1));
    assert_eq!(after.undo, Some(Move::GoTo));
    assert_eq!(brightness(&undone), ControlValue::Number(0.25));
    let undone = driven.run([Command::Undo]);
    assert_eq!(history(&undone).end, 3);
    assert_eq!(brightness(&undone), ControlValue::Number(0.75));
    // Original, then Original again, which changes nothing.
    let original = driven.run([Command::GoTo { step: None }]);
    assert_eq!(history(&original).end, 0);
    assert_eq!(edit(&driven), Edit::default());
    let nothing = driven.run([Command::GoTo { step: None }]);
    assert_eq!(kinds(&nothing), ["values"]);
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// `GoTo` naming a step not in the log is refused, and changes nothing.
#[test]
fn go_to_an_unknown_step_is_refused() {
    let path = scratch("history-go-to-unknown");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let set_one = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    let unknown = StepId(!history(&set_one).steps[0].id.0);
    let events = driven.run([Command::GoTo {
        step: Some(unknown),
    }]);
    assert!(
        matches!(
            failures(&events)[..],
            [SessionError::UnknownStep { step }] if *step == unknown
        ),
        "{events:?}"
    );
    assert!(histories(&events).is_empty());
    assert_eq!(brightness(&events), ControlValue::Number(0.5));
    remove(&path);
}

/// The history's commands are commands on the edit: refused while the photo is not live,
/// and on a photo open for viewing only, whose history is still sent.
#[test]
fn the_history_s_commands_are_refused_while_not_live_and_view_only() {
    let commands = || {
        [
            Command::Undo,
            Command::Redo,
            Command::GoTo { step: None },
            Command::Step {
                source: StepSource::You,
                note: None,
                commands: vec![set("brightness", ControlValue::Number(1.0), true)],
            },
        ]
    };
    let path = scratch("history-not-live");
    let mut driven = Driven::new(&[&Sample]);
    let (hold, begun, permit) = export_job::testing::Hold::new();
    driven.run([Command::HoldDecode(hold)]);
    driven.run([Command::Open(path.clone())]);
    driven.pump(|_, events| events.iter().any(|e| matches!(e, Event::History(_))));
    begun.recv_timeout(TEST_WAIT).expect("the decode begins");
    let events = driven.run(commands());
    let refused = failures(&events);
    assert_eq!(refused.len(), 4, "{events:?}");
    assert!(refused.iter().all(|e| matches!(e, SessionError::NotLive)));
    assert!(histories(&events).is_empty());
    permit.send(()).unwrap();
    driven.settle();
    remove(&path);

    let path = scratch("history-view-only");
    let sidecar = sidecar::path_for(&path);
    let xml = brightened("0.9.0").replace(r#"apertrust:version="1""#, r#"apertrust:version="2""#);
    std::fs::write(&sidecar, &xml).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    assert_eq!(history(&opened).steps.len(), 1);
    let events = driven.run(commands());
    let refused = failures(&events);
    assert_eq!(refused.len(), 4, "{events:?}");
    assert!(
        refused
            .iter()
            .all(|e| matches!(e, SessionError::ViewOnly(_)))
    );
    assert!(histories(&events).is_empty());
    assert_eq!(brightness(&events), ControlValue::Number(0.5));
    assert!(driven.writes().is_empty());
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), xml);
    remove(&path);
}
