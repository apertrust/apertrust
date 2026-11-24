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

use std::fs;

use apertrust_engine::session::{ControlValue, Entry, Event, Mark, ToolState, tool_values};
use apertrust_engine::sidecar;
use apertrust_tools::{RETIRED, TOOLS};
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::mirror::tests::{mirror, photo, report, scratch};

/// The built-in tool `id`.
fn tool(id: &str) -> &'static dyn ProcessingTool {
    *(TOOLS.iter())
        .find(|tool| tool.definition().tool.id == id)
        .unwrap()
}

/// `id` at its defaults, as presets' tests make a tool's state.
fn untouched(id: &str) -> ToolState {
    crate::presets::tests::state_of(tool(id))
}

/// `id` with its number control `control` at `value`, kept, as a slider leaves it; a
/// switched tool on.
fn moved(id: &str, control: &str, value: f32) -> ToolState {
    let mut state = untouched(id);
    let shown = (state.controls.iter_mut())
        .find(|shown| shown.control == control)
        .unwrap();
    shown.value = ControlValue::Number(value);
    shown.mark = Some(Mark::Kept);
    state.edited = true;
    state.enabled = true;
    state
}

/// Every built-in tool at its defaults but those of `touched`, in the session's order.
fn edit(touched: Vec<ToolState>) -> Vec<ToolState> {
    (TOOLS.iter())
        .map(|tool| {
            let id = tool.definition().tool.id;
            (touched.iter())
                .find(|state| state.tool == id)
                .cloned()
                .unwrap_or_else(|| untouched(id))
        })
        .collect()
}

/// Vignetting turned off, its marks kept: the edit holds it, with no value.
fn vignette_off() -> ToolState {
    let mut state = untouched("vignette");
    state.controls[0].mark = Some(Mark::Kept);
    state
}

/// An edit of Exposure, White balance, AgX, Vignetting turned off and Grain, written in
/// the session's order, which is not the pipeline's.
fn touched_edit() -> Vec<ToolState> {
    edit(vec![
        moved("grain", "strength", 40.0),
        moved("exposure", "exposure", 1.2),
        moved("temperature", "red", 1.5),
        moved("agx", "curve_pivot_y_linear_output", 0.25),
        vignette_off(),
    ])
}

#[test]
fn the_dialog_lists_the_touched_tools_in_pipeline_order_the_look_s_checked() {
    let words = Translator::new(Locale::EnGb);
    let lines = lines(TOOLS, &touched_edit(), &words);
    let listed: Vec<(&str, &str, bool)> = (lines.iter())
        .map(|line| (line.tool, line.title.as_str(), line.checked))
        .collect();
    assert_eq!(
        listed,
        [
            ("temperature", "White balance", false),
            ("exposure", "Exposure", false),
            ("agx", "AgX", true),
            ("grain", "Grain", true),
            ("vignette", "Vignetting", true),
        ]
    );
    // Pipeline order is the operations': each tool's first, among darktable's order.
    let places: Vec<_> = (lines.iter())
        .map(|line| crate::layout::hidden_order(line.tool))
        .collect();
    assert!(places.is_sorted(), "{places:?}");
    // Nothing touched, nothing listed.
    assert!(super::lines(TOOLS, &edit(Vec::new()), &words).is_empty());
}

#[test]
fn only_a_live_edit_that_touches_a_tool_creates_a_style() {
    // No photo.
    let mut m = mirror();
    assert!(!can_create(&m));
    // A photo open, its edit untouched.
    m.apply(Event::Opened(photo(1)));
    m.apply(Event::Values(report(0.0, 1)));
    assert!(!can_create(&m));
    // Touched.
    m.apply(Event::Values(report(1.0, 2)));
    assert!(can_create(&m));
    // A mark alone touches it too.
    let mut marked = report(0.0, 3);
    marked.tools[0].controls[0].mark = Some(Mark::Kept);
    m.apply(Event::Values(marked));
    assert!(can_create(&m));
    // Open for viewing only, touched or not.
    let mut view_only = photo(1);
    view_only.view_only = Some(sidecar::ViewOnly {
        build: None,
        reason: sidecar::Reason::Newer(sidecar::Newer::Version(3)),
    });
    m.apply(Event::Opened(view_only));
    m.apply(Event::Values(report(1.0, 4)));
    assert!(!can_create(&m));
}

#[test]
fn create_writes_each_tool_checked_as_a_preset_saves_it() {
    let folder = scratch("create");
    let states = touched_edit();
    let asked = Asked {
        name: "Warm film".to_owned(),
        description: Some("Soft and warm".to_owned()),
        tools: vec!["exposure", "agx", "vignette"],
    };
    let path = create(&folder, TOOLS, &states, &asked, "0.1.0").unwrap();
    assert_eq!(path, folder.join("Warm film.apstyle"));
    let listed = files::list(&folder, Kind::Style);
    assert!(listed.failed.is_empty());
    let [stored] = listed.files.as_slice() else {
        panic!("one style: {:?}", listed.files);
    };
    assert_eq!(stored.name, "Warm film");
    assert_eq!(stored.description.as_deref(), Some("Soft and warm"));
    let tools: Vec<&str> = (stored.entries.iter())
        .map(|entry| match entry {
            Entry::Tool { tool, .. } => tool.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(tools, ["exposure", "agx", "vignette"]);
    // Each entry reads back as the tool's values now, as a preset saves them.
    for (entry, id) in stored.entries.iter().zip(tools) {
        let state = states.iter().find(|state| state.tool == id).unwrap();
        let definition = tool(id).definition();
        let (read, values) = tool_values(entry, TOOLS, RETIRED).unwrap();
        assert_eq!(read, id);
        assert_eq!(values, current(definition, state), "{id}");
    }
    // A switched tool off: no `enabled`, no value; never `false`.
    let Entry::Tool {
        enabled, controls, ..
    } = &stored.entries[2]
    else {
        unreachable!()
    };
    assert_eq!((enabled, controls.len()), (&None, 0));
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("apertrust:style=\"1\""), "{text}");
    assert!(!text.contains("enabled=\"false\""), "{text}");
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_name_a_style_holds_is_refused_in_the_dialog_and_on_disk() {
    let folder = scratch("taken");
    let states = touched_edit();
    let asked = |name: &str| Asked {
        name: name.to_owned(),
        description: None,
        tools: vec!["agx"],
    };
    create(&folder, TOOLS, &states, &asked("Warm"), "0.1.0").unwrap();
    assert_eq!(taken(&folder), ["Warm"]);
    // Taken since the dialog opened: refused, the first file kept.
    let again = create(&folder, TOOLS, &states, &asked(" Warm "), "0.1.0");
    assert!(matches!(again, Err(PresetError::NameTaken(name)) if name == "Warm"));
    // Another case is another style, as names are compared exactly.
    let path = create(&folder, TOOLS, &states, &asked("warm"), "0.1.0").unwrap();
    assert_eq!(path, folder.join("warm 2.apstyle"));
    let _ = fs::remove_dir_all(&folder);
}

/// The dialog over `lines`, refusing `taken`, on `os`, in the theme `which`, with what it
/// asked kept.
fn dialog(
    taken: &[&str],
    lines: Vec<Line>,
    os: OperatingSystem,
    which: Builtin,
) -> Harness<'static, (crate::presets::Sheet<StyleDialog>, Vec<Output>)> {
    let taken = taken.iter().map(|name| (*name).to_owned()).collect();
    let dialog = StyleDialog::new(taken, lines);
    let builder = Harness::builder()
        .with_size(egui::vec2(352.0, 344.0))
        .with_os(os)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
        });
    crate::presets::tests::drawn(builder, crate::presets::Sheet(dialog))
}

fn touched_lines() -> Vec<Line> {
    lines(TOOLS, &touched_edit(), &Translator::new(Locale::EnGb))
}

#[test]
fn create_asks_for_the_tools_checked_with_the_name_trimmed() {
    let mut harness = dialog(&[], touched_lines(), OperatingSystem::Nix, Builtin::Dark);
    // The name has the focus, empty, and Create waits for it.
    let field = harness.get_by_role_and_label(Role::TextInput, "Name");
    assert!(field.is_focused());
    assert_eq!(field.accesskit_node().value().as_deref(), Some(""));
    harness.get_by_label("A style needs a name");
    let create = harness.get_by_role_and_label(Role::Button, "Create");
    assert!(create.accesskit_node().is_disabled());
    // The tools not checked at first say why in their tooltip.
    for (title, checked) in [
        ("White balance", false),
        ("Exposure", false),
        ("AgX", true),
        ("Grain", true),
        ("Vignetting", true),
    ] {
        let node = harness.get_by_role_and_label(Role::CheckBox, title);
        let ticked = node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True);
        assert_eq!(ticked, checked, "{title}");
        let tip = node.accesskit_node().description();
        let expected = (!checked).then_some("Usually belongs to one photo");
        assert_eq!(tip.as_deref(), expected, "{title}");
    }
    harness.state_mut().0.name = "  Warm film ".to_owned();
    harness.run();
    for title in ["Exposure", "Grain"] {
        harness.get_by_role_and_label(Role::CheckBox, title).click();
        harness.run();
    }
    let checked: Vec<bool> = (harness.state().0.lines.iter())
        .map(|line| line.checked)
        .collect();
    assert_eq!(checked, [false, true, true, false, true]);
    harness
        .get_by_role_and_label(Role::Button, "Create")
        .click();
    harness.run();
    let asked = Asked {
        name: "Warm film".to_owned(),
        description: None,
        tools: vec!["exposure", "agx", "vignette"],
    };
    assert_eq!(harness.state().1, [Output::Style(asked)]);
}

#[test]
fn a_taken_name_or_no_tool_disables_create_and_says_why() {
    let mut harness = dialog(
        &["Warm"],
        touched_lines(),
        OperatingSystem::Nix,
        Builtin::Dark,
    );
    harness.state_mut().0.name = "Warm".to_owned();
    harness.run();
    harness.get_by_label("A style already has this name");
    let create = harness.get_by_role_and_label(Role::Button, "Create");
    assert!(create.accesskit_node().is_disabled());
    harness.state_mut().0.name = "Warm 2".to_owned();
    harness.run();
    assert!(
        harness
            .query_by_label("A style already has this name")
            .is_none()
    );
    let create = harness.get_by_role_and_label(Role::Button, "Create");
    assert!(!create.accesskit_node().is_disabled());
    // No tool checked.
    for line in &mut harness.state_mut().0.lines {
        line.checked = false;
    }
    harness.run();
    harness.get_by_label("Choose at least one tool");
    let create = harness.get_by_role_and_label(Role::Button, "Create");
    assert!(create.accesskit_node().is_disabled());
    assert!(harness.state().1.is_empty());
}

#[test]
fn the_status_says_the_style_created_or_why_not() {
    let words = Translator::new(Locale::EnGb);
    assert_eq!(said(&words, "Warm", None), "Style “Warm” created");
    let error = PresetError::NameTaken("Warm".to_owned());
    assert_eq!(
        said(&words, "Warm", Some(&error)),
        "Style “Warm” not created: another file in the folder is named Warm"
    );
}

fn photograph(which: Builtin, name: &str) {
    let mut harness = dialog(&[], touched_lines(), OperatingSystem::Mac, which);
    harness.state_mut().0.name = "Warm film".to_owned();
    harness.run();
    harness.snapshot(name);
}

#[test]
fn create_a_style_in_the_dark_theme() {
    photograph(Builtin::Dark, "create-style-dark");
}

#[test]
fn create_a_style_in_the_light_theme() {
    photograph(Builtin::Light, "create-style-light");
}
