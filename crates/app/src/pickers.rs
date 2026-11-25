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

//! The active colour picker in the app (`docs/pickers.md`, decision 79): what the mirror
//! keeps of it, its marks over the photo and the commands the pointer and the keys send
//! there, the viewer bar's words, and the live region that reads a pick out.
//!
//! The session measures and applies every pick; the app sends `Command::Picker` and draws
//! what `Event::Picker` says. What the app asked since the last reading shows at once: a
//! picker turned on or off, and the region a drag or a key sent, until a reading includes
//! it, so nothing waits for the GPU.
//!
//! - **On the photo** ([`on_photo`]): the active point or area as the design system's
//!   [`PickerMarks`] draw it, labelled with its tool's name, and the picker's icon beside
//!   the pointer ([`label`], [`badge`]), and darktable's pointer (`darkroom.c:3815-4140`): a press
//!   with a point moves it; with an area, a press within 6 points of a corner takes that
//!   corner, the opposite one held, and one elsewhere starts a new area 2 % of the photo's
//!   width each way of it, as tall on screen, the press held; Ctrl-click (Cmd-click on the
//!   Mac) switches point and area, and a right-click resets an area. A drag's steps are
//!   `done: false` and its release `done: true`.
//! - **Keys,** while its node has the focus, which keeps them from the app's actions
//!   (`actions::Focus::keeps`): the arrows move the point or the area a frame pixel, Shift
//!   ten; Alt with them moves an area's right or bottom edge; Space switches point and
//!   area, Delete or Backspace resets, and Esc turns the picker off, the focus going back to
//!   its button. Each is a completed change.
//! - **To screen readers** a group named "Picker area, Exposure", its value the region in
//!   percent of the photo, Increment and Decrement moving it along x and the custom
//!   actions Move up and Move down along y; and a polite live region reading out what a
//!   pick changed, or the colour measured, once the picker has rested half a second
//!   ([`Said`]).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use apertrust_engine::picker::{DEFAULT_AREA, DEFAULT_POINT};
use apertrust_engine::session::{
    Command, ControlValue, PickKeys, PickerCommand, PickerReading, PickerRef, Region, Values,
};
use apertrust_tools::pick::{LastPick, PRESSED, Patch, PickWarnings, Shown};
use apertrust_tools::{
    ControlKind, PickerKind, ProcessingDefinition, ProcessingTool, Row, messages,
};
use apertrust_ui::components::{PickerAction, PickerMarks, PickerPlace, ReadoutRow};
use apertrust_ui::i18n::Args;
use apertrust_ui::overlay::{Labelled, Overlay, Point, Weight};
use apertrust_ui::{Icon, Translator};
use eframe::egui::{
    self, Color32, CursorIcon, Key, Modifiers, Pos2, Rect, Role, Sense, Vec2, WidgetInfo, pos2,
    vec2,
};
use egui::accesskit::{Action, ActionData, CustomAction, Live};

use crate::settings::ToolSettings;

/// What the app knows of the active picker: the session's latest reading, and what the
/// app asked since that no reading shows yet.
#[derive(Debug, Default)]
pub struct Mirror {
    /// The session's latest reading; `None` while no picker is on.
    reading: Option<PickerReading>,
    /// The picker the app turned on, with whether it measures an area, or off (`None`), by
    /// its command's number, until a reading or an end through it says so or it is refused.
    asked: Option<(u64, Option<(PickerRef, bool)>)>,
    /// The commands the session's last `Event::Picker` is through: where it turned the
    /// picker off, the command that turned it off.
    through: Option<u64>,
    /// The region the app sent last, by its command's number, until a reading includes it.
    sent: Option<(u64, Region)>,
    /// Each picker's last point and area this session, as the session keeps them across
    /// photos: where `Switch` goes, shown before its reading arrives.
    places: BTreeMap<PickerRef, Places>,
}

/// A picker's last point and area, each with the command it is through.
#[derive(Debug, Default, Clone, Copy)]
struct Places {
    point: Option<(u64, Region)>,
    area: Option<(u64, Region)>,
}

impl Places {
    /// Keeps `region`, through the first `through` commands, unless its kind's place is
    /// through a later one already: a reading measured before a move never replaces it.
    fn keep(&mut self, through: u64, region: Region) {
        let place = match region {
            Region::Point(_) => &mut self.point,
            Region::Area(_) => &mut self.area,
        };
        if place.is_none_or(|(kept, _)| kept <= through) {
            *place = Some((through, region));
        }
    }
}

impl Mirror {
    /// The session's latest reading.
    pub fn reading(&self) -> Option<&PickerReading> {
        self.reading.as_ref()
    }

    /// The picker on, as the app shows it: the one it asked for, until the session answers,
    /// else the session's.
    pub fn active(&self) -> Option<PickerRef> {
        match self.asked {
            Some((_, asked)) => asked.map(|(picker, _)| picker),
            None => self.reading.as_ref().map(|reading| reading.picker),
        }
    }

    /// The active picker's region as shown: the one the app sent until a reading includes
    /// it, else the session's; `None` until the picker's first reading.
    pub fn region(&self) -> Option<Region> {
        let active = self.active()?;
        if let Some((_, region)) = self.sent {
            return Some(region);
        }
        let reading = self.reading.as_ref()?;
        (reading.picker == active).then_some(reading.region)
    }

    /// Whether the active picker measures an area: its region's kind, or before its first
    /// reading the kind it was turned on as.
    pub fn area(&self) -> Option<bool> {
        match (self.region(), self.asked) {
            (Some(region), _) => Some(matches!(region, Region::Area(_))),
            (None, Some((_, Some((_, area))))) => Some(area),
            _ => None,
        }
    }

    /// The command whose answer the picker on shows (`None`, off): the one the app asked by,
    /// until the session answers, else the one the session's last report is through.
    pub fn through(&self) -> Option<u64> {
        self.asked.map(|(number, _)| number).or(self.through)
    }

    /// The command numbered `number` turned `picker` on, measuring an area where its flag
    /// says, or the picker off (`None`).
    pub fn asked(&mut self, number: u64, picker: Option<(PickerRef, bool)>) {
        self.asked = Some((number, picker));
        self.sent = None;
    }

    /// The command numbered `number` moved the active picker to `region`.
    pub fn sent(&mut self, number: u64, region: Region) {
        self.sent = Some((number, region));
        if let Some(picker) = self.active() {
            self.places.entry(picker).or_default().keep(number, region);
        }
    }

    /// Where `Switch` takes the active picker, at `region`: its other kind's last place
    /// this session, else darktable's default for it.
    pub fn switched(&self, region: Region) -> Region {
        let places = (self.active()).and_then(|picker| self.places.get(&picker).copied());
        let places = places.unwrap_or_default();
        match region {
            Region::Point(_) => places.area.map_or(DEFAULT_AREA, |(_, area)| area),
            Region::Area(_) => places.point.map_or(DEFAULT_POINT, |(_, point)| point),
        }
    }

    /// The session's `Event::Picker`, through the first `through` commands: what the app
    /// asked by one of them is answered, a picker turned on included where the session
    /// turned it off again before its first reading, as a change of its tool does.
    pub fn apply(&mut self, through: u64, reading: Option<PickerReading>) {
        let answered = |sent: Option<u64>| sent.is_some_and(|n| n <= through);
        if answered(self.asked.map(|(n, _)| n)) {
            self.asked = None;
        }
        if answered(self.sent.map(|(n, _)| n)) || reading.is_none() && self.asked.is_none() {
            self.sent = None;
        }
        if let Some(reading) = &reading {
            (self.places.entry(reading.picker).or_default()).keep(through, reading.region);
        }
        self.reading = reading;
        self.through = Some(through);
    }

    /// The command numbered `number` was refused: what it asked stops showing.
    pub fn refused(&mut self, number: u64) {
        if self.asked.is_some_and(|(n, _)| n == number) {
            self.asked = None;
        }
        if self.sent.is_some_and(|(n, _)| n == number) {
            self.sent = None;
        }
    }

    /// The photo is left: its picker with it, as the session turns it off; each picker's
    /// places stay, as the session keeps them.
    pub fn forget(&mut self) {
        *self = Mirror {
            places: std::mem::take(&mut self.places),
            ..Mirror::default()
        };
    }
}

/// What the tools' picks showed on the photo, darktable's `spot_RGB` and L readout: each
/// tool's last input patch, for its panel's patches (`Row::Patches`), kept after its
/// picker turns off until the tool's values change from outside a pick ([`Picked::sent`])
/// or the photo closes, as darktable's `gui_update` zeroes `spot_RGB`; what its last pick
/// did and its warnings, kept and taken away with it (decision 88); and the tool's settings
/// a pick measured, as a measure mode's target, handed to the app's settings once.
#[derive(Debug, Default)]
pub struct Picked {
    patches: BTreeMap<&'static str, Patch>,
    /// What each tool's last pick did and its warnings, kept and taken away as its patch is
    /// (decision 88).
    lasts: BTreeMap<&'static str, (&'static str, LastPick, PickWarnings)>,
    /// The latest pick shown of the tool whose picker is on: every reading after it
    /// carries it again, so its patch and the settings it measured are taken once.
    seen: Option<(&'static str, Shown)>,
    /// The last command that took every tool's patch away, by its number.
    cleared: u64,
    /// The last command that took one tool's patch away, by its number, by tool.
    cleared_tool: BTreeMap<String, u64>,
    /// The last command that cleared a tool's measured settings, its reference, by its
    /// number, by tool (decision 88).
    cleared_settings: BTreeMap<String, u64>,
}

impl Picked {
    pub const fn new() -> Picked {
        Picked {
            patches: BTreeMap::new(),
            lasts: BTreeMap::new(),
            seen: None,
            cleared: 0,
            cleared_tool: BTreeMap::new(),
            cleared_settings: BTreeMap::new(),
        }
    }

    /// The session's `Event::Picker`: the tool's patch kept, and the tool with the
    /// settings its pick measured, where the pick comes for the first time. A pick measured
    /// before a command that took the tool's patch away, still on its way as that command
    /// was sent, shows no patch, as darktable's `gui_update` leaves it zeroed until the next
    /// pick; one measured before a command that cleared the settings it measures hands
    /// none of them back ([`Picked::settings_cleared`]).
    pub fn apply(
        &mut self,
        reading: Option<&PickerReading>,
    ) -> Option<(&'static str, Vec<(&'static str, ControlValue)>)> {
        let Some(reading) = reading else {
            // Off: the next `On` applies a reading of its own.
            self.seen = None;
            return None;
        };
        let PickerRef::Tool { tool, picker } = reading.picker else {
            return None;
        };
        let shown = reading.tool.as_ref()?.picked.as_ref()?;
        let seen = (tool, shown.clone());
        if self.seen.as_ref() == Some(&seen) {
            return None;
        }
        let settings_cleared = self.cleared_settings.get(tool).copied().unwrap_or(0);
        let refused = !shown.settings.is_empty() && reading.through < settings_cleared;
        // A pick whose settings a clear refused is not taken as seen, so a later one with
        // the same values, measured after the clear, hands them back.
        if !refused {
            self.seen = Some(seen);
        }
        let cleared =
            (self.cleared_tool.get(tool).copied()).map_or(self.cleared, |n| n.max(self.cleared));
        if let Some(patch) = shown.patch
            && reading.through >= cleared
        {
            self.patches.insert(tool, patch);
        }
        // A pick of the picker that gave the Last pick shown, with none of its own, as one
        // toward L 0 that writes nothing, takes it away rather than leave an older one.
        if reading.through >= cleared {
            match shown.last {
                Some(last) => {
                    self.lasts.insert(tool, (picker, last, shown.warnings));
                }
                None if self.lasts.get(tool).is_some_and(|(by, ..)| *by == picker) => {
                    self.lasts.remove(tool);
                }
                None => {}
            }
        }
        (!shown.settings.is_empty() && !refused).then(|| (tool, shown.settings.clone()))
    }

    /// The command numbered `number`, `tool`'s `Settings`, cleared settings its picks
    /// measure, as Clear Reference clears Exposure's reference (decision 88): a pick
    /// measured before it, still on its way, hands none of them back ([`Picked::apply`]).
    pub fn settings_cleared(&mut self, tool: &str, number: u64) {
        self.cleared_settings.insert(tool.to_owned(), number);
    }

    /// The input patch `tool`'s last pick showed on the photo.
    pub fn patch(&self, tool: &str) -> Option<&Patch> {
        self.patches.get(tool)
    }

    /// What `tool`'s last pick did, and its warnings (decision 88).
    pub fn last(&self, tool: &str) -> Option<(&LastPick, &PickWarnings)> {
        self.lasts
            .get(tool)
            .map(|(_, last, warnings)| (last, warnings))
    }

    /// The photo is left: its patches with it.
    pub fn forget(&mut self) {
        *self = Picked::new();
    }

    /// The patches `command` takes away as it changes tools' values from outside a pick,
    /// as darktable's `gui_update` zeroes `spot_RGB` and so the L shown with it
    /// (`exposure.c:707-710`): `Some(None)` every tool's, for undo, redo, a move through
    /// history, the edit's reset, and Compress, Truncate and Discard History, after which
    /// darktable reloads the history and so its modules' `gui_update` too
    /// (`dt_dev_history_truncate`, `src/develop/develop.c:1860`);
    /// `Some(Some(tool))` one tool's, for its reset and for its whole values set, as a
    /// preset sets them; a `Step`'s, what its commands take, every tool's where they take
    /// more than one tool's; `None` for any other command.
    pub fn clears(command: &Command) -> Option<Option<String>> {
        match command {
            Command::Undo
            | Command::Redo
            | Command::GoTo { .. }
            | Command::Reset
            | Command::Compress { .. }
            | Command::Truncate { .. }
            | Command::DiscardHistory => Some(None),
            Command::ResetTool { tool } | Command::SetTool { tool, .. } => Some(Some(tool.clone())),
            Command::Step { commands, .. } => {
                let mut cleared = commands.iter().filter_map(Picked::clears);
                let first = cleared.next()?;
                match cleared.all(|other| other == first) {
                    true => Some(first),
                    false => Some(None),
                }
            }
            _ => None,
        }
    }

    /// The command numbered `number` was sent, taking away the patches `clears` says
    /// ([`Picked::clears`]): a pick measured before it shows none ([`Picked::apply`]).
    pub fn sent(&mut self, number: u64, clears: Option<Option<String>>) {
        match clears {
            Some(None) => {
                self.patches.clear();
                self.lasts.clear();
                self.cleared = number;
            }
            Some(Some(tool)) => {
                self.patches.remove(tool.as_str());
                self.lasts.remove(tool.as_str());
                self.cleared_tool.insert(tool, number);
            }
            None => {}
        }
    }
}

/// The region a picker command moves the active picker to, if it moves it.
pub fn moved_to(command: &PickerCommand) -> Option<Region> {
    match *command {
        PickerCommand::Point { at, .. } => Some(Region::Point(at)),
        PickerCommand::Area { from, to, .. } => Some(Region::Area([
            from[0].min(to[0]),
            from[1].min(to[1]),
            from[0].max(to[0]),
            from[1].max(to[1]),
        ])),
        _ => None,
    }
}

/// What a press on `picker`'s button sends, `keys` held: `On`, with the settings its
/// tool's pickers read as the app keeps them, `settings`, and no channel tab until a tool
/// reads one, or `Off` for the picker on, since `On` for it would start it again
/// (`docs/pickers.md`, Against darktable).
pub fn pressed(
    picker: PickerRef,
    action: PickerAction,
    keys: PickKeys,
    settings: Vec<(&'static str, ControlValue)>,
) -> PickerCommand {
    match action {
        PickerAction::On { area } => PickerCommand::On {
            picker,
            area,
            keys,
            settings,
            channel: None,
        },
        PickerAction::Off => PickerCommand::Off,
    }
}

/// What the app shows of a picker command at once, until the session answers: a picker
/// turned on, measuring the kind it asks, or off; a region moved; and, for a command that
/// applies a reading, a pick for the live region to wait for.
#[derive(Debug)]
pub struct Showing {
    asked: Option<Option<(PickerRef, bool)>>,
    moved: Option<Region>,
    /// `Switch` or `Reset`, which move the picker where the mirror knows.
    kind: Option<PickerCommand>,
    applies: bool,
}

impl Showing {
    /// What `command` shows, its picker among `tools`.
    pub fn of(command: &PickerCommand, tools: &[&dyn ProcessingTool]) -> Showing {
        let asked = match *command {
            PickerCommand::On { picker, area, .. } => {
                let kind = declared(tools, picker).map(|(_, declared)| declared.kind);
                let area = kind.map_or(area, |kind| measures_area(kind, area));
                Some(Some((picker, area)))
            }
            PickerCommand::Off => Some(None),
            _ => None,
        };
        let kind = matches!(command, PickerCommand::Switch | PickerCommand::Reset);
        Showing {
            asked,
            moved: moved_to(command),
            kind: kind.then(|| command.clone()),
            applies: applies(command),
        }
    }

    /// Shows it, the command sent as `number`, in `mirror`, with `said` waiting for its
    /// pick against `values`; a picker turned on moves the focus to the photo.
    pub fn sent(
        self,
        number: u64,
        mirror: &mut Mirror,
        said: &mut Said,
        values: &Values,
        ctx: &egui::Context,
    ) {
        if let Some(asked) = self.asked {
            mirror.asked(number, asked);
            if asked.is_some() {
                focus_photo(ctx);
            }
        }
        let before = mirror.region();
        let moved = match (&self.kind, before) {
            (Some(command), Some(region)) => Some(follow(command, region, mirror.switched(region))),
            _ => self.moved,
        };
        if let Some(region) = moved {
            mirror.sent(number, region);
        }
        if self.applies
            && let Some(picker) = mirror.active()
        {
            // A move to where the picker is brings no reading: it rests the picker, its
            // reading still the one the last move awaits.
            if self.moved.is_some() && moved == before {
                said.rested(Instant::now());
            } else {
                said.moved(number, Instant::now(), picker, values);
            }
        }
    }
}

/// Where `command` moves a picker at `region`, `switched` where `Switch` takes it: a move
/// to its place, `Switch` to its other kind's, `Reset` to its kind's default.
pub fn follow(command: &PickerCommand, region: Region, switched: Region) -> Region {
    match command {
        PickerCommand::Switch => switched,
        PickerCommand::Reset => match region {
            Region::Point(_) => DEFAULT_POINT,
            Region::Area(_) => DEFAULT_AREA,
        },
        command => moved_to(command).unwrap_or(region),
    }
}

/// Whether the session applies a tool picker's reading after `command`
/// (`docs/pickers.md`, What a pick writes): what the live region waits for.
pub fn applies(command: &PickerCommand) -> bool {
    matches!(
        command,
        PickerCommand::On { .. }
            | PickerCommand::Point { .. }
            | PickerCommand::Area { .. }
            | PickerCommand::Switch
            | PickerCommand::Reset
    )
}

/// Whether `command` completes a change where its pick writes, as the session saves it:
/// `On`, `Switch`, `Reset`, and a move's release (`docs/pickers.md`, What a pick writes).
pub fn completes(command: &PickerCommand) -> bool {
    matches!(
        command,
        PickerCommand::On { .. }
            | PickerCommand::Switch
            | PickerCommand::Reset
            | PickerCommand::Point { done: true, .. }
            | PickerCommand::Area { done: true, .. }
    )
}

/// Whether `command`, sent while `active` is the picker on, completes a change on the edit,
/// which the session answers with a write or the edit reported held: a tool picker's
/// command that [`completes`] one, and `Settings`, which applies its tool's latest reading
/// again whether its picker is on or not; never the Colour picker's, which writes nothing.
pub fn completes_edit(command: &PickerCommand, active: Option<PickerRef>) -> bool {
    let picker = match command {
        PickerCommand::Settings { .. } => return true,
        PickerCommand::On { picker, .. } => Some(*picker),
        _ => active,
    };
    matches!(picker, Some(PickerRef::Tool { .. })) && completes(command)
}

/// Whether a picker of `kind` turned on with `area` measures an area: one kind, that
/// kind; both, an area where asked.
pub fn measures_area(kind: PickerKind, area: bool) -> bool {
    match kind {
        PickerKind::Area => true,
        PickerKind::Point => false,
        PickerKind::PointArea => area,
    }
}

/// The keys held as a pick reads them: Ctrl, or Cmd on the Mac, and Shift.
pub fn pick_keys(modifiers: Modifiers) -> PickKeys {
    PickKeys {
        command: modifiers.command || modifiers.ctrl || modifiers.mac_cmd,
        shift: modifiers.shift,
    }
}

/// Whether a press with `modifiers` is darktable's Ctrl-click on the photo, which compares
/// the whole mask: Ctrl alone, or on the Mac (`mac`) Cmd alone, Control-click being the
/// Mac's right-click ([`mac_right_click`]).
fn command_alone(modifiers: Modifiers, mac: bool) -> bool {
    let command = if mac {
        modifiers.mac_cmd && !modifiers.ctrl
    } else {
        modifiers.command || modifiers.ctrl
    };
    command && !modifiers.shift && !modifiers.alt
}

/// Whether a primary press with `modifiers` is the Mac's right-click, Control-click with
/// no Cmd, as everywhere in the app (`docs/gui.md`, Popups).
pub fn mac_right_click(modifiers: Modifiers, mac: bool) -> bool {
    mac && modifiers.ctrl && !modifiers.mac_cmd
}

/// A tool's picker and its definition, among `tools`.
fn declared<'m>(
    tools: &'m [&'m dyn ProcessingTool],
    picker: PickerRef,
) -> Option<(&'m ProcessingDefinition, &'static apertrust_tools::Picker)> {
    let PickerRef::Tool { tool, picker } = picker else {
        return None;
    };
    let found = tools.iter().find(|m| m.definition().tool.id == tool)?;
    let definition = found.definition();
    Some((definition, definition.picker(picker)?))
}

/// A picker's name (`docs/pickers.md`, On the photo): its control's label where it sits in
/// a slider's quad, else its own words.
pub fn name(words: &Translator, definition: &ProcessingDefinition, picker: &str) -> String {
    let tool = definition.tool.id;
    let quad = definition.rows().find_map(|row| match row {
        Row::Slider(slider) if row.pickers() == [picker] => Some(slider),
        _ => None,
    });
    let id = match quad {
        Some(slider) => match slider.label {
            Some(own) => messages::row_words(tool, slider.control, own)[0].clone(),
            None => messages::label(tool, slider.control),
        },
        None => messages::picker(tool, picker)[0].clone(),
    };
    words.text(&id)
}

/// The label naming `picker`'s tool on its area or point (`docs/pickers.md`, On the photo):
/// the tool's title, or where the tool declares more than one picker, each turned on from
/// a button, a quad or a preset row's option, the title and the picker's [`name`],
/// "Exposure · Reference" (`picker-area-label`); the title alone where the name, as shown,
/// repeats it, as Exposure's own picker's does.
pub fn label(words: &Translator, definition: &ProcessingDefinition, picker: &str) -> String {
    let title = words.text(&messages::title(definition.tool.id));
    let name = name(words, definition, picker);
    if definition.pickers.len() < 2 || name == title {
        return title;
    }
    let mut args = Args::new();
    args.set("tool", title);
    args.set("picker", name);
    words.text_with("picker-area-label", Some(&args))
}

/// The pointer's badge for `picker` (`docs/pickers.md`, On the photo): the icon of its
/// cursor in the Toolbar's well, White balance's `pick-white-balance` or Exposure's
/// `pick-exposure`, else the pipette.
pub fn badge(definition: &ProcessingDefinition, picker: &str) -> Icon {
    (definition.tool.cursors.iter())
        .find(|cursor| cursor.picker == picker)
        .map_or(Icon::Pick, |cursor| {
            crate::cursors::icon(cursor.cursor.icon)
        })
}

/// A picker's words that say what a pick will do, by the option the setting its words
/// follow holds (`Picker::words_by`, decision 88), as the app keeps it in `settings`, else
/// that setting's default: `part` `promise`, its tooltip, or `hint`, the viewer bar's
/// name of it, "Make an area middle grey". Each of the tool's number settings is the
/// message's variable of its id, in the language's numbers to the setting's decimals,
/// none shown after the point where it is whole, "Make an area L 92 %"; a reference's L
/// the same, to one decimal, and its photo the name of its file
/// (`$reference_photo`, "Match an area to IMG_2041 (L 62 %)"), or "this photo" where it
/// is the `open` one. `None` for a picker whose words follow no setting.
pub fn by_setting(
    words: &Translator,
    definition: &ProcessingDefinition,
    picker: &apertrust_tools::Picker,
    settings: &ToolSettings,
    open: Option<&std::path::Path>,
    part: &str,
) -> Option<String> {
    let tool = definition.tool.id;
    let setting = picker.words_by?;
    let options = definition.choice_setting(setting)?;
    let rows: Vec<&apertrust_tools::SettingRow> = (definition.rows())
        .filter_map(|row| match row {
            Row::Setting(row) => Some(row),
            _ => None,
        })
        .collect();
    let value = |row: &apertrust_tools::SettingRow| {
        settings
            .get(tool, row.id)
            .cloned()
            .unwrap_or_else(|| row.default())
    };
    let chosen = rows
        .iter()
        .find(|row| row.id == setting)
        .map(|row| value(row));
    let option = match chosen {
        Some(ControlValue::Choice(at)) => options.get(at as usize),
        _ => None,
    }
    .or(options.first())?;
    let mut args = Args::new();
    for row in &rows {
        if let ControlValue::Number(number) = value(row) {
            let scale = 10f64.powi(i32::from(row.decimals));
            let rounded = (f64::from(number) * scale).round() / scale;
            args.set(row.id, rounded);
        }
    }
    let mut named = true;
    let mut this_photo = false;
    for row in definition.rows() {
        if let Row::Reference {
            lightness, photo, ..
        } = row
        {
            if let Some(ControlValue::Number(l)) = settings.get(tool, lightness) {
                args.set(*lightness, (f64::from(*l) * 10.0).round() / 10.0);
            }
            match settings.text(tool, photo) {
                Some(path) => {
                    this_photo = open.is_some_and(|open| open == std::path::Path::new(path));
                    args.set(*photo, photo_name(path));
                }
                None => named = false,
            }
        }
    }
    // A reference whose photo is not known, as one `--conf` gives, says only its L; one
    // measured on the open photo says so.
    let shows_reference = messages::reference_options(definition, picker).contains(option);
    let id = if shows_reference && !named {
        messages::picker_option_no_photo(tool, picker.id, part, option)
    } else if shows_reference && this_photo {
        messages::picker_option_this_photo(tool, picker.id, part, option)
    } else {
        messages::picker_option(tool, picker.id, part, option)
    };
    Some(words.text_with(&id, Some(&args)))
}

/// A last pick's words (decision 88, `Row::LastPick`): its readout, "■ L 41 % → ■ L 50 % ·
/// +0.55 EV", each ■ where the row draws a patch, and its warning lines, each with its
/// tooltip, the clipped one first.
#[derive(Debug, Clone, PartialEq)]
pub struct LastPickWords {
    pub label: String,
    pub value: String,
    pub promise: String,
    pub warnings: Vec<(String, String)>,
}

impl LastPickWords {
    /// What the live region says of it, once: its warnings, then the readout.
    pub fn said(&self) -> String {
        let mut lines: Vec<&str> = self
            .warnings
            .iter()
            .map(|(text, _)| text.as_str())
            .collect();
        let readout = format!("{} {}", self.label, ReadoutRow::spoken(&self.value));
        lines.push(&readout);
        lines.join(". ")
    }
}

/// The words of `picker`'s last pick `last` with its `warnings`, the control it set the
/// picker's first write, its change and an end it was held at written with its unit in
/// the language's numbers, the Ls whole. `None` for a picker of no tool's or one that
/// writes nothing.
pub fn last_pick_words(
    words: &Translator,
    definition: &ProcessingDefinition,
    picker: &apertrust_tools::Picker,
    last: &LastPick,
    warnings: &PickWarnings,
) -> Option<LastPickWords> {
    let tool = definition.tool.id;
    let control = *picker.writes.first()?;
    let unit = definition.control(control).and_then(|found| found.unit);
    let with_unit = |value: f32, decimals: usize| {
        let number = words.format_number(f64::from(value), decimals, true);
        match unit {
            Some(unit) => {
                let mut args = Args::new();
                args.set("value", number);
                words.text_with(&messages::unit_value(unit), Some(&args))
            }
            None => number,
        }
    };
    let lightness = |l: f32| words.format_number(f64::from(l), 0, false);
    let [
        label,
        value_id,
        promise,
        clipped,
        clipped_tip,
        short,
        short_tip,
    ] = messages::LAST_PICK;
    let mut args = Args::new();
    args.set("before", lightness(last.before));
    args.set("after", lightness(last.after));
    args.set("change", with_unit(last.exposure, 2));
    let value = words.text_with(value_id, Some(&args));
    let mut lines = Vec::new();
    if warnings.clipped {
        lines.push((words.text(clipped), words.text(clipped_tip)));
    }
    if let Some(held) = &warnings.short {
        let mut args = Args::new();
        args.set("control", words.text(&messages::label(tool, control)));
        args.set("value", with_unit(held.exposure, 0));
        args.set("reached", lightness(held.reached));
        args.set("target", lightness(held.target));
        let mut tip = words.text_with(short_tip, Some(&args));
        if held.black {
            tip.push('\n');
            tip.push_str(&words.text(&messages::short_black(tool, picker.id)));
        }
        lines.push((words.text_with(short, Some(&args)), tip));
    }
    Some(LastPickWords {
        label: words.text(label),
        value,
        promise: words.text(promise),
        warnings: lines,
    })
}

/// The name a reference's photo shows by, "IMG_2041": its file's name without its
/// extension, as Collections shows a photo, whatever folder it is in.
pub fn photo_name(path: &str) -> String {
    let path = std::path::Path::new(path);
    path.file_stem().or(path.file_name()).map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// Whether the photos references were measured on are still there (decision 88): each
/// path asked on a thread of its own, never the interface's, so a slow share never holds
/// a frame, and a path still being asked is not asked again. The app asks for every
/// reference as each photo opens and for one as it is set.
#[derive(Debug)]
pub struct ReferencePhotos {
    gone: std::collections::BTreeSet<String>,
    asking: std::collections::BTreeSet<String>,
    answer: std::sync::mpsc::Sender<(String, bool)>,
    answers: std::sync::mpsc::Receiver<(String, bool)>,
    /// The open whose photo the references were last asked after for.
    asked_for: Option<u64>,
}

impl ReferencePhotos {
    pub fn new() -> ReferencePhotos {
        let (answer, answers) = std::sync::mpsc::channel();
        ReferencePhotos {
            gone: Default::default(),
            asking: Default::default(),
            answer,
            answers,
            asked_for: None,
        }
    }

    /// Whether `open`, the number of the open of the photo shown, is new since the last
    /// time this said so: the references are to be asked after again.
    pub fn photo_opened(&mut self, open: Option<u64>) -> bool {
        let new = open.is_some() && open != self.asked_for;
        if new {
            self.asked_for = open;
        }
        new
    }

    /// Asks whether `path` is there, on a thread of its own, unless it is being asked.
    pub fn check(&mut self, path: String, ctx: &egui::Context) {
        if !self.asking.insert(path.clone()) {
            return;
        }
        let answer = self.answer.clone();
        // The answer drawn as it comes, not at the next input.
        let ctx = ctx.clone();
        let asked = std::thread::Builder::new()
            .name("reference check".into())
            .spawn(move || {
                let there = std::path::Path::new(&path).try_exists().unwrap_or(true);
                let _ = answer.send((path, there));
                ctx.request_repaint();
            });
        if let Err(e) = asked {
            tracing::warn!(target: "settings", "reference not checked: no thread: {e}");
        }
    }

    /// Marks the answers that have come, without waiting for the others. Returns whether
    /// any changed what is marked.
    pub fn take(&mut self) -> bool {
        let mut changed = false;
        while let Ok((path, there)) = self.answers.try_recv() {
            self.asking.remove(&path);
            changed |= if there {
                self.gone.remove(&path)
            } else {
                self.gone.insert(path)
            };
        }
        changed
    }

    /// Whether the photo at `path` was found gone.
    pub fn gone(&self, path: &str) -> bool {
        self.gone.contains(path)
    }
}

/// The viewer bar's words while `picker` is on, measuring an area where `area` says
/// (`docs/pickers.md`, On the photo): which picker it is, the tool's title and the
/// picker's name, or what a pick will do where its words follow a setting ([`by_setting`],
/// "Exposure · Make an area middle grey"), and how to use it by the kinds it declares, a
/// switch's modifier, for a picker with both, as `mac` spells it. `open` is the photo that is open, which a reference measured on it names as this
/// photo. `None` for the Colour picker's, whose words come with its tool.
pub fn bar_words(
    words: &Translator,
    tools: &[&dyn ProcessingTool],
    settings: &ToolSettings,
    open: Option<&std::path::Path>,
    picker: PickerRef,
    area: bool,
    mac: bool,
) -> Option<(String, String)> {
    let (definition, declared) = declared(tools, picker)?;
    let mut args = Args::new();
    args.set("tool", words.text(&messages::title(definition.tool.id)));
    // What a pick will do, where the picker's words follow a setting; a reference row's
    // picker's own words over the photo.
    let reference = (definition.rows())
        .any(|row| matches!(row, Row::Reference { picker, .. } if *picker == declared.id));
    let hint = by_setting(words, definition, declared, settings, open, "hint").or_else(|| {
        reference.then(|| words.text(&messages::picker_hint(definition.tool.id, declared.id)))
    });
    args.set(
        "picker",
        hint.unwrap_or_else(|| name(words, definition, declared.id)),
    );
    let name = words.text_with("picker-bar-name", Some(&args));
    let mut args = Args::new();
    args.set("command", if mac { "Cmd" } else { "Ctrl" });
    let what = || words.text(&messages::picker(definition.tool.id, declared.id)[3]);
    // A picker that takes one kind keeps it; one with both switches until 105.5.
    let how = match declared.kind {
        PickerKind::Area => words.text("picker-bar-area"),
        PickerKind::Point => {
            args.set("what", what());
            words.text_with("picker-bar-point", Some(&args))
        }
        PickerKind::PointArea if area => words.text_with("picker-bar-both-area", Some(&args)),
        PickerKind::PointArea => {
            args.set("what", what());
            words.text_with("picker-bar-both-point", Some(&args))
        }
    };
    Some((name, how))
}

/// The id of the picker's node on the photo, whatever draws it.
pub fn photo_id() -> egui::Id {
    egui::Id::unique("picker on the photo")
}

/// Asks the picker's node on the photo to take the keyboard's focus as it next shows: a
/// picker turned on moves the focus to the photo, as darktable gives the centre view the
/// focus (`lib.c:1590`).
pub fn focus_photo(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        d.insert_temp(photo_id().with("focus"), true);
        d.remove::<(PickerRef, u64)>(photo_id().with("give back"));
    });
}

/// How many egui passes after Esc the picker's button may take the focus back, counted by
/// `Context::cumulative_pass_nr`, not frames shown: a frame may run more than one pass,
/// as when egui asks to discard one. The next pass, or the one after where the first
/// drew the button before the picker turned off; later the focus never jumps to it.
const GIVE_BACK: u64 = 2;

/// Asks `picker`'s button to take the focus back as it is next drawn, within [`GIVE_BACK`]
/// frames: Esc on the photo.
fn give_back(ctx: &egui::Context, picker: PickerRef) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(photo_id().with("give back"), (picker, pass)));
}

/// Whether `picker`'s button takes the focus back now, which it then has.
pub fn takes_back(ctx: &egui::Context, picker: PickerRef) -> bool {
    let id = photo_id().with("give back");
    let Some((asked, at)) = ctx.data(|d| d.get_temp::<(PickerRef, u64)>(id)) else {
        return false;
    };
    let late = ctx.cumulative_pass_nr() > at + GIVE_BACK;
    if late || asked == picker {
        ctx.data_mut(|d| d.remove::<(PickerRef, u64)>(id));
    }
    asked == picker && !late
}

/// The active picker over the photo this frame.
pub struct OnPhoto<'a> {
    pub picker: PickerRef,
    /// The kinds it declares, both for the Colour picker's: one kind it keeps on the photo,
    /// both it switches (decision 105).
    pub kind: PickerKind,
    /// The photo area, the part of the photo shown, which takes the pointer and cuts the
    /// marks: the whole photo's rectangle at Fit.
    pub area: Rect,
    /// The whole photo's rectangle on the screen at the zoom, larger than the area where
    /// the photo overflows it, which the marks are placed on (`docs/gui.md`, On-photo
    /// overlays).
    pub photo: Rect,
    /// The whole photo's size in points at Fit, `photo`'s there: a click's area keeps its
    /// size at Fit on the screen at every zoom (decision 105).
    pub fitted: Vec2,
    /// The whole photo's size in pixels of the frame's scale, so a frame pixel spans
    /// `photo`'s width over its first.
    pub pixels: [u32; 2],
    /// Where the picker is, as shown ([`Mirror::region`]).
    pub region: Region,
    /// Where `Switch` takes it ([`Mirror::switched`]).
    pub switched: Region,
    /// What its point sampled, as the screen shows it.
    pub colour: Color32,
    /// Its name, as the viewer bar gives it ([`name`]).
    pub name: &'a str,
    /// The label naming its tool by its area or point ([`label`]); none drawn while empty.
    pub label: &'a str,
    /// Its icon beside the pointer ([`badge`]).
    pub badge: Icon,
    /// Whether A is held, which borrows Pan over the picker (darktable's force-pan): its
    /// marks stay and its node keeps the focus, but a press is the pan's, and a drag under
    /// way ends where it was.
    pub panning: bool,
}

/// What a press of the picker on the photo moves, between the press and the release.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// The point follows the pointer; `last` is where it was sent last.
    Point { last: [f32; 2] },
    /// The area runs from `held`, the corner or the press that stays, to the pointer;
    /// `last` is what was sent last, which the release sends again as done.
    Area {
        held: [f32; 2],
        last: Option<([f32; 2], [f32; 2])>,
    },
}

/// A press on the photo with its drag, between the press and the release.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    /// Where it went down, on the screen.
    origin: Pos2,
    /// Whether the pointer has gone past [`CLICK`] from `origin`: a drag from then to the
    /// release, wherever it goes after; until then a click, and nothing follows it.
    dragging: bool,
    drag: Drag,
}

/// How near an area's edges, in points, a press takes their corner, strictly within it.
const CORNER: f32 = 6.0;

/// How far, in points on the screen, a press may move from where it went down and stay a
/// click (decision 105): egui's default `max_click_dist`, a drag only past it, but by
/// distance alone, never by egui's click time.
const CLICK: f32 = 6.0;

/// Whether a press that went down at `origin` is a drag with the pointer at `pos`: past
/// [`CLICK`] points from it, however long it has been held.
fn dragging(origin: Pos2, pos: Pos2) -> bool {
    origin.distance(pos) > CLICK
}

/// Whether a press of the secondary button, `secondary`, resets the picker at `region`:
/// on an area, from the button itself, never egui's long press on a touch screen,
/// `long_touched`, until Touch mode settles touch (decision 105).
fn resets(secondary: bool, long_touched: bool, region: Region) -> bool {
    secondary && !long_touched && matches!(region, Region::Area(_))
}

/// What the picker on the photo did this frame ([`on_photo`]).
#[derive(Debug, Default, PartialEq)]
pub struct PhotoPicked {
    /// The commands the pointer and the keys sent, in order.
    pub commands: Vec<PickerCommand>,
    /// While a drag holds the pointer past the photo area's edge, how far past, in points:
    /// the view moves that far towards it each frame, as darktable's
    /// (`src/views/darkroom.c:3895-3902`).
    pub pan: Option<Vec2>,
    /// Where a key moved the mark past the photo area's edge, the mark's box grown by its
    /// reach, in the photo's coordinates: the view moves just enough to show it again
    /// (`docs/gui.md`, On-photo overlays, At every zoom).
    pub reveal: Option<Rect>,
}

/// Draws the picker's marks over the photo and returns what the pointer and the keys did
/// this frame. Its node takes the focus where asked ([`focus_photo`]); Esc's `Off` gives
/// it back to the picker's button. The pointer over it is the crosshair, the move pointer
/// while an area is dragged.
pub fn on_photo(ui: &mut egui::Ui, on: &OnPhoto) -> PhotoPicked {
    let ctx = ui.ctx().clone();
    let id = photo_id();
    // While A is held the pan takes the pointer, the node keeping its focus.
    let sense = if on.panning {
        Sense::focusable_noninteractive()
    } else {
        Sense::click_and_drag()
    };
    let response = ui.interact(on.area, id, sense);
    if ctx.data_mut(|d| d.remove_temp::<bool>(id.with("focus"))) == Some(true) {
        response.request_focus();
    }
    // The arrows and Esc are the picker's, never egui's moves of the focus.
    let keeps = egui::EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        escape: true,
        ..Default::default()
    };
    ctx.memory_mut(|m| m.set_focus_lock_filter(id, keeps));
    let size = on.photo.size();
    let pixel = [
        1.0 / on.pixels[0].max(1) as f32,
        1.0 / on.pixels[1].max(1) as f32,
    ];
    // The pointer held inside the area, past which the view moves instead.
    let at = |pos: Pos2| {
        let p = (on.area.clamp(pos) - on.photo.min) / size;
        [p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0)]
    };
    let mut commands = Vec::new();

    // The pointer.
    let drag_id = id.with("drag");
    let mut drag: Option<Press> = ctx.data(|d| d.get_temp(drag_id));
    let (pressed, released, down, moved, pos, origin, modifiers) = ui.input(|i| {
        (
            i.pointer.primary_pressed(),
            i.pointer.primary_released(),
            i.pointer.primary_down(),
            i.pointer.delta() != Vec2::ZERO,
            i.pointer.latest_pos(),
            i.pointer.press_origin(),
            i.modifiers,
        )
    });
    // A touch screen's pinch zooms the photo: a drag its first finger began ends where it
    // was, and the fingers pick nothing more until they lift.
    let pinching = ui.input(|i| i.multi_touch().is_some());
    let released = released || ((pinching || on.panning) && drag.is_some());
    let pinching = pinching || on.panning;
    // A drag lasts while the button is held: one whose release this never saw, as the
    // picker turned off under it, is over.
    if !(down || released) {
        drag = None;
    }
    let keys = pick_keys(modifiers);
    // The press where it was, a move after it in the same frame then followed.
    let pressed_at = origin
        .or(pos)
        .filter(|&at| pressed && !pinching && on.area.contains(at) && response.contains_pointer());
    if let Some(origin) = pressed_at {
        response.request_focus();
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        drag =
            pressed_on(&mut commands, on, at(origin), origin, modifiers, mac).map(|drag| Press {
                origin,
                dragging: false,
                drag,
            });
    }
    let moved = moved && pressed_at.is_none_or(|origin| pos != Some(origin));
    // Held past the area's edge, the view moves under a still pointer: followed too, and
    // on the release, so it ends where the pointer is on the photo as now placed.
    let outside = pos.is_some_and(|pos| !on.area.contains(pos));
    let held_past = drag.is_some_and(|press| press.dragging) && (down || released) && outside;
    let moved = !pinching && (moved || held_past);
    // A click until the pointer goes past the threshold: what the press sent stays.
    if let (Some(press), Some(pos), true) = (drag, pos, moved)
        && (press.dragging || dragging(press.origin, pos))
    {
        let to = at(pos);
        let followed = match press.drag {
            Drag::Point { .. } => {
                commands.push(PickerCommand::Point {
                    at: to,
                    done: false,
                    keys,
                });
                Drag::Point { last: to }
            }
            Drag::Area { held, .. } => {
                let to = at_least_a_pixel(held, to, pixel);
                commands.push(PickerCommand::Area {
                    from: held,
                    to,
                    done: false,
                    keys,
                });
                Drag::Area {
                    held,
                    last: Some((held, to)),
                }
            }
        };
        drag = Some(Press {
            dragging: true,
            drag: followed,
            ..press
        });
    }
    if released && let Some(ended) = drag.take() {
        match ended.drag {
            Drag::Point { last } => commands.push(PickerCommand::Point {
                at: last,
                done: true,
                keys,
            }),
            Drag::Area {
                last: Some((from, to)),
                ..
            } => commands.push(PickerCommand::Area {
                from,
                to,
                done: true,
                keys,
            }),
            // A corner taken and let go where it was moves nothing.
            Drag::Area { last: None, .. } => {}
        }
    }
    if resets(
        response.secondary_clicked(),
        response.long_touched(),
        on.region,
    ) {
        commands.push(PickerCommand::Reset);
    }
    // The crosshair, the move pointer over a corner a press would take and while an area
    // is dragged.
    let corner = pos.is_some_and(|pos| on_corner(on, pos));
    let area_dragged = drag.is_some_and(|press| matches!(press.drag, Drag::Area { .. }));
    if area_dragged || (!on.panning && response.hovered() && corner) {
        ctx.set_cursor_icon(CursorIcon::Move);
    } else if !on.panning && (response.hovered() || drag.is_some()) {
        ctx.set_cursor_icon(CursorIcon::Crosshair);
    }
    // The keys and screen readers' actions, while the node has the focus.
    let (mut region, mut switched) = (on.region, on.switched);
    let mut keyed = false;
    for command in &commands {
        (region, switched) = followed(command, region, switched);
    }
    if response.has_focus() {
        let pressed = ui.input_mut(|i| {
            let mut taken = Vec::new();
            i.events.retain(|event| match *event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if picker_key(key, modifiers).is_some() => {
                    taken.extend(picker_key(key, modifiers));
                    false
                }
                _ => true,
            });
            taken
        });
        for key in pressed {
            let Some(command) = key_command(key, on.kind, region, pixel) else {
                continue;
            };
            if command == PickerCommand::Off {
                give_back(&ctx, on.picker);
            } else {
                keyed = true;
            }
            // Keyboard changes end a pointer gesture at its old region.
            drag = None;
            (region, switched) = followed(&command, region, switched);
            commands.push(command);
        }
    }
    let requests = ui.input(|i| {
        let along = |action| i.num_accesskit_action_requests(id, action);
        let custom = |which: i32| {
            i.accesskit_action_requests(id, Action::CustomAction)
                .filter(|r| r.data == Some(ActionData::CustomAction(which)))
                .count()
        };
        [
            (Step::Right, along(Action::Increment)),
            (Step::Left, along(Action::Decrement)),
            (Step::Up, custom(MOVE_UP)),
            (Step::Down, custom(MOVE_DOWN)),
        ]
    });
    for (step, count) in requests {
        for _ in 0..count {
            let command = key_command(PickerKey::Move(step, 1.0), on.kind, region, pixel);
            if let Some(command) = command {
                drag = None;
                keyed = true;
                (region, switched) = followed(&command, region, switched);
                commands.push(command);
            }
        }
    }

    ctx.data_mut(|d| match drag {
        Some(drag) => {
            d.insert_temp(drag_id, drag);
        }
        None => d.remove::<Press>(drag_id),
    });
    // A drag past the area's edge, once past the threshold.
    let pan = (drag.is_some_and(|press| press.dragging) && down)
        .then_some(pos)
        .flatten()
        .map(|pos| pos - on.area.clamp(pos))
        .filter(|past| *past != Vec2::ZERO);

    // The marks, at what was sent this frame.
    let place = match region {
        Region::Point(at) => PickerPlace::Point {
            at: pos2(at[0], at[1]),
            colour: on.colour,
        },
        Region::Area(area) => PickerPlace::Area(Rect::from_min_max(
            pos2(area[0], area[1]),
            pos2(area[2], area[3]),
        )),
    };
    let zoom = on.photo.width() / on.pixels[0].max(1) as f32;
    let painter = ui.painter_at(on.area);
    let overlay = Overlay::new(&painter, on.photo, zoom);
    PickerMarks::new(place)
        .focused(response.has_focus())
        .paint(&overlay);
    paint_label(&overlay, on, place, response.has_focus());
    // None while A held gives the pointer to the pan.
    let pointer = pos.filter(|_| !on.panning && response.contains_pointer());
    paint_badge(&overlay, on, pointer, area_dragged);
    let reveal = keyed.then(|| revealed(on, region, &overlay)).flatten();

    // To screen readers.
    let words = apertrust_ui::translator(&ctx);
    let mut args = Args::new();
    args.set("picker", on.name);
    let (kind, value) = match region {
        Region::Point([x, y]) => {
            args.set("x", percent(&words, x));
            args.set("y", percent(&words, y));
            ("picker-photo-point", "picker-photo-at-point")
        }
        Region::Area([left, top, right, bottom]) => {
            args.set("left", percent(&words, left));
            args.set("top", percent(&words, top));
            args.set("right", percent(&words, right));
            args.set("bottom", percent(&words, bottom));
            ("picker-photo-area", "picker-photo-at-area")
        }
    };
    let label = words.text_with(kind, Some(&args));
    let value = words.text_with(value, Some(&args));
    let [up, down] = ["picker-photo-up", "picker-photo-down"].map(|id| words.text(id));
    response.widget_info(|| WidgetInfo::labeled(Role::Group, true, &label));
    ctx.accesskit_node_builder(id, |node| {
        node.set_value(value);
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);
        node.set_custom_actions([
            CustomAction {
                id: MOVE_UP,
                description: up,
            },
            CustomAction {
                id: MOVE_DOWN,
                description: down,
            },
        ]);
    });
    PhotoPicked {
        commands,
        pan,
        reveal,
    }
}

/// The label naming the picker's tool by its area or point at `place`, `focused` while its
/// node has the keyboard's focus, inside the photo area (`docs/pickers.md`, On the photo):
/// decorative, no target, and nothing while the mark is wholly out of view.
fn paint_label(overlay: &Overlay, on: &OnPhoto, place: PickerPlace, focused: bool) {
    if on.label.is_empty() {
        return;
    }
    let mark = match place {
        PickerPlace::Area(area) => Labelled::Area { area, focused },
        PickerPlace::Point { at, .. } => Labelled::Point(at),
    };
    overlay.mark_label(mark, on.label, on.area);
}

/// The pointer's badge, the picker's icon beside `pointer` while it is over the photo area,
/// hidden while an area is `dragged`, the label then saying whose it is (`docs/pickers.md`,
/// On the photo). Drawn, not a system pointer, so it trails the pointer by a frame.
fn paint_badge(overlay: &Overlay, on: &OnPhoto, pointer: Option<Pos2>, dragged: bool) {
    if let Some(pointer) = pointer.filter(|&at| !dragged && on.area.contains(at)) {
        overlay.badge(on.badge, pointer, on.area);
    }
}

/// The box the mark at `region` needs in view, its reach round it on the `overlay`, in the
/// photo's coordinates, where it leaves the photo area: None while it is all in view.
fn revealed(on: &OnPhoto, region: Region, overlay: &Overlay) -> Option<Rect> {
    let mark = match region {
        Region::Point([x, y]) => to_screen(on.photo, [x, y, x, y]),
        Region::Area(area) => to_screen(on.photo, area),
    };
    let needed = mark.expand(overlay.reach(Point::Active, Weight::Normal));
    let size = on.photo.size();
    let photo = |at: Pos2| ((at - on.photo.min) / size).to_pos2();
    (!on.area.contains_rect(needed))
        .then(|| Rect::from_min_max(photo(needed.min), photo(needed.max)))
}

/// What a press at `at`, in the photo's coordinates, `pos` on the screen, with `modifiers`
/// held, on the Mac where `mac`, sends into `commands`, and the drag it starts: Ctrl alone
/// (Cmd on the Mac) switches point and area on a picker that takes both, and is a plain
/// press carrying `command` on one that takes one kind (decision 105); the Mac's
/// Control-click is a right-click, resetting an area; with a point, the point moves there; with an area, a press within
/// [`CORNER`] points of its nearest edge on each axis takes that corner, the opposite one
/// held, as darktable's (`darkroom.c:4050-4062`), and one elsewhere starts a new area round
/// it, the click's area, of the size on screen it has at Fit, where it is
/// [`Region::pressed`]'s (decision 105).
fn pressed_on(
    commands: &mut Vec<PickerCommand>,
    on: &OnPhoto,
    at: [f32; 2],
    pos: Pos2,
    modifiers: Modifiers,
    mac: bool,
) -> Option<Drag> {
    let keys = pick_keys(modifiers);
    if mac_right_click(modifiers, mac) {
        if matches!(on.region, Region::Area(_)) {
            commands.push(PickerCommand::Reset);
        }
        return None;
    }
    if on.kind == PickerKind::PointArea && command_alone(modifiers, mac) {
        commands.push(PickerCommand::Switch);
        return None;
    }
    let area = match on.region {
        Region::Point(_) => {
            commands.push(PickerCommand::Point {
                at,
                done: false,
                keys,
            });
            return Some(Drag::Point { last: at });
        }
        Region::Area(area) => area,
    };
    let rect = to_screen(on.photo, area);
    // The nearest edge on each axis, darktable's `MIN(dx0, dx1) < hx`; the corner held is
    // the opposite one.
    let (left, right) = ((pos.x - rect.left()).abs(), (pos.x - rect.right()).abs());
    let (top, bottom) = ((pos.y - rect.top()).abs(), (pos.y - rect.bottom()).abs());
    if left.min(right) < CORNER && top.min(bottom) < CORNER {
        let held = [
            if left < right { area[2] } else { area[0] },
            if top < bottom { area[3] } else { area[1] },
        ];
        return Some(Drag::Area { held, last: None });
    }
    // As tall on screen as it is wide, as the session's one-shot pick makes it, and as
    // wide on screen as at Fit, where `scale` is exactly 1, so the area is `pressed`'s
    // there to the bit.
    let size = on.photo.size();
    let aspect = size.x / size.y.max(1.0);
    let scale = on.fitted.x / size.x.max(1.0);
    let half = [PRESSED * scale, PRESSED * aspect * scale];
    let [left, top, right, bottom] = Region::around(at, half);
    let (from, to) = ([left, top], [right, bottom]);
    commands.push(PickerCommand::Area {
        from,
        to,
        done: false,
        keys,
    });
    Some(Drag::Area {
        held: at,
        last: Some((from, to)),
    })
}

/// Whether a press at `pos` would take a corner of the area on the photo, strictly within
/// [`CORNER`] of an edge on each axis, as [`pressed_on`] takes it.
fn on_corner(on: &OnPhoto, pos: Pos2) -> bool {
    let Region::Area(area) = on.region else {
        return false;
    };
    let rect = to_screen(on.photo, area);
    let near = |a: f32, b: f32, at: f32| (at - a).abs().min((at - b).abs()) < CORNER;
    near(rect.left(), rect.right(), pos.x) && near(rect.top(), rect.bottom(), pos.y)
}

/// The picker's place after `command`, at `region` with `switched` its other kind's, and
/// its other kind's then: a switch trades them.
fn followed(command: &PickerCommand, region: Region, switched: Region) -> (Region, Region) {
    let next = follow(command, region, switched);
    if *command == PickerCommand::Switch {
        (next, region)
    } else {
        (next, switched)
    }
}

/// The custom actions' ids: Move up and Move down.
const MOVE_UP: i32 = 0;
const MOVE_DOWN: i32 = 1;

/// A direction a key moves the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Left,
    Right,
    Up,
    Down,
}

/// What a key does to the picker on the photo.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PickerKey {
    /// The point or the whole area, by this many frame pixels.
    Move(Step, f32),
    /// An area's right or bottom edge, by this many frame pixels.
    Resize(Step, f32),
    Switch,
    Reset,
    Off,
}

/// The picker's key `key` with `modifiers` is, if any: the arrows alone or with Shift,
/// with Alt and maybe Shift, Space, Delete, Backspace and Esc alone.
fn picker_key(key: Key, modifiers: Modifiers) -> Option<PickerKey> {
    let command = modifiers.command || modifiers.ctrl || modifiers.mac_cmd;
    if command {
        return None;
    }
    let step = match key {
        Key::ArrowLeft => Some(Step::Left),
        Key::ArrowRight => Some(Step::Right),
        Key::ArrowUp => Some(Step::Up),
        Key::ArrowDown => Some(Step::Down),
        _ => None,
    };
    let by = if modifiers.shift { 10.0 } else { 1.0 };
    if let Some(step) = step {
        return Some(if modifiers.alt {
            PickerKey::Resize(step, by)
        } else {
            PickerKey::Move(step, by)
        });
    }
    if modifiers.shift || modifiers.alt {
        return None;
    }
    match key {
        Key::Space => Some(PickerKey::Switch),
        Key::Delete | Key::Backspace => Some(PickerKey::Reset),
        Key::Escape => Some(PickerKey::Off),
        _ => None,
    }
}

/// The command `key` sends with the picker of `kind` at `region`, a frame pixel `pixel` of
/// the photo on each axis: a move or a resize held inside the photo, an area at least a
/// pixel each way. A resize of a point sends nothing, and so does a switch of a picker that
/// takes one kind (decision 105).
fn key_command(
    key: PickerKey,
    kind: PickerKind,
    region: Region,
    pixel: [f32; 2],
) -> Option<PickerCommand> {
    let keys = PickKeys::default();
    let delta = |step: Step, by: f32| match step {
        Step::Left => [-by * pixel[0], 0.0],
        Step::Right => [by * pixel[0], 0.0],
        Step::Up => [0.0, -by * pixel[1]],
        Step::Down => [0.0, by * pixel[1]],
    };
    Some(match (key, region) {
        (PickerKey::Move(step, by), Region::Point(at)) => {
            let [dx, dy] = delta(step, by);
            PickerCommand::Point {
                at: [(at[0] + dx).clamp(0.0, 1.0), (at[1] + dy).clamp(0.0, 1.0)],
                done: true,
                keys,
            }
        }
        (PickerKey::Move(step, by), Region::Area([left, top, right, bottom])) => {
            let [dx, dy] = delta(step, by);
            // The whole area, held inside the photo at its size.
            let dx = dx.clamp(-left, 1.0 - right);
            let dy = dy.clamp(-top, 1.0 - bottom);
            PickerCommand::Area {
                from: [left + dx, top + dy],
                to: [right + dx, bottom + dy],
                done: true,
                keys,
            }
        }
        (PickerKey::Resize(step, by), Region::Area([left, top, right, bottom])) => {
            let [dx, dy] = delta(step, by);
            let right = (right + dx).clamp((left + pixel[0]).min(1.0), 1.0);
            let bottom = (bottom + dy).clamp((top + pixel[1]).min(1.0), 1.0);
            PickerCommand::Area {
                from: [left, top],
                to: [right, bottom],
                done: true,
                keys,
            }
        }
        (PickerKey::Resize(..), Region::Point(_)) => return None,
        (PickerKey::Switch, _) if kind == PickerKind::PointArea => PickerCommand::Switch,
        (PickerKey::Switch, _) => return None,
        (PickerKey::Reset, _) => PickerCommand::Reset,
        (PickerKey::Off, _) => PickerCommand::Off,
    })
}

/// `to`, moved where needed to be at least a frame pixel from `from` on each axis, inside
/// the photo.
pub fn at_least_a_pixel(from: [f32; 2], to: [f32; 2], pixel: [f32; 2]) -> [f32; 2] {
    let apart = |a: f32, b: f32, least: f32| {
        if (b - a).abs() >= least {
            b
        } else if b >= a && a + least <= 1.0 || a - least < 0.0 {
            a + least
        } else {
            a - least
        }
    };
    [
        apart(from[0], to[0], pixel[0]),
        apart(from[1], to[1], pixel[1]),
    ]
}

/// An area in the photo's coordinates, left, top, right and bottom, on the screen.
fn to_screen(photo: Rect, [left, top, right, bottom]: [f32; 4]) -> Rect {
    let at = |x: f32, y: f32| photo.min + vec2(x * photo.width(), y * photo.height());
    Rect::from_min_max(at(left, top), at(right, bottom))
}

/// A share of the photo, 0 to 1, as a whole percentage in the language's numbers.
fn percent(words: &Translator, share: f32) -> String {
    words.format_number(f64::from(share) * 100.0, 0, false)
}

/// The colour a reading's frame part measured, its mean as the screen shows it; clear
/// where it measured none.
pub fn sampled(reading: Option<&PickerReading>) -> Color32 {
    let Some([r, g, b]) = reading.and_then(|r| r.frame).map(|frame| frame.rgb[0]) else {
        return Color32::TRANSPARENT;
    };
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(byte(r), byte(g), byte(b))
}

/// How long the pointer or the keys rest before a pick is read out.
pub const RESTS: Duration = Duration::from_millis(500);

/// What the live region reads out of a pick (`docs/pickers.md`, Accessibility): once a
/// reading includes the last command that moved or pressed the picker and the picker has
/// rested [`RESTS`], each control the pick changed with its value, from the values that
/// follow, or where it changed none the colour the frame measured.
#[derive(Debug, Default)]
pub struct Said {
    /// The picker its pick and its words are about.
    picker: Option<PickerRef>,
    rest: Option<Rest>,
    text: String,
}

/// A pick waiting to be read out.
#[derive(Debug)]
struct Rest {
    /// When the picker last moved, and by which command.
    at: Instant,
    number: u64,
    /// The picker's tool's values when its gesture began.
    before: Vec<(&'static str, ControlValue)>,
}

impl Said {
    /// What the live region says now.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The picker on is `active`, as the mirror shows it: another, or none, as it turns off
    /// or the photo is left, forgets the pick waiting and what was said.
    pub fn follow(&mut self, active: Option<PickerRef>) {
        if active != self.picker {
            *self = Said {
                picker: active,
                ..Said::default()
            };
        }
    }

    /// The command numbered `number`, which applies a reading, was sent at `now` for
    /// `picker`, the values shown being `values`: the moves of one gesture are read out
    /// once, against the values before its first.
    pub fn moved(&mut self, number: u64, now: Instant, picker: PickerRef, values: &Values) {
        self.follow(Some(picker));
        let before = match self.rest.take() {
            Some(rest) => rest.before,
            None => tool_values(picker, values),
        };
        self.rest = Some(Rest {
            at: now,
            number,
            before,
        });
    }

    /// The picker rested again at `now` with nothing new to measure: the pick waiting, if
    /// any, waits from then, for the reading it already awaits.
    pub fn rested(&mut self, now: Instant) {
        if let Some(rest) = &mut self.rest {
            rest.at = now;
        }
    }

    /// Reads the pick out where it is due at `now`, from `reading` and the values shown;
    /// returns how long until it is due, for a frame then.
    pub fn update(
        &mut self,
        now: Instant,
        reading: Option<&PickerReading>,
        values: &Values,
        tools: &[&dyn ProcessingTool],
        words: &Translator,
    ) -> Option<Duration> {
        let rest = self.rest.as_ref()?;
        let due = rest.at + RESTS;
        if now < due {
            return Some(due - now);
        }
        let reading = reading.filter(|r| r.through >= rest.number)?;
        let rest = self.rest.take()?;
        self.text = said(reading, &rest.before, values, tools, words);
        None
    }
}

/// The controls of `picker`'s tool, as `values` show them.
pub fn tool_values(picker: PickerRef, values: &Values) -> Vec<(&'static str, ControlValue)> {
    let PickerRef::Tool { tool, .. } = picker else {
        return Vec::new();
    };
    values
        .tools
        .iter()
        .find(|state| state.tool == tool)
        .map(|state| {
            state
                .controls
                .iter()
                .map(|c| (c.control, c.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// What `reading` read out says: for a picker with a Last pick row, that row's words
/// ([`LastPickWords::said`]); else each control its pick wrote that differs from `before`,
/// with its value, or the frame's mean colour.
pub fn said(
    reading: &PickerReading,
    before: &[(&'static str, ControlValue)],
    values: &Values,
    tools: &[&dyn ProcessingTool],
    words: &Translator,
) -> String {
    // A picker with a Last pick row says that row's words, its warnings first (decision
    // 88).
    if let Some((definition, declared)) = declared(tools, reading.picker) {
        let has_row = (definition.rows())
            .any(|row| matches!(row, Row::LastPick { picker } if *picker == declared.id));
        let picked = reading.tool.as_ref().and_then(|tool| tool.picked.as_ref());
        if has_row
            && let Some(shown) = picked
            && let Some(last) = &shown.last
            && let Some(said) = last_pick_words(words, definition, declared, last, &shown.warnings)
        {
            return said.said();
        }
    }
    let now = tool_values(reading.picker, values);
    let changed: Vec<String> = declared(tools, reading.picker)
        .map(|(definition, declared)| {
            declared
                .writes
                .iter()
                .filter_map(|&control| {
                    let value = now.iter().find(|(c, _)| *c == control)?.1.clone();
                    let was = before.iter().find(|(c, _)| *c == control).map(|(_, v)| v);
                    if was == Some(&value) {
                        return None;
                    }
                    let mut args = Args::new();
                    args.set(
                        "control",
                        words.text(&messages::label(definition.tool.id, control)),
                    );
                    args.set("value", value_said(words, definition, control, &value)?);
                    Some(words.text_with("picker-said-control", Some(&args)))
                })
                .collect()
        })
        .unwrap_or_default();
    if !changed.is_empty() {
        return changed.join(", ");
    }
    let Some(frame) = reading.frame else {
        return String::new();
    };
    let mut args = Args::new();
    for (name, v) in ["red", "green", "blue"].into_iter().zip(frame.rgb[0]) {
        let byte = (f64::from(v).clamp(0.0, 1.0) * 255.0).round();
        args.set(name, words.format_number(byte, 0, false));
    }
    words.text_with("picker-said-rgb", Some(&args))
}

/// A control's value as its row shows it: a number with its row's decimals, signed where
/// its range spans zero, and its unit; an option by its words.
fn value_said(
    words: &Translator,
    definition: &ProcessingDefinition,
    control: &str,
    value: &ControlValue,
) -> Option<String> {
    let tool = definition.tool.id;
    let found = definition.control(control)?;
    match (found.kind, value) {
        (ControlKind::Number { min, max, .. }, ControlValue::Number(value)) => {
            let decimals = definition
                .rows()
                .find_map(|row| match row {
                    Row::Slider(slider) if slider.control == control => Some(slider.decimals),
                    _ => None,
                })
                .unwrap_or(2);
            let signed = min < 0.0 && max > 0.0;
            let number = words.format_number(f64::from(*value), usize::from(decimals), signed);
            Some(match found.unit {
                Some(unit) => {
                    let mut args = Args::new();
                    args.set("value", number);
                    words.text_with(&messages::unit_value(unit), Some(&args))
                }
                None => number,
            })
        }
        (ControlKind::Choice { options, .. }, ControlValue::Choice(index)) => {
            let option = options.get(*index as usize)?;
            Some(words.text(&messages::option(tool, control, option)))
        }
        _ => None,
    }
}

/// The polite live region at `at`, saying `text`: a status that takes no room.
pub fn live_region(ui: &mut egui::Ui, at: Pos2, text: &str) {
    let response = ui.interact(
        Rect::from_min_size(at, Vec2::ZERO),
        photo_id().with("said"),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Status, true, text));
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_live(Live::Polite));
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use apertrust_engine::session::{
        ControlState, FrameReading, PickKeys, PickerCommand, PickerReading, PickerRef, Region,
        ToolState, Values,
    };
    use apertrust_tools::testing::Sample;
    use apertrust_tools::{ControlValue, ProcessingTool};
    use apertrust_ui::{Builtin, Locale, Theme};
    use egui::accesskit::{Action, ActionData, ActionRequest, Role, Toggled, TreeId};
    use egui::os::OperatingSystem;
    use egui::{Color32, Key, Modifiers, PointerButton, Rect, UiBuilder, pos2, vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use super::*;
    use crate::tools::{self, Change, Panels};

    static TOOLS: &[&dyn ProcessingTool] = &[&Sample];

    /// `testing::Sample`'s pickers: brightness's quad, an area; and the row's spread and
    /// show, each a point or an area.
    const BRIGHTNESS: PickerRef = PickerRef::Tool {
        tool: "sample",
        picker: "brightness",
    };
    const SPREAD: PickerRef = PickerRef::Tool {
        tool: "sample",
        picker: "spread",
    };
    const SHOW: PickerRef = PickerRef::Tool {
        tool: "sample",
        picker: "show",
    };

    /// The pickers' buttons, by their full names: Sample has no messages, so its words are
    /// its message ids.
    const QUAD: &str = "tool-sample-picker-brightness-full";
    const SPREAD_BUTTON: &str = "tool-sample-picker-spread-full";
    const SHOW_BUTTON: &str = "tool-sample-picker-show-full";

    /// Where the panel is, and the photo, its frame a pixel a point.
    const PANEL: Rect = Rect {
        min: pos2(0.0, 0.0),
        max: pos2(320.0, 400.0),
    };
    const PHOTO: Rect = Rect {
        min: pos2(340.0, 40.0),
        max: pos2(740.0, 340.0),
    };
    const PIXELS: [u32; 2] = [400, 300];

    const DEFAULT_AREA: Region = Region::Area([0.02, 0.02, 0.98, 0.98]);

    /// Sample's section and the photo, wired as the app wires them: a press on a button
    /// sends its command, the photo shows the picker on and sends what the pointer and the
    /// keys do, every command numbered as the session numbers it. The session's answers
    /// are the test's to give ([`answer`]).
    struct Scene {
        mirror: Mirror,
        said: Said,
        values: Values,
        number: u64,
        sent: Vec<PickerCommand>,
    }

    impl Scene {
        fn send(&mut self, command: PickerCommand, ctx: &egui::Context) {
            let showing = Showing::of(&command, TOOLS);
            self.number += 1;
            showing.sent(
                self.number,
                &mut self.mirror,
                &mut self.said,
                &self.values,
                ctx,
            );
            self.sent.push(command);
        }
    }

    /// Sample's values at its definition's defaults, every control available.
    fn values() -> Values {
        let definition = Sample.definition();
        let controls = definition
            .controls
            .iter()
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
            .collect();
        Values {
            tools: vec![ToolState {
                tool: definition.tool.id,
                enabled: true,
                edited: false,
                hidden: false,
                controls,
                presets: Vec::new(),
                views: Vec::new(),
                computed: Vec::new(),
                readouts: Vec::new(),
            }],
            settings: Vec::new(),
            edited: false,
            through: 0,
            extent: [6000, 4000],
            live: true,
        }
    }

    /// The scene on `os`.
    fn scene_on(os: OperatingSystem) -> Harness<'static, Scene> {
        let mut harness = Harness::builder()
            .with_size(vec2(760.0, 400.0))
            .with_os(os)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
            .build_ui_state(
                |ui, scene: &mut Scene| {
                    let ctx = ui.ctx().clone();
                    let t = apertrust_ui::theme(&ctx);
                    let colours = &t.colors;
                    (ui.painter()).rect_filled(ctx.content_rect(), 0.0, colours.photo_surround);
                    ui.painter().rect_filled(PANEL, 0.0, colours.panel);
                    let pickers = tools::Pickers {
                        drawn: &["sample"],
                        on: scene.mirror.active(),
                        enabled: true,
                        why: None,
                        ..tools::Pickers::NONE
                    };
                    let state = &scene.values.tools[0];
                    let changes = ui
                        .scope_builder(UiBuilder::new().max_rect(PANEL), |ui| {
                            tools::section(
                                ui,
                                &Sample,
                                state,
                                tools::Head {
                                    open: true,
                                    ..tools::Head::default()
                                },
                                &Panels::default(),
                                &|_: &str| false,
                                true,
                                None,
                                &pickers,
                            )
                        })
                        .inner;
                    for change in changes {
                        if let Change::Picker {
                            tool,
                            picker,
                            action,
                            keys,
                        } = change
                        {
                            let picker = PickerRef::Tool { tool, picker };
                            scene.send(pressed(picker, action, keys, Vec::new()), &ctx);
                        }
                    }
                    // A sky over a pale wall over earth, as the wireframe's photo.
                    let bands = [(141, 162, 181), (217, 210, 196), (110, 91, 69)];
                    for (i, (r, g, b)) in bands.into_iter().enumerate() {
                        let third = PHOTO.height() / 3.0;
                        let band = Rect::from_min_size(
                            PHOTO.min + vec2(0.0, i as f32 * third),
                            vec2(PHOTO.width(), third),
                        );
                        (ui.painter()).rect_filled(band, 0.0, Color32::from_rgb(r, g, b));
                    }
                    let shown = (scene.mirror.active()).zip(scene.mirror.region());
                    let Some((picker, region)) = shown else {
                        return;
                    };
                    let PickerRef::Tool { picker: id, .. } = picker else {
                        return;
                    };
                    let words = apertrust_ui::translator(&ctx);
                    let name = name(&words, Sample.definition(), id);
                    let label = label(&words, Sample.definition(), id);
                    let kind = Sample.definition().picker(id).expect("declared").kind;
                    let on = OnPhoto {
                        picker,
                        kind,
                        area: PHOTO,
                        photo: PHOTO,
                        fitted: PHOTO.size(),
                        pixels: PIXELS,
                        region,
                        switched: scene.mirror.switched(region),
                        colour: sampled(scene.mirror.reading()),
                        name: &name,
                        label: &label,
                        badge: badge(Sample.definition(), id),
                        panning: false,
                    };
                    for command in on_photo(ui, &on).commands {
                        scene.send(command, &ctx);
                    }
                    live_region(ui, PHOTO.left_bottom(), scene.said.text());
                },
                Scene {
                    mirror: Mirror::default(),
                    said: Said::default(),
                    values: values(),
                    number: 0,
                    sent: Vec::new(),
                },
            );
        harness.run();
        harness
    }

    fn scene() -> Harness<'static, Scene> {
        scene_on(OperatingSystem::Nix)
    }

    /// The session's reading of `picker` at `region`, through `through`, its frame's mean
    /// a warm grey.
    fn reading(picker: PickerRef, region: Region, through: u64) -> PickerReading {
        PickerReading {
            through,
            picker,
            region,
            tool: None,
            frame: Some(FrameReading {
                rgb: [[0.6, 0.5, 0.4]; 3],
                lab: [[0.0; 3]; 3],
            }),
        }
    }

    /// The session answers every command sent: the active picker at `region`.
    fn answer(harness: &mut Harness<'_, Scene>, region: Region) {
        let scene = harness.state_mut();
        let picker = scene.mirror.active().expect("a picker on");
        let through = scene.number;
        scene
            .mirror
            .apply(through, Some(reading(picker, region, through)));
        harness.run();
    }

    fn sent(harness: &mut Harness<'_, Scene>) -> Vec<PickerCommand> {
        std::mem::take(&mut harness.state_mut().sent)
    }

    fn toggled(harness: &Harness<'_, Scene>, name: &str) -> Option<Toggled> {
        harness
            .get_by_role_and_label(Role::Button, name)
            .accesskit_node()
            .toggled()
    }

    fn on(picker: PickerRef, area: bool, keys: PickKeys) -> PickerCommand {
        PickerCommand::On {
            picker,
            area,
            keys,
            settings: Vec::new(),
            channel: None,
        }
    }

    /// A point on the photo, in the photo's coordinates, on the screen.
    fn at(x: f32, y: f32) -> egui::Pos2 {
        PHOTO.min + vec2(x * PHOTO.width(), y * PHOTO.height())
    }

    /// The picker on the photo, as screen readers find it: a group with a value.
    fn on_the_photo<'h, S>(harness: &'h Harness<'_, S>) -> egui_kittest::Node<'h> {
        harness.get_by(|node| node.role() == Role::Group && node.value().is_some())
    }

    fn shown_on_the_photo<S>(harness: &Harness<'_, S>) -> bool {
        (harness.query_by(|node| node.role() == Role::Group && node.value().is_some())).is_some()
    }

    /// The quad and the row's buttons show their picker on, at once as pressed, and a
    /// press on the one on turns it off; a right-click on a row's button and Ctrl-click
    /// on either turn it on as an area, the keys held riding with it. Turning one on moves
    /// the focus to the photo, where its marks show once the session's reading arrives.
    #[test]
    fn the_panel_s_buttons_show_the_picker_on_and_turn_it_on_and_off() {
        let mut harness = scene();
        for button in [QUAD, SPREAD_BUTTON, SHOW_BUTTON] {
            assert_eq!(toggled(&harness, button), Some(Toggled::False), "{button}");
        }
        // The row's label, at its start.
        harness.get_by_label("tool-sample-group-measure");

        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        assert_eq!(
            sent(&mut harness),
            [on(BRIGHTNESS, false, PickKeys::default())]
        );
        assert_eq!(toggled(&harness, QUAD), Some(Toggled::True));
        assert!(!shown_on_the_photo(&harness));
        answer(&mut harness, DEFAULT_AREA);
        let photo = on_the_photo(&harness);
        assert!(photo.is_focused());
        assert_eq!(
            photo.accesskit_node().label().as_deref(),
            Some("Picker area, tool-sample-control-brightness-label")
        );
        assert_eq!(
            photo.value().as_deref(),
            Some("area from 2%, 2% to 98%, 98%")
        );

        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        assert_eq!(sent(&mut harness), [PickerCommand::Off]);
        assert_eq!(toggled(&harness, QUAD), Some(Toggled::False));
        assert!(!shown_on_the_photo(&harness));

        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click_secondary();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, SHOW_BUTTON)
            .click_modifiers(Modifiers::COMMAND);
        harness.run();
        let command = PickKeys {
            command: true,
            shift: false,
        };
        assert_eq!(
            sent(&mut harness),
            [
                on(SPREAD, true, PickKeys::default()),
                on(SHOW, true, command)
            ]
        );
        assert_eq!(toggled(&harness, SHOW_BUTTON), Some(Toggled::True));
        assert_eq!(toggled(&harness, SPREAD_BUTTON), Some(Toggled::False));
    }

    /// Presses at `from` and releases at `to` on the photo, moving between them.
    fn drag<S>(harness: &mut Harness<'_, S>, from: egui::Pos2, to: egui::Pos2) {
        harness.hover_at(from);
        harness.run();
        harness.drag_at(from);
        harness.run();
        if to != from {
            harness.hover_at(to);
            harness.run();
        }
        harness.event(egui::Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
    }

    /// A point follows a drag at once, each step `done: false` and the release `done:
    /// true`, before any reading includes it.
    #[test]
    fn a_dragged_point_sends_its_steps_then_its_release() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        drag(&mut harness, at(0.25, 0.5), at(0.3, 0.75));
        let point = |at, done| PickerCommand::Point {
            at,
            done,
            keys: PickKeys::default(),
        };
        assert_eq!(
            sent(&mut harness),
            [
                point([0.25, 0.5], false),
                point([0.3, 0.75], false),
                point([0.3, 0.75], true)
            ]
        );
        assert_eq!(
            harness.state().mirror.region(),
            Some(Region::Point([0.3, 0.75]))
        );
        assert_eq!(
            on_the_photo(&harness).value().as_deref(),
            Some("point at 30%, 75%")
        );
    }

    /// A second finger down mid-drag, a touch screen's pinch, ends the drag where it was,
    /// and the fingers' moves pick nothing more.
    #[test]
    fn a_pinch_ends_a_drag_where_it_was() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        let touch = |finger: u64, phase, pos| egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(finger),
            phase,
            pos,
            force: None,
        };
        let (first, second) = (at(0.25, 0.5), at(0.75, 0.5));
        harness.hover_at(first);
        harness.run();
        harness.drag_at(first);
        harness.run();
        harness.event(touch(0, egui::TouchPhase::Start, first));
        harness.event(touch(1, egui::TouchPhase::Start, second));
        harness.run();
        let moved = at(0.2, 0.5);
        harness.event(egui::Event::PointerMoved(moved));
        harness.event(touch(0, egui::TouchPhase::Move, moved));
        harness.run();
        let point = |at, done| PickerCommand::Point {
            at,
            done,
            keys: PickKeys::default(),
        };
        assert_eq!(
            sent(&mut harness),
            [point([0.25, 0.5], false), point([0.25, 0.5], true)]
        );
    }

    /// A press within 6 points of an area's corner takes it, the opposite corner held; one
    /// elsewhere draws a new area round it, 2 % of the photo's width each way and as tall
    /// on screen; a corner taken and let go where it was sends nothing.
    #[test]
    fn an_area_is_dragged_by_its_corner_or_drawn_anew() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let area = |from, to, done| PickerCommand::Area {
            from,
            to,
            done,
            keys: PickKeys::default(),
        };

        let corner = at(0.98, 0.98) + vec2(4.0, -3.0);
        drag(&mut harness, corner, at(0.5, 0.5));
        assert_eq!(
            sent(&mut harness),
            [
                area([0.02, 0.02], [0.5, 0.5], false),
                area([0.02, 0.02], [0.5, 0.5], true)
            ]
        );

        let down = 0.02 * PHOTO.width() / PHOTO.height();
        let new = ([0.75 - 0.02, 0.75 - down], [0.75 + 0.02, 0.75 + down]);
        drag(&mut harness, at(0.75, 0.75), at(0.75, 0.75));
        assert_eq!(
            sent(&mut harness),
            [area(new.0, new.1, false), area(new.0, new.1, true)]
        );
        // The new area's top left: taken and let go where it was.
        let shown = harness.state().mirror.region();
        drag(&mut harness, at(new.0[0], new.0[1]), at(new.0[0], new.0[1]));
        assert_eq!(sent(&mut harness), []);
        assert_eq!(harness.state().mirror.region(), shown);

        // Exactly 6 points from a corner is outside it, as darktable's `<`: a new area.
        answer(&mut harness, DEFAULT_AREA);
        let outside = at(0.02, 0.02) + vec2(-6.0, 2.0);
        drag(&mut harness, outside, outside);
        assert!(matches!(
            sent(&mut harness)[..],
            [PickerCommand::Area { from, .. }, _] if from != [0.98, 0.98]
        ));
    }

    /// Where `pos` on the screen is in the photo's coordinates, as the photo takes a press.
    fn photo_at(pos: egui::Pos2) -> [f32; 2] {
        let p = (pos - PHOTO.min) / PHOTO.size();
        [p.x, p.y]
    }

    fn area(from: [f32; 2], to: [f32; 2], done: bool) -> PickerCommand {
        PickerCommand::Area {
            from,
            to,
            done,
            keys: PickKeys::default(),
        }
    }

    /// The click's area round the photo's middle, as a press there sends it.
    fn clicked(done: bool) -> PickerCommand {
        let [left, top, right, bottom] = Region::pressed([0.5, 0.5], 400.0 / 300.0);
        area([left, top], [right, bottom], done)
    }

    /// A press is a click until the pointer goes more than 6 points from where it went
    /// down, egui's `max_click_dist` (decision 105): on an area picker the click's area
    /// stays as the press sent it, done on the release. Past them the area runs from the
    /// press to the pointer, at least a frame pixel each way, and stays a drag when the
    /// pointer comes back.
    #[test]
    fn an_area_s_press_is_a_click_until_it_moves_past_6_points() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let press = at(0.5, 0.5);
        for by in [vec2(5.0, 0.0), vec2(0.0, -6.0), vec2(-6.0, 0.0)] {
            answer(&mut harness, DEFAULT_AREA);
            drag(&mut harness, press, press + by);
            assert_eq!(
                sent(&mut harness),
                [clicked(false), clicked(true)],
                "{by:?}"
            );
        }

        answer(&mut harness, DEFAULT_AREA);
        let past = press + vec2(7.0, 0.0);
        drag(&mut harness, press, past);
        let pixel = [1.0 / 400.0, 1.0 / 300.0];
        let to = at_least_a_pixel([0.5, 0.5], photo_at(past), pixel);
        assert_eq!(
            sent(&mut harness),
            [
                clicked(false),
                area([0.5, 0.5], to, false),
                area([0.5, 0.5], to, true)
            ]
        );

        // Back to the press after passing the threshold: still a drag.
        answer(&mut harness, DEFAULT_AREA);
        harness.hover_at(press);
        harness.run();
        harness.drag_at(press);
        harness.run();
        harness.hover_at(past);
        harness.run();
        harness.hover_at(press);
        harness.run();
        harness.drop_at(press);
        harness.run();
        let back = at_least_a_pixel([0.5, 0.5], [0.5, 0.5], pixel);
        assert_eq!(
            sent(&mut harness),
            [
                clicked(false),
                area([0.5, 0.5], to, false),
                area([0.5, 0.5], back, false),
                area([0.5, 0.5], back, true)
            ]
        );
    }

    /// Time never counts: a press held still for over a second, egui's click time being
    /// 0.8 s, is a click all the same.
    #[test]
    fn a_press_held_still_is_a_click_however_long() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let press = at(0.5, 0.5);
        harness.hover_at(press);
        harness.run();
        harness.drag_at(press);
        // Each step a quarter of a second.
        for _ in 0..5 {
            harness.step();
        }
        harness.hover_at(press + vec2(3.0, 2.0));
        harness.step();
        harness.drop_at(press + vec2(3.0, 2.0));
        harness.run();
        assert_eq!(sent(&mut harness), [clicked(false), clicked(true)]);
    }

    /// A corner taken moves nothing until the pointer is past the threshold: let go within
    /// it, nothing is sent; past it, the area runs from the opposite corner.
    #[test]
    fn a_corner_moves_only_past_6_points() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let corner = at(0.98, 0.98);
        drag(&mut harness, corner, corner + vec2(-5.0, 0.0));
        assert_eq!(sent(&mut harness), []);
        assert_eq!(harness.state().mirror.region(), Some(DEFAULT_AREA));
        let past = corner + vec2(-7.0, 0.0);
        drag(&mut harness, corner, past);
        assert_eq!(
            sent(&mut harness),
            [
                area([0.02, 0.02], photo_at(past), false),
                area([0.02, 0.02], photo_at(past), true)
            ]
        );
    }

    /// A point stays where the press put it until the pointer is past the threshold, then
    /// follows it.
    #[test]
    fn a_point_follows_only_past_6_points() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        let point = |at, done| PickerCommand::Point {
            at,
            done,
            keys: PickKeys::default(),
        };
        let press = at(0.25, 0.5);
        drag(&mut harness, press, press + vec2(5.0, 0.0));
        assert_eq!(
            sent(&mut harness),
            [point([0.25, 0.5], false), point([0.25, 0.5], true)]
        );
        let past = press + vec2(7.0, 0.0);
        drag(&mut harness, press, past);
        assert_eq!(
            sent(&mut harness),
            [
                point([0.25, 0.5], false),
                point(photo_at(past), false),
                point(photo_at(past), true)
            ]
        );
    }

    /// The threshold is egui's default click distance, a drag only past it; and only the
    /// secondary button itself resets an area, never egui's long press on a touch screen.
    #[test]
    fn the_threshold_is_egui_s_and_a_long_press_resets_nothing() {
        assert_eq!(CLICK, egui::InputOptions::default().max_click_dist);
        let origin = pos2(100.0, 100.0);
        assert!(!dragging(origin, origin + vec2(5.9, 0.0)));
        assert!(!dragging(origin, origin + vec2(0.0, 6.0)));
        assert!(dragging(origin, origin + vec2(6.1, 0.0)));
        let area = Region::Area([0.2, 0.2, 0.4, 0.4]);
        assert!(resets(true, false, area));
        assert!(!resets(true, true, area));
        assert!(!resets(false, false, area));
        assert!(!resets(true, false, Region::Point([0.5, 0.5])));
    }

    /// Clicks `button` in the photo's middle with `modifiers` held.
    fn click<S>(harness: &mut Harness<'_, S>, button: PointerButton, modifiers: Modifiers) {
        harness.hover_at(at(0.5, 0.5));
        harness.run();
        for pressed in [true, false] {
            let event = egui::Event::PointerButton {
                pos: at(0.5, 0.5),
                button,
                pressed,
                modifiers,
            };
            harness.event_modifiers(event, modifiers);
        }
        harness.run();
    }

    const COMMAND: PickKeys = PickKeys {
        command: true,
        shift: false,
    };

    /// A picker that takes one kind keeps it (decision 105): on the area picker Ctrl-click,
    /// Cmd-click on the Mac, is a plain click carrying `command`, making the click's area; a
    /// picker that takes both kinds still switches. A right-click resets an area, as the
    /// Mac's Control-click does, and on a point does nothing, as darktable's.
    #[test]
    fn ctrl_click_is_a_click_on_one_kind_and_a_right_click_resets_an_area() {
        let with_command = |done| match clicked(done) {
            PickerCommand::Area { from, to, done, .. } => PickerCommand::Area {
                from,
                to,
                done,
                keys: COMMAND,
            },
            other => other,
        };
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        click(&mut harness, PointerButton::Secondary, Modifiers::NONE);
        click(&mut harness, PointerButton::Primary, Modifiers::COMMAND);
        assert_eq!(
            sent(&mut harness),
            [
                PickerCommand::Reset,
                with_command(false),
                with_command(true)
            ]
        );

        // On the Mac, Control-click is the right-click, and Cmd-click a click.
        let mut harness = scene_on(OperatingSystem::Mac);
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        click(&mut harness, PointerButton::Primary, Modifiers::CTRL);
        click(&mut harness, PointerButton::Primary, Modifiers::MAC_CMD);
        assert_eq!(
            sent(&mut harness),
            [
                PickerCommand::Reset,
                with_command(false),
                with_command(true)
            ]
        );

        // A picker that takes both kinds, turned on as an area: Ctrl-click switches, and a
        // right-click on its point does nothing.
        for (os, command) in [
            (OperatingSystem::Nix, Modifiers::COMMAND),
            (OperatingSystem::Mac, Modifiers::MAC_CMD),
        ] {
            let mut harness = scene_on(os);
            harness
                .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
                .click_secondary();
            harness.run();
            answer(&mut harness, DEFAULT_AREA);
            sent(&mut harness);
            click(&mut harness, PointerButton::Primary, command);
            assert_eq!(sent(&mut harness), [PickerCommand::Switch], "{os:?}");
            answer(&mut harness, Region::Point([0.5, 0.5]));
            click(&mut harness, PointerButton::Secondary, Modifiers::NONE);
            if os == OperatingSystem::Mac {
                click(&mut harness, PointerButton::Primary, Modifiers::CTRL);
            }
            assert_eq!(sent(&mut harness), [], "{os:?}");
        }
    }

    /// A picker of `kind` alone over the photo at `region`, on `os`, as the Viewer draws it:
    /// the commands it sends.
    fn alone(
        kind: PickerKind,
        region: Region,
        os: OperatingSystem,
    ) -> Harness<'static, Vec<PickerCommand>> {
        let mut harness = Harness::builder()
            .with_size(vec2(760.0, 400.0))
            .with_os(os)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, sent: &mut Vec<PickerCommand>| {
                    let on = OnPhoto {
                        picker: SPREAD,
                        kind,
                        area: PHOTO,
                        photo: PHOTO,
                        fitted: PHOTO.size(),
                        pixels: PIXELS,
                        region,
                        switched: region,
                        colour: Color32::RED,
                        name: "",
                        label: "",
                        badge: Icon::Pick,
                        panning: false,
                    };
                    sent.extend(on_photo(ui, &on).commands);
                },
                Vec::new(),
            );
        harness.run();
        harness
    }

    /// A point picker keeps its point: Ctrl-click, Cmd-click on the Mac, moves the point,
    /// carrying `command`, and Space on its focused node does nothing.
    #[test]
    fn a_point_picker_s_ctrl_click_moves_the_point_and_space_does_nothing() {
        let point = |done| PickerCommand::Point {
            at: [0.5, 0.5],
            done,
            keys: COMMAND,
        };
        for (os, command) in [
            (OperatingSystem::Nix, Modifiers::COMMAND),
            (OperatingSystem::Mac, Modifiers::MAC_CMD),
        ] {
            let mut harness = alone(PickerKind::Point, Region::Point([0.25, 0.25]), os);
            click(&mut harness, PointerButton::Primary, command);
            assert_eq!(
                std::mem::take(harness.state_mut()),
                [point(false), point(true)],
                "{os:?}"
            );
            assert!(on_the_photo(&harness).is_focused());
            harness.key_press(Key::Space);
            harness.run();
            assert_eq!(harness.state()[..], [], "{os:?}");
        }
    }

    /// With the picker on the photo focused: the arrows move it a frame pixel, Shift ten,
    /// Alt with them an area's right or bottom edge, Delete and Backspace reset, and Esc ends
    /// it, the focus back on its button. Each is a completed change. Space switches nothing
    /// on the area picker, which takes one kind (decision 105); it switches one with both
    /// ([`a_switch_or_a_reset_shows_at_once_and_the_keys_after_it_follow`]).
    #[test]
    fn the_keys_move_resize_reset_and_end_it() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, Region::Area([0.25, 0.25, 0.5, 0.5]));
        sent(&mut harness);
        assert!(on_the_photo(&harness).is_focused());
        let (x, y) = (1.0 / 400.0_f32, 1.0 / 300.0_f32);
        let area = |from, to| PickerCommand::Area {
            from,
            to,
            done: true,
            keys: PickKeys::default(),
        };
        harness.key_press(Key::ArrowRight);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
        harness.run();
        harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
        harness.run();
        harness.key_press_modifiers(Modifiers::ALT | Modifiers::SHIFT, Key::ArrowUp);
        harness.run();
        let (left, right) = (0.25 + x, 0.5 + x);
        let (top, bottom) = (0.25 + 10.0 * y, 0.5 + 10.0 * y);
        assert_eq!(
            sent(&mut harness),
            [
                area([left, 0.25], [right, 0.5]),
                area([left, top], [right, bottom]),
                area([left, top], [right + x, bottom]),
                area([left, top], [right + x, bottom - 10.0 * y]),
            ]
        );
        for key in [Key::Space, Key::Delete, Key::Backspace] {
            harness.key_press(key);
            harness.run();
        }
        assert_eq!(
            sent(&mut harness),
            [PickerCommand::Reset, PickerCommand::Reset]
        );
        harness.key_press(Key::Escape);
        harness.run();
        harness.run();
        assert_eq!(sent(&mut harness), [PickerCommand::Off]);
        assert!(!shown_on_the_photo(&harness));
        assert!(
            harness
                .get_by_role_and_label(Role::Button, QUAD)
                .is_focused()
        );
    }

    /// Screen readers move the picker along x with Increment and Decrement, and along y
    /// with its custom actions, Move up and Move down.
    #[test]
    fn screen_readers_move_the_picker_on_the_photo() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        on_the_photo(&harness).increment_accesskit();
        harness.run();
        harness.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: Action::CustomAction,
            target_tree: TreeId::ROOT,
            target_node: photo_id().accesskit_id(),
            data: Some(ActionData::CustomAction(MOVE_DOWN)),
        }));
        harness.run();
        let (x, y) = (1.0 / 400.0_f32, 1.0 / 300.0_f32);
        let point = |at| PickerCommand::Point {
            at,
            done: true,
            keys: PickKeys::default(),
        };
        assert_eq!(
            sent(&mut harness),
            [point([0.5 + x, 0.5]), point([0.5 + x, 0.5 + y])]
        );
    }

    /// The live region is a polite status on the photo.
    fn said(harness: &Harness<'_, Scene>) -> String {
        let live = harness
            .query_all_by_role(Role::Status)
            .find(|node| node.accesskit_node().data().live() == Some(Live::Polite))
            .expect("a live region");
        live.accesskit_node()
            .label()
            .unwrap_or_default()
            .to_string()
    }

    /// A pick is read out once a reading includes the last move and the picker has rested
    /// half a second: each control it changed with its value, else the frame's colour.
    #[test]
    fn a_pick_is_read_out_once_it_has_rested() {
        let words = Translator::new(Locale::EnGb);
        let mut values = values();
        let mut said = Said::default();
        let now = Instant::now();
        said.moved(3, now, BRIGHTNESS, &values);
        // A later step of the same drag keeps what the values were as it began.
        said.moved(4, now + Duration::from_millis(100), BRIGHTNESS, &values);
        let brightness = &mut values.tools[0].controls[0];
        assert_eq!(brightness.control, "brightness");
        brightness.value = ControlValue::Number(0.5);
        let at = |ms| now + Duration::from_millis(ms);
        let read = reading(BRIGHTNESS, DEFAULT_AREA, 4);
        let early = reading(BRIGHTNESS, DEFAULT_AREA, 3);
        let update =
            |said: &mut Said, ms, read| said.update(at(ms), Some(read), &values, TOOLS, &words);
        assert_eq!(
            update(&mut said, 400, &read),
            Some(Duration::from_millis(200))
        );
        assert_eq!(said.text(), "");
        // Rested, but no reading includes the last move yet.
        assert_eq!(update(&mut said, 700, &early), None);
        assert_eq!(said.text(), "");
        assert_eq!(update(&mut said, 700, &read), None);
        assert_eq!(said.text(), "tool-sample-control-brightness-label +0.50 EV");

        // The picker turning off, or another on, forgets what was said and what waits.
        said.follow(Some(BRIGHTNESS));
        assert_ne!(said.text(), "");
        said.follow(None);
        assert_eq!(said.text(), "");
        said.moved(5, at(750), BRIGHTNESS, &values);
        said.follow(Some(SPREAD));
        assert_eq!(update(&mut said, 1400, &read), None);
        assert_eq!(said.text(), "");

        // A pick that changed nothing reads out the colour measured.
        said.moved(5, at(800), SHOW, &values);
        let read = reading(SHOW, Region::Point([0.5, 0.5]), 5);
        assert_eq!(update(&mut said, 1400, &read), None);
        assert_eq!(said.text(), "mean RGB 153, 128, 102");
    }

    /// The scene's live region says it, after the picker has rested.
    #[test]
    fn the_live_region_reads_the_pick_out() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SHOW_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        assert_eq!(said(&harness), "");
        // As the app's frame does: a frame once the rest is over.
        std::thread::sleep(RESTS);
        let scene = harness.state_mut();
        let words = Translator::new(Locale::EnGb);
        let reading = scene.mirror.reading().cloned();
        scene.said.update(
            Instant::now(),
            reading.as_ref(),
            &scene.values,
            TOOLS,
            &words,
        );
        harness.run();
        assert_eq!(said(&harness), "mean RGB 153, 128, 102");
    }

    /// Esc's focus given back to a picker's button is taken by that button within two
    /// frames, never later, nor once a picker turns on again.
    #[test]
    fn the_focus_given_back_lasts_two_frames() {
        let ctx = egui::Context::default();
        let frame = |ctx: &egui::Context| {
            let mut output = ctx.run_ui(Default::default(), |_| {});
            output.textures_delta.clear();
        };
        give_back(&ctx, SPREAD);
        frame(&ctx);
        assert!(!takes_back(&ctx, SHOW));
        assert!(takes_back(&ctx, SPREAD));
        assert!(!takes_back(&ctx, SPREAD));

        give_back(&ctx, SPREAD);
        for _ in 0..3 {
            frame(&ctx);
        }
        assert!(!takes_back(&ctx, SPREAD));

        give_back(&ctx, SPREAD);
        focus_photo(&ctx);
        assert!(!takes_back(&ctx, SPREAD));
    }

    /// A release to where the last move left the picker brings no reading of its own: the
    /// live region reads the pick out from the move's reading once the picker has rested.
    #[test]
    fn a_release_where_the_move_left_it_waits_for_no_reading_of_its_own() {
        let words = Translator::new(Locale::EnGb);
        let ctx = egui::Context::default();
        let values = values();
        let (mut mirror, mut said) = (Mirror::default(), Said::default());
        let send = |number, command: PickerCommand, mirror: &mut Mirror, said: &mut Said| {
            Showing::of(&command, TOOLS).sent(number, mirror, said, &values, &ctx);
        };
        send(
            1,
            on(SHOW, false, PickKeys::default()),
            &mut mirror,
            &mut said,
        );
        mirror.apply(1, Some(reading(SHOW, Region::Point([0.5, 0.5]), 1)));
        let point = |done| PickerCommand::Point {
            at: [0.3, 0.3],
            done,
            keys: PickKeys::default(),
        };
        send(2, point(false), &mut mirror, &mut said);
        send(3, point(true), &mut mirror, &mut said);
        // The session measures the move; the release, unchanged, brings nothing.
        mirror.apply(2, Some(reading(SHOW, Region::Point([0.3, 0.3]), 2)));
        let later = Instant::now() + RESTS + Duration::from_millis(100);
        assert_eq!(
            said.update(later, mirror.reading(), &values, TOOLS, &words),
            None
        );
        assert_eq!(said.text(), "mean RGB 153, 128, 102");
    }

    /// `Switch` and `Reset` show at once where they take the picker, so a move after them,
    /// in the same frame or before their reading, moves that kind: Space then → on a
    /// point moves the area `Switch` went to, and a reset area is darktable's default.
    #[test]
    fn a_switch_or_a_reset_shows_at_once_and_the_keys_after_it_follow() {
        let mut harness = scene();
        harness
            .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
            .click();
        harness.run();
        answer(&mut harness, Region::Point([0.5, 0.5]));
        sent(&mut harness);
        harness.key_press(Key::Space);
        harness.key_press(Key::ArrowRight);
        harness.run();
        let x = 1.0 / 400.0_f32;
        assert_eq!(
            sent(&mut harness),
            [
                PickerCommand::Switch,
                PickerCommand::Area {
                    from: [0.02 + x, 0.02],
                    to: [0.98 + x, 0.98],
                    done: true,
                    keys: PickKeys::default(),
                }
            ]
        );
        // Switched back before any reading: the point it left.
        harness.key_press(Key::Space);
        harness.run();
        assert_eq!(sent(&mut harness), [PickerCommand::Switch]);
        assert_eq!(
            harness.state().mirror.region(),
            Some(Region::Point([0.5, 0.5]))
        );
        harness.key_press(Key::Space);
        harness.key_press(Key::Delete);
        harness.run();
        sent(&mut harness);
        assert_eq!(harness.state().mirror.region(), Some(DEFAULT_AREA));
    }

    #[test]
    fn keyboard_changes_cancel_the_pointer_gesture() {
        for key in [Key::Space, Key::Delete] {
            let mut harness = scene();
            harness
                .get_by_role_and_label(Role::Button, SPREAD_BUTTON)
                .click();
            harness.run();
            answer(&mut harness, Region::Point([0.5, 0.5]));
            sent(&mut harness);
            harness.hover_at(at(0.3, 0.3));
            harness.run();
            harness.drag_at(at(0.3, 0.3));
            harness.run();
            sent(&mut harness);
            harness.key_press(key);
            harness.run();
            assert!(!sent(&mut harness).is_empty());
            let region = harness.state().mirror.region();
            harness.hover_at(at(0.6, 0.6));
            harness.run();
            assert!(sent(&mut harness).is_empty());
            assert_eq!(harness.state().mirror.region(), region);
        }
    }

    /// A press on an area's corner and a move in the same frame take the corner, from
    /// where the press was, and follow the move.
    #[test]
    fn a_press_and_a_move_in_one_frame_take_the_corner_pressed() {
        let mut harness = scene();
        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, DEFAULT_AREA);
        sent(&mut harness);
        let corner = at(0.98, 0.98);
        harness.hover_at(corner);
        harness.run();
        harness.event(egui::Event::PointerButton {
            pos: corner,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.event(egui::Event::PointerMoved(at(0.5, 0.5)));
        harness.run();
        assert_eq!(
            sent(&mut harness),
            [PickerCommand::Area {
                from: [0.02, 0.02],
                to: [0.5, 0.5],
                done: false,
                keys: PickKeys::default(),
            }]
        );
    }

    /// A reading measured before a move never replaces the place the move sent in the
    /// mirror's memory of places: switching away and back before the move's reading comes
    /// returns to the moved place.
    #[test]
    fn an_older_reading_keeps_the_place_a_later_move_sent() {
        let mut mirror = Mirror::default();
        mirror.asked(1, Some((BRIGHTNESS, true)));
        let (old, new) = ([0.1, 0.1, 0.4, 0.4], [0.2, 0.2, 0.6, 0.6]);
        mirror.apply(1, Some(reading(BRIGHTNESS, Region::Area(old), 1)));
        mirror.sent(2, Region::Area(new));
        // The frame before the move, measured late.
        mirror.apply(1, Some(reading(BRIGHTNESS, Region::Area(old), 1)));
        assert_eq!(
            mirror.switched(Region::Point([0.5, 0.5])),
            Region::Area(new)
        );
    }

    /// What the app asked shows until the session answers: a picker on or off, and the
    /// region sent, until a reading through its command; a refused command stops showing.
    #[test]
    fn what_the_app_asked_shows_until_the_session_answers() {
        let mut mirror = Mirror::default();
        mirror.asked(1, Some((SPREAD, false)));
        assert_eq!(mirror.active(), Some(SPREAD));
        assert_eq!((mirror.region(), mirror.area()), (None, Some(false)));
        mirror.apply(1, Some(reading(SPREAD, Region::Point([0.5, 0.5]), 1)));
        mirror.sent(2, Region::Point([0.2, 0.3]));
        assert_eq!(mirror.region(), Some(Region::Point([0.2, 0.3])));
        // A reading of the frame before the move leaves the move shown.
        mirror.apply(1, Some(reading(SPREAD, Region::Point([0.5, 0.5]), 1)));
        assert_eq!(mirror.region(), Some(Region::Point([0.2, 0.3])));
        mirror.apply(2, Some(reading(SPREAD, Region::Point([0.21, 0.3]), 2)));
        assert_eq!(mirror.region(), Some(Region::Point([0.21, 0.3])));

        // Another picker shows on at once, with no region until its reading.
        mirror.asked(3, Some((BRIGHTNESS, true)));
        assert_eq!((mirror.active(), mirror.region()), (Some(BRIGHTNESS), None));
        mirror.refused(3);
        assert_eq!(mirror.active(), Some(SPREAD));

        // A picker turned on that the session turns off again before its first reading,
        // as a change of its tool does, by command 5: the end answers the ask.
        mirror.asked(4, Some((BRIGHTNESS, true)));
        mirror.apply(5, None);
        assert_eq!((mirror.active(), mirror.region()), (None, None));
        mirror.apply(5, Some(reading(SPREAD, Region::Point([0.21, 0.3]), 5)));

        // Off shows at once; a reading from before it changes nothing, and the end
        // confirms it.
        mirror.asked(6, None);
        assert_eq!(mirror.active(), None);
        mirror.apply(3, Some(reading(SPREAD, Region::Point([0.21, 0.3]), 3)));
        assert_eq!(mirror.active(), None);
        mirror.apply(6, None);
        assert_eq!((mirror.active(), mirror.region()), (None, None));
    }

    /// Sample with its spread picker taking a point alone, as RGB levels' do.
    struct PointSample;

    impl ProcessingTool for PointSample {
        fn definition(&self) -> &'static ProcessingDefinition {
            use apertrust_tools::{Picker, ToolDefinition};
            let sample = Sample.definition();
            let pickers = (sample.pickers.iter())
                .map(|picker| match picker.id {
                    "spread" => Picker {
                        kind: PickerKind::Point,
                        ..*picker
                    },
                    _ => *picker,
                })
                .collect::<Vec<_>>();
            Box::leak(Box::new(ProcessingDefinition {
                tool: ToolDefinition {
                    before: sample.tool.before,
                    ..ToolDefinition::processing("sample", &[])
                },
                version: sample.version,
                layers: sample.layers,
                auto: sample.auto,
                switched: sample.switched,
                copy_safe: true,
                in_styles: false,
                controls: sample.controls,
                preset_rows: sample.preset_rows,
                views: sample.views,
                pickers: Box::leak(pickers.into_boxed_slice()),
                facts: None,
            }))
        }

        fn owns(&self) -> &'static [apertrust_tools::Owned] {
            Sample.owns()
        }

        fn bind(
            &self,
            _: &apertrust_tools::ToolValues,
            _: &apertrust_plugin::ImageContext,
            _: &apertrust_tools::Base,
        ) -> Vec<apertrust_tools::Setting> {
            Vec::new()
        }
    }

    /// The viewer bar names the tool and the picker, its control's label on a quad, and
    /// says how to use it by the kinds the picker takes (decision 105): an area picker's
    /// words, a point picker's own sentence, and for one with both until 105.5 its switch,
    /// Cmd on the Mac.
    #[test]
    fn the_viewer_bar_names_the_picker_and_says_how_to_use_it() {
        let words = Translator::new(Locale::EnGb);
        let settings = ToolSettings::default();
        let bar = |picker, area, mac| {
            bar_words(&words, TOOLS, &settings, None, picker, area, mac).unwrap()
        };
        let area = "Click for a small area, or drag one. Drag a corner to resize. Right-click \
                    resets the area. Esc ends.";
        assert_eq!(
            bar(BRIGHTNESS, true, false),
            (
                "tool-sample-title · tool-sample-control-brightness-label".to_owned(),
                area.to_owned()
            )
        );
        // An area picker is never a point, and says no modifier.
        assert_eq!(bar(BRIGHTNESS, false, true).1, area);
        let point: &[&dyn ProcessingTool] = &[&PointSample];
        assert_eq!(
            bar_words(&words, point, &settings, None, SPREAD, false, false)
                .unwrap()
                .1,
            "tool-sample-picker-spread-what Esc ends."
        );
        assert_eq!(
            bar(SPREAD, false, false),
            (
                "tool-sample-title · tool-sample-picker-spread-label".to_owned(),
                "tool-sample-picker-spread-what Ctrl-click switches to an area. Esc ends."
                    .to_owned()
            )
        );
        assert_eq!(
            bar(SPREAD, true, true).1,
            "Drag an area on the photo. Cmd-click switches to a point. Right-click resets the \
             area. Esc ends."
        );
        assert_eq!(
            bar_words(
                &words,
                TOOLS,
                &settings,
                None,
                PickerRef::ColourPicker,
                true,
                false
            ),
            None
        );
    }

    /// Exposure's picker says over the photo what a pick will do, by Picker target as the
    /// app keeps it (decision 88): middle grey by default; with Lightness chosen, the
    /// lightness in the language's numbers, its decimal only where it has one.
    #[test]
    fn the_viewer_bar_says_what_exposure_s_pick_will_do_by_its_target() {
        use apertrust_tools::exposure::Exposure;
        let tools: &[&dyn ProcessingTool] = &[&Exposure];
        let exposure = PickerRef::Tool {
            tool: "exposure",
            picker: "exposure",
        };
        let mut settings = ToolSettings::default();
        let bar = |settings: &ToolSettings, locale| {
            let words = Translator::new(locale);
            bar_words(&words, tools, settings, None, exposure, true, false)
                .unwrap()
                .0
        };
        assert_eq!(
            bar(&settings, Locale::EnGb),
            "Exposure · Make an area middle grey"
        );
        assert_eq!(
            bar(&settings, Locale::EnUs),
            "Exposure · Make an area middle gray"
        );
        settings.set("exposure", "target", ControlValue::Choice(1));
        settings.set("exposure", "lightness", ControlValue::Number(92.0));
        assert_eq!(
            bar(&settings, Locale::EnGb),
            "Exposure · Make an area L 92 %"
        );
        settings.set("exposure", "lightness", ControlValue::Number(62.5));
        assert_eq!(
            bar(&settings, Locale::EnGb),
            "Exposure · Make an area L 62.5 %"
        );
        assert_eq!(
            bar(&settings, Locale::Uk),
            "Експозиція · Зробити ділянку L 62,5 %"
        );
        // Toward a reference, the photo it was measured on by its name; the reference's
        // own picker says what it keeps.
        settings.set("exposure", "target", ControlValue::Choice(2));
        settings.set(
            "exposure",
            "reference_lightness",
            ControlValue::Number(62.0),
        );
        let photo = "/photos/2026/IMG_2041.CR3".to_owned();
        settings.set_text("exposure", "reference_photo", photo);
        assert_eq!(
            bar(&settings, Locale::EnGb),
            "Exposure · Match an area to IMG_2041 (L 62 %)"
        );
        // The photo it was measured on, once open: this photo, in each language.
        let open = std::path::Path::new("/photos/2026/IMG_2041.CR3");
        let words = Translator::new(Locale::EnGb);
        let here = |locale| {
            let words = Translator::new(locale);
            bar_words(&words, tools, &settings, Some(open), exposure, true, false)
                .unwrap()
                .0
        };
        assert_eq!(
            here(Locale::EnGb),
            "Exposure · Match an area to this photo (L 62 %)"
        );
        assert_eq!(
            here(Locale::Uk),
            "Експозиція · Зіставити ділянку з цим знімком (L 62 %)"
        );
        assert_eq!(
            here(Locale::Pl),
            "Ekspozycja · Dopasuj obszar do tego zdjęcia (L 62 %)"
        );
        let elsewhere = std::path::Path::new("/photos/2026/IMG_9999.CR3");
        let (name, _) = bar_words(
            &words,
            tools,
            &settings,
            Some(elsewhere),
            exposure,
            true,
            false,
        )
        .unwrap();
        assert_eq!(name, "Exposure · Match an area to IMG_2041 (L 62 %)");
        let reference = PickerRef::Tool {
            tool: "exposure",
            picker: "reference",
        };
        let (name, _) = bar_words(&words, tools, &settings, None, reference, true, false).unwrap();
        assert_eq!(
            name,
            "Exposure · Drag over the area to keep as the reference"
        );
        // A reference whose photo is not known, as `--conf` gives one: its L alone.
        settings.clear("exposure", "reference_photo");
        assert_eq!(
            bar(&settings, Locale::EnGb),
            "Exposure · Match an area to L 62 %"
        );
    }

    /// A reference's photo is asked after off the caller's thread: one there is not gone,
    /// one missing is; the references are asked after once a photo opens. Each answer asks
    /// for a frame.
    #[test]
    fn a_reference_s_photo_is_found_gone_off_the_interface_s_thread() {
        let here = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let missing = format!("{here}.not-there");
        let mut photos = ReferencePhotos::new();
        assert!(!photos.photo_opened(None));
        assert!(photos.photo_opened(Some(3)));
        assert!(!photos.photo_opened(Some(3)));
        // Each answer asks for a frame, so it is drawn without waiting for input.
        let ctx = egui::Context::default();
        let repaints = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = repaints.clone();
        ctx.set_request_repaint_callback(move |_| {
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        photos.check(here.clone(), &ctx);
        photos.check(missing.clone(), &ctx);
        let started = Instant::now();
        while !photos.gone(&missing) && started.elapsed() < Duration::from_secs(5) {
            photos.take();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(photos.gone(&missing));
        assert!(!photos.gone(&here));
        let started = Instant::now();
        // egui asks once for requests made before the frame they ask for.
        while repaints.load(std::sync::atomic::Ordering::SeqCst) == 0
            && started.elapsed() < Duration::from_secs(5)
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(repaints.load(std::sync::atomic::Ordering::SeqCst) >= 1);
    }

    /// The live region reads Exposure's pick out as its Last pick row says it, its
    /// warnings first, in place of each control the pick changed (decision 88).
    #[test]
    fn the_live_region_says_the_last_pick_row_s_words() {
        use apertrust_engine::session::ToolReading;
        use apertrust_tools::exposure::Exposure;
        use apertrust_tools::pick::{LastPick, Patch, Short, Stats};
        let stats = Stats {
            mean: [0.15; 4],
            min: [0.15; 4],
            max: [0.15; 4],
        };
        let shown = Shown {
            last: Some(LastPick {
                before: 2.0,
                after: 12.0,
                exposure: 17.2,
                patches: [
                    Patch::of_lch([2.0, 0.0, 0.0]),
                    Patch::of_lch([12.0, 0.0, 0.0]),
                ],
            }),
            warnings: PickWarnings {
                clipped: true,
                short: Some(Short {
                    exposure: 18.0,
                    reached: 12.0,
                    target: 50.0,
                    black: false,
                }),
            },
            ..Shown::default()
        };
        let reading = PickerReading {
            through: 3,
            picker: PickerRef::Tool {
                tool: "exposure",
                picker: "exposure",
            },
            region: Region::Area([0.2, 0.2, 0.4, 0.4]),
            tool: Some(ToolReading {
                input: Some(stats),
                output: None,
                picked: Some(shown),
            }),
            frame: None,
        };
        let words = Translator::new(Locale::EnGb);
        let tools: &[&dyn ProcessingTool] = &[&Exposure];
        assert_eq!(
            super::said(&reading, &[], &values(), tools, &words),
            "The area has clipped pixels. Exposure at +18 EV: the area is L 12 %, not 50 %. \
             Last pick L 2 % → L 12 % · +17.20 EV"
        );
    }

    /// The Colour picker at `region` over the photo area `PHOTO`, the photo shown at
    /// `zoom` times what fits it about its centre, as the Viewer shows a zoomed photo: the
    /// whole photo's rectangle that much larger. The state keeps every command sent, the
    /// last frame's pan and the last box a key asked the view to show.
    fn zoomed(zoom: f32, region: Region) -> Harness<'static, (Region, PhotoPicked)> {
        Harness::builder()
            .with_size(vec2(760.0, 400.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, (region, picked): &mut (Region, PhotoPicked)| {
                    let photo = Rect::from_center_size(PHOTO.center(), PHOTO.size() * zoom);
                    let on = OnPhoto {
                        picker: PickerRef::ColourPicker,
                        kind: PickerKind::PointArea,
                        area: PHOTO,
                        photo,
                        fitted: PHOTO.size(),
                        pixels: PIXELS.map(|side| (side as f32 * zoom) as u32),
                        region: *region,
                        switched: *region,
                        colour: Color32::RED,
                        name: "",
                        label: LABEL,
                        badge: Icon::PickExposure,
                        panning: false,
                    };
                    let now = on_photo(ui, &on);
                    picked.commands.extend(now.commands);
                    picked.pan = now.pan;
                    picked.reveal = now.reveal.or(picked.reveal);
                },
                (region, PhotoPicked::default()),
            )
    }

    /// The Colour picker's area at `area` over [`zoomed`]'s photo at 400%, its node with the
    /// focus, after Shift+→ moved it ten frame pixels right: where the view must move to
    /// show it.
    fn shifted_right(area: [f32; 4]) -> Option<Rect> {
        let mut harness = zoomed(4.0, Region::Area(area));
        focus_photo(&harness.ctx);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
        harness.run();
        harness.state().1.reveal
    }

    /// Whether A is held, the commands sent and the picker's region.
    type Pointing = (bool, Vec<PickerCommand>, Region);

    /// The crosshair over the photo while a picker is on, from either door, the move
    /// pointer over an area's corner; while A is held the pan has the pointer: a drag under
    /// way ends where it was, as its release would, no crosshair shows and a press sends
    /// nothing (`docs/cursors.md`, On the photo).
    #[test]
    fn the_crosshair_shows_and_a_held_gives_the_pointer_to_the_pan() {
        let mut harness = Harness::builder()
            .with_size(vec2(760.0, 400.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, (panning, sent, region): &mut Pointing| {
                    let on = OnPhoto {
                        picker: PickerRef::ColourPicker,
                        kind: PickerKind::PointArea,
                        area: PHOTO,
                        photo: PHOTO,
                        fitted: PHOTO.size(),
                        pixels: PIXELS,
                        region: *region,
                        switched: DEFAULT_AREA,
                        colour: Color32::RED,
                        name: "",
                        label: "",
                        badge: Icon::Pick,
                        panning: *panning,
                    };
                    sent.extend(on_photo(ui, &on).commands);
                },
                (false, Vec::new(), Region::Point([0.5, 0.5])),
            );
        let pointer =
            |harness: &Harness<'_, Pointing>| harness.output().platform_output.cursor_icon;
        let at = PHOTO.center();
        harness.hover_at(at);
        harness.run();
        assert_eq!(pointer(&harness), CursorIcon::Crosshair);
        harness.drag_at(at);
        harness.run();
        harness.hover_at(at + vec2(40.0, 0.0));
        harness.run();
        harness.state_mut().1.clear();
        // A goes down mid-drag: the point's move ends, done, where it was.
        harness.state_mut().0 = true;
        harness.run();
        let ended = std::mem::take(&mut harness.state_mut().1);
        assert!(
            matches!(ended[..], [PickerCommand::Point { done: true, .. }]),
            "{ended:?}"
        );
        harness.drop_at(at + vec2(40.0, 0.0));
        harness.run();
        assert_ne!(pointer(&harness), CursorIcon::Crosshair);
        harness.drag_at(at);
        harness.run();
        harness.hover_at(at + vec2(-40.0, 20.0));
        harness.run();
        harness.drop_at(at + vec2(-40.0, 20.0));
        harness.run();
        assert_eq!(harness.state().1, []);
        // An area: the move pointer over a corner a press would take.
        harness.state_mut().0 = false;
        harness.state_mut().2 = Region::Area([0.25, 0.25, 0.75, 0.75]);
        let corner = PHOTO.min + PHOTO.size() * 0.25 + vec2(2.0, 2.0);
        harness.hover_at(corner);
        harness.run();
        assert_eq!(pointer(&harness), CursorIcon::Move);
        harness.hover_at(at);
        harness.run();
        assert_eq!(pointer(&harness), CursorIcon::Crosshair);
    }

    #[test]
    fn a_picker_area_moved_past_the_edge_pulls_the_view() {
        // At 400% about the centre the area shows 0.375 to 0.625 of the photo each way, a
        // frame pixel 1/1600 of its width. Well inside, a key moves no view.
        assert_eq!(shifted_right([0.45, 0.45, 0.55, 0.55]), None);
        // To 6 points from the edge: the box with the mark's reach round it, to bring in.
        let reveal = shifted_right([0.55, 0.45, 0.615, 0.55]).expect("a box to show");
        let moved = Rect::from_min_max(pos2(0.55625, 0.45), pos2(0.62125, 0.55));
        assert!(reveal.contains_rect(moved), "{reveal:?}");
        assert!(reveal.right() > 0.625, "{reveal:?}");
        // Its reach, the same each way.
        let reach = moved.left() - reveal.left();
        assert!(
            (reveal.right() - moved.right() - reach).abs() < 1e-5,
            "{reveal:?}"
        );
    }

    /// The label [`zoomed`]'s picker carries.
    const LABEL: &str = "Exposure";

    /// Where the label `text` was drawn on the photo in the last frame.
    fn label_at<S>(harness: &Harness<'_, S>, text: &str) -> Option<Rect> {
        let shapes = &harness.output().shapes;
        shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.visual_bounding_rect())
            }
            _ => None,
        })
    }

    /// Where the pointer's badge was drawn in the last frame: the image on the photo.
    fn badge_at<S>(harness: &Harness<'_, S>) -> Option<Rect> {
        let shapes = &harness.output().shapes;
        shapes
            .iter()
            .rev()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh)
                    if clipped.clip_rect == PHOTO
                        && mesh.texture_id != egui::TextureId::default() =>
                {
                    Some(mesh.calc_bounds())
                }
                _ => None,
            })
    }

    /// Sample's first rows, its preset row and its brightness quad, its preset row's Camera
    /// option turning on its spread picker: one picker on a quad and one on a preset row.
    fn quad_and_preset_row() -> &'static ProcessingDefinition {
        use apertrust_tools::{PresetRow, ToolDefinition};
        let sample = Sample.definition();
        let preset_rows = Box::leak(Box::new([PresetRow {
            id: "look",
            options: &["untouched", "grainy", "camera"],
            controls: &["brightness", "grain", "size"],
            pickers: &[("camera", "spread")],
        }]));
        let pickers = (sample.pickers.iter())
            .filter(|picker| ["brightness", "spread"].contains(&picker.id))
            .copied()
            .collect::<Vec<_>>();
        Box::leak(Box::new(ProcessingDefinition {
            tool: ToolDefinition {
                before: sample.tool.before,
                ..ToolDefinition::processing("sample", &[])
            },
            version: sample.version,
            layers: sample.layers,
            auto: sample.auto,
            switched: sample.switched,
            copy_safe: true,
            in_styles: false,
            controls: sample.controls,
            preset_rows,
            views: &[],
            pickers: Box::leak(pickers.into_boxed_slice()),
            facts: None,
        }))
    }

    /// The label names the picker's tool, and the picker where the tool has more than one
    /// a person can turn on; the badge is the picker's cursor's icon, else the pipette.
    #[test]
    fn the_label_names_the_tool_and_the_badge_shows_the_picker_s_icon() {
        use apertrust_tools::exposure::Exposure;
        use apertrust_tools::white_balance::WhiteBalance;
        let words = Translator::new(Locale::EnGb);
        assert_eq!(
            label(&words, Sample.definition(), "spread"),
            "tool-sample-title · tool-sample-picker-spread-label"
        );
        assert_eq!(
            label(&words, WhiteBalance.definition(), "spot"),
            "White balance"
        );
        assert_eq!(
            label(&words, Exposure.definition(), "reference"),
            "Exposure · Reference"
        );
        // A picker a preset row's option turns on counts as one a person can turn on.
        let both = quad_and_preset_row();
        assert_eq!(
            label(&words, both, "spread"),
            "tool-sample-title · tool-sample-picker-spread-label"
        );
        // A picker's name that repeats its tool's is said once, in every language.
        assert_eq!(label(&words, Exposure.definition(), "exposure"), "Exposure");
        let words = Translator::new(Locale::Uk);
        let title = words.text(&messages::title("exposure"));
        assert_eq!(label(&words, Exposure.definition(), "exposure"), title);
        assert_eq!(
            badge(WhiteBalance.definition(), "spot"),
            Icon::PickWhiteBalance
        );
        assert_eq!(badge(Exposure.definition(), "exposure"), Icon::PickExposure);
        assert_eq!(badge(Sample.definition(), "spread"), Icon::Pick);
    }

    /// The label shows while the picker is on, whatever turned it on, and goes with it;
    /// the badge sits below and right of the pointer over the photo, flips near its right
    /// and bottom edges, and hides while an area is dragged, the label following the area.
    #[test]
    fn the_label_and_the_badge_show_while_the_picker_is_on() {
        let mut harness = scene();
        let text = "tool-sample-title · tool-sample-control-brightness-label";
        harness.hover_at(at(0.5, 0.5));
        harness.run();
        assert_eq!(label_at(&harness, text), None);
        assert_eq!(badge_at(&harness), None);

        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        answer(&mut harness, Region::Area([0.2, 0.3, 0.5, 0.6]));
        // Above the area's top-left handle.
        let drawn = label_at(&harness, text).expect("the label");
        assert!(drawn.bottom() <= at(0.2, 0.3).y - 5.0, "{drawn:?}");
        harness.hover_at(at(0.5, 0.5));
        harness.run();
        let badge = badge_at(&harness).expect("the badge");
        assert!(
            (badge.min - (at(0.5, 0.5) + vec2(12.0, 12.0))).length() < 1.0,
            "{badge:?}"
        );
        // Near the right and bottom edges, on the pointer's other side.
        harness.hover_at(at(0.99, 0.99));
        harness.run();
        let badge = badge_at(&harness).expect("the badge");
        assert!(badge.max.x < at(0.99, 0.99).x && badge.max.y < at(0.99, 0.99).y);

        // Dragging a new area: no badge, the label following the area at once.
        harness.hover_at(at(0.7, 0.6));
        harness.run();
        harness.drag_at(at(0.7, 0.6));
        harness.run();
        harness.hover_at(at(0.9, 0.8));
        harness.run();
        assert_eq!(badge_at(&harness), None);
        let drawn = label_at(&harness, text).expect("the label");
        // Sample's long words cross the photo's right edge left-aligned: right-aligned
        // to the right-hand handles.
        let handles = at(0.9, 0.8).x + 5.0;
        assert!((drawn.right() - handles).abs() < 2.0, "{drawn:?}");
        harness.drop_at(at(0.9, 0.8));
        harness.run();
        // kittest's drop takes the pointer away; back over the photo, the badge is back.
        harness.hover_at(at(0.5, 0.5));
        harness.run();
        assert!(badge_at(&harness).is_some());

        harness.get_by_role_and_label(Role::Button, QUAD).click();
        harness.run();
        assert_eq!(label_at(&harness, text), None);
        assert_eq!(badge_at(&harness), None);
    }

    #[test]
    fn the_label_is_not_drawn_while_the_area_is_out_of_view() {
        let mut harness = zoomed(4.0, Region::Area([0.1, 0.1, 0.2, 0.2]));
        harness.run();
        assert_eq!(label_at(&harness, LABEL), None);
        let mut harness = zoomed(4.0, Region::Area([0.45, 0.45, 0.55, 0.55]));
        harness.run();
        assert!(label_at(&harness, LABEL).is_some());
    }

    /// Where the sampled colour's dot, the point's centre, was drawn in the last frame.
    fn dot(harness: &Harness<'_, (Region, PhotoPicked)>) -> Option<Pos2> {
        let shapes = &harness.output().shapes;
        shapes.iter().find_map(|clipped| match &clipped.shape {
            egui::Shape::Circle(circle) if circle.fill == Color32::RED => Some(circle.center),
            _ => None,
        })
    }

    #[test]
    fn the_marks_stay_on_their_photo_point_at_every_zoom() {
        let point = [0.45, 0.55];
        for zoom in [1.0, 4.0] {
            let mut harness = zoomed(zoom, Region::Point(point));
            harness.run();
            let photo = Rect::from_center_size(PHOTO.center(), PHOTO.size() * zoom);
            let expected = photo.min + vec2(point[0], point[1]) * photo.size();
            let drawn = dot(&harness).expect("the point's dot");
            // On the pixel the point falls in, its centre.
            assert!(
                (drawn - expected).abs().max_elem() <= 0.5,
                "{zoom}: {drawn:?}"
            );
        }
    }

    #[test]
    fn the_marks_are_clipped_to_the_photo_area() {
        // An area larger than the part of the photo shown at 400%.
        let mut harness = zoomed(4.0, Region::Area([0.1, 0.1, 0.9, 0.9]));
        harness.run();
        let shapes = &harness.output().shapes;
        assert!(!shapes.is_empty());
        // Every mark's, the panel's ground under them aside.
        let marks = shapes.iter().filter(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => rect.fill == Color32::TRANSPARENT,
            _ => true,
        });
        for clipped in marks {
            let clip = clipped.clip_rect;
            assert!(PHOTO.contains_rect(clip), "{clip:?}");
        }
    }

    /// The area a click at `press` on the area picker makes over the photo at `photo`, the
    /// photo area [`PHOTO`] and the photo at Fit as large.
    fn clicked_over(photo: Rect, press: Pos2) -> [f32; 4] {
        let zoom = photo.width() / PHOTO.width();
        let mut harness = Harness::builder()
            .with_size(vec2(760.0, 400.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                move |ui, sent: &mut Vec<PickerCommand>| {
                    let on = OnPhoto {
                        picker: BRIGHTNESS,
                        kind: PickerKind::Area,
                        area: PHOTO,
                        photo,
                        fitted: PHOTO.size(),
                        pixels: PIXELS.map(|side| (side as f32 * zoom) as u32),
                        region: DEFAULT_AREA,
                        switched: DEFAULT_AREA,
                        colour: Color32::RED,
                        name: "",
                        label: "",
                        badge: Icon::Pick,
                        panning: false,
                    };
                    sent.extend(on_photo(ui, &on).commands);
                },
                Vec::new(),
            );
        harness.run();
        drag(&mut harness, press, press);
        match harness.state()[..] {
            [
                PickerCommand::Area {
                    from: [left, top],
                    to: [right, bottom],
                    done: false,
                    ..
                },
                _,
            ] => [left, top, right, bottom],
            ref sent => panic!("{sent:?}"),
        }
    }

    /// A click's area keeps its size on screen at every zoom (decision 105): at Fit
    /// [`Region::pressed`]'s, zoomed in as many points wide and as tall on screen, so a
    /// smaller share of the photo, and clamped to the photo at its edge.
    #[test]
    fn a_click_s_area_keeps_its_size_on_screen_at_every_zoom() {
        let press = PHOTO.center() + vec2(10.0, -20.0);
        let at = photo_at(press);
        assert_eq!(
            clicked_over(PHOTO, press),
            Region::pressed(at, 400.0 / 300.0)
        );
        let points = |photo: Rect, [left, top, right, bottom]: [f32; 4]| {
            vec2(
                (right - left) * photo.width(),
                (bottom - top) * photo.height(),
            )
        };
        let fit = points(PHOTO, clicked_over(PHOTO, press));
        assert!((fit.x - 2.0 * 0.02 * PHOTO.width()).abs() < 0.01, "{fit:?}");
        assert!((fit.y - fit.x).abs() < 0.01, "{fit:?}");
        for zoom in [2.0, 8.0] {
            let photo = Rect::from_center_size(PHOTO.center(), PHOTO.size() * zoom);
            let area = clicked_over(photo, press);
            let zoomed = points(photo, area);
            assert!((zoomed - fit).abs().max_elem() < 0.01, "{zoom}: {zoomed:?}");
            assert!(
                ((area[2] - area[0]) * zoom - 2.0 * 0.02).abs() < 1e-5,
                "{zoom}: {area:?}"
            );
        }
        // At 800% with the photo's top-left corner in view, near it: clamped to the photo.
        let photo = Rect::from_min_size(PHOTO.min, PHOTO.size() * 8.0);
        let [left, top, right, bottom] = clicked_over(photo, PHOTO.min + vec2(2.0, 3.0));
        assert_eq!([left, top], [0.0, 0.0]);
        let shown = points(photo, [left, top, right, bottom]);
        assert!((shown.x - (2.0 + 8.0)).abs() < 0.01, "{shown:?}");
        assert!((shown.y - (3.0 + 8.0)).abs() < 0.01, "{shown:?}");
    }

    #[test]
    fn a_picker_drag_past_the_edge_moves_the_view() {
        let mut harness = zoomed(4.0, Region::Point([0.5, 0.5]));
        harness.run();
        harness.hover_at(PHOTO.center());
        harness.drag_at(PHOTO.center());
        harness.run();
        assert_eq!(harness.state().1.pan, None);
        // 30 points past the area's right edge and 10 above its top: the view moves that
        // far each frame, and the point stays at the area's corner.
        let past = pos2(PHOTO.right() + 30.0, PHOTO.top() - 10.0);
        harness.hover_at(past);
        harness.run();
        let picked = &harness.state().1;
        assert_eq!(picked.pan, Some(vec2(30.0, -10.0)));
        let photo = Rect::from_center_size(PHOTO.center(), PHOTO.size() * 4.0);
        let corner = (PHOTO.right_top() - photo.min) / photo.size();
        let last = picked.commands.last();
        let followed = PickerCommand::Point {
            at: [corner.x, corner.y],
            done: false,
            keys: PickKeys::default(),
        };
        assert_eq!(last, Some(&followed));
        // Held there, it keeps moving, the point following the view under the still
        // pointer each frame; let go, it stops.
        let sent = harness.state().1.commands.len();
        harness.run();
        assert_eq!(harness.state().1.pan, Some(vec2(30.0, -10.0)));
        assert!(harness.state().1.commands.len() > sent);
        harness.drop_at(past);
        harness.run();
        assert_eq!(harness.state().1.pan, None);
        // A drag inside the area moves no view.
        let mut harness = zoomed(4.0, Region::Point([0.5, 0.5]));
        harness.hover_at(PHOTO.center());
        harness.drag_at(PHOTO.center());
        harness.run();
        harness.hover_at(PHOTO.center() + vec2(40.0, 20.0));
        harness.run();
        assert_eq!(harness.state().1.pan, None);
    }
}
