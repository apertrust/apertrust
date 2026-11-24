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

//! A tool's presets (decision 106; `docs/gui.md`, A tool's presets): the pull-down its
//! header's presets button opens, and what each item does.
//!
//! - **The folder** is `<user data>/presets/<tool>/`, read through
//!   `apertrust_library::presets` as the pull-down opens ([`read`]), never watched.
//! - **The items** ([`entries`]): Default settings, then the person's presets by name in the
//!   file browser's order, ✓ before each whose values the tool holds now ([`matches()`]), a
//!   parked file disabled with why as its tooltip, then Save as Preset…, Update "…"
//!   while the values differ from the preset last applied or saved on the tool this run,
//!   Rename… and Delete…, which act on that preset.
//! - **Choosing** a preset is one history step, `Step { source: You }` noted with its name,
//!   holding `SetTool` ([`apply`]); Default settings the same holding `ResetTool`
//!   ([`default_settings`]).
//! - **Save as Preset…** and **Rename…** ask for a name in a dialog of the host's
//!   ([`NameDialog`]), a sheet on the Mac; **Update** and **Delete** ask first with the
//!   system's alert (`alerts.rs`), and rest where the platform shows none.
//! - **A utility's presets,** Export's, are the same pull-down ([`pull_down`]), dialogs
//!   and alerts over the settings its page stores ([`Settings`]): its built-in presets
//!   after Default settings ([`builtins`]), then the person's ([`read_settings`]), each
//!   applied through the settings store with no history step (`export.rs`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use apertrust_base::folders::Folders;
use apertrust_engine::session::{
    Command, Entry, Note, Parked, StepSource, ToolState, ToolValues, tool_entry, tool_values,
};
use apertrust_library::presets::{self as files, Kind, PresetError, Stored};
use apertrust_tools::{ProcessingDefinition, ProcessingTool, messages};
use apertrust_ui::components::{Button, ButtonOrder, TextField};
use apertrust_ui::i18n::Args;
use apertrust_ui::popup::{Entry as MenuEntry, Item as MenuItem};
use apertrust_ui::{Theme, Translator, theme, tr, translator};
use eframe::egui::{self, Label, RichText, os::OperatingSystem};

use crate::dialog_host::{Asks, Dialog, Form, Keys, Look, Output, platform};

/// The message id of a preset's history note, its name as `name`.
pub const NOTE: &str = "history-preset";
/// The message id of Default settings' history note.
pub const DEFAULT_NOTE: &str = "history-default-settings";

/// Where the tools' presets live, and the one each tool last had applied or saved this
/// run, which Update, Rename… and Delete… act on.
#[derive(Debug, Default)]
pub struct Presets {
    folders: Option<Folders>,
    chosen: BTreeMap<String, String>,
}

impl Presets {
    /// The presets of the user data folder `folders` chose; none where the app has no user
    /// data folder, whose presets buttons are then drawn disabled.
    pub fn new(folders: Option<Folders>) -> Presets {
        Presets {
            folders,
            chosen: BTreeMap::new(),
        }
    }

    /// `tool`'s presets folder, which may not exist yet.
    pub fn folder(&self, tool: &str) -> Option<PathBuf> {
        self.folders.as_ref().map(|folders| folders.presets(tool))
    }

    /// What `tool`'s header needs to build its pull-down, where the app has a user data
    /// folder; `alerts` whether the platform can ask before Update and Delete.
    pub fn menu(&self, tool: &str, alerts: bool) -> Option<Menu> {
        Some(Menu {
            folder: self.folder(tool)?,
            chosen: self.chosen.get(tool).cloned(),
            alerts,
        })
    }

    /// The preset last applied or saved on `tool` this run.
    pub fn chosen(&self, tool: &str) -> Option<&str> {
        self.chosen.get(tool).map(String::as_str)
    }

    /// `name` is now `tool`'s preset applied or saved last.
    pub fn choose(&mut self, tool: &str, name: &str) {
        self.chosen.insert(tool.to_owned(), name.to_owned());
    }

    /// `tool`'s preset `name` is gone: Update, Rename… and Delete… act on none.
    pub fn forget(&mut self, tool: &str, name: &str) {
        if self.chosen(tool) == Some(name) {
            self.chosen.remove(tool);
        }
    }
}

/// What a tool's header needs to build its presets' pull-down as it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// The tool's presets folder.
    pub folder: PathBuf,
    /// The preset last applied or saved on the tool this run.
    pub chosen: Option<String>,
    /// Whether the platform asks before Update and Delete: they rest where it cannot.
    pub alerts: bool,
}

/// Why a preset is listed but cannot be applied, said in its item's tooltip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// A newer Apertrust wrote it: a format version or a field this build does not know,
    /// or, at the tool's version, a value it cannot keep.
    Newer,
    /// Made for a newer version of its tool, `toolVersion` as written.
    NewerTool(String),
}

/// One of a tool's preset files as the pull-down lists it: a processing tool's, its values
/// `ToolValues`, or a utility's, its [`Settings`].
#[derive(Clone, Debug, PartialEq)]
pub struct Listed<V = ToolValues> {
    pub name: String,
    pub path: PathBuf,
    /// Its values, a processing tool's migrated and clamped, or why it is parked.
    pub values: Result<V, Why>,
}

/// A utility preset's settings (decision 106): each setting's key in full and its value as
/// text, as `--conf` takes it.
pub type Settings = Vec<(String, String)>;

/// `tool`'s presets in `folder`, by name in the file browser's order: each file read, its
/// one entry made the tool's values (`session::tool_values`), or parked, for a newer
/// version of the tool only where its `toolVersion` is above the tool's. A file that is no
/// preset of `tool`, holding another tool's entry or not one entry, or that cannot be
/// read, is left out with a warning under `-d settings`.
pub fn read(folder: &Path, tool: &dyn ProcessingTool) -> Vec<Listed> {
    let definition = tool.definition();
    let id = definition.tool.id;
    let listed = files::list(folder, Kind::Preset);
    for (path, error) in &listed.failed {
        let path = path.display();
        tracing::warn!(target: "settings", "preset {path} not read: {error}");
    }
    let mut presets = Vec::new();
    for stored in listed.files {
        let path = stored.path.display();
        let values = match stored.entries.as_slice() {
            [Entry::Tool { tool: of, .. }] if of.as_str() != id => {
                tracing::warn!(target: "settings", "preset {path} is {of}'s, not {id}'s: left out");
                continue;
            }
            _ if stored.newer().is_some() => Err(Why::Newer),
            // Only this tool's entry is read, so it is never unknown nor retired here.
            [entry] => match tool_values(entry, &[tool], apertrust_tools::RETIRED) {
                Ok((_, values)) => Ok(values),
                Err(Parked::NewerTool { version })
                    if version.parse::<u32>().is_ok_and(|v| v > definition.version) =>
                {
                    Err(Why::NewerTool(version))
                }
                Err(_) => Err(Why::Newer),
            },
            _ => {
                tracing::warn!(target: "settings", "preset {path} holds no one tool: left out");
                continue;
            }
        };
        presets.push(Listed {
            name: stored.name.trim().to_owned(),
            path: stored.path.clone(),
            values,
        });
    }
    presets
}

/// Whether `state`, the tool's values now, are what applying `values` gives: every
/// control's value, its default where `values` holds none, and its mark, and for a
/// switched tool whether it is on. A value at its default counts as unset, as a step
/// compares them.
pub fn matches(definition: &ProcessingDefinition, state: &ToolState, values: &ToolValues) -> bool {
    let controls = state.controls.iter().all(|control| {
        let value = values.get(control.control).unwrap_or(&control.default);
        *value == control.value && values.mark(control.control) == control.mark
    });
    controls && (!definition.switched || values.enabled() == state.enabled)
}

/// The tool's values now, as a preset saves them: those off their defaults and every mark,
/// and for a switched tool whether it is on, off holding its marks alone (decision 32).
pub fn current(definition: &ProcessingDefinition, state: &ToolState) -> ToolValues {
    let mut values = ToolValues::new();
    let on = !definition.switched || state.enabled;
    for control in &state.controls {
        if on && control.value != control.default {
            values.set(control.control, control.value.clone());
        }
        values.set_mark(control.control, control.mark);
    }
    if definition.switched {
        values.set_enabled(state.enabled);
    }
    values
}

/// An item of the pull-down, by what it does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// Default settings: `ResetTool`.
    Default,
    /// The preset built in with this id applied.
    Builtin(String),
    /// The preset of this name applied.
    Apply(String),
    /// Save as Preset…: the name dialog.
    Save,
    /// Update "…": the alert, then the file written with the tool's values now.
    Update(String),
    /// Rename…: the name dialog.
    Rename(String),
    /// Delete…: the alert, then the file removed.
    Delete(String),
}

impl Item {
    /// Its id in the pull-down.
    pub fn id(&self) -> String {
        match self {
            Item::Default => "preset-default".to_owned(),
            Item::Builtin(id) => format!("preset-builtin:{id}"),
            Item::Apply(name) => format!("preset-apply:{name}"),
            Item::Save => "preset-save".to_owned(),
            Item::Update(name) => format!("preset-update:{name}"),
            Item::Rename(name) => format!("preset-rename:{name}"),
            Item::Delete(name) => format!("preset-delete:{name}"),
        }
    }

    /// The item of the pull-down's id `id`, if it is one.
    pub fn of(id: &str) -> Option<Item> {
        let named = |prefix: &str| id.strip_prefix(prefix).map(str::to_owned);
        match id {
            "preset-default" => Some(Item::Default),
            "preset-save" => Some(Item::Save),
            _ => (named("preset-builtin:").map(Item::Builtin))
                .or_else(|| named("preset-apply:").map(Item::Apply))
                .or_else(|| named("preset-update:").map(Item::Update))
                .or_else(|| named("preset-rename:").map(Item::Rename))
                .or_else(|| named("preset-delete:").map(Item::Delete)),
        }
    }

    /// Whether choosing it applies values: Default settings, a built-in preset or the
    /// person's. A processing tool's then change the photo's edit; Export's change its
    /// settings alone, and none of a processing tool's is built in yet.
    pub fn edits(&self) -> bool {
        matches!(self, Item::Default | Item::Builtin(_) | Item::Apply(_))
    }
}

/// The pull-down of `tool`'s presets, `listed` ([`read`]), with the values `state` holds
/// now, in `words` ([`pull_down`]): Default settings, ✓ while the tool is at its defaults,
/// then each preset, ✓ while the tool holds its values, a parked one disabled with why.
pub fn entries(
    tool: &dyn ProcessingTool,
    state: &ToolState,
    listed: &[Listed],
    menu: &Menu,
    words: &Translator,
) -> Vec<MenuEntry> {
    let definition = tool.definition();
    let title = words.text(&messages::title(definition.tool.id));
    let at_defaults = matches(definition, state, &ToolValues::new());
    let lines = (listed.iter())
        .map(|preset| Line {
            item: Item::Apply(preset.name.clone()),
            name: preset.name.clone(),
            checked: match &preset.values {
                Ok(values) => Ok(matches(definition, state, values)),
                Err(why) => Err(parked(words, why, &title)),
            },
        })
        .collect();
    pull_down(at_defaults, [Vec::new(), lines], menu, words)
}

/// One preset as the pull-down lists it: its item, its name, and whether the values now
/// match it, or why it is parked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub item: Item,
    pub name: String,
    pub checked: Result<bool, String>,
}

/// The pull-down of a tool's presets in `words`, every tool's: Default settings, ✓ while
/// `at_defaults`; then the built-in presets and the person's, `lines`, each group after a
/// line, ✓ on one that matches, a parked one disabled with why; a line, then Save as
/// Preset…, Update "…" while the values differ from the chosen preset's, Rename… and
/// Delete…, the last three on the person's preset `menu.chosen`, disabled with "Choose a
/// preset first" while there is none, and Update and Delete while the platform cannot ask
/// first.
pub fn pull_down(
    at_defaults: bool,
    lines: [Vec<Line>; 2],
    menu: &Menu,
    words: &Translator,
) -> Vec<MenuEntry> {
    let mut entries = vec![MenuEntry::Item(
        MenuItem::new(Item::Default.id(), words.text("presets-default")).checked(at_defaults),
    )];
    for group in &lines {
        if !group.is_empty() {
            entries.push(MenuEntry::Separator);
        }
        for line in group {
            let item = MenuItem::new(line.item.id(), line.name.clone());
            entries.push(MenuEntry::Item(match &line.checked {
                Ok(checked) => item.checked(*checked),
                Err(why) => item.checked(false).enabled(false).tip(why.clone()),
            }));
        }
    }
    entries.push(MenuEntry::Separator);
    entries.push(MenuEntry::Item(MenuItem::new(
        Item::Save.id(),
        words.text("presets-save"),
    )));
    let [_, own] = &lines;
    let chosen = (menu.chosen.as_deref())
        .and_then(|name| own.iter().find(|line| line.name == name))
        .filter(|line| line.checked.is_ok());
    let first = words.text("presets-choose-first");
    let no_alert = words.text("presets-no-alert");
    if let Some(line) = chosen
        && line.checked == Ok(false)
    {
        let mut args = Args::new();
        args.set("name", line.name.as_str());
        let update = words.text_with("presets-update", Some(&args));
        let item = MenuItem::new(Item::Update(line.name.clone()).id(), update);
        entries.push(MenuEntry::Item(if menu.alerts {
            item
        } else {
            item.enabled(false).tip(no_alert.clone())
        }));
    }
    let name = chosen.map(|line| line.name.clone()).unwrap_or_default();
    let rename = MenuItem::new(
        Item::Rename(name.clone()).id(),
        words.text("presets-rename"),
    );
    let delete = MenuItem::new(Item::Delete(name).id(), words.text("presets-delete"));
    match chosen {
        Some(_) => {
            entries.push(MenuEntry::Item(rename));
            entries.push(MenuEntry::Item(if menu.alerts {
                delete
            } else {
                delete.enabled(false).tip(no_alert)
            }));
        }
        None => {
            for item in [rename, delete] {
                entries.push(MenuEntry::Item(item.enabled(false).tip(first.clone())));
            }
        }
    }
    entries
}

/// Why a parked preset of the tool titled `title` cannot be applied, in `words`.
pub fn parked(words: &Translator, why: &Why, title: &str) -> String {
    match why {
        Why::Newer => words.text("presets-parked-newer"),
        Why::NewerTool(version) => {
            let mut args = Args::new();
            args.set("tool", title);
            args.set("version", version.as_str());
            words.text_with("presets-parked-tool", Some(&args))
        }
    }
}

/// The step that applies the preset `name`'s `values` to `tool`: one history step, source
/// You, noted with its name ([`NOTE`]).
pub fn apply(tool: &str, name: &str, values: ToolValues) -> Command {
    Command::Step {
        source: StepSource::You,
        note: Some(Note::Message {
            id: NOTE.to_owned(),
            args: vec![("name".to_owned(), name.to_owned())],
        }),
        commands: vec![Command::SetTool {
            tool: tool.to_owned(),
            values,
        }],
    }
}

/// The step Default settings sends: `tool`'s reset, noted Default settings
/// ([`DEFAULT_NOTE`]).
pub fn default_settings(tool: &str) -> Command {
    Command::Step {
        source: StepSource::You,
        note: Some(Note::Message {
            id: DEFAULT_NOTE.to_owned(),
            args: Vec::new(),
        }),
        commands: vec![Command::ResetTool {
            tool: tool.to_owned(),
        }],
    }
}

/// The preset named `name` among `tool`'s in `folder`, read afresh as it is chosen.
pub fn find(folder: &Path, tool: &dyn ProcessingTool, name: &str) -> Option<Listed> {
    read(folder, tool)
        .into_iter()
        .find(|preset| preset.name == name)
}

/// Writes `tool`'s values now, `state`, as the preset `name` in `folder`, with
/// `description`, over the file holding that name if there is one, `build` recorded in
/// it. Returns its path.
pub fn save(
    folder: &Path,
    tool: &dyn ProcessingTool,
    state: &ToolState,
    (name, description): (&str, Option<&str>),
    build: &str,
) -> Result<PathBuf, PresetError> {
    let definition = tool.definition();
    let mut stored = Stored::new(Kind::Preset, name);
    stored.description = description.map(str::to_owned);
    stored.entries = vec![tool_entry(definition, &current(definition, state))];
    files::write(folder, &stored, build)
}

/// Writes `tool`'s values now, `state`, over its preset `name` in `folder`, keeping its
/// description. Returns its path.
pub fn update(
    folder: &Path,
    tool: &dyn ProcessingTool,
    state: &ToolState,
    name: &str,
    build: &str,
) -> Result<PathBuf, PresetError> {
    let description = (files::list(folder, Kind::Preset).files.into_iter())
        .find(|stored| stored.name.trim() == name)
        .and_then(|stored| stored.description);
    save(folder, tool, state, (name, description.as_deref()), build)
}

/// Renames `tool`'s preset `from` in `folder` to `to`. Returns the file's new path.
pub fn rename(
    folder: &Path,
    tool: &dyn ProcessingTool,
    from: &str,
    to: &str,
) -> Result<PathBuf, PresetError> {
    files::rename(&path_of(read(folder, tool), from)?, to)
}

/// Deletes `tool`'s preset `name` in `folder`.
pub fn delete(folder: &Path, tool: &dyn ProcessingTool, name: &str) -> Result<(), PresetError> {
    files::delete(&path_of(read(folder, tool), name)?)
}

/// The file of the preset `name` among `listed`.
fn path_of<V>(listed: Vec<Listed<V>>, name: &str) -> Result<PathBuf, PresetError> {
    (listed.into_iter())
        .find(|preset| preset.name == name)
        .map(|preset| preset.path)
        .ok_or(PresetError::NotAPreset)
}

/// The names a new preset of `tool` cannot take in `folder`: its presets' and Default
/// settings', as `words` show it (decision 106).
pub fn taken(folder: &Path, tool: &dyn ProcessingTool, words: &Translator) -> Vec<String> {
    let mut names: Vec<String> = read(folder, tool).into_iter().map(|p| p.name).collect();
    names.push(words.text("presets-default"));
    names
}

/// The utility `tool`'s presets in `folder`, by name in the file browser's order, each
/// with its settings, or parked where a newer Apertrust wrote it. A file that holds no
/// settings of `tool`'s, a processing tool's preset or another utility's, or that cannot
/// be read, is left out with a warning under `-d settings`.
pub fn read_settings(folder: &Path, tool: &str) -> Vec<Listed<Settings>> {
    let listed = files::list(folder, Kind::Preset);
    for (path, error) in &listed.failed {
        let path = path.display();
        tracing::warn!(target: "settings", "preset {path} not read: {error}");
    }
    let mut presets = Vec::new();
    for stored in listed.files {
        if !stored.entries.is_empty() || stored.settings_tool.as_deref() != Some(tool) {
            let path = stored.path.display();
            tracing::warn!(target: "settings", "preset {path} holds no settings of {tool}'s: left out");
            continue;
        }
        let values = match stored.newer() {
            Some(_) => Err(Why::Newer),
            None => Ok(stored.settings),
        };
        presets.push(Listed {
            name: stored.name.trim().to_owned(),
            path: stored.path,
            values,
        });
    }
    presets
}

/// The utility `tool`'s presets built in, in their order, each by its id with its
/// settings (`apertrust_library::presets::builtins`).
pub fn builtins(tool: &str) -> Vec<(&'static str, Settings)> {
    (files::builtins(tool).into_iter())
        .map(|(id, stored)| (id, stored.settings))
        .collect()
}

/// The name the preset built in for `tool` as `id` shows, the Fluent message
/// `preset-<tool>-<id>`, in `words`.
pub fn builtin_name(tool: &str, id: &str, words: &Translator) -> String {
    words.text(&format!("preset-{tool}-{id}"))
}

/// The names a new preset of the utility `tool` cannot take in `folder`: its presets',
/// Default settings' and its built-in presets', as `words` show them (decision 106).
pub fn taken_settings(folder: &Path, tool: &str, words: &Translator) -> Vec<String> {
    let own = read_settings(folder, tool).into_iter().map(|p| p.name);
    let builtins = (builtins(tool).into_iter()).map(|(id, _)| builtin_name(tool, id, words));
    let mut names: Vec<String> = own.chain(builtins).collect();
    names.push(words.text("presets-default"));
    names
}

/// Writes `settings`, the utility `tool`'s, as the preset `name` in `folder`, with
/// `description`, over the file holding that name if there is one, `build` recorded in
/// it. Returns its path.
pub fn save_settings(
    folder: &Path,
    tool: &str,
    settings: Settings,
    (name, description): (&str, Option<&str>),
    build: &str,
) -> Result<PathBuf, PresetError> {
    let mut stored = Stored::new(Kind::Preset, name);
    stored.description = description.map(str::to_owned);
    stored.settings_tool = Some(tool.to_owned());
    stored.settings = settings;
    files::write(folder, &stored, build)
}

/// Writes `settings` over the utility `tool`'s preset `name` in `folder`, keeping its
/// description and, as read, each setting of the file's that `settings` lack: a newer
/// build's, which this one does not know, or one `--conf` holds for the run (decision
/// 106). Returns its path.
pub fn update_settings(
    folder: &Path,
    tool: &str,
    mut settings: Settings,
    name: &str,
    build: &str,
) -> Result<PathBuf, PresetError> {
    let path = path_of(read_settings(folder, tool), name)?;
    let stored = (files::list(folder, Kind::Preset).files.into_iter())
        .find(|stored| stored.path == path)
        .ok_or(PresetError::NotAPreset)?;
    for (key, text) in stored.settings {
        if !settings.iter().any(|(own, _)| *own == key) {
            settings.push((key, text));
        }
    }
    let named = (name, stored.description.as_deref());
    save_settings(folder, tool, settings, named, build)
}

/// Renames the utility `tool`'s preset `from` in `folder` to `to`, everything else in its
/// file kept. Returns the file's new path.
pub fn rename_settings(
    folder: &Path,
    tool: &str,
    from: &str,
    to: &str,
) -> Result<PathBuf, PresetError> {
    files::rename(&path_of(read_settings(folder, tool), from)?, to)
}

/// Deletes the utility `tool`'s preset `name` in `folder`.
pub fn delete_settings(folder: &Path, tool: &str, name: &str) -> Result<(), PresetError> {
    files::delete(&path_of(read_settings(folder, tool), name)?)
}

/// The first name of `title` and a number, 1, 2 and on, that is not `taken`, in `words`
/// ("Exposure 1").
pub fn free_name(title: &str, taken: &[String], words: &Translator) -> String {
    (1..)
        .map(|number: usize| {
            let mut args = Args::new();
            args.set("tool", title);
            args.set("number", number as f64);
            words.text_with("presets-new-name", Some(&args))
        })
        .find(|name| !taken.contains(name))
        .expect("a free name")
}

/// What the name dialog asked of the app as it closed with its button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Save as Preset…'s Save: `tool`'s values now written as `name`.
    Save {
        tool: &'static str,
        name: String,
        description: Option<String>,
    },
    /// Rename…'s Rename: `tool`'s preset `from` named `to`.
    Rename {
        tool: &'static str,
        from: String,
        to: String,
    },
}

/// Which of the two the name dialog is.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Purpose {
    Save,
    Rename(String),
}

/// Save as Preset… and Rename… (decision 41): a sheet on the Mac, a modal window
/// elsewhere, with Name, its first focus, Save's Description, optional, and the button,
/// the default, with Cancel. An empty name, or one `taken`, disables the button and says
/// why under the field. Return presses it; Escape cancels, from the field too.
#[derive(Debug)]
pub struct NameDialog {
    purpose: Purpose,
    tool: &'static str,
    /// The tool's title, translated as the dialog opened.
    title: String,
    /// The names it cannot take, trimmed.
    taken: Vec<String>,
    name: String,
    description: String,
    /// Whether the name has had the focus, given as the dialog first draws.
    focused: bool,
}

impl NameDialog {
    /// Save as Preset… for `tool`, titled `title`, its name `name` at first.
    pub fn save(tool: &'static str, title: String, taken: Vec<String>, name: String) -> Self {
        NameDialog {
            purpose: Purpose::Save,
            tool,
            title,
            taken,
            name,
            description: String::new(),
            focused: false,
        }
    }

    /// Rename… for `tool`'s preset `from`, titled `title`; `taken` holds every name, its
    /// own among them, which it may keep.
    pub fn rename(tool: &'static str, title: String, taken: Vec<String>, from: String) -> Self {
        let taken = taken.into_iter().filter(|name| *name != from).collect();
        NameDialog {
            purpose: Purpose::Rename(from.clone()),
            tool,
            title,
            taken,
            name: from,
            description: String::new(),
            focused: false,
        }
    }

    /// Why the name cannot be taken, in `words`; none where it can.
    fn refused(&self, words: &Translator) -> Option<String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Some(words.text("presets-name-empty"));
        }
        self.taken.iter().any(|taken| taken == name).then(|| {
            let mut args = Args::new();
            args.set("tool", self.title.as_str());
            words.text_with("presets-name-taken", Some(&args))
        })
    }

    /// What its button asks.
    fn asked(&self) -> Asked {
        let name = self.name.trim().to_owned();
        match &self.purpose {
            Purpose::Save => {
                let description = self.description.trim();
                Asked::Save {
                    tool: self.tool,
                    name,
                    description: (!description.is_empty()).then(|| description.to_owned()),
                }
            }
            Purpose::Rename(from) => Asked::Rename {
                tool: self.tool,
                from: from.clone(),
                to: name,
            },
        }
    }
}

/// The dialog's width inside its margin, in points.
const WIDTH: f32 = 320.0;

impl Dialog for NameDialog {
    fn id(&self) -> &'static str {
        "preset-name"
    }

    fn title(&self, ctx: &egui::Context) -> String {
        match self.purpose {
            Purpose::Save => tr(ctx, "presets-save-title"),
            Purpose::Rename(_) => tr(ctx, "presets-rename-title"),
        }
    }

    fn form(&self, _os: OperatingSystem) -> Form {
        Form::Sheet
    }

    fn look(&self, t: &Theme) -> Look {
        Look {
            background: t.colors.panel,
            margin: t.spacing.l,
        }
    }

    fn keys(&self, _os: OperatingSystem) -> Keys {
        Keys {
            escape: true,
            close_window: true,
            cancel: true,
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) -> Asks {
        let ctx = ui.ctx().clone();
        let (t, words) = (theme(&ctx), translator(&ctx));
        let refused = self.refused(&words);
        let mut pressed = None;
        let mut enter = false;
        ui.scope(|ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing = egui::vec2(t.spacing.s, t.spacing.s);
            // A sheet has no frame to hold its title.
            if !platform::draws_frame(ctx.os()) {
                let title = self.title(&ctx);
                ui.add(Label::new(RichText::new(title).heading()).selectable(false));
            }
            let label = words.text("presets-name");
            ui.add(Label::new(&label).selectable(false));
            let id = ui.make_persistent_id("preset name");
            let (field, typed) = TextField::new(id, &label, &self.name)
                .error(refused.is_some() && !self.name.is_empty())
                .show(ui);
            if !self.focused {
                field.request_focus();
                self.focused = true;
            }
            if let Some(typed) = typed {
                self.name = typed;
            }
            enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            // The reason under the field, the text that disables the button.
            let refused = self.refused(&words);
            if let Some(why) = &refused {
                let why = RichText::new(why).size(t.text.small).color(t.colors.error);
                ui.add(Label::new(why).selectable(false));
            }
            if self.purpose == Purpose::Save {
                let label = words.text("presets-description");
                ui.add(Label::new(&label).selectable(false));
                let hint = words.text("presets-description-hint");
                let id = ui.make_persistent_id("preset description");
                let (_, typed) = TextField::new(id, &label, &self.description)
                    .hint(&hint)
                    .show(ui);
                if let Some(typed) = typed {
                    self.description = typed;
                }
            }
            ui.add_space(t.spacing.s);
            let button = match self.purpose {
                Purpose::Save => words.text("presets-save-button"),
                Purpose::Rename(_) => words.text("presets-rename-button"),
            };
            let cancel = words.text("alert-cancel");
            let able = refused.is_none();
            pressed = buttons(ui, [(&button, able), (&cancel, true)]);
        });
        let able = self.refused(&words).is_none();
        let confirmed = pressed == Some(0) || enter && able;
        Asks {
            outputs: if confirmed {
                vec![Output::Preset(self.asked())]
            } else {
                Vec::new()
            },
            close: confirmed || pressed == Some(1),
            // Escape cancels from the field too, which takes nothing of it.
            gives_escape: true,
            ..Asks::default()
        }
    }
}

/// The dialog's two buttons at the end of a row, in the platform's order, the first the
/// default, each `(words, enabled)`: returns which was clicked. Their width is the last
/// pass's, as `dialog_host::buttons` measures it.
fn buttons(ui: &mut egui::Ui, buttons: [(&str, bool); 2]) -> Option<usize> {
    let id = ui.make_persistent_id("preset buttons");
    let width: Option<f32> = ui.data(|d| d.get_temp(id));
    let mut clicked = None;
    let row = ui.horizontal(|ui| {
        ui.add_space((ui.available_width() - width.unwrap_or(0.0)).max(0.0));
        let start = ui.cursor().min.x;
        for i in ButtonOrder::platform().arrange(buttons.len(), 0) {
            let (words, enabled) = buttons[i];
            if Button::new(words)
                .primary(i == 0)
                .enabled(enabled)
                .show(ui)
                .clicked()
            {
                clicked = Some(i);
            }
        }
        ui.min_rect().max.x - start
    });
    if width != Some(row.inner) {
        ui.data_mut(|d| d.insert_temp(id, row.inner));
        ui.ctx()
            .request_discard("a preset dialog's buttons measured");
    }
    clicked
}

#[cfg(test)]
mod tests;
