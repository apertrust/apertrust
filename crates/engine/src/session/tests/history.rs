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

//! The session's history (decision 78): each completed change a step of each module it
//! touched, Undo and Redo, Reset a step, a control's quick repeats merged, and the `History`
//! fact.

use apertrust_modules::soften::Soften;

use super::*;
use crate::session::history::{MERGE_WITHIN, REVIEW};

/// The times a test sends its batches at, from now on, each later than the last: a
/// change sent a merge window [`Times::apart`] never merges into the step before it.
struct Times(Instant);

impl Times {
    fn new() -> Self {
        Times(Instant::now())
    }

    /// [`MERGE_WITHIN`] after the last.
    fn apart(&mut self) -> Instant {
        self.after(MERGE_WITHIN)
    }

    /// `time` after the last.
    fn after(&mut self, time: Duration) -> Instant {
        self.0 += time;
        self.0
    }
}

/// Undo or Redo moving the one step `id`.
fn single(id: StepId) -> Move {
    Move::Steps {
        first: id,
        count: 1,
    }
}

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
    assert_eq!(released.undo, Some(single(step.id)));
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
    // Apart, so the second does not merge into the first.
    let mut times = Times::new();
    let half = driven.run_at(
        times.apart(),
        [set("brightness", ControlValue::Number(0.5), true)],
    );
    let one = driven.run_at(
        times.apart(),
        [set("brightness", ControlValue::Number(1.0), true)],
    );
    let second = history(&one).steps[1].id;
    driven.writes();
    let undone = driven.run([Command::Undo]);
    assert_eq!(kinds(&undone), ["values", "history", "frame"]);
    assert_eq!(brightness(&undone), ControlValue::Number(0.5));
    assert!(frame(&driven.gpu, &undone) == frame(&driven.gpu, &half));
    let after = history(&undone);
    // Undo takes the step out of the log; Redo would put it back.
    assert_eq!((after.steps.len(), after.end), (1, 1));
    assert_eq!(after.redo, Some(single(second)));
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
    assert_eq!((&step.note, step.batch), (&Some(Note::reset()), None));
    // One step of each module the edit held, the switched one turned off.
    let modules: Vec<_> = step
        .changes
        .iter()
        .map(|c| (&*c.module, c.enabled))
        .collect();
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
    assert_eq!((after.undo, after.redo), (None, Some(single(read))));
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

/// Several commands as one `Step`, writing two modules: a step of each, in the order written,
/// of its source and note, sharing a batch, which one Undo takes back whole and one Redo
/// puts back.
#[test]
fn a_step_of_two_modules_is_two_steps_in_one_batch_which_one_undo_takes_back() {
    let path = scratch("history-step");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    driven.open(&path);
    let note = Note::Text("Brighter, and softer".to_owned());
    let events = driven.run([Command::Step {
        source: StepSource::Assistant,
        note: Some(note.clone()),
        commands: vec![
            amount(30.0, true),
            set("brightness", ControlValue::Number(0.5), true),
        ],
    }]);
    assert_eq!(kinds(&events), ["values", "history", "frame"]);
    let made = history(&events);
    assert_eq!((made.steps.len(), made.end), (2, 2));
    let modules: Vec<_> = (made.steps.iter())
        .map(|step| {
            let modules: Vec<_> = step.changes.iter().map(|change| &*change.module).collect();
            (step.source, step.note.clone(), modules)
        })
        .collect();
    let assistant = |module| (StepSource::Assistant, Some(note.clone()), vec![module]);
    assert_eq!(modules, [assistant("soften"), assistant("sample")]);
    let batch = made.steps[0].batch;
    assert!(batch.is_some());
    assert_eq!(made.steps[1].batch, batch);
    let both = Move::Steps {
        first: made.steps[0].id,
        count: 2,
    };
    assert_eq!(made.undo, Some(both));
    assert_eq!(fold(&driven), edit(&driven));
    let edited = edit(&driven);
    let undone = driven.run([Command::Undo]);
    assert_eq!(edit(&driven), Edit::default());
    let after = history(&undone);
    assert_eq!((after.steps.len(), after.redo), (0, Some(both)));
    driven.run([Command::Redo]);
    assert_eq!(edit(&driven), edited);
    // After reopening, with no undo records, Undo's fallback moves over the whole batch.
    driven.writes();
    let reopened = driven.open(&path);
    let read = history(&reopened);
    assert_eq!(read.end, 2);
    driven.run([Command::Undo]);
    assert_eq!(edit(&driven), Edit::default());
    remove(&path);
}

/// A `Step`'s steps follow the first of its commands that changed each module: one that
/// changes nothing counts as none, as on its own.
#[test]
fn a_step_s_command_that_changes_nothing_orders_none_of_its_steps() {
    let path = scratch("history-step-order");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    driven.open(&path);
    driven.run([amount(30.0, true)]);
    let events = driven.run([Command::Step {
        source: StepSource::You,
        note: None,
        commands: vec![
            amount(30.0, true),
            set("brightness", ControlValue::Number(0.5), true),
            amount(40.0, true),
        ],
    }]);
    let made = history(&events);
    let modules: Vec<_> = (made.steps[1..].iter())
        .map(|step| &*step.changes[0].module)
        .collect();
    assert_eq!(modules, ["sample", "soften"]);
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
    // Each batch apart, so no step merges into the one before it.
    let mut times = Times::new();
    let mut run = |command| driven.run_at(times.apart(), [command]);
    // Auto's drag, then the person's on the same control: two gestures.
    run(step(StepSource::Auto, None, 0.2, false));
    run(brightness(0.4, false));
    run(brightness(0.6, true));
    // The person's drag, then an assistant's release of it: two steps.
    run(brightness(0.8, false));
    let note = Note::Text("Brighter".to_owned());
    run(step(StepSource::Assistant, Some(note.clone()), 1.0, true));
    // Auto's drag, then Auto's release with a note: two steps.
    run(step(StepSource::Auto, None, 1.2, false));
    let events = run(step(StepSource::Auto, Some(note.clone()), 1.4, true));
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
        .map(|step| step.changes[0].module.clone())
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
    let mut times = Times::new();
    for value in [0.2, 0.4] {
        let events = driven.run_at(times.apart(), [auto(value, false)]);
        assert!(histories(&events).is_empty(), "{events:?}");
    }
    let events = driven.run_at(times.apart(), [auto(0.6, true)]);
    let made = history(&events);
    assert_eq!(made.steps.len(), 1);
    let change = &made.steps[0].changes[0].controls[0];
    assert_eq!(
        (&change.before, &change.after),
        (&ControlValue::Number(0.0), &ControlValue::Number(0.6))
    );
    // Ended by another command, it is a step of its own, of its source; apart, so it does
    // not merge into the step before it.
    driven.run_at(times.apart(), [auto(0.8, false)]);
    let events = driven.run_at(times.apart(), [Command::Reset]);
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
    // Apart, so none merges into the one before it.
    let mut times = Times::new();
    let mut brighten = |driven: &mut Driven, value| {
        driven.run_at(
            times.apart(),
            [set("brightness", ControlValue::Number(value), true)],
        )
    };
    let first = brighten(&mut driven, 0.25);
    // Read now: a later frame takes its display buffer in turn.
    let first = frame(&driven.gpu, &first);
    brighten(&mut driven, 0.5);
    let last = brighten(&mut driven, 0.75);
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
        (Some(Move::GoTo), Some(single(ids[1])))
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
    // A module version above Sample's: newer, and its step still read.
    let xml = brightened("0.9.0").replace(
        r#"apertrust:moduleVersion="1""#,
        r#"apertrust:moduleVersion="2""#,
    );
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

/// A control's quick repeats merge into one step, as darktable's undo merges a widget's:
/// each sent less than 2 s after the frame of the last, and less than 10 s after the step's
/// first. The merged step keeps its id and goes from before the first to after the last; a
/// reset of the control merges too. One Undo takes each merged step back whole.
#[test]
fn quick_changes_to_one_control_merge_into_one_step() {
    let path = scratch("history-merge");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let mut times = Times::new();
    let sixteenths = |value: f32| ControlValue::Number(value / 16.0);
    let mut nudge = |driven: &mut Driven, value: f32, after| {
        let command = set("brightness", sixteenths(value), true);
        driven.run_at(times.after(after), [command])
    };
    let quick = Duration::from_millis(1250);
    let first = history(&nudge(&mut driven, 1.0, MERGE_WITHIN)).steps[0].id;
    // 1.25 s apart, each frame published as its change is sent: merged, up to 8.75 s.
    for value in 2..=8 {
        let events = nudge(&mut driven, value as f32, quick);
        let merged = history(&events);
        let ids: Vec<StepId> = merged.steps.iter().map(|step| step.id).collect();
        assert_eq!((ids, merged.undo), (vec![first], Some(single(first))));
        let change = &merged.steps[0].changes[0].controls[0];
        assert_eq!(
            (&change.before, &change.after),
            (&ControlValue::Number(0.0), &sixteenths(value as f32))
        );
    }
    // 10 s after the step's first: a step of its own.
    let split = nudge(&mut driven, 9.0, quick);
    assert_eq!(history(&split).steps.len(), 2);
    // 2 s after the frame of the last: a step of its own.
    let reviewed = nudge(&mut driven, 10.0, REVIEW);
    assert_eq!(history(&reviewed).steps.len(), 3);
    // A reset of the control a second later merges into it.
    let reset = driven.run_at(
        times.after(Duration::from_secs(1)),
        [Command::ResetControl {
            module: "sample".to_owned(),
            control: "brightness".to_owned(),
        }],
    );
    assert_eq!(history(&reset).steps.len(), 3);
    assert_eq!(fold(&driven), edit(&driven));
    // One Undo each.
    for value in [9.0, 8.0, 0.0] {
        assert_eq!(brightness(&driven.run([Command::Undo])), sixteenths(value));
    }
    assert_eq!(kinds(&driven.run([Command::Undo])), ["values"]);
    remove(&path);
}

/// No merge across another control, another command, a note, another source or a `Step`
/// of several commands, each sent a quarter second after the last; Auto's quick repeats
/// merge as the person's do, and so do a preset row's.
#[test]
fn no_merge_across_another_command_control_note_or_source() {
    let path = scratch("history-no-merge");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let brightness = |value| set("brightness", ControlValue::Number(value), true);
    let step = |source, note, commands| Command::Step {
        source,
        note,
        commands,
    };
    let note = || Some(Note::Text("Brighter".to_owned()));
    let look = |option: &str| Command::ApplyPreset {
        module: "sample".to_owned(),
        preset: "look".to_owned(),
        option: option.to_owned(),
    };
    let release = Command::Keep {
        module: "sample".to_owned(),
        control: "brightness".to_owned(),
        kept: false,
    };
    // Each command, and how many steps the log holds after it.
    let sequence = [
        (brightness(0.1), 1),
        (set("grain", ControlValue::Number(10.0), true), 2),
        (brightness(0.2), 3),
        // Refused: a command between.
        (set("nothing", ControlValue::Number(1.0), true), 3),
        (brightness(0.3), 4),
        (release, 5),
        (brightness(0.35), 6),
        (step(StepSource::You, note(), vec![brightness(0.4)]), 7),
        (brightness(0.5), 8),
        (step(StepSource::Auto, None, vec![brightness(0.6)]), 9),
        (step(StepSource::Auto, None, vec![brightness(0.7)]), 9),
        (
            step(
                StepSource::You,
                None,
                vec![brightness(0.8), brightness(0.9)],
            ),
            10,
        ),
        (brightness(1.0), 11),
        (look("grainy"), 12),
        (look("untouched"), 12),
        (brightness(0.45), 13),
        // Refused before it touches the gesture, as a `Step` of `Default`: a command between.
        (step(StepSource::Default, None, vec![brightness(0.5)]), 13),
        (brightness(0.55), 14),
        (brightness(0.6), 14),
        // An empty `Step`, refused alike.
        (step(StepSource::You, None, Vec::new()), 14),
        (brightness(0.65), 15),
    ];
    let mut times = Times::new();
    for (index, (command, steps)) in sequence.into_iter().enumerate() {
        driven.run_at(times.after(Duration::from_millis(250)), [command]);
        let log = driven.engine.open.as_ref().unwrap().history.log();
        assert_eq!(log.steps().len(), steps, "command {index}");
    }
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

/// A change that changes nothing counts as none: a gesture's on another control, or a
/// `Set`'s, sets neither a target nor when the change began, and is no command between, so
/// the control's next change still merges into the step before it.
#[test]
fn a_change_that_changes_nothing_counts_as_none() {
    let path = scratch("history-merge-nothing");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let mut times = Times::new();
    let brightness = |value| set("brightness", ControlValue::Number(value), true);
    let black = || Command::Set {
        operation: "exposure".to_owned(),
        param: "black".to_owned(),
        value: Value::Float(0.01),
    };
    let steps = |driven: &Driven| {
        let open = driven.engine.open.as_ref().unwrap();
        open.history.log().steps().len()
    };
    // Grain kept at 10, then black set with a brightness, a step of each in a batch, then a
    // step of brightness alone.
    let grain = set("grain", ControlValue::Number(10.0), true);
    driven.run_at(times.apart(), [grain, black(), brightness(0.25)]);
    driven.run_at(times.apart(), [brightness(0.5)]);
    assert_eq!(steps(&driven), 4);
    // Grain dragged to where it is, then brightness: merged.
    let quick = Duration::from_millis(500);
    let still = set("grain", ControlValue::Number(10.0), false);
    let before = edit(&driven);
    driven.run_at(times.after(quick), [still]);
    assert_eq!(edit(&driven), before);
    driven.run_at(times.after(quick), [brightness(0.75)]);
    assert_eq!(steps(&driven), 4);
    // Black set to what it is, then brightness: merged.
    driven.run_at(times.after(quick), [black()]);
    driven.run_at(times.after(quick), [brightness(1.0)]);
    assert_eq!(steps(&driven), 4);
    assert_eq!(fold(&driven), edit(&driven));
    remove(&path);
}

// -- Saved in the sidecar (9.5) -----------------------------------------------------------

/// The modules of the tests that save: Sample, and Soften, which another program changes.
const SAVING: &[&dyn apertrust_modules::Module] = &[&Sample, &Soften];

/// The sidecar of `path` as this build reads it.
fn saved(path: &Path) -> sidecar::Loaded {
    sidecar::read(&sidecar::path_for(path), OPERATIONS, SAVING, &[]).unwrap()
}

/// The log is saved in the sidecar's version 2 with its end: reopening the photo shows
/// every step, those after the end too, and Undo and Redo move the end through them, a
/// step at a time, as their fallback does.
#[test]
fn reopened_the_history_shows_every_step_and_undo_and_redo_go_on() {
    let path = scratch("history-saved");
    let mut times = Times::new();
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    for value in [0.25, 0.5, 0.75] {
        let at = times.apart();
        driven.run_at(at, [set("brightness", ControlValue::Number(value), true)]);
    }
    // The end on the second step, the third kept after it, as Undo would not keep it.
    let second = fold_ids(&driven)[1];
    driven.run([Command::GoTo { step: Some(second) }]);
    assert_eq!(
        written(&driven.writes()).last(),
        Some(&(sidecar::path_for(&path).as_path(), true))
    );
    let file = std::fs::read_to_string(sidecar::path_for(&path)).unwrap();
    assert!(file.contains(r#"apertrust:version="2""#), "{file}");
    let ids: Vec<StepId> = fold_ids(&driven);
    assert_eq!(saved(&path).log.end(), 2);

    let reopened = driven.open(&path);
    let read = history(&reopened);
    let read_ids: Vec<StepId> = read.steps.iter().map(|step| step.id).collect();
    assert_eq!((read_ids, read.end), (ids.clone(), 2));
    assert_eq!(brightness(&reopened), ControlValue::Number(0.5));
    // No undo record survives: Undo and Redo move the end, one step each.
    assert_eq!(
        (read.undo, read.redo),
        (Some(single(ids[1])), Some(single(ids[2])))
    );
    let redone = driven.run([Command::Redo]);
    assert_eq!(brightness(&redone), ControlValue::Number(0.75));
    for (end, value) in [(2, 0.5), (1, 0.25), (0, 0.0)] {
        let undone = driven.run([Command::Undo]);
        assert_eq!(history(&undone).end, end);
        assert_eq!(brightness(&undone), ControlValue::Number(value));
    }
    // Each move saved: reopened again, at Original with every step kept.
    driven.writes();
    assert_eq!(saved(&path).log.end(), 0);
    let again = driven.open(&path);
    assert_eq!((history(&again).steps.len(), history(&again).end), (3, 0));
    remove(&path);
}

/// The ids of the open photo's steps.
fn fold_ids(driven: &Driven) -> Vec<StepId> {
    let open = driven.engine.open.as_ref().unwrap();
    open.history
        .log()
        .steps()
        .iter()
        .map(|step| step.id)
        .collect()
}

/// Each step and each move of the end advances the copy's clock vector with this machine's
/// clock, which the file saves (decision 78).
#[test]
fn each_step_and_each_move_advances_the_clock_vector() {
    let path = scratch("history-clocks");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let after_step = saved(&path);
    let step = after_step.log.steps()[0].clock;
    assert_eq!(after_step.clocks(), [step]);
    driven.run([Command::Undo]);
    driven.writes();
    let after_undo = saved(&path);
    let [moved] = after_undo.clocks() else {
        panic!("one machine: {:?}", after_undo.clocks());
    };
    assert_eq!(moved.machine, step.machine);
    assert!(*moved > step);
    remove(&path);
}

/// A version 1 file is read as a step of each module and written as version 2 at the next
/// change, its steps kept under the new one.
#[test]
fn a_version_1_file_is_written_as_version_2_at_the_next_change() {
    let path = scratch("history-version-1");
    let sidecar = sidecar::path_for(&path);
    std::fs::write(&sidecar, brightened("0.3.0")).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    assert_eq!(history(&opened).steps.len(), 1);
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    let file = std::fs::read_to_string(&sidecar).unwrap();
    assert!(file.contains(r#"apertrust:version="2""#), "{file}");
    let read = saved(&path);
    assert_eq!((read.log.steps().len(), read.log.end()), (2, 2));
    assert_eq!(read.edit, edit(&driven));
    remove(&path);
}

/// What `apertrust-cli edit` does: a step setting Soften's amount added to the file's log
/// after its end, written by another program, here at the unknown machine's clock a minute
/// after the session's.
fn add_elsewhere(path: &Path, value: f32) {
    let last = (saved(path).log.steps().iter().map(|step| step.clock)).max();
    let clock = Clock::at(
        last.unwrap().when + 60_000,
        None,
        "00000000".parse().unwrap(),
    );
    add_at(path, value, clock);
}

/// [`add_elsewhere`] at `clock`.
fn add_at(path: &Path, value: f32, clock: Clock) {
    let read = saved(path);
    let mut log = read.log.clone();
    let mut edit = log.fold();
    let amount = ControlValue::Number(value);
    (edit.set_control(&Soften, "amount", amount, &testing::image())).unwrap();
    let changes = crate::history::Change::between(&log.fold(), &edit, &[]);
    log.add_change(changes, StepSource::You, None, clock);
    let mut clocks = read.clocks().to_vec();
    crate::history::advance(&mut clocks, clock);
    let path = sidecar::path_for(path);
    sidecar::write_log(
        &path,
        &read.file,
        &log,
        &clocks,
        (OPERATIONS, SAVING),
        "cli",
    )
    .unwrap();
}

/// Soften's amount in `values`.
fn softened(values: &Values) -> ControlValue {
    control(values, "soften", "amount").value.clone()
}

/// A newer file on disk, as `apertrust-cli edit` writes: the writer finds it changed and
/// writes nothing; the session takes it in, lays its step not yet saved on the newer log
/// after its end, sends `Values` and `History` again and writes the joined log. The undo
/// records are gone with the log they describe: Undo moves the end.
#[test]
fn a_newer_file_is_taken_in_before_a_write_and_the_step_laid_on_it() {
    let path = scratch("history-newer-file");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    add_elsewhere(&path, 40.0);
    // The session's next change: not written over the newer file.
    let changed = driven.run([set("size", ControlValue::Choice(2), true)]);
    let ours = history(&changed).steps.last().unwrap().id;
    assert!(driven.writes().is_empty());
    assert_eq!(saved(&path).build.as_deref(), Some("cli"));
    // Taken in: its step, then the session's, the edit holding both.
    let taken = driven.engine.newer_file(false);
    assert_eq!(kinds(&taken), ["values", "history", "frame"]);
    let after = history(&taken);
    assert_eq!((after.steps.len(), after.end), (3, 3));
    assert_eq!(after.steps[2].id, ours);
    let values = values(&taken);
    assert_eq!(softened(values), ControlValue::Number(40.0));
    assert_eq!(
        control(values, "sample", "size").value,
        ControlValue::Choice(2)
    );
    assert_eq!(
        control(values, "sample", "brightness").value,
        ControlValue::Number(0.5)
    );
    // And written, the joined log.
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    let read = saved(&path);
    assert_eq!(read.log.steps().len(), 3);
    assert_eq!(read.edit, edit(&driven));
    assert_eq!(read.clocks().len(), 2, "this machine's and the other's");
    // No undo record: Undo moves the end over the session's step, and the next change is
    // written at once, the file as the session wrote it.
    let undone = driven.run([Command::Undo]);
    assert_eq!(history(&undone).end, 2);
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    assert!(driven.engine.newer_file(false).is_empty());
    remove(&path);
}

/// A newer file that holds nothing newer by its clock vector, every machine's clock at or
/// below the session's, as a copy of an older version of the file put back, is written over.
#[test]
fn a_file_holding_nothing_newer_is_written_over() {
    let path = scratch("history-older-file");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let older = std::fs::read(&sidecar).unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    driven.writes();
    // Put back, a little longer, so its size tells it changed whatever the clock's
    // resolution.
    let mut put_back = older.clone();
    put_back.extend_from_slice(b"\n");
    std::fs::write(&sidecar, &put_back).unwrap();
    driven.run([set("size", ControlValue::Choice(2), true)]);
    assert!(driven.writes().is_empty());
    // Nothing taken in, nothing to tell: the session's log written over it.
    assert!(driven.engine.newer_file(false).is_empty());
    assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
    assert_eq!(saved(&path).log.steps().len(), 3);
    assert_eq!(saved(&path).edit, edit(&driven));
    remove(&path);
}

/// As the photo closes, a newer file the last write found is taken in and the joined log
/// written, so the step it was to save is not lost.
#[test]
fn a_newer_file_found_as_the_photo_closes_is_taken_in() {
    let path = scratch("history-newer-close");
    let other = scratch("history-newer-close-other");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    add_elsewhere(&path, 40.0);
    // A drag never released, made a step as the photo closes.
    driven.run([set("size", ControlValue::Choice(2), false)]);
    driven.open(&other);
    driven.writes();
    let read = saved(&path);
    assert_eq!(read.log.steps().len(), 3);
    let soften = &read.edit.modules["soften"];
    assert_eq!(soften.get("amount"), Some(&ControlValue::Number(40.0)));
    let sample = &read.edit.modules["sample"];
    assert_eq!(sample.get("size"), Some(&ControlValue::Choice(2)));
    remove(&path);
    remove(&other);
}

/// A newer file a newer Apertrust wrote, found before a write: shown as it reads, for
/// viewing only, and never written over.
#[test]
fn a_newer_build_s_file_found_before_a_write_makes_the_photo_view_only() {
    let path = scratch("history-newer-build");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let newer = brightened("9.0.0").replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
    std::fs::write(&sidecar, &newer).unwrap();
    // Two changes, the second's write handed over before the first's finds the file.
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    driven.run([set("size", ControlValue::Choice(2), true)]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    // `Opened` again, now for viewing only, then what was not saved, then the values.
    let Some(Event::Opened(photo)) = taken.first() else {
        panic!("{taken:?}");
    };
    assert!(photo.view_only.is_some());
    assert!(
        matches!(
            failures(&taken)[..],
            [SessionError::NotSaved { error, .. }] if matches!(**error, SessionError::ViewOnly(_))
        ),
        "{taken:?}"
    );
    let values = values(&taken);
    assert!(values.modules[0].controls.iter().all(|c| !c.available));
    // Nothing handed over before it is written over the newer build's file.
    assert!(driven.writes().is_empty());
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), newer);
    let refused = driven.run([set("grain", ControlValue::Number(10.0), true)]);
    assert!(matches!(
        failures(&refused)[..],
        [SessionError::ViewOnly(_)]
    ));
    driven.open(&scratch("history-newer-build-other"));
    assert!(driven.writes().is_empty());
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), newer);
    remove(&path);
}

/// Every [`sidecar::Problem`] opens the photo for viewing only, saying so, not as a newer
/// build's: its history sent, every change refused and the file never written.
#[test]
fn every_problem_opens_the_photo_for_viewing_only() {
    let mut driven = Driven::new(&[&Sample]);
    let path = scratch("history-problem-source");
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    driven.writes();
    let xml = std::fs::read_to_string(sidecar::path_for(&path)).unwrap();
    let id = format!(r#"apertrust:id="{}""#, saved(&path).log.steps()[0].id);
    // The value of the first `name` after `from` in place of its own.
    let set_after = |from: &str, name: &str, value: &str| {
        let at = xml.find(from).unwrap_or_else(|| panic!("{from}"));
        let field = format!(r#"{name}=""#);
        let start = at + xml[at..].find(&field).unwrap() + field.len();
        let end = start + xml[start..].find('"').unwrap();
        format!("{}{value}{}", &xml[..start], &xml[end..])
    };
    // `text` among the first step's attributes, after its id.
    let fields = |text: &str| xml.replacen(&id, &format!("{id} {text}"), 1);
    // `text` among the first step's elements, before its entries.
    let elements = |xml: &str, text: &str| {
        let at = xml.find(&id).unwrap();
        let entries = at + xml[at..].find("<apertrust:entries>").unwrap();
        format!("{}{text}{}", &xml[..entries], &xml[entries..])
    };
    let message = r#"apertrust:noteMessage="history-style""#;
    let cases = [
        ("step clock", set_after(&id, "apertrust:when", "never")),
        (
            "vector clock",
            set_after("<rdf:Bag>", "apertrust:machine", "zz"),
        ),
        (
            "two notes",
            fields(&format!(r#"apertrust:note="Words" {message}"#)),
        ),
        (
            "arguments without a message",
            elements(
                &xml,
                r#"<apertrust:noteArgs><rdf:Description arg:style="Film"/></apertrust:noteArgs>"#,
            ),
        ),
        (
            "note arguments",
            elements(
                &fields(message),
                r#"<apertrust:noteArgs><rdf:Description param:style="Film"/></apertrust:noteArgs>"#,
            ),
        ),
        (
            "unreadable note",
            elements(
                &xml,
                r#"<apertrust:note xml:lang="en">Words</apertrust:note>"#,
            ),
        ),
        (
            "step id",
            set_after("<apertrust:history>", "apertrust:id", "1"),
        ),
        ("batch", fields(r#"apertrust:batch="0f1e""#)),
        ("source", set_after(&id, "apertrust:source", "robot")),
        ("unknown field", fields(r#"apertrust:rating="2""#)),
        ("duplicate step", {
            // The first step's id the second's, the end's, so the end still names a step.
            let second = format!(r#"apertrust:id="{}""#, saved(&path).log.steps()[1].id);
            xml.replacen(&id, &second, 1)
        }),
        (
            "clock past vector",
            set_after("<rdf:Bag>", "apertrust:when", "2020-01-01T00:00:00.000Z"),
        ),
        (
            "end",
            set_after("<apertrust:copies>", "apertrust:end", "00000000000000ff"),
        ),
    ];
    for (name, damaged) in cases {
        let path = scratch(&format!("history-problem-{}", name.replace(' ', "-")));
        let sidecar = sidecar::path_for(&path);
        std::fs::write(&sidecar, &damaged).unwrap();
        let opened = driven.open(&path);
        let Some(Event::Opened(photo)) = opened.first() else {
            unreachable!()
        };
        let view_only = photo.view_only.as_ref().unwrap_or_else(|| panic!("{name}"));
        let sidecar::Reason::Unreadable(problem) = &view_only.reason else {
            panic!("{name}: {view_only:?}");
        };
        assert_eq!(
            crate::sidecar::tests::kind(problem),
            name,
            "{name}: {problem}"
        );
        assert!(!history(&opened).steps.is_empty(), "{name}");
        let refused = driven.run([
            set("brightness", ControlValue::Number(1.0), true),
            Command::Undo,
        ]);
        assert_eq!(failures(&refused).len(), 2, "{name}: {refused:?}");
        assert!(driven.writes().is_empty(), "{name}");
        driven.open(&scratch("history-problem-other"));
        assert!(driven.writes().is_empty(), "{name}");
        assert_eq!(
            std::fs::read_to_string(&sidecar).unwrap(),
            damaged,
            "{name}"
        );
        remove(&path);
    }
    remove(&path);
}

/// Another program on this machine, as `apertrust-cli edit`, shares its machine id: its
/// step is newer than what the session last read or wrote, though not than the session's
/// own next step, so it is taken in, never written over.
#[test]
fn this_machine_s_other_program_s_step_is_taken_in() {
    let path = scratch("history-same-machine");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let machine = apertrust_library::clock::MachineId::current();
    let last = (saved(&path).log.steps().iter().map(|step| step.clock)).max();
    add_at(&path, 40.0, Clock::now(last.as_ref(), machine));
    // The session's step, after the other program's, on the same machine.
    let changed = driven.run([set("size", ControlValue::Choice(2), true)]);
    assert!(history(&changed).steps.last().unwrap().clock.machine == machine);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert_eq!(softened(values(&taken)), ControlValue::Number(40.0));
    driven.writes();
    let read = saved(&path);
    assert_eq!(read.log.steps().len(), 3);
    assert_eq!(
        read.edit.modules["soften"].get("amount"),
        Some(&ControlValue::Number(40.0))
    );
    remove(&path);
}

/// Another program's write during a drag, then the drag cancelled: the cancel's held write
/// finds the file changed and writes nothing, and the next change is laid on the file
/// taken in, so the other program's step survives.
#[test]
fn a_cancel_after_another_program_s_write_takes_its_file_in() {
    let path = scratch("history-held-newer");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    driven.run([set("grain", ControlValue::Number(30.0), false)]);
    add_elsewhere(&path, 40.0);
    driven.run([cancel()]);
    assert!(
        driven.writes().is_empty(),
        "nothing written or reported saved"
    );
    driven.engine.newer_file(false);
    driven.run([set("size", ControlValue::Choice(2), true)]);
    driven.writes();
    let read = saved(&path);
    assert_eq!(
        read.edit.modules["soften"].get("amount"),
        Some(&ControlValue::Number(40.0))
    );
    assert_eq!(
        read.edit.modules["sample"].get("size"),
        Some(&ControlValue::Choice(2))
    );
    remove(&path);
}

/// A newer file that cannot be read is never written over: reported once, each later write
/// finds it again and writes nothing, and once it reads, it is taken in.
#[test]
fn an_unreadable_newer_file_is_never_written_over() {
    let path = scratch("history-unreadable");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let readable = std::fs::read_to_string(&sidecar).unwrap();
    std::fs::write(&sidecar, "half a file <x:xmp").unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(
        matches!(failures(&taken)[..], [SessionError::NotSaved { .. }]),
        "{taken:?}"
    );
    // Found again at the next write, which is answered too, the grain at 40 % included,
    // so a client waiting for that write stops waiting.
    driven.run([set("grain", ControlValue::Number(40.0), true)]);
    assert!(driven.writes().is_empty());
    let again = driven.engine.newer_file(false);
    let through = driven.engine.applied;
    assert!(
        matches!(failures(&again)[..], [SessionError::NotSaved { through: t, .. }] if *t == through),
        "{again:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&sidecar).unwrap(),
        "half a file <x:xmp"
    );
    // Readable again, with another program's step: taken in at the next write.
    std::fs::write(&sidecar, readable).unwrap();
    add_elsewhere(&path, 40.0);
    driven.run([set("size", ControlValue::Choice(2), true)]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(failures(&taken).is_empty(), "{taken:?}");
    driven.writes();
    let read = saved(&path);
    let sample = &read.edit.modules["sample"];
    assert_eq!(sample.get("grain"), Some(&ControlValue::Number(40.0)));
    assert_eq!(
        read.edit.modules["soften"].get("amount"),
        Some(&ControlValue::Number(40.0))
    );
    remove(&path);
}

/// Undo, Redo and `GoTo` hold the edit they move to to the ranges its modules' values
/// give, as the photo's open does, the clamp a change not yet a step, which never becomes a
/// step of its own: here a hand-written log of coarseness at 150 %, then the bias at 20 %,
/// which allows 120 %. Two Undos reach Original, two Redos come back, each clamped.
#[test]
fn a_move_of_the_end_clamps_what_it_moves_to() {
    let path = scratch("history-move-clamp");
    let value = |bias: Option<f32>| {
        let mut sample = apertrust_modules::ModuleValues::new();
        sample.set("coarseness", ControlValue::Number(150.0));
        if let Some(bias) = bias {
            sample.set("bias", ControlValue::Number(bias));
        }
        let mut edit = Edit::default();
        edit.modules.insert("sample".to_owned(), sample);
        edit
    };
    let mut log = crate::history::Log::default();
    let clock = Clock::at(1_773_496_931_120, None, "00000000".parse().unwrap());
    let (coarse, biased) = (value(None), value(Some(20.0)));
    for (before, after) in [(&Edit::default(), &coarse), (&coarse, &biased)] {
        let changes = crate::history::Change::between(before, after, &[]);
        log.add_change(changes, StepSource::You, None, clock);
    }
    let xml = sidecar::log_xml(
        &sidecar::Edits::new(""),
        &log,
        &[clock],
        (OPERATIONS, &[&Sample]),
        "hand",
    );
    std::fs::write(sidecar::path_for(&path), xml).unwrap();
    let mut driven = Driven::new(&[&Sample]);
    let opened = driven.open(&path);
    let coarseness = |events: &[Event]| {
        control(values(events), "sample", "coarseness")
            .value
            .clone()
    };
    assert_eq!(coarseness(&opened), ControlValue::Number(120.0));
    for (command, end, expected) in [
        (Command::Undo, 1, 150.0),
        (Command::Undo, 0, 100.0),
        (Command::Redo, 1, 150.0),
        (Command::Redo, 2, 120.0),
    ] {
        let moved = driven.run([command.clone()]);
        let history = history(&moved);
        assert_eq!((history.steps.len(), history.end), (2, end), "{command:?}");
        assert_eq!(
            coarseness(&moved),
            ControlValue::Number(expected),
            "{command:?}"
        );
    }
    // The clamp no step: none as the photo closes either.
    driven.open(&scratch("history-move-clamp-other"));
    driven.writes();
    assert_eq!(saved(&path).log.steps().len(), 2);
    remove(&path);
}

/// Steps kept while the file could not be read reach it as the photo closes once it reads:
/// the close hands over a write, which finds the file and takes it in.
#[test]
fn steps_kept_for_an_unreadable_file_are_saved_as_the_photo_closes() {
    let path = scratch("history-unreadable-close");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let readable = std::fs::read_to_string(&sidecar).unwrap();
    std::fs::write(&sidecar, "half a file <x:xmp").unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    driven.writes();
    let taken = driven.engine.newer_file(false);
    assert!(
        matches!(failures(&taken)[..], [SessionError::NotSaved { through, .. }] if *through == driven.engine.applied),
        "{taken:?}"
    );
    std::fs::write(&sidecar, readable).unwrap();
    driven.open(&scratch("history-unreadable-close-other"));
    driven.writes();
    let read = saved(&path);
    assert_eq!(
        read.edit.modules["sample"].get("grain"),
        Some(&ControlValue::Number(30.0))
    );
    remove(&path);
}

/// A file still unreadable as the photo closes: the close tells it, as the photo's last
/// word, which `Open` passes on.
#[test]
fn an_unreadable_file_as_the_photo_closes_is_told() {
    let path = scratch("history-unreadable-closed");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    std::fs::write(&sidecar, "half a file <x:xmp").unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), false)]);
    let other = scratch("history-unreadable-closed-other");
    let events = driven.run([Command::Open(other.clone())]);
    assert!(
        matches!(failures(&events)[..], [SessionError::NotSaved { path: p, .. }] if *p == sidecar),
        "{events:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&sidecar).unwrap(),
        "half a file <x:xmp"
    );
    driven.settle();
    remove(&path);
    remove(&other);
}

/// What another tool wrote beside the edit after the session's last write, as a rating
/// added by hand, is kept when the file holds nothing newer of the history.
#[test]
fn a_rating_added_by_hand_survives_the_next_save() {
    let path = scratch("history-rating");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let xml = std::fs::read_to_string(&sidecar).unwrap();
    let rated = xml.replacen(
        r#"xmlns:arg="https://apertrust.org/ns/xmp/arg/""#,
        r#"xmlns:arg="https://apertrust.org/ns/xmp/arg/"
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmp:Rating="4""#,
        1,
    );
    assert_ne!(rated, xml);
    std::fs::write(&sidecar, rated).unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    driven.writes();
    driven.engine.newer_file(false);
    driven.writes();
    let written = std::fs::read_to_string(&sidecar).unwrap();
    assert!(written.contains("<xmp:Rating>4</xmp:Rating>"), "{written}");
    assert_eq!(
        saved(&path).edit.modules["sample"].get("grain"),
        Some(&ControlValue::Number(30.0))
    );
    remove(&path);
}

/// An Undo handed over just after another program added a step is made again on the file
/// taken in: the end goes back to the step the session's end is on, so the Undo stays
/// undone, the other program's step after it.
#[test]
fn an_undo_not_yet_saved_stays_undone_over_another_program_s_step() {
    let path = scratch("history-undo-newer");
    let mut times = Times::new();
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run_at(
        times.apart(),
        [set("brightness", ControlValue::Number(0.5), true)],
    );
    driven.run_at(
        times.apart(),
        [set("brightness", ControlValue::Number(1.0), true)],
    );
    driven.writes();
    add_elsewhere(&path, 40.0);
    let undone = driven.run([Command::Undo]);
    assert_eq!(brightness(&undone), ControlValue::Number(0.5));
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(failures(&taken).is_empty(), "{taken:?}");
    let after = history(&taken);
    assert_eq!((after.steps.len(), after.end), (3, 1));
    assert_eq!(
        control(values(&taken), "sample", "brightness").value,
        ControlValue::Number(0.5)
    );
    driven.writes();
    let read = saved(&path);
    assert_eq!((read.log.steps().len(), read.log.end()), (3, 1));
    remove(&path);
}

/// Reported once while the photo is edited, the unreadable file is told again as the photo
/// closes after a completed change: that change's write is answered too.
#[test]
fn an_unreadable_file_reported_while_editing_is_told_again_as_the_photo_closes() {
    let path = scratch("history-unreadable-again");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    std::fs::write(&sidecar, "half a file <x:xmp").unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    driven.writes();
    let first = driven.engine.newer_file(false);
    assert!(
        matches!(failures(&first)[..], [SessionError::NotSaved { .. }]),
        "{first:?}"
    );
    driven.run([set("size", ControlValue::Choice(2), true)]);
    let other = scratch("history-unreadable-again-other");
    let events = driven.run([Command::Open(other.clone())]);
    assert!(
        matches!(failures(&events)[..], [SessionError::NotSaved { path: p, .. }] if *p == sidecar),
        "{events:?}"
    );
    driven.settle();
    remove(&path);
    remove(&other);
}

/// Another program's history written over the file: `steps` of the file's log as it reads,
/// ending after `end` of them, then its own step setting Soften's amount, at the unknown
/// machine's clock a minute after the file's latest.
fn rewrite_elsewhere(path: &Path, steps: &[usize], end: usize) {
    let read = saved(path);
    let kept: Vec<_> = steps
        .iter()
        .map(|at| read.log.steps()[*at].clone())
        .collect();
    let mut log = crate::history::Log::from_steps(kept, end);
    let mut edit = log.fold();
    let amount = ControlValue::Number(40.0);
    (edit.set_control(&Soften, "amount", amount, &testing::image())).unwrap();
    let changes = crate::history::Change::between(&log.fold(), &edit, &[]);
    let last = (read.log.steps().iter().map(|step| step.clock))
        .max()
        .unwrap();
    let clock = Clock::at(last.when + 60_000, None, "00000000".parse().unwrap());
    log.add_change(changes, StepSource::You, None, clock);
    let mut clocks = read.clocks().to_vec();
    crate::history::advance(&mut clocks, clock);
    let path = sidecar::path_for(path);
    sidecar::write_log(
        &path,
        &read.file,
        &log,
        &clocks,
        (OPERATIONS, SAVING),
        "cli",
    )
    .unwrap();
}

/// The session's two steps, a merge window apart, saved.
fn two_steps(driven: &mut Driven) {
    let mut times = Times::new();
    driven.run_at(
        times.apart(),
        [set("brightness", ControlValue::Number(0.5), true)],
    );
    driven.run_at(
        times.apart(),
        [set("brightness", ControlValue::Number(1.0), true)],
    );
    driven.writes();
}

/// An Undo not yet saved, where another program dropped the step it undid and added its
/// own: the move cannot be made again, so the newer end stands, and the person is told.
#[test]
fn an_undo_over_a_step_another_program_dropped_is_not_kept_and_told() {
    let path = scratch("history-move-not-kept");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    two_steps(&mut driven);
    // The other program's log: the first step, then its own, the second dropped.
    rewrite_elsewhere(&path, &[0], 1);
    driven.run([Command::Undo]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(
        matches!(failures(&taken)[..], [SessionError::MoveNotKept { .. }]),
        "{taken:?}"
    );
    let after = history(&taken);
    assert_eq!((after.steps.len(), after.end), (2, 2));
    assert_eq!(softened(values(&taken)), ControlValue::Number(40.0));
    remove(&path);
}

/// The same Undo where the other program's end is on the session's end's step already, as
/// after an Undo of its own: kept, nothing to tell.
#[test]
fn an_undo_the_other_program_made_too_is_kept() {
    let path = scratch("history-move-same");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    two_steps(&mut driven);
    // The other program's: both steps, its end moved back on the first, as its own Undo
    // leaves it.
    let read = saved(&path);
    let mut log = read.log.clone();
    log.set_end(1);
    let last = (log.steps().iter().map(|step| step.clock)).max().unwrap();
    let mut clocks = read.clocks().to_vec();
    crate::history::advance(
        &mut clocks,
        Clock::at(last.when + 60_000, None, "00000000".parse().unwrap()),
    );
    let file = sidecar::path_for(&path);
    sidecar::write_log(
        &file,
        &read.file,
        &log,
        &clocks,
        (OPERATIONS, SAVING),
        "cli",
    )
    .unwrap();
    driven.run([Command::Undo]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(failures(&taken).is_empty(), "{taken:?}");
    assert_eq!(history(&taken).end, 1);
    remove(&path);
}

/// An Undo then an edit before the file is taken in: the edit's step is laid on the newer
/// log, under which the step the Undo took out applies again, and the person is told.
#[test]
fn an_undo_then_an_edit_over_another_program_s_step_is_told() {
    let path = scratch("history-undo-edit");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    two_steps(&mut driven);
    add_elsewhere(&path, 40.0);
    driven.run([Command::Undo]);
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(
        matches!(failures(&taken)[..], [SessionError::MoveNotKept { .. }]),
        "{taken:?}"
    );
    let after = history(&taken);
    assert_eq!((after.steps.len(), after.end), (4, 4));
    remove(&path);
}

/// A quick repeat of a control merged into its step, which keeps the step's id with a new
/// clock, after another program wrote the file: the merged step is laid on the newer log,
/// and nothing the session took out applies again, so nothing is told.
#[test]
fn a_merged_repeat_over_another_program_s_step_is_no_move_not_kept() {
    let path = scratch("history-merged-newer");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    let at = Instant::now();
    driven.run_at(at, [set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    add_elsewhere(&path, 40.0);
    let merged = driven.run_at(
        at + Duration::from_millis(100),
        [set("brightness", ControlValue::Number(0.6), true)],
    );
    assert_eq!(history(&merged).steps.len(), 1, "merged into the step");
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(failures(&taken).is_empty(), "{taken:?}");
    assert_eq!(brightness(&taken), ControlValue::Number(0.6));
    assert_eq!(softened(values(&taken)), ControlValue::Number(40.0));
    remove(&path);
}

/// Both sides undid the same step of the base, then the session made a change: the step
/// applies nowhere, so nothing is told.
#[test]
fn an_undo_both_sides_made_is_no_move_not_kept() {
    let path = scratch("history-both-undid");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    two_steps(&mut driven);
    // The other program's: its end back on the first step, the second after it.
    let read = saved(&path);
    let mut log = read.log.clone();
    log.set_end(1);
    let last = (log.steps().iter().map(|step| step.clock)).max().unwrap();
    let mut clocks = read.clocks().to_vec();
    let clock = Clock::at(last.when + 60_000, None, "00000000".parse().unwrap());
    crate::history::advance(&mut clocks, clock);
    let file = sidecar::path_for(&path);
    sidecar::write_log(
        &file,
        &read.file,
        &log,
        &clocks,
        (OPERATIONS, SAVING),
        "cli",
    )
    .unwrap();
    driven.run([Command::Undo]);
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(failures(&taken).is_empty(), "{taken:?}");
    assert_eq!(brightness(&taken), ControlValue::Number(0.5));
    remove(&path);
}

/// A file holding nothing newer of the first copy's history but another program's change
/// to another virtual copy: written over with that copy kept as the other program left it.
#[test]
fn another_program_s_change_to_another_copy_is_kept() {
    let path = scratch("history-other-copy");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let mut file = sidecar::Edits::parse(&std::fs::read_to_string(&sidecar).unwrap(), 0).unwrap();
    let mut other = file.copies[0].clone();
    other.id = "7c0e9a14".to_owned();
    other.number = "1".to_owned();
    other.name = "Warm".to_owned();
    file.copies.push(other.clone());
    std::fs::write(&sidecar, file.to_xml()).unwrap();
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(driven.writes().is_empty());
    assert!(driven.engine.newer_file(false).is_empty());
    driven.writes();
    let written = sidecar::Edits::parse(&std::fs::read_to_string(&sidecar).unwrap(), 0).unwrap();
    assert_eq!(written.copies.len(), 2);
    assert_eq!(written.copies[1], other);
    assert_eq!(
        saved(&path).edit.modules["sample"].get("grain"),
        Some(&ControlValue::Number(30.0))
    );
    remove(&path);
}

/// A Redo not yet saved of a step another program dropped, then a change laid on its file:
/// the redone step applies nowhere in the joined log, so the person is told.
#[test]
fn a_redo_of_a_step_another_program_dropped_is_told() {
    let path = scratch("history-redo-dropped");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    two_steps(&mut driven);
    let first = saved(&path).log.steps()[0].id;
    driven.run([Command::GoTo { step: Some(first) }]);
    driven.writes();
    // The other program's: the first step, then its own, the second dropped.
    rewrite_elsewhere(&path, &[0], 1);
    driven.run([Command::Redo]);
    driven.run([set("grain", ControlValue::Number(30.0), true)]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(
        matches!(failures(&taken)[..], [SessionError::MoveNotKept { .. }]),
        "{taken:?}"
    );
    remove(&path);
}

/// Steps laid on a newer file get a clock of their own in the copy's vector, later than
/// every clock the file and the steps had, so another program of this machine, whose base
/// held the file before, finds the written file newer, never written over by it.
#[test]
fn steps_laid_on_a_newer_file_advance_the_clock_vector() {
    let path = scratch("history-laid-clock");
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    add_elsewhere(&path, 40.0);
    let read = saved(&path);
    let changed = driven.run([set("size", ControlValue::Choice(2), true)]);
    let laid = history(&changed).steps.last().unwrap().clock;
    assert!(driven.writes().is_empty());
    driven.engine.newer_file(false);
    driven.writes();
    let written = saved(&path);
    let ours = (written.clocks().iter())
        .find(|clock| clock.machine == laid.machine)
        .copied()
        .unwrap();
    assert!(ours > laid, "{ours:?} after {laid:?}");
    assert!(
        read.clocks()
            .iter()
            .all(|theirs| ours > *theirs || theirs.machine != ours.machine)
    );
    remove(&path);
}

/// Another program that keeps changing the sidecar while the photo closes: each write the
/// close makes finds it changed again, and after a few rounds the close tells it. The other
/// program writes as each newer file is taken, so its write always lands between the take
/// and the write that follows it, whatever the timing.
#[test]
fn a_file_another_program_keeps_changing_as_the_photo_closes_is_told() {
    let path = scratch("history-still-changing");
    let sidecar = sidecar::path_for(&path);
    let mut driven = Driven::new(SAVING);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    // Two of the other program's files, of different sizes, written in turn.
    add_elsewhere(&path, 40.0);
    let one = std::fs::read_to_string(&sidecar).unwrap();
    add_elsewhere(&path, 60.0);
    let two = std::fs::read_to_string(&sidecar).unwrap();
    // A drag never released, which the close makes a step and writes.
    driven.run([set("grain", ControlValue::Number(30.0), false)]);
    let turns = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (taken, rewrites) = (Arc::clone(&turns), sidecar.clone());
    driven.engine.taking = Some(Box::new(move || {
        let turn = taken.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let xml = if turn % 2 == 0 { &one } else { &two };
        apertrust_base::fs::write(&rewrites, xml.as_bytes()).unwrap();
    }));
    let other = scratch("history-still-changing-other");
    let events = driven.run([Command::Open(other.clone())]);
    assert!(
        (failures(&events).iter()).any(|failure| matches!(failure, SessionError::NotSaved { error, .. } if matches!(**error, SessionError::StillChanging))),
        "{events:?}"
    );
    // Taken three times: the close's take, then twice more.
    assert_eq!(turns.load(std::sync::atomic::Ordering::Relaxed), 3);
    driven.engine.taking = None;
    driven.settle();
    remove(&path);
    remove(&other);
}
