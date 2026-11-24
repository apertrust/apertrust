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
//!   settings.
//! - **The mode** ([`Mode`]): the cursor chosen, Select at every start and never saved;
//!   each group's active cursor, the one its face shows, kept for the run; A held, which
//!   borrows Pan while it is down. A picker cursor is its picker's second door: choosing
//!   it asks for its picker on ([`Mode::asked_picker`]), which the app sends as the picker
//!   button's `On` does. The mode follows the picker on ([`Mode::follow`]): a picker with a
//!   cursor, whichever door turned it on, chooses that cursor, and its turning off chooses
//!   Select, or keeps the cursor the person chose; screen readers hear it turn on and off,
//!   with the reason where the person did not ask ([`turned_words`]).
//! - **The well's words** ([`Well`]): each group's cursors with their names, keys, what
//!   they do and why one cannot act now, which the Toolbar draws as `CursorWell`.
//!
//! On the photo, Select and Pan pan as the built drag does (`docs/gui.md`, Panning), which
//! the Viewer keeps, and a picker on takes every press, from either door (`pickers.rs`).
//! The engine never hears of cursors: a cursor sends commands its owner already sends.

use std::collections::{BTreeMap, BTreeSet};

use apertrust_engine::session::{Command, ControlValue, PickerRef, Values};
use apertrust_tools::{
    CursorDef, CursorGroup, CursorGroupDef, Gesture, ProcessingTool, Row, Surface,
};
use apertrust_ui::components::{CursorWell, CursorWellAction, WellCursor, WellGroup};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Icon, Translator};
use eframe::egui::{self, Pos2, Rect, Sense, Vec2};

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

/// What a cursor's readiness reads of the app this frame.
pub struct Context<'a> {
    /// Why no pick can be made on the photo now, in the interface's words: the photo
    /// still opening, or open for viewing only; None where picks are taken.
    pub why: Option<&'a str>,
    /// The tool the History tab's eye hides while it is held, whose picks wait for it.
    pub hidden: Option<&'static str>,
    /// The tools' settings as the app keeps them, which a picker's readiness reads.
    pub settings: &'a ToolSettings,
    /// The photo open, which a reference measured on it names as this photo.
    pub photo: Option<&'a std::path::Path>,
    pub words: &'a Translator,
}

/// What every owner of cursors but a tool implements (`docs/cursors.md`, The API); a tool's
/// picker cursors share one built-in owner ([`ready`], [`Mode::asked_picker`]).
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
/// (`docs/gui.md`, Panning).
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

impl CursorOwner for AppCursors {
    fn cursors(&self) -> &'static [CursorDef] {
        &[SELECT, PAN]
    }

    fn ready(&self, _cursor: &str, _cx: &Context) -> Result<(), String> {
        Ok(())
    }
}

/// One cursor of the registry: whose it is, its declaration, and for a tool's, the picker
/// it turns on.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub cursor: Cursor,
    pub def: &'static CursorDef,
    pub picker: Option<&'static str>,
}

/// One group of the well and its cursors in their order. Its settings join it with the
/// first group that has some, Copy and apply's.
#[derive(Debug)]
pub struct Group {
    pub group: CursorGroup,
    pub cursors: Vec<Entry>,
}

/// Every owner's cursors, by group in the well's order, then by `order`; a group no owner
/// fills is not there. The owners' group settings are checked, none declared twice.
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
        let mut settings = BTreeSet::new();
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
                if !settings.insert(declared.group) {
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
    /// Whether A is held, which borrows Pan while it is down.
    pan_held: bool,
    /// The picker cursor the person chose since [`Mode::asked_picker`] last looked, whose
    /// picker the app turns on: the cursor itself is chosen once the picker is on.
    turn_on: Option<Cursor>,
    /// The picker on as the well last followed it.
    following: Option<Following>,
    /// Why the picker on is to turn off, as the app knows from each command it sent that
    /// may end it, by that command's number: each until the session's answer to it shows
    /// the picker still on ([`Mode::answered`]). The turn says the reason of the command
    /// whose answer shows it ([`Mode::follow`]).
    ending: BTreeMap<u64, Ended>,
    /// Whether the person chose a cursor since [`Mode::ends`] last looked.
    chose: bool,
    /// What screen readers last heard of a picker turning on or off.
    turned: String,
}

/// A picker on as the well follows it: its cursor, if it has one, and the open of the
/// photo it was turned on for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Following {
    pub picker: PickerRef,
    pub cursor: Option<Cursor>,
    pub open: Option<u64>,
}

/// A picker turning on or off, as the well follows it ([`Mode::follow`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turned {
    /// The picker on now.
    pub on: Option<PickerRef>,
    /// The picker on before, and why it turned off, where the app knows.
    pub off: Option<(Following, Option<Ended>)>,
}

/// Why a picker turned off, as the app knows it from what it sent (`docs/cursors.md`,
/// Keyboard and screen readers). Another photo, a photo closed and a tool turned off are
/// read from the mirror as it turns off ([`turned_words`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The person asked: Esc, another cursor, its picker button or its face pressed again.
    Asked,
    /// One of its tool's controls changed.
    Control { tool: String, control: String },
    /// A tool changed otherwise: its own reset or a preset, or another tool's control.
    Tool(String),
    /// Its tool's section closed in the Tools panel.
    Closed(String),
    /// Undo, Redo, a move through the history or a rewrite of it.
    History,
    /// The reference it picks toward cleared, as Exposure's Clear Reference does.
    ReferenceCleared,
    /// The whole edit reset.
    Reset,
}

/// Why `command`, sent while a picker of `tool` is on, turns it off in the session, where
/// it does: a change of `tool`, a move through the history or the edit's reset.
pub fn ending_of(command: &Command, tool: &str) -> Option<Ended> {
    match command {
        Command::SetControl {
            tool: t, control, ..
        }
        | Command::ResetControl { tool: t, control }
        | Command::Keep {
            tool: t, control, ..
        } if t == tool => Some(Ended::Control {
            tool: t.clone(),
            control: control.clone(),
        }),
        Command::ResetTool { tool: t } | Command::ApplyPreset { tool: t, .. } if t == tool => {
            Some(Ended::Tool(t.clone()))
        }
        Command::Undo
        | Command::Redo
        | Command::GoTo { .. }
        | Command::Compress { .. }
        | Command::Truncate { .. }
        | Command::DiscardHistory => Some(Ended::History),
        Command::Reset => Some(Ended::Reset),
        Command::Step { commands, .. } => commands.iter().find_map(|c| ending_of(c, tool)),
        _ => None,
    }
}

/// Why `change` of the Tools panel's, made while `active` is on, turns a tool's picker off
/// before it is made (`docs/pickers.md`, Turning off): Apertrust has no focused tool, so
/// darktable's module losing focus is another tool's control changing, or the picker's own
/// tool's section closing, `open` saying whether a tool's section is open as it is
/// toggled. The Colour picker's stays on as built.
pub fn ended_by(
    change: &crate::tools::Change,
    active: Option<PickerRef>,
    open: impl Fn(&str) -> bool,
) -> Option<Ended> {
    let Some(PickerRef::Tool { tool: on, .. }) = active else {
        return None;
    };
    match change {
        crate::tools::Change::Toggle { tool } if *tool == on && open(tool) => {
            Some(Ended::Closed(on.to_owned()))
        }
        change => changed_by(change)
            .filter(|tool| *tool != on)
            .map(|tool| Ended::Tool(tool.to_owned())),
    }
}

/// Why `settings`, sent for `tool`'s pickers while `active` is on, turn it off in the
/// session, where they do: the picker on no longer ready with them, as Exposure's toward a
/// reference that Clear Reference clears (`docs/pickers.md`, Declaring pickers).
pub fn cleared_by(
    tools: &[&dyn ProcessingTool],
    tool: &str,
    settings: &[(&str, ControlValue)],
    active: Option<PickerRef>,
) -> Option<Ended> {
    let Some(PickerRef::Tool { tool: on, picker }) = active else {
        return None;
    };
    let found = tools.iter().find(|t| t.definition().tool.id == on)?;
    (on == tool && !found.picker_ready(picker, settings)).then_some(Ended::ReferenceCleared)
}

/// The tool whose control `change` changes.
fn changed_by(change: &crate::tools::Change) -> Option<&'static str> {
    use crate::tools::Change;
    match change {
        Change::Set { tool, .. }
        | Change::Reset { tool, .. }
        | Change::Keep { tool, .. }
        | Change::ResetTool { tool }
        | Change::Action { tool, .. }
        | Change::ApplyPreset { tool, .. } => Some(tool),
        Change::Step(changes) => changes.iter().find_map(changed_by),
        _ => None,
    }
}

/// The cursor of `picker`, a tool's picker with one in the registry.
pub fn cursor_of(picker: PickerRef) -> Option<Cursor> {
    let PickerRef::Tool { tool, picker } = picker else {
        return None;
    };
    (registry().entries())
        .find(|e| e.cursor.owner == Owner::Tool(tool) && e.picker == Some(picker))
        .map(|e| e.cursor)
}

impl Mode {
    /// Select chosen, each group's first cursor active.
    pub fn new() -> Mode {
        let active = (registry().groups())
            .iter()
            .map(|g| (g.group, g.cursors[0].cursor))
            .collect();
        Mode {
            chosen: Cursor::SELECT,
            active,
            pan_held: false,
            turn_on: None,
            following: None,
            ending: BTreeMap::new(),
            chose: false,
            turned: String::new(),
        }
    }

    /// The cursor chosen.
    pub fn chosen(&self) -> Cursor {
        self.chosen
    }

    /// The cursor a press on the photo acts with now: Pan while A is held, else the one
    /// chosen.
    pub fn acting(&self) -> Cursor {
        if self.pan_held {
            Cursor::PAN
        } else {
            self.chosen
        }
    }

    /// `group`'s active cursor.
    pub fn active(&self, group: CursorGroup) -> Option<Cursor> {
        self.active.get(&group).copied()
    }

    /// Chooses `cursor`, which becomes its group's active one. A picker cursor is chosen
    /// exactly while its picker is on, so choosing one asks for its picker instead
    /// ([`Mode::asked_picker`]), and the mode chooses it as it follows the picker on
    /// ([`Mode::follow`]).
    pub fn choose(&mut self, cursor: Cursor) {
        if registry().entry(cursor).is_some_and(|e| e.picker.is_some()) {
            self.turn_on = Some(cursor);
        } else if self.show(cursor) {
            self.turn_on = None;
            self.chose = true;
        }
    }

    /// The picker the person's choice of its cursor, since this was last asked, turns on,
    /// as its button's `On` does, unless it is on already, `active`: its cursor chosen again
    /// leaves it on (`docs/cursors.md`, Keys).
    pub fn asked_picker(&mut self, active: Option<PickerRef>) -> Option<PickerRef> {
        let cursor = self.turn_on.take()?;
        let picker = registry().entry(cursor)?.picker;
        let (Owner::Tool(tool), Some(picker)) = (cursor.owner, picker) else {
            return None;
        };
        let picker = PickerRef::Tool { tool, picker };
        (active != Some(picker)).then_some(picker)
    }

    /// Shows `cursor` chosen, the person's choice or the picker's; whether the registry has
    /// it.
    fn show(&mut self, cursor: Cursor) -> bool {
        let Some(entry) = registry().entry(cursor) else {
            return false;
        };
        self.chosen = cursor;
        self.active.insert(entry.def.group, cursor);
        true
    }

    /// Whether the person's choice of a cursor since this was last asked ends the tool's
    /// picker on, `active`: one other than its own cursor, Select by Esc among them
    /// (`docs/cursors.md`, A cursor). The Colour picker's stays on as built.
    pub fn ends(&mut self, active: Option<PickerRef>) -> bool {
        let chose = std::mem::take(&mut self.chose);
        chose
            && matches!(active, Some(PickerRef::Tool { .. }))
            && active.and_then(cursor_of) != Some(self.chosen)
    }

    /// Whether `cursor`'s face pressed again ends the picker on, `active`: its own picker's
    /// cursor, as a second press on its picker button does (`docs/cursors.md`, The well).
    pub fn again(&self, cursor: Cursor, active: Option<PickerRef>) -> bool {
        active.and_then(cursor_of) == Some(cursor)
    }

    /// The picker on is to turn off for `why` by the command numbered `number`, which
    /// screen readers hear as it does.
    pub fn ending(&mut self, number: u64, why: Ended) {
        self.ending.insert(number, why);
    }

    /// The session has answered the first `through` commands: the reasons noted for those
    /// of them that left the picker on, as a change to a keep-active picker's tool does,
    /// are dropped, so a later turn-off never says them.
    pub fn answered(&mut self, through: u64) {
        self.ending.retain(|number, _| *number > through);
    }

    /// Follows the picker on as the mirror shows it, `on`, on the photo opened by `open`
    /// (`docs/cursors.md`, A cursor): a picker with a cursor, whichever door turned it on,
    /// chooses that cursor; as it turns off its cursor still chosen goes back to Select,
    /// while a cursor the person chose stays. What turned, if anything did, with why the
    /// picker before turned off where the app knows: the reason noted for `through`, the
    /// command whose answer the mirror shows, the app's own or the session's report's, else
    /// the latest noted before it, as an `Off` the app sent before an `On`.
    pub fn follow(
        &mut self,
        on: Option<PickerRef>,
        open: Option<u64>,
        through: Option<u64>,
    ) -> Option<Turned> {
        if on == self.following.map(|f| f.picker) {
            return None;
        }
        let before = self.following.take();
        // What ends the picker before, never one turned on after it.
        let ending = std::mem::take(&mut self.ending);
        let ending = through.and_then(|through| ending.range(..=through).next_back());
        let ending = ending.map(|(_, why)| why.clone());
        let cursor = on.and_then(cursor_of);
        match cursor {
            Some(cursor) => {
                self.show(cursor);
            }
            None if before.is_some_and(|b| b.cursor == Some(self.chosen)) => {
                self.show(Cursor::SELECT);
            }
            None => {}
        }
        self.following = on.map(|picker| Following {
            picker,
            cursor,
            open,
        });
        Some(Turned {
            on,
            off: before.map(|before| (before, ending)),
        })
    }

    /// Whether a tool's picker is on, as the well last followed it: Esc then ends it
    /// wherever the focus is ([`crate::Shell::find_keys`]).
    pub fn picking(&self) -> bool {
        self.following
            .is_some_and(|f| matches!(f.picker, PickerRef::Tool { .. }))
    }

    /// What screen readers hear of a picker turning on or off: `text`, said politely.
    pub fn say_turned(&mut self, text: String) {
        self.turned = text;
    }

    /// What screen readers last heard of a picker turning on or off.
    pub fn turned(&self) -> &str {
        &self.turned
    }

    /// A held, or let go, this frame.
    pub fn hold_pan(&mut self, held: bool) {
        self.pan_held = held;
    }

    /// What takes a press on the photo this frame (`docs/cursors.md`, A cursor): Pan while
    /// A is held, over a picker too; a tool's picker while one is on, `picker`, from either
    /// door, as darktable's button has it; else the cursor chosen: Pan's drag panning as
    /// the built drag does, and Select's marks, which come with stored samples and masks,
    /// never a pan.
    pub fn press(&self, picker: bool) -> Press {
        let Some(entry) = registry().entry(self.acting()) else {
            return Press::Pan;
        };
        match entry.def.group {
            // A held pans over a picker too, darktable's force-pan.
            _ if self.pan_held => Press::Pan,
            _ if picker => Press::Picker,
            CursorGroup::Pan => Press::Pan,
            // A picker cursor is chosen only while its picker is on, which takes the press.
            _ => Press::Select,
        }
    }
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::new()
    }
}

/// What a press on the photo does this frame ([`Mode::press`]).
#[derive(Clone, Copy, Debug)]
pub enum Press {
    /// The tool's picker on takes it, from either door.
    Picker,
    /// Select's: a click or a drag on a mark, once marks come; a drag on empty photo does
    /// nothing, never a pan.
    Select,
    /// A drag pans a zoomed photo.
    Pan,
}

/// Whether `entry` can act now, or why not (`docs/cursors.md`, A cursor): the app's own
/// always; a tool's picker cursor while the photo takes picks, its tool is not hidden by
/// the History tab's eye, its picker is ready with the tool's settings, as Exposure's
/// waits for a reference, and a reference row's picker while `--conf` gives none of the
/// reference, which keeps the row's button disabled for the run (decision 43).
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
    let given = |row: &Row| match row {
        Row::Reference {
            picker: of,
            lightness,
            photo,
        } => {
            *of == picker
                && (cx.settings.overridden(tool, lightness) || cx.settings.overridden(tool, photo))
        }
        _ => false,
    };
    if found.is_some_and(|found| found.definition().rows().any(given)) {
        let mut args = Args::new();
        args.set("flag", apertrust_base::settings::CONF_FLAG);
        return Err(cx.words.text_with("settings-overridden", Some(&args)));
    }
    Ok(())
}

/// What screen readers hear of `turned` (`docs/cursors.md`, Keyboard and screen readers):
/// "White balance picker on" as a picker turns on, from either door; as a tool's turns off
/// with none on after it, where the person did not ask, why: another photo, or the photo
/// closed, against `open`, the photo open now; its tool turned off, as `values` show; else
/// what the app sent, "White balance picker off: Temperature changed", or no reason where
/// it knows none. Nothing where the person asked, so the next turn is said again.
pub fn turned_words(
    words: &Translator,
    tools: &[&dyn ProcessingTool],
    turned: &Turned,
    open: Option<u64>,
    values: &Values,
) -> String {
    let mut args = Args::new();
    // The one turning off, a tool's, where the person did not ask.
    let before = (turned.off.as_ref())
        .filter(|(_, why)| *why != Some(Ended::Asked))
        .and_then(|(before, why)| match before.picker {
            PickerRef::Tool { tool, .. } => {
                Some((before, why, tool, picker_name(words, tools, before.picker)?))
            }
            PickerRef::ColourPicker => None,
        });
    if let Some(on) = turned.on {
        let Some(name) = picker_name(words, tools, on) else {
            return String::new();
        };
        args.set("picker", name);
        // Another picker replacing it: both, in one announcement.
        if let Some((_, _, _, before)) = before {
            args.set("before", before);
            return words.text_with("picker-turned-replaced", Some(&args));
        }
        return words.text_with("picker-turned-on", Some(&args));
    }
    let Some((before, why, tool, name)) = before else {
        return String::new();
    };
    args.set("picker", name);
    let title = |tool: &str| words.text(&apertrust_tools::messages::title(tool));
    let with = |id: &str, name: &str, value: String| {
        let mut args = Args::new();
        args.set(name, value);
        words.text_with(id, Some(&args))
    };
    let off = (values.tools.iter()).any(|state| state.tool == tool && !state.enabled);
    let why = match why {
        _ if open != before.open && open.is_none() => Some(words.text("picker-off-closed")),
        _ if open != before.open => Some(words.text("picker-off-photo")),
        _ if off => Some(with("picker-off-tool-off", "tool", title(tool))),
        Some(Ended::Control { tool, control }) => {
            let label = words.text(&apertrust_tools::messages::label(tool, control));
            Some(with("picker-off-control", "control", label))
        }
        Some(Ended::Tool(tool)) => Some(with("picker-off-tool", "tool", title(tool))),
        Some(Ended::Closed(tool)) => Some(with("picker-off-section", "tool", title(tool))),
        Some(Ended::History) => Some(words.text("picker-off-history")),
        Some(Ended::Reset) => Some(words.text("picker-off-reset")),
        Some(Ended::ReferenceCleared) => Some(words.text("picker-off-reference")),
        Some(Ended::Asked) | None => None,
    };
    match why {
        Some(why) => {
            args.set("why", why);
            words.text_with("picker-turned-off-why", Some(&args))
        }
        None => words.text_with("picker-turned-off", Some(&args)),
    }
}

/// A tool's picker's name as screen readers hear it turn on and off: its cursor's,
/// "White balance picker", else its tool's title and its own, as the viewer bar gives
/// them; none for the Colour picker's, whose words come with its section.
fn picker_name(
    words: &Translator,
    tools: &[&dyn ProcessingTool],
    picker: PickerRef,
) -> Option<String> {
    let PickerRef::Tool { tool, picker: id } = picker else {
        return None;
    };
    if let Some(cursor) = cursor_of(picker) {
        return Some(words.text(&apertrust_tools::messages::cursor(tool, cursor.id)));
    }
    let found = tools.iter().find(|t| t.definition().tool.id == tool)?;
    let mut args = Args::new();
    args.set("tool", words.text(&apertrust_tools::messages::title(tool)));
    args.set("picker", pickers::name(words, found.definition(), id));
    Some(words.text_with("picker-bar-name", Some(&args)))
}

/// The polite live region at `at` that says a picker turning on and off, `text`: a status
/// that takes no room.
pub fn turned_region(ui: &mut egui::Ui, at: Pos2, text: &str) {
    let response = ui.interact(
        Rect::from_min_size(at, Vec2::ZERO),
        egui::Id::unique("apertrust picker turned"),
        Sense::hover(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Status, true, text));
    (ui.ctx()).accesskit_node_builder(response.id, |node| {
        node.set_live(egui::accesskit::Live::Polite);
    });
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
}

/// The well as this frame draws it (`docs/cursors.md`, The well): each group's cursors with
/// their words and keys, the active one and the one chosen. A group's settings are drawn
/// with the first group that has some, Copy and apply's.
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
            groups.push(GroupShown {
                name: words.text(&format!("cursor-group-{}", group_name(group.group))),
                cursors,
                active,
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
        let groups: Vec<WellGroup<'_>> = (self.groups.iter())
            .zip(&cursors)
            .map(|(group, cursors)| WellGroup::new(&group.name, cursors, group.active))
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
            CursorWellAction::Again { group } => {
                let shown = self.groups.get(group)?;
                Some(Answer::Again(shown.cursors.get(shown.active)?.cursor))
            }
            CursorWellAction::Amounts { .. }
            | CursorWellAction::Toggle { .. }
            | CursorWellAction::Choice { .. } => None,
        }
    }
}

/// What a press in the well asks of the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The cursor's action, which chooses it, or for a picker cursor turns its picker on.
    Choose(Cursor),
    /// The chosen cursor's face pressed again: a picker cursor's turns its picker off.
    Again(Cursor),
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
pub fn icon(name: &str) -> Icon {
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

    /// The built-in registry: Select, Pan, then Pick with White balance's cursor before
    /// Exposure's two, Colour calibration's and Colour balance RGB's four hue pickers', in
    /// its page's order.
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
                    vec![
                        tool("temperature", "spot"),
                        tool("exposure", "exposure"),
                        tool("exposure", "reference"),
                        tool("channelmixerrgb", "illuminant"),
                        tool("colorbalancergb", "global_hue"),
                        tool("colorbalancergb", "shadows_hue"),
                        tool("colorbalancergb", "highlights_hue"),
                        tool("colorbalancergb", "midtones_hue"),
                    ]
                ),
            ]
        );
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
                &[CursorGroupDef {
                    group: CursorGroup::Pick,
                    settings: &[],
                }]
            }
            fn ready(&self, _: &str, _: &Context) -> Result<(), String> {
                Ok(())
            }
        }
        let refused = Registry::new(&[&Settings, &Settings], &[]).err();
        assert_eq!(refused, Some(RegistryError::Settings(CursorGroup::Pick)));
    }

    /// Select at start, each group's first active; a cursor chosen becomes its group's
    /// active one and stays; A held borrows Pan and gives the cursor back on release.
    #[test]
    fn the_mode_starts_at_select_and_keeps_each_group_s_active_cursor() {
        let mut mode = Mode::new();
        assert_eq!(mode.chosen(), Cursor::SELECT);
        let exposure = Cursor {
            owner: Owner::Tool("exposure"),
            id: "exposure",
        };
        assert_eq!(mode.active(CursorGroup::Pick), Some(spot()));
        mode.follow(Some(EXPOSURE_PICKER), Some(1), None);
        assert_eq!(mode.chosen(), exposure);
        assert_eq!(mode.active(CursorGroup::Pick), Some(exposure));
        mode.choose(Cursor::PAN);
        assert_eq!(mode.active(CursorGroup::Pick), Some(exposure));
        mode.choose(Cursor::SELECT);
        mode.hold_pan(true);
        assert_eq!(
            (mode.chosen(), mode.acting()),
            (Cursor::SELECT, Cursor::PAN)
        );
        mode.hold_pan(false);
        assert_eq!(mode.acting(), Cursor::SELECT);
        // A cursor no owner declares changes nothing.
        mode.choose(Cursor {
            owner: Owner::App,
            id: "brush",
        });
        assert_eq!(mode.chosen(), Cursor::SELECT);
    }

    /// A picker cursor chosen, by its face, its list item or its key, asks for its picker
    /// on, as its button's `On` does, once, and is chosen only as the picker turns on; one
    /// whose picker is on already asks for nothing, which leaves it on; another cursor
    /// chosen after it in the same frame asks for none.
    #[test]
    fn a_picker_cursor_chosen_asks_for_its_picker_on() {
        let mut mode = Mode::new();
        mode.choose(spot());
        assert_eq!(
            mode.chosen(),
            Cursor::SELECT,
            "chosen as its picker turns on"
        );
        assert!(!mode.ends(None));
        assert_eq!(mode.asked_picker(None), Some(SPOT_PICKER));
        assert_eq!(mode.asked_picker(None), None, "asked once");
        mode.follow(Some(SPOT_PICKER), Some(1), None);
        assert_eq!(mode.chosen(), spot());
        // Chosen again while its picker is on: left on.
        mode.choose(spot());
        assert_eq!(mode.asked_picker(Some(SPOT_PICKER)), None);
        assert!(!mode.ends(Some(SPOT_PICKER)));
        // Another picker's cursor, from White balance's: that picker, its own `On` turning
        // White balance's off in the session, so nothing ends it first.
        let exposure = Cursor {
            owner: Owner::Tool("exposure"),
            id: "exposure",
        };
        mode.choose(exposure);
        assert!(!mode.ends(Some(SPOT_PICKER)));
        assert_eq!(mode.asked_picker(Some(SPOT_PICKER)), Some(EXPOSURE_PICKER));
        // Pan after it in the same frame: Pan, ending the picker, and no picker turned on.
        mode.choose(exposure);
        mode.choose(Cursor::PAN);
        assert_eq!(mode.asked_picker(Some(SPOT_PICKER)), None);
        assert!(mode.ends(Some(SPOT_PICKER)));
    }

    /// Who takes a press on the photo: A held pans, over a picker too; a picker on, from
    /// every cursor; Pan pans; Select never does.
    #[test]
    fn a_picker_on_takes_the_photo_then_the_cursor_acting_does() {
        let cases = [
            (Cursor::SELECT, false, false, "select"),
            (Cursor::SELECT, true, false, "pan"),
            (Cursor::PAN, false, false, "pan"),
            (Cursor::SELECT, false, true, "picker"),
            (spot(), false, true, "picker"),
            (spot(), true, true, "pan"),
            (Cursor::PAN, false, true, "picker"),
        ];
        for (chosen, held, picker, takes) in cases {
            let mut mode = Mode::new();
            mode.choose(chosen);
            if chosen == spot() {
                mode.follow(Some(SPOT_PICKER), Some(1), None);
            }
            assert_eq!(mode.chosen(), chosen);
            mode.hold_pan(held);
            let found = match mode.press(picker) {
                Press::Picker => "picker",
                Press::Select => "select",
                Press::Pan => "pan",
            };
            assert_eq!(found, takes, "{chosen:?}, A held {held}, picker {picker}");
        }
    }

    /// A picker cursor cannot act, saying why, while the photo takes no pick, while its
    /// tool is hidden by the History tab's eye, for Exposure's, toward a reference with
    /// none set, and for its reference's, while `--conf` gives the reference; Select and
    /// Pan always can.
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
        // The reference's own cursor can always turn its picker on, but where `--conf`
        // gives the reference, which keeps its row's button disabled for the run.
        let reference_cursor = entry("reference");
        assert_eq!(
            ready(&reference_cursor, &cx(None, None, &reference)),
            Ok(())
        );
        let given = vec!["tools.exposure.reference_lightness=60".to_owned()];
        let overrides = crate::settings::overrides(&given).expect("read");
        let folder = crate::settings::testing::Folder::new("reference-conf-cursor");
        let (_, kept) = crate::settings::Keeper::open_with(folder.storage(), &overrides);
        assert_eq!(
            ready(&reference_cursor, &cx(None, None, &kept.tool_settings)),
            Err("Set by --conf for this run".to_owned())
        );
        assert_eq!(
            ready(&exposure, &cx(None, None, &kept.tool_settings)),
            Ok(())
        );
        for app in [Cursor::SELECT, Cursor::PAN] {
            let entry = registry().entry(app).unwrap();
            assert_eq!(ready(entry, &cx(Some("Opening"), None, &settings)), Ok(()));
        }
    }

    const SPOT_PICKER: PickerRef = PickerRef::Tool {
        tool: "temperature",
        picker: "spot",
    };
    const EXPOSURE_PICKER: PickerRef = PickerRef::Tool {
        tool: "exposure",
        picker: "exposure",
    };
    const REFERENCE_PICKER: PickerRef = PickerRef::Tool {
        tool: "exposure",
        picker: "reference",
    };

    fn spot() -> Cursor {
        Cursor {
            owner: Owner::Tool("temperature"),
            id: "spot",
        }
    }

    /// The well follows the picker on, whichever door turned it on: its cursor chosen while
    /// it is on, Select as it turns off; another picker's cursor as that one turns on, Select
    /// for one with no cursor; a cursor the person chose stays as the picker turns off, and
    /// it ends the picker, but its own cursor chosen again does not, nor the Colour picker's.
    #[test]
    fn the_mode_follows_the_picker_on() {
        let mut mode = Mode::new();
        mode.choose(Cursor::PAN);
        assert!(!mode.ends(None), "no picker on");
        let turned = mode.follow(Some(SPOT_PICKER), Some(1), None);
        assert_eq!(
            turned,
            Some(Turned {
                on: Some(SPOT_PICKER),
                off: None
            })
        );
        assert_eq!(mode.chosen(), spot());
        assert_eq!(mode.active(CursorGroup::Pick), Some(spot()));
        assert!(mode.picking());
        assert_eq!(
            mode.follow(Some(SPOT_PICKER), Some(1), None),
            None,
            "no turn"
        );
        // Its own key again leaves it on; its face again ends it.
        mode.choose(spot());
        assert!(!mode.ends(Some(SPOT_PICKER)));
        assert!(mode.again(spot(), Some(SPOT_PICKER)));
        assert!(!mode.again(Cursor::PAN, Some(SPOT_PICKER)));
        mode.ending(9, Ended::Asked);
        let turned = mode.follow(None, Some(1), Some(9)).unwrap();
        assert_eq!(
            turned.off.map(|(f, why)| (f.picker, why)),
            Some((SPOT_PICKER, Some(Ended::Asked)))
        );
        assert_eq!(mode.chosen(), Cursor::SELECT);
        assert!(!mode.picking());
        // Another picker's cursor, then one with none: Select.
        mode.follow(Some(SPOT_PICKER), Some(1), None);
        mode.follow(Some(EXPOSURE_PICKER), Some(1), None);
        let exposure = Cursor {
            owner: Owner::Tool("exposure"),
            id: "exposure",
        };
        assert_eq!(mode.chosen(), exposure);
        mode.follow(Some(REFERENCE_PICKER), Some(1), None);
        assert_eq!(mode.chosen().id, "reference");
        let agx = PickerRef::Tool {
            tool: "agx",
            picker: "white",
        };
        mode.follow(Some(agx), Some(1), None);
        assert_eq!(mode.chosen(), Cursor::SELECT);
        // Pan chosen by the person ends it, and stays as it turns off.
        mode.follow(Some(SPOT_PICKER), Some(1), None);
        mode.choose(Cursor::PAN);
        assert!(mode.ends(Some(SPOT_PICKER)));
        assert!(!mode.ends(Some(SPOT_PICKER)), "asked once");
        mode.follow(None, Some(1), None);
        assert_eq!(mode.chosen(), Cursor::PAN);
        // The Colour picker's stays on as built.
        mode.follow(Some(PickerRef::ColourPicker), Some(1), None);
        mode.choose(Cursor::SELECT);
        assert!(!mode.ends(Some(PickerRef::ColourPicker)));
        assert!(!mode.picking());
    }

    /// What screen readers hear as a picker turns on and off: its name and on; off with
    /// why where the person did not ask, another photo and the photo closed read from the
    /// open, its tool turned off from the values, the rest from what the app sent; nothing
    /// where the person asked.
    #[test]
    fn a_picker_turning_on_and_off_is_said_with_why() {
        let words = Translator::new(apertrust_ui::Locale::EnGb);
        let tools = apertrust_tools::TOOLS;
        let mut values = crate::mirror::tests::mirror().values;
        let on = Turned {
            on: Some(SPOT_PICKER),
            off: None,
        };
        assert_eq!(
            turned_words(&words, tools, &on, Some(1), &values),
            "White balance picker on"
        );
        let reference = Turned {
            on: Some(REFERENCE_PICKER),
            off: None,
        };
        assert_eq!(
            turned_words(&words, tools, &reference, Some(1), &values),
            "Exposure reference picker on"
        );
        // A picker with no cursor: its tool's title and its own name.
        let agx = Turned {
            on: Some(PickerRef::Tool {
                tool: "agx",
                picker: "white",
            }),
            off: None,
        };
        let said = turned_words(&words, tools, &agx, Some(1), &values);
        assert!(said.starts_with("AgX · "), "{said}");
        let off = |why: Option<Ended>| Turned {
            on: None,
            off: Some((
                Following {
                    picker: SPOT_PICKER,
                    cursor: Some(spot()),
                    open: Some(1),
                },
                why,
            )),
        };
        let said =
            |why, open, values: &Values| turned_words(&words, tools, &off(why), open, values);
        let control = Ended::Control {
            tool: "temperature".to_owned(),
            control: "temperature".to_owned(),
        };
        let cases = [
            (Some(Ended::Asked), Some(1), ""),
            (None, Some(1), "White balance picker off"),
            (None, Some(2), "White balance picker off: another photo"),
            (
                Some(control.clone()),
                None,
                "White balance picker off: photo closed",
            ),
            (
                Some(control),
                Some(1),
                "White balance picker off: Temperature changed",
            ),
            (
                Some(Ended::Tool("exposure".to_owned())),
                Some(1),
                "White balance picker off: Exposure changed",
            ),
            (
                Some(Ended::Closed("temperature".to_owned())),
                Some(1),
                "White balance picker off: White balance closed",
            ),
            (
                Some(Ended::History),
                Some(1),
                "White balance picker off: history changed",
            ),
            (
                Some(Ended::Reset),
                Some(1),
                "White balance picker off: edit reset",
            ),
        ];
        for (why, open, text) in cases {
            assert_eq!(said(why.clone(), open, &values), text, "{why:?}, {open:?}");
        }
        for state in &mut values.tools {
            state.enabled &= state.tool != "temperature";
        }
        assert_eq!(
            said(Some(Ended::History), Some(1), &values),
            "White balance picker off: White balance turned off"
        );
        // Another picker replacing it: both said in one announcement; where the person
        // ended it, the new one alone.
        let both = |why| Turned {
            on: Some(EXPOSURE_PICKER),
            ..off(why)
        };
        assert_eq!(
            turned_words(&words, tools, &both(None), Some(1), &values),
            "White balance picker off: another picker turned on. Exposure picker on"
        );
        assert_eq!(
            turned_words(&words, tools, &both(Some(Ended::Asked)), Some(1), &values),
            "Exposure picker on"
        );
    }

    /// A reason noted for a command that left the picker on, as a change to a keep-active
    /// picker's tool does, is dropped once the session answers it, so a later turn-off says
    /// no reason of its; one noted for a later command stays until it is answered, and a
    /// picker turned on starts with none.
    #[test]
    fn a_reason_lasts_only_until_its_command_is_answered() {
        let mut mode = Mode::new();
        mode.follow(Some(SPOT_PICKER), Some(1), None);
        mode.ending(4, Ended::History);
        mode.answered(3);
        assert_eq!(mode.follow(Some(SPOT_PICKER), Some(1), None), None);
        mode.answered(4);
        let turned = mode.follow(None, Some(1), None).unwrap();
        assert_eq!(turned.off.map(|(_, why)| why), Some(None));
        mode.ending(7, Ended::Reset);
        mode.answered(6);
        let turned = mode.follow(Some(SPOT_PICKER), Some(1), None).unwrap();
        assert_eq!(turned.off, None);
        let turned = mode.follow(None, Some(1), None).unwrap();
        assert_eq!(
            turned.off.map(|(_, why)| why),
            Some(None),
            "not the earlier one's"
        );
    }

    /// Two commands that may end the picker sent before the session's report of the first:
    /// each keeps its reason, and the turn-off says that of the command the report is
    /// through, at which the session ended it; the reasons go with the turn, so the next
    /// picker starts with none.
    #[test]
    fn two_commands_ending_a_picker_say_the_first() {
        let mut mode = Mode::new();
        mode.follow(Some(EXPOSURE_PICKER), Some(1), None);
        let control = Ended::Control {
            tool: "exposure".to_owned(),
            control: "black".to_owned(),
        };
        mode.ending(4, control.clone());
        mode.ending(5, Ended::History);
        mode.answered(3);
        let turned = mode.follow(None, Some(1), Some(4)).unwrap();
        assert_eq!(turned.off.map(|(_, why)| why), Some(Some(control)));
        mode.follow(Some(EXPOSURE_PICKER), Some(1), Some(6));
        let turned = mode.follow(None, Some(1), Some(7)).unwrap();
        assert_eq!(
            turned.off.map(|(_, why)| why),
            Some(None),
            "not the second's"
        );
    }

    /// A keep-active picker's own control changed, then Undo, in one round trip: the change
    /// leaves it on and Undo ends it, so the turn-off says Undo's reason, the command the
    /// report is through, not the control's before it.
    #[test]
    fn a_keep_active_picker_ended_by_undo_after_its_control_says_undo() {
        let mut mode = Mode::new();
        mode.follow(Some(SPOT_PICKER), Some(1), Some(2));
        let control = Ended::Control {
            tool: "temperature".to_owned(),
            control: "temperature".to_owned(),
        };
        mode.ending(3, control);
        mode.ending(4, Ended::History);
        mode.answered(2);
        let turned = mode.follow(None, Some(1), Some(4)).unwrap();
        assert_eq!(turned.off.map(|(_, why)| why), Some(Some(Ended::History)));
    }

    /// Clear Reference while Exposure picks toward the reference turns its picker off in the
    /// session, said with why; never the reference's own picker, one picking toward
    /// another target, nor another tool's.
    #[test]
    fn clearing_the_reference_ends_a_picker_toward_it() {
        let tools = apertrust_tools::TOOLS;
        let target = apertrust_tools::exposure::PickerTarget::Reference;
        let at =
            <apertrust_tools::exposure::PickerTarget as apertrust_base::settings::Choice>::OPTIONS
                .iter()
                .position(|(option, _)| *option == target)
                .unwrap();
        let cleared = [("target", ControlValue::Choice(at as u32))];
        assert_eq!(
            cleared_by(tools, "exposure", &cleared, Some(EXPOSURE_PICKER)),
            Some(Ended::ReferenceCleared)
        );
        assert_eq!(
            cleared_by(tools, "exposure", &cleared, Some(REFERENCE_PICKER)),
            None
        );
        assert_eq!(
            cleared_by(tools, "exposure", &[], Some(EXPOSURE_PICKER)),
            None
        );
        assert_eq!(
            cleared_by(tools, "exposure", &cleared, Some(SPOT_PICKER)),
            None
        );
        assert_eq!(cleared_by(tools, "exposure", &cleared, None), None);
        let words = Translator::new(apertrust_ui::Locale::EnGb);
        let values = crate::mirror::tests::mirror().values;
        let off = Turned {
            on: None,
            off: Some((
                Following {
                    picker: EXPOSURE_PICKER,
                    cursor: cursor_of(EXPOSURE_PICKER),
                    open: Some(1),
                },
                Some(Ended::ReferenceCleared),
            )),
        };
        assert_eq!(
            turned_words(&words, tools, &off, Some(1), &values),
            "Exposure picker off: reference cleared"
        );
    }

    /// Why a command turns a tool's picker off in the session, and why a change of the
    /// Tools panel's turns it off in the app: another tool's control, its own section
    /// closed; never its own tool's change, which the session ends, nor the Colour picker's.
    #[test]
    fn what_ends_a_picker_is_named() {
        let set = |tool: &str| Command::SetControl {
            tool: tool.to_owned(),
            control: "exposure".to_owned(),
            value: ControlValue::Number(1.0),
            done: true,
        };
        assert_eq!(
            ending_of(&set("exposure"), "exposure"),
            Some(Ended::Control {
                tool: "exposure".to_owned(),
                control: "exposure".to_owned()
            })
        );
        assert_eq!(ending_of(&set("temperature"), "exposure"), None);
        assert_eq!(ending_of(&Command::Undo, "exposure"), Some(Ended::History));
        assert_eq!(ending_of(&Command::Reset, "exposure"), Some(Ended::Reset));
        let step = Command::Step {
            source: apertrust_engine::history::StepSource::You,
            note: None,
            commands: vec![set("exposure")],
        };
        assert!(matches!(
            ending_of(&step, "exposure"),
            Some(Ended::Control { .. })
        ));

        use crate::tools::Change;
        let change = |tool| Change::Reset {
            tool,
            control: "exposure",
        };
        let open = |_: &str| true;
        assert_eq!(
            ended_by(&change("exposure"), Some(SPOT_PICKER), open),
            Some(Ended::Tool("exposure".to_owned()))
        );
        assert_eq!(
            ended_by(&change("temperature"), Some(SPOT_PICKER), open),
            None
        );
        assert_eq!(
            ended_by(&change("exposure"), Some(PickerRef::ColourPicker), open),
            None
        );
        assert_eq!(ended_by(&change("exposure"), None, open), None);
        let toggle = |tool| Change::Toggle { tool };
        assert_eq!(
            ended_by(&toggle("temperature"), Some(SPOT_PICKER), open),
            Some(Ended::Closed("temperature".to_owned()))
        );
        assert_eq!(
            ended_by(&toggle("temperature"), Some(SPOT_PICKER), |_| false),
            None
        );
        assert_eq!(ended_by(&toggle("exposure"), Some(SPOT_PICKER), open), None);
    }
}
