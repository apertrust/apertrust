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

use apertrust_engine::session::{ControlState, ControlValue, Mark};
use apertrust_tools::exposure::Exposure;
use apertrust_ui::{Builtin, Locale};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::tools::{self, Change, Head, Panels, Pickers};

/// A folder of the test's own, emptied first.
fn scratch(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("apertrust-presets-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// `dialog` drawn as the dialog host draws it each frame, in the harness `builder` sizes
/// and sets up, what it asked kept beside it; run once.
pub(crate) fn drawn<D: Dialog + 'static>(
    builder: egui_kittest::HarnessBuilder<(D, Vec<Output>)>,
    dialog: D,
) -> Harness<'static, (D, Vec<Output>)> {
    let mut harness = builder.build_ui_state(
        |ui, (dialog, outputs): &mut (D, Vec<Output>)| {
            outputs.extend(crate::dialog_host::draw(ui, dialog).outputs);
        },
        (dialog, Vec::new()),
    );
    harness.run();
    harness
}

/// Exposure at its defaults, every control available.
fn untouched() -> ToolState {
    state_of(&Exposure)
}

/// `tool` at its defaults, every control available, off where it is switched.
pub(crate) fn state_of(tool: &dyn ProcessingTool) -> ToolState {
    let definition = tool.definition();
    ToolState {
        tool: definition.tool.id,
        enabled: !definition.switched,
        edited: false,
        hidden: false,
        controls: (definition.controls.iter())
            .map(|control| ControlState {
                control: control.id,
                value: control.default(),
                default: control.default(),
                mark: None,
                available: true,
                options: Vec::new(),
                range: None,
                visible: true,
                label_value: None,
            })
            .collect(),
        presets: Vec::new(),
        views: Vec::new(),
        computed: Vec::new(),
        readouts: Vec::new(),
        actions: Vec::new(),
    }
}

/// Exposure with its exposure at `ev`, kept, as a slider leaves it.
fn exposed(ev: f32) -> ToolState {
    let mut state = untouched();
    let control = (state.controls.iter_mut())
        .find(|control| control.control == "exposure")
        .unwrap();
    control.value = ControlValue::Number(ev);
    control.mark = Some(Mark::Kept);
    state.edited = true;
    state
}

/// The preset `name` saved in `folder` from `state`.
fn saved(folder: &Path, name: &str, state: &ToolState) -> PathBuf {
    save(folder, &Exposure, state, (name, None), "0.1.0").unwrap()
}

/// What the pull-down of Exposure's presets in `folder` holds for `state`, `chosen` the
/// preset chosen last, in British English, as (words, checked, enabled, tip) each.
fn items(
    folder: &Path,
    state: &ToolState,
    chosen: Option<&str>,
) -> Vec<(String, Option<bool>, bool, Option<String>)> {
    let menu = Menu {
        folder: folder.to_owned(),
        chosen: chosen.map(str::to_owned),
        alerts: true,
    };
    let words = Translator::new(Locale::EnGb);
    let listed = read(folder, &Exposure);
    (entries(&Exposure, state, &listed, &menu, &words).into_iter())
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some((item.text, item.checked, item.enabled, item.tip)),
            _ => None,
        })
        .collect()
}

/// Exposure's section, its presets those of `folder`, in the theme `which`, with what it
/// asked for kept.
struct Panel {
    state: ToolState,
    menu: Menu,
    changes: Vec<Change>,
}

fn panel(which: Builtin, folder: &Path, state: ToolState) -> Harness<'static, Panel> {
    let menu = Menu {
        folder: folder.to_owned(),
        chosen: None,
        alerts: true,
    };
    Harness::builder()
        .with_size(egui::vec2(400.0, 340.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            |ui, panel: &mut Panel| {
                let head = Head {
                    open: false,
                    presets: Some(panel.menu.clone()),
                    ..Head::default()
                };
                let changes = tools::section(
                    ui,
                    &Exposure,
                    &panel.state,
                    head,
                    &Panels::default(),
                    &|_| true,
                    true,
                    None,
                    &Pickers::NONE,
                );
                panel.changes.extend(changes);
            },
            Panel {
                state,
                menu,
                changes: Vec::new(),
            },
        )
}

/// The pull-down opened from Exposure's presets button.
fn opened(harness: &mut Harness<'_, Panel>) {
    harness.run();
    let button = harness.get_by_role_and_label(Role::Button, "Exposure presets");
    assert!(!button.accesskit_node().is_disabled());
    button.click();
    harness.run();
}

#[test]
fn the_button_opens_default_settings_alone_with_no_files() {
    let folder = scratch("none");
    let mut harness = panel(Builtin::Dark, &folder, untouched());
    opened(&mut harness);
    let default = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Default settings");
    assert_eq!(default.accesskit_node().toggled(), Some(Toggled::True));
    harness.get_by_role_and_label(Role::MenuItem, "Save as Preset…");
    for words in ["Rename…", "Delete…"] {
        let item = harness.get_by_role_and_label(Role::MenuItem, words);
        assert!(item.accesskit_node().is_disabled(), "{words}");
    }
    assert!(harness.query_by_label_contains("Update").is_none());
    // A menu button, expanded while its pull-down is open.
    let button = harness.get_by_role_and_label(Role::Button, "Exposure presets");
    assert_eq!(button.accesskit_node().data().is_expanded(), Some(true));
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Default settings")
        .click();
    harness.run();
    assert_eq!(
        harness.state().changes,
        [Change::Preset {
            tool: "exposure",
            item: Item::Default
        }]
    );
    let _ = fs::remove_dir_all(&folder);
}

/// With no user data folder there is no menu, so the button is drawn disabled; the preset
/// chosen last is each tool's own, and forgotten as it is deleted.
#[test]
fn the_preset_chosen_last_is_each_tool_s() {
    let mut presets = Presets::default();
    assert_eq!(presets.menu("exposure", true), None);
    presets.choose("exposure", "Bright");
    presets.choose("temperature", "Tungsten");
    assert_eq!(presets.chosen("exposure"), Some("Bright"));
    presets.forget("exposure", "Other");
    assert_eq!(presets.chosen("exposure"), Some("Bright"));
    presets.forget("exposure", "Bright");
    assert_eq!(presets.chosen("exposure"), None);
    assert_eq!(presets.chosen("temperature"), Some("Tungsten"));
}

#[test]
fn save_as_preset_writes_the_tool_s_values() {
    let folder = scratch("save");
    let words = Translator::new(Locale::EnGb);
    let taken = taken(&folder, &Exposure, &words);
    assert_eq!(taken, ["Default settings"]);
    let name = free_name("Exposure", &taken, &words);
    assert_eq!(name, "Exposure 1");
    let dialog = NameDialog::save("exposure", "Exposure".to_owned(), taken, name);
    let builder = Harness::builder()
        .with_size(egui::vec2(400.0, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()));
    let mut harness = drawn(builder, Sheet(dialog));
    let field = harness.get_by_role_and_label(Role::TextInput, "Name");
    assert_eq!(
        field.accesskit_node().value().as_deref(),
        Some("Exposure 1")
    );
    harness.get_by_role_and_label(Role::Button, "Save").click();
    harness.run();
    let asked = Asked::Save {
        tool: "exposure",
        name: "Exposure 1".to_owned(),
        description: None,
    };
    assert_eq!(harness.state().1, [Output::Preset(asked)]);

    // What the app then writes: the values off their defaults and every mark.
    let path = saved(&folder, "Exposure 1", &exposed(1.2));
    assert_eq!(path, folder.join("Exposure 1.appreset"));
    let stored = files::parse(&fs::read_to_string(&path).unwrap(), Kind::Preset).unwrap();
    assert_eq!(stored.name, "Exposure 1");
    let version = Exposure.definition().version.to_string();
    assert_eq!(
        stored.entries,
        [Entry::Tool {
            tool: "exposure".to_owned(),
            version: Some(version),
            kept: vec!["exposure".to_owned()],
            auto: Vec::new(),
            enabled: None,
            controls: vec![("exposure".to_owned(), "1.2".to_owned())],
        }]
    );
    // And reads back as the values it was saved from.
    let listed = read(&folder, &Exposure);
    let values = listed[0].values.as_ref().unwrap();
    assert!(matches(Exposure.definition(), &exposed(1.2), values));
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn a_name_taken_or_empty_disables_save_and_says_why() {
    let taken = vec!["Bright".to_owned(), "Default settings".to_owned()];
    let dialog = NameDialog::save(
        "exposure",
        "Exposure".to_owned(),
        taken,
        "Bright".to_owned(),
    );
    let builder = Harness::builder()
        .with_size(egui::vec2(400.0, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()));
    let mut harness = drawn(builder, Sheet(dialog));
    harness.get_by_label("Exposure already has a preset with this name");
    let save = harness.get_by_role_and_label(Role::Button, "Save");
    assert!(save.accesskit_node().is_disabled());
    harness.state_mut().0.name = "  ".to_owned();
    harness.run();
    harness.get_by_label("A preset needs a name");
    harness.state_mut().0.name = "Bright 2".to_owned();
    harness.run();
    let save = harness.get_by_role_and_label(Role::Button, "Save");
    assert!(!save.accesskit_node().is_disabled());
    assert!(harness.state().1.is_empty());
}

#[test]
fn a_preset_is_checked_until_a_slider_moves_and_update_shows() {
    let folder = scratch("matching");
    saved(&folder, "Bright", &exposed(1.2));
    saved(&folder, "Dim", &exposed(-1.0));
    let checked = |items: &[(String, Option<bool>, bool, Option<String>)], words: &str| {
        (items.iter())
            .find(|item| item.0 == words)
            .and_then(|item| item.1)
    };
    let held = items(&folder, &exposed(1.2), Some("Bright"));
    assert_eq!(checked(&held, "Bright"), Some(true));
    assert_eq!(checked(&held, "Dim"), Some(false));
    assert_eq!(checked(&held, "Default settings"), Some(false));
    assert!(held.iter().all(|item| !item.0.starts_with("Update")));
    // A slider moved: no ✓, and Update for the preset chosen last.
    let moved = items(&folder, &exposed(0.5), Some("Bright"));
    assert_eq!(checked(&moved, "Bright"), Some(false));
    let words: Vec<&str> = moved.iter().map(|item| item.0.as_str()).collect();
    assert_eq!(
        words,
        [
            "Default settings",
            "Bright",
            "Dim",
            "Save as Preset…",
            "Update “Bright”",
            "Rename…",
            "Delete…"
        ]
    );
    // Released, the mark gone, it matches no more either.
    let mut released = exposed(1.2);
    for control in released.controls.iter_mut() {
        control.mark = None;
    }
    let released = items(&folder, &released, None);
    assert_eq!(checked(&released, "Bright"), Some(false));
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn rename_and_delete_change_the_folder() {
    let folder = scratch("rename");
    saved(&folder, "Bright", &exposed(1.2));
    let path = rename(&folder, &Exposure, "Bright", "Brighter").unwrap();
    assert_eq!(path, folder.join("Brighter.appreset"));
    let names: Vec<String> = read(&folder, &Exposure)
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["Brighter"]);
    delete(&folder, &Exposure, "Brighter").unwrap();
    assert!(read(&folder, &Exposure).is_empty());
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
    // One that has gone is not found.
    assert!(delete(&folder, &Exposure, "Brighter").is_err());
    let _ = fs::remove_dir_all(&folder);
}

/// A preset file of format version `format` holding Exposure at tool version `version`.
fn written(folder: &Path, name: &str, format: u32, version: u32) {
    written_with(folder, name, format, version, r#"control:exposure="1""#);
}

/// The same with `control`, its control fields as written.
fn written_with(folder: &Path, name: &str, format: u32, version: u32, control: &str) {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Apertrust 9.0.0">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:apertrust="https://apertrust.org/ns/xmp/"
    xmlns:control="https://apertrust.org/ns/xmp/control/"
    apertrust:preset="{format}"
    apertrust:name="{name}">
   <apertrust:tools>
    <rdf:Seq>
     <rdf:li>
      <rdf:Description
        apertrust:tool="exposure"
        apertrust:toolVersion="{version}"
        {control}/>
     </rdf:li>
    </rdf:Seq>
   </apertrust:tools>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
"#
    );
    fs::write(folder.join(format!("{name}.appreset")), xml).unwrap();
}

#[test]
fn a_parked_file_is_listed_disabled_with_why() {
    let folder = scratch("parked");
    written(&folder, "Newer file", 2, Exposure.definition().version);
    written(&folder, "Newer tool", 1, 99);
    let items = items(&folder, &untouched(), None);
    let find = |words: &str| items.iter().find(|item| item.0 == words).unwrap().clone();
    let newer = find("Newer file");
    assert!(!newer.2);
    assert_eq!(newer.3.as_deref(), Some("Made by a newer Apertrust"));
    let tool = find("Newer tool");
    assert!(!tool.2);
    assert_eq!(
        tool.3.as_deref(),
        Some("Made for a newer Exposure: version 99")
    );
    let _ = fs::remove_dir_all(&folder);
}

/// A file at the tool's own version holding what this build cannot keep, a control it lacks
/// or a value out of range, was made by a newer Apertrust, not for a newer tool.
#[test]
fn a_value_the_tool_s_version_cannot_keep_is_a_newer_apertrust_s() {
    let folder = scratch("parked-current");
    let version = Exposure.definition().version;
    written_with(
        &folder,
        "Unknown control",
        1,
        version,
        r#"control:nonesuch="1""#,
    );
    written_with(
        &folder,
        "Out of range",
        1,
        version,
        r#"control:exposure="1000""#,
    );
    let items = items(&folder, &untouched(), None);
    for name in ["Unknown control", "Out of range"] {
        let item = items.iter().find(|item| item.0 == name).unwrap();
        assert!(!item.2, "{name}");
        assert_eq!(
            item.3.as_deref(),
            Some("Made by a newer Apertrust"),
            "{name}"
        );
    }
    let _ = fs::remove_dir_all(&folder);
}

/// A switched tool's preset saved while it is on holds it on, and matches only while the
/// tool is on with those values.
#[test]
fn a_switched_tool_s_preset_matches_only_while_it_is_on() {
    use apertrust_tools::vignetting::Vignetting;

    let folder = scratch("switched");
    let mut on = state_of(&Vignetting);
    on.enabled = true;
    on.edited = true;
    let scale = (on.controls.iter_mut())
        .find(|control| control.control == "scale")
        .unwrap();
    let ControlValue::Number(default) = scale.default else {
        panic!("a number");
    };
    scale.value = ControlValue::Number(default / 2.0);
    scale.mark = Some(Mark::Kept);
    save(&folder, &Vignetting, &on, ("Soft", None), "0.1.0").unwrap();
    let stored = &files::list(&folder, Kind::Preset).files[0];
    assert!(matches!(
        stored.entries.as_slice(),
        [Entry::Tool { enabled: Some(on), .. }] if on == "true"
    ));
    let listed = read(&folder, &Vignetting);
    let values = listed[0].values.as_ref().unwrap();
    assert!(values.enabled());
    let definition = Vignetting.definition();
    assert!(matches(definition, &on, values));
    let mut off = on.clone();
    off.enabled = false;
    assert!(!matches(definition, &off, values));
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn an_item_s_id_reads_back() {
    for item in [
        Item::Default,
        Item::Apply("Bright: interiors".to_owned()),
        Item::Save,
        Item::Update("a".to_owned()),
        Item::Rename("b".to_owned()),
        Item::Delete("c".to_owned()),
    ] {
        assert_eq!(Item::of(&item.id()), Some(item));
    }
    assert_eq!(Item::of("reset"), None);
}

/// Through a real session: a preset applied is one step, noted with its name, and Undo
/// takes it back.
#[test]
fn a_real_session_applies_a_preset_as_one_step_that_undo_takes_back() {
    use crate::mirror::tests::{Live, scratch};

    let Some(mut live) = Live::start() else {
        return;
    };
    let folder = scratch("preset-applied");
    live.open(&folder.join("preset.ARW"));
    live.until_live();
    let exposure = |m: &crate::mirror::Mirror| {
        let state = m.values.tools.iter().find(|s| s.tool == "exposure")?;
        let shown = state.controls.iter().find(|c| c.control == "exposure")?;
        Some(shown.value.clone())
    };
    let before = exposure(&live.mirror);
    let steps = |m: &crate::mirror::Mirror| m.history.as_ref().map_or(0, |h| h.steps.len());
    let start = steps(&live.mirror);
    let mut values = ToolValues::new();
    values.set("exposure", ControlValue::Number(1.25));
    values.set_mark("exposure", Some(Mark::Kept));
    live.session.send(apply("exposure", "Bright", values));
    live.until(|m| exposure(m) == Some(ControlValue::Number(1.25)));
    live.until(|m| steps(m) > start);
    let history = live.mirror.history.clone().unwrap();
    // A new photo's default steps first, where it had none, then the preset's one.
    let last = history.steps.last().unwrap();
    assert_eq!(history.end, history.steps.len());
    assert_eq!(
        last.note,
        Some(Note::Message {
            id: NOTE.to_owned(),
            args: vec![("name".to_owned(), "Bright".to_owned())],
        })
    );
    assert_eq!(last.changes.len(), 1);
    let words = Translator::new(Locale::EnGb);
    let tools = live.mirror.tools();
    let listed = crate::history::listed(Some(&history), tools, &words, crate::history::today());
    assert!(
        listed.lines().any(|line| line.name == "Exposure"),
        "{listed:?}"
    );
    live.session.send(apertrust_engine::session::Command::Undo);
    live.until(|m| exposure(m) == before);
    let _ = fs::remove_dir_all(&folder);
}

fn photograph(which: Builtin, name: &str) {
    let folder = scratch(name);
    saved(&folder, "Bright interiors", &exposed(1.2));
    saved(&folder, "Night street", &exposed(-0.5));
    written(&folder, "Old look", 2, 1);
    let mut harness = panel(which, &folder, exposed(1.2));
    harness.state_mut().menu.chosen = Some("Bright interiors".to_owned());
    opened(&mut harness);
    harness.snapshot(name);
    let _ = fs::remove_dir_all(&folder);
}

#[test]
fn presets_in_the_dark_theme() {
    photograph(Builtin::Dark, "presets-dark");
}

#[test]
fn presets_in_the_light_theme() {
    photograph(Builtin::Light, "presets-light");
}
