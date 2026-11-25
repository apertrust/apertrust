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

//! `Command::SetTool` (decision 106): a tool's values set whole as one change, as a preset,
//! a style or an applied edit sets them.

use apertrust_tools::grain::Grain;
use apertrust_tools::soften::Soften;
use apertrust_tools::vignetting::Vignetting;

use super::*;

fn set_tool(tool: &str, values: ToolValues) -> Command {
    Command::SetTool {
        tool: tool.to_owned(),
        values,
    }
}

/// Values of `(control, value, mark)`, a value of `None` a mark alone.
fn values_of(set: &[(&str, Option<ControlValue>, Option<Mark>)]) -> ToolValues {
    let mut values = ToolValues::new();
    for (control, value, mark) in set {
        if let Some(value) = value {
            values.set(control, value.clone());
        }
        values.set_mark(control, *mark);
    }
    values
}

/// The open photo's edit, as the session holds it.
fn edit(driven: &Driven) -> Edit {
    driven.engine.open.as_ref().unwrap().edit.clone()
}

/// The one `History` among `events`.
fn history(events: &[Event]) -> &History {
    let mut histories = events.iter().filter_map(|event| match event {
        Event::History(history) => Some(history),
        _ => None,
    });
    let history = histories.next().expect("a History");
    assert!(histories.next().is_none(), "one History in {events:?}");
    history
}

/// Whether the tool `id` renders, as the values report it.
fn enabled(events: &[Event], id: &str) -> bool {
    (values(events).tools.iter())
        .find(|state| state.tool == id)
        .unwrap()
        .enabled
}

/// Each control `values` holds takes its value and its mark, a mark alone holding the
/// control at its default; every other goes back to its default, unmarked. One write saves
/// it.
#[test]
fn set_tool_sets_values_and_marks_and_takes_every_other_control_to_its_default() {
    let path = scratch("set-tool");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("grain", ControlValue::Number(20.0), true)]);
    driven.writes();
    let values = values_of(&[
        (
            "brightness",
            Some(ControlValue::Number(0.5)),
            Some(Mark::Kept),
        ),
        ("bias", Some(ControlValue::Number(40.0)), Some(Mark::Auto)),
        ("size", None, Some(Mark::Kept)),
    ]);
    let events = driven.run([set_tool("sample", values.clone())]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(edit(&driven).tool("sample"), Some(&values));
    let reported = super::values(&events);
    let state = |id| {
        let state = control(reported, "sample", id);
        (state.value.clone(), state.mark)
    };
    assert_eq!(state("grain"), (ControlValue::Number(0.0), None));
    assert_eq!(
        state("brightness"),
        (ControlValue::Number(0.5), Some(Mark::Kept))
    );
    assert_eq!(
        state("bias"),
        (ControlValue::Number(40.0), Some(Mark::Auto))
    );
    assert_eq!(state("size").1, Some(Mark::Kept));
    assert_eq!(driven.writes().len(), 1);
    remove(&path);
}

/// A switched tool is on or off as the values say, off holding its marks alone (decision
/// 32); one that is not switched stays on whatever they say.
#[test]
fn set_tool_turns_a_switched_tool_on_and_off() {
    let path = scratch("set-tool-switched");
    let mut driven = Driven::new(&[&Sample, &Soften]);
    let opened = driven.open(&path);
    assert!(!enabled(&opened, "soften"));
    let mut on = values_of(&[("amount", Some(ControlValue::Number(30.0)), None)]);
    on.set_enabled(true);
    let events = driven.run([set_tool("soften", on.clone())]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert!(enabled(&events, "soften"));
    assert_eq!(edit(&driven).tool("soften"), Some(&on));
    // Off, with a value and a mark: the mark alone is kept.
    let mut off = on.clone();
    off.set_enabled(false);
    off.set_mark("size", Some(Mark::Kept));
    let events = driven.run([set_tool("soften", off)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert!(!enabled(&events, "soften"));
    let marked = values_of(&[("size", None, Some(Mark::Kept))]);
    assert_eq!(edit(&driven).tool("soften"), Some(&marked));
    // Off and unmarked, the tool is untouched.
    driven.run([set_tool("soften", ToolValues::new())]);
    assert_eq!(edit(&driven).tool("soften"), None);
    // Sample is not switched: `enabled` is not its to set.
    let mut sample = values_of(&[("brightness", Some(ControlValue::Number(0.5)), None)]);
    sample.set_enabled(true);
    let events = driven.run([set_tool("sample", sample)]);
    assert!(enabled(&events, "sample"));
    let held = edit(&driven).tool("sample").cloned().unwrap();
    assert!(!held.enabled());
    assert_eq!(held.number("brightness"), Some(0.5));
    remove(&path);
}

/// What `SetControl` refuses, `SetTool` refuses alike, every value checked before any is
/// set: a tool or a control the session lacks, a value of another kind and a choice's option
/// the photo does not list (decision 30). A number outside its range, the definition's or
/// the one its tool's values give, is brought inside, as a saved edit's is as it is read,
/// where `SetControl` refuses it.
#[test]
fn set_tool_refuses_as_set_control_does_and_clamps_a_number_as_a_read_edit_s() {
    let path = scratch("set-tool-checked");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    let before = edit(&driven);
    let refusal = |events: &[Event]| match failures(events)[..] {
        [SessionError::Edit(error)] => error.clone(),
        _ => panic!("one refusal in {events:?}"),
    };
    let cases = [
        ("brightness", ControlValue::Bool(true)),
        ("method", ControlValue::Choice(CAMERA)),
        ("nothing", ControlValue::Number(1.0)),
    ];
    for (id, value) in cases {
        let by_set_tool = driven.run([set_tool(
            "sample",
            values_of(&[(id, Some(value.clone()), None)]),
        )]);
        let by_set_control = driven.run([set(id, value, true)]);
        assert_eq!(refusal(&by_set_tool), refusal(&by_set_control), "{id}");
        assert!(histories_in(&by_set_tool).is_empty(), "{id}");
        assert_eq!(edit(&driven), before, "{id}");
    }
    let unknown_mark = driven.run([set_tool(
        "sample",
        values_of(&[("nothing", None, Some(Mark::Kept))]),
    )]);
    assert!(matches!(
        refusal(&unknown_mark),
        EditError::UnknownControl { .. }
    ));
    // A valid value beside an unlisted option: refused whole, the valid one unset.
    let mixed = driven.run([set_tool(
        "sample",
        values_of(&[
            ("grain", Some(ControlValue::Number(30.0)), Some(Mark::Kept)),
            ("method", Some(ControlValue::Choice(CAMERA)), None),
        ]),
    )]);
    assert!(matches!(refusal(&mixed), EditError::NotListed { .. }));
    assert_eq!(edit(&driven), before);
    let unknown = driven.run([set_tool("nothing", ToolValues::new())]);
    assert_eq!(
        refusal(&unknown),
        EditError::UnknownTool("nothing".to_owned())
    );
    assert_eq!(edit(&driven), before);
    assert!(driven.writes().is_empty());

    // Grain past its range's top is 100 %; coarseness past the 120 % a bias of 20 % gives
    // is 120 %, its mark as given.
    let events = driven.run([set_tool(
        "sample",
        values_of(&[
            ("grain", Some(ControlValue::Number(150.0)), Some(Mark::Kept)),
            ("bias", Some(ControlValue::Number(20.0)), Some(Mark::Kept)),
            (
                "coarseness",
                Some(ControlValue::Number(150.0)),
                Some(Mark::Auto),
            ),
        ]),
    )]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let held = edit(&driven).tool("sample").cloned().unwrap();
    assert_eq!(held.number("grain"), Some(100.0));
    assert_eq!(
        (held.number("coarseness"), held.mark("coarseness")),
        (Some(120.0), Some(Mark::Auto))
    );
    remove(&path);
}

/// The `History` facts among `events`.
fn histories_in(events: &[Event]) -> Vec<&History> {
    (events.iter())
        .filter_map(|event| match event {
            Event::History(history) => Some(history),
            _ => None,
        })
        .collect()
}

/// One step of the tool, source You with no note, which Undo takes back exactly and Redo
/// puts back.
#[test]
fn set_tool_is_one_step_that_undo_takes_back_exactly() {
    let path = scratch("set-tool-undo");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    driven.run([
        set("grain", ControlValue::Number(20.0), true),
        set("size", ControlValue::Choice(2), true),
        Command::Keep {
            tool: "sample".to_owned(),
            control: "bias".to_owned(),
            kept: true,
        },
    ]);
    let before = edit(&driven);
    let values = values_of(&[(
        "brightness",
        Some(ControlValue::Number(0.5)),
        Some(Mark::Kept),
    )]);
    let events = driven.run([set_tool("sample", values.clone())]);
    let made = history(&events);
    assert_eq!((made.steps.len(), made.end), (4, 4));
    let step = made.steps.last().unwrap();
    let tools: Vec<_> = step.changes.iter().map(|change| &*change.tool).collect();
    assert_eq!(
        (step.source, &step.note, step.batch, tools),
        (StepSource::You, &None, None, vec!["sample"])
    );
    driven.run([Command::Undo]);
    assert_eq!(edit(&driven), before);
    driven.run([Command::Redo]);
    assert_eq!(edit(&driven).tool("sample"), Some(&values));
    remove(&path);
}

/// A style's tools in one `Step`: a step of each, sharing a batch and the note, which one
/// Undo takes back.
#[test]
fn set_tools_in_one_step_share_a_batch_that_one_undo_takes_back() {
    let path = scratch("set-tool-step");
    let mut driven = Driven::new(&[&Soften, &Grain, &Vignetting]);
    driven.open(&path);
    let before = edit(&driven);
    let switched_on = |control: &str, value: f32| {
        let mut values = values_of(&[(control, Some(ControlValue::Number(value)), None)]);
        values.set_enabled(true);
        values
    };
    let note = Note::Text("Warm film".to_owned());
    let events = driven.run([Command::Step {
        source: StepSource::Style,
        note: Some(note.clone()),
        commands: vec![
            set_tool("soften", switched_on("amount", 30.0)),
            set_tool("grain", switched_on("strength", 40.0)),
            set_tool("vignette", switched_on("scale", 50.0)),
        ],
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let made = history(&events);
    let steps: Vec<_> = (made.steps.iter())
        .map(|step| {
            let tools: Vec<_> = step.changes.iter().map(|change| &*change.tool).collect();
            (step.source, step.note.clone(), tools)
        })
        .collect();
    let style = |tool| (StepSource::Style, Some(note.clone()), vec![tool]);
    assert_eq!(steps, [style("soften"), style("grain"), style("vignette")]);
    let batch = made.steps[0].batch;
    assert!(batch.is_some());
    assert!(made.steps.iter().all(|step| step.batch == batch));
    let all = Move::Steps {
        first: made.steps[0].id,
        count: 3,
        photos: 1,
    };
    assert_eq!(made.undo, Some(all));
    driven.run([Command::Undo]);
    assert_eq!(edit(&driven), before);
    remove(&path);
}
