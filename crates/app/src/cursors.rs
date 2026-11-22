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

//! The cursors in the app (decision 104; `docs/cursors.md`): the cursor that decides what a
//! press on the photo does, one chosen at a time.
//!
//! - **The registry** ([`registry`]), made once as the app starts: the app's own cursors,
//!   Select and Pan ([`AppCursors`], a [`CursorOwner`]), and every tool's picker cursor
//!   (`ToolDefinition::cursors`), in the well's groups and their order, with each group's
//!   settings, Pick's Show the last pick among them.
//! - **The mode** ([`Mode`]): the cursor chosen, Select at every start and never saved;
//!   each group's active cursor, the one its face shows, kept for the run.
//! - **The well's words** ([`Well`]): each group's cursors with their names, keys, what
//!   they do and why one cannot act now, which the Toolbar draws as `CursorWell`.
//!
//! The engine never hears of cursors: a gesture becomes commands its owner already sends.

use std::collections::BTreeMap;

use apertrust_tools::{
    ControlKind, CursorDef, CursorGroup, CursorGroupDef, Gesture, ProcessingTool, Row, SettingRow,
    SettingWords, Surface, Track,
};
use apertrust_ui::components::{CursorWell, CursorWellAction, WellCursor, WellGroup, WellSetting};
use apertrust_ui::{Icon, Translator};
use eframe::egui;

use crate::actions::{ActionId, Keymap};
use crate::pickers;
use crate::settings::ToolSettings;

/// Who a cursor belongs to: the app, or a tool, by its id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Owner {
    App,
    Tool(&'static str),
}

/// A cursor, by its owner and its id among the owner's cursors, as `CursorDef::id`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cursor {
    pub owner: Owner,
    pub id: &'static str,
}

impl Cursor {
    /// Select, the default: a click selects a mark, a drag moves it, and a drag that
    /// starts on empty photo pans a zoomed photo.
    pub const SELECT: Cursor = Cursor {
        owner: Owner::App,
        id: "select",
    };
    /// Pan: any drag pans a zoomed photo, one that starts on a mark too.
    pub const PAN: Cursor = Cursor {
        owner: Owner::App,
        id: "pan",
    };

    /// Its action in the registry (decision 98): the app's `cursor-<id>`, a tool's
    /// `tool-<tool>-cursor-<id>`, which its key, its face and its line in its group's list
    /// run.
    pub fn action(self) -> ActionId {
        match (self.owner, self.id) {
            (Owner::App, "select") => ActionId::CURSOR_SELECT,
            (Owner::App, "pan") => ActionId::CURSOR_PAN,
            _ => crate::actions::cursor_action(self),
        }
    }
}

/// What a cursor's readiness and its press read of the app this frame.
pub struct Context<'a> {
    /// Why no pick can be made on the photo now, in the interface's words: the photo
    /// still opening, or open for viewing only; None where picks are taken.
    pub why: Option<&'a str>,
    /// The tool the History tab's eye hides while it is held, whose picks wait for it.
    pub hidden: Option<&'static str>,
    /// The tools' settings as the app keeps them, which a pick is sent with.
    pub settings: &'a ToolSettings,
    /// The photo open, which a reference measured on it names as this photo.
    pub photo: Option<&'a std::path::Path>,
    pub words: &'a Translator,
}

/// What every owner of cursors but a tool implements (`docs/cursors.md`, The API); a tool's
/// picker cursors share one built-in owner ([`ready`]).
pub trait CursorOwner: Sync {
    fn cursors(&self) -> &'static [CursorDef];
    /// The settings of the groups it owns, declared once.
    fn groups(&self) -> &'static [CursorGroupDef] {
        &[]
    }
    /// Whether a press with `cursor` can act now, or why not, in the interface's words.
    fn ready(&self, cursor: &str, cx: &Context) -> Result<(), String>;
}

/// The app's own cursors: Select and Pan, whose drags are the built pan the Viewer keeps
/// (`docs/gui.md`, Panning), and the owner of Pick's settings.
pub struct AppCursors;

/// Select: Esc, where nothing focused keeps it.
const SELECT: CursorDef = CursorDef {
    id: "select",
    group: CursorGroup::Select,
    icon: "select",
    key: Some("Esc"),
    order: 100,
    gestures: &[Gesture::Click, Gesture::Area],
    surfaces: &[Surface::Viewer],
    options: &[],
};

/// Pan: A, held, darktable's own key to force panning.
const PAN: CursorDef = CursorDef {
    id: "pan",
    group: CursorGroup::Pan,
    icon: "pan",
    key: Some("A"),
    order: 100,
    gestures: &[Gesture::Area],
    surfaces: &[Surface::Viewer],
    options: &[],
};

/// Pick's setting: Show the last pick on the photo (`cursors.pick.show_last`).
pub const SHOW_LAST: SettingRow = SettingRow {
    id: "show_last",
    kind: ControlKind::Bool { default: true },
    decimals: 0,
    track: Track::Plain,
    key: Some("cursors.pick.show_last"),
    words: Some(&SettingWords {
        label: "cursor-setting-pick-show-last",
        full: None,
        promise: None,
        options: &[],
    }),
};

impl CursorOwner for AppCursors {
    fn cursors(&self) -> &'static [CursorDef] {
        &[SELECT, PAN]
    }

    fn groups(&self) -> &'static [CursorGroupDef] {
        &[CursorGroupDef {
            group: CursorGroup::Pick,
            settings: &[Row::Setting(SHOW_LAST)],
        }]
    }

    fn ready(&self, _cursor: &str, _cx: &Context) -> Result<(), String> {
        Ok(())
    }
}

/// One cursor of the registry: whose it is, its declaration, and for a tool's, the picker
/// it picks with.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub cursor: Cursor,
    pub def: &'static CursorDef,
    pub picker: Option<&'static str>,
}

/// One group of the well, its cursors in their order, and its settings.
#[derive(Debug)]
pub struct Group {
    pub group: CursorGroup,
    pub cursors: Vec<Entry>,
    pub settings: &'static [Row],
}

/// Every owner's cursors and group settings, by group in the well's order, then by
/// `order`; a group no owner fills is not there.
#[derive(Debug)]
pub struct Registry {
    groups: Vec<Group>,
}

/// Why the registry refused what its owners declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// One owner's id twice.
    Duplicate(Cursor),
    /// A key on two cursors.
    Key(&'static str),
    /// A group's settings declared twice.
    Settings(CursorGroup),
}

impl Registry {
    /// The registry of `owners`' cursors and `tools`' picker cursors.
    pub fn new(
        owners: &[&dyn CursorOwner],
        tools: &[&dyn ProcessingTool],
    ) -> Result<Registry, RegistryError> {
        let mut entries: Vec<Entry> = Vec::new();
        let mut settings = BTreeMap::new();
        for owner in owners {
            for def in owner.cursors() {
                let cursor = Cursor {
                    owner: Owner::App,
                    id: def.id,
                };
                entries.push(Entry {
                    cursor,
                    def,
                    picker: None,
                });
            }
            for declared in owner.groups() {
                if settings.insert(declared.group, declared.settings).is_some() {
                    return Err(RegistryError::Settings(declared.group));
                }
            }
        }
        for tool in tools {
            let definition = &tool.definition().tool;
            for found in definition.cursors {
                let cursor = Cursor {
                    owner: Owner::Tool(definition.id),
                    id: found.cursor.id,
                };
                entries.push(Entry {
                    cursor,
                    def: &found.cursor,
                    picker: Some(found.picker),
                });
            }
        }
        for (i, entry) in entries.iter().enumerate() {
            let before = &entries[..i];
            if before.iter().any(|e| e.cursor == entry.cursor) {
                return Err(RegistryError::Duplicate(entry.cursor));
            }
            if let Some(key) = entry.def.key
                && before.iter().any(|e| e.def.key == Some(key))
            {
                return Err(RegistryError::Key(key));
            }
        }
        entries.sort_by_key(|e| (e.def.group, e.def.order));
        let mut groups: Vec<Group> = Vec::new();
        for entry in entries {
            match groups.last_mut() {
                Some(last) if last.group == entry.def.group => last.cursors.push(entry),
                _ => groups.push(Group {
                    group: entry.def.group,
                    cursors: vec![entry],
                    settings: settings.get(&entry.def.group).copied().unwrap_or(&[]),
                }),
            }
        }
        Ok(Registry { groups })
    }

    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// `cursor`'s entry, if the registry has it.
    pub fn entry(&self, cursor: Cursor) -> Option<&Entry> {
        (self.groups.iter())
            .flat_map(|g| &g.cursors)
            .find(|e| e.cursor == cursor)
    }

    /// Every cursor, in the well's order.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.groups.iter().flat_map(|g| &g.cursors)
    }
}

/// The built-in registry: the app's cursors and the built-in tools', made once.
pub fn registry() -> &'static Registry {
    static MADE: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
    MADE.get_or_init(|| {
        Registry::new(&[&AppCursors], apertrust_tools::TOOLS)
            .expect("the built-in cursors hold together")
    })
}

/// The cursor mode, the app's state for the run (`docs/cursors.md`, A cursor).
#[derive(Debug, Clone)]
pub struct Mode {
    chosen: Cursor,
    /// Each group's active cursor, the one its face shows; a group's first at start.
    active: BTreeMap<CursorGroup, Cursor>,
    /// Pick's Show the last pick on the photo (`cursors.pick.show_last`).
    pub show_last: bool,
}

impl Mode {
    /// Select chosen, each group's first cursor active.
    pub fn new(show_last: bool) -> Mode {
        let active = (registry().groups())
            .iter()
            .map(|g| (g.group, g.cursors[0].cursor))
            .collect();
        Mode {
            chosen: Cursor::SELECT,
            active,
            show_last,
        }
    }

    /// The cursor chosen.
    pub fn chosen(&self) -> Cursor {
        self.chosen
    }

    /// `group`'s active cursor.
    pub fn active(&self, group: CursorGroup) -> Option<Cursor> {
        self.active.get(&group).copied()
    }

    /// Chooses `cursor`, which becomes its group's active one.
    pub fn choose(&mut self, cursor: Cursor) {
        let Some(entry) = registry().entry(cursor) else {
            return;
        };
        self.chosen = cursor;
        self.active.insert(entry.def.group, cursor);
    }
}

/// Whether `entry` can act now, or why not (`docs/cursors.md`, A cursor): the app's own
/// always; a tool's picker cursor while the photo takes picks, its tool is not hidden by
/// the History tab's eye, and its picker is ready with the tool's settings, as Exposure's
/// waits for a reference.
pub fn ready(entry: &Entry, cx: &Context) -> Result<(), String> {
    let (Owner::Tool(tool), Some(picker)) = (entry.cursor.owner, entry.picker) else {
        return AppCursors.ready(entry.cursor.id, cx);
    };
    if let Some(why) = cx.why {
        return Err(why.to_owned());
    }
    let found = apertrust_tools::TOOLS
        .iter()
        .find(|t| t.definition().tool.id == tool);
    if cx.hidden == Some(tool) {
        let mut args = apertrust_ui::i18n::Args::new();
        args.set(
            "tool",
            cx.words.text(&apertrust_tools::messages::title(tool)),
        );
        return Err(cx.words.text_with("section-hidden", Some(&args)));
    }
    let settings = cx.settings.of(tool);
    if let Some(found) = found
        && !found.picker_ready(picker, &settings)
    {
        let why = apertrust_tools::messages::picker_unavailable(tool, picker);
        return Err(cx.words.text(&why));
    }
    Ok(())
}

/// A cursor's words and state as its face and its group's list show them.
struct Shown {
    cursor: Cursor,
    icon: Icon,
    name: String,
    key: Option<String>,
    how: Option<String>,
    /// Why it cannot act now.
    why: Option<String>,
}

/// One group's button as the well draws it.
struct GroupShown {
    name: String,
    cursors: Vec<Shown>,
    active: usize,
    /// Its toggles, by their words, and whether each is on.
    settings: Vec<(String, bool)>,
}

/// The well as this frame draws it (`docs/cursors.md`, The well): each group's cursors with
/// their words and keys, the active one and the one chosen, and the group's settings.
pub struct Well {
    groups: Vec<GroupShown>,
    chosen: Option<usize>,
    /// Why no cursor can be chosen: no photo shown.
    disabled: Option<String>,
}

impl Well {
    /// The well of the registry's groups, as `mode` has them, the keys `keymap`'s on `os`,
    /// each cursor's readiness from `cx`; disabled while no photo is shown.
    pub fn new(
        mode: &Mode,
        cx: &Context,
        keymap: &Keymap,
        os: egui::os::OperatingSystem,
        shown: bool,
    ) -> Well {
        let words = cx.words;
        let mut groups = Vec::new();
        let mut chosen = None;
        for (g, group) in registry().groups().iter().enumerate() {
            let active = mode.active(group.group);
            let cursors: Vec<Shown> = (group.cursors.iter())
                .map(|entry| {
                    let (name, how) = words_of(entry, cx);
                    Shown {
                        cursor: entry.cursor,
                        icon: icon(entry.def.icon),
                        name,
                        key: keymap.tip(entry.cursor.action(), os, words),
                        how,
                        why: ready(entry, cx).err(),
                    }
                })
                .collect();
            let active = (cursors.iter().position(|c| Some(c.cursor) == active)).unwrap_or(0);
            if cursors.iter().any(|c| c.cursor == mode.chosen()) {
                chosen = Some(g);
            }
            let settings = (group.settings.iter())
                .filter_map(|row| match row {
                    Row::Setting(row) if row.id == SHOW_LAST.id => {
                        let label = row.words.map_or(row.id, |w| w.label);
                        Some((words.text(label), mode.show_last))
                    }
                    _ => None,
                })
                .collect();
            groups.push(GroupShown {
                name: words.text(&format!("cursor-group-{}", group_name(group.group))),
                cursors,
                active,
                settings,
            });
        }
        Well {
            groups,
            chosen,
            disabled: (!shown).then(|| words.text("cursor-well-no-photo")),
        }
    }

    /// Draws it with `draw`, which hands the design system's well to the Toolbar.
    pub fn with<R>(&self, draw: impl FnOnce(CursorWell<'_>) -> R) -> R {
        let cursors: Vec<Vec<WellCursor<'_>>> = (self.groups.iter())
            .map(|group| {
                (group.cursors.iter())
                    .map(|c| {
                        let mut cursor = WellCursor::new(c.icon, &c.name);
                        if let Some(key) = &c.key {
                            cursor = cursor.key(key);
                        }
                        if let Some(how) = &c.how {
                            cursor = cursor.how(how);
                        }
                        if let Some(why) = &c.why {
                            cursor = cursor.disabled(why);
                        }
                        cursor
                    })
                    .collect()
            })
            .collect();
        let settings: Vec<Vec<WellSetting<'_>>> = (self.groups.iter())
            .map(|group| {
                (group.settings.iter())
                    .map(|(name, on)| WellSetting::Toggle { name, on: *on })
                    .collect()
            })
            .collect();
        let groups: Vec<WellGroup<'_>> = (self.groups.iter())
            .zip(&cursors)
            .zip(&settings)
            .map(|((group, cursors), settings)| {
                WellGroup::new(&group.name, cursors, group.active).settings(settings)
            })
            .collect();
        let mut well = CursorWell::new(&groups);
        if let Some(chosen) = self.chosen {
            well = well.chosen(chosen);
        }
        if let Some(why) = &self.disabled {
            well = well.disabled(why);
        }
        draw(well)
    }

    /// What the well's `action` asks for.
    pub fn answer(&self, action: CursorWellAction) -> Option<Answer> {
        match action {
            CursorWellAction::Cursor { group, cursor } => {
                let shown = self.groups.get(group)?.cursors.get(cursor)?;
                Some(Answer::Choose(shown.cursor))
            }
            // Pick's one setting, Show the last pick.
            CursorWellAction::Toggle { on, .. } => Some(Answer::ShowLast(on)),
            CursorWellAction::Amounts { .. } | CursorWellAction::Choice { .. } => None,
        }
    }
}

/// What a press in the well asks of the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The cursor's action, which chooses it.
    Choose(Cursor),
    /// Pick's Show the last pick turned on or off.
    ShowLast(bool),
}

/// A cursor's name and what a press with it does: the app's `cursor-<id>` and
/// `cursor-<id>-how`; a tool's `tool-<tool>-cursor-<cursor>` and its `-how`, or, where its
/// picker's words follow a setting, the picker's promise for it (decision 88).
fn words_of(entry: &Entry, cx: &Context) -> (String, Option<String>) {
    let words = cx.words;
    let (Owner::Tool(tool), Some(picker)) = (entry.cursor.owner, entry.picker) else {
        let id = format!("cursor-{}", entry.cursor.id);
        return (words.text(&id), Some(words.text(&format!("{id}-how"))));
    };
    let name = words.text(&apertrust_tools::messages::cursor(tool, entry.cursor.id));
    let found = (apertrust_tools::TOOLS.iter()).find(|t| t.definition().tool.id == tool);
    let definition = found.map(|t| t.definition());
    let declared = definition.and_then(|d| Some((d, d.picker(picker)?)));
    let follows = declared.and_then(|(definition, declared)| {
        pickers::by_setting(
            words,
            definition,
            declared,
            cx.settings,
            cx.photo,
            "promise",
        )
    });
    let how = follows.unwrap_or_else(|| {
        words.text(&apertrust_tools::messages::cursor_how(
            tool,
            entry.cursor.id,
        ))
    });
    (name, Some(how))
}

/// The design system's icon of `name`, a cursor's `CursorDef::icon`.
fn icon(name: &str) -> Icon {
    (Icon::ALL.iter().copied())
        .find(|icon| icon.name() == name)
        .unwrap_or(Icon::Pick)
}

/// A group's part of its words' ids: `cursor-group-<group>`.
fn group_name(group: CursorGroup) -> &'static str {
    match group {
        CursorGroup::Select => "select",
        CursorGroup::Pan => "pan",
        CursorGroup::Pick => "pick",
        CursorGroup::Mask => "mask",
        CursorGroup::Retouch => "retouch",
        CursorGroup::Annotate => "annotate",
        CursorGroup::CopyApply => "copy-apply",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apertrust_engine::session::ControlValue;

    /// The built-in registry: Select, Pan, then Pick with White balance's cursor before
    /// Exposure's, Pick holding Show the last pick.
    #[test]
    fn the_registry_holds_phase_2_s_groups_in_the_well_s_order() {
        let groups: Vec<(CursorGroup, Vec<Cursor>)> = (registry().groups().iter())
            .map(|g| (g.group, g.cursors.iter().map(|e| e.cursor).collect()))
            .collect();
        let tool = |tool, id| Cursor {
            owner: Owner::Tool(tool),
            id,
        };
        assert_eq!(
            groups,
            vec![
                (CursorGroup::Select, vec![Cursor::SELECT]),
                (CursorGroup::Pan, vec![Cursor::PAN]),
                (
                    CursorGroup::Pick,
                    vec![tool("temperature", "spot"), tool("exposure", "exposure")]
                ),
            ]
        );
        let pick = &registry().groups()[2];
        assert!(matches!(pick.settings, [Row::Setting(row)] if row.id == "show_last"));
    }

    /// A cursor declared twice by one owner, a key on two cursors and a group's settings
    /// declared twice are refused.
    #[test]
    fn the_registry_refuses_what_does_not_hold_together() {
        struct Twice;
        impl CursorOwner for Twice {
            fn cursors(&self) -> &'static [CursorDef] {
                &[SELECT]
            }
            fn ready(&self, _: &str, _: &Context) -> Result<(), String> {
                Ok(())
            }
        }
        let refused = Registry::new(&[&AppCursors, &Twice], &[]).err();
        assert_eq!(refused, Some(RegistryError::Duplicate(Cursor::SELECT)));
        struct OnK;
        impl CursorOwner for OnK {
            fn cursors(&self) -> &'static [CursorDef] {
                const K: CursorDef = CursorDef {
                    id: "other",
                    key: Some("K"),
                    ..PAN
                };
                &[K]
            }
            fn ready(&self, _: &str, _: &Context) -> Result<(), String> {
                Ok(())
            }
        }
        let refused = Registry::new(&[&OnK], apertrust_tools::TOOLS).err();
        assert_eq!(refused, Some(RegistryError::Key("K")));
        struct Settings;
        impl CursorOwner for Settings {
            fn cursors(&self) -> &'static [CursorDef] {
                &[]
            }
            fn groups(&self) -> &'static [CursorGroupDef] {
                AppCursors.groups()
            }
            fn ready(&self, _: &str, _: &Context) -> Result<(), String> {
                Ok(())
            }
        }
        let refused = Registry::new(&[&AppCursors, &Settings], &[]).err();
        assert_eq!(refused, Some(RegistryError::Settings(CursorGroup::Pick)));
    }

    /// Select at start, each group's first active; a cursor chosen becomes its group's
    /// active one and stays.
    #[test]
    fn the_mode_starts_at_select_and_keeps_each_group_s_active_cursor() {
        let mut mode = Mode::new(true);
        assert_eq!(mode.chosen(), Cursor::SELECT);
        let spot = Cursor {
            owner: Owner::Tool("temperature"),
            id: "spot",
        };
        let exposure = Cursor {
            owner: Owner::Tool("exposure"),
            id: "exposure",
        };
        assert_eq!(mode.active(CursorGroup::Pick), Some(spot));
        mode.choose(exposure);
        assert_eq!(mode.chosen(), exposure);
        assert_eq!(mode.active(CursorGroup::Pick), Some(exposure));
        mode.choose(Cursor::SELECT);
        assert_eq!(mode.active(CursorGroup::Pick), Some(exposure));
        mode.choose(exposure);
        // A cursor no owner declares changes nothing.
        mode.choose(Cursor {
            owner: Owner::App,
            id: "brush",
        });
        assert_eq!(mode.chosen(), exposure);
    }

    /// A picker cursor cannot act, saying why, while the photo takes no pick, while its
    /// tool is hidden by the History tab's eye, and, for Exposure's, toward a reference with
    /// none set; Select and Pan always can.
    #[test]
    fn a_picker_cursor_says_why_it_cannot_act() {
        let words = Translator::new(apertrust_ui::Locale::EnGb);
        let settings = ToolSettings::new();
        let cx = |why, hidden, settings| Context {
            why,
            hidden,
            settings,
            photo: None,
            words: &words,
        };
        let entry = |id| *registry().entries().find(|e| e.cursor.id == id).unwrap();
        let (spot, exposure) = (entry("spot"), entry("exposure"));
        assert_eq!(ready(&spot, &cx(None, None, &settings)), Ok(()));
        assert_eq!(
            ready(&spot, &cx(Some("Opening IMG_2041…"), None, &settings)),
            Err("Opening IMG_2041…".to_owned())
        );
        assert_eq!(
            ready(&spot, &cx(None, Some("temperature"), &settings)),
            Err("White balance hidden".to_owned())
        );
        assert_eq!(
            ready(&exposure, &cx(None, Some("temperature"), &settings)),
            Ok(())
        );
        let mut reference = ToolSettings::new();
        let target = apertrust_tools::exposure::PickerTarget::Reference;
        let at =
            <apertrust_tools::exposure::PickerTarget as apertrust_base::settings::Choice>::OPTIONS
                .iter()
                .position(|(option, _)| *option == target)
                .unwrap();
        reference.set("exposure", "target", ControlValue::Choice(at as u32));
        let why = ready(&exposure, &cx(None, None, &reference)).unwrap_err();
        assert_eq!(
            why,
            "Set a reference first, with the picker on the Reference row"
        );
        for app in [Cursor::SELECT, Cursor::PAN] {
            let entry = registry().entry(app).unwrap();
            assert_eq!(ready(entry, &cx(Some("Opening"), None, &settings)), Ok(()));
        }
    }
}
