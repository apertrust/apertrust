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

//! The styles applied to a photo (decision 106): `ApplyStyle` and `ClearStyle`, each one
//! change of source Style, and the list the log holds, which Undo, Redo, `GoTo`, Compress,
//! reopening and a newer file taken in move with the tools.

use apertrust_tools::grain::Grain;
use apertrust_tools::soften::Soften;
use apertrust_tools::vignetting::Vignetting;

use super::*;
use crate::history::{AppliedStyle, Change, Log, StyleTool};

/// The tools of these tests, each switched, so a style turns it on.
const TOOLS: &[&dyn ProcessingTool] = &[&Grain, &Soften, &Vignetting];

/// A switched tool on, with `control` at `value`.
fn on(control: &str, value: f32) -> ToolValues {
    let mut values = ToolValues::new();
    values.set(control, ControlValue::Number(value));
    values.set_enabled(true);
    values
}

fn grain(strength: f32) -> (&'static str, ToolValues) {
    ("grain", on("strength", strength))
}

fn soften(amount: f32) -> (&'static str, ToolValues) {
    ("soften", on("amount", amount))
}

fn vignette(scale: f32) -> (&'static str, ToolValues) {
    ("vignette", on("scale", scale))
}

/// `ApplyStyle` of the style `name`, its file `<name>.apstyle`, setting `tools`.
fn apply(name: &str, mode: StyleMode, tools: &[(&str, ToolValues)]) -> Command {
    Command::ApplyStyle {
        name: name.to_owned(),
        file: format!("{name}.apstyle"),
        mode,
        tools: (tools.iter())
            .map(|(tool, values)| ((*tool).to_owned(), values.clone()))
            .collect(),
    }
}

fn append(name: &str, tools: &[(&str, ToolValues)]) -> Command {
    apply(name, StyleMode::Append, tools)
}

/// `ClearStyle` of the style `name`, its file `<name>.apstyle`.
fn clear(name: &str) -> Command {
    Command::ClearStyle {
        name: name.to_owned(),
        file: format!("{name}.apstyle"),
    }
}

/// The person's `SetTool`.
fn set_tool((tool, values): (&str, ToolValues)) -> Command {
    Command::SetTool {
        tool: tool.to_owned(),
        values,
    }
}

fn edit(driven: &Driven) -> Edit {
    driven.engine.open.as_ref().unwrap().edit.clone()
}

fn log(driven: &Driven) -> Log {
    driven.engine.open.as_ref().unwrap().history.log().clone()
}

/// The open photo's styles applied, as its history holds them.
fn styles(driven: &Driven) -> Vec<AppliedStyle> {
    log(driven).styles().to_vec()
}

/// The names of the styles applied the values report, newest first.
fn listed(events: &[Event]) -> Vec<&str> {
    let values = values(events);
    values
        .styles
        .iter()
        .map(|style| style.name.as_str())
        .collect()
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

/// The tool `tool`'s state in `style`, before and after it.
fn tool_of<'s>(style: &'s AppliedStyle, tool: &str) -> &'s StyleTool {
    (style.tools.iter())
        .find(|kept| kept.tool == tool)
        .unwrap_or_else(|| panic!("{tool} in {style:?}"))
}

/// A style applied is one change of source Style, noted with its name: a step of each tool,
/// sharing a batch, the last holding the list, the style at its top with each tool's state
/// before and after it. One Undo takes it back, the list included; Redo puts it back.
#[test]
fn a_style_is_one_change_that_undo_takes_back_and_redo_puts_back() {
    let path = scratch("styles-apply");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    let before = edit(&driven);
    let events = driven.run([append("Warm film", &[grain(40.0), vignette(50.0)])]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let made = history(&events);
    let steps: Vec<_> = (made.steps.iter().rev().take(2).rev())
        .map(|step| {
            let tools: Vec<_> = step.changes.iter().map(|change| &*change.tool).collect();
            (step.source, step.note.clone(), tools)
        })
        .collect();
    let style = |tool| {
        (
            StepSource::Style,
            Some(Note::style("Warm film")),
            vec![tool],
        )
    };
    assert_eq!(steps, [style("grain"), style("vignette")]);
    let applied = log(&driven);
    let [.., first, last] = applied.steps() else {
        panic!("two steps");
    };
    assert!(first.batch.is_some() && first.batch == last.batch);
    assert_eq!(first.styles, None);
    assert_eq!(listed(&events), ["Warm film"]);
    let reported = &values(&events).styles[0];
    assert_eq!(reported.file, "Warm film.apstyle");
    assert_eq!(reported.tools, ["grain", "vignette"]);
    let [warm] = &styles(&driven)[..] else {
        panic!("one style");
    };
    assert_eq!(last.styles.as_deref(), Some(std::slice::from_ref(warm)));
    let grained = tool_of(warm, "grain");
    assert_eq!(grained.before, ToolValues::new());
    assert_eq!(grained.after, on("strength", 40.0));
    let undone = driven.run([Command::Undo]);
    assert_eq!(edit(&driven), before);
    assert!(listed(&undone).is_empty());
    let redone = driven.run([Command::Redo]);
    assert_eq!(listed(&redone), ["Warm film"]);
    assert_eq!(edit(&driven).tool("vignette"), Some(&on("scale", 50.0)));
    // Going to the first step of its batch shows the list before it.
    driven.run([Command::GoTo {
        step: Some(first.id),
    }]);
    assert!(styles(&driven).is_empty());
    remove(&path);
}

/// Applied again, a style moves to the top: a tool that still holds what it set keeps the
/// earlier state before it, any other takes its state just before; one whose tools hold its
/// values already moves the list alone, a step of its own, and one already at the top with
/// nothing to change makes none.
#[test]
fn a_style_applied_again_moves_to_the_top_keeping_its_earlier_before() {
    let path = scratch("styles-again");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    let warm = [grain(40.0), vignette(50.0)];
    driven.run([append("Warm film", &warm)]);
    driven.run([append("Faded", &[soften(30.0)])]);
    assert_eq!(
        styles(&driven)
            .iter()
            .map(|s| &s.name[..])
            .collect::<Vec<_>>(),
        ["Faded", "Warm film"]
    );
    // Its tools hold its values: the list alone moves.
    let steps = log(&driven).steps().len();
    let again = driven.run([append("Warm film", &warm)]);
    assert!(failures(&again).is_empty(), "{again:?}");
    assert_eq!(listed(&again), ["Warm film", "Faded"]);
    let moved = log(&driven);
    assert_eq!(moved.steps().len(), steps + 1);
    let last = moved.steps().last().unwrap();
    assert_eq!(last.change, Change::None);
    assert_eq!(
        (last.source, &last.note),
        (StepSource::Style, &Some(Note::style("Warm film")))
    );
    let shown = history(&again).steps.last().unwrap().clone();
    assert!(shown.changes.is_empty());
    let kept = styles(&driven)[0].clone();
    assert_eq!(tool_of(&kept, "grain").before, on("strength", 10.0));
    // At the top already, its tools holding its values: no step.
    driven.run([append("Warm film", &warm)]);
    assert_eq!(log(&driven).steps().len(), steps + 1);
    // Grain changed since: it takes its state just before; Vignetting keeps the earlier.
    driven.run([set_tool(grain(30.0))]);
    driven.run([append("Warm film", &warm)]);
    let again = styles(&driven)[0].clone();
    assert_eq!(tool_of(&again, "grain").before, on("strength", 30.0));
    assert_eq!(tool_of(&again, "vignette").before, ToolValues::new());
    driven.run([clear("Warm film")]);
    assert_eq!(edit(&driven).tool("grain"), Some(&on("strength", 30.0)));
    assert_eq!(edit(&driven).tool("vignette"), None);
    remove(&path);
}

/// Clear puts back each tool that still holds the style's state, as one change of source
/// Style noted with its name, and takes the style off the list; a tool changed since keeps
/// its value. One Undo takes it back, the list included. A name the list lacks is refused.
#[test]
fn clear_puts_back_the_tools_that_still_hold_the_style_s_values() {
    let path = scratch("styles-clear");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Warm film", &[grain(40.0), vignette(50.0)])]);
    driven.run([set_tool(grain(30.0))]);
    let cleared = driven.run([clear("Warm film")]);
    assert!(failures(&cleared).is_empty(), "{cleared:?}");
    assert!(listed(&cleared).is_empty());
    let step = history(&cleared).steps.last().unwrap().clone();
    let tools: Vec<_> = step.changes.iter().map(|change| &*change.tool).collect();
    assert_eq!(
        (step.source, step.note.clone(), step.batch, tools),
        (
            StepSource::Style,
            Some(Note::clear_style("Warm film")),
            None,
            vec!["vignette"]
        )
    );
    assert_eq!(edit(&driven).tool("vignette"), None);
    assert_eq!(edit(&driven).tool("grain"), Some(&on("strength", 30.0)));
    let undone = driven.run([Command::Undo]);
    assert_eq!(listed(&undone), ["Warm film"]);
    assert_eq!(edit(&driven).tool("vignette"), Some(&on("scale", 50.0)));
    let unknown = driven.run([clear("Nothing")]);
    assert!(
        matches!(&failures(&unknown)[..], [SessionError::UnknownStyle { name }] if name == "Nothing"),
        "{unknown:?}"
    );
    remove(&path);
}

/// A Clear whose tools have all changed since is one step holding the list alone; neither
/// command is taken inside a `Step`.
#[test]
fn clear_with_every_tool_changed_is_a_step_of_the_list_alone() {
    let path = scratch("styles-clear-alone");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Warm film", &[grain(40.0)])]);
    driven.run([set_tool(grain(30.0))]);
    let before = edit(&driven);
    let cleared = driven.run([clear("Warm film")]);
    assert!(listed(&cleared).is_empty());
    assert_eq!(edit(&driven), before);
    let last = log(&driven).steps().last().unwrap().clone();
    assert_eq!(
        (last.change, last.styles, last.note),
        (
            Change::None,
            Some(Vec::new()),
            Some(Note::clear_style("Warm film"))
        )
    );
    assert!(history(&cleared).steps.last().unwrap().changes.is_empty());
    let undone = driven.run([Command::Undo]);
    assert_eq!(listed(&undone), ["Warm film"]);
    assert_eq!(edit(&driven), before);
    for command in [clear("Warm film"), append("Faded", &[soften(30.0)])] {
        let refused = driven.run([Command::Step {
            source: StepSource::Style,
            note: None,
            commands: vec![command],
        }]);
        assert!(
            matches!(
                &failures(&refused)[..],
                [SessionError::InStep { error, .. }] if matches!(**error, SessionError::NotInStep)
            ),
            "{refused:?}"
        );
    }
    assert_eq!(listed(&driven.run([Command::RetrySave])), ["Warm film"]);
    remove(&path);
}

/// Replace resets the whole edit first, its Reset's step holding the emptied list, then sets
/// the style's tools, one Undo for all; each tool's state before is its reset state, so
/// Clear leaves it at its defaults, as the Replace began.
#[test]
fn replace_resets_first_and_clear_leaves_its_tools_at_their_defaults() {
    let path = scratch("styles-replace");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    driven.run([append("Faded", &[soften(30.0)])]);
    let before = edit(&driven);
    let replaced = driven.run([apply("Warm film", StyleMode::Replace, &[vignette(50.0)])]);
    assert!(failures(&replaced).is_empty(), "{replaced:?}");
    assert_eq!(listed(&replaced), ["Warm film"]);
    let replaced_log = log(&driven);
    let [.., reset, set] = replaced_log.steps() else {
        panic!("two steps");
    };
    assert!(matches!(reset.change, Change::Reset(_)), "{reset:?}");
    assert_eq!(reset.styles, Some(Vec::new()));
    assert_eq!(reset.note, Some(Note::reset()));
    assert_eq!(set.note, Some(Note::style("Warm film")));
    assert!(reset.batch.is_some() && reset.batch == set.batch);
    assert_eq!(
        (edit(&driven).tool("grain"), edit(&driven).tool("soften")),
        (None, None)
    );
    let warm = styles(&driven)[0].clone();
    assert_eq!(tool_of(&warm, "vignette").before, ToolValues::new());
    let undone = driven.run([Command::Undo]);
    assert_eq!(listed(&undone), ["Faded"]);
    assert_eq!(edit(&driven), before);
    driven.run([Command::Redo]);
    // Going to its Reset shows the list emptied.
    driven.run([Command::GoTo {
        step: Some(reset.id),
    }]);
    assert!(styles(&driven).is_empty());
    driven.run([Command::GoTo { step: Some(set.id) }]);
    driven.run([clear("Warm film")]);
    assert!(edit(&driven).is_empty());
    remove(&path);
}

/// A Replace that moves no tool still empties the list and puts the style on it: its Reset's
/// step holds the emptied list, and a step of the list alone after it, in its batch, the
/// style.
#[test]
fn a_replace_that_moves_no_tool_resets_the_list_then_holds_the_style() {
    let path = scratch("styles-replace-nothing");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Faded", &[soften(30.0)])]);
    driven.run([Command::ResetTool {
        tool: "soften".to_owned(),
    }]);
    assert!(edit(&driven).is_empty());
    let steps = log(&driven).steps().len();
    let replaced = driven.run([apply(
        "Plain",
        StyleMode::Replace,
        &[("grain", ToolValues::new())],
    )]);
    assert!(failures(&replaced).is_empty(), "{replaced:?}");
    assert_eq!(listed(&replaced), ["Plain"]);
    let replaced = log(&driven);
    let [reset, list] = &replaced.steps()[steps..] else {
        panic!("two steps: {:?}", replaced.steps());
    };
    assert_eq!(
        (&reset.change, &reset.styles),
        (&Change::Reset(Vec::new()), &Some(Vec::new()))
    );
    assert_eq!(list.change, Change::None);
    assert!(reset.batch.is_some() && reset.batch == list.batch);
    let undone = driven.run([Command::Undo]);
    assert_eq!(listed(&undone), ["Faded"]);
    remove(&path);
}

/// Reset empties the styles applied, its step holding the emptied list, even where it resets
/// no tool; a photo reset with no style applied holds no `apertrust:styles`.
#[test]
fn reset_empties_the_styles_applied() {
    let path = scratch("styles-reset");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0)), Command::Reset]);
    driven.writes();
    let file = std::fs::read_to_string(sidecar::path_for(&path)).unwrap();
    assert!(!file.contains("apertrust:styles"), "{file}");
    driven.run([append("Warm film", &[grain(40.0)])]);
    let reset = driven.run([Command::Reset]);
    assert!(listed(&reset).is_empty());
    let last = log(&driven).steps().last().unwrap().clone();
    assert!(matches!(last.change, Change::Reset(_)));
    assert_eq!(last.styles, Some(Vec::new()));
    // Its tools at their defaults, the style still listed: Reset is a step of its own.
    driven.run([Command::Undo]);
    driven.run([Command::ResetTool {
        tool: "grain".to_owned(),
    }]);
    assert!(edit(&driven).is_empty());
    let steps = log(&driven).steps().len();
    let reset = driven.run([Command::Reset]);
    assert!(listed(&reset).is_empty());
    let emptied = log(&driven);
    assert_eq!(emptied.steps().len(), steps + 1);
    let last = emptied.steps().last().unwrap();
    assert_eq!(
        (&last.change, &last.styles, &last.note),
        (
            &Change::Reset(Vec::new()),
            &Some(Vec::new()),
            &Some(Note::reset())
        )
    );
    // The emptied list is saved, though the edit was empty already.
    driven.writes();
    assert!(saved(&path).log.styles().is_empty());
    // Nothing to reset, nothing listed: no step.
    driven.run([Command::Reset]);
    assert_eq!(log(&driven).steps().len(), steps + 1);
    remove(&path);
}

/// Compress takes the list off every step it keeps and writes it on a list-only step of its
/// own, so a tool reset after the style stays reset; Compress again changes nothing.
#[test]
fn compress_keeps_the_list_and_restores_no_reset_tool() {
    let path = scratch("styles-compress");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Warm film", &[grain(40.0)])]);
    driven.run([set_tool(soften(30.0))]);
    driven.run([Command::ResetTool {
        tool: "grain".to_owned(),
    }]);
    let compressed = driven.run([Command::Compress { up_to: None }]);
    assert!(failures(&compressed).is_empty(), "{compressed:?}");
    assert_eq!(listed(&compressed), ["Warm film"]);
    assert_eq!(edit(&driven).tool("grain"), None);
    assert_eq!(edit(&driven).tool("soften"), Some(&on("amount", 30.0)));
    let kept = log(&driven);
    let last = &kept.steps()[kept.end() - 1];
    assert_eq!(
        (&last.change, last.source, &last.note),
        (
            &Change::None,
            StepSource::Style,
            &Some(Note::styles_applied())
        )
    );
    let holding: Vec<_> = (kept.steps().iter())
        .filter(|step| step.styles.is_some())
        .map(|step| step.id)
        .collect();
    assert_eq!(holding, [last.id]);
    // Compress again: nothing.
    driven.run([Command::Compress { up_to: None }]);
    assert_eq!(log(&driven), kept);
    // A Clear after it: Grain no longer holds the style's values, so the list alone.
    driven.run([clear("Warm film")]);
    assert_eq!(edit(&driven).tool("grain"), None);
    assert!(styles(&driven).is_empty());
    remove(&path);
}

/// The sidecar of `path` as these tests' tools read it.
fn saved(path: &Path) -> sidecar::Loaded {
    sidecar::read(&sidecar::path_for(path), OPERATIONS, TOOLS, &[]).unwrap()
}

/// The list is saved with the log and read again as the photo reopens, a list-only step
/// among it; Undo's fallback then moves it with the end.
#[test]
fn the_styles_applied_are_saved_and_read_again_on_reopening() {
    let path = scratch("styles-reopen");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Warm film", &[grain(40.0)])]);
    driven.run([set_tool(grain(30.0))]);
    driven.run([clear("Warm film")]);
    driven.run([append("Faded", &[soften(30.0)])]);
    let before = log(&driven);
    driven.writes();
    let read = saved(&path);
    assert!(read.view_only().is_none(), "{:?}", read.problems);
    assert_eq!(read.log, before);
    let reopened = driven.open(&path);
    assert_eq!(listed(&reopened), ["Faded"]);
    // Undo's fallback: over Faded, then over the Clear's list-only step.
    let undone = driven.run([Command::Undo]);
    assert!(listed(&undone).is_empty());
    let undone = driven.run([Command::Undo]);
    assert_eq!(listed(&undone), ["Warm film"]);
    remove(&path);
}

/// What another program writes beside the session, as `apertrust-cli edit` would: Grain at
/// `strength` and, where `styled`, the style Other applied on Soften, as one change at a
/// clock a minute after the file's last.
fn write_elsewhere(path: &Path, (tool, values): (&dyn ProcessingTool, ToolValues), styled: bool) {
    let read = saved(path);
    let mut log = read.log.clone();
    let last = (log.steps().iter().map(|step| step.clock)).max();
    let clock = Clock::at(
        last.map_or(0, |last| last.when) + 60_000,
        None,
        "00000000".parse().unwrap(),
    );
    let mut edit = log.fold();
    let image = testing::image();
    (edit.set_tool(tool, &values, &image)).unwrap();
    if styled {
        (edit.set_tool(&Soften, &on("amount", 20.0), &image)).unwrap();
    }
    let changes = crate::history::Change::between(&log.fold(), &edit, &[]);
    log.add_change(
        changes,
        StepSource::Style,
        Some(Note::style("Other")),
        None,
        clock,
    );
    if styled {
        let other = AppliedStyle {
            name: "Other".to_owned(),
            file: "Other.apstyle".to_owned(),
            tools: vec![StyleTool {
                tool: "soften".to_owned(),
                before: ToolValues::new(),
                after: on("amount", 20.0),
            }],
        };
        let list = crate::history::on_top(log.styles(), other);
        log.hold_styles(list);
    }
    let mut clocks = read.clocks().to_vec();
    crate::history::advance(&mut clocks, clock);
    let path = sidecar::path_for(path);
    sidecar::write_log(&path, &read.file, &log, &clocks, (OPERATIONS, TOOLS), "cli").unwrap();
}

/// A newer file taken in: the session's style laid on it is replayed onto the newer list,
/// beside the style the other side applied, each tool's state before it what the joined log
/// holds just before its step, not what the session recorded.
#[test]
fn a_newer_file_taken_in_replays_the_style_with_the_joined_log_s_before() {
    let path = scratch("styles-newer");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    driven.writes();
    write_elsewhere(&path, (&Grain, on("strength", 20.0)), true);
    // The session applies Warm film, setting Grain from 10, as it holds it, to 40.
    driven.run([append("Warm film", &[grain(40.0)])]);
    assert_eq!(
        tool_of(&styles(&driven)[0], "grain").before,
        on("strength", 10.0)
    );
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert_eq!(listed(&taken), ["Warm film", "Other"]);
    let warm = styles(&driven)[0].clone();
    assert_eq!(tool_of(&warm, "grain").before, on("strength", 20.0));
    assert_eq!(tool_of(&warm, "grain").after, on("strength", 40.0));
    assert_eq!(written(&driven.writes()).len(), 1);
    assert_eq!(saved(&path).log.styles(), styles(&driven));
    // Clear puts Grain back to what the other side set.
    driven.run([clear("Warm film")]);
    assert_eq!(edit(&driven).tool("grain"), Some(&on("strength", 20.0)));
    assert_eq!(edit(&driven).tool("soften"), Some(&on("amount", 20.0)));
    remove(&path);
}

/// Every Reset laid on a newer file empties its list, though the session's Reset stored none,
/// as the session's list was empty: the other side's style never outlives the session's
/// Reset.
#[test]
fn a_reset_laid_on_a_newer_file_empties_its_list() {
    let path = scratch("styles-newer-reset");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    driven.writes();
    write_elsewhere(&path, (&Grain, on("strength", 20.0)), true);
    driven.run([Command::Reset]);
    assert_eq!(log(&driven).steps().last().unwrap().styles, None);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert!(listed(&taken).is_empty(), "{taken:?}");
    let joined = log(&driven);
    let last = joined.steps().last().unwrap();
    assert!(matches!(last.change, Change::Reset(_)));
    assert_eq!(last.styles, Some(Vec::new()));
    remove(&path);
}

/// An older version 2 build reads `apertrust:styles` as a field of a step it does not write:
/// the photo opens for viewing only, its history one it cannot keep, nothing written over
/// it, until Discard History starts it afresh without the field. Here this build reads such
/// a file as an older one reads a list: under a name it does not know.
#[test]
fn an_older_build_reads_the_list_as_a_field_it_cannot_keep() {
    let path = scratch("styles-older-build");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Warm film", &[grain(40.0)])]);
    driven.writes();
    let sidecar = sidecar::path_for(&path);
    let file = std::fs::read_to_string(&sidecar).unwrap();
    assert!(file.contains("<apertrust:styles>"), "{file}");
    let older = file.replace("apertrust:styles>", "apertrust:stylesLater>");
    std::fs::write(&sidecar, &older).unwrap();
    let opened = driven.open(&path);
    let photo = (opened.iter())
        .find_map(|event| match event {
            Event::Opened(photo) => Some(photo),
            _ => None,
        })
        .unwrap();
    let reason = &photo.view_only.as_ref().expect("for viewing only").reason;
    assert!(
        matches!(reason, sidecar::Reason::Unreadable(sidecar::Problem::UnknownField { field, .. }) if field == "apertrust:stylesLater"),
        "{reason:?}"
    );
    for command in [append("Faded", &[soften(30.0)]), clear("Warm film")] {
        let refused = driven.run([command]);
        assert!(
            matches!(&failures(&refused)[..], [SessionError::ViewOnly(_)]),
            "{refused:?}"
        );
    }
    assert!(driven.writes().is_empty());
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), older);
    driven.run([Command::DiscardHistory]);
    driven.writes();
    let afresh = std::fs::read_to_string(&sidecar).unwrap();
    assert!(!afresh.contains("stylesLater"), "{afresh}");
    remove(&path);
}

/// A Clear laid on a newer file promotes the style below it to the top without applying it
/// again: its state before each tool stays as the session recorded it, though the Clear
/// touched that tool.
#[test]
fn a_clear_laid_on_a_newer_file_leaves_the_style_below_as_it_was() {
    let path = scratch("styles-newer-clear");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    driven.run([append("Faded", &[grain(20.0)])]);
    driven.run([append("Warm film", &[grain(40.0)])]);
    driven.writes();
    write_elsewhere(&path, (&Vignetting, on("scale", 50.0)), false);
    driven.run([clear("Warm film")]);
    assert!(driven.writes().is_empty());
    let taken = driven.engine.newer_file(false);
    assert_eq!(listed(&taken), ["Faded"]);
    let faded = styles(&driven)[0].clone();
    assert_eq!(tool_of(&faded, "grain").before, on("strength", 10.0));
    driven.run([clear("Faded")]);
    assert_eq!(edit(&driven).tool("grain"), Some(&on("strength", 10.0)));
    assert_eq!(edit(&driven).tool("vignette"), Some(&on("scale", 50.0)));
    remove(&path);
}

/// A Replace laid on a newer file keeps each tool's state before it its reset state: the
/// Replace records its Reset step, empty where it reset no tool the session held, so the
/// joined log says the edit was reset just before the style's steps.
#[test]
fn a_replace_laid_on_a_newer_file_keeps_the_reset_state_before() {
    let path = scratch("styles-newer-replace");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    driven.writes();
    write_elsewhere(&path, (&Vignetting, on("scale", 50.0)), false);
    driven.run([apply("Warm film", StyleMode::Replace, &[grain(40.0)])]);
    let replaced = log(&driven);
    let [.., reset, _] = replaced.steps() else {
        panic!("two steps");
    };
    assert_eq!(reset.change, Change::Reset(Vec::new()));
    assert_eq!(
        tool_of(&styles(&driven)[0], "grain").before,
        ToolValues::new()
    );
    // The session's save finds the newer file first.
    assert!(driven.writes().is_empty());
    driven.engine.newer_file(false);
    assert_eq!(
        tool_of(&styles(&driven)[0], "grain").before,
        ToolValues::new()
    );
    driven.run([clear("Warm film")]);
    assert_eq!(edit(&driven).tool("grain"), None);
    remove(&path);
}

/// A style replayed on a newer file holding an item of its name is applied again over that
/// item: the item's tools this application does not set stay.
#[test]
fn a_style_replayed_keeps_the_earlier_item_s_other_tools() {
    let path = scratch("styles-newer-extra");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([set_tool(grain(10.0))]);
    driven.writes();
    // The other side applies Other on Soften; the session a style of that name on Grain.
    write_elsewhere(&path, (&Grain, on("strength", 20.0)), true);
    driven.run([append("Other", &[grain(40.0)])]);
    // The session's save finds the newer file first.
    assert!(driven.writes().is_empty());
    driven.engine.newer_file(false);
    let other = styles(&driven)[0].clone();
    let tools: Vec<_> = other.tools.iter().map(|tool| &tool.tool[..]).collect();
    assert_eq!(tools, ["grain", "soften"], "{other:?}");
    assert_eq!(tool_of(&other, "grain").before, on("strength", 20.0));
    assert_eq!(tool_of(&other, "soften").before, ToolValues::new());
    remove(&path);
}

/// A style one of whose tools is refused is refused whole: the edit and the list stay as
/// they were, and no step is made.
#[test]
fn a_style_refused_part_way_changes_nothing() {
    let path = scratch("styles-refused");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Faded", &[soften(30.0)])]);
    let (before, logged) = (edit(&driven), log(&driven));
    let refused = driven.run([append(
        "Warm film",
        &[grain(40.0), ("nothing", on("amount", 1.0))],
    )]);
    assert!(
        matches!(&failures(&refused)[..], [SessionError::Edit(EditError::UnknownTool(tool))] if tool == "nothing"),
        "{refused:?}"
    );
    assert_eq!(edit(&driven), before);
    assert_eq!(log(&driven), logged);
    assert_eq!(listed(&refused), ["Faded"]);
    remove(&path);
}

/// Truncate keeps the list as the steps up to the end hold it: the styles applied after the
/// end go with their steps.
#[test]
fn truncate_moves_the_list_with_the_steps() {
    let path = scratch("styles-truncate");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([append("Warm film", &[grain(40.0)])]);
    let warm = log(&driven).steps().last().unwrap().id;
    driven.run([append("Faded", &[soften(30.0)])]);
    driven.run([Command::GoTo { step: Some(warm) }]);
    let truncated = driven.run([Command::Truncate { after: None }]);
    assert!(failures(&truncated).is_empty(), "{truncated:?}");
    assert_eq!(listed(&truncated), ["Warm film"]);
    let kept = log(&driven);
    assert_eq!(kept.steps().last().unwrap().id, warm);
    let undone = driven.run([Command::Undo]);
    assert_eq!(listed(&undone), ["Warm film"]);
    driven.run([Command::Redo]);
    assert_eq!(styles(&driven)[0].name, "Warm film");
    remove(&path);
}

/// A Replace on an edit with nothing to reset and no list, its style moving no tool,
/// still records its Reset step, so the log says the edit was reset.
#[test]
fn a_replace_that_moves_nothing_records_its_reset_step() {
    let path = scratch("styles-replace-empty");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    let steps = log(&driven).steps().len();
    let replaced = driven.run([apply(
        "Plain",
        StyleMode::Replace,
        &[("grain", ToolValues::new())],
    )]);
    assert!(failures(&replaced).is_empty(), "{replaced:?}");
    assert_eq!(listed(&replaced), ["Plain"]);
    let replaced = log(&driven);
    let [reset, list] = &replaced.steps()[steps..] else {
        panic!("two steps: {:?}", replaced.steps());
    };
    assert_eq!(
        (&reset.change, &reset.styles),
        (&Change::Reset(Vec::new()), &None)
    );
    assert_eq!(list.change, Change::None);
    remove(&path);
}

/// `ApplyStyle` of the built-in style `id` under `name`, its file `builtin:<id>`.
fn built_in(name: &str, id: &str, tools: &[(&str, ToolValues)]) -> Command {
    match append(name, tools) {
        Command::ApplyStyle {
            name, mode, tools, ..
        } => Command::ApplyStyle {
            name,
            file: format!("builtin:{id}"),
            mode,
            tools,
        },
        _ => unreachable!("an ApplyStyle"),
    }
}

/// A built-in style is known by its file (decision 109): applied again in another language
/// it moves to the top under the name sent rather than being listed twice, and a person's
/// style named as a built-in is listed beside it.
#[test]
fn a_built_in_style_applied_again_in_another_language_moves_to_the_top() {
    let path = scratch("styles-built-in");
    let mut driven = Driven::new(TOOLS);
    driven.open(&path);
    driven.run([built_in("Lomo", "lomo", &[grain(40.0)])]);
    driven.run([append("Faded", &[soften(30.0)])]);
    let events = driven.run([built_in("Ломо", "lomo", &[grain(60.0)])]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(listed(&events), ["Ломо", "Faded"]);
    // Grain still held the built-in's after: its state before the built-in first set it.
    assert_eq!(
        tool_of(&styles(&driven)[0], "grain").before,
        ToolValues::new()
    );
    let events = driven.run([append("Ломо", &[vignette(50.0)])]);
    assert_eq!(listed(&events), ["Ломо", "Ломо", "Faded"]);
    assert_eq!(styles(&driven)[0].file, "Ломо.apstyle");
    // Saved, the two read again as two styles, no problem.
    driven.writes();
    let read = saved(&path);
    assert!(read.problems.is_empty(), "{:?}", read.problems);
    assert_eq!(read.log.styles().len(), 3);
    // Clear on the built-in's row of the two of one name: by its file, the built-in alone,
    // and a picker's turning off looks at its tools, not the person's style's.
    let engine = &driven.engine;
    let tools = |file: &str| engine.style_tools("Ломо", file);
    assert_eq!(tools("builtin:lomo"), [Some("grain".to_owned())]);
    assert_eq!(tools("Ломо.apstyle"), [Some("vignette".to_owned())]);
    let cleared = driven.run([Command::ClearStyle {
        name: "Ломо".to_owned(),
        file: "builtin:lomo".to_owned(),
    }]);
    assert!(failures(&cleared).is_empty(), "{cleared:?}");
    assert_eq!(listed(&cleared), ["Ломо", "Faded"]);
    assert_eq!(styles(&driven)[0].file, "Ломо.apstyle");
    assert!(edit(&driven).tool("grain").is_none());
    assert!(edit(&driven).tool("vignette").is_some());
    remove(&path);
}

/// The tools of the photos a style is applied to open or not: two with default steps, White
/// balance redrawn on the photo, and the switched tools a style sets.
const BOTH: &[&dyn ProcessingTool] = &[
    &Grain,
    &Soften,
    &Vignetting,
    &apertrust_tools::white_balance::WhiteBalance,
    &apertrust_tools::exposure::Exposure,
];

/// Another machine's step on the sidecar beside `path`, Soften at 5, its clock at `when`
/// milliseconds, as `apertrust-cli edit` on a computer whose clock is ahead would write it.
fn step_at(path: &Path, when: u64) {
    let sidecar = sidecar::path_for(path);
    let read = sidecar::read(&sidecar, OPERATIONS, BOTH, &[]).unwrap();
    let mut log = read.log.clone();
    let clock = Clock::at(when, None, "00000000".parse().unwrap());
    let mut edit = log.fold();
    (edit.set_tool(&Soften, &on("amount", 5.0), &testing::image())).unwrap();
    let changes = Change::between(&log.fold(), &edit, &[]);
    log.add_change(changes, StepSource::You, None, None, clock);
    let mut clocks = read.clocks().to_vec();
    crate::history::advance(&mut clocks, clock);
    let build = "cli";
    sidecar::write_log(
        &sidecar,
        &read.file,
        &log,
        &clocks,
        (OPERATIONS, BOTH),
        build,
    )
    .unwrap();
}

/// What a step says of the photo's history, its id and clock left out: its source, note,
/// change and list, and whether it is in `batch`, else its own batch.
type Seen = (
    StepSource,
    Option<Note>,
    Change,
    Option<Vec<AppliedStyle>>,
    Option<Result<(), BatchId>>,
);

fn seen(log: &Log, batch: BatchId) -> (Vec<Seen>, usize) {
    let steps = (log.steps().iter())
        .map(|step| {
            let batch = step
                .batch
                .map(|of| if of == batch { Ok(()) } else { Err(of) });
            let styles = step.styles.clone();
            (
                step.source,
                step.note.clone(),
                step.change.clone(),
                styles,
                batch,
            )
        })
        .collect();
    (steps, log.end())
}

/// A style gives the same log on a photo opened or not (decision 109): each case's photo is
/// edited open, saved, then the style is applied by the session to the photo opened again
/// and by [`style_log`] to the log its sidecar holds, on the image context the open read.
/// The two logs hold the same steps, their ids and clocks apart, the style's in one batch,
/// each after every clock before it, the photo not open's at the command's wall time.
#[test]
fn a_style_gives_the_same_log_on_a_photo_opened_or_not() {
    use crate::session::styles::{Batch, Style, style_log};
    let go_to = |at: fn(&Log) -> usize| {
        move |driven: &mut Driven| {
            let step = Some(log(driven).steps()[at(&log(driven))].id);
            driven.run([Command::GoTo { step }]);
        }
    };
    let edited = |driven: &mut Driven| {
        driven.run([set_tool(grain(30.0))]);
        driven.run([set_tool(soften(20.0))]);
    };
    type Setup = Box<dyn Fn(&mut Driven)>;
    // (the case, what is done to the photo first, the style); a case named `ahead-<ms>` has
    // another machine's step at the wall clock and that many milliseconds laid on its file.
    let cases: Vec<(&str, Setup, Command)> = vec![
        (
            "append",
            Box::new(edited),
            append("Warm film", &[grain(40.0), vignette(50.0)]),
        ),
        (
            "replace",
            Box::new(edited),
            apply("Warm film", StyleMode::Replace, &[grain(40.0)]),
        ),
        (
            "applied-again",
            Box::new(|driven: &mut Driven| {
                driven.run([append("Warm film", &[grain(40.0)])]);
                driven.run([set_tool(soften(10.0))]);
            }),
            append("Warm film", &[grain(60.0), vignette(50.0)]),
        ),
        (
            "no-history",
            Box::new(|_: &mut Driven| {}),
            append("Faded", &[soften(30.0)]),
        ),
        (
            "end-among-defaults",
            Box::new(move |driven: &mut Driven| {
                edited(driven);
                go_to(|_| 0)(driven);
            }),
            append("Faded", &[soften(30.0)]),
        ),
        (
            "replace-among-defaults",
            Box::new(move |driven: &mut Driven| {
                edited(driven);
                go_to(|_| 0)(driven);
            }),
            apply("Faded", StyleMode::Replace, &[]),
        ),
        (
            "end-below-the-last-step",
            Box::new(move |driven: &mut Driven| {
                edited(driven);
                go_to(|log| log.steps().len() - 2)(driven);
            }),
            append("Faded", &[vignette(50.0)]),
        ),
        // Another machine's clock ahead, within the drift bound, then past it.
        (
            "ahead-120000",
            Box::new(edited),
            append("Warm film", &[grain(40.0), vignette(50.0)]),
        ),
        (
            "ahead-3600000",
            Box::new(edited),
            append("Warm film", &[grain(40.0), vignette(50.0)]),
        ),
    ];
    for (case, setup, command) in cases {
        let path = scratch(&format!("styles-opened-or-not-{case}"));
        let mut driven = Driven::new(BOTH);
        driven.open(&path);
        setup(&mut driven);
        driven.writes();
        drop(driven);
        if let Some(ahead) = case.strip_prefix("ahead-") {
            let ahead: u64 = ahead.parse().unwrap();
            step_at(&path, apertrust_library::clock::wall_clock() + ahead);
        }
        let sidecar = sidecar::path_for(&path);
        let (log_read, clocks) = match sidecar.exists() {
            true => {
                let read = sidecar::read(&sidecar, OPERATIONS, BOTH, &[]).unwrap();
                let clocks = read.clocks().to_vec();
                (read.log, clocks)
            }
            false => (Log::default(), Vec::new()),
        };
        // The latest clock this machine honours, another's past the drift bound ignored.
        let read = (log_read.steps().iter().map(|step| &step.clock)).chain(&clocks);
        let latest = crate::history::latest(read, crate::history::MachineId::current());
        // Opened again, the style applied by the session.
        let mut driven = Driven::new(BOTH);
        driven.open(&path);
        let image = driven.engine.open.as_ref().unwrap().image.clone();
        let events = driven.run([command.clone()]);
        assert!(failures(&events).is_empty(), "{case}: {events:?}");
        let opened = log(&driven);
        let ours = opened.steps().last().unwrap().batch.unwrap();
        // Not open: the same style on the log the sidecar holds.
        let Command::ApplyStyle {
            name,
            file,
            mode,
            tools,
        } = &command
        else {
            unreachable!("an ApplyStyle")
        };
        let style = Style {
            name,
            file,
            mode: *mode,
            tools,
        };
        let batch = Batch {
            id: BatchId(0x109),
            wall: apertrust_library::clock::wall_clock(),
        };
        let defaults = crate::history::default_tools(BOTH, OPERATIONS);
        let (not_open, vector) = style_log(
            (log_read.clone(), clocks.clone()),
            &defaults,
            style,
            (BOTH, &image),
            batch,
        )
        .unwrap()
        .unwrap_or_else(|| panic!("{case}: a change"));
        // Every case's log begins with White balance's and Exposure's default steps.
        assert_eq!(not_open.defaults(), 2, "{case}: {not_open:?}");
        assert_eq!(
            seen(&not_open, batch.id),
            seen(&opened, ours),
            "{case}: the same log"
        );
        // Every step the style made carries the batch, at the wall time or, past a clock
        // ahead within the bound, just after it.
        let made: Vec<_> = (not_open.steps().iter())
            .filter(|step| log_read.position(step.id).is_none())
            .filter(|step| step.source != StepSource::Default)
            .collect();
        assert!(!made.is_empty(), "{case}");
        let when = latest.map_or(batch.wall, |latest| latest.when.max(batch.wall));
        for step in made {
            assert_eq!(
                step.batch,
                Some(batch.id),
                "{case}: every step in the batch"
            );
            assert!(
                Some(step.clock) > latest,
                "{case}: after the latest honoured"
            );
            assert_eq!(step.clock.when, when, "{case}: the command's wall time");
        }
        if case == "ahead-3600000" {
            assert_eq!(when, batch.wall, "{case}: the clock past the bound ignored");
        }
        if case == "ahead-120000" {
            assert!(when > batch.wall, "{case}: after the clock ahead");
        }
        let last = not_open.steps().iter().map(|step| step.clock).max();
        assert_eq!(
            vector.iter().copied().max(),
            last,
            "{case}: the vector advanced"
        );
        // The batch reached it already: left as it is.
        let again = style_log((not_open, vector), &defaults, style, (BOTH, &image), batch);
        assert!(again.unwrap().is_none(), "{case}: applied once");
        remove(&path);
    }
}
