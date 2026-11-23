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

//! The tool panel renderer (`docs/gui.md`, Tool panels from definitions; decision 28):
//! any tool's section, drawn from its definition and the session mirror's values, with
//! no code of the tool's own.
//!
//! - The section is `Section::edits()`: its title, its tabs when it has more than one,
//!   Auto drawn disabled until Auto runs, presets opening the tool's presets' pull-down
//!   (`presets.rs`, decision 106), and options holding the app's items, the tool's and the
//!   layout's. Closed, it shows its header alone.
//! - Each row is a design-system component: a slider is a `SliderRow`, its track over
//!   darktable's soft range where the definition gives one, a choice a `Choice` in its
//!   Blended style, darktable's combobox, with its kept marker (decision 42), or
//!   `Segmented`, every option drawn and those the photo does not list disabled with
//!   their reasons (decision 88), a toggle a `ToggleRow`, a plain group a `PlainGroup`, a
//!   collapsible group a `CollapsibleGroup` (decision 100), a group first in its container,
//!   a tool's body, a tab or a group, drawing no space above it, a curve of points a
//!   `Curve`, drawn through
//!   the curve the tool says it renders, and the tone equalizer's graph of its bands an
//!   `Equalizer`, drawn through the curve the tool says it renders through them. The
//!   closed registry's other painted widgets join with the tools that use them.
//! - A preset row (decision 2) is a Blended `Choice`, with no marker, showing the option
//!   the session reports the controls hold, or Custom, which is no option; the options this
//!   photo cannot have are listed disabled, and choosing one applies it as one change.
//! - A slider row naming a lossless view's coordinate (decision 2) shows it as the
//!   session reports it, and sets, resets and keeps it as a control. While a change is
//!   pending, [`Pending::show`] moves the view's controls and its coordinates as the
//!   session will, through the tool's own `view` and `unview`, so a drag follows at once.
//! - The page is darktable's (decision 29): the rows before every tab's, the
//!   open tab's, then those after; rows that follow another control's mode only while it
//!   holds one of their values; a choice's options its operation does not port listed
//!   disabled. A row the session reports hidden on the photo is not drawn, Exposure's
//!   two camera rows where the photo gives them nothing to do, and a toggle the photo
//!   gives a value shows it after its label, "Compensate camera exposure −⅔ EV"
//!   (decision 88). A collapsible group's open state is the app's, kept in settings,
//!   and its "N set" counts the controls it shows that are off their defaults.
//!   darktable's own settings for a tool, which no edit stores, are drawn disabled until
//!   what uses them is built.
//! - A tab with no rows is drawn disabled: nothing is behind it yet.
//! - A colour picker (decision 79; `docs/pickers.md`, The picker button) is the design
//!   system's `PickerButton`: on a slider's quad in its quad mode, and in a line of pickers
//!   as toggle buttons at the row's end, each on while its picker is the active one, and
//!   disabled with the reason on a photo not yet live or open for viewing only. A press
//!   comes back as [`Change::Picker`]. Only the tools whose pick is built draw theirs
//!   ([`PICKING`]): a quad with nothing behind it is not drawn.
//! - Values, defaults and marks come from the mirror, a control that can change nothing
//!   on the photo, or that another control's value disables (decision 30), is drawn
//!   disabled as the mirror reports it, keeping its value, and everything the user does
//!   comes back as a [`Change`]: the session's commands, which the app sends through its
//!   pending values so a slider never jumps back, and the section's own state, which the
//!   app keeps. While a change is pending, [`Pending::show`] moves availability with the
//!   values, through the tool's own `available`.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use apertrust_base::settings::Kind;
use apertrust_engine::session::{
    ControlState, ControlValue, CoordinateState, LabelValue, Mark, PickKeys, PickerRef, ToolState,
};
use apertrust_plugin::ImageContext;
use apertrust_tools::about::{self, Facts};
use apertrust_tools::utility;
use apertrust_tools::{
    ActionEnd, ButtonIcon, ChoiceStyle, Computed, ControlKind, Controls, End, Face, Fact, Fields,
    Group, GroupKind, LineStart, MAX_POINTS, PickerKind, ProcessingDefinition, ProcessingTool,
    PublishedValues, ReadoutDef, Reset, Row, SettingRow, Slider, ToolDefinition, Track, Unit, View,
    Widget, Word, check, messages,
};
use apertrust_ui::components::{
    self as parts, ActionRow, ButtonState, ButtonsAction, ButtonsRow, Choice, ChoiceAction,
    CollapsibleGroup, Curve, CurveAction, EndAction, EndItem, Equalizer, EqualizerAction, HueCurve,
    HueCurveAction, IconButton, LineAction, LineRow, PickerAction, PickerButton, PlainGroup,
    ReadoutRow, Reference, ReferenceAction, ReferenceRow, Section, SectionAction, Segmented,
    SliderAction, SliderRow, ToggleRow, WarningRow, Wheel,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Translator, popup, translator};
use eframe::egui;
use eframe::egui::accesskit::{Action as AccessAction, ActionData, CustomAction};

use crate::actions::{self, ActionId, Placing};
use crate::pickers::Picked;
use crate::settings::ToolSettings;

/// What the user did to a tool's section this frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A control's new value, `done` on the last change of a gesture: `SetControl`. A
    /// lossless view's coordinate is set, reset and kept as a control (decision 2).
    Set {
        tool: &'static str,
        control: &'static str,
        value: ControlValue,
        done: bool,
    },
    /// A control back to its default, by a double-click or Delete: `ResetControl`.
    Reset {
        tool: &'static str,
        control: &'static str,
    },
    /// The marker, keeping or releasing a control, or a coordinate's view's controls: `Keep`.
    Keep {
        tool: &'static str,
        control: &'static str,
        kept: bool,
    },
    /// The header's reset: `ResetTool`.
    ResetTool { tool: &'static str },
    /// Several of these changes as one history step, `Step { source: You }` (decision 78):
    /// the tone equalizer's release of the bands a drag moved, and its reset of the bands.
    Step(Vec<Change>),
    /// One of darktable's buttons at a row's end, run on the values shown: the app sends
    /// each control the tool's `action` writes as `SetControl`, one change.
    Action {
        tool: &'static str,
        action: &'static str,
        values: Controls,
    },
    /// An option of a preset row, setting the controls it writes: `ApplyPreset`.
    ApplyPreset {
        tool: &'static str,
        preset: &'static str,
        option: &'static str,
    },
    /// The section opened or closed: the app's own state; `shift`, Shift held on the
    /// header, does the opposite of Open one processing tool at a time.
    Toggle { tool: &'static str, shift: bool },
    /// Another of the section's tabs: the app's own state.
    Tab { tool: &'static str, tab: usize },
    /// One of its collapsible groups opened or closed: the app's own state, kept in
    /// settings by tool and group, never in the edit.
    Group {
        tool: &'static str,
        group: &'static str,
    },
    /// A colour picker's button pressed, with the keys held: `PickerCommand::On`, or `Off`
    /// for the picker on, since `On` for it would start it again (`docs/pickers.md`,
    /// Against darktable).
    Picker {
        tool: &'static str,
        picker: &'static str,
        action: PickerAction,
        keys: PickKeys,
    },
    /// One of the tool's settings, kept by the app in its settings, never in the edit
    /// (decision 29): `done` on the last change of a gesture, when the app sends the
    /// tool's settings as `PickerCommand::Settings`.
    Setting {
        tool: &'static str,
        setting: &'static str,
        value: ControlValue,
        done: bool,
    },
    /// A reference cleared, ⋯ › Clear Reference (decision 88): both its settings gone from
    /// the app's, which sends the tool's settings as `PickerCommand::Settings`.
    ClearReference {
        tool: &'static str,
        lightness: &'static str,
        photo: &'static str,
    },
    /// An item of the tool's ⋯ chosen (decision 93): its registry action, which the app
    /// runs as its keys run it.
    Run(ActionId),
    /// An item of the tool's presets chosen (decision 106): a preset or Default settings
    /// applied, or one of the items that manage them.
    Preset {
        tool: &'static str,
        item: crate::presets::Item,
    },
}

/// What the shell gives a tool's header each frame (decision 93; `docs/gui.md`, One tool
/// model): its section open or closed, the peek keys, and its ⋯. What the header shows
/// besides follows from the tool's declaration ([`header`]).
#[derive(Debug, Default)]
pub struct Head {
    pub open: bool,
    /// Whether the keys that peek are held (`Section::peek`).
    pub peek: bool,
    /// ⋯'s entries: the app's editing items and those the tool declares at
    /// `Place::Options`, as [`crate::menus::options`] builds them, then, until they are
    /// declared, Collections' own; then the layout's ([`Head::with_layout`]).
    pub options: Vec<popup::Entry>,
    /// Whether ⋯ opens while it holds anything: History's only while the photo has a
    /// history.
    pub enabled: bool,
    /// The open of the photo ⋯'s items act on: another since the last frame closes ⋯, and
    /// a choice from it is dropped, never run on the photo after ([`pressed`]).
    pub photo: Option<u64>,
    /// Where the section is in the workspace's layout, for one of the Tools panel's tabs
    /// (decisions 83 and 96): Alt+↑ and Alt+↓ on its focused header move it where it is in
    /// a tab, and screen readers' actions on the header are the layout's ([`pressed`]).
    /// None for a section that does not move.
    pub layout: Option<Placing>,
    /// Where the tool lives, while Find Tool… lists it among the tools found (decision 96).
    pub found: Option<Found>,
    /// A tool's presets, a processing tool's or a utility's, where the app has a folder for
    /// them (decision 106): its presets button opens their pull-down, else it is drawn
    /// disabled.
    pub presets: Option<crate::presets::Menu>,
}

impl Head {
    /// This header placed in the layout as `layout` says, `Shell::layout_head`'s: where it
    /// is, and ⋯'s layout items after its own, a line between them.
    pub fn with_layout(mut self, layout: Head) -> Head {
        if !self.options.is_empty() && !layout.options.is_empty() {
            self.options.push(popup::Entry::Separator);
        }
        self.options.extend(layout.options);
        self.layout = layout.layout;
        self
    }
}

/// Where a tool Find Tool… found lives, said after its title (`Section::found`): its tab's
/// name, translated, or Hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub place: String,
    pub hidden: bool,
}

/// What a tool's header asked for this frame (decision 93).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pressed {
    /// Open the section, or close it.
    Toggle,
    /// Another of its tabs.
    Tab(usize),
    /// ↺ where it is the tool's own: a processing tool's `ResetTool`, a utility's
    /// `UtilityTool::reset`.
    Reset,
    /// A registry action: ↺'s where it runs one, as History's Reset Edit, or an item of ⋯.
    Run(ActionId),
    /// An item of ⋯ the tool has not declared yet, by its id: Collections'.
    Own(String),
    /// An item of a tool's presets, by its id ([`crate::presets::Item::of`]).
    Preset(String),
}

/// The design system's section for `tool`'s header (decision 93), titled `title`, as `head`
/// has it: ↺ where the declaration's `reset` is not none, presets where its `presets`
/// holds, and ⋯ always, holding `head.options`, drawn disabled while it holds nothing. A
/// processing tool's are an editing tool's three (`Section::edits`); a utility's ↺ is named
/// `reset_name`, and its presets button, where it has one, `presets`. The presets are drawn
/// disabled until [`section`], or `utility::section`, gives them their pull-down (decision
/// 106). ↺, and ⋯'s Reset with it, is drawn disabled unless `resettable`. Its tooltip
/// says `about`, where the tool says anything, and after its title it says where it lives
/// while Find Tool… lists it (`head.found`).
pub fn header<'a>(
    tool: &ToolDefinition,
    title: &'a str,
    reset_name: &'a str,
    presets: &'a str,
    resettable: bool,
    about: Option<parts::About>,
    mut head: Head,
) -> Section<'a> {
    // ⋯'s Reset is ↺, enabled as it is (decision 91).
    for entry in &mut head.options {
        if let popup::Entry::Item(item) = entry
            && item.id == ActionId::SECTION_RESET.0
        {
            item.enabled = resettable;
        }
    }
    let movable = head.layout.is_some_and(|placing| !placing.hidden);
    let mut section = Section::new(title, head.open)
        .peek(head.peek)
        .movable(movable);
    if let Some(about) = about {
        section = section.about(about);
    }
    if let Some(found) = head.found {
        section = section.found(found.place, found.hidden);
    }
    section = match tool.reset {
        Reset::Tool => section.edits().reset(resettable).presets(false),
        Reset::Action(_) | Reset::Own => section.utility_reset(reset_name).reset(resettable),
        Reset::None => section,
    };
    if tool.presets && tool.reset != Reset::Tool {
        section = section.utility_presets(presets).presets(false);
    }
    if head.options.is_empty() {
        section.options(false)
    } else {
        section.options(head.enabled).options_menu(head.options)
    }
}

/// What `tool`'s header, drawn as `out` reports, asked for this frame: its ↺ as the
/// declaration's `reset` says, and the item chosen in ⋯, a registry action or one of its
/// own, its Reset as ↺, unless the photo ⋯ acts on is no longer `photo`, the open of the
/// photo now: ⋯, opened for the photo before, then closes, and its choice is dropped.
/// An item chosen in a tool's presets comes as [`Pressed::Preset`]; an empty ⋯
/// is drawn disabled, nothing behind it yet.
///
/// What it asks of the layout, where `layout` places it in the workspace, goes to the
/// shell apart, by the tool's id ([`layout_asked`]): Alt+↑ and Alt+↓ on its focused
/// header, a layout item of ⋯, and screen readers' actions on its header, which it gives
/// them here; and whether its header has the focus.
pub fn pressed(
    ui: &egui::Ui,
    tool: &ToolDefinition,
    out: &parts::SectionOutput,
    photo: Option<u64>,
    layout: Option<Placing>,
) -> Vec<Pressed> {
    if let Some(placing) = layout {
        moves(ui, tool.id, out, placing);
    }
    let photo_id = ui.make_persistent_id(("options photo", tool.id));
    let before = ui.data(|d| d.get_temp::<Option<u64>>(photo_id));
    ui.data_mut(|d| d.insert_temp(photo_id, photo));
    let replaced = before.is_some_and(|before| before != photo);
    let reset = match tool.reset {
        Reset::Tool | Reset::Own => Some(Pressed::Reset),
        Reset::Action(id) => crate::menus::chosen(id).map(Pressed::Run),
        Reset::None => None,
    };
    let mut pressed: Vec<Pressed> = (out.actions.iter())
        .filter_map(|action| match action {
            SectionAction::Toggle => Some(Pressed::Toggle),
            SectionAction::Tab(tab) => Some(Pressed::Tab(*tab)),
            SectionAction::Reset => reset.clone(),
            SectionAction::Presets
            | SectionAction::Options
            | SectionAction::MoveUp
            | SectionAction::MoveDown => None,
        })
        .collect();
    if replaced {
        for button in [out.options, out.presets].into_iter().flatten() {
            popup::close(ui.ctx(), popup::button_menu(button));
        }
    } else if let Some(id) = &out.preset {
        pressed.push(Pressed::Preset(id.clone()));
    } else if let Some(id) = out.chosen.as_deref() {
        match crate::menus::chosen(id) {
            // The layout's, on this tool, whatever photo is open.
            Some(action) if actions::moves_section(action) => {
                ask_layout(ui.ctx(), |asked| asked.run.push((tool.id, action)));
            }
            // Reset, as ↺ is pressed.
            Some(ActionId::SECTION_RESET) => pressed.extend(reset),
            Some(action) => pressed.push(Pressed::Run(action)),
            None => pressed.push(Pressed::Own(id.to_owned())),
        }
    }
    pressed
}

/// What the headers asked of the workspace's layout this frame (decisions 83 and 96), for
/// the shell to carry out after the frame ([`layout_asked`]).
#[derive(Clone, Debug, Default)]
pub struct LayoutAsked {
    /// The layout's actions to run, each on its tool's section.
    pub run: Vec<(&'static str, ActionId)>,
    /// Each header drawn, by its tool: the one with the focus is the one the registry's
    /// layout actions act on, and the focus may go to one.
    pub headers: Vec<(&'static str, egui::Id)>,
}

/// Where the frame's [`LayoutAsked`] waits in the context's memory.
fn layout_asked_id() -> egui::Id {
    egui::Id::unique("tools layout asked")
}

/// Adds to what the headers asked of the layout this frame.
fn ask_layout(ctx: &egui::Context, ask: impl FnOnce(&mut LayoutAsked)) {
    ctx.data_mut(|d| ask(d.get_temp_mut_or_default::<LayoutAsked>(layout_asked_id())));
}

/// What the headers drawn this frame asked of the layout ([`pressed`]), taken: drawn again,
/// they ask afresh.
pub fn layout_asked(ctx: &egui::Context) -> LayoutAsked {
    ctx.data_mut(|d| d.remove_temp::<LayoutAsked>(layout_asked_id()))
        .unwrap_or_default()
}

/// Screen readers' actions on a header (decisions 83 and 96), AccessKit's custom actions by
/// these ids, each the layout action of the same words.
const MOVES: [(i32, ActionId); 5] = [
    (1, ActionId::SECTION_MOVE_UP),
    (2, ActionId::SECTION_MOVE_DOWN),
    (3, ActionId::SECTION_PIN),
    (4, ActionId::SECTION_HIDE),
    (5, ActionId::SECTION_SHOW),
];

/// The layout's side of the header of `tool`, drawn as `out` reports, where `placing` says
/// it is: Alt+↑ and Alt+↓ on its focused header and screen readers' actions on it, each
/// asked of the shell ([`layout_asked`]), and its header named. Its
/// actions are those that can do something where it is: Move Tool Up and Move Tool Down
/// away from its list's ends, Pin Tool or Unpin Tool and Remove Tool, but Export's, in a
/// tab, Show Tool in Tab hidden.
fn moves(ui: &egui::Ui, tool: &'static str, out: &parts::SectionOutput, placing: Placing) {
    let ctx = ui.ctx();
    let keys = out.actions.iter().filter_map(|action| match action {
        SectionAction::MoveUp => Some(ActionId::SECTION_MOVE_UP),
        SectionAction::MoveDown => Some(ActionId::SECTION_MOVE_DOWN),
        _ => None,
    });
    let mut run: Vec<ActionId> = keys.collect();
    if let Some(header) = out.header {
        let offered = |action: ActionId| match action {
            ActionId::SECTION_MOVE_UP => !placing.hidden && placing.up,
            ActionId::SECTION_MOVE_DOWN => !placing.hidden && placing.down,
            ActionId::SECTION_PIN => !placing.hidden,
            ActionId::SECTION_HIDE => !placing.hidden && placing.removable,
            _ => placing.hidden && placing.show,
        };
        let words = translator(ctx);
        let custom = MOVES.iter().filter(|(_, action)| offered(*action));
        let custom: Vec<CustomAction> = custom
            .map(|(id, action)| CustomAction {
                id: *id,
                description: words.text(actions::section_words(*action, placing)),
            })
            .collect();
        ctx.accesskit_node_builder(header, |node| node.set_custom_actions(custom));
        let requested = ui.input(|i| {
            let requests = i.accesskit_action_requests(header, AccessAction::CustomAction);
            let requests = requests.filter_map(|request| match request.data {
                Some(ActionData::CustomAction(which)) => Some(which),
                _ => None,
            });
            requests.collect::<Vec<i32>>()
        });
        let by_id = |which: i32| MOVES.iter().find(|(id, _)| *id == which);
        run.extend(requested.into_iter().filter_map(by_id).map(|(_, a)| *a));
    }
    ask_layout(ctx, |asked| {
        asked
            .run
            .extend(run.into_iter().map(|action| (tool, action)));
        asked
            .headers
            .extend(out.header.map(|header| (tool, header)));
    });
}

/// The title of the tool or utility `id`, in `words`: a utility's `section-<id>`, a
/// processing tool's its own message.
pub fn title(words: &Translator, id: &str) -> String {
    match utility::utility(id) {
        Some(_) => words.text(&format!("section-{id}")),
        None => words.text(&messages::title(id)),
    }
}

/// What a utility's ↺ is called, `title` its title: for what it resets, "Reset edit" for
/// the one that runs Reset Edit, History's; its own, `<id>-reset`, for a utility whose ↺ is
/// its own, as Export's; otherwise "Reset" and its title.
pub fn reset_name(words: &Translator, tool: &ToolDefinition, title: &str) -> String {
    match tool.reset {
        Reset::Action(id) if id == ActionId::RESET.0 => words.text("history-reset-edit"),
        Reset::Own => words.text(&messages::utility_reset(tool.id)),
        _ => {
            let mut args = Args::new();
            args.set("section", title);
            words.text_with("section-reset", Some(&args))
        }
    }
}

/// The tools whose pickers the panel draws: those whose pick is built, each joining with
/// it (`docs/pickers.md`, The plan): Exposure's, White balance's, turned on from Settings,
/// Colour calibration's, on its readout row, Colour balance RGB's six quads, and AgX's
/// four quads and Auto tune levels' row. Until then a tool's quad, its picker rows'
/// buttons and its patches' input are not drawn, as a quad with nothing behind it is not,
/// and a preset option that turns its picker on is drawn disabled.
pub const PICKING: &[&str] = &[
    "exposure",
    "temperature",
    "channelmixerrgb",
    "colorbalancergb",
    "agx",
];

/// The colour pickers as a section draws them.
#[derive(Clone, Copy)]
pub struct Pickers<'a> {
    /// The tools whose pickers are drawn ([`PICKING`]).
    pub drawn: &'a [&'a str],
    /// The picker on, as the app shows it.
    pub on: Option<PickerRef>,
    /// Whether they can be pressed: the photo live and not open for viewing only.
    pub enabled: bool,
    /// Why not, where there is a reason to tell: the photo opening, or open for viewing
    /// only.
    pub why: Option<&'a str>,
    /// The tools' settings as the app keeps them: a setting kept there is drawn live,
    /// any other disabled at its default until what uses it is built.
    pub settings: &'a ToolSettings,
    /// What the tools' picks showed on the photo, for their patches.
    pub picked: &'a Picked,
    /// The photo open or opening, which a reference measured on it names as this photo.
    pub photo: Option<&'a std::path::Path>,
    /// Whether the photo at a path a reference keeps was found gone (decision 88).
    pub gone: &'a dyn Fn(&str) -> bool,
}

/// No photo found gone, as before any is asked after.
#[cfg(test)]
pub fn none_gone(_: &str) -> bool {
    false
}

/// No tool setting kept and nothing picked.
#[cfg(test)]
static NO_SETTINGS: ToolSettings = ToolSettings::new();
#[cfg(test)]
static NOTHING_PICKED: Picked = Picked::new();

#[cfg(test)]
impl Pickers<'static> {
    /// No tool's pickers drawn.
    pub const NONE: Self = Pickers {
        drawn: &[],
        on: None,
        enabled: true,
        why: None,
        settings: &NO_SETTINGS,
        picked: &NOTHING_PICKED,
        photo: None,
        gone: &none_gone,
    };

    /// The pickers of [`PICKING`]'s tools drawn, none on.
    pub const DRAWN: Self = Pickers {
        drawn: PICKING,
        ..Self::NONE
    };
}

/// Whether a collapsible group is open when settings have not kept it otherwise: closed,
/// as darktable's start (decision 29).
pub const GROUP_OPEN: bool = false;

/// The tab each tool's section shows, the first by default (`docs/gui.md`, A frame).
/// Whether a section is open is the shell's, which settings keep (`Shell::section_open`,
/// by the tool's id), else as the tool declares it at first
/// ([`apertrust_tools::ToolDefinition::open_at_first`]); the tab lasts for the run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Panels {
    tabs: BTreeMap<&'static str, usize>,
}

impl Panels {
    pub fn tab(&self, tool: &str) -> usize {
        self.tabs.get(tool).copied().unwrap_or_default()
    }

    /// Applies a change of tab; the others are the app's.
    pub fn apply(&mut self, change: &Change) {
        if let Change::Tab { tool, tab } = *change {
            self.tabs.insert(tool, tab);
        }
    }
}

/// A change the app has sent the session and no values report includes yet, which the app
/// shows in its mirror until one does, so a slider never jumps back (decision 17).
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    /// A control's new value, or a lossless view's coordinate's (`Command::SetControl`).
    Control {
        tool: &'static str,
        control: &'static str,
        value: ControlValue,
    },
    /// A control or a coordinate back to its default (`Command::ResetControl`).
    Reset {
        tool: &'static str,
        control: &'static str,
    },
    /// A control kept or released, its value as it is, or a coordinate's view's controls
    /// (`Command::Keep`).
    Keep {
        tool: &'static str,
        control: &'static str,
        kept: bool,
    },
    /// Every control of a tool back to its default (`Command::ResetTool`).
    ResetTool { tool: &'static str },
}

impl Pending {
    /// The tool it changes.
    pub fn tool(&self) -> &'static str {
        match *self {
            Pending::Control { tool, .. }
            | Pending::Reset { tool, .. }
            | Pending::Keep { tool, .. }
            | Pending::ResetTool { tool } => tool,
        }
    }

    /// Shows the change in `state`, `tool`'s values as the app shows them, as the session
    /// will report it: a control set shows its value, kept, as the session keeps it, and
    /// one reset its default, released.
    ///
    /// A lossless view (decision 2) moves as the session's edit moves it, through the
    /// tool's pure `view` and `unview` on `image`. A coordinate set or reset writes
    /// `unview` of the view's coordinates as shown, that one replaced, into the view's
    /// controls, each kept, and leaves the other coordinates exactly as shown; a change
    /// that moves one of those controls to another value, through its own row or another
    /// view over it, shows the tool's `view` of the values shown, and so does a reset
    /// of one, which takes its value out of the edit. The coordinates shown stand in for
    /// the session's held ones (`edit::Held`): a report carries those it holds through a
    /// gesture, else its view of the stored values, and it holds them while the view's
    /// controls keep the values it wrote, which is when the coordinates shown stay here. What
    /// the session would refuse, a coordinate outside its range or a control `unview` puts
    /// outside its own, shows nothing, and so does a view the tool does not give, or
    /// whose `view` or `unview` panics: the tool's code never takes the UI thread down.
    ///
    /// Availability follows the values shown as the session's next report will (decision
    /// 30), through the tool's pure `available`, so a control another's value disables
    /// is drawn disabled at once, its value kept ([`follow_availability`]); and so does a
    /// number's range, through the tool's pure `range`, a value the moved range leaves
    /// outside shown clamped, as the session will store it (decision 30,
    /// [`follow_ranges`]).
    pub fn show(&self, tool: &dyn ProcessingTool, state: &mut ToolState, image: &ImageContext) {
        let definition = tool.definition();
        let before = values(state);
        match *self {
            Pending::Control {
                control, ref value, ..
            } => match definition.coordinate(control) {
                Some((view, at)) => {
                    if let Ok(value) = view.coordinates[at].check(value) {
                        set_coordinate(tool, state, view, at, value, image);
                        // Any write turns a switched tool on (decision 32).
                        state.enabled = true;
                    }
                }
                None => {
                    if control_mut(state, control).is_some() {
                        state.enabled = true;
                    }
                    if put(state, control, value.clone(), Some(Mark::Kept)) {
                        follow(tool, state, &[control], None, image);
                    }
                }
            },
            Pending::Reset { control, .. } => match definition.coordinate(control) {
                Some((view, at)) => reset_coordinate(tool, state, view, at, image),
                None => {
                    // A reset takes the control's value out of the edit, which ends every
                    // hold over it however the value shows.
                    let default = self::control(state, control).map(|c| c.default.clone());
                    if let Some(default) = default {
                        put(state, control, default, None);
                        follow(tool, state, &[control], None, image);
                    }
                }
            },
            Pending::Keep { control, kept, .. } => {
                // A view's marks are its controls'.
                let controls = definition
                    .coordinate(control)
                    .map_or(std::slice::from_ref(&control), |(view, _)| view.controls);
                for id in controls {
                    if let Some(shown) = control_mut(state, id) {
                        shown.mark = kept.then_some(Mark::Kept);
                    }
                }
            }
            Pending::ResetTool { .. } => {
                for shown in &mut state.controls {
                    shown.value = shown.default.clone();
                    shown.mark = None;
                }
                for shown in &mut state.views {
                    shown.value = shown.default;
                }
                // The reset turns a switched tool off (decision 32).
                state.enabled = !definition.switched;
            }
        }
        follow_ranges(tool, state, &before);
        coordinate_marks(definition, state);
        follow_availability(tool, state, &before, image);
        follow_warnings(tool, state, &before, image);
        state.edited = edited(tool, state);
    }
}

/// Whether the tool's reset would change what `state` shows rendered, as the session
/// reports it (`ToolState::edited`): a control off its default for the photo, or a
/// switched tool on (decision 32). Its reset is available only then.
pub fn edited(tool: &dyn ProcessingTool, state: &ToolState) -> bool {
    (tool.definition().switched && state.enabled)
        || state.controls.iter().any(|c| c.value != c.default)
}

/// Shows each number's range as the session will report it now that the values `state`
/// shows have moved from `before` (decision 30): first a number a moved range leaves
/// outside, clamped through the function the edit clamps with ([`check::clamp`]), kept, or
/// released at its default; then, where the tool's `range` answers otherwise for the
/// values shown than for `before`, the new range, held to the definition's
/// (`edit::range`). Every other range shows what the session reported. A tool whose
/// `range` panics leaves the values and ranges as reported, as `available` panicking does.
fn follow_ranges(tool: &dyn ProcessingTool, state: &mut ToolState, before: &Controls) {
    let after = values(state);
    if after == *before {
        return;
    }
    let Ok(writes) = catch_unwind(AssertUnwindSafe(|| check::clamp(tool, &after))) else {
        return;
    };
    for (id, value) in writes.iter() {
        let default = control(state, id).map(|shown| shown.default.clone());
        let mark = (default.as_ref() != Some(value)).then_some(Mark::Kept);
        put(state, id, value.clone(), mark);
    }
    let after = values(state);
    for found in tool.definition().controls {
        let id = found.id;
        let moved = catch_unwind(AssertUnwindSafe(|| {
            tool.range(id, &after) != tool.range(id, before)
        }));
        if matches!(moved, Ok(true))
            && let Some(shown) = control_mut(state, id)
        {
            shown.range = apertrust_engine::edit::range(tool, id, &after).map(|(range, _)| range);
        }
    }
}

/// Every control's value as `state` shows it: what the tool's pure functions read.
fn values(state: &ToolState) -> Controls {
    let mut values = Controls::new();
    for shown in &state.controls {
        values.set(shown.control, shown.value.clone());
    }
    values
}

/// Shows each control's availability, and each action's at a row's end, as the session
/// will report it now that the values `state` shows have moved from `before`: where the
/// tool's `available` answers otherwise for them than for `before`, the control or the
/// action shows the new answer, never available for one not ported; every other control shows what the session
/// reported, on the same image context (`PhotoState::image`, the session's `Photo`'s). A
/// tool whose `available` panics leaves every control as reported, as `view` and
/// `unview` panicking leave the values. Either way a choice whose photo lists only the
/// option it now shows, by the options the session reported, is unavailable, as the
/// session will report it (decision 30).
fn follow_availability(
    tool: &dyn ProcessingTool,
    state: &mut ToolState,
    before: &Controls,
    image: &ImageContext,
) {
    let after = values(state);
    if after == *before {
        return;
    }
    let controls = tool.definition().controls;
    let moved = catch_unwind(AssertUnwindSafe(|| {
        let moved = controls.iter().filter_map(|control| {
            let now = tool.available(control.id, &after, image);
            (now != tool.available(control.id, before, image))
                .then_some((control.id, now && !control.unported))
        });
        moved.collect::<Vec<_>>()
    }));
    if let Ok(moved) = moved {
        for (id, available) in moved {
            if let Some(shown) = control_mut(state, id) {
                shown.available = available;
            }
        }
    }
    // An action at a row's end, or a menu's, follows the same rule by its id (decision 107).
    let ids = tool.definition().action_ids();
    let moved = catch_unwind(AssertUnwindSafe(|| {
        let moved = ids.iter().filter_map(|&(id, unported)| {
            let now = tool.available(id, &after, image);
            (now != tool.available(id, before, image)).then_some((id, now && !unported))
        });
        moved.collect::<Vec<_>>()
    }));
    if let Ok(moved) = moved {
        for (id, available) in moved {
            if let Some(shown) = state.actions.iter_mut().find(|shown| shown.id == id) {
                shown.available = available;
            }
        }
    }
    for control in controls {
        if let Some(shown) = control_mut(state, control.id)
            && check::lists_only(control, &shown.options, &shown.value)
        {
            shown.available = false;
        }
    }
}

/// Shows each warning at a slider's end as the session will report it now that the values
/// `state` shows have moved from `before` (decision 107): where the tool's `warning` check
/// answers otherwise for them than for `before`, the new answer; every other warning as
/// the session reported it. A tool whose check panics leaves them as reported.
fn follow_warnings(
    tool: &dyn ProcessingTool,
    state: &mut ToolState,
    before: &Controls,
    image: &ImageContext,
) {
    let after = values(state);
    if after == *before {
        return;
    }
    let ids = tool.definition().warnings();
    let moved = catch_unwind(AssertUnwindSafe(|| {
        let moved = ids.iter().filter_map(|&id| {
            let now = tool.warning(id, &after, image);
            (now != tool.warning(id, before, image)).then_some((id, now))
        });
        moved.collect::<Vec<_>>()
    }));
    if let Ok(moved) = moved {
        for (id, on) in moved {
            if let Some(shown) = state.warnings.iter_mut().find(|shown| shown.id == id) {
                shown.on = on;
            }
        }
    }
}

fn control_mut<'s>(state: &'s mut ToolState, id: &str) -> Option<&'s mut ControlState> {
    state
        .controls
        .iter_mut()
        .find(|control| control.control == id)
}

/// Shows `value` and `mark` for the control `id`; whether its value moved.
fn put(state: &mut ToolState, id: &str, value: ControlValue, mark: Option<Mark>) -> bool {
    let Some(shown) = control_mut(state, id) else {
        return false;
    };
    shown.mark = mark;
    let moved = shown.value != value;
    shown.value = value;
    moved
}

/// What `of` reads of each of the view's coordinates as shown, in the view's order; `None`
/// where the session reports none, as for a view the tool failed to give.
fn coordinates(
    state: &ToolState,
    view: &View,
    of: fn(&CoordinateState) -> f32,
) -> Option<Vec<f32>> {
    view.coordinates
        .iter()
        .map(|coordinate| {
            state
                .views
                .iter()
                .find(|shown| shown.view == view.id && shown.coordinate == coordinate.id)
                .map(of)
        })
        .collect()
}

/// Shows `values` as the view's coordinates.
fn show_coordinates(state: &mut ToolState, view: &View, values: &[f32]) {
    for (coordinate, &value) in view.coordinates.iter().zip(values) {
        let shown = state
            .views
            .iter_mut()
            .find(|shown| shown.view == view.id && shown.coordinate == coordinate.id);
        if let Some(shown) = shown {
            shown.value = value;
        }
    }
}

/// The view's coordinate `at` set to `value`, in its range, the others held as shown.
fn set_coordinate(
    tool: &dyn ProcessingTool,
    state: &mut ToolState,
    view: &View,
    at: usize,
    value: f32,
    image: &ImageContext,
) {
    let Some(mut shown) = coordinates(state, view, |c| c.value) else {
        return;
    };
    if shown[at] == value {
        // As a control set to the value it has: kept, its values as they are.
        for id in view.controls {
            if let Some(control) = control_mut(state, id) {
                control.mark = Some(Mark::Kept);
            }
        }
        return;
    }
    shown[at] = value;
    write(tool, state, view, &shown, image);
}

/// The view's coordinate `at` back to its default, the others held as shown; once every
/// coordinate is at its default, the view's controls are back at theirs, released.
fn reset_coordinate(
    tool: &dyn ProcessingTool,
    state: &mut ToolState,
    view: &View,
    at: usize,
    image: &ImageContext,
) {
    let shown = coordinates(state, view, |c| c.value);
    let (Some(mut shown), Some(defaults)) = (shown, coordinates(state, view, |c| c.default)) else {
        return;
    };
    let unchanged = shown[at] == defaults[at];
    shown[at] = defaults[at];
    if shown == defaults {
        for id in view.controls {
            if let Some(control) = control_mut(state, id) {
                control.value = control.default.clone();
                control.mark = None;
            }
        }
        show_coordinates(state, view, &defaults);
        // Their values are out of the edit, which ends every other view's hold over them.
        follow(tool, state, view.controls, Some(view.id), image);
    } else if !unchanged {
        write(tool, state, view, &shown, image);
    }
}

/// Shows the tool's `unview` of `values` in the view's controls, each kept, and `values`
/// as its coordinates, once each control can take what it is given; nothing where one
/// cannot, as the session refuses it. Every other view over a control whose value moved
/// shows the tool's view of the values, as the session's next report will, since the
/// change ends that view's hold.
fn write(
    tool: &dyn ProcessingTool,
    state: &mut ToolState,
    view: &View,
    values: &[f32],
    image: &ImageContext,
) {
    let definition = tool.definition();
    // A tool's code may panic, as the session guards against: nothing shows, as the
    // session refuses the change, and the UI thread goes on.
    let given = catch_unwind(AssertUnwindSafe(|| tool.unview(view.id, values, image)));
    let Ok(Some(given)) = given else {
        return;
    };
    let mut wrote = Vec::with_capacity(view.controls.len());
    for &id in view.controls {
        match (definition.control(id), given.get(id)) {
            // Also refuses NaN and infinities, which no range contains.
            (Some(control), Some(value)) if control.check(value).is_ok() => {
                wrote.push(value.clone());
            }
            _ => return,
        }
    }
    let mut moved = Vec::with_capacity(view.controls.len());
    for (&id, value) in view.controls.iter().zip(wrote) {
        if put(state, id, value, Some(Mark::Kept)) {
            moved.push(id);
        }
    }
    show_coordinates(state, view, values);
    follow(tool, state, &moved, Some(view.id), image);
}

/// Each view covering one of `moved`, controls whose values have just moved or been reset,
/// but the view `except` they were written through, shows the tool's view of the values
/// shown, as the session reports once such a change has ended the view's hold. A view the
/// report leaves out, as one whose tool failed, is left as it is, its tool not called;
/// so is one the tool does not give, or whose `view` panics.
fn follow(
    tool: &dyn ProcessingTool,
    state: &mut ToolState,
    moved: &[&'static str],
    except: Option<&str>,
    image: &ImageContext,
) {
    let views: Vec<&View> = tool
        .definition()
        .views
        .iter()
        .filter(|view| Some(view.id) != except)
        .filter(|view| view.controls.iter().any(|id| moved.contains(id)))
        .filter(|view| coordinates(state, view, |c| c.value).is_some())
        .collect();
    if views.is_empty() {
        return;
    }
    let values = values(state);
    for view in views {
        // A panic, as in `write`, leaves the coordinates reported.
        let viewed = catch_unwind(AssertUnwindSafe(|| tool.view(view.id, &values, image)));
        if let Ok(Some(viewed)) = viewed
            && viewed.len() == view.coordinates.len()
        {
            show_coordinates(state, view, &viewed);
        }
    }
}

/// Shows each coordinate with the mark its view's controls share, `None` where they
/// differ, as the session reports it.
fn coordinate_marks(definition: &ProcessingDefinition, state: &mut ToolState) {
    let marks: Vec<Option<Mark>> = state
        .views
        .iter()
        .map(|shown| {
            let controls = definition
                .view(shown.view)
                .map_or(&[][..], |view| view.controls);
            let mut marks = controls
                .iter()
                .map(|id| control(state, id).and_then(|c| c.mark));
            let first = marks.next().flatten();
            first.filter(|_| marks.all(|mark| mark == first))
        })
        .collect();
    for (shown, mark) in state.views.iter_mut().zip(marks) {
        shown.mark = mark;
    }
}

/// The curve a band graph draws through the bands from `start` EV to +0 EV, sampled: the
/// tool's curve for `values`, `None` where it says no curve fits them, and an error
/// where its code panicked. A switched tool that is off, not `enabled`, corrects
/// nothing, so its curve is drawn flat (decision 32); on, it is the curve the operation
/// renders, darktable's ripple at the defaults.
fn equalizer_curve(
    tool: &dyn ProcessingTool,
    enabled: bool,
    first: &str,
    values: &Controls,
    start: f32,
) -> std::thread::Result<Option<Vec<f32>>> {
    catch_unwind(AssertUnwindSafe(|| {
        if !enabled {
            return Some(vec![0.0; EQUALIZER_SAMPLES]);
        }
        let curve = tool.curve(first, values)?;
        Some(
            (0..EQUALIZER_SAMPLES)
                .map(|k| curve(start * (1.0 - k as f32 / (EQUALIZER_SAMPLES - 1) as f32)))
                .collect::<Vec<f32>>(),
        )
    }))
}

/// Draws `tool`'s section from its definition, with the values `state` mirrors, its header
/// as `head` has it ([`header`]), each of its collapsible groups open as `groups` says by
/// the group's id. Its reset is drawn
/// disabled unless `editable`, as on a photo open for viewing only, whose controls the
/// session reports unavailable, and for a switched tool unless it is edited, as one off
/// never is (decision 32), whose section screen readers describe as not applied. While
/// `opening` holds the tooltip of a photo opening, read-only until it is live (decision
/// 77), hovering its rows says why. Its colour pickers are drawn as `pickers` says. Its
/// presets button opens the pull-down of `head.presets`, the folder read as it opens, while
/// `editable`, and is drawn disabled otherwise. Returns what the user did.
#[expect(
    clippy::too_many_arguments,
    reason = "the app's own state for the section, each part apart"
)]
pub fn section(
    ui: &mut egui::Ui,
    tool: &dyn ProcessingTool,
    state: &ToolState,
    head: Head,
    panels: &Panels,
    groups: &dyn Fn(&str) -> bool,
    editable: bool,
    opening: Option<&str>,
    pickers: &Pickers,
) -> Vec<Change> {
    let definition = tool.definition();
    let id = definition.tool.id;
    let words = translator(ui.ctx());
    let title = words.text(&messages::title(id));
    let tabs: Vec<String> = definition
        .tool
        .tabs
        .iter()
        .filter_map(|tab| tab.id)
        .map(|tab| words.text(&messages::tab(id, tab)))
        .collect();
    let tabs: Vec<&str> = tabs.iter().map(String::as_str).collect();
    let selected = panels
        .tab(id)
        .min(definition.tool.tabs.len().saturating_sub(1));
    let later: Vec<usize> = (0..definition.tool.tabs.len())
        .filter(|&i| definition.tool.tabs[i].rows.is_empty())
        .collect();
    let later_tip = words.text("section-tab-later");

    // A switched tool's reset is available only while it would change the render
    // (decision 32): off, it has nothing to reset, and shows nothing more than that, but
    // for screen readers, who hear "Not applied".
    let resettable = state.edited || !definition.switched;
    let about = about(&words, id, definition.facts.as_ref());
    let (photo, layout) = (head.photo, head.layout);
    let reset = editable && resettable;
    let presets = head.presets.clone().filter(|_| editable);
    let mut section = header(&definition.tool, &title, "", "", reset, Some(about), head)
        .not_applied(!state.enabled);
    if let Some(menu) = presets {
        let words = std::sync::Arc::clone(&words);
        section = section.presets(true).presets_menu(move || {
            let listed = crate::presets::read(&menu.folder, tool);
            crate::presets::entries(tool, state, &listed, &menu, &words)
        });
    }
    if tabs.len() > 1 {
        section = section
            .tabs(&tabs, selected)
            .disabled_tabs(&later, &later_tip);
    }
    let mut changes = Vec::new();
    let page = Page {
        words: &words,
        tool,
        state,
        editable,
        groups,
        pickers,
        controls: Default::default(),
    };
    let out = section.show(ui, |ui| {
        // While the photo opens, the tip over each row of controls on the edit, where they
        // were drawn last frame, asked before theirs: egui shows one tooltip a layer, the
        // first asked, so hovering a disabled control says why rather than what it does,
        // and the rows that stay usable keep their own. Screen readers hear it as each
        // row's description.
        let rows_id = ui.scope_id().with("opening rows");
        let drawn = ui
            .ctx()
            .data(|data| data.get_temp::<Vec<egui::Rect>>(rows_id));
        if let (Some(tip), Some(drawn)) = (opening, drawn) {
            for (i, rect) in drawn.into_iter().enumerate() {
                let id = rows_id.with(i);
                let row = ui.interact(rect, id, egui::Sense::hover());
                ui.ctx().accesskit_node_builder(id, |node| {
                    node.set_role(egui::accesskit::Role::Group);
                    node.set_description(tip.to_owned());
                });
                row.on_hover_text_at_pointer(tip);
            }
        }
        self::page(ui, &page, selected, &mut changes);
        let rows = page.controls.take();
        ui.ctx().data_mut(|data| data.insert_temp(rows_id, rows));
    });
    for pressed in pressed(ui, &definition.tool, &out, photo, layout) {
        changes.push(match pressed {
            Pressed::Toggle => Change::Toggle {
                tool: id,
                shift: crate::history::shift_clicked(ui),
            },
            Pressed::Tab(tab) => Change::Tab { tool: id, tab },
            Pressed::Reset => Change::ResetTool { tool: id },
            Pressed::Run(action) => Change::Run(action),
            Pressed::Preset(chosen) => match crate::presets::Item::of(&chosen) {
                Some(item) => Change::Preset { tool: id, item },
                None => continue,
            },
            // A processing tool's ⋯ holds its declared items alone.
            Pressed::Own(_) => continue,
        });
    }
    changes
}

/// What the header's tooltip of the tool or utility `id` says (`docs/gui.md`, The header's
/// tooltip), in `words`: its sentence; for a utility ([`apertrust_tools::utility::UTILITIES`]),
/// that it edits nothing and what it measures; for a tool with darktable's `facts`, its
/// kind and what it works on in plain words and darktable's lines word for word; for a tool
/// that is not darktable's, its sentence alone.
pub fn about(words: &Translator, id: &str, facts: Option<&Facts>) -> parts::About {
    let description = words.text(&messages::description(id));
    if let Some(utility) = utility::utility(id) {
        // A utility's own Kind line where its words hold one, as the Metadata tool's, which
        // writes the photos' metadata (decision 110); else that it edits nothing.
        let own = format!("tool-{id}-kind");
        let kind = if words.has(&own) {
            words.text(&own)
        } else {
            words.text(about::Kind::EditsNothing.message())
        };
        return parts::About {
            description,
            kind: Some(kind),
            place: utility
                .measures
                .map(|place| parts::Place::Measures(words.text(place.message()))),
            facts: None,
        };
    }
    let Some(facts) = facts else {
        return parts::About {
            description,
            ..Default::default()
        };
    };
    parts::About {
        description,
        kind: Some(words.text(facts.kind().message())),
        place: facts
            .works_on()
            .map(|place| parts::Place::WorksOn(words.text(place.message()))),
        facts: Some(
            [facts.input, facts.process, facts.output].map(|fact| words.text(&fact.message())),
        ),
    }
}

fn control<'s>(state: &'s ToolState, id: &str) -> Option<&'s ControlState> {
    state.controls.iter().find(|control| control.control == id)
}

/// What the one page drawer ([`page`]) asks of a tool's page (decision 93; `docs/gui.md`,
/// One tool model): a processing tool's ([`section`]) or a utility's
/// ([`crate::utility::section`]). The drawer draws the rows any tool may hold itself,
/// labels, collapsible groups, rows following a value, settings and actions; its kind
/// draws the rest ([`Rows::own`]): a processing tool's controls, pickers and computed
/// values, and every custom row, a utility's by its `UtilityTool::body`.
pub trait Rows {
    fn words(&self) -> &Translator;

    fn definition(&self) -> &ToolDefinition;

    /// Whether its collapsible group `group` is open, as the app keeps it.
    fn group_open(&self, group: &str) -> bool;

    /// The tools' settings, as the app keeps them (decision 29).
    fn settings(&self) -> &ToolSettings;

    /// Whether a picker's words follow its setting `setting` (decision 88).
    fn words_by(&self, _setting: &str) -> bool {
        false
    }

    /// Whether the photo hides `row` (decision 88).
    fn hidden(&self, _row: &Row) -> bool {
        false
    }

    /// How many controls `rows` show now, those inside them included, are off their
    /// defaults for the photo: a collapsible group's "N set".
    fn set_in(&self, _rows: &[Row]) -> usize {
        0
    }

    /// Whether rows following its control `control` show: whether its value is one of
    /// `is` (`Row::When`).
    fn follows(&self, _control: &str, _is: &[&str]) -> bool {
        false
    }

    /// Keeps `rect`, where a row of controls on the edit was drawn, which a photo opening
    /// says over (decision 77).
    fn on_edit(&self, _rect: egui::Rect) {}

    /// Where its slider and choice rows take the scroll wheel: under the pointer, as a
    /// tool's, unless a utility says otherwise, as Export's only while focused (decision
    /// 90; `docs/gui.md`, The scroll wheel over a row).
    fn wheel(&self) -> Wheel {
        Wheel::Pointer
    }

    /// Draws `row`, one of the rows its kind draws.
    fn own(&self, ui: &mut egui::Ui, row: &Row, changes: &mut Vec<Change>);

    /// Draws `facts`, a run of a utility's facts the page shows together, as one list
    /// (decision 110); a processing tool has none.
    fn facts(&self, _ui: &mut egui::Ui, _facts: &[&Fact], _changes: &mut Vec<Change>) {}

    /// The tooltip of its group `id`'s title, where it gives one, as the Metadata tool's
    /// name the standard each group follows (decision 110).
    fn group_tip(&self, _id: &str) -> Option<String> {
        None
    }

    /// The warning button at the end of its group `id`'s title row, where a problem covers
    /// the whole group, as the Metadata tool's view-only photos among those selected
    /// (decision 110): its name and its menu.
    fn group_warning(&self, _id: &str) -> Option<(String, Vec<popup::Entry>)> {
        None
    }

    /// The item `chosen` in the menu of its group `id`'s warning button.
    fn group_warned(&self, _id: &str, _chosen: &str) {}

    /// Why its collapsible group `id`, rows and all, is drawn disabled and closed, where
    /// what it holds is not built yet, as Export's Metadata group.
    fn group_later(&self, _id: &str) -> Option<String> {
        None
    }

    /// The title of its group `id` of the kind `kind`: the one its id and kind imply,
    /// unless a utility declares its own (decision 93).
    fn title(&self, id: &str, kind: GroupKind) -> String {
        let tool = self.definition().id;
        self.words().text(&messages::group_title(tool, id, kind))
    }

    /// The value its setting row `row` shows, and whether it can be changed: the tools'
    /// settings as the app keeps them, one `--conf` gives keeping its value for the run,
    /// its row disabled as the Settings dialog draws an overridden row (decision 43); none
    /// for a setting not kept yet, drawn disabled at its default.
    fn setting(&self, row: &SettingRow) -> Option<(ControlValue, bool)> {
        tool_setting(self.settings(), self.definition().id, row)
    }

    /// The options of its choice setting `row` that cannot be chosen now, by index, and
    /// why: a utility's, as Export's Overwrite where the platform cannot ask first
    /// (decision 93).
    fn disabled(&self, _row: &SettingRow) -> Option<(Vec<usize>, String)> {
        None
    }

    /// What `row`'s tooltip says now in place of its promise, and its description: a
    /// utility's, as Export's Set size from the session's preview (decision 93).
    fn tip(&self, _row: &Row) -> Option<String> {
        None
    }

    /// The note under `row`, as a utility gives the problem that stops its work under the
    /// row it is about (decision 93).
    fn note(&self, _row: &Row) -> Option<Note> {
        None
    }

    /// The fix of the note under `row` was pressed.
    fn fixed(&self, _row: &Row) {}
}

/// `tool`'s setting row `row` as `settings` keep it, and whether it can be changed: one
/// `--conf` gives keeps its value for the run, its row disabled ([`Rows::setting`]).
pub(crate) fn tool_setting(
    settings: &ToolSettings,
    tool: &str,
    row: &SettingRow,
) -> Option<(ControlValue, bool)> {
    let kept = settings.get(tool, row.id)?;
    Some((kept.clone(), !settings.overridden(tool, row.id)))
}

/// A note under a row, which a utility gives (decision 93): a problem that stops its
/// work, in `error`, the one line a row keeps, and the button of the fix it offers, by its
/// words, where it offers one, as Export's "Use $(FILE_NAME)" under its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub fix: Option<String>,
}

/// The one page drawer (decision 93): draws the page of `rows`' tool, the rows before its
/// tabs', its tab `tab`'s, then those after, every tool's alike, one container, the tool's
/// body. Returns what the user did in `changes`.
pub fn page(ui: &mut egui::Ui, rows: &dyn Rows, tab: usize, changes: &mut Vec<Change>) {
    let definition = rows.definition();
    let tab = definition.tabs.get(tab).map_or(&[][..], |tab| tab.rows);
    let top = ui.cursor().top();
    for list in [definition.before, tab, definition.after] {
        draw_list(ui, rows, list, top, changes);
    }
}

/// Draws `list`, the rows of one container, a group's, from where `ui` is now.
fn draw_rows(ui: &mut egui::Ui, rows: &dyn Rows, list: &[Row], changes: &mut Vec<Change>) {
    let top = ui.cursor().top();
    draw_list(ui, rows, list, top, changes);
}

/// Draws `list`'s rows in its container, whose first row starts at `top`: each run of
/// facts the page shows together as one list ([`Rows::facts`], decision 110), every other
/// row by itself.
fn draw_list(
    ui: &mut egui::Ui,
    rows: &dyn Rows,
    list: &[Row],
    top: f32,
    changes: &mut Vec<Change>,
) {
    let mut at = 0;
    while at < list.len() {
        let run: Vec<&Fact> = list[at..]
            .iter()
            .map_while(|row| match row {
                Row::Fact(fact) => Some(fact),
                _ => None,
            })
            .collect();
        if run.is_empty() {
            draw_row(ui, rows, &list[at], top, changes);
            at += 1;
            continue;
        }
        at += run.len();
        let shown: Vec<&Fact> = (run.into_iter())
            .filter(|fact| !rows.hidden(&Row::Fact(**fact)))
            .collect();
        if !shown.is_empty() {
            rows.facts(ui, &shown, changes);
        }
    }
}

/// What a processing tool's rows are drawn from: the words, the tool, the values the
/// mirror shows, and whether each collapsible group is open, by its id.
struct Page<'a> {
    words: &'a Translator,
    tool: &'a dyn ProcessingTool,
    state: &'a ToolState,
    /// Whether the photo takes edits, which its actions need.
    editable: bool,
    groups: &'a dyn Fn(&str) -> bool,
    pickers: &'a Pickers<'a>,
    /// Where each row of controls on the edit was drawn, which a photo opening says over
    /// (decision 77): not a group's header, a tool's setting nor the guides, which stay
    /// usable and keep their own tips.
    controls: std::cell::RefCell<Vec<egui::Rect>>,
}

impl Page<'_> {
    /// The controls `rows` show now, each once, into `into`.
    fn shown(&self, rows: &[Row], into: &mut Vec<&'static str>) {
        for row in rows {
            if let Row::When { control, is, .. } = row
                && !self.follows(control, is)
            {
                continue;
            }
            if let Row::WhenSetting { setting, is, .. } = row
                && !follows_setting(self, setting, is)
            {
                continue;
            }
            if self.hidden(row) {
                continue;
            }
            for &id in row.controls() {
                if !into.contains(&id) {
                    into.push(id);
                }
            }
            self.shown(row.rows(), into);
        }
    }
}

impl Rows for Page<'_> {
    fn words(&self) -> &Translator {
        self.words
    }

    fn definition(&self) -> &ToolDefinition {
        &self.tool.definition().tool
    }

    fn group_open(&self, group: &str) -> bool {
        (self.groups)(group)
    }

    fn settings(&self) -> &ToolSettings {
        self.pickers.settings
    }

    /// Its title, with "(active)" after it while the tool says its settings, as the app
    /// keeps them, make what the group holds act (`ProcessingTool::group_active`), as
    /// darktable's "area colour mapping (active)".
    fn title(&self, id: &str, kind: GroupKind) -> String {
        let tool = self.tool.definition().tool.id;
        let title = self.words.text(&messages::group_title(tool, id, kind));
        let settings = self.pickers.settings.of(tool);
        let active = kind == GroupKind::Collapsible
            && catch_unwind(AssertUnwindSafe(|| self.tool.group_active(id, &settings)))
                .unwrap_or(false);
        if !active {
            return title;
        }
        let mut args = Args::new();
        args.set("group", title);
        self.words.text_with(messages::GROUP_ACTIVE, Some(&args))
    }

    fn words_by(&self, setting: &str) -> bool {
        (self.tool.definition().pickers.iter()).any(|picker| picker.words_by == Some(setting))
    }

    /// Whether the photo hides `row` (decision 88): it names controls, and the mirror
    /// reports every one of them hidden on this photo (`ControlState::visible`).
    fn hidden(&self, row: &Row) -> bool {
        let controls = row.controls();
        !controls.is_empty()
            && controls
                .iter()
                .all(|id| control(self.state, id).is_some_and(|shown| !shown.visible))
    }

    fn set_in(&self, rows: &[Row]) -> usize {
        let mut shown = Vec::new();
        self.shown(rows, &mut shown);
        shown
            .into_iter()
            .filter(|id| control(self.state, id).is_some_and(|c| c.value != c.default))
            .count()
    }

    /// Whether its value in the mirror is one of `is`.
    fn follows(&self, control: &str, is: &[&str]) -> bool {
        let found = self.tool.definition().control(control);
        match (found, self::control(self.state, control)) {
            (Some(found), Some(shown)) => found.is_one_of(&shown.value, is),
            _ => false,
        }
    }

    fn on_edit(&self, rect: egui::Rect) {
        self.controls.borrow_mut().push(rect);
    }

    fn own(&self, ui: &mut egui::Ui, row: &Row, changes: &mut Vec<Change>) {
        let (words, tool, state) = (self.words, self.tool, self.state);
        let definition = tool.definition();
        match row {
            Row::Slider(slider) => slider_row(ui, self, slider, changes),
            Row::Choice { control, style, .. } => {
                choice_row(ui, words, definition, control, *style, state, changes);
            }
            Row::Toggle { control, end } => toggle_row(ui, self, control, end, changes),
            Row::Custom {
                widget: Widget::Curve,
                controls: [points, ..],
            } => curve_row(ui, words, tool, points, state, changes),
            Row::Custom {
                widget: Widget::Equalizer,
                controls,
            } => equalizer_row(ui, words, tool, controls, state, changes),
            Row::Custom {
                widget: Widget::HueCurve,
                controls,
            } => hue_curve_row(ui, words, tool, controls, state, self.wheel(), changes),
            // The closed registry's other painted widgets (the colour wheels, the
            // swatches) join the renderer with the first tool that uses each.
            Row::Custom { .. } => {}
            // A list whatever its style: `ProcessingDefinition::check` refuses segments,
            // which cannot draw Custom or an option disabled yet.
            Row::Preset { preset, .. } => preset_row(ui, self, preset, changes),
            Row::Patches { picker, target } => patches_row(ui, self, picker, target),
            Row::Computed(computed) => {
                computed_row(ui, words, definition.tool.id, computed, state);
            }
            Row::Reference {
                picker,
                lightness,
                photo,
            } => reference_row(ui, self, picker, lightness, photo, changes),
            Row::LastPick { picker } => last_pick_row(ui, self, picker),
            // The three shapes the rows before decision 107 drew keep their own drawing,
            // and so their pixels: icon actions that run after a control, each allowed by
            // the tool's rule, as the button row draws them with the control's
            // availability; one text action alone, drawn at the line's start; or pickers
            // after no start, a label or a readout. Every other line is a line row.
            Row::Line {
                start: LineStart::Control(control),
                readout: None,
                end,
            } if end.iter().all(runs_with_icon)
                && end
                    .iter()
                    .filter_map(End::action)
                    .all(|a| self.allowed(a.id)) =>
            {
                let buttons: Vec<(&'static str, ButtonIcon)> = (end.iter())
                    .filter_map(|item| match item {
                        End::Action(ActionEnd {
                            id,
                            face: Face::Icon(icon),
                            ..
                        }) => Some((*id, *icon)),
                        _ => None,
                    })
                    .collect();
                buttons_row(ui, words, definition, control, &buttons, state, changes);
            }
            Row::Line {
                start: LineStart::None,
                readout: None,
                end:
                    [
                        End::Action(
                            action @ ActionEnd {
                                face: Face::Text, ..
                            },
                        ),
                    ],
            } => action_row(ui, self, action, changes),
            Row::Line {
                start: start @ (LineStart::None | LineStart::Label(_)),
                readout,
                end,
            } if end.iter().all(|item| item.picker().is_some()) => {
                let label = match start {
                    LineStart::Label(id) => Some(*id),
                    _ => None,
                };
                let pickers: Vec<&'static str> = end.iter().filter_map(End::picker).collect();
                pickers_row(ui, self, label, readout.as_ref(), &pickers, changes);
            }
            Row::Line {
                start,
                readout,
                end,
            } => line_row(ui, self, start, readout.as_ref(), end, changes),
            Row::Guides => guides_row(ui, words),
            // The page drawer's own.
            Row::Group(_)
            | Row::When { .. }
            | Row::WhenSetting { .. }
            | Row::Setting(_)
            | Row::Fields(_)
            | Row::Fact(_)
            | Row::Field(_) => {}
        }
    }
}

/// Whether rows following the setting `setting` of `rows`' tool show: whether the option
/// it holds, as the page shows it ([`Rows::setting`]), else its default, is one of `is`
/// (`Row::WhenSetting`).
pub(crate) fn follows_setting(rows: &dyn Rows, setting: &str, is: &[&str]) -> bool {
    let definition = rows.definition();
    let Some(row) = definition.setting(setting) else {
        return false;
    };
    let ControlKind::Choice { options, .. } = row.kind else {
        return false;
    };
    let kept = rows.setting(row).map(|(value, _)| value);
    match kept.unwrap_or_else(|| row.default()) {
        ControlValue::Choice(at) => options
            .get(at as usize)
            .is_some_and(|option| is.contains(option)),
        _ => false,
    }
}

/// Draws `row` of the container whose rows start at `top`: a group with nothing drawn
/// above it there is its first row (decision 100), whatever rows before it the photo hides
/// or a mode leaves out.
fn draw_row(ui: &mut egui::Ui, rows: &dyn Rows, row: &Row, top: f32, changes: &mut Vec<Change>) {
    // A row the photo gives nothing to do is not drawn, and screen readers find no node
    // for it (decision 88).
    if rows.hidden(row) {
        return;
    }
    // Where a row of controls on the edit is drawn, for a photo opening's tip: a line only
    // with a control at its start.
    let on_edit = !matches!(
        row,
        Row::Group(_)
            | Row::When { .. }
            | Row::WhenSetting { .. }
            | Row::Setting(_)
            | Row::Fields(_)
            | Row::Line {
                start: LineStart::None | LineStart::Label(_),
                ..
            }
            | Row::Patches { .. }
            | Row::Computed(_)
            | Row::Reference { .. }
            | Row::LastPick { .. }
            | Row::Guides
            | Row::Field(_)
    );
    let start = ui.cursor().top();
    draw_kind(ui, rows, row, top, changes);
    if on_edit {
        let bottom = ui.min_rect().bottom().max(start);
        rows.on_edit(egui::Rect::from_x_y_ranges(
            ui.max_rect().x_range(),
            start..=bottom,
        ));
    }
    if let Some(note) = rows.note(row) {
        note_row(ui, rows, row, &note);
    }
}

/// The note under `row` (decision 93): its text in `error`, in the small text, wrapped,
/// and the button of its fix, which tells the page when it is pressed.
fn note_row(ui: &mut egui::Ui, rows: &dyn Rows, row: &Row, note: &Note) {
    let t = apertrust_ui::theme(ui.ctx());
    let text = egui::RichText::new(&note.text)
        .size(t.text.small)
        .color(t.colors.error);
    ui.add(egui::Label::new(text).wrap());
    if let Some(fix) = &note.fix
        && parts::Button::new(fix).show(ui).clicked()
    {
        rows.fixed(row);
    }
}

/// Draws `row` of the container whose rows start at `top`: the rows any tool may hold
/// here, the others by `rows`' kind. A mode's rows are its container's.
fn draw_kind(ui: &mut egui::Ui, rows: &dyn Rows, row: &Row, top: f32, changes: &mut Vec<Change>) {
    match row {
        Row::Group(group) => {
            // Nothing drawn in its container yet: its first row.
            let first = ui.cursor().top() <= top;
            group_row(ui, rows, group, first, changes);
        }
        Row::When {
            control,
            is,
            rows: inside,
        } => {
            if rows.follows(control, is) {
                for row in *inside {
                    draw_row(ui, rows, row, top, changes);
                }
            }
        }
        Row::WhenSetting {
            setting,
            is,
            rows: inside,
        } => {
            if follows_setting(rows, setting, is) {
                for row in *inside {
                    draw_row(ui, rows, row, top, changes);
                }
            }
        }
        Row::Setting(setting) => setting_row(ui, rows, setting, changes),
        Row::Fields(fields) => fields_row(ui, rows, fields, changes),
        _ => rows.own(ui, row, changes),
    }
}

/// A control's line of darktable's buttons, orientation's transform: the control's label,
/// a double-click resetting it, and its icon actions, by id and icon, each run on the
/// values shown.
fn buttons_row(
    ui: &mut egui::Ui,
    words: &Translator,
    definition: &ProcessingDefinition,
    id: &'static str,
    buttons: &[(&'static str, ButtonIcon)],
    state: &ToolState,
    changes: &mut Vec<Change>,
) {
    let tool = definition.tool.id;
    let Some(shown) = control(state, id) else {
        return;
    };
    let [label, full, promise] = [
        messages::label(tool, id),
        messages::full(tool, id),
        messages::promise(tool, id),
    ]
    .map(|id| words.text(&id));
    let names: Vec<String> = buttons
        .iter()
        .map(|(action, _)| words.text(&messages::action(tool, action)[1]))
        .collect();
    let icons: Vec<(apertrust_ui::Icon, &str)> = buttons
        .iter()
        .zip(&names)
        .map(|((_, button), name)| (icon(*button), name.as_str()))
        .collect();
    let action = ButtonsRow::new(&label, &icons)
        .name(&full)
        .promise(&promise)
        .enabled(shown.available)
        .show(ui);
    match action {
        Some(ButtonsAction::Reset) => changes.push(Change::Reset { tool, control: id }),
        Some(ButtonsAction::Pressed(index)) => {
            let mut values = Controls::new();
            for shown in &state.controls {
                values.set(shown.control, shown.value.clone());
            }
            changes.push(Change::Action {
                tool,
                action: buttons[index].0,
                values,
            });
        }
        None => {}
    }
}

/// The facts `tool`'s action `action` reads ([`apertrust_tools::ActionEnd::reads`]), each
/// from its publishing tool's own `publish` on the values `states` show for that tool, its
/// defaults included, on `image` (decision 107): what the action is given of the rest of
/// the edit, and nothing else. A fact whose tool is not built or gives none is left out,
/// and so is one whose tool's code panics: the tool's code never takes the UI thread down.
pub fn published(
    tools: &[&dyn ProcessingTool],
    states: &[ToolState],
    tool: &str,
    action: &str,
    image: &ImageContext,
) -> PublishedValues {
    let mut read = PublishedValues::new();
    let found = tools
        .iter()
        .find(|found| found.definition().tool.id == tool);
    let reads = found.map_or(&[][..], |found| found.definition().reads(action));
    for fact in reads {
        let publisher = tools
            .iter()
            .find(|found| found.definition().tool.id == fact.tool);
        let state = states.iter().find(|state| state.tool == fact.tool);
        let (Some(publisher), Some(state)) = (publisher, state) else {
            continue;
        };
        let shown = values(state);
        let value = catch_unwind(AssertUnwindSafe(|| {
            publisher.publish(fact.id, &shown, image)
        }));
        if let Ok(Some(value)) = value {
            read.set(*fact, value);
        }
    }
    read
}

/// What a button's action sends, as `SetControl`s: each control `tool`'s action writes
/// on `values`, the values shown, and `read`, the facts it reads ([`published`]), every
/// write but the last `done: false` and the last
/// `done: true`, so several writes are one gesture, one change and one history step.
/// None on a photo open for viewing only, nor for a tool or action the tools do not
/// have.
pub fn action_writes(
    tools: &[&dyn ProcessingTool],
    tool: &str,
    action: &str,
    values: &Controls,
    read: &PublishedValues,
    image: &ImageContext,
    view_only: bool,
) -> Vec<(&'static str, ControlValue, bool)> {
    if view_only {
        return Vec::new();
    }
    let found = tools
        .iter()
        .find(|found| found.definition().tool.id == tool);
    let writes = found.and_then(|found| found.action(action, values, read, image));
    let writes: Vec<(&'static str, ControlValue)> = writes
        .iter()
        .flat_map(Controls::iter)
        .map(|(control, value)| (control, value.clone()))
        .collect();
    let last = writes.len().saturating_sub(1);
    writes
        .into_iter()
        .enumerate()
        .map(|(index, (control, value))| (control, value, index == last))
        .collect()
}

/// The design system's icon for a button's.
pub fn icon(icon: ButtonIcon) -> apertrust_ui::Icon {
    match icon {
        ButtonIcon::RotateLeft => apertrust_ui::Icon::RotateLeft,
        ButtonIcon::RotateRight => apertrust_ui::Icon::RotateRight,
        ButtonIcon::FlipHorizontal => apertrust_ui::Icon::FlipHorizontal,
        ButtonIcon::FlipVertical => apertrust_ui::Icon::FlipVertical,
        ButtonIcon::PrimariesMenu => apertrust_ui::Icon::PrimariesMenu,
        ButtonIcon::Camera => apertrust_ui::Icon::Camera,
    }
}

/// darktable's guides widget: show guides with the global guide settings button at its
/// end, drawn disabled, off, until the tool's guides come. It reports nothing.
fn guides_row(ui: &mut egui::Ui, words: &Translator) {
    let [label, full, promise, settings, settings_promise] =
        messages::GUIDES.map(|id| words.text(id));
    let settings = format!("{settings}. {settings_promise}");
    let t = apertrust_ui::theme(ui.ctx());
    ui.horizontal(|ui| {
        // The row takes the width it is given, so the button's room, and the gap before it,
        // are kept back first; the gap is the only space between them.
        ui.spacing_mut().item_spacing.x = 0.0;
        let button = IconButton::new(apertrust_ui::Icon::Grid, &settings).enabled(false);
        let room = (ui.available_width() - button.side(ui) - t.spacing.xs).max(0.0);
        ui.allocate_ui(egui::vec2(room, t.sizes.panel_line), |ui| {
            let _ = ToggleRow::new(&label, false)
                .name(&full)
                .promise(&promise)
                .enabled(false)
                .show(ui);
        });
        ui.add_space(t.spacing.xs);
        let _ = button.show(ui);
    });
}

/// Whether an end item is an icon action that runs, as a button row draws them.
fn runs_with_icon(item: &End) -> bool {
    matches!(
        item,
        End::Action(ActionEnd {
            face: Face::Icon(_),
            menu: [],
            unported: false,
            ..
        })
    )
}

/// A line of one of darktable's buttons alone, a text action that writes the tool's
/// controls: an `ActionRow`, its button at the line's start, its promise the tooltip,
/// drawn disabled while the photo takes no edits and until what it runs is built. A press
/// runs it on the values shown.
fn action_row(ui: &mut egui::Ui, page: &Page, action: &ActionEnd, changes: &mut Vec<Change>) {
    let tool = page.tool.definition().tool.id;
    let [label, full, promise] = messages::action(tool, action.id).map(|id| page.words.text(&id));
    let pressed = ActionRow::new(&label)
        .name(&full)
        .promise(&promise)
        .enabled(page.runs(action))
        .show(ui);
    if pressed {
        changes.push(page.run(action.id));
    }
}

/// An end item as the panel draws it: a picker's button with its words, or an action's
/// words and whether it can run, and a menu button's items, each an action with its short
/// label and whether it can run.
enum EndWords<'a> {
    Picker(&'static str, PickerWords<'a>),
    Action {
        action: ActionEnd,
        label: String,
        full: String,
        promise: String,
        enabled: bool,
        items: Vec<(&'static str, String, bool)>,
    },
}

impl<'a> Page<'a> {
    /// Whether the action can run now: the photo takes edits, it is ported, and the tool's
    /// rule allows it on the values shown, as the session reports it with the tool's state
    /// and the pending display follows it (decision 107).
    fn runs(&self, action: &ActionEnd) -> bool {
        self.runs_id(action.id, action.unported)
    }

    /// Whether the action `id`, a menu's item among them, can run now, as [`Self::runs`].
    fn runs_id(&self, id: &str, unported: bool) -> bool {
        self.editable && !unported && self.allowed(id)
    }

    /// Whether the tool's rule allows the action `id` on the values shown, as the session
    /// reports it with the tool's state and the pending display follows it.
    fn allowed(&self, id: &str) -> bool {
        let reported = (self.state.actions.iter()).find(|state| state.id == id);
        reported.is_none_or(|state| state.available)
    }

    /// What pressing the action `id` asks: the tool's action on the values shown.
    fn run(&self, id: &'static str) -> Change {
        Change::Action {
            tool: self.tool.definition().tool.id,
            action: id,
            values: values(self.state),
        }
    }

    /// The end items `end` as the panel draws them, each with its words; a picker's only
    /// where its tool draws its pickers.
    fn end_words(&self, ctx: &egui::Context, end: &[End]) -> Vec<EndWords<'a>> {
        let tool = self.tool.definition().tool.id;
        end.iter()
            .filter_map(|item| match *item {
                End::Picker(id) => self
                    .button(ctx, id)
                    .map(|(id, words)| EndWords::Picker(id, words)),
                // A slider's alone, which its row draws in the quad's place.
                End::Warning(_) => None,
                End::Action(action) => {
                    let [label, full, promise] =
                        messages::action(tool, action.id).map(|id| self.words.text(&id));
                    let items = (action.menu.iter())
                        .map(|&item| {
                            let [label, ..] = messages::action(tool, item);
                            let runs = self.runs_id(item, action.unported);
                            (item, self.words.text(&label), runs)
                        })
                        .collect();
                    Some(EndWords::Action {
                        action,
                        label,
                        full,
                        promise,
                        enabled: self.runs(&action),
                        items,
                    })
                }
            })
            .collect()
    }

    /// What pressing the end item `words[index]` asks, as `pressed` says.
    fn end_change(
        &self,
        ui: &egui::Ui,
        words: &[EndWords],
        (index, pressed): (usize, EndAction),
    ) -> Option<Change> {
        match (words.get(index)?, pressed) {
            (EndWords::Picker(picker, _), EndAction::Picker(action)) => {
                Some(self.pressed(ui, picker, action))
            }
            (EndWords::Action { action, .. }, EndAction::Pressed) => Some(self.run(action.id)),
            // A menu's item, one of its own actions.
            (EndWords::Action { items, .. }, EndAction::Chosen(chosen)) => items
                .iter()
                .find(|(item, _, runs)| *item == chosen && *runs)
                .map(|(item, _, _)| self.run(item)),
            _ => None,
        }
    }
}

/// The design system's end items for `words`, in order.
fn end_items<'w>(words: &'w [EndWords]) -> Vec<EndItem<'w>> {
    words
        .iter()
        .map(|words| match words {
            EndWords::Picker(_, button) => EndItem::Picker(button.button()),
            EndWords::Action {
                action,
                full,
                promise,
                enabled,
                items,
                ..
            } if !items.is_empty() => EndItem::Menu {
                icon: icon(match action.face {
                    Face::Icon(face) => face,
                    // A menu button is an icon's (`ProcessingDefinition::check`).
                    Face::Text => ButtonIcon::PrimariesMenu,
                }),
                name: full,
                promise: Some(promise),
                enabled: *enabled,
                entries: (items.iter())
                    .map(|(id, label, runs)| {
                        popup::Entry::Item(popup::Item::new(*id, label.clone()).enabled(*runs))
                    })
                    .collect(),
            },
            EndWords::Action {
                action,
                label,
                full,
                promise,
                enabled,
                ..
            } => match action.face {
                Face::Icon(face) => EndItem::Icon {
                    icon: icon(face),
                    name: full,
                    promise: Some(promise),
                    enabled: *enabled,
                },
                Face::Text => EndItem::Text {
                    label,
                    name: full,
                    promise: Some(promise),
                    enabled: *enabled,
                },
            },
        })
        .collect()
}

/// A line of darktable's widgets packed in a box, of any shape the three rows before it
/// did not draw (decision 107): its start, a plain group's words, a control's reset label
/// or a readout, and its end items, pickers and actions, as a `LineRow`.
fn line_row(
    ui: &mut egui::Ui,
    page: &Page,
    start: &LineStart,
    readout: Option<&ReadoutDef>,
    end: &[End],
    changes: &mut Vec<Change>,
) {
    let tool = page.tool.definition().tool.id;
    let control_words = match start {
        LineStart::Control(id) => {
            let Some(shown) = control(page.state, id) else {
                return;
            };
            Some((words_of(page.words, tool, id, None), shown.available, *id))
        }
        _ => None,
    };
    let label = match start {
        LineStart::Label(id) => Some(page.words.text(&messages::group(tool, id))),
        _ => None,
    };
    let shown = readout.and_then(|def| readout_words(page, tool, def));
    let line_start = match (&control_words, &label, &shown) {
        (Some(([label, full, promise], enabled, _)), _, _) => parts::LineStart::Control {
            label,
            name: full,
            promise: Some(promise),
            enabled: *enabled,
        },
        (None, Some(label), _) => parts::LineStart::Label(label),
        (None, None, Some(shown)) => parts::LineStart::Readout {
            text: &shown.text,
            promise: &shown.promise,
            swatch: shown
                .swatch
                .as_ref()
                .map(|(colour, tip)| (*colour, tip.as_str())),
        },
        (None, None, None) => parts::LineStart::None,
    };
    let words = page.end_words(ui.ctx(), end);
    match LineRow::new(line_start, end_items(&words)).show(ui) {
        Some(LineAction::Reset) => {
            if let Some((_, _, control)) = control_words {
                changes.push(Change::Reset { tool, control });
            }
        }
        Some(LineAction::End(index, pressed)) => {
            changes.extend(page.end_change(ui, &words, (index, pressed)));
        }
        None => {}
    }
}

/// A computed value's row (`Row::Computed`, decision 87), the design system's readout
/// row: its label, and the value the session reports with the tool's state, signed as a
/// slider's over zero, with its unit; nothing at the end while its operation computes
/// none.
fn computed_row(
    ui: &mut egui::Ui,
    words: &Translator,
    tool: &str,
    computed: &Computed,
    state: &ToolState,
) {
    let [label, promise] = messages::computed(tool, computed.id).map(|id| words.text(&id));
    let value = state
        .computed
        .iter()
        .find(|shown| shown.id == computed.id)
        .and_then(|shown| shown.value);
    let unit = computed.unit.map(messages::unit_value);
    let mut row = ReadoutRow::new(&label, value.map(f64::from))
        .promise(&promise)
        .decimals(usize::from(computed.decimals))
        .signed(true);
    if let Some(unit) = &unit {
        row = row.unit(unit);
    }
    row.show(ui);
}

/// A preset row (decision 2), Blended as a tool's choice: it shows the option the
/// session reports the controls hold, or Custom, which the list never offers; the options
/// this photo cannot have are listed disabled. Choosing one applies it. It stores nothing,
/// so it has no marker and no reset of its own.
///
/// An option that turns a picker on (`PresetRow::pickers`), White balance's From image
/// area, asks for its picker as an area, as a press on its button would, and nothing while
/// that picker is on, as darktable presses the button only when it is not active
/// (`temperature.c:1816-1822`); it is disabled where its picker cannot be pressed.
fn preset_row(ui: &mut egui::Ui, page: &Page, id: &'static str, changes: &mut Vec<Change>) {
    let (words, definition, state) = (page.words, page.tool.definition(), page.state);
    let tool = definition.tool.id;
    let shown = state.presets.iter().find(|shown| shown.preset == id);
    let (Some(row), Some(shown)) = (definition.preset(id), shown) else {
        return;
    };
    let [label, full, promise] = messages::preset(tool, id).map(|message| words.text(&message));
    let names: Vec<String> = row
        .options
        .iter()
        .map(|option| words.text(&messages::preset_option(tool, id, option)))
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let custom = words.text(messages::PRESET_CUSTOM);
    let picker_of = |option: &str| {
        (row.pickers.iter())
            .find(|(turning, _)| *turning == option)
            .map(|&(_, picker)| picker)
    };
    let pickers = page.pickers;
    let can_pick = pickers.enabled && pickers.drawn.contains(&tool);
    let unavailable: Vec<usize> = (0..row.options.len())
        .filter(|&i| {
            let option = row.options[i];
            !shown.available.contains(&option) || (picker_of(option).is_some() && !can_pick)
        })
        .collect();
    // An option that turns a picker on, disabled as its picker is, says why as its button
    // would: the photo opening, or open for viewing only.
    let picker_disabled = (row.options.iter())
        .any(|option| picker_of(option).is_some() && shown.available.contains(option) && !can_pick);
    let unavailable_tip = match pickers.why {
        Some(why) if picker_disabled => why.to_owned(),
        _ => words.text("preset-option-unavailable"),
    };
    let matching = shown
        .matching
        .and_then(|matching| row.options.iter().position(|&option| option == matching));
    let chosen = Choice::new(&label, &names, matching)
        .name(&full)
        .promise(&promise)
        .style(parts::ChoiceStyle::Blended)
        .wheel(page.wheel())
        .none_chosen(&custom)
        .disabled_options(&unavailable, &unavailable_tip)
        // A row with no option the photo can have can't run, so it is drawn disabled.
        .enabled(!shown.available.is_empty())
        .show(ui);
    if let Some(ChoiceAction::Set(chosen)) = chosen {
        let option = row.options[chosen];
        match picker_of(option) {
            Some(picker) => {
                let this = PickerRef::Tool { tool, picker };
                if pickers.on != Some(this) {
                    let on = PickerAction::On { area: true };
                    changes.push(page.pressed(ui, picker, on));
                }
            }
            // The box shows it once the session reports it: its values are the photo's,
            // which only the session knows.
            None => changes.push(Change::ApplyPreset {
                tool,
                preset: id,
                option,
            }),
        }
    }
}

/// A line of a utility's settings as fields (decision 107), Export's `[w] × [h] px`: each
/// field its setting's value as the page shows it, named by its words with its unit, its
/// tooltip a setting row's ([`setting_tips`]), and drawn disabled, that tooltip its reason,
/// where the page says it is not live, as where `--conf` gives it. A value typed is
/// reported as [`Change::Setting`] by its key, rounded to the decimals the field shows.
fn fields_row(ui: &mut egui::Ui, page: &dyn Rows, fields: &Fields, changes: &mut Vec<Change>) {
    /// What a field shows, owned for the row to borrow.
    struct Shown {
        name: String,
        unit: Option<String>,
        tip: Option<String>,
        live: bool,
        value: f64,
        range: (f32, f32),
    }
    let (words, tool) = (page.words(), page.definition().id);
    let shown: Vec<Shown> = (fields.fields.iter())
        .map(|field| {
            let setting = &field.setting;
            let [_, full, _] = messages::setting_row(tool, setting);
            let (value, live) = page
                .setting(setting)
                .unwrap_or_else(|| (setting.default(), false));
            Shown {
                name: full.map(|id| words.text(&id)).unwrap_or_default(),
                unit: field.unit.map(|unit| words.text(&messages::unit(unit))),
                tip: setting_tips(page, setting).0,
                live,
                value: match value {
                    ControlValue::Number(value) => widen(value),
                    _ => 0.0,
                },
                range: match setting.kind {
                    ControlKind::Number { min, max, .. } => (min, max),
                    _ => (0.0, 0.0),
                },
            }
        })
        .collect();
    let row: Vec<parts::RowField<'_>> = (fields.fields.iter().zip(&shown))
        .map(|(field, shown)| {
            let (min, max) = shown.range;
            let mut row = parts::RowField::new(&shown.name, shown.value, widen(min)..=widen(max))
                .decimals(usize::from(field.setting.decimals));
            if let Some(before) = field.before {
                row = row.before(before.text());
            }
            if let Some(unit) = &shown.unit {
                row = row.unit(unit);
            }
            match (&shown.tip, shown.live) {
                (tip, false) => row = row.disabled(tip.as_deref().unwrap_or_default()),
                (Some(tip), true) => row = row.tip(tip),
                (None, true) => {}
            }
            row
        })
        .collect();
    let name = words.text(fields.words);
    let id = egui::Id::unique(("fields row", tool, fields.id));
    if let Some((at, typed)) = parts::FieldsRow::new(id, &name, &row).show(ui) {
        let (setting, (min, max)) = (&fields.fields[at].setting, shown[at].range);
        changes.push(Change::Setting {
            tool,
            setting: setting.key.unwrap_or(setting.id),
            value: ControlValue::Number(shown_as(typed, setting.decimals, min, max)),
            done: true,
        });
    }
}

/// What a setting's row says over it, in the page's words: its tooltip, the page's for it
/// where the page gives one, else the setting's promise; and the page's alone, which is its
/// description too where the row shows one.
fn setting_tips(page: &dyn Rows, setting: &SettingRow) -> (Option<String>, Option<String>) {
    let (words, tool) = (page.words(), page.definition().id);
    let [_, _, promise] = messages::setting_row(tool, setting);
    let tip = page.tip(&Row::Setting(*setting));
    (tip.clone().or(promise.map(|id| words.text(&id))), tip)
}

/// One of darktable's own settings for a tool, or a utility's: no control, so nothing an
/// edit stores. One the app keeps in settings is drawn live with its value there, a change
/// reported as [`Change::Setting`] by its key in full where it names one, else by its id;
/// any other, the colour checker's chart, is drawn disabled at darktable's default until
/// what uses it is built. Its words are its own where it declares them (decision 93); a
/// tooltip the page supplies, as a utility's from the session, replaces its promise and
/// is its description too; options the page says cannot be chosen now are drawn disabled
/// with the page's reason; and a boolean whose words name No and Yes is darktable's choice
/// of the two (decision 90).
fn setting_row(
    ui: &mut egui::Ui,
    page: &dyn Rows,
    setting: &SettingRow,
    changes: &mut Vec<Change>,
) {
    let (words, tool) = (page.words(), page.definition().id);
    let [label, full, _] =
        messages::setting_row(tool, setting).map(|id| id.map(|id| words.text(&id)));
    let (label, full) = (label.unwrap_or_default(), full.unwrap_or_default());
    // The page's tooltip for it, its description too, else its promise.
    let (promise, tip) = setting_tips(page, setting);
    let (value, live) = page
        .setting(setting)
        .unwrap_or_else(|| (setting.default(), false));
    let id = setting.key.unwrap_or(setting.id);
    let mut set = |value: ControlValue, done: bool| {
        changes.push(Change::Setting {
            tool,
            setting: id,
            value,
            done,
        });
    };
    let options: Vec<String> = messages::setting_row_options(tool, setting)
        .into_iter()
        .map(|word| match word {
            Word::Message(id) => words.text(&id),
            Word::Name(name) => name,
        })
        .collect();
    let options: Vec<&str> = options.iter().map(String::as_str).collect();
    // A boolean drawn as darktable's choice of No and Yes, false the first.
    if let (true, ControlValue::Bool(on)) = (setting.no_or_yes(), &value) {
        let mut row = Choice::new(&label, &options, usize::from(*on))
            .name(&full)
            .style(parts::ChoiceStyle::Blended)
            .wheel(page.wheel())
            .enabled(live);
        if let Some(promise) = &promise {
            row = row.promise(promise);
        }
        if let Some(tip) = &tip {
            row = row.description(tip);
        }
        if let Some(ChoiceAction::Set(chosen)) = row.show(ui) {
            set(ControlValue::Bool(chosen == 1), true);
        }
        return;
    }
    match (setting.kind, value) {
        (
            ControlKind::Number {
                min,
                max,
                step,
                default,
            },
            ControlValue::Number(value),
        ) => {
            let mut row = SliderRow::new(&label, widen(value), widen(min)..=widen(max))
                .name(&full)
                .step(widen(step))
                .decimals(usize::from(setting.decimals))
                .wheel(page.wheel())
                .enabled(live);
            if let Some(promise) = &promise {
                row = row.promise(promise);
            }
            // The unit the settings schema declares for it, as a control's.
            let declared = apertrust_tools::settings::setting(tool, setting.id);
            let unit = declared.and_then(|declared| match declared.kind() {
                Kind::Number(number) => number.unit.map(|unit| format!("unit-{unit}-value")),
                _ => None,
            });
            if let Some(unit) = &unit {
                row = row.unit(unit);
            }
            row = row.track(track(setting.track, (min, max)));
            match row.show(ui) {
                Some(SliderAction::Set { value, done }) => {
                    let value = shown_as(value, setting.decimals, min, max);
                    set(ControlValue::Number(value), done);
                }
                Some(SliderAction::Reset) => set(ControlValue::Number(default), true),
                _ => {}
            }
        }
        (
            ControlKind::Choice {
                options: ids,
                unported,
                ..
            },
            ControlValue::Choice(chosen),
        ) => {
            // What each option makes a picker do, where a picker's words follow the setting
            // (decision 88).
            let tips: Vec<String> = if page.words_by(setting.id) {
                ids.iter()
                    .map(|o| words.text(&messages::setting_option_promise(tool, setting.id, o)))
                    .collect()
            } else {
                Vec::new()
            };
            let tips: Vec<&str> = tips.iter().map(String::as_str).collect();
            // The closed row's tooltip is the chosen option's own where the options have
            // tips, else the setting's.
            let closed = tips.get(chosen as usize).copied().or(promise.as_deref());
            // An option still to come is listed, as one darktable lists that is not ported,
            // and so is one the page says cannot be chosen now, each with its own reason.
            let later_tip = words.text("choice-option-later");
            let later: Vec<usize> = (ids.iter().enumerate())
                .filter(|(_, option)| unported.contains(option))
                .map(|(at, _)| at)
                .collect();
            let now = page.disabled(setting);
            let reasons: Vec<(usize, &str)> = now
                .iter()
                .flat_map(|(now, why)| now.iter().map(move |at| (*at, why.as_str())))
                .collect();
            let mut row = Choice::new(&label, &options, chosen as usize)
                .name(&full)
                .option_tips(&tips)
                .disabled_options(&later, &later_tip)
                .disabled_reasons(&reasons)
                .style(parts::ChoiceStyle::Blended)
                .wheel(page.wheel())
                .enabled(live);
            if let Some(closed) = closed {
                row = row.promise(closed);
            }
            if let Some(tip) = &tip {
                row = row.description(tip);
            }
            if let Some(ChoiceAction::Set(chosen)) = row.show(ui) {
                set(ControlValue::Choice(chosen as u32), true);
            }
        }
        (ControlKind::Bool { .. }, ControlValue::Bool(on)) => {
            let promise = promise.unwrap_or_default();
            let toggled = ToggleRow::new(&label, on)
                .name(&full)
                .promise(&promise)
                .enabled(live)
                .show(ui);
            if let Some(on) = toggled.filter(|_| live) {
                set(ControlValue::Bool(on), true);
            }
        }
        // No setting is curve points (`ProcessingDefinition::check`), and a kept value is
        // its kind's.
        _ => {}
    }
}

/// darktable's lightness track: the grey of each lightness from `min` to `max`, at its
/// slider's 20 stops (`DT_BAUHAUS_SLIDER_MAX_STOPS`), as `_paint_hue` sets them.
fn lightness_ramp(min: f32, max: f32) -> parts::Track {
    const STOPS: usize = 20;
    parts::Track::Gradient(
        (0..STOPS)
            .map(|i| {
                let stop = i as f32 / (STOPS - 1) as f32;
                let lightness = min + stop * (max - min);
                let patch = apertrust_tools::pick::Patch::of_lch([lightness, 0.0, 0.0]);
                (stop, colour(patch.srgb))
            })
            .collect(),
    )
}

/// A reference row (`Row::Reference`, decision 88), the design system's `ReferenceRow`:
/// its label the picker's, the reference as the app keeps it, its patch the grey of its L
/// and the name of the photo it was measured on, "this photo" where that is the photo
/// open, dimmed once found gone; "measuring…" while its picker is on, "none" with none
/// set. Its picker's button as the tool's other pickers' are, and ⋯ with Clear Reference,
/// both settings gone, and Set Reference from an Area, a press of its picker. Drawn only
/// where its tool draws its pickers.
fn reference_row(
    ui: &mut egui::Ui,
    page: &Page,
    picker: &'static str,
    lightness: &'static str,
    photo: &'static str,
    changes: &mut Vec<Change>,
) {
    let Some((_, button)) = page.button(ui.ctx(), picker) else {
        return;
    };
    let words = page.words;
    let tool = page.tool.definition().tool.id;
    let [label, _, promise, _] = messages::picker(tool, picker).map(|id| words.text(&id));
    let [
        value_id,
        photo_id,
        this_id,
        none,
        measuring,
        gone,
        more,
        clear,
        set,
    ] = messages::REFERENCE;
    let (none, measuring, gone) = (words.text(none), words.text(measuring), words.text(gone));
    let settings = page.pickers.settings;
    let path = settings.text(tool, photo);
    let reference = match settings.get(tool, lightness) {
        Some(ControlValue::Number(l)) => Some(*l),
        _ => None,
    };
    let value = reference.map(|l| {
        let mut args = Args::new();
        args.set("lightness", (f64::from(l) * 10.0).round() / 10.0);
        words.text_with(value_id, Some(&args))
    });
    // The photo it was measured on, where one is known: none for one `--conf` gives.
    let named = path.map(|path| {
        let open = page.pickers.photo;
        if open.is_some_and(|open| open == std::path::Path::new(path)) {
            words.text(this_id)
        } else {
            let mut args = Args::new();
            args.set("photo", crate::pickers::photo_name(path));
            words.text_with(photo_id, Some(&args))
        }
    });
    let measuring_now = page.pickers.on == Some(PickerRef::Tool { tool, picker });
    let shown = match reference {
        _ if measuring_now => Reference::Measuring(&measuring),
        Some(l) => Reference::Set {
            patch: colour(apertrust_tools::pick::Patch::of_lch([l, 0.0, 0.0]).srgb),
            value: value.as_deref().unwrap_or_default(),
            photo: named.as_deref(),
            gone: path
                .filter(|path| (page.pickers.gone)(path))
                .map(|_| gone.as_str()),
        },
        None => Reference::None(&none),
    };
    let (more, clear, set) = (words.text(more), words.text(clear), words.text(set));
    // A reference `--conf` gives, either half, keeps it for the run: the row disabled,
    // its picker and ⋯ with it, saying why, as the Settings dialog says of such a row
    // (decision 43).
    let given = (page.pickers.settings).overridden(tool, lightness)
        || (page.pickers.settings).overridden(tool, photo);
    let why = given.then(|| {
        let mut args = Args::new();
        args.set("flag", apertrust_base::settings::CONF_FLAG);
        words.text_with("settings-overridden", Some(&args))
    });
    let mut picker_button = button.button();
    if let Some(why) = &why {
        picker_button = picker_button.disabled(why);
    }
    let action = ReferenceRow::new(&label, shown, picker_button, (&more, &clear, &set))
        .promise(why.as_deref().unwrap_or(&promise))
        .enabled(page.pickers.enabled && !given)
        .show(ui);
    match action {
        Some(ReferenceAction::Picker(action)) => changes.push(page.pressed(ui, picker, action)),
        Some(ReferenceAction::Set) => {
            let on = PickerAction::On { area: true };
            changes.push(page.pressed(ui, picker, on));
        }
        Some(ReferenceAction::Clear) => changes.push(Change::ClearReference {
            tool,
            lightness,
            photo,
        }),
        None => {}
    }
}

/// What the last pick of `picker` did (`Row::LastPick`, decision 88): its warnings, one
/// line each in the warning colour, the clipped one first, then the readout with the area
/// before and after as a patch before each value, "■ L 41 % → ■ L 50 % · +0.55 EV".
/// Nothing before a pick,
/// nor where the tool's pickers are not drawn.
fn last_pick_row(ui: &mut egui::Ui, page: &Page, picker: &str) {
    let definition = page.tool.definition();
    let tool = definition.tool.id;
    if !page.pickers.drawn.contains(&tool) {
        return;
    }
    let Some(declared) = definition.picker(picker) else {
        return;
    };
    let Some((last, warnings)) = page.pickers.picked.last(tool) else {
        return;
    };
    let words = page.words;
    let Some(said) = crate::pickers::last_pick_words(words, definition, declared, last, warnings)
    else {
        return;
    };
    for (text, tip) in &said.warnings {
        WarningRow::new(text).tip(tip).show(ui);
    }
    let [before, after] = last.patches.map(|patch| colour(patch.srgb));
    ReadoutRow::new(&said.label, None)
        .text(&said.value)
        .patches(&[before, after])
        .promise(&said.promise)
        .show(ui);
}

/// The setting holding the photo of the reference whose L is `tool`'s setting `setting`,
/// if a reference row names it (decision 88): where the app keeps the open photo's path as
/// a pick hands the L back.
pub fn reference_photo(
    tools: &[&dyn ProcessingTool],
    tool: &str,
    setting: &str,
) -> Option<&'static str> {
    let found = tools.iter().find(|t| t.definition().tool.id == tool)?;
    found.definition().rows().find_map(|row| match row {
        Row::Reference {
            lightness, photo, ..
        } if *lightness == setting => Some(*photo),
        _ => None,
    })
}

/// A picker's input and target patches (`Row::Patches`), darktable's: the input the area
/// its tool's last pick showed on the photo, with its L readout, or for a target of L, C
/// and h its L, h and c, "N/A" before one; the target the colour of its settings, L then
/// C and h, as the app keeps them or at their defaults. Drawn disabled while the tool's
/// pickers are not drawn.
fn patches_row(ui: &mut egui::Ui, page: &Page, picker: &str, target: &[&'static str]) {
    let (words, definition) = (page.words, page.tool.definition());
    let tool = definition.tool.id;
    let enabled = page.pickers.drawn.contains(&tool) && definition.picker(picker).is_some();
    let lch: Vec<f32> = target
        .iter()
        .map(|id| {
            let row = definition.rows().find_map(|row| match row {
                Row::Setting(setting) if setting.id == *id => Some(setting),
                _ => None,
            });
            let value = page.pickers.settings.get(tool, id).cloned();
            match value.or_else(|| row.map(SettingRow::default)) {
                Some(ControlValue::Number(value)) => value,
                _ => 0.0,
            }
        })
        .collect();
    let target_colour = apertrust_tools::pick::Patch::of_lch([
        lch.first().copied().unwrap_or_default(),
        lch.get(1).copied().unwrap_or_default(),
        lch.get(2).copied().unwrap_or_default(),
    ]);
    let input = page.pickers.picked.patch(tool).filter(|_| enabled);
    // darktable's readout: L, then with a target of L, C and h, h and c, a line each
    // (`channelmixerrgb.c:4182-4191`), "N/A" for each before a pick.
    let lines: &[(usize, &str, &str)] = if target.len() == 3 {
        &[
            (0, "picker-patch-lightness", "picker-patch-none"),
            (2, "picker-patch-hue", "picker-patch-hue-none"),
            (1, "picker-patch-chroma", "picker-patch-chroma-none"),
        ]
    } else {
        &[(0, "picker-patch-lightness", "picker-patch-none")]
    };
    let readout: Vec<String> = lines
        .iter()
        .map(|&(at, value, none)| match input {
            Some(patch) => {
                let mut args = Args::new();
                args.set(
                    "value",
                    words.format_number(f64::from(patch.lch[at]), 1, false),
                );
                words.text_with(value, Some(&args))
            }
            None => words.text(none),
        })
        .collect();
    let readout = readout.join("\n");
    let [input_heading, input_tip, target_heading, lch_tip, ..] =
        messages::PATCHES.map(|id| words.text(id));
    let target_tip = target
        .first()
        .map(|id| words.text(&messages::setting(tool, id)[2]))
        .unwrap_or_default();
    parts::Patches::new(
        parts::Patch {
            heading: &input_heading,
            colour: input.map(|patch| colour(patch.srgb)),
            tip: &input_tip,
            readout: Some((&readout, &lch_tip)),
        },
        parts::Patch {
            heading: &target_heading,
            colour: Some(colour(target_colour.srgb)),
            tip: &target_tip,
            readout: None,
        },
    )
    .enabled(enabled)
    .show(ui);
}

/// A patch's sRGB, 0 to 1, as the screen shows it.
fn colour(srgb: [f32; 3]) -> egui::Color32 {
    let [r, g, b] = srgb.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    egui::Color32::from_rgb(r, g, b)
}

/// A collapsible group: its title, closed or open as the app keeps it, and its rows while
/// open. While closed with controls or settings set, a dot says so, and its tooltip how
/// many, as the wireframe's "Margins: 1 set".
fn group_row(
    ui: &mut egui::Ui,
    page: &dyn Rows,
    group: &Group,
    first: bool,
    changes: &mut Vec<Change>,
) {
    let tool = page.definition().id;
    let Group { id, kind, rows } = *group;
    let title = page.title(id, kind);
    if kind == GroupKind::Plain {
        PlainGroup::new(&title)
            .first(first)
            .show(ui, |ui| draw_rows(ui, page, rows, changes));
        return;
    }
    // A group with no rows has nothing behind it yet: drawn disabled, as a tab with none;
    // so is one whose rows the page says are not built yet.
    let tip = page.group_tip(id);
    let later = page.group_later(id);
    if rows.is_empty() || later.is_some() {
        let later = later.unwrap_or_else(|| page.words().text("section-tab-later"));
        let mut group = CollapsibleGroup::new(&title, false)
            .first(first)
            .disabled(&later);
        if let Some(tip) = &tip {
            group = group.tip(tip);
        }
        group.show(ui, |_| {});
        return;
    }
    let open = page.group_open(id);
    let set = page.set_in(rows);
    // A scope, whose response is the group's row while it is closed, carries the tooltip.
    let warning = page.group_warning(id);
    let group = ui.scope(|ui| {
        let mut group = CollapsibleGroup::new(&title, open).first(first).set(set);
        if let Some(tip) = &tip {
            group = group.tip(tip);
        }
        if let Some((name, menu)) = &warning {
            group = group.end_warning(name, menu.clone());
        }
        group.show_with(ui, |ui| draw_rows(ui, page, rows, changes))
    });
    if !open && set > 0 {
        let mut args = Args::new();
        args.set("group", title.as_str());
        args.set("count", set);
        let tip = page.words().text_with("collapsible-group-tip", Some(&args));
        group.response.on_hover_text(tip);
    }
    if let Some(chosen) = &group.inner.chosen {
        page.group_warned(id, chosen);
    }
    if group.inner.toggled {
        changes.push(Change::Group { tool, group: id });
    }
}

/// A control's words: its short label, its full name and its promise, or the row's own
/// where darktable labels the control otherwise in the row's mode ([`Slider::label`]).
fn words_of(words: &Translator, tool: &str, control: &str, own: Option<&str>) -> [String; 3] {
    match own {
        Some(own) => messages::row_words(tool, control, own),
        None => [
            messages::label(tool, control),
            messages::full(tool, control),
            messages::promise(tool, control),
        ],
    }
    .map(|id| words.text(&id))
}

/// What a slider row shows: a number control as the mirror shows it, or a lossless view's
/// coordinate as the session reports it (decision 2), with its range, step and unit.
struct Shown {
    min: f32,
    max: f32,
    step: f32,
    /// The soft range the session reports, where the tool's range has moved off the
    /// definition's (decision 30); `None` for the row's own.
    soft: Option<(f32, f32)>,
    unit: Option<Unit>,
    value: f32,
    mark: Option<Mark>,
    available: bool,
}

impl Shown {
    /// The number `id` names: one of the tool's controls, or else a coordinate of one of
    /// its views. A coordinate's mark is the one its view's controls share, and it can
    /// change the photo while one of them can.
    fn of(definition: &ProcessingDefinition, state: &ToolState, id: &str) -> Option<Self> {
        if let Some(found) = definition.control(id) {
            let shown = control(state, id)?;
            let (ControlKind::Number { min, max, step, .. }, ControlValue::Number(value)) =
                (found.kind, &shown.value)
            else {
                return None;
            };
            // The range the session reports for the tool's values (decision 30); while
            // it is the definition's, the row's own soft range stands.
            let (min, max, step, soft) = match shown.range {
                Some(range) if Some(range) != definition.range(id) => {
                    (range.min, range.max, range.step, Some(range.soft))
                }
                _ => (min, max, step, None),
            };
            return Some(Self {
                min,
                max,
                step,
                soft,
                unit: found.unit,
                value: *value,
                mark: shown.mark,
                available: shown.available,
            });
        }
        let (view, at) = definition.coordinate(id)?;
        let found = &view.coordinates[at];
        let shown = state.views.iter().find(|shown| shown.coordinate == id)?;
        Some(Self {
            min: found.min,
            max: found.max,
            step: found.step,
            soft: None,
            unit: found.unit,
            value: shown.value,
            mark: shown.mark,
            // The session's answer for the coordinate itself, while a control it moves is
            // shown available, as a pending change may make one.
            available: shown.available
                && view
                    .controls
                    .iter()
                    .any(|id| control(state, id).is_some_and(|c| c.available)),
        })
    }
}

fn slider_row(ui: &mut egui::Ui, page: &Page, slider: &Slider, changes: &mut Vec<Change>) {
    let (words, state) = (page.words, page.state);
    let definition = page.tool.definition();
    let (tool, id) = (definition.tool.id, slider.control);
    let Some(shown) = Shown::of(definition, state, id) else {
        return;
    };
    let (min, max) = (shown.min, shown.max);
    // A coordinate's words are a control's.
    let [label, full, promise] = words_of(words, tool, id, slider.label);
    let unit = shown.unit.map(messages::unit_value);
    // A value outside the range, where a view's controls put it, shows as the value, the
    // track at its end.
    let mut row = SliderRow::new(&label, widen(shown.value), widen(min)..=widen(max))
        .name(&full)
        .promise(&promise)
        .step(widen(shown.step))
        .decimals(usize::from(slider.decimals))
        .mark(mark(shown.mark))
        .track(track(
            slider.track,
            shown.soft.or(slider.soft).unwrap_or((min, max)),
        ))
        .wheel(page.wheel())
        .enabled(shown.available);
    if !slider.fill {
        row = row.no_fill();
    }
    // darktable's soft range: the track spans it, and the control's range stays what a
    // typed value, a script or a preset may reach. A range that follows other controls
    // spans the one the session reports (decision 30).
    if let Some((low, high)) = shown.soft.or(slider.soft) {
        row = row.soft_range(widen(low)..=widen(high));
    }
    if let Some(unit) = &unit {
        row = row.unit(unit);
    }
    // A picker at the row's end, where its tool's pick is built; the other end items
    // come with decision 107's next steps, `ProcessingDefinition::check` refusing them
    // until then, rather than one with nothing behind it.
    let picker = match slider.end {
        Some(End::Picker(picker)) => page.button(ui.ctx(), picker),
        _ => None,
    };
    if let Some((_, button)) = &picker {
        row = row.picker(button.button());
    }
    // A warning at the row's end, its place kept while it is off (decision 107).
    let warning = match slider.end {
        Some(End::Warning(id)) => {
            let on = (page.state.warnings.iter()).any(|state| state.id == id && state.on);
            Some(on.then(|| page.words.text(&messages::warning(tool, id))))
        }
        _ => None,
    };
    if let Some(message) = &warning {
        row = row.warning(message.as_deref());
    }
    let control = id;
    match row.show(ui) {
        Some(SliderAction::Set { value, done }) => changes.push(Change::Set {
            tool,
            control,
            value: ControlValue::Number(shown_as(value, slider.decimals, min, max)),
            done,
        }),
        Some(SliderAction::Reset) => changes.push(Change::Reset { tool, control }),
        Some(SliderAction::ToggleKeep) => changes.push(Change::Keep {
            tool,
            control,
            kept: shown.mark != Some(Mark::Kept),
        }),
        Some(SliderAction::Picker(action)) => {
            if let Some((picker, _)) = picker {
                changes.push(page.pressed(ui, picker, action));
            }
        }
        Some(SliderAction::Quad) | None => {}
    }
}

/// A picker's button as the panel draws it: its words and its state.
struct PickerWords<'a> {
    full: String,
    promise: String,
    on: bool,
    both: bool,
    focus: bool,
    enabled: bool,
    why: Option<&'a str>,
    /// Why it cannot start with the tool's settings (`ProcessingTool::picker_ready`,
    /// decision 88).
    unready: Option<String>,
}

impl PickerWords<'_> {
    fn button(&self) -> PickerButton<'_> {
        let mut button = PickerButton::new(&self.full, self.on)
            .tip(&self.promise)
            .point_or_area(self.both)
            .focus(self.focus);
        if !self.enabled {
            button = match self.why {
                Some(why) => button.disabled(why),
                None => button.state(ButtonState::Disabled),
            };
        } else if let Some(why) = &self.unready {
            button = button.disabled(why);
        }
        button
    }
}

impl<'a> Page<'a> {
    /// The button of the tool's picker `id`, if its tool draws its pickers: its
    /// words and whether it is on, enabled, and taking the focus back from the photo.
    fn button(
        &self,
        ctx: &egui::Context,
        id: &'static str,
    ) -> Option<(&'static str, PickerWords<'a>)> {
        let definition = self.tool.definition();
        let tool = definition.tool.id;
        if !self.pickers.drawn.contains(&tool) {
            return None;
        }
        let picker = definition.picker(id)?;
        let [_, full, promise, _] = messages::picker(tool, id);
        // What a pick will do, where the picker's words follow a setting (decision 88).
        let settings = self.pickers.settings;
        let action = crate::pickers::by_setting(
            self.words,
            definition,
            picker,
            settings,
            self.pickers.photo,
            "promise",
        );
        let this = PickerRef::Tool { tool, picker: id };
        let ready = self.tool.picker_ready(id, &settings.of(tool));
        let unready = (!ready).then(|| self.words.text(&messages::picker_unavailable(tool, id)));
        // The button is named by its action: where its words follow a setting, what a pick
        // will do; one that cannot start, why not.
        let full = match (&unready, &action) {
            (Some(why), _) => why.clone(),
            (None, Some(action)) => action.clone(),
            (None, None) => self.words.text(&full),
        };
        let promise = action.unwrap_or_else(|| self.words.text(&promise));
        Some((
            id,
            PickerWords {
                full,
                promise,
                on: self.pickers.on == Some(this),
                both: picker.kind == PickerKind::PointArea,
                focus: crate::pickers::takes_back(ctx, this),
                enabled: self.pickers.enabled,
                why: self.pickers.why,
                unready,
            },
        ))
    }

    /// What a press on `picker`'s button asks, with the keys held.
    fn pressed(&self, ui: &egui::Ui, picker: &'static str, action: PickerAction) -> Change {
        let keys = crate::pickers::pick_keys(ui.input(|i| i.modifiers));
        Change::Picker {
            tool: self.tool.definition().tool.id,
            picker,
            action,
            keys,
        }
    }
}

/// A line of pickers, darktable's toggle buttons packed in a box: its label at the start,
/// where it has one, or its readout's text, with its tooltip, and swatch, filling the row
/// up to the buttons, as the session reports them (`ToolState::readouts`), and the buttons
/// at the row's end, in order, where its tool draws its pickers.
fn pickers_row(
    ui: &mut egui::Ui,
    page: &Page,
    label: Option<&str>,
    readout: Option<&ReadoutDef>,
    ids: &[&'static str],
    changes: &mut Vec<Change>,
) {
    let buttons: Vec<(&'static str, PickerWords)> = ids
        .iter()
        .filter_map(|&id| page.button(ui.ctx(), id))
        .collect();
    let tool = page.tool.definition().tool.id;
    let shown = readout.and_then(|def| readout_words(page, tool, def));
    if buttons.is_empty() && shown.is_none() {
        return;
    }
    let label = label.map(|id| page.words.text(&messages::group(tool, id)));
    let t = apertrust_ui::theme(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_min_height(t.sizes.panel_line);
        if let Some(label) = &label {
            ui.add(egui::Label::new(label.as_str()).selectable(false));
        }
        // At the row's end, left to right, so readers and Tab meet them in order.
        let count = buttons.len() as f32;
        let gap = ui.spacing().item_spacing.x;
        let width = (count * t.sizes.field + (count - 1.0) * gap).max(0.0);
        let after = if count > 0.0 { gap } else { 0.0 };
        if let Some(shown) = &shown {
            // The text truncates in what the buttons and the swatch's two quads leave, so
            // a long language's text never pushes the pickers off the panel.
            let swatch = if shown.swatch.is_some() {
                2.0 * t.sizes.field + gap
            } else {
                0.0
            };
            let room = (ui.available_width() - width - after - swatch).max(0.0);
            ui.scope(|ui| {
                ui.set_max_width(room);
                ui.add(
                    egui::Label::new(shown.text.as_str())
                        .selectable(false)
                        .truncate(),
                )
            })
            .inner
            .on_hover_text(&shown.promise);
            if let Some((colour, tip)) = &shown.swatch {
                let room = ui.available_width() - width - after;
                parts::ColourSwatch::new(*colour, tip).width(room).show(ui);
            }
        }
        ui.add_space((ui.available_width() - width).max(0.0));
        for (picker, words) in &buttons {
            let (_, pressed) = words.button().show(ui);
            if let Some(action) = pressed {
                let picker: &'static str = picker;
                changes.push(page.pressed(ui, picker, action));
            }
        }
    });
}

/// What a readout at a line of pickers' start shows, in words.
struct ReadoutWords {
    text: String,
    promise: String,
    /// The swatch's colour, as the screen shows it, and its tooltip.
    swatch: Option<(egui::Color32, String)>,
}

/// The readout `def` of `tool` as the session reports it (`ToolState::readouts`): its text,
/// its number in the language's numbers to its decimals, its tooltip, and its swatch,
/// whose colour is handed to the screen as it is, as cairo paints darktable's. None before
/// the session reports one.
fn readout_words(page: &Page, tool: &str, def: &ReadoutDef) -> Option<ReadoutWords> {
    let state = page
        .state
        .readouts
        .iter()
        .find(|state| state.id == def.id)?;
    let readout = state.readout?;
    let [text, promise] = messages::readout(tool, def.id, readout.text);
    let mut args = Args::new();
    if let Some(value) = readout.value {
        let decimals = usize::from(def.decimals);
        args.set(
            "value",
            page.words.format_number(f64::from(value), decimals, false),
        );
    }
    let tip = page.words.text(&messages::readout_swatch(tool, def.id));
    Some(ReadoutWords {
        text: page.words.text_with(&text, Some(&args)),
        promise: page.words.text(&promise),
        swatch: readout.swatch.map(|rgb| (colour(rgb), tip)),
    })
}

/// An f32 as the decimal it was written as: a definition's 0.01 is f32's 0.0099999998,
/// and a slider snapping to that step in f64 would land beside the values it shows.
fn widen(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(f64::from(value))
}

/// A value a slider reports, rounded to the decimals its row shows and kept in the
/// control's range: a key step with Ctrl moves a tenth of a step, and a control set to
/// 0.1 while its row shows "0" would change the image while it reads as untouched.
fn shown_as(value: f64, decimals: u8, min: f32, max: f32) -> f32 {
    let scale = 10f64.powi(i32::from(decimals));
    let rounded = ((value * scale).round() / scale) as f32;
    rounded.clamp(min, max)
}

fn choice_row(
    ui: &mut egui::Ui,
    words: &Translator,
    definition: &ProcessingDefinition,
    id: &'static str,
    style: ChoiceStyle,
    state: &ToolState,
    changes: &mut Vec<Change>,
) {
    let tool = definition.tool.id;
    let (Some(control), Some(shown)) = (definition.control(id), control(state, id)) else {
        return;
    };
    let (
        ControlKind::Choice {
            options, unported, ..
        },
        ControlValue::Choice(index),
    ) = (control.kind, &shown.value)
    else {
        return;
    };
    let [label, full, promise] = words_of(words, tool, id, None);
    let index = *index as usize;
    // Whether the photo lists the option `i`: a report that lists none lists them all.
    let lists = |i: usize| shown.options.is_empty() || shown.options.contains(&options[i]);
    let chosen = match style {
        // darktable's combobox, its kept marker the mirror's, as a slider row's. What a
        // quad runs comes with the first tool whose quad runs, as on a slider row.
        ChoiceStyle::List => {
            // The options the session reports the photo lists, in the definition's order,
            // then, while the edit holds one the photo does not list, from another photo's
            // edit, that one, chosen and disabled (decision 30).
            let mut listed: Vec<usize> = (0..options.len()).filter(|&i| lists(i)).collect();
            let stored = (!listed.contains(&index)).then(|| {
                listed.push(index);
                listed.len() - 1
            });
            let names: Vec<String> = listed
                .iter()
                .map(|&i| words.text(&messages::option(tool, id, options[i])))
                .collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            // darktable's options its operation does not port: listed, never chosen.
            let later: Vec<usize> = (0..listed.len())
                .filter(|&at| Some(at) != stored && unported.contains(&options[listed[at]]))
                .collect();
            let later_tip = words.text("choice-option-later");
            let unlisted_tip = words.text("choice-option-unavailable");
            let at = listed.iter().position(|&i| i == index).unwrap_or_default();
            let mut row = Choice::new(&label, &names, at)
                .name(&full)
                .promise(&promise)
                .style(parts::ChoiceStyle::Blended)
                .mark(mark(shown.mark))
                .wheel(Wheel::Pointer)
                .enabled(shown.available)
                .disabled_options(&later, &later_tip);
            if let Some(stored) = stored {
                row = row.unlisted(stored, &unlisted_tip);
            }
            match row.show(ui) {
                Some(ChoiceAction::Set(chosen)) => listed.get(chosen).copied(),
                Some(ChoiceAction::ToggleKeep) => {
                    changes.push(Change::Keep {
                        tool,
                        control: id,
                        kept: shown.mark != Some(Mark::Kept),
                    });
                    None
                }
                Some(ChoiceAction::Quad) | None => None,
            }
        }
        // Every option, in the definition's order: those the photo does not list drawn
        // disabled with their reasons, so the one an edit from another photo holds is
        // shown chosen and disabled, and with nothing else to choose the chosen one is drawn
        // as anywhere, the others disabled (decision 88). A segmented row has no unported
        // option.
        ChoiceStyle::Segmented => {
            let all: Vec<String> = options
                .iter()
                .map(|option| words.text(&messages::option(tool, id, option)))
                .collect();
            let all: Vec<&str> = all.iter().map(String::as_str).collect();
            let reasons: Vec<(usize, String)> = (0..options.len())
                .filter(|&i| !lists(i))
                .map(|i| {
                    let why = messages::option_unavailable(tool, id, options[i]);
                    (i, words.text(&why))
                })
                .collect();
            let reasons: Vec<(usize, &str)> =
                reasons.iter().map(|(i, why)| (*i, why.as_str())).collect();
            // Unavailable only because the photo lists just the option it holds (decision
            // 30's rule for a list), the switch says so option by option instead: the held
            // one drawn as anywhere, the others disabled with their reasons (decision 88).
            let only = check::lists_only(control, &shown.options, &shown.value);
            Segmented::new(&full, &all, index)
                .full_width()
                .promise(&promise)
                .enabled(shown.available || only)
                .disabled_options(&reasons)
                .show(ui)
        }
    };
    if let Some(chosen) = chosen {
        changes.push(Change::Set {
            tool,
            control: id,
            value: ControlValue::Choice(chosen as u32),
            done: true,
        });
    }
}

/// A boolean's row, with its end items after its switch (decision 107).
fn toggle_row(
    ui: &mut egui::Ui,
    page: &Page,
    id: &'static str,
    end: &[End],
    changes: &mut Vec<Change>,
) {
    let (words, state) = (page.words, page.state);
    let tool = page.tool.definition().tool.id;
    let Some(shown) = control(state, id) else {
        return;
    };
    let ControlValue::Bool(value) = shown.value else {
        return;
    };
    let [label, full, promise] = words_of(words, tool, id, None);
    let after = shown
        .label_value
        .map(|value| label_value(words, tool, id, &value));
    let mut row = ToggleRow::new(&label, value)
        .name(&full)
        .promise(&promise)
        .enabled(shown.available);
    if let Some(after) = &after {
        row = row.value(after);
    }
    let ends = page.end_words(ui.ctx(), end);
    let (changed, pressed) = row.end(end_items(&ends)).show_with_end(ui);
    if let Some(pressed) = pressed {
        changes.extend(page.end_change(ui, &ends, pressed));
    }
    if let Some(on) = changed {
        changes.push(Change::Set {
            tool,
            control: id,
            value: ControlValue::Bool(on),
            done: true,
        });
    }
}

/// What a row shows after its label of the photo's value (decision 88):
/// `tool-<tool>-control-<control>-label-value`, its `$value` the number in the
/// language's format, as its `ValueFormat` writes it, with its unit, "−⅔ EV".
fn label_value(words: &Translator, tool: &str, control: &str, value: &LabelValue) -> String {
    let number =
        value.write(|number, decimals, signed| words.format_number(number, decimals, signed));
    let mut args = Args::new();
    args.set("value", number);
    let with_unit = words.text_with(&messages::unit_value(value.unit), Some(&args));
    let mut args = Args::new();
    args.set("value", with_unit);
    words.text_with(&messages::label_value(tool, control), Some(&args))
}

/// A curve of points: the points from the mirror between the curve's ends, (0, 0) and
/// (1, 1), drawn through the curve the tool says it renders (`ProcessingTool::curve`),
/// straight lines if it says none. Moving, adding and removing a point set the whole list,
/// which holds the points between the ends; the ends stay where they are, so moving one
/// sends nothing. Resetting the curve resets its control.
fn curve_row(
    ui: &mut egui::Ui,
    words: &Translator,
    tool: &dyn ProcessingTool,
    id: &'static str,
    state: &ToolState,
    changes: &mut Vec<Change>,
) {
    let definition = tool.definition();
    let Some(shown) = control(state, id) else {
        return;
    };
    let ControlValue::Points(points) = &shown.value else {
        return;
    };
    let full = words.text(&messages::full(definition.tool.id, id));
    // The widget's points are the ends with the control's between them: its point i is
    // the control's i − 1.
    let pairs: Vec<(f32, f32)> = std::iter::once((0.0, 0.0))
        .chain(points.iter().map(|&[x, y]| (x, y)))
        .chain(std::iter::once((1.0, 1.0)))
        .collect();
    let between = |index: usize| (1..=points.len()).contains(&index).then(|| index - 1);
    let mut values = Controls::new();
    for shown in &state.controls {
        values.set(shown.control, shown.value.clone());
    }
    let drawn = tool.curve(id, &values);
    let mut curve = Curve::new(&full, &pairs)
        .max_points(MAX_POINTS + 2)
        .enabled(shown.available);
    if let Some(drawn) = &drawn {
        curve = curve.function(drawn.as_ref());
    }
    let tool = definition.tool.id;
    let mut points = points.clone();
    // The handles come with the controls they set; this curve draws none.
    let done = match curve.show(ui) {
        Some(CurveAction::Move { index, x, y, done }) => {
            let Some(i) = between(index) else {
                return;
            };
            points[i] = [x, y];
            done
        }
        Some(CurveAction::Add { x, y }) => {
            points.insert(points.partition_point(|p| p[0] < x), [x, y]);
            true
        }
        Some(CurveAction::Remove(index)) => {
            let Some(i) = between(index) else {
                return;
            };
            points.remove(i);
            true
        }
        Some(CurveAction::Reset) => {
            return changes.push(Change::Reset { tool, control: id });
        }
        _ => return,
    };
    changes.push(Change::Set {
        tool,
        control: id,
        value: ControlValue::Points(points),
        done,
    });
}

/// How many points darktable samples its tone equalizer's graph at (`UI_SAMPLES`).
const EQUALIZER_SAMPLES: usize = 256;

/// darktable's graph of a tone equalizer's bands (`Widget::Equalizer`): the bands from the
/// mirror, each node named by its band, the curve the tool says the operation renders
/// through them (`ProcessingTool::curve` of the first) sampled as darktable samples its
/// graph, in the warning colour where the tool says none fits, and a drag reaching as far
/// as the tool says the curve is wide (`ProcessingTool::curve_width`), each step of it
/// settling the bands on the curve the tool says they give, as darktable's does. It draws
/// no histogram until the engine measures the mask (decision 19). Moving the bands sets
/// each band whose value, rounded to the decimals of the band's own row, changes, no key
/// moving one by less than the last of them, and the release ends the gesture on each band
/// it moved, as one history step with the drag; a reset resets the bands, one or all, all
/// as one step. The tool's code runs on the UI thread, so a panic in it draws no curve and
/// moves nothing, as a pending change's `view` and `unview` are guarded.
fn equalizer_row(
    ui: &mut egui::Ui,
    words: &Translator,
    tool: &dyn ProcessingTool,
    controls: &[&'static str],
    state: &ToolState,
    changes: &mut Vec<Change>,
) {
    let definition = tool.definition();
    let id = definition.tool.id;
    // Each band as the mirror shows it, with its range, its row's decimals and its step.
    let shown: Option<Vec<Band>> = controls
        .iter()
        .map(|&band| Band::of(definition, state, band))
        .collect();
    let (Some(shown), Some(&first)) = (shown, controls.first()) else {
        return;
    };
    let bands: Vec<f32> = shown.iter().map(|band| band.value).collect();
    let names: Vec<String> = controls
        .iter()
        .map(|band| words.text(&messages::full(id, band)))
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut values = Controls::new();
    for shown in &state.controls {
        values.set(shown.control, shown.value.clone());
    }
    // The bands lie 1 EV apart up to +0 EV.
    let start = -(bands.len().saturating_sub(1) as f32);
    let drawn = equalizer_curve(tool, state.enabled, first, &values, start);
    let width = catch_unwind(AssertUnwindSafe(|| tool.curve_width(first, &values)));
    let mut args = Args::new();
    args.set("tool", words.text(&messages::title(id)));
    let name = words.text_with("equalizer-name", Some(&args));
    let mut graph = Equalizer::new(&name, &bands, &names)
        .step(shown[0].step)
        .decimals(usize::from(shown[0].decimals))
        .enabled(shown.iter().any(|band| band.available));
    graph = match &drawn {
        Ok(Some(samples)) => graph.curve(samples),
        Ok(None) => graph.failed(true),
        Err(_) => graph.no_curve(),
    };
    if let Ok(Some(width)) = width {
        graph = graph.smoothing(width);
    }
    // As darktable's drag, each step settles the bands on the curve they give, its
    // correction at each node; none where the tool says no curve fits them, or where its
    // code panics, so a drag then moves nothing.
    let guarded = drawn.is_ok() && width.is_ok();
    let settle = |moved: &[f32]| -> Option<Vec<f32>> {
        if !guarded {
            return None;
        }
        let mut values = values.clone();
        for (&control, &value) in controls.iter().zip(moved) {
            values.set(control, ControlValue::Number(value));
        }
        catch_unwind(AssertUnwindSafe(|| {
            let curve = tool.curve(first, &values)?;
            Some((0..moved.len()).map(|i| curve(start + i as f32)).collect())
        }))
        .ok()
        .flatten()
    };
    graph = graph.settle(&settle);
    match graph.show(ui) {
        Some(EqualizerAction::Move { from, to, done }) => {
            let mut moved = Vec::new();
            for ((band, &control), (&from, &to)) in
                shown.iter().zip(controls).zip(from.iter().zip(&to))
            {
                let value = band.rounded(to);
                // While dragged, what moves; on release, everything the gesture moved.
                if value != band.value || (done && value != band.rounded(from)) {
                    moved.push(Change::Set {
                        tool: id,
                        control,
                        value: ControlValue::Number(value),
                        done,
                    });
                }
            }
            // The release, one step with the drag (decision 31).
            if done {
                changes.extend(one_step(moved));
            } else {
                changes.extend(moved);
            }
        }
        Some(EqualizerAction::ResetBand(i)) => {
            if let Some(&control) = controls.get(i) {
                changes.push(Change::Reset { tool: id, control });
            }
        }
        Some(EqualizerAction::Reset) => {
            let resets = controls
                .iter()
                .map(|&control| Change::Reset { tool: id, control });
            changes.extend(one_step(resets.collect()));
        }
        None => {}
    }
}

/// How many points darktable samples its colour equalizer's graph at, one a degree.
const HUE_CURVE_SAMPLES: usize = 361;

/// darktable's graph of one of a colour equalizer's curves (`Widget::HueCurve`): the
/// nodes from the mirror, each named by its node, around the hue circle from where the
/// tool says the first sits (`ProcessingTool::curve_offset`), the curve the tool says the
/// operation renders through them (`ProcessingTool::curve` of the first) sampled as
/// darktable samples its graph, and the colours the tool paints (`ProcessingTool::curve_paint`).
/// A node moved sets its control, rounded to the decimals of its own row, the release
/// ending the gesture; a reset resets one node or all, all as one step. The tool's code
/// runs on the UI thread, so a panic in it draws no curve and moves nothing.
fn hue_curve_row(
    ui: &mut egui::Ui,
    words: &Translator,
    tool: &dyn ProcessingTool,
    controls: &[&'static str],
    state: &ToolState,
    wheel: Wheel,
    changes: &mut Vec<Change>,
) {
    let definition = tool.definition();
    let id = definition.tool.id;
    let shown: Option<Vec<Band>> = controls
        .iter()
        .map(|&node| Band::of(definition, state, node))
        .collect();
    let (Some(shown), Some(&first)) = (shown, controls.first()) else {
        return;
    };
    let nodes: Vec<f32> = shown.iter().map(|node| node.value).collect();
    let names: Vec<String> = controls
        .iter()
        .map(|node| words.text(&messages::full(id, node)))
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut values = Controls::new();
    for shown in &state.controls {
        values.set(shown.control, shown.value.clone());
    }
    // While the tool is off it draws its defaults, flat, as the equalizer does.
    let default = match definition.control(first).map(|c| c.kind) {
        Some(ControlKind::Number { default, .. }) => default,
        _ => 0.0,
    };
    let drawn = catch_unwind(AssertUnwindSafe(|| {
        if !state.enabled {
            return Some(vec![default; HUE_CURVE_SAMPLES]);
        }
        let curve = tool.curve(first, &values)?;
        let last = (HUE_CURVE_SAMPLES - 1) as f32;
        Some(
            (0..HUE_CURVE_SAMPLES)
                .map(|k| curve(360.0 * k as f32 / last))
                .collect::<Vec<f32>>(),
        )
    }));
    let offset = catch_unwind(AssertUnwindSafe(|| tool.curve_offset(first, &values)));
    let paint = catch_unwind(AssertUnwindSafe(|| tool.curve_paint(first, &values)))
        .ok()
        .flatten();
    let colour = |[r, g, b]: [f32; 3]| {
        let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        egui::Color32::from_rgb(byte(r), byte(g), byte(b))
    };
    let strip: Vec<egui::Color32> = paint
        .iter()
        .flat_map(|paint| paint.strip.iter().copied().map(colour))
        .collect();
    let background: Vec<egui::Color32> = paint
        .iter()
        .flat_map(|paint| paint.background.iter().copied().map(colour))
        .collect();
    let mut args = Args::new();
    args.set("tool", words.text(&messages::title(id)));
    let name = words.text_with("equalizer-name", Some(&args));
    let unit = definition
        .control(first)
        .and_then(|c| c.unit)
        .map(messages::unit_value);
    let mut graph = HueCurve::new(&name, &nodes, &names, [shown[0].min, shown[0].max])
        .step(shown[0].step)
        .decimals(usize::from(shown[0].decimals))
        .wheel(wheel)
        .strip(&strip)
        .enabled(shown.iter().any(|node| node.available));
    if let Some(paint) = &paint {
        graph = graph.background(&background, paint.columns);
    }
    if let Some(unit) = &unit {
        graph = graph.unit(unit);
    }
    if let Ok(Some(offset)) = offset {
        graph = graph.offset(offset);
    }
    let guarded = drawn.is_ok() && offset.is_ok();
    graph = match &drawn {
        Ok(Some(samples)) => graph.curve(samples),
        Ok(None) => graph,
        Err(_) => graph.no_curve(),
    };
    if !guarded {
        graph = graph.enabled(false);
    }
    match graph.show(ui) {
        Some(HueCurveAction::Move { node, value, done }) => {
            let (Some(band), Some(&control)) = (shown.get(node), controls.get(node)) else {
                return;
            };
            let value = band.rounded(value);
            // While dragged, what moves; on release, the value the gesture ends at.
            if value != band.value || done {
                changes.push(Change::Set {
                    tool: id,
                    control,
                    value: ControlValue::Number(value),
                    done,
                });
            }
        }
        Some(HueCurveAction::ResetNode(i)) => {
            if let Some(&control) = controls.get(i) {
                changes.push(Change::Reset { tool: id, control });
            }
        }
        Some(HueCurveAction::Reset) => {
            let resets = controls
                .iter()
                .map(|&control| Change::Reset { tool: id, control });
            changes.extend(one_step(resets.collect()));
        }
        None => {}
    }
}

/// `changes` as one history step: the change itself where there is one, else
/// [`Change::Step`].
fn one_step(mut changes: Vec<Change>) -> Option<Change> {
    match changes.len() {
        0 => None,
        1 => changes.pop(),
        _ => Some(Change::Step(changes)),
    }
}

/// One band of a band graph as the mirror shows it, with what its own row would round a
/// value to.
struct Band {
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    decimals: u8,
    available: bool,
}

impl Band {
    /// The number control `id`, its decimals those of its own slider row.
    fn of(definition: &ProcessingDefinition, state: &ToolState, id: &'static str) -> Option<Self> {
        let shown = control(state, id)?;
        let (Some(ControlKind::Number { min, max, step, .. }), ControlValue::Number(value)) =
            (definition.control(id).map(|c| c.kind), &shown.value)
        else {
            return None;
        };
        let decimals = definition
            .rows()
            .find_map(|row| match row {
                Row::Slider(slider) if slider.control == id => Some(slider.decimals),
                _ => None,
            })
            .unwrap_or(Slider::new(id).decimals);
        Some(Self {
            value: *value,
            min,
            max,
            step,
            decimals,
            available: shown.available,
        })
    }

    /// `value` as the band's row shows it, in its range.
    fn rounded(&self, value: f32) -> f32 {
        shown_as(f64::from(value), self.decimals, self.min, self.max)
    }
}

fn mark(mark: Option<Mark>) -> parts::Mark {
    match mark {
        Some(Mark::Kept) => parts::Mark::Kept,
        Some(Mark::Auto) => parts::Mark::Auto,
        None => parts::Mark::None,
    }
}

/// The design system's track for a definition's, over `span`, what the track spans in the
/// row's units: the lightness ramp's greys follow it. Saturation and luminance bands are
/// not drawn yet, so those tracks are plain (`docs/gui.md`, the renderer's gaps).
fn track(track: Track, (min, max): (f32, f32)) -> parts::Track {
    match track {
        Track::Temperature => parts::Track::temperature(),
        Track::Tint => parts::Track::tint(),
        Track::Hue => parts::Track::hue(),
        Track::Lightness => lightness_ramp(min, max),
        Track::Plain | Track::Saturation | Track::Luminance => parts::Track::Plain,
    }
}

#[cfg(test)]
mod tests {
    use apertrust_engine::session::PresetState;
    use apertrust_tools::agx::Agx;
    use apertrust_tools::colour_balance_rgb::ColourBalanceRgb;
    use apertrust_tools::colour_calibration::ColourCalibration;
    use apertrust_tools::demosaic::Demosaic;
    use apertrust_tools::exposure::Exposure;
    use apertrust_tools::highlight_reconstruction::HighlightReconstruction;
    use apertrust_tools::input_colour_profile::InputColourProfile;
    use apertrust_tools::output_colour_profile::OutputColourProfile;
    use apertrust_tools::rgb_primaries::RgbPrimaries;
    use apertrust_tools::testing::{self, Sample, Twin};
    use apertrust_tools::tone_curve::{self, ToneCurve};
    use apertrust_tools::tone_equalizer::{self, ToneEqualizer};
    use apertrust_tools::vignetting::Vignetting;
    use apertrust_tools::white_balance::WhiteBalance;
    use apertrust_tools::{Moment, Reset, ToolDefinition};
    use apertrust_ui::components::Marker;
    use apertrust_ui::{Builtin, Locale, Theme, theme};
    use egui::accesskit::{Role, Toggled};
    use egui::{Key, Modifiers, pos2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    use super::*;
    use apertrust_engine::session::{ActionState, WarningState};

    /// The point `at` of the way along a slider's `track`, as the marker travels it: short of
    /// each end by the marker's stop (decision 75).
    fn along(track: egui::Rect, at: f32) -> egui::Pos2 {
        let travel = Marker::default().travel(track.x_range());
        pos2(travel.min + travel.span() * at, track.center().y)
    }

    /// A tool's controls at their definition's defaults, as the session reports them
    /// for an untouched photo, the sample photo.
    fn untouched(tool: &dyn ProcessingTool) -> ToolState {
        let definition = tool.definition();
        let mut state = ToolState {
            tool: definition.tool.id,
            enabled: !definition.switched,
            edited: false,
            hidden: false,
            controls: definition
                .controls
                .iter()
                .map(|control| ControlState {
                    control: control.id,
                    value: control.default(),
                    default: control.default(),
                    mark: None,
                    available: true,
                    options: tool.options(control.id, &testing::image()),
                    range: tool.range(control.id, &definition.defaults()),
                    visible: tool.visible(control.id, &testing::image()),
                    label_value: tool.label_value(control.id, &testing::image()),
                })
                .collect(),
            presets: Vec::new(),
            views: views(tool),
            computed: Vec::new(),
            readouts: Vec::new(),
            actions: Vec::new(),
            warnings: Vec::new(),
        };
        state.presets = presets(tool, &state);
        report_availability(tool, &mut state);
        report_readouts(tool, &mut state);
        state
    }

    /// Each readout of the tool's lines of pickers as the session reports it for the values
    /// `state` shows on the sample photo.
    fn report_readouts(tool: &dyn ProcessingTool, state: &mut ToolState) {
        let (image, values) = (testing::image(), values(state));
        state.readouts = (tool.definition().readouts())
            .map(|readout| apertrust_engine::session::ReadoutState {
                id: readout.id,
                readout: tool.readout(readout.id, &values, &image),
            })
            .collect();
    }

    /// Each control's availability as the session reports it for the values `state`
    /// shows on the sample photo: what the tool says, never for a control not ported.
    fn report_availability(tool: &dyn ProcessingTool, state: &mut ToolState) {
        let (image, values) = (testing::image(), values(state));
        for (control, shown) in tool.definition().controls.iter().zip(&mut state.controls) {
            shown.available = !control.unported && tool.available(control.id, &values, &image);
        }
        state.warnings = (tool.definition().warnings().into_iter())
            .map(|id| WarningState {
                id,
                on: tool.warning(id, &values, &image),
            })
            .collect();
        state.actions = (tool.definition().action_ids().into_iter())
            .map(|(id, unported)| ActionState {
                id,
                available: !unported && tool.available(id, &values, &image),
            })
            .collect();
    }

    /// A header's tooltip: a utility edits nothing and says what it measures, whatever
    /// facts it is given; darktable's tool has its rows; a tool that is not darktable's
    /// has its sentence alone.
    #[test]
    fn a_header_s_tooltip_tells_utilities_darktable_s_tools_and_others_apart() {
        let words = Translator::new(apertrust_ui::Locale::EnGb);
        let scopes = about(&words, "scopes", None);
        assert_eq!(scopes.kind.as_deref(), Some("Edits nothing of its own"));
        assert_eq!(
            scopes.place,
            Some(parts::Place::Measures("The finished picture".to_owned()))
        );
        assert_eq!(scopes.facts, None);
        let history = about(&words, "history", Exposure.definition().facts.as_ref());
        assert_eq!(history.kind.as_deref(), Some("Edits nothing of its own"));
        assert_eq!((history.place, history.facts), (None, None));

        let exposure = about(&words, "exposure", Exposure.definition().facts.as_ref());
        assert_eq!(
            exposure.kind.as_deref(),
            Some("Fixes a problem and shapes the look")
        );
        assert_eq!(
            exposure.place,
            Some(parts::Place::WorksOn("The light as captured".to_owned()))
        );
        assert_eq!(
            exposure.facts.map(|lines| lines[1].clone()).as_deref(),
            Some("linear, RGB")
        );

        let sample = about(&words, "sample", Sample.definition().facts.as_ref());
        assert_eq!(sample.description, words.text("tool-sample-description"));
        assert_eq!(
            (sample.kind, sample.place, sample.facts),
            (None, None, None)
        );
    }

    /// The tool's views' coordinates at its defaults, as the session reports them for
    /// an untouched photo.
    fn views(tool: &dyn ProcessingTool) -> Vec<CoordinateState> {
        let image = testing::image();
        let defaults = tool.defaults(&image);
        let mut views = Vec::new();
        for view in tool.definition().views {
            let coordinates = tool.view(view.id, &defaults, &image).unwrap();
            for (coordinate, value) in view.coordinates.iter().zip(coordinates) {
                views.push(CoordinateState {
                    view: view.id,
                    coordinate: coordinate.id,
                    value,
                    default: value,
                    mark: None,
                    available: true,
                });
            }
        }
        views
    }

    /// What the app shows of a change it sends until a report includes it, if anything.
    fn pending(change: &Change) -> Option<Pending> {
        Some(match *change {
            Change::Set {
                tool,
                control,
                ref value,
                ..
            } => Pending::Control {
                tool,
                control,
                value: value.clone(),
            },
            Change::Reset { tool, control } => Pending::Reset { tool, control },
            Change::Keep {
                tool,
                control,
                kept,
            } => Pending::Keep {
                tool,
                control,
                kept,
            },
            Change::ResetTool { tool } => Pending::ResetTool { tool },
            _ => return None,
        })
    }

    /// The tool's preset rows while its controls hold what `state` shows, as the
    /// session reports them for the sample photo, which records no exposure bias.
    fn presets(tool: &dyn ProcessingTool, state: &ToolState) -> Vec<PresetState> {
        let image = testing::image();
        let mut values = Controls::new();
        for shown in &state.controls {
            values.set(shown.control, shown.value.clone());
        }
        tool.definition()
            .preset_rows
            .iter()
            .map(|row| PresetState {
                preset: row.id,
                matching: tool.matching(row.id, &values, &image),
                // As the session lists them: an option that turns a picker on among them,
                // and one the tool's memory as the photo opens gives.
                available: row
                    .options
                    .iter()
                    .copied()
                    .filter(|option| {
                        let memory =
                            tool.remember(Moment::Opened, &Controls::new(), &values, &image);
                        row.pickers.iter().any(|(id, _)| id == option)
                            || tool.preset(row.id, option, &image).is_some()
                            || tool.recall(row.id, option, &memory, &image).is_some()
                    })
                    .collect(),
            })
            .collect()
    }

    /// The sample photo with a camera bias of −⅔ EV and 1 EV held back by a
    /// highlight-preserving mode, which Exposure's camera rows act on (decision 88).
    fn camera_photo() -> ImageContext {
        let mut image = testing::image();
        let camera = image.camera.as_mut().expect("a raw's");
        camera.exposure_bias = -2.0 / 3.0;
        camera.highlight_preservation = 1.0;
        image
    }

    /// `state` as the session reports it on `image`: each row shown or hidden, the values
    /// after their labels, and what each can change, as the tool says for that photo.
    fn on_photo(tool: &dyn ProcessingTool, state: &mut ToolState, image: &ImageContext) {
        let values = values(state);
        for (definition, control) in tool.definition().controls.iter().zip(&mut state.controls) {
            control.visible = tool.visible(control.control, image);
            control.label_value = tool.label_value(control.control, image);
            control.options = tool.options(control.control, image);
            // As the session reports it: a choice the photo lists only the held option of
            // is unavailable too (decision 30).
            control.available = !definition.unported
                && tool.available(control.control, &values, image)
                && !check::lists_only(definition, &control.options, &control.value);
        }
    }

    fn set(state: &mut ToolState, id: &str, value: ControlValue, mark: Option<Mark>) {
        let shown = state.controls.iter_mut().find(|c| c.control == id).unwrap();
        shown.value = value;
        shown.mark = mark;
        state.edited = state.controls.iter().any(|c| c.value != c.default);
    }

    /// One tool's section and what it reported, its values changed as the app's mirror
    /// shows them before the engine reports ([`Pending::show`]), an option of a preset row
    /// applied and its preset rows reported each frame as the session would, its groups
    /// opened and closed as the app's shell keeps them.
    struct Panel {
        tool: &'static dyn ProcessingTool,
        state: ToolState,
        open: bool,
        panels: Panels,
        groups: BTreeMap<&'static str, bool>,
        changes: Vec<Change>,
        /// The picker on, and why the pickers cannot be pressed.
        on: Option<PickerRef>,
        why: Option<&'static str>,
        /// ⋯'s entries, as the shell gives them; none by default.
        options: Vec<popup::Entry>,
    }

    fn panel(tool: &'static dyn ProcessingTool, state: ToolState) -> Harness<'static, Panel> {
        Harness::builder()
            .with_size(egui::vec2(400.0, 300.0))
            // Frames a sixtieth of a second apart, so two clicks make a double-click.
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
            .build_ui_state(
                |ui, panel: &mut Panel| {
                    panel.state.presets = presets(panel.tool, &panel.state);
                    let groups =
                        |group: &str| panel.groups.get(group).copied().unwrap_or(GROUP_OPEN);
                    // Sample's pickers, and White balance's, which Settings turns on.
                    let pickers = Pickers {
                        drawn: &["sample", "temperature"],
                        on: panel.on,
                        enabled: panel.why.is_none(),
                        why: panel.why,
                        ..Pickers::NONE
                    };
                    let changes = section(
                        ui,
                        panel.tool,
                        &panel.state,
                        Head {
                            open: panel.open,
                            options: panel.options.clone(),
                            enabled: true,
                            ..Head::default()
                        },
                        &panel.panels,
                        &groups,
                        true,
                        None,
                        &pickers,
                    );
                    for change in &changes {
                        panel.panels.apply(change);
                        if matches!(change, Change::Toggle { .. }) {
                            panel.open = !panel.open;
                        }
                        let shown: Vec<Pending> = match change {
                            Change::Step(changes) => changes.iter().filter_map(pending).collect(),
                            change => pending(change).into_iter().collect(),
                        };
                        for pending in shown {
                            pending.show(panel.tool, &mut panel.state, &testing::image());
                        }
                        match change {
                            Change::Group { group, .. } => {
                                let open = panel.groups.entry(group).or_insert(GROUP_OPEN);
                                *open = !*open;
                            }
                            Change::ApplyPreset { preset, option, .. } => {
                                let image = testing::image();
                                let tool = panel.tool;
                                let memory = tool.remember(
                                    Moment::Opened,
                                    &Controls::new(),
                                    &values(&panel.state),
                                    &image,
                                );
                                let values = (tool.preset(preset, option, &image))
                                    .or_else(|| tool.recall(preset, option, &memory, &image))
                                    .unwrap();
                                let row = panel.tool.definition().preset(preset).unwrap();
                                for control in row.controls {
                                    let value = values.get(control).unwrap().clone();
                                    set(&mut panel.state, control, value, Some(Mark::Kept));
                                }
                            }
                            _ => {}
                        }
                    }
                    panel.changes.extend(changes);
                },
                Panel {
                    tool,
                    state,
                    // Opened, as the person would, to show its page: every processing
                    // tool is closed at first (`ToolDefinition::open_at_first`).
                    open: true,
                    panels: Panels::default(),
                    groups: BTreeMap::new(),
                    changes: Vec::new(),
                    on: None,
                    why: None,
                    options: Vec::new(),
                },
            )
    }

    #[test]
    fn dragging_a_slider_sets_its_control_and_its_release_is_done() {
        let mut harness = panel(&Exposure, untouched(&Exposure));
        harness.run();
        let track = harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .rect();
        // The track spans darktable's soft range, −3 to +4 EV.
        let at = along(track, 0.75);
        harness.drag_at(at);
        harness.run();
        harness.drop_at(at);
        harness.run();
        let set = |done| Change::Set {
            tool: "exposure",
            control: "exposure",
            value: ControlValue::Number(2.25),
            done,
        };
        assert_eq!(harness.state().changes, [set(false), set(true)]);
        // Screen readers hear the value with its unit.
        let node = harness.get_by_role_and_label(Role::Slider, "Exposure");
        assert_eq!(node.accesskit_node().value().as_deref(), Some("+2.250 EV"));
    }

    /// Drags `name`'s track at `at`, a fraction of its width, and returns the value it
    /// sent on release.
    fn drag_to(harness: &mut Harness<'static, Panel>, name: &str, at: f32) -> ControlValue {
        let track = harness.get_by_role_and_label(Role::Slider, name).rect();
        let at = along(track, at);
        harness.drag_at(at);
        harness.run();
        harness.drop_at(at);
        harness.run();
        match harness.state().changes.last() {
            Some(Change::Set {
                value, done: true, ..
            }) => value.clone(),
            other => panic!("{other:?}"),
        }
    }

    /// Types `text` into the slider `name`'s value field as the keyboard does: Enter on its
    /// focused track opens the field, its number selected, then Enter sets it.
    fn type_into(harness: &mut Harness<'_, Panel>, name: &str, text: &str) {
        harness.get_by_role_and_label(Role::Slider, name).focus();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        harness.event(egui::Event::Text(text.to_owned()));
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
    }

    #[test]
    fn a_track_spans_darktable_s_soft_range_and_a_typed_value_reaches_the_range() {
        // Global vibrance holds ±100 % and its track spans darktable's ±50 %.
        let name = "Global vibrance";
        let mut harness = panel(&ColourBalanceRgb, untouched(&ColourBalanceRgb));
        harness.run();
        let percent = |value| ControlValue::Number(value);
        assert_eq!(drag_to(&mut harness, name, 1.0), percent(50.0));
        assert_eq!(drag_to(&mut harness, name, 0.75), percent(25.0));
        // Screen readers are told the range a value may hold.
        let node = harness.get_by_role_and_label(Role::Slider, name);
        let node = node.accesskit_node();
        assert_eq!(
            (node.min_numeric_value(), node.max_numeric_value()),
            (Some(-100.0), Some(100.0))
        );
        // A typed value beyond the soft range reaches the range, and the track widens to
        // show it: from −50 % to 80 %, so three quarters of it is 47.5 %.
        type_into(&mut harness, name, "80");
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set {
                tool: "colorbalancergb",
                control: "vibrance",
                value: percent(80.0),
                done: true,
            })
        );
        assert_eq!(drag_to(&mut harness, name, 0.75), percent(47.5));
        // It keeps that span, as darktable's does, until the row is reset.
        assert_eq!(drag_to(&mut harness, name, 1.0), percent(80.0));
        harness.get_by_role_and_label(Role::Slider, name).focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(drag_to(&mut harness, name, 1.0), percent(50.0));
        // Typed beyond the range, it stops at the range's end.
        type_into(&mut harness, name, "150");
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set {
                tool: "colorbalancergb",
                control: "vibrance",
                value: percent(100.0),
                done: true,
            })
        );
    }

    #[test]
    fn delete_or_a_double_click_resets_a_control() {
        let mut state = untouched(&Exposure);
        set(
            &mut state,
            "exposure",
            ControlValue::Number(1.0),
            Some(Mark::Kept),
        );
        let mut harness = panel(&Exposure, state);
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "Exposure")
            .focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        let reset = Change::Reset {
            tool: "exposure",
            control: "exposure",
        };
        assert_eq!(harness.state().changes, std::slice::from_ref(&reset));

        // Set again, then two clicks on the track a frame apart.
        set(
            &mut harness.state_mut().state,
            "exposure",
            ControlValue::Number(1.0),
            Some(Mark::Kept),
        );
        harness.state_mut().changes.clear();
        for _ in 0..30 {
            harness.step();
        }
        for _ in 0..2 {
            harness
                .get_by_role_and_label(Role::Slider, "Exposure")
                .click();
            harness.step();
        }
        harness.run();
        assert_eq!(harness.state().changes.last(), Some(&reset));
    }

    #[test]
    fn a_key_step_finer_than_the_row_shows_moves_by_its_last_decimal() {
        // Sample's brightness steps by 0.01 and shows two decimals; Sample has no
        // messages, so its words are its message ids.
        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "tool-sample-control-brightness-full")
            .focus();
        harness.run();
        // Ctrl steps a tenth of 0.01, under the two decimals the row shows: it moves by
        // 0.01, darktable's `min_visible`, rather than by nothing that would show.
        harness.key_press_modifiers(Modifiers::CTRL | Modifiers::COMMAND, Key::ArrowRight);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
        harness.run();
        let set = |value| Change::Set {
            tool: "sample",
            control: "brightness",
            value: ControlValue::Number(value),
            done: true,
        };
        assert_eq!(harness.state().changes, [set(0.01), set(0.11)]);
        // A value is still rounded to the decimals shown before it is sent.
        assert_eq!(shown_as(0.0004, 3, -0.1, 0.1), 0.0);
        assert_eq!(shown_as(0.0126, 2, -4.0, 4.0), 0.01);
        assert_eq!(shown_as(9.0, 2, -4.0, 4.0), 4.0);
    }

    /// A run of notches over a tool's slider, Ctrl with the wheel while it scrolls the Tools
    /// panel, is completed changes of one control, as its keys' are, which the session's
    /// history merges into one step (decision 78; the engine's
    /// `quick_changes_to_one_control_merge_into_one_step`); a plain scroll changes nothing.
    #[test]
    fn a_run_of_notches_is_completed_changes_of_one_control() {
        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        harness
            .get_by_role_and_label(Role::Slider, "tool-sample-control-brightness-full")
            .hover();
        harness.run();
        let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
        for held in [Modifiers::NONE, ctrl, ctrl, ctrl] {
            let event = egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: held,
            };
            harness.event_modifiers(event, held);
            harness.run_steps(4);
        }
        let set = |value| Change::Set {
            tool: "sample",
            control: "brightness",
            value: ControlValue::Number(value),
            done: true,
        };
        assert_eq!(harness.state().changes, [set(0.01), set(0.02), set(0.03)]);
    }

    #[test]
    fn the_marker_keeps_and_the_header_resets_the_tool() {
        let mut harness = panel(&Exposure, untouched(&Exposure));
        harness.run();
        harness.get_by_label("Keep Exposure").click();
        harness.run();
        harness.get_by_label("Reset Exposure").click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [
                Change::Keep {
                    tool: "exposure",
                    control: "exposure",
                    kept: true
                },
                Change::ResetTool { tool: "exposure" }
            ]
        );
    }

    /// A click on the header opens or closes the section, and says whether Shift was held,
    /// which does the opposite of Open one processing tool at a time; Shift+Return or Shift+Space
    /// on the focused header as Shift-click.
    #[test]
    fn the_header_s_toggle_says_whether_shift_was_held() {
        let mut harness = panel(&Exposure, untouched(&Exposure));
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Exposure")
            .click();
        harness.run();
        (harness.get_by_role_and_label(Role::Button, "Exposure"))
            .click_modifiers(egui::Modifiers::SHIFT);
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Exposure")
            .focus();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Enter);
        harness.run();
        let toggle = |shift| Change::Toggle {
            tool: "exposure",
            shift,
        };
        assert_eq!(
            harness.state().changes,
            [toggle(false), toggle(true), toggle(true)]
        );
    }

    /// A choice row has the kept marker a slider row has (decision 42): Highlight
    /// reconstruction's Method, kept once set, released by a click on its marker and kept
    /// again by another, the marker following at once as the session will.
    #[test]
    fn a_choice_row_s_marker_releases_and_keeps_its_control() {
        let mut kept = untouched(&HighlightReconstruction);
        set(&mut kept, "mode", ControlValue::Choice(0), Some(Mark::Kept));
        let mut harness = panel(&HighlightReconstruction, kept);
        harness.run();
        let marked = |harness: &Harness<'_, Panel>| {
            let marker = harness.get_by_role_and_label(Role::Button, "Keep Method");
            marker.accesskit_node().toggled()
        };
        assert_eq!(marked(&harness), Some(Toggled::True));
        harness.get_by_label("Keep Method").click();
        harness.run();
        assert_eq!(marked(&harness), Some(Toggled::False));
        harness.get_by_label("Keep Method").click();
        harness.run();
        let keep = |kept| Change::Keep {
            tool: "highlights",
            control: "mode",
            kept,
        };
        assert_eq!(harness.state().changes, [keep(false), keep(true)]);
        // Still the combo box it was, which a click on its marker never opens.
        harness.get_by_role_and_label(Role::ComboBox, "Method");
        assert!(harness.query_by_label("Clip highlights").is_none());
    }

    /// With no presets folder given and no ⋯ entries, as with no user data folder, the
    /// presets and options buttons are drawn disabled and do nothing; there is no Auto.
    #[test]
    fn without_a_presets_folder_or_options_both_are_disabled_and_there_is_no_auto() {
        let mut harness = panel(&Exposure, untouched(&Exposure));
        harness.run();
        for name in ["Exposure presets", "Exposure options"] {
            let button = harness.get_by_label(name);
            assert!(button.accesskit_node().is_disabled(), "{name}");
            button.click();
            harness.run();
        }
        assert!(harness.state().changes.is_empty());
        // A tool's header draws no Auto yet.
        assert!(harness.query_by_label("Auto").is_none());
    }

    /// `tool`'s section with ⋯ holding what the shell gives it (`menus::options`), the
    /// actions all able to run, and ⋯ opened.
    fn options_opened(
        tool: &'static dyn ProcessingTool,
        state: ToolState,
    ) -> Harness<'static, Panel> {
        let mut harness = panel(tool, state);
        let ctx = harness.ctx.clone();
        let keymap = actions::Keymap::defaults(ctx.os());
        let id = tool.definition().tool.id;
        let state = actions::tests::everything();
        harness.state_mut().options = crate::menus::options(&ctx, id, &keymap, &state);
        harness.run();
        let title = translator(&ctx).text(&messages::title(id));
        (harness.get_by_role_and_label(Role::Button, &format!("{title} options"))).click();
        harness.run();
        harness
    }

    /// A processing tool's ⋯ (decision 91): the app's Reset first, enabled as ↺ is and
    /// running as it does, then Copy Settings and Paste Settings, drawn disabled with
    /// their tip until they are built.
    #[test]
    fn a_tool_s_options_hold_reset_then_copy_and_paste_settings() {
        let mut harness = options_opened(&Exposure, untouched(&Exposure));
        let top = |h: &Harness<'_, Panel>, words: &str| {
            h.get_by_role_and_label(Role::MenuItem, words).rect().top()
        };
        assert!(top(&harness, "Reset") < top(&harness, "Copy Settings"));
        assert!(top(&harness, "Copy Settings") < top(&harness, "Paste Settings"));
        for words in ["Copy Settings", "Paste Settings"] {
            let item = harness.get_by_role_and_label(Role::MenuItem, words);
            assert!(item.accesskit_node().is_disabled(), "{words}");
            item.hover();
            // Past the tooltip's delay, at a sixtieth of a second a frame.
            harness.run_steps(60);
            harness.get_by_label("Coming in a later version");
            harness.get_by_role_and_label(Role::MenuItem, words).click();
            harness.run();
        }
        assert!(harness.state().changes.is_empty());
        let reset = harness.get_by_role_and_label(Role::MenuItem, "Reset");
        assert!(!reset.accesskit_node().is_disabled());
        reset.click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::ResetTool { tool: "exposure" }]
        );
        // A switched tool still off has nothing to reset: ⋯'s Reset is disabled, as ↺ is.
        let mut harness = options_opened(&Vignetting, untouched(&Vignetting));
        let header = harness.get_by_role_and_label(Role::Button, "Reset Vignetting");
        assert!(header.accesskit_node().is_disabled());
        let reset = harness.get_by_role_and_label(Role::MenuItem, "Reset");
        assert!(reset.accesskit_node().is_disabled());
        reset.click();
        harness.run();
        assert!(harness.state().changes.is_empty());
    }

    #[test]
    fn switching_tabs_shows_the_other_tab_s_rows() {
        let mut harness = panel(&ColourCalibration, untouched(&ColourCalibration));
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, "Green, input R")
                .is_none()
        );
        harness.get_by_role_and_label(Role::Tab, "G").click();
        harness.run();
        // G is the third of darktable's tabs, after CAT and R.
        assert_eq!(
            harness.state().changes,
            [Change::Tab {
                tool: "channelmixerrgb",
                tab: 2
            }]
        );
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, "Red, input R")
                .is_none()
        );
        harness.get_by_role_and_label(Role::Slider, "Green, input R");
        harness.get_by_role_and_label(Role::Switch, "Green, normalise channels");
    }

    #[test]
    fn exposure_is_the_board_s_page_on_one_tab() {
        let mut state = untouched(&Exposure);
        on_photo(&Exposure, &mut state, &camera_photo());
        let mut harness = panel(&Exposure, state);
        harness.run();
        assert!(harness.query_by_role(Role::Tab).is_none());
        // Mode is a Manual | Automatic switch (decision 88), both live on a raw.
        harness.get_by_role_and_label(Role::RadioGroup, "Mode");
        for option in ["Manual", "Automatic"] {
            let node = harness.get_by_role_and_label(Role::RadioButton, option);
            assert!(!node.accesskit_node().is_disabled(), "{option}");
        }
        // The camera's rows, with their values on the photo, heard after their names
        // (decision 88).
        for name in [
            "Compensate camera exposure −⅔ EV",
            "Highlight preservation mode +1.0 EV",
        ] {
            harness.get_by_role_and_label(Role::Switch, name);
        }
        for name in ["Exposure", "Black level correction"] {
            harness.get_by_role_and_label(Role::Slider, name);
        }
        // Picker target, a combo box, in place of the area exposure mapping, and
        // Lightness hidden while Middle grey, the default, is chosen (decision 88).
        let target = harness.get_by_role_and_label(Role::ComboBox, "Picker target");
        assert_eq!(
            target.accesskit_node().value().as_deref(),
            Some("Middle grey")
        );
        for gone in ["Area exposure mapping", "Area mode", "Target lightness"] {
            assert!(harness.query_by_label(gone).is_none(), "{gone}");
        }
        // The camera's rows first, as darktable's stack has them, then Exposure and
        // Picker target right under it.
        let top = |role, name| harness.get_by_role_and_label(role, name).rect().top();
        assert!(
            top(Role::Switch, "Highlight preservation mode +1.0 EV")
                < top(Role::Slider, "Exposure")
        );
        assert!(top(Role::Slider, "Exposure") < top(Role::ComboBox, "Picker target"));
        // Automatic's rows, which manual hides, and Contrast and its pivot, which left
        // Exposure for Colour balance RGB (decision 2).
        for hidden in ["Measure at", "Place it at", "Contrast", "Contrast pivot"] {
            assert!(harness.query_by_label(hidden).is_none(), "{hidden}");
        }
        assert!(harness.query_by_label_contains("This photo").is_none());
    }

    #[test]
    fn automatic_mode_shows_its_rows_and_the_computed_exposure() {
        let mut state = untouched(&Exposure);
        for control in &mut state.controls {
            if control.control == "mode" {
                control.value = ControlValue::Choice(1);
            }
        }
        state.computed = vec![apertrust_engine::session::ComputedState {
            id: "computed_exposure",
            value: Some(-1.234),
        }];
        let mut harness = panel(&Exposure, state);
        harness.run();
        // Percentile and Target level under the redesign's labels (decision 88).
        for name in ["Measure at", "Place it at", "Black level correction"] {
            harness.get_by_role_and_label(Role::Slider, name);
        }
        // Manual's rows hidden, the computed value after Place it at, darktable's
        // "computed EC: −1.23 EV" as This photo.
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, "Exposure")
                .is_none()
        );
        harness.get_by_role_and_label(Role::Label, "This photo −1.23 EV");
        // None yet: the label alone.
        harness.state_mut().state.computed[0].value = None;
        harness.run();
        harness.get_by_role_and_label(Role::Label, "This photo");
    }

    /// Exposure as the session reports it on an image that is not a raw, its Mode
    /// holding `mode`: the photo lists manual alone (decision 88).
    fn not_a_raw(mode: u32) -> ToolState {
        let mut state = untouched(&Exposure);
        set(&mut state, "mode", ControlValue::Choice(mode), None);
        on_photo(&Exposure, &mut state, &ImageContext::full(8, 8));
        state
    }

    /// On an image that is not a raw, the switch draws Automatic disabled, saying why
    /// (decision 88). Holding Manual there it has nothing else to offer: Manual is drawn
    /// as chosen, not disabled, and nothing else can be chosen; an edit pasted there
    /// holding Automatic shows it chosen and disabled, and Manual can be chosen.
    #[test]
    fn mode_s_automatic_is_disabled_on_an_image_that_is_not_a_raw() {
        let why = "Automatic needs a raw photo's histogram";
        let mut harness = panel(&Exposure, not_a_raw(0));
        harness.run();
        let group = harness.get_by_role_and_label(Role::RadioGroup, "Mode");
        assert!(!group.accesskit_node().is_disabled());
        let manual = harness.get_by_role_and_label(Role::RadioButton, "Manual");
        assert!(!manual.accesskit_node().is_disabled());
        assert_eq!(manual.accesskit_node().toggled(), Some(Toggled::True));
        let automatic = harness.get_by_role_and_label(Role::RadioButton, "Automatic");
        assert!(automatic.accesskit_node().is_disabled());
        assert_eq!(
            automatic.accesskit_node().description().as_deref(),
            Some(why)
        );
        automatic.click();
        harness
            .get_by_role_and_label(Role::RadioButton, "Manual")
            .click();
        harness.run();
        assert!(harness.state().changes.is_empty());

        // Pasted as Automatic: chosen, disabled, rendering as manual mode; Manual live.
        harness.state_mut().state = not_a_raw(1);
        harness.run();
        let group = harness.get_by_role_and_label(Role::RadioGroup, "Mode");
        assert!(!group.accesskit_node().is_disabled());
        let automatic = harness.get_by_role_and_label(Role::RadioButton, "Automatic");
        assert!(automatic.accesskit_node().is_disabled());
        assert_eq!(automatic.accesskit_node().toggled(), Some(Toggled::True));
        harness
            .get_by_role_and_label(Role::RadioButton, "Manual")
            .click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set {
                tool: "exposure",
                control: "mode",
                value: ControlValue::Choice(0),
                done: true
            }]
        );
    }

    /// On a photo that gives them nothing to do, Exposure's camera rows are not drawn
    /// and screen readers find no node for them; the other rows stay (decision 88).
    #[test]
    fn rows_the_photo_hides_are_not_drawn() {
        let mut harness = panel(&Exposure, untouched(&Exposure));
        harness.run();
        for name in ["Compensate camera exposure", "Highlight preservation mode"] {
            assert!(harness.query_by_label_contains(name).is_none(), "{name}");
        }
        harness.get_by_role_and_label(Role::Slider, "Exposure");
        // One the photo gives something to do shows, with its value: here the bias alone.
        let mut image = testing::image();
        image.camera.as_mut().unwrap().exposure_bias = 4.0 / 3.0;
        on_photo(&Exposure, &mut harness.state_mut().state, &image);
        harness.run();
        harness.get_by_role_and_label(Role::Switch, "Compensate camera exposure +1⅓ EV");
        assert!(
            harness
                .query_by_label_contains("Highlight preservation mode")
                .is_none()
        );
    }

    /// The value after a label is written in the language's numbers: Ukrainian's comma.
    #[test]
    fn a_label_s_value_is_written_in_the_language_s_numbers() {
        let words = Translator::new(apertrust_ui::i18n::Locale::Uk);
        let given_back = Exposure
            .label_value("compensate_hilite_pres", &camera_photo())
            .unwrap();
        assert_eq!(
            label_value(&words, "exposure", "compensate_hilite_pres", &given_back),
            "+1,0 EV"
        );
        let bias = Exposure
            .label_value("camera_bias", &camera_photo())
            .unwrap();
        assert_eq!(
            label_value(&words, "exposure", "camera_bias", &bias),
            "−⅔ EV"
        );
    }

    #[test]
    fn a_control_that_cannot_change_the_photo_is_disabled() {
        let mut state = untouched(&Exposure);
        on_photo(&Exposure, &mut state, &camera_photo());
        for control in &mut state.controls {
            control.available = control.control != "camera_bias";
        }
        let mut harness = panel(&Exposure, state);
        harness.run();
        let name = "Compensate camera exposure −⅔ EV";
        let toggle = harness.get_by_role_and_label(Role::Switch, name);
        assert!(toggle.accesskit_node().is_disabled());
        toggle.click();
        harness.run();
        assert!(harness.state().changes.is_empty());
        // Where it can change the photo, it sets its control.
        for control in &mut harness.state_mut().state.controls {
            control.available = true;
        }
        harness.run();
        harness.get_by_role_and_label(Role::Switch, name).click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set {
                tool: "exposure",
                control: "camera_bias",
                value: ControlValue::Bool(false),
                done: true
            }]
        );
    }

    /// Sample's coarseness, which acts only on coarse grain (decision 30), by its full
    /// name. Sample has no messages, so its words are its message ids.
    const COARSENESS: &str = "tool-sample-control-coarseness-full";

    /// Sample's coarseness reaches up to 100 % plus the bias (decision 30): a pending bias
    /// moves its range at once and shows it pushed inside, kept, as the session will store
    /// it; its row then spans the moved range, which screen readers are told and a drag to
    /// the track's end reaches.
    #[test]
    fn a_pending_change_moves_a_range_that_follows_it_and_shows_its_clamp() {
        let mut state = untouched(&Sample);
        set(&mut state, "coarseness", ControlValue::Number(150.0), None);
        let pending = Pending::Control {
            tool: "sample",
            control: "bias",
            value: ControlValue::Number(20.0),
        };
        pending.show(&Sample, &mut state, &testing::image());
        let shown = control(&state, "coarseness").unwrap();
        assert_eq!(
            (shown.value.clone(), shown.mark),
            (ControlValue::Number(120.0), Some(Mark::Kept))
        );
        assert_eq!(
            shown.range.map(|range| (range.max, range.soft)),
            Some((120.0, (50.0, 120.0)))
        );
        // Live, as on coarse grain that is on, so it takes a drag.
        control_mut(&mut state, "coarseness").unwrap().available = true;
        let mut harness = panel(&Sample, state);
        harness.run();
        let node = harness.get_by_role_and_label(Role::Slider, COARSENESS);
        let node = node.accesskit_node();
        assert_eq!(
            (node.min_numeric_value(), node.max_numeric_value()),
            (Some(50.0), Some(120.0))
        );
        assert_eq!(
            drag_to(&mut harness, COARSENESS, 1.0),
            ControlValue::Number(120.0)
        );
        assert_eq!(
            drag_to(&mut harness, COARSENESS, 0.5),
            ControlValue::Number(85.0)
        );
    }

    /// Whether Sample's coarseness slider is drawn disabled, what screen readers hear of
    /// its value, and the value the panel shows.
    fn coarseness(harness: &Harness<'_, Panel>) -> (bool, Option<String>, ControlValue) {
        let row = harness.get_by_role_and_label(Role::Slider, COARSENESS);
        let shown = control(&harness.state().state, "coarseness").unwrap();
        (
            row.accesskit_node().is_disabled(),
            heard(harness, COARSENESS),
            shown.value.clone(),
        )
    }

    #[test]
    fn a_control_another_s_value_disables_is_drawn_disabled_while_pending_and_reported() {
        // Grain at 50 % and coarseness at 150 %, with `size` as given, as the session
        // reports them.
        let at = ControlValue::Number(150.0);
        let reported = |size: u32| {
            let mut state = untouched(&Sample);
            set(
                &mut state,
                "grain",
                ControlValue::Number(50.0),
                Some(Mark::Kept),
            );
            let mark = (size != 1).then_some(Mark::Kept);
            set(&mut state, "size", ControlValue::Choice(size), mark);
            set(&mut state, "coarseness", at.clone(), Some(Mark::Kept));
            report_availability(&Sample, &mut state);
            state
        };
        let choose = |harness: &mut Harness<'static, Panel>, option: &str| {
            let name = format!("tool-sample-control-size-option-{option}");
            harness.get_by_label(&name).click();
            harness.run();
        };
        // Medium grain: disabled.
        let mut harness = panel(&Sample, reported(1));
        harness.run();
        let (disabled, heard, value) = coarseness(&harness);
        assert!(disabled);
        assert_eq!(value, at);
        // Coarse grain chosen: while the change is pending, it is live at once, through
        // the tool's own availability, and so after the session's report.
        choose(&mut harness, "coarse");
        assert_eq!(coarseness(&harness), (false, heard.clone(), at.clone()));
        harness.state_mut().state = reported(2);
        harness.run();
        assert_eq!(coarseness(&harness), (false, heard.clone(), at.clone()));
        // Fine grain chosen: disabled at once, its value kept, and so after the report.
        choose(&mut harness, "fine");
        let size = |choice| Change::Set {
            tool: "sample",
            control: "size",
            value: ControlValue::Choice(choice),
            done: true,
        };
        assert_eq!(harness.state().changes, [size(2), size(0)]);
        assert_eq!(coarseness(&harness), (true, heard.clone(), at.clone()));
        harness.state_mut().state = reported(0);
        harness.run();
        assert_eq!(coarseness(&harness), (true, heard, at));
        // Disabled, it takes no change.
        drag(&mut harness, COARSENESS, 0.75);
        harness
            .get_by_role_and_label(Role::Slider, COARSENESS)
            .focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(harness.state().changes.len(), 2);
    }

    #[test]
    fn a_choice_row_sets_its_control() {
        // Sample's size, segments in a plain group. Sample has no messages, so its
        // words are its message ids.
        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        harness.get_by_label("tool-sample-group-grain");
        harness
            .get_by_label("tool-sample-control-size-option-fine")
            .click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set {
                tool: "sample",
                control: "size",
                value: ControlValue::Choice(0),
                done: true
            }]
        );
    }

    /// Sample's section open on its Shape tab, where its Midtones group is.
    fn shape(state: ToolState) -> Harness<'static, Panel> {
        let mut harness = panel(&Sample, state);
        harness.state_mut().panels.apply(&Change::Tab {
            tool: "sample",
            tab: 1,
        });
        harness.run();
        harness
    }

    /// Every collapsible group keeps the id the app keeps its open state by
    /// (`edit.groups`), so one opened stays open (decision 100): each tool's and utility's,
    /// in its page's order, and none of its plain groups among them.
    #[test]
    fn every_collapsible_group_keeps_its_id() {
        let definitions = apertrust_tools::TOOLS
            .iter()
            .map(|tool| &tool.definition().tool)
            .chain(apertrust_tools::utility::UTILITIES.iter().map(|u| &u.tool));
        let mut kept = Vec::new();
        for tool in definitions {
            let mut rows: Vec<&Row> = Vec::new();
            fn walk<'r>(list: &'r [Row], into: &mut Vec<&'r Row>) {
                for row in list {
                    into.push(row);
                    walk(row.rows(), into);
                }
            }
            for tab in tool.tabs {
                walk(tab.rows, &mut rows);
            }
            walk(tool.before, &mut rows);
            walk(tool.after, &mut rows);
            kept.extend(rows.into_iter().filter_map(|row| match row {
                Row::Group(Group {
                    id,
                    kind: GroupKind::Collapsible,
                    ..
                }) => Some((tool.id, *id)),
                _ => None,
            }));
        }
        kept.sort_unstable();
        assert_eq!(
            kept,
            [
                ("agx", "advanced_curve_parameters"),
                ("agx", "look"),
                ("agx", "show_curve"),
                ("channelmixerrgb", "area_colour_mapping"),
                ("channelmixerrgb", "colour_checker"),
                ("colorequal", "options"),
                ("demosaic", "capture_sharpen_controls"),
                ("export", "global"),
                ("export", "metadata"),
                ("metadata", "basic"),
                ("metadata", "camera"),
                ("metadata", "content"),
                ("metadata", "creator"),
                ("metadata", "exposure"),
                ("metadata", "gps"),
                ("metadata", "lens"),
                ("metadata", "location"),
                ("metadata", "rights"),
                ("metadata", "status"),
                ("temperature", "channel_coefficients"),
            ]
        );
    }

    /// Sample's Midtones group's row. Sample has no messages, so its words are its
    /// message ids.
    const MIDTONES: &str = "tool-sample-collapsible-midtones";

    /// The method's list, and Sample's bias slider while it shows.
    const METHOD: &str = "tool-sample-control-method-full";
    const BIAS: &str = "tool-sample-control-bias-full";

    #[test]
    fn a_collapsible_group_starts_closed_and_opens_and_closes() {
        let mut harness = shape(untouched(&Sample));
        let group = harness.get_by_role_and_label(Role::Button, MIDTONES);
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(false));
        assert!(
            harness
                .query_by_role_and_label(Role::ComboBox, METHOD)
                .is_none()
        );
        group.click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Group {
                tool: "sample",
                group: "midtones"
            }]
        );
        harness.run();
        let group = harness.get_by_role_and_label(Role::Button, MIDTONES);
        assert_eq!(group.accesskit_node().data().is_expanded(), Some(true));
        harness.get_by_role_and_label(Role::ComboBox, METHOD);
        // Enter closes it again from the keyboard.
        group.focus();
        harness.run();
        harness.key_press(Key::Enter);
        harness.run();
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::ComboBox, METHOD)
                .is_none()
        );
        assert_eq!(harness.state().changes.len(), 2);
    }

    #[test]
    fn a_closed_group_says_how_many_of_its_controls_are_set() {
        let mut state = untouched(&Sample);
        set(
            &mut state,
            "method",
            ControlValue::Choice(1),
            Some(Mark::Kept),
        );
        set(
            &mut state,
            "bias",
            ControlValue::Number(40.0),
            Some(Mark::Kept),
        );
        let mut harness = shape(state);
        let group = harness.get_by_role_and_label(Role::Button, MIDTONES);
        assert_eq!(
            group.accesskit_node().description().as_deref(),
            Some("2 set")
        );
        // Its tooltip, while what is set is out of sight, as the wireframe's "Margins: 1 set".
        let tip = format!("{MIDTONES}: 2 set");
        group.hover();
        harness.run();
        assert!(harness.query_by_label(&tip).is_some(), "{tip}");
        // A control in a row its mode hides is not counted: back to even, the bias set
        // is out of the count, and so is the tooltip.
        set(
            &mut harness.state_mut().state,
            "method",
            ControlValue::Choice(0),
            Some(Mark::Kept),
        );
        harness.run();
        let group = harness.get_by_role_and_label(Role::Button, MIDTONES);
        assert_eq!(group.accesskit_node().description(), None);
        assert!(harness.query_by_label(&tip).is_none());
        // Hovering reports nothing.
        assert!(harness.state().changes.is_empty());
    }

    #[test]
    fn a_row_following_a_mode_shows_only_in_that_mode() {
        let mut harness = shape(untouched(&Sample));
        harness
            .get_by_role_and_label(Role::Button, MIDTONES)
            .click();
        harness.run();
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, BIAS)
                .is_none()
        );
        // Biased, as the list chooses it: the bias row shows.
        harness
            .get_by_role_and_label(Role::ComboBox, METHOD)
            .click();
        harness.run();
        harness
            .get_by_label("tool-sample-control-method-option-biased")
            .click();
        harness.run();
        harness.run();
        harness.get_by_role_and_label(Role::Slider, BIAS);
        // Even again: it hides.
        harness
            .get_by_role_and_label(Role::ComboBox, METHOD)
            .click();
        harness.run();
        harness
            .get_by_label("tool-sample-control-method-option-even")
            .click();
        harness.run();
        harness.run();
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, BIAS)
                .is_none()
        );
        let method = |option| Change::Set {
            tool: "sample",
            control: "method",
            value: ControlValue::Choice(option),
            done: true,
        };
        assert_eq!(harness.state().changes[1..], [method(1), method(0)]);
    }

    #[test]
    fn an_option_not_ported_is_listed_but_cannot_be_chosen_and_the_session_refuses_it() {
        let mut harness = shape(untouched(&Sample));
        harness
            .get_by_role_and_label(Role::Button, MIDTONES)
            .click();
        harness.run();
        harness.run();
        harness
            .get_by_role_and_label(Role::ComboBox, METHOD)
            .click();
        harness.run();
        let film = harness.get_by_label("tool-sample-control-method-option-film");
        assert!(film.accesskit_node().is_disabled());
        film.click();
        harness.run();
        assert!(
            !harness
                .state()
                .changes
                .iter()
                .any(|change| matches!(change, Change::Set { .. })),
            "{:?}",
            harness.state().changes
        );
        // Sent anyway, as a script might, the session's edit refuses it.
        let mut edit = apertrust_engine::edit::Edit::default();
        let refused = edit.set_control(
            &Sample,
            "method",
            ControlValue::Choice(2),
            &testing::image(),
        );
        assert!(
            matches!(
                refused,
                Err(apertrust_engine::edit::EditError::Control {
                    source: apertrust_tools::ControlError::Unported { option: "film", .. },
                    ..
                })
            ),
            "{refused:?}"
        );
        assert!(edit.is_empty());
    }

    /// A choice lists the options the session reports the photo lists (decision 30):
    /// Sample's method lists camera only on a photo that records a bias, which the sample
    /// photo does not, so camera is not drawn. An edit from another photo holding it shows
    /// it last, chosen and disabled, the box described as not available for this photo,
    /// until another option is chosen.
    #[test]
    fn a_choice_lists_the_photo_s_options_and_one_from_another_photo_last_and_disabled() {
        const CAMERA: &str = "tool-sample-control-method-option-camera";
        const FILM: &str = "tool-sample-control-method-option-film";
        const EVEN: &str = "tool-sample-control-method-option-even";
        const UNAVAILABLE: &str = "Not available for this photo";
        let open_group = |harness: &mut Harness<'static, Panel>| {
            harness
                .get_by_role_and_label(Role::Button, MIDTONES)
                .click();
            harness.run();
            harness.run();
        };
        let open_list = |harness: &mut Harness<'static, Panel>| {
            harness
                .get_by_role_and_label(Role::ComboBox, METHOD)
                .click();
            harness.run();
        };
        let mut harness = shape(untouched(&Sample));
        open_group(&mut harness);
        open_list(&mut harness);
        harness.get_by_label(EVEN);
        assert!(harness.query_by_label(CAMERA).is_none());
        let method = harness.get_by_role_and_label(Role::ComboBox, METHOD);
        let said = method.accesskit_node().description().unwrap_or_default();
        assert!(!said.contains(UNAVAILABLE), "{said}");

        let mut state = untouched(&Sample);
        set(
            &mut state,
            "method",
            ControlValue::Choice(3),
            Some(Mark::Kept),
        );
        let mut harness = shape(state);
        open_group(&mut harness);
        let method = harness.get_by_role_and_label(Role::ComboBox, METHOD);
        let node = method.accesskit_node();
        assert_eq!(node.value().as_deref(), Some(CAMERA));
        assert_eq!(node.description().as_deref(), Some(UNAVAILABLE));
        assert!(!node.is_disabled());
        open_list(&mut harness);
        let camera = harness.get_by_label(CAMERA);
        assert!(camera.accesskit_node().is_disabled());
        assert!(camera.rect().top() > harness.get_by_label(FILM).rect().top());
        // The group's toggle is the only change so far, and choosing camera sends none.
        let sets = |harness: &Harness<'static, Panel>| -> Vec<Change> {
            let changes = harness.state().changes.iter();
            let set = changes.filter(|change| matches!(change, Change::Set { .. }));
            set.cloned().collect()
        };
        camera.click();
        harness.run();
        assert!(sets(&harness).is_empty());
        // Another option chosen, camera leaves the list.
        if harness.query_by_label(EVEN).is_none() {
            open_list(&mut harness);
        }
        harness.get_by_label(EVEN).click();
        harness.run();
        assert_eq!(
            sets(&harness),
            [Change::Set {
                tool: "sample",
                control: "method",
                value: ControlValue::Choice(0),
                done: true,
            }]
        );
        harness.run();
        open_list(&mut harness);
        assert!(harness.query_by_label(CAMERA).is_none());
        let method = harness.get_by_role_and_label(Role::ComboBox, METHOD);
        let said = method.accesskit_node().description().unwrap_or_default();
        assert!(!said.contains(UNAVAILABLE), "{said}");
    }

    /// A choice whose photo lists one option, holding another from another photo's edit,
    /// is live; once that one option is chosen it is unavailable at once, before the
    /// session reports it so (decision 30).
    #[test]
    fn a_choice_left_with_its_photo_s_one_option_is_disabled_at_once() {
        let mut state = untouched(&Sample);
        set(
            &mut state,
            "method",
            ControlValue::Choice(1),
            Some(Mark::Kept),
        );
        let method = state
            .controls
            .iter_mut()
            .find(|c| c.control == "method")
            .unwrap();
        method.options = vec!["even"];
        let mut harness = shape(state);
        harness
            .get_by_role_and_label(Role::Button, MIDTONES)
            .click();
        harness.run();
        harness.run();
        let disabled = |harness: &Harness<'static, Panel>| {
            let row = harness.get_by_role_and_label(Role::ComboBox, METHOD);
            row.accesskit_node().is_disabled()
        };
        assert!(!disabled(&harness));
        harness
            .get_by_role_and_label(Role::ComboBox, METHOD)
            .click();
        harness.run();
        harness
            .get_by_label("tool-sample-control-method-option-even")
            .click();
        harness.run();
        harness.run();
        assert!(disabled(&harness));
    }

    #[test]
    fn a_row_before_every_tab_s_shows_whichever_tab_is_open() {
        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        let brightness = "tool-sample-control-brightness-full";
        harness.get_by_role_and_label(Role::Slider, brightness);
        harness
            .get_by_role_and_label(Role::Tab, "tool-sample-tab-shape")
            .click();
        harness.run();
        harness.run();
        harness.get_by_role_and_label(Role::Slider, brightness);
        harness.get_by_role_and_label(Role::Button, MIDTONES);
        assert!(
            harness
                .query_by_label("tool-sample-control-size-option-fine")
                .is_none()
        );
    }

    /// Sample's look, a preset row, and its options. Sample has no messages, so its words
    /// are its message ids; Custom is everyone's.
    const LOOK: &str = "tool-sample-preset-look-full";
    const UNTOUCHED: &str = "tool-sample-preset-look-option-untouched";
    const GRAINY: &str = "tool-sample-preset-look-option-grainy";
    const CAMERA: &str = "tool-sample-preset-look-option-camera";

    /// What the look's box shows, as screen readers hear it.
    fn look(harness: &Harness<'_, Panel>) -> Option<String> {
        let row = harness.get_by_role_and_label(Role::ComboBox, LOOK);
        row.accesskit_node().value().as_deref().map(str::to_owned)
    }

    /// Sample's controls holding its look's grainy option, kept.
    fn grainy() -> ToolState {
        let mut state = untouched(&Sample);
        set(
            &mut state,
            "grain",
            ControlValue::Number(50.0),
            Some(Mark::Kept),
        );
        set(
            &mut state,
            "size",
            ControlValue::Choice(2),
            Some(Mark::Kept),
        );
        state
    }

    #[test]
    fn a_preset_row_shows_the_option_its_controls_hold() {
        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        assert_eq!(look(&harness).as_deref(), Some(UNTOUCHED));
        // It stores nothing, so it has no marker.
        assert!(harness.query_by_label(&format!("Keep {LOOK}")).is_none());
        harness.state_mut().state = grainy();
        harness.run();
        assert_eq!(look(&harness).as_deref(), Some(GRAINY));
        assert!(harness.state().changes.is_empty());
    }

    #[test]
    fn a_preset_row_shows_custom_once_a_control_moves_off_its_option() {
        let mut harness = panel(&Sample, grainy());
        harness.run();
        assert_eq!(look(&harness).as_deref(), Some(GRAINY));
        let brightness = "tool-sample-control-brightness-full";
        harness
            .get_by_role_and_label(Role::Slider, brightness)
            .focus();
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Set {
                tool: "sample",
                control: "brightness",
                value: ControlValue::Number(0.01),
                done: true
            }]
        );
        assert_eq!(look(&harness).as_deref(), Some("Custom"));
        // Custom is what the box shows, never an option to choose.
        harness.get_by_role_and_label(Role::ComboBox, LOOK).click();
        harness.run();
        harness.get_by_label(GRAINY);
        assert!(harness.query_by_label("Custom").is_none());
    }

    #[test]
    fn choosing_an_option_of_a_preset_row_applies_it() {
        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        harness.get_by_role_and_label(Role::ComboBox, LOOK).click();
        harness.run();
        harness.get_by_label(GRAINY).click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::ApplyPreset {
                tool: "sample",
                preset: "look",
                option: "grainy"
            }]
        );
        // Once the session reports the controls it wrote, the box shows it.
        harness.run();
        assert_eq!(look(&harness).as_deref(), Some(GRAINY));
    }

    #[test]
    fn an_option_the_photo_cannot_have_is_drawn_disabled_and_cannot_be_chosen() {
        // The sample photo records no exposure bias, which the look's camera option sets.
        let mut harness = panel(&Sample, grainy());
        harness.run();
        harness.get_by_role_and_label(Role::ComboBox, LOOK).click();
        harness.run();
        let camera = harness.get_by_label(CAMERA);
        assert!(camera.accesskit_node().is_disabled());
        camera.click();
        harness.run();
        assert!(harness.state().changes.is_empty());
        assert_eq!(look(&harness).as_deref(), Some(GRAINY));
        // The list stays open past the click, and the others are chosen as ever.
        harness.get_by_label(UNTOUCHED).click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::ApplyPreset {
                tool: "sample",
                preset: "look",
                option: "untouched"
            }]
        );
    }

    /// White balance's Settings and its From image area, by their words.
    const WHITE_BALANCE_SETTINGS: &str = "White balance settings";
    const FROM_IMAGE_AREA: &str = "From image area";
    const SPOT: PickerRef = PickerRef::Tool {
        tool: "temperature",
        picker: "spot",
    };

    /// What choosing From image area in White balance's Settings sends, with the picker
    /// `on` and the pickers pressable unless `why` says why not.
    fn from_image_area(on: Option<PickerRef>, why: Option<&'static str>) -> Vec<Change> {
        let mut harness = panel(&WhiteBalance, untouched(&WhiteBalance));
        harness.state_mut().on = on;
        harness.state_mut().why = why;
        harness.run();
        harness
            .get_by_role_and_label(Role::ComboBox, WHITE_BALANCE_SETTINGS)
            .click();
        harness.run();
        let option = harness.get_by_label(FROM_IMAGE_AREA);
        assert_eq!(option.accesskit_node().is_disabled(), why.is_some());
        option.click();
        harness.run();
        harness.state().changes.clone()
    }

    /// White balance's User modified is offered and sent as any option is: the session
    /// writes back the coefficients it remembers (decision 27).
    #[test]
    fn user_modified_is_offered_and_applied() {
        // The photo's defaults, as shot to reference, so User modified is another option.
        let mut state = untouched(&WhiteBalance);
        for (control, value) in WhiteBalance.defaults(&testing::image()).iter() {
            set(&mut state, control, value.clone(), None);
        }
        let mut harness = panel(&WhiteBalance, state);
        harness.run();
        harness
            .get_by_role_and_label(Role::ComboBox, WHITE_BALANCE_SETTINGS)
            .click();
        harness.run();
        let option = harness.get_by_label("User modified");
        assert!(!option.accesskit_node().is_disabled());
        option.click();
        harness.run();
        assert!(
            harness.state().changes.contains(&Change::ApplyPreset {
                tool: "temperature",
                preset: "settings",
                option: "user",
            }),
            "{:?}",
            harness.state().changes
        );
    }

    /// Settings' From image area turns White balance's picker on as an area, as darktable's
    /// `_preset_tune_callback` presses its button (`temperature.c:1816-1822`), never
    /// applying the option; again while the picker is on, nothing, as darktable presses it
    /// only when it is not active; and it is disabled where the pickers cannot be pressed,
    /// as on a photo open for viewing only.
    #[test]
    fn from_image_area_turns_white_balance_s_picker_on() {
        let changes = from_image_area(None, None);
        assert!(
            matches!(
                changes[..],
                [Change::Picker {
                    tool: "temperature",
                    picker: "spot",
                    action: PickerAction::On { area: true },
                    ..
                }]
            ),
            "{changes:?}"
        );
        assert_eq!(from_image_area(Some(SPOT), None), []);
        assert_eq!(from_image_area(None, Some("Open for viewing only")), []);
    }

    /// Sample's pair's rows and grain's and bias's own, by their full names. Sample has no
    /// messages, so its words are its message ids.
    const SUM: &str = "tool-sample-control-sum-full";
    const DIFFERENCE: &str = "tool-sample-control-difference-full";
    const GRAIN: &str = "tool-sample-control-grain-full";

    /// What the slider `name` shows, as screen readers hear it.
    fn heard(harness: &Harness<'_, Panel>, name: &str) -> Option<String> {
        let row = harness.get_by_role_and_label(Role::Slider, name);
        row.accesskit_node().value().as_deref().map(str::to_owned)
    }

    /// Grain and bias for the pair's `coordinates`: the tool's inverse of its view.
    fn unviewed(coordinates: [f32; 2]) -> [f32; 2] {
        let values = Sample
            .unview("pair", &coordinates, &testing::image())
            .unwrap();
        ["grain", "bias"].map(|id| values.number(id))
    }

    /// The pair's coordinates for the values `state` shows: the tool's view of them.
    fn viewed(state: &ToolState) -> Vec<f32> {
        let mut values = Controls::new();
        for shown in &state.controls {
            values.set(shown.control, shown.value.clone());
        }
        Sample.view("pair", &values, &testing::image()).unwrap()
    }

    /// The pair's coordinates as `state` shows them, with their mark.
    fn pair(state: &ToolState) -> [(f32, Option<Mark>); 2] {
        ["sum", "difference"].map(|id| {
            let shown = state.views.iter().find(|c| c.coordinate == id).unwrap();
            (shown.value, shown.mark)
        })
    }

    /// Grain and bias as `state` shows them, with their marks.
    fn grain_and_bias(state: &ToolState) -> [(ControlValue, Option<Mark>); 2] {
        ["grain", "bias"].map(|id| {
            let shown = control(state, id).unwrap();
            (shown.value.clone(), shown.mark)
        })
    }

    /// Sample's values as the session reports them holding the pair at `coordinates`, as
    /// after a script set them or through a gesture: grain and bias what the tool's
    /// inverse gives them, kept, the coordinates exactly, and the method biased, so bias's
    /// own row shows.
    fn paired(coordinates: [f32; 2]) -> ToolState {
        let mut state = untouched(&Sample);
        for (id, value) in ["grain", "bias"].into_iter().zip(unviewed(coordinates)) {
            set(
                &mut state,
                id,
                ControlValue::Number(value),
                Some(Mark::Kept),
            );
        }
        let biased = ControlValue::Choice(1);
        set(&mut state, "method", biased, Some(Mark::Kept));
        for (shown, value) in state.views.iter_mut().zip(coordinates) {
            shown.value = value;
            shown.mark = Some(Mark::Kept);
        }
        state
    }

    /// Sample's section on its Shape tab with the Midtones group open: the pair's rows,
    /// and bias's while the method is biased.
    fn midtones(state: ToolState) -> Harness<'static, Panel> {
        let mut harness = shape(state);
        harness.state_mut().groups.insert("midtones", true);
        harness.run();
        harness
    }

    /// Drags the slider `name` to `at` of its track's width and lets go.
    fn drag(harness: &mut Harness<'_, Panel>, name: &str, at: f32) {
        let track = harness.get_by_role_and_label(Role::Slider, name).rect();
        let at = along(track, at);
        harness.drag_at(at);
        harness.run();
        harness.drop_at(at);
        harness.run();
    }

    #[test]
    fn a_view_s_rows_show_the_coordinates_the_session_reports_in_their_words() {
        // Untouched, grain 0 and bias 100: their sum 100 %, their difference −100 %. Screen
        // readers hear each by its full name with its unit, and its promise is its tooltip.
        let mut harness = midtones(untouched(&Sample));
        assert_eq!(heard(&harness, SUM).as_deref(), Some("100%"));
        assert_eq!(heard(&harness, DIFFERENCE).as_deref(), Some("−100%"));
        let (keep_sum, keep_difference) = (format!("Keep {SUM}"), format!("Keep {DIFFERENCE}"));
        let keep = harness.get_by_label(&keep_sum);
        assert_eq!(keep.accesskit_node().toggled(), Some(Toggled::False));
        harness.get_by_role_and_label(Role::Slider, SUM).hover();
        // A second's frames, past egui's tooltip delay.
        for _ in 0..60 {
            harness.step();
        }
        assert!(
            harness
                .query_by_label("tool-sample-control-sum-promise")
                .is_some()
        );
        // Held where the session holds them, kept as their controls are.
        harness.state_mut().state = paired([110.0, -40.3]);
        harness.run();
        assert_eq!(heard(&harness, SUM).as_deref(), Some("110%"));
        assert_eq!(heard(&harness, DIFFERENCE).as_deref(), Some("−40%"));
        let keep = harness.get_by_label(&keep_difference);
        assert_eq!(keep.accesskit_node().toggled(), Some(Toggled::True));
        assert!(harness.state().changes.is_empty());
    }

    #[test]
    fn dragging_a_coordinate_sets_it_and_shows_the_others_exactly_as_held() {
        let mut harness = midtones(paired([100.0, -40.3]));
        // Three quarters along the sum's track, which spans its soft range, 50 to 150 %.
        drag(&mut harness, SUM, 0.75);
        let set = |done| Change::Set {
            tool: "sample",
            control: "sum",
            value: ControlValue::Number(125.0),
            done,
        };
        assert_eq!(harness.state().changes, [set(false), set(true)]);
        // While it is pending, the difference stays exactly where the session holds it,
        // and grain and bias are what the tool's inverse gives the two, kept ...
        let state = &harness.state().state;
        let kept = Some(Mark::Kept);
        assert_eq!(pair(state), [(125.0, kept), (-40.3, kept)]);
        let [grain, bias] = unviewed([125.0, -40.3]);
        let number = ControlValue::Number;
        assert_eq!(
            grain_and_bias(state),
            [(number(grain), kept), (number(bias), kept)]
        );
        // ... where the tool's view of them gives the difference back only to a rounding.
        assert_ne!(viewed(state)[1], -40.3);
        // Their own rows follow at once: bias's here, grain's on its tab.
        assert_eq!(
            heard(&harness, "tool-sample-control-bias-full").as_deref(),
            Some("83%")
        );
        harness
            .get_by_role_and_label(Role::Tab, "tool-sample-tab-grain")
            .click();
        harness.run();
        harness.run();
        assert_eq!(heard(&harness, GRAIN).as_deref(), Some("42%"));
    }

    #[test]
    fn a_key_steps_a_coordinate_by_its_step_holding_the_others() {
        let mut harness = midtones(paired([100.0, -40.3]));
        harness.get_by_role_and_label(Role::Slider, SUM).focus();
        harness.run();
        // The coordinate's step, 1 %, and ten of them with Shift.
        harness.key_press(Key::ArrowRight);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
        harness.run();
        let set = |value| Change::Set {
            tool: "sample",
            control: "sum",
            value: ControlValue::Number(value),
            done: true,
        };
        assert_eq!(harness.state().changes, [set(101.0), set(111.0)]);
        let kept = Some(Mark::Kept);
        assert_eq!(pair(&harness.state().state), [(111.0, kept), (-40.3, kept)]);
    }

    #[test]
    fn a_view_s_rows_are_disabled_on_a_photo_open_for_viewing_only() {
        // The session reports no control available on such a photo, so none the pair's
        // rows move can change it.
        let mut state = paired([100.0, -40.3]);
        for control in &mut state.controls {
            control.available = false;
        }
        let mut harness = midtones(state);
        for name in [SUM, DIFFERENCE] {
            let row = harness.get_by_role_and_label(Role::Slider, name);
            assert!(row.accesskit_node().is_disabled(), "{name}");
        }
        drag(&mut harness, SUM, 0.75);
        harness.get_by_role_and_label(Role::Slider, SUM).focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert!(
            harness.state().changes.is_empty(),
            "{:?}",
            harness.state().changes
        );
        // One control that can is enough, since moving a coordinate moves them all.
        harness.state_mut().state.controls[1].available = true;
        harness.run();
        let row = harness.get_by_role_and_label(Role::Slider, SUM);
        assert!(!row.accesskit_node().is_disabled());
    }

    /// Twin's coordinates as `state` shows them, by view: its pair's, then its mix's.
    fn twin_views(state: &ToolState) -> [[f32; 2]; 2] {
        let of = |view: &str| {
            let shown: Vec<f32> = state
                .views
                .iter()
                .filter(|c| c.view == view)
                .map(|c| c.value)
                .collect();
            [shown[0], shown[1]]
        };
        [of("pair"), of("mix")]
    }

    /// Twin's view `view` of the values `state` shows: the tool's own.
    fn twin_viewed(state: &ToolState, view: &str) -> [f32; 2] {
        let mut values = Controls::new();
        for shown in &state.controls {
            values.set(shown.control, shown.value.clone());
        }
        let viewed = Twin.view(view, &values, &testing::image()).unwrap();
        [viewed[0], viewed[1]]
    }

    #[test]
    fn a_coordinate_moves_another_view_of_the_same_controls_at_once() {
        // Twin's mix, u = x + 2y, stepped twice by a key: x and y are its inverse's, and
        // its own coordinates stay exactly as stepped, while the pair, a second view of x
        // and y as hue and chroma are of White balance's light, shows its view of them.
        let mut harness = panel(&Twin, untouched(&Twin));
        harness.run();
        assert_eq!(
            twin_views(&harness.state().state),
            [[80.0, -40.0], [140.0, -20.0]]
        );
        harness
            .get_by_role_and_label(Role::Slider, "tool-twin-control-u-full")
            .focus();
        harness.run();
        for _ in 0..2 {
            harness.key_press(Key::ArrowRight);
            harness.run();
        }
        let state = &harness.state().state;
        let [pair, mix] = twin_views(state);
        assert_eq!(mix, [142.0, -20.0]);
        assert_eq!(pair, twin_viewed(state, "pair"));
        assert_ne!(pair, [80.0, -40.0]);
        let kept = Some(Mark::Kept);
        assert!(
            state.views.iter().all(|c| c.mark == kept),
            "{:?}",
            state.views
        );
    }

    /// Twin's definition with an `unview` that panics, as a tool's code may, and a `view`
    /// and an `available` that panic too or are Twin's; it counts the panics.
    struct Panicking {
        view_panics: bool,
        available_panics: bool,
        panics: std::sync::atomic::AtomicUsize,
    }

    impl Panicking {
        fn new(view_panics: bool) -> Self {
            Self {
                view_panics,
                available_panics: false,
                panics: Default::default(),
            }
        }

        fn panics(&self) -> usize {
            self.panics.load(std::sync::atomic::Ordering::Relaxed)
        }

        fn panic(&self, what: &str) -> ! {
            self.panics
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            panic!("{what} that panics")
        }
    }

    impl ProcessingTool for Panicking {
        fn definition(&self) -> &'static ProcessingDefinition {
            Twin.definition()
        }

        fn owns(&self) -> &'static [apertrust_tools::Owned] {
            &[]
        }

        fn bind(
            &self,
            _: &apertrust_tools::ToolValues,
            _: &apertrust_plugin::ImageContext,
            _: &apertrust_tools::Base,
        ) -> Vec<apertrust_tools::Setting> {
            Vec::new()
        }

        fn view(&self, view: &str, values: &Controls, image: &ImageContext) -> Option<Vec<f32>> {
            if self.view_panics {
                self.panic("a view")
            }
            Twin.view(view, values, image)
        }

        fn unview(&self, _: &str, _: &[f32], _: &ImageContext) -> Option<Controls> {
            self.panic("an unview")
        }

        fn available(&self, control: &str, values: &Controls, image: &ImageContext) -> bool {
            if self.available_panics {
                self.panic("an available")
            }
            Twin.available(control, values, image)
        }
    }

    /// Twin's `x` set to 30 and its pair's sum to 110, as the app sends them.
    fn twin_changes() -> [Pending; 2] {
        [("x", 30.0), ("sum", 110.0)].map(|(control, value)| Pending::Control {
            tool: "twin",
            control,
            value: ControlValue::Number(value),
        })
    }

    #[test]
    fn a_view_the_report_leaves_out_is_never_asked_for() {
        // The session leaves out a view whose tool panics: a change to its controls, or
        // to one of its coordinates, shows without calling it.
        let mut state = untouched(&Twin);
        state.views.clear();
        let (tool, image) = (Panicking::new(true), testing::image());
        for change in twin_changes() {
            change.show(&tool, &mut state, &image);
        }
        Pending::Reset {
            tool: "twin",
            control: "x",
        }
        .show(&tool, &mut state, &image);
        assert_eq!(tool.panics(), 0);
        assert_eq!(
            control(&state, "y").unwrap().value,
            ControlValue::Number(60.0)
        );
        assert!(state.views.is_empty());
    }

    #[test]
    fn a_view_or_unview_that_panics_on_pending_values_shows_nothing() {
        // Reported, as a view that works on the session's values is: a coordinate set
        // through an `unview` that panics shows nothing, as the session refuses it ...
        let image = testing::image();
        let untouched = untouched(&Twin);
        let tool = Panicking::new(false);
        let mut state = untouched.clone();
        let [_, sum] = twin_changes();
        sum.show(&tool, &mut state, &image);
        assert_eq!(tool.panics(), 1);
        assert_eq!(state, untouched);
        // ... and a control set through its own row shows, its views as reported where
        // their `view` panics.
        let tool = Panicking::new(true);
        let [x, _] = twin_changes();
        x.show(&tool, &mut state, &image);
        assert_eq!(tool.panics(), 2, "one for each view");
        assert_eq!(
            control(&state, "x").unwrap().value,
            ControlValue::Number(30.0)
        );
        assert_eq!(state.views, untouched.views);
    }

    #[test]
    fn an_availability_that_panics_on_pending_values_leaves_it_as_reported() {
        let image = testing::image();
        let mut state = untouched(&Twin);
        control_mut(&mut state, "y").unwrap().available = false;
        let tool = Panicking {
            available_panics: true,
            ..Panicking::new(false)
        };
        let [x, _] = twin_changes();
        x.show(&tool, &mut state, &image);
        assert_eq!(tool.panics(), 1);
        // The value shows, and each control's availability as the session reported it.
        let shown = |id| control(&state, id).unwrap();
        assert_eq!(shown("x").value, ControlValue::Number(30.0));
        assert!(shown("x").available);
        assert!(!shown("y").available);
    }

    /// Twin with x and y at their defaults, kept, as a write through a view can leave them,
    /// and its views' coordinates `pair` and `mix`, kept: held where they are off the
    /// tool's view of x and y.
    fn twin_held(pair: [f32; 2], mix: [f32; 2]) -> ToolState {
        let mut state = untouched(&Twin);
        for (id, value) in [("x", 20.0), ("y", 60.0)] {
            set(
                &mut state,
                id,
                ControlValue::Number(value),
                Some(Mark::Kept),
            );
        }
        let shown = pair.into_iter().chain(mix);
        for (coordinate, value) in state.views.iter_mut().zip(shown) {
            coordinate.value = value;
            coordinate.mark = Some(Mark::Kept);
        }
        state
    }

    #[test]
    fn a_reset_ends_every_hold_over_a_control_whose_value_shows_the_same() {
        // The mix held a rounding off its view of x and y at their defaults.
        let image = testing::image();
        let off = [140f32.next_up(), -20.0];
        // x reset: its value shows the same, but out of the edit it holds nothing, so the
        // mix is its view of x and y again, as the session reports.
        let mut state = twin_held([80.0, -40.0], off);
        Pending::Reset {
            tool: "twin",
            control: "x",
        }
        .show(&Twin, &mut state, &image);
        assert_eq!(twin_views(&state), [[80.0, -40.0], [140.0, -20.0]]);
        // The pair's difference reset, a rounding off its default: both of its
        // coordinates are back, so x and y are out of the edit, their values the same, and
        // the mix is its view of them again too.
        let mut state = twin_held([80.0, (-40f32).next_up()], off);
        Pending::Reset {
            tool: "twin",
            control: "difference",
        }
        .show(&Twin, &mut state, &image);
        assert_eq!(twin_views(&state), [[80.0, -40.0], [140.0, -20.0]]);
        assert!(state.controls.iter().all(|c| c.mark.is_none()));
    }

    #[test]
    fn the_header_s_reset_takes_every_coordinate_back_to_its_default() {
        let mut harness = midtones(paired([150.0, -40.3]));
        harness.get_by_label("Reset tool-sample-title").click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::ResetTool { tool: "sample" }]
        );
        let state = &harness.state().state;
        assert_eq!(pair(state), [(100.0, None), (-100.0, None)]);
        let number = ControlValue::Number;
        assert_eq!(
            grain_and_bias(state),
            [(number(0.0), None), (number(100.0), None)]
        );
    }

    #[test]
    fn the_marker_of_a_kept_coordinate_releases_its_view_s_controls() {
        let mut harness = midtones(paired([150.0, -40.3]));
        harness.get_by_label(&format!("Keep {DIFFERENCE}")).click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Keep {
                tool: "sample",
                control: "difference",
                kept: false
            }]
        );
        // Released, its values as they are: the controls' marks, and so the coordinates'.
        let state = &harness.state().state;
        assert_eq!(pair(state), [(150.0, None), (-40.3, None)]);
        let [grain, bias] = unviewed([150.0, -40.3]);
        let number = ControlValue::Number;
        assert_eq!(
            grain_and_bias(state),
            [(number(grain), None), (number(bias), None)]
        );
    }

    #[test]
    fn a_control_s_own_row_moves_the_coordinates_of_its_view() {
        // Grain's row, on its tab, while the session holds the pair: the coordinates are
        // the tool's view of the values again, as the session reports once grain moves.
        let mut harness = panel(&Sample, paired([100.0, -40.3]));
        harness.run();
        drag(&mut harness, GRAIN, 0.75);
        let state = &harness.state().state;
        assert_eq!(
            control(state, "grain").unwrap().value,
            ControlValue::Number(75.0)
        );
        let shown = pair(state).map(|(value, _)| value);
        assert_eq!(shown[..], viewed(state)[..]);
        assert_ne!(shown[1], -40.3);
        // The pair's rows show them.
        harness.state_mut().panels.apply(&Change::Tab {
            tool: "sample",
            tab: 1,
        });
        harness.state_mut().groups.insert("midtones", true);
        harness.run();
        assert_eq!(heard(&harness, SUM).as_deref(), Some("145%"));
        assert_eq!(heard(&harness, DIFFERENCE).as_deref(), Some("+5%"));
    }

    #[test]
    fn a_coordinate_resets_and_keeps_as_a_control() {
        let mut harness = midtones(paired([150.0, -40.3]));
        // Delete takes the sum back to its default, holding the difference ...
        harness.get_by_role_and_label(Role::Slider, SUM).focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        let kept = Some(Mark::Kept);
        assert_eq!(pair(&harness.state().state), [(100.0, kept), (-40.3, kept)]);
        // ... and the difference's too leaves grain and bias untouched, released.
        harness
            .get_by_role_and_label(Role::Slider, DIFFERENCE)
            .focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        let state = &harness.state().state;
        assert_eq!(pair(state), [(100.0, None), (-100.0, None)]);
        let number = ControlValue::Number;
        assert_eq!(
            grain_and_bias(state),
            [(number(0.0), None), (number(100.0), None)]
        );
        // The marker keeps the view's controls, which the coordinates' marks show.
        harness.get_by_label(&format!("Keep {SUM}")).click();
        harness.run();
        let state = &harness.state().state;
        assert_eq!(pair(state), [(100.0, kept), (-100.0, kept)]);
        assert_eq!(
            grain_and_bias(state),
            [(number(0.0), kept), (number(100.0), kept)]
        );
        let tool = "sample";
        assert_eq!(
            harness.state().changes,
            [
                Change::Reset {
                    tool,
                    control: "sum"
                },
                Change::Reset {
                    tool,
                    control: "difference"
                },
                Change::Keep {
                    tool,
                    control: "sum",
                    kept: true
                },
            ]
        );
    }

    /// A page as local contrast draws one, for the paths no built tool takes yet:
    /// `sigma_r` under its own label with the bilateral grid and under Highlights with the
    /// local Laplacian filter, and the mask display after every tab's rows. It binds
    /// nothing; its words are its message ids.
    struct LocalContrast;

    static LOCAL_CONTRAST: ProcessingDefinition = ProcessingDefinition {
        tool: ToolDefinition {
            id: "bilat",
            tabs: &[
                apertrust_tools::Tab {
                    id: Some("main"),
                    rows: &[
                        Row::Choice {
                            control: "mode",
                            style: ChoiceStyle::List,
                            end: &[],
                        },
                        Row::When {
                            control: "mode",
                            is: &["bilateral"],
                            rows: &[Row::Slider(Slider::new("sigma_r"))],
                        },
                        Row::When {
                            control: "mode",
                            is: &["laplacian"],
                            rows: &[Row::Slider(Slider::new("sigma_r").label("highlights"))],
                        },
                    ],
                },
                apertrust_tools::Tab {
                    id: Some("detail"),
                    rows: &[Row::Slider(Slider::new("detail"))],
                },
            ],
            before: &[],
            after: &[Row::Toggle {
                control: "mask",
                end: &[],
            }],
            reset: Reset::Tool,
            presets: true,
            open_at_first: false,
            menus: &[],
            toolbar: &[],
            cursors: &[],
        },
        version: 1,
        layers: true,
        auto: false,
        switched: false,
        copy_safe: true,
        in_styles: false,
        publishes: &[],
        controls: &[
            apertrust_tools::Control {
                id: "mode",
                kind: ControlKind::Choice {
                    options: &["bilateral", "laplacian"],
                    default: 0,
                    unported: &[],
                },
                unit: None,
                unported: false,
            },
            apertrust_tools::Control {
                id: "sigma_r",
                kind: ControlKind::Number {
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                    default: 0.5,
                },
                unit: None,
                unported: false,
            },
            apertrust_tools::Control {
                id: "detail",
                kind: ControlKind::Number {
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                    default: 0.0,
                },
                unit: None,
                unported: false,
            },
            apertrust_tools::Control {
                id: "mask",
                kind: ControlKind::Bool { default: false },
                unit: None,
                unported: false,
            },
        ],
        preset_rows: &[],
        views: &[],
        pickers: &[],
        facts: None,
    };

    impl ProcessingTool for LocalContrast {
        fn definition(&self) -> &'static ProcessingDefinition {
            &LOCAL_CONTRAST
        }

        fn owns(&self) -> &'static [apertrust_tools::Owned] {
            &[]
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

    #[test]
    fn a_control_under_another_label_in_each_mode_shows_each_mode_s_words() {
        LOCAL_CONTRAST.check().unwrap();
        let mut harness = panel(&LocalContrast, untouched(&LocalContrast));
        harness.run();
        let (own, highlights) = (
            "tool-bilat-control-sigma_r-full",
            "tool-bilat-control-sigma_r-as-highlights-full",
        );
        harness.get_by_role_and_label(Role::Slider, own);
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, highlights)
                .is_none()
        );
        // The local Laplacian filter: the same control under its other label, one row at a
        // time, and a drag still sets the control.
        set(
            &mut harness.state_mut().state,
            "mode",
            ControlValue::Choice(1),
            Some(Mark::Kept),
        );
        harness.run();
        assert!(harness.query_by_role_and_label(Role::Slider, own).is_none());
        let track = harness
            .get_by_role_and_label(Role::Slider, highlights)
            .rect();
        let at = along(track, 0.75);
        harness.drag_at(at);
        harness.run();
        harness.drop_at(at);
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Set {
                tool: "bilat",
                control: "sigma_r",
                value: ControlValue::Number(0.75),
                done: true,
            })
        );
    }

    #[test]
    fn a_row_after_every_tab_s_shows_whichever_tab_is_open() {
        let mut harness = panel(&LocalContrast, untouched(&LocalContrast));
        harness.run();
        let mask = "tool-bilat-control-mask-full";
        harness.get_by_role_and_label(Role::Switch, mask);
        harness
            .get_by_role_and_label(Role::Tab, "tool-bilat-tab-detail")
            .click();
        harness.run();
        harness.run();
        harness.get_by_role_and_label(Role::Slider, "tool-bilat-control-detail-full");
        // After the tab's rows, as the tone equalizer's mask display.
        let detail = harness
            .get_by_role_and_label(Role::Slider, "tool-bilat-control-detail-full")
            .rect();
        let below = harness.get_by_role_and_label(Role::Switch, mask).rect();
        assert!(below.top() >= detail.bottom(), "{below:?} after {detail:?}");
    }

    /// The Luma curve's plot, less the inset that keeps its points inside it, and where a
    /// value pair lies on it.
    fn plot(harness: &Harness<'_, Panel>) -> egui::Rect {
        harness
            .get_by_role_and_label(Role::ListBox, "Tone curve, Luma")
            .rect()
            .shrink(Theme::default().spacing.s)
    }

    fn on(plot: egui::Rect, x: f32, y: f32) -> egui::Pos2 {
        pos2(
            plot.left() + x * plot.width(),
            plot.bottom() - y * plot.height(),
        )
    }

    /// The points a change sets on the Luma curve, and whether it ends its gesture.
    fn luma_set(change: &Change) -> (&[[f32; 2]], bool) {
        match change {
            Change::Set {
                tool: "tone_curve",
                control: "luma_points",
                value: ControlValue::Points(points),
                done,
            } => (points, *done),
            other => panic!("not the Luma curve's points: {other:?}"),
        }
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
    }

    #[test]
    fn pressing_on_the_curve_adds_a_point_and_dragging_moves_it() {
        let mut harness = panel(&ToneCurve, untouched(&ToneCurve));
        harness.run();
        let plot = plot(&harness);
        // A press away from the diagonal adds a point there, kept between the ends.
        let (from, to) = (on(plot, 0.25, 0.45), on(plot, 0.3, 0.6));
        harness.hover_at(from);
        harness.run();
        harness.drag_at(from);
        harness.run();
        harness.hover_at(to);
        harness.run();
        harness.drop_at(to);
        harness.run();
        let changes = &harness.state().changes;
        // The control holds the points between the ends, which it does not store.
        let (added, done) = luma_set(&changes[0]);
        assert!(done, "adding is a step of its own");
        assert_eq!(added.len(), 1);
        assert!(near(added[0], [0.25, 0.45]), "{added:?}");
        // Dragging it on, the same press, moves the new point; its release ends the step.
        let (moved, done) = luma_set(changes.last().unwrap());
        assert!(done);
        assert!(near(moved[0], [0.3, 0.6]), "{moved:?}");
        assert!(changes[1..changes.len() - 1].iter().all(|c| !luma_set(c).1));
        // Screen readers hear the points, the ends among them.
        harness.get_by(|node| {
            node.role() == Role::ListBoxOption
                && node
                    .label()
                    .is_some_and(|label| label.starts_with("Point 3:"))
        });
    }

    #[test]
    fn the_curve_s_ends_stay_where_they_are() {
        let mut harness = panel(&ToneCurve, untouched(&ToneCurve));
        harness.run();
        let plot = plot(&harness);
        // Dragged, with the pointer, and moved, with the keys: nothing is sent. While the
        // button is down the curve keeps asking to move the end, so the frames are stepped.
        for (from, to) in [((0.0, 0.0), (0.2, 0.3)), ((1.0, 1.0), (0.8, 0.6))] {
            let (from, to) = (on(plot, from.0, from.1), on(plot, to.0, to.1));
            harness.hover_at(from);
            harness.run();
            harness.drag_at(from);
            harness.step();
            harness.hover_at(to);
            harness.step();
            harness.drop_at(to);
            harness.run();
        }
        harness
            .get_by_role_and_label(Role::ListBox, "Tone curve, Luma")
            .focus();
        harness.run();
        for key in [Key::Home, Key::End] {
            harness.key_press(key);
            harness.run();
            harness.key_press(Key::ArrowLeft);
            harness.run();
            harness.key_press(Key::ArrowDown);
            harness.run();
            harness.key_press(Key::Delete);
            harness.run();
        }
        assert!(
            harness.state().changes.is_empty(),
            "{:?}",
            harness.state().changes
        );
    }

    #[test]
    fn a_point_on_the_straight_curve_keeps_it_straight() {
        let mut harness = panel(&ToneCurve, untouched(&ToneCurve));
        harness.run();
        harness
            .get_by_role_and_label(Role::ListBox, "Tone curve, Luma")
            .focus();
        harness.run();
        // Insert adds a point midway, on the curve drawn: the diagonal itself.
        harness.key_press(Key::Insert);
        harness.run();
        let (added, _) = luma_set(&harness.state().changes[0]);
        assert_eq!(added.len(), 1);
        assert!(tone_curve::straight(added), "{added:?}");
    }

    #[test]
    fn resetting_the_curve_straightens_it() {
        let mut state = untouched(&ToneCurve);
        let bent = ControlValue::Points(vec![[0.25, 0.35]]);
        set(&mut state, "luma_points", bent.clone(), Some(Mark::Kept));
        let mut harness = panel(&ToneCurve, state);
        harness.run();
        harness
            .get_by_role_and_label(Role::ListBox, "Tone curve, Luma")
            .focus();
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::Delete);
        harness.run();
        let reset = Change::Reset {
            tool: "tone_curve",
            control: "luma_points",
        };
        assert_eq!(harness.state().changes, std::slice::from_ref(&reset));
        assert_eq!(
            control(&harness.state().state, "luma_points")
                .unwrap()
                .value,
            ControlValue::Points(Vec::new())
        );
        // The header's reset resets the tool, the curve with it.
        set(
            &mut harness.state_mut().state,
            "luma_points",
            bent,
            Some(Mark::Kept),
        );
        harness.state_mut().changes.clear();
        harness.run();
        harness.get_by_label("Reset Tone curve").click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::ResetTool { tool: "tone_curve" }]
        );
    }

    #[test]
    fn the_tabs_to_come_are_disabled_and_advanced_holds_preserve_colours() {
        let mut harness = panel(&ToneCurve, untouched(&ToneCurve));
        harness.run();
        for name in ["RGB", "Red", "Green", "Blue"] {
            let tab = harness.get_by_role_and_label(Role::Tab, name);
            assert!(tab.accesskit_node().is_disabled(), "{name}");
            tab.click();
            harness.run();
        }
        assert!(harness.state().changes.is_empty());
        harness.get_by_role_and_label(Role::Tab, "Advanced").click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Tab {
                tool: "tone_curve",
                tab: 5
            }]
        );
        harness.run();
        let preserve =
            harness.get_by_role_and_label(Role::ComboBox, "Preserve colours in the Luma curve");
        assert_eq!(
            preserve.accesskit_node().value().as_deref(),
            Some("Luminance")
        );
        assert!(
            harness
                .query_by_role_and_label(Role::ListBox, "Tone curve, Luma")
                .is_none()
        );
    }

    #[test]
    fn colour_balance_rgb_draws_darktable_s_three_tabs() {
        let tool: &'static dyn ProcessingTool = &ColourBalanceRgb;
        let mut harness = panel(tool, untouched(tool));
        harness.run();
        for name in ["Master", "4 ways", "Masks"] {
            let tab = harness.get_by_role_and_label(Role::Tab, name);
            assert!(!tab.accesskit_node().is_disabled(), "{name}");
        }
        // Master first, its sliders in darktable's plain groups.
        harness.get_by_role_and_label(Role::Slider, "Hue shift");
        harness.get_by_role_and_label(Role::Slider, "Contrast");
        harness.get_by_label("Linear chroma grading");
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, "Global offset luminance")
                .is_none()
        );

        harness.get_by_role_and_label(Role::Tab, "4 ways").click();
        harness.run();
        harness.run();
        harness.get_by_label("Global offset");
        harness.get_by_role_and_label(Role::Slider, "Global offset hue");
        harness.get_by_role_and_label(Role::Slider, "Shadows lift chroma");
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, "Contrast")
                .is_none()
        );

        harness.get_by_role_and_label(Role::Tab, "Masks").click();
        harness.run();
        harness.run();
        let formula = harness.get_by_role_and_label(Role::ComboBox, "Saturation formula");
        assert_eq!(
            formula.accesskit_node().value().as_deref(),
            Some("UCS (2022)")
        );
        let fulcrum = harness.get_by_role_and_label(Role::Slider, "Contrast grey fulcrum");
        assert_eq!(fulcrum.accesskit_node().value().as_deref(), Some("18.45%"));
        let tool = "colorbalancergb";
        assert_eq!(
            harness.state().changes,
            [Change::Tab { tool, tab: 1 }, Change::Tab { tool, tab: 2 }]
        );
    }

    #[test]
    fn colour_calibration_draws_its_seven_tabs_and_the_colour_checker_under_each() {
        let mut harness = panel(&ColourCalibration, untouched(&ColourCalibration));
        harness.run();
        const CHECKER: &str = "Calibrate with a colour checker";
        // The CAT tab first and open, darktable's page at the definition's defaults,
        // daylight under CAT16: the adaptation, the illuminant, the temperature daylight
        // reads, gamut compression, clipping and the area colour mapping, closed.
        assert_eq!(harness.state().panels.tab("channelmixerrgb"), 0);
        let adaptation = harness.get_by_role_and_label(Role::ComboBox, "Adaptation");
        assert_eq!(
            adaptation.accesskit_node().value().as_deref(),
            Some("CAT16 (CIECAM16)")
        );
        let illuminant = harness.get_by_role_and_label(Role::ComboBox, "Illuminant");
        assert_eq!(
            illuminant.accesskit_node().value().as_deref(),
            Some("D (daylight)")
        );
        harness.get_by_role_and_label(Role::Slider, "Illuminant temperature");
        for hidden in ["Illuminant hue", "Illuminant chroma"] {
            assert!(
                harness
                    .query_by_role_and_label(Role::Slider, hidden)
                    .is_none(),
                "{hidden} with daylight"
            );
        }
        harness.get_by_role_and_label(Role::Slider, "Gamut compression");
        harness.get_by_role_and_label(Role::Switch, "Clip negative RGB from gamut");
        let mapping = harness.get_by_role_and_label(Role::Button, "Area colour mapping");
        assert_eq!(mapping.accesskit_node().data().is_expanded(), Some(false));
        harness.get_by_role_and_label(Role::Button, CHECKER);
        let tabs = [
            ("R", "Red"),
            ("G", "Green"),
            ("B", "Blue"),
            ("Colourfulness", "Colourfulness"),
            ("Brightness", "Brightness"),
            ("Grey", "Grey"),
        ];
        for (i, (tab, rows)) in tabs.into_iter().enumerate() {
            let node = harness.get_by_role_and_label(Role::Tab, tab);
            assert!(!node.accesskit_node().is_disabled(), "{tab}");
            node.click();
            harness.run();
            harness.run();
            assert_eq!(
                harness.state().panels.tab("channelmixerrgb"),
                i + 1,
                "{tab}"
            );
            assert!(
                harness
                    .query_by_role_and_label(Role::ComboBox, "Adaptation")
                    .is_none(),
                "the CAT tab's rows under {tab}"
            );
            // Its own rows, darktable's page: Input R, G and B, then Normalise channels.
            for input in ["R", "G", "B"] {
                let slider = format!("{rows}, input {input}");
                harness.get_by_role_and_label(Role::Slider, &slider);
            }
            let normalise = format!("{rows}, normalise channels");
            harness.get_by_role_and_label(Role::Switch, &normalise);
            // Every other tab's rows are out of sight.
            for (_, other) in tabs.iter().filter(|(_, other)| *other != rows) {
                let slider = format!("{other}, input R");
                assert!(
                    harness
                        .query_by_role_and_label(Role::Slider, &slider)
                        .is_none(),
                    "{slider} under {tab}"
                );
            }
            // The colour checker under it, closed at first, as darktable's section.
            let group = harness.get_by_role_and_label(Role::Button, CHECKER);
            assert_eq!(group.accesskit_node().data().is_expanded(), Some(false));
        }
        assert!(
            harness
                .state()
                .changes
                .iter()
                .all(|change| matches!(change, Change::Tab { .. })),
            "{:?}",
            harness.state().changes
        );
    }

    #[test]
    fn the_colour_checker_s_settings_are_drawn_disabled_and_set_nothing() {
        let mut harness = panel(&ColourCalibration, untouched(&ColourCalibration));
        // Tall enough for the CAT tab, its readout row too, with the group under it.
        harness.set_size(egui::vec2(400.0, 600.0));
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Calibrate with a colour checker")
            .click();
        harness.run();
        assert_eq!(
            harness.state().changes,
            [Change::Group {
                tool: "channelmixerrgb",
                group: "colour_checker"
            }]
        );
        harness.run();
        // darktable's settings for its colour checker, at darktable's defaults.
        let chart = harness.get_by_role_and_label(Role::ComboBox, "Colour checker chart");
        assert_eq!(
            chart.accesskit_node().value().as_deref(),
            Some("Xrite ColorChecker 24 pre-2014")
        );
        let optimise =
            harness.get_by_role_and_label(Role::ComboBox, "Colours the colour checker favours");
        assert_eq!(optimise.accesskit_node().value().as_deref(), Some("None"));
        let patches = harness.get_by_role_and_label(Role::Slider, "Colour checker patch scale");
        for node in [&chart, &optimise, &patches] {
            assert!(node.accesskit_node().is_disabled());
        }
        chart.click();
        patches.click();
        harness.run();
        harness.run();
        assert_eq!(harness.state().changes.len(), 1);
        assert!(
            harness
                .query_by_label("Datacolor SpyderCheckr Photo")
                .is_none()
        );
    }

    #[test]
    fn rgb_primaries_draws_darktable_s_eight_sliders_in_order() {
        let tool: &'static dyn ProcessingTool = &RgbPrimaries;
        let mut state = untouched(tool);
        set(
            &mut state,
            "red_hue",
            ControlValue::Number(9.5),
            Some(Mark::Kept),
        );
        let purity = ControlValue::Number(-15.7);
        set(&mut state, "red_purity", purity, Some(Mark::Kept));
        let mut harness = panel(tool, state);
        harness.run();
        // One untitled tab, so no tab strip.
        assert!(harness.query_by_role(Role::Tab).is_none());
        let names = [
            "Red hue",
            "Red purity",
            "Green hue",
            "Green purity",
            "Blue hue",
            "Blue purity",
            "Tint hue",
            "Tint purity",
        ];
        let tops: Vec<f32> = names
            .iter()
            .map(|name| {
                harness
                    .get_by_role_and_label(Role::Slider, name)
                    .rect()
                    .top()
            })
            .collect();
        assert!(tops.is_sorted(), "{tops:?}");
        // Each value as darktable's slider shows it: one decimal, a primary's purity less
        // 100, signed while its range spans zero, zero as "+0.0", as the tint's purity's,
        // from 0, does not.
        for (name, value) in [
            ("Red hue", "+9.5°"),
            ("Red purity", "−15.7%"),
            ("Green purity", "+0.0%"),
            ("Tint hue", "+0.0°"),
            ("Tint purity", "0.0%"),
        ] {
            let row = harness.get_by_role_and_label(Role::Slider, name);
            assert_eq!(
                row.accesskit_node().value().as_deref(),
                Some(value),
                "{name}"
            );
        }
        assert!(harness.state().changes.is_empty());
    }

    /// The rows of Vignetting's page that hold a value, by their full names and roles, in
    /// darktable's order.
    const VIGNETTING: [(&str, Role); 10] = [
        ("Fall-off start", Role::Slider),
        ("Fall-off radius", Role::Slider),
        ("Brightness", Role::Slider),
        ("Saturation", Role::Slider),
        ("Horizontal centre", Role::Slider),
        ("Vertical centre", Role::Slider),
        ("Shape", Role::Slider),
        ("Automatic ratio", Role::Switch),
        ("Width/height ratio", Role::Slider),
        ("Dithering", Role::ComboBox),
    ];

    /// Vignetting's section, tall enough for its whole page.
    fn vignetting(state: ToolState) -> Harness<'static, Panel> {
        let mut harness = panel(&Vignetting, state);
        harness.set_size(egui::vec2(400.0, 640.0));
        harness
    }

    /// A switched tool off (decision 32) draws as it would on, at darktable's defaults,
    /// but for its reset, unavailable, and screen readers hear "Not applied" on its
    /// section; a control's change turns it on at once, the reset available and the
    /// description gone, and the reset turns it off again.
    #[test]
    fn a_switched_tool_off_has_no_reset_and_says_not_applied() {
        let mut harness = vignetting(untouched(&Vignetting));
        harness.run();
        let reset = |harness: &Harness<'_, Panel>| {
            harness
                .get_by_label("Reset Vignetting")
                .accesskit_node()
                .is_disabled()
        };
        let said = |harness: &Harness<'_, Panel>| {
            harness
                .get_by_role_and_label(Role::Button, "Vignetting")
                .accesskit_node()
                .description()
                .is_some_and(|said| said.starts_with("Not applied"))
        };
        assert!(reset(&harness));
        assert!(said(&harness));
        // Nothing else is drawn disabled.
        for (name, role) in VIGNETTING {
            let node = harness.get_by_role_and_label(role, name);
            assert!(!node.accesskit_node().is_disabled(), "{name}");
        }

        harness.get_by_label("Automatic ratio").click();
        harness.run();
        assert!(harness.state().state.enabled && harness.state().state.edited);
        assert!(!reset(&harness));
        assert!(!said(&harness));

        harness.get_by_label("Reset Vignetting").click();
        harness.run();
        assert!(!harness.state().state.enabled);
        assert!(reset(&harness));
        assert!(said(&harness));
    }

    #[test]
    fn vignetting_draws_darktable_s_page_in_order() {
        let mut harness = vignetting(untouched(&Vignetting));
        harness.run();
        // One untitled tab, so no tab strip.
        assert!(harness.query_by_role(Role::Tab).is_none());
        let top = |name: &str, role: Role| harness.get_by_role_and_label(role, name).rect().top();
        let tops: Vec<f32> = VIGNETTING
            .iter()
            .map(|&(name, role)| top(name, role))
            .collect();
        assert!(tops.is_sorted(), "{tops:?}");
        // darktable's "position / form" after the strengths, a plain group holding the rest.
        let group = harness.get_by_role_and_label(Role::Group, "Position / form");
        let label = group.rect().top();
        assert!(tops[3] < label && label < tops[4], "{label} among {tops:?}");
        group.get_by_role_and_label(Role::ComboBox, "Dithering");
        // Each value as darktable's slider writes it, with its digits, in the page's
        // order; brightness and saturation at darktable's −0.5, signed as their ranges span
        // zero, as the centres' do. The radii are in percent, the rest numbers.
        let mut values: Vec<(f32, String)> = harness
            .get_all_by_role(Role::Slider)
            .map(|node| {
                let value = node.accesskit_node().value().unwrap_or_default();
                (node.rect().top(), value)
            })
            .collect();
        values.sort_by(|a, b| a.0.total_cmp(&b.0));
        let values: Vec<&str> = values.iter().map(|(_, value)| value.as_str()).collect();
        assert_eq!(
            values,
            [
                "80.00%", "50.00%", "−0.500", "−0.500", "+0.000", "+0.000", "1.00", "1.000"
            ]
        );
        assert_eq!(heard(&harness, "Fall-off start").as_deref(), Some("80.00%"));
        assert_eq!(
            heard(&harness, "Fall-off radius").as_deref(),
            Some("50.00%")
        );
        let dithering = harness.get_by_role_and_label(Role::ComboBox, "Dithering");
        assert_eq!(dithering.accesskit_node().value().as_deref(), Some("Off"));
        assert!(harness.state().changes.is_empty());
    }

    #[test]
    fn vignetting_s_ratio_is_drawn_disabled_while_automatic_ratio_is_on() {
        const RATIO: &str = "Width/height ratio";
        let at = ControlValue::Number(1.5);
        let reported = |autoratio: bool| {
            let mut state = untouched(&Vignetting);
            set(&mut state, "whratio", at.clone(), Some(Mark::Kept));
            let mark = autoratio.then_some(Mark::Kept);
            set(&mut state, "autoratio", ControlValue::Bool(autoratio), mark);
            report_availability(&Vignetting, &mut state);
            state
        };
        let ratio = |harness: &Harness<'_, Panel>| {
            let row = harness.get_by_role_and_label(Role::Slider, RATIO);
            let shown = control(&harness.state().state, "whratio")
                .unwrap()
                .value
                .clone();
            let node = row.accesskit_node();
            (node.is_disabled(), node.numeric_value(), shown)
        };
        let toggle = |harness: &mut Harness<'static, Panel>| {
            harness
                .get_by_role_and_label(Role::Switch, "Automatic ratio")
                .click();
            harness.run();
        };
        let mut harness = vignetting(reported(false));
        harness.run();
        assert_eq!(ratio(&harness), (false, Some(1.5), at.clone()));
        // Automatic ratio on: while the change is pending, the ratio is disabled at once,
        // keeping its value, and so after the session's report.
        toggle(&mut harness);
        let autoratio = |on| Change::Set {
            tool: "vignette",
            control: "autoratio",
            value: ControlValue::Bool(on),
            done: true,
        };
        assert_eq!(harness.state().changes, [autoratio(true)]);
        assert_eq!(ratio(&harness), (true, Some(1.5), at.clone()));
        harness.state_mut().state = reported(true);
        harness.run();
        assert_eq!(ratio(&harness), (true, Some(1.5), at.clone()));
        // Disabled, it takes no change.
        drag(&mut harness, RATIO, 0.25);
        harness.get_by_role_and_label(Role::Slider, RATIO).focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(harness.state().changes.len(), 1);
        // Off again: live at once, its value as it was.
        toggle(&mut harness);
        assert_eq!(harness.state().changes, [autoratio(true), autoratio(false)]);
        assert_eq!(ratio(&harness), (false, Some(1.5), at));
    }

    /// Demosaic's page as darktable's `gui_changed` shows it for the photo's method, each
    /// row with its value, by its full name: on a Bayer raw, RCD, colour smoothing, match
    /// greens and capture sharpen; on an X-Trans raw, Markesteijn 1-pass and the same but
    /// match greens. The method is live and lists the sensor's methods alone, as the
    /// session reports them (decision 30), the unported disabled: a Bayer raw's PPG, never
    /// an X-Trans raw's VNG, and the reverse. Capture sharpen, not ported, is disabled,
    /// and, off, hides its group.
    #[test]
    fn demosaic_draws_darktable_s_rows_for_the_photo_s_method() {
        let bayer = testing::image();
        let mut x_trans = testing::image();
        x_trans.camera.as_mut().unwrap().cfa = Some(apertrust_plugin::Pattern {
            width: 6,
            height: 6,
            values: vec![
                1, 1, 0, 1, 1, 2, 1, 1, 2, 1, 1, 0, 2, 0, 1, 0, 2, 1, 1, 1, 2, 1, 1, 0, 1, 1, 0, 1,
                1, 2, 0, 2, 1, 2, 0, 1,
            ],
        });
        let rows = |photo: &ImageContext, listed: &str, unlisted: &str| {
            let mut state = untouched(&Demosaic);
            let defaults = Demosaic.defaults(photo);
            let shown = state
                .controls
                .iter_mut()
                .find(|c| c.control == "demosaicing_method")
                .unwrap();
            // The photo's method and options, as the session reports them.
            shown.value = defaults.get("demosaicing_method").unwrap().clone();
            shown.default = shown.value.clone();
            shown.options = Demosaic.options("demosaicing_method", photo);
            let mut harness = panel(&Demosaic, state);
            harness.set_size(egui::vec2(400.0, 400.0));
            harness.run();
            assert!(harness.query_by_role(Role::Tab).is_none());
            assert!(harness.query_by_label("Capture sharpen controls").is_none());
            let mut rows: Vec<(f32, String, Option<String>, bool)> = harness
                .get_all_by_role(Role::ComboBox)
                .chain(harness.get_all_by_role(Role::Switch))
                .map(|node| {
                    let n = node.accesskit_node();
                    let name = n.label().unwrap_or_default();
                    (node.rect().top(), name, n.value(), n.is_disabled())
                })
                .collect();
            rows.sort_by(|a, b| a.0.total_cmp(&b.0));
            // The list holds the sensor's methods, the unported disabled.
            harness
                .get_by_role_and_label(Role::ComboBox, "Demosaicing method")
                .click();
            harness.run();
            assert!(harness.get_by_label(listed).accesskit_node().is_disabled());
            assert!(harness.query_by_label(unlisted).is_none());
            assert!(harness.state().changes.is_empty());
            rows.into_iter()
                .map(|(_, name, value, disabled)| (name, value, disabled))
                .collect::<Vec<_>>()
        };
        let row = |name: &str, value: Option<&str>, disabled| {
            (name.to_owned(), value.map(str::to_owned), disabled)
        };
        let colour_smoothing = row("Colour smoothing", Some("Disabled"), false);
        let capture = row("Capture sharpen", None, true);
        assert_eq!(
            rows(&bayer, "PPG", "VNG"),
            [
                row("Demosaicing method", Some("RCD"), false),
                colour_smoothing.clone(),
                row("Match greens", Some("Disabled"), false),
                capture.clone(),
            ]
        );
        assert_eq!(
            rows(&x_trans, "VNG", "PPG"),
            [
                row("Demosaicing method", Some("Markesteijn 1-pass"), false),
                colour_smoothing,
                capture,
            ]
        );

        // A Bayer raw whose edit, from an X-Trans raw's, holds Markesteijn 1-pass: shown
        // chosen, described as not available for this photo, last in the list and
        // disabled, its rows following it as stored (decision 30; Gaps 2).
        let mut state = untouched(&Demosaic);
        let shown = state
            .controls
            .iter_mut()
            .find(|c| c.control == "demosaicing_method")
            .unwrap();
        let markesteijn = Demosaic.defaults(&x_trans);
        shown.value = markesteijn.get("demosaicing_method").unwrap().clone();
        shown.options = Demosaic.options("demosaicing_method", &bayer);
        let mut harness = panel(&Demosaic, state);
        harness.set_size(egui::vec2(400.0, 400.0));
        harness.run();
        let method = harness.get_by_role_and_label(Role::ComboBox, "Demosaicing method");
        let node = method.accesskit_node();
        assert_eq!(node.value().as_deref(), Some("Markesteijn 1-pass"));
        assert_eq!(
            node.description().as_deref(),
            Some("Not available for this photo")
        );
        assert!(harness.query_by_label("Match greens").is_none());
        harness
            .get_by_role_and_label(Role::ComboBox, "Demosaicing method")
            .click();
        harness.run();
        let stored = harness.get_by_label("Markesteijn 1-pass");
        assert!(stored.accesskit_node().is_disabled());
        let last = harness
            .get_by_label("Photosite colour (debug)")
            .rect()
            .top();
        assert!(stored.rect().top() > last);
    }

    /// A tool's pickers are drawn only where its pick is built: Exposure's quad is not
    /// yet, Sample's are. On a photo opening or open for viewing only each is disabled,
    /// saying why, and sends nothing.
    #[test]
    fn pickers_are_drawn_where_their_pick_is_built_and_disabled_saying_why() {
        let pickers = |harness: &Harness<'_, Panel>| {
            harness
                .query_all_by_role(Role::Button)
                .filter_map(|node| node.accesskit_node().label())
                .filter(|label| label.contains("-picker-"))
                .collect::<Vec<_>>()
        };
        let mut harness = panel(&Exposure, untouched(&Exposure));
        harness.run();
        assert_eq!(pickers(&harness), Vec::<String>::new());

        let mut harness = panel(&Sample, untouched(&Sample));
        harness.run();
        assert_eq!(
            pickers(&harness),
            [
                "tool-sample-picker-brightness-full",
                "tool-sample-picker-spread-full",
                "tool-sample-picker-show-full"
            ]
        );
        // Each tooltip is its promise, a picker of both kinds ending with the area's hint.
        let quad =
            harness.get_by_role_and_label(Role::Button, "tool-sample-picker-brightness-full");
        assert_eq!(
            quad.accesskit_node().description().as_deref(),
            Some("tool-sample-picker-brightness-promise")
        );
        let spread = harness.get_by_role_and_label(Role::Button, "tool-sample-picker-spread-full");
        let tip = spread.accesskit_node().description().unwrap_or_default();
        assert!(
            tip.starts_with("tool-sample-picker-spread-promise\n"),
            "{tip}"
        );

        let why = "Opening IMG_2041…";
        harness.state_mut().why = Some(why);
        harness.run();
        let quad =
            harness.get_by_role_and_label(Role::Button, "tool-sample-picker-brightness-full");
        assert!(quad.accesskit_node().is_disabled());
        assert_eq!(quad.accesskit_node().description().as_deref(), Some(why));
        quad.click();
        harness.run();
        assert_eq!(harness.state().changes, []);
    }

    /// `testing::Sample`'s picker rows as the panel draws them, its brightness quad on,
    /// beside a photo with the area's marks focused after a key, at twice the size: only the
    /// rows with pickers, so the snapshot shows the pickers alone. Accepted with
    /// `UPDATE_SNAPSHOTS=1`, reviewed like a golden image.
    fn photograph_pickers(which: Builtin, name: &str) {
        use crate::pickers::{OnPhoto, label as picker_label, name as picker_name, on_photo};
        use apertrust_engine::session::Region;

        const BRIGHTNESS: PickerRef = PickerRef::Tool {
            tool: "sample",
            picker: "brightness",
        };
        let state = untouched(&Sample);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(720.0, 300.0))
            .with_pixels_per_point(2.0)
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui(move |ui| {
                let ctx = ui.ctx().clone();
                let t = theme(&ctx);
                let colours = &t.colors;
                (ui.painter()).rect_filled(ctx.content_rect(), 0.0, colours.photo_surround);
                let panel = egui::Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(300.0, 300.0));
                ui.painter().rect_filled(panel, 0.0, colours.panel);
                let words = translator(&ctx);
                let groups = |_: &str| GROUP_OPEN;
                let pickers = Pickers {
                    drawn: &["sample"],
                    on: Some(BRIGHTNESS),
                    enabled: true,
                    why: None,
                    ..Pickers::NONE
                };
                let page = Page {
                    words: &words,
                    tool: &Sample,
                    state: &state,
                    editable: true,
                    groups: &groups,
                    pickers: &pickers,
                    controls: Default::default(),
                };
                let inside = egui::UiBuilder::new().max_rect(panel.shrink(12.0));
                ui.scope_builder(inside, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let rows = Sample.definition().rows();
                    let top = ui.cursor().top();
                    for row in rows.filter(|row| row.pickers().next().is_some()) {
                        draw_row(ui, &page, row, top, &mut Vec::new());
                    }
                });
                // A sky over a pale wall over earth, as the wireframe's photo.
                let photo = egui::Rect::from_min_size(pos2(330.0, 40.0), egui::vec2(360.0, 220.0));
                let bands = [(141, 162, 181), (217, 210, 196), (110, 91, 69)];
                for (i, (r, g, b)) in bands.into_iter().enumerate() {
                    let third = photo.height() / 3.0;
                    let band = egui::Rect::from_min_size(
                        photo.min + egui::vec2(0.0, i as f32 * third),
                        egui::vec2(photo.width(), third),
                    );
                    let colour = egui::Color32::from_rgb(r, g, b);
                    ui.painter().rect_filled(band, 0.0, colour);
                }
                let name = picker_name(&words, Sample.definition(), "brightness");
                let on = OnPhoto {
                    picker: BRIGHTNESS,
                    kind: PickerKind::Area,
                    area: photo,
                    photo,
                    fitted: photo.size(),
                    pixels: [720, 440],
                    region: Region::Area([0.2, 0.3, 0.7, 0.65]),
                    switched: Region::Point([0.5, 0.5]),
                    colour: egui::Color32::TRANSPARENT,
                    name: &name,
                    label: &picker_label(&words, Sample.definition(), "brightness"),
                    badge: apertrust_ui::Icon::Pick,
                    panning: false,
                };
                on_photo(ui, &on);
            });
        harness.run();
        // The focus ring shows after a key, as on any control.
        harness
            .get_by(|node| node.role() == Role::Group && node.value().is_some())
            .focus();
        harness.run();
        harness.key_press(Key::ArrowDown);
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn pickers_in_the_dark_theme() {
        photograph_pickers(Builtin::Dark, "pickers-dark");
    }

    #[test]
    fn pickers_in_the_light_theme() {
        photograph_pickers(Builtin::Light, "pickers-light");
    }

    /// A button's writes, one gesture: one write ends it at once; several are every one
    /// but the last `done: false`; none on a photo open for viewing only, nor for a tool
    /// or action there is not.
    #[test]
    fn a_button_s_writes_are_one_gesture() {
        use apertrust_tools::orientation::Orientation;
        use apertrust_tools::{Base, Owned, Setting, ToolValues};

        /// Orientation, its rotate writing a second control too.
        struct Twice;
        impl ProcessingTool for Twice {
            fn definition(&self) -> &'static ProcessingDefinition {
                Orientation.definition()
            }
            fn owns(&self) -> &'static [Owned] {
                Orientation.owns()
            }
            fn bind(&self, _: &ToolValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
                Vec::new()
            }
            fn action(
                &self,
                _: &str,
                _: &Controls,
                _: &PublishedValues,
                _: &ImageContext,
            ) -> Option<Controls> {
                let mut writes = Controls::new();
                writes.set("first", ControlValue::Choice(1));
                writes.set("second", ControlValue::Choice(2));
                Some(writes)
            }
        }

        let image = testing::image();
        let values = Orientation.defaults(&image);
        let orientation: &[&dyn ProcessingTool] = &[&Orientation];
        let read = PublishedValues::new();
        assert_eq!(
            action_writes(
                orientation,
                "flip",
                "rotate_cw",
                &values,
                &read,
                &image,
                false
            ),
            [("orientation", ControlValue::Choice(6), true)]
        );
        assert!(
            action_writes(
                orientation,
                "flip",
                "rotate_cw",
                &values,
                &read,
                &image,
                true
            )
            .is_empty()
        );
        assert!(
            action_writes(orientation, "flip", "spin", &values, &read, &image, false).is_empty()
        );
        assert!(
            action_writes(
                orientation,
                "exposure",
                "rotate_cw",
                &values,
                &read,
                &image,
                false
            )
            .is_empty()
        );
        let twice: &[&dyn ProcessingTool] = &[&Twice];
        let dones: Vec<bool> =
            action_writes(twice, "flip", "rotate_cw", &values, &read, &image, false)
                .into_iter()
                .map(|(_, _, done)| done)
                .collect();
        assert_eq!(dones, [false, true]);
    }

    /// Orientation's page as darktable draws it: the transform label with darktable's
    /// four buttons inline, each named, and show guides with the guide settings button
    /// drawn disabled. A button reports its action on the values shown, which the tool
    /// turns as darktable's `do_rotate` does; a double-click on the label, or Delete on it
    /// focused, resets.
    #[test]
    fn orientation_draws_its_buttons_and_a_click_runs_one() {
        use apertrust_tools::orientation::Orientation;
        let mut harness = panel(&Orientation, untouched(&Orientation));
        harness.run();
        let buttons: Vec<(String, bool)> = harness
            .get_all_by_role(Role::Button)
            .map(|node| {
                let n = node.accesskit_node();
                (n.label().unwrap_or_default(), n.is_disabled())
            })
            .filter(|(label, _)| label.starts_with("Rotate") || label.starts_with("Flip"))
            .collect();
        let names = [
            "Rotate 90° anticlockwise",
            "Rotate 90° clockwise",
            "Flip horizontally",
            "Flip vertically",
        ];
        assert_eq!(buttons, names.map(|name| (name.to_owned(), false)).to_vec());
        assert!(
            harness
                .get_by_label("Transform")
                .accesskit_node()
                .label()
                .is_some()
        );
        let guides = harness.get_by_label("Show guides");
        assert!(guides.accesskit_node().is_disabled());
        let settings = harness.get_by_label("Change global guide settings. These settings apply globally, to every tool that shows guide overlays.");
        assert!(settings.accesskit_node().is_disabled());

        harness.get_by_label("Rotate 90° clockwise").click();
        harness.run();
        let changes = std::mem::take(&mut harness.state_mut().changes);
        let [
            Change::Action {
                tool: "flip",
                action: "rotate_cw",
                values,
            },
        ] = &changes[..]
        else {
            panic!("{changes:?}");
        };
        // On auto, the photo's own orientation, none here: a quarter turn clockwise.
        let image = testing::image();
        let read = PublishedValues::new();
        let writes = Orientation
            .action("rotate_cw", values, &read, &image)
            .unwrap();
        assert_eq!(
            writes.get("orientation"),
            Some(&ControlValue::Choice(6)),
            "rotate_cw_90, darktable's 5"
        );

        // Half a second on, so the button's click makes no double-click; then two clicks
        // on the label a frame apart.
        for _ in 0..30 {
            harness.step();
        }
        for _ in 0..2 {
            harness.get_by_label("Transform").click();
            harness.step();
        }
        harness.step();
        let reset = Change::Reset {
            tool: "flip",
            control: "orientation",
        };
        let changes = std::mem::take(&mut harness.state_mut().changes);
        assert!(changes.contains(&reset), "{changes:?}");

        // Delete on the focused label resets too, as a slider's does.
        harness.get_by_label("Transform").focus();
        harness.run();
        harness.key_press(Key::Delete);
        harness.run();
        assert_eq!(harness.state().changes, std::slice::from_ref(&reset));
    }

    /// Output colour profile's page as darktable's `gui_init` draws it: Export profile
    /// alone, by its full name, at sRGB (web-safe), with no tab strip. It is drawn
    /// disabled, as sRGB is the only profile an edit can choose until the display path
    /// converts from the edit's profile (decision 26).
    #[test]
    fn output_colour_profile_draws_export_profile_at_srgb_disabled() {
        let mut harness = panel(&OutputColourProfile, untouched(&OutputColourProfile));
        harness.run();
        assert!(harness.query_by_role(Role::Tab).is_none());
        let rows: Vec<(String, Option<String>, bool)> = harness
            .get_all_by_role(Role::ComboBox)
            .map(|node| {
                let n = node.accesskit_node();
                (n.label().unwrap_or_default(), n.value(), n.is_disabled())
            })
            .collect();
        assert_eq!(
            rows,
            [(
                "Export profile".to_owned(),
                Some("sRGB (web-safe)".to_owned()),
                true
            )]
        );
        harness.get_by_role(Role::ComboBox).click();
        harness.run();
        assert!(harness.state().changes.is_empty());
    }

    /// Input colour profile's page as darktable's `gui_init` draws it: Input profile,
    /// Working profile and Gamut clipping, by their full names, with no tab strip, on a
    /// colour raw at the standard colour matrix. Working profile, porting its default
    /// alone, is drawn disabled; Input profile's list holds the photo's matrix and
    /// darktable's fifteen built-in profiles, the thirteen not ported disabled, and
    /// choosing a linear one sends it; Gamut clipping's five are all enabled, and choosing
    /// one sends it.
    #[test]
    fn input_colour_profile_draws_darktable_s_three_lists() {
        let mut harness = panel(&InputColourProfile, untouched(&InputColourProfile));
        harness.set_size(egui::vec2(400.0, 700.0));
        harness.run();
        assert!(harness.query_by_role(Role::Tab).is_none());
        let rows: Vec<(String, Option<String>, bool)> = harness
            .get_all_by_role(Role::ComboBox)
            .map(|node| {
                let n = node.accesskit_node();
                (n.label().unwrap_or_default(), n.value(), n.is_disabled())
            })
            .collect();
        let row =
            |name: &str, value: &str, disabled| (name.to_owned(), Some(value.to_owned()), disabled);
        assert_eq!(
            rows,
            [
                row("Input profile", "Standard colour matrix", false),
                row("Working profile", "Linear Rec2020 RGB", true),
                row("Gamut clipping", "Off", false),
            ]
        );
        harness
            .get_by_role_and_label(Role::ComboBox, "Input profile")
            .click();
        harness.run();
        assert!(harness.query_by_label("Embedded matrix").is_none());
        for (option, disabled) in [
            ("sRGB", true),
            ("Linear XYZ", true),
            ("BRG (for testing)", true),
            ("Linear Rec709 RGB", false),
        ] {
            let node = harness.get_by_label(option);
            assert_eq!(node.accesskit_node().is_disabled(), disabled, "{option}");
        }
        assert!(harness.state().changes.is_empty());
        harness.get_by_label("Linear Rec709 RGB").click();
        harness.run();
        let lin_rec709 = apertrust_tools::input_colour_profile::INPUT_PROFILES
            .iter()
            .position(|&p| p == "lin_rec709")
            .unwrap();
        let chosen = Change::Set {
            tool: "colorin",
            control: "type",
            value: ControlValue::Choice(lin_rec709 as u32),
            done: true,
        };
        assert_eq!(harness.state().changes, std::slice::from_ref(&chosen));
        // The disabled row sends nothing.
        harness
            .get_by_role_and_label(Role::ComboBox, "Working profile")
            .click();
        harness.run();
        assert_eq!(harness.state().changes, std::slice::from_ref(&chosen));
        harness
            .get_by_role_and_label(Role::ComboBox, "Gamut clipping")
            .click();
        harness.run();
        for option in [
            "sRGB",
            "Adobe RGB (compatible)",
            "Linear Rec709 RGB",
            "Linear Rec2020 RGB",
        ] {
            let node = harness.get_by_label(option);
            assert!(!node.accesskit_node().is_disabled(), "{option}");
        }
        harness.get_by_label("Adobe RGB (compatible)").click();
        harness.run();
        let clipped = Change::Set {
            tool: "colorin",
            control: "normalize",
            value: ControlValue::Choice(2),
            done: true,
        };
        assert_eq!(harness.state().changes, [chosen, clipped]);
    }

    /// Input colour profile's page, then its Input profile list open, in the dark theme.
    #[test]
    fn input_colour_profile_in_the_dark_theme() {
        let mut results = egui_kittest::SnapshotResults::new();
        let state = untouched(&InputColourProfile);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(320.0, 560.0))
            .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
            .build_ui(move |ui| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                ui.spacing_mut().item_spacing.y = 0.0;
                let groups = |_: &str| GROUP_OPEN;
                section(
                    ui,
                    &InputColourProfile,
                    &state,
                    Head {
                        open: true,
                        ..Head::default()
                    },
                    &Panels::default(),
                    &groups,
                    true,
                    None,
                    &Pickers::NONE,
                );
            });
        harness.run();
        harness.snapshot("input-colour-profile");
        harness
            .get_by_role_and_label(Role::ComboBox, "Input profile")
            .click();
        harness.run();
        harness.snapshot("input-colour-profile-list");
        results.extend_harness(&mut harness);
    }

    /// AgX's page as darktable draws it: on Settings, Read exposure drawn disabled until
    /// what it runs is built, dynamic range scaling live, and the three collapsible groups, Show curve disabled, Advanced curve
    /// parameters showing the gamma only while the pivot is not kept on the diagonal; on
    /// Primaries, while adjustments are on, Reset primaries' menu button, and the primaries
    /// before tone mapping, Reverse all with Set from above at its end, which runs on the
    /// values shown and is disabled at once as Reverse all turns on, and while Reverse all
    /// is off the purity boosts after it.
    #[test]
    fn agx_draws_darktable_s_page_its_actions_disabled() {
        let mut harness = panel(&Agx, untouched(&Agx));
        harness.set_size(egui::vec2(400.0, 1600.0));
        harness.run();
        let tabs: Vec<String> = harness
            .get_all_by_role(Role::Tab)
            .map(|tab| tab.accesskit_node().label().unwrap_or_default())
            .collect();
        assert_eq!(tabs, ["Settings", "Primaries"]);
        let disabled = |harness: &Harness<'_, Panel>, role, label: &str| {
            harness
                .get_by_role_and_label(role, label)
                .accesskit_node()
                .is_disabled()
        };
        assert!(!disabled(&harness, Role::Button, "Read exposure"));
        assert!(!disabled(&harness, Role::Slider, "Dynamic range scaling"));
        assert!(!disabled(&harness, Role::Slider, "White relative exposure"));
        assert!(!disabled(&harness, Role::Slider, "Pivot relative exposure"));
        for group in ["Advanced curve parameters", "Look"] {
            assert!(!disabled(&harness, Role::Button, group), "{group}");
        }
        // Show curve has nothing behind it yet, so it is disabled, as a tab with no rows.
        assert!(disabled(&harness, Role::Button, "Show curve"));
        assert!(harness.query_by_label("Curve y gamma").is_none());

        // Kept open by the shell, Show curve stays closed; the advanced parameters open.
        for group in ["show_curve", "advanced_curve_parameters"] {
            harness.state_mut().groups.insert(group, true);
        }
        harness.run();
        let show_curve = harness.get_by_role_and_label(Role::Button, "Show curve");
        assert_eq!(
            show_curve.accesskit_node().data().is_expanded(),
            Some(false)
        );
        assert!(harness.query_by_label("Curve y gamma").is_some());
        set(
            &mut harness.state_mut().state,
            "auto_gamma",
            ControlValue::Bool(true),
            None,
        );
        harness.run();
        assert!(harness.query_by_label("Curve y gamma").is_none());

        harness.state_mut().panels.apply(&Change::Tab {
            tool: "agx",
            tab: 1,
        });
        harness.run();
        assert!(!disabled(&harness, Role::Button, "Reset primaries"));
        assert!(!disabled(&harness, Role::Button, "Set from above"));
        assert!(harness.query_by_label("Red attenuation").is_some());
        assert!(harness.query_by_label("Red purity boost").is_some());
        harness
            .get_by_role_and_label(Role::Button, "Set from above")
            .click();
        harness.run();
        let pressed = harness.state().changes.last();
        assert!(
            matches!(
                pressed,
                Some(Change::Action {
                    tool: "agx",
                    action: "set_from_above",
                    ..
                })
            ),
            "{pressed:?}"
        );
        // Reverse all on: Set from above is disabled at once, on the change pending.
        harness
            .get_by_role_and_label(Role::Switch, "Reverse all")
            .click();
        harness.run();
        assert!(harness.query_by_label("Red purity boost").is_none());
        assert!(disabled(&harness, Role::Button, "Set from above"));
        let state = &harness.state().state;
        let shown = state.actions.iter().find(|a| a.id == "set_from_above");
        assert_eq!(shown.map(|a| a.available), Some(false));
        // So is its registry action, as the mirror shows the change pending.
        let mirrored = crate::actions::tool_action_bits(std::slice::from_ref(state));
        let at = (crate::actions::tool_actions().iter())
            .position(|a| a.id.0 == "tool-agx-set_from_above")
            .unwrap();
        assert_eq!(mirrored & (1 << at), 0);
        set(
            &mut harness.state_mut().state,
            "disable_primaries_adjustments",
            ControlValue::Bool(true),
            None,
        );
        harness.run();
        for gone in ["Red attenuation", "Reset primaries", "Set from above"] {
            assert!(harness.query_by_label(gone).is_none(), "{gone}");
        }
        assert!(harness.state().changes.iter().all(|change| matches!(
            change,
            Change::Tab { .. } | Change::Group { .. } | Change::Action { .. } | Change::Set { .. }
        )));
    }

    /// Set from above's writes as the app sends them: one `SetControl` for each control it
    /// writes, every one but the last `done: false`, so they are one gesture and one
    /// history step.
    #[test]
    fn set_from_above_is_sent_as_one_gesture() {
        let mut values = values(&untouched(&Agx));
        values.set("red_inset", ControlValue::Number(20.0));
        let writes = action_writes(
            &[&Agx],
            "agx",
            "set_from_above",
            &values,
            &PublishedValues::new(),
            &testing::image(),
            false,
        );
        assert_eq!(writes.len(), 8);
        let done: Vec<bool> = writes.iter().map(|(_, _, done)| *done).collect();
        assert_eq!(
            done,
            [false, false, false, false, false, false, false, true]
        );
        assert!(writes.contains(&("red_outset", ControlValue::Number(20.0), false)));
        // Nothing on a photo open for viewing only.
        let viewing = action_writes(
            &[&Agx],
            "agx",
            "set_from_above",
            &values,
            &PublishedValues::new(),
            &testing::image(),
            true,
        );
        assert!(viewing.is_empty());
    }

    /// Every slider row of every built tool, at either end of its range, in each
    /// language and the pseudo-locale, at the Tools panel's narrowest: its value is drawn
    /// whole, with its unit, as darktable writes it and as screen readers hear it, 5 points
    /// from the row's end, the widest, the tone equalizer's Edges refinement/feathering at
    /// its top, "10000.00", included, never elided; the label, cut where the line has no
    /// room for it, ends before it (decision 42: the label is cut, never the value). A
    /// galley's text is the whole of it either way, so what was cut is its `elided`.
    #[test]
    fn every_row_s_value_is_drawn_whole_and_its_label_cut_before_it() {
        let t = Theme::default();
        let content = 240.0 - 2.0 * t.spacing.l;
        let end = t.spacing.xs + t.shape.hairline;
        for locale in [Locale::EnGb, Locale::Uk, Locale::Pl, Locale::Pseudo] {
            let mut cut = 0;
            for at_max in [false, true] {
                let states: Vec<(&'static dyn ProcessingTool, ToolState)> = apertrust_tools::TOOLS
                    .iter()
                    .map(|&tool| {
                        let mut state = untouched(tool);
                        for shown in &mut state.controls {
                            shown.available = true;
                        }
                        for control in tool.definition().controls {
                            if let ControlKind::Number { min, max, .. } = control.kind {
                                let value = if at_max { max } else { min };
                                set(&mut state, control.id, ControlValue::Number(value), None);
                            }
                        }
                        (tool, state)
                    })
                    .collect();
                let rows: usize = states
                    .iter()
                    .map(|(tool, _)| sliders(tool.definition()).len())
                    .sum();
                let height = rows as f32 * t.sizes.slider_row + 40.0;
                let mut harness = Harness::builder()
                    .with_size(egui::vec2(content, height))
                    .with_context_setup(move |ctx| {
                        apertrust_ui::install(ctx, Theme::default());
                        apertrust_ui::set_locale(ctx, locale);
                    })
                    .build_ui(move |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let words = translator(ui.ctx());
                        for (tool, state) in &states {
                            let definition = tool.definition();
                            let groups = |_: &str| GROUP_OPEN;
                            let page = Page {
                                words: &words,
                                tool: *tool,
                                state,
                                editable: true,
                                groups: &groups,
                                pickers: &Pickers::NONE,
                                controls: Default::default(),
                            };
                            ui.push_id(definition.tool.id, |ui| {
                                for slider in sliders(definition) {
                                    let mut changes = Vec::new();
                                    slider_row(ui, &page, slider, &mut changes);
                                }
                            });
                        }
                    });
                harness.run();
                let texts: Vec<(String, egui::Rect, bool)> = harness
                    .output()
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(shape) => Some((
                            shape.galley.text().to_owned(),
                            shape.galley.rect.translate(shape.pos.to_vec2()),
                            shape.galley.elided,
                        )),
                        _ => None,
                    })
                    .collect();
                let tracks: Vec<(egui::Rect, String)> = harness
                    .get_all_by_role(Role::Slider)
                    .map(|node| {
                        let value = node.accesskit_node().value().unwrap_or_default();
                        (node.rect(), value)
                    })
                    .collect();
                assert_eq!(tracks.len(), rows, "{locale:?}: a track a row");
                for (strip, heard) in tracks {
                    // The top line's texts, over the strip.
                    let middle = strip.top() - t.sizes.field / 2.0;
                    let line: Vec<&(String, egui::Rect, bool)> = texts
                        .iter()
                        .filter(|(_, rect, _)| {
                            (rect.center().y - middle).abs() < 2.0
                                && rect.left() >= strip.left() - 0.5
                                && rect.right() <= strip.right() + 0.5
                        })
                        .collect();
                    let (value, at, elided) = line
                        .iter()
                        .max_by(|a, b| a.1.right().total_cmp(&b.1.right()))
                        .copied()
                        .unwrap_or_else(|| panic!("{locale:?}: {heard} is drawn"));
                    assert_eq!(value, &heard, "{locale:?}: drawn as screen readers hear it");
                    assert!(!elided, "{locale:?}: {value} cut");
                    assert!(
                        (at.right() - (strip.right() - end)).abs() < 0.5,
                        "{locale:?}: {value} ends at {at:?} in {strip:?}"
                    );
                    for (label, rect, elided) in &line {
                        if rect.right() < at.right() {
                            assert!(
                                rect.right() <= at.left() + 0.5,
                                "{locale:?}: {label} runs into {value}"
                            );
                            cut += usize::from(*elided);
                        }
                    }
                }
                // The widest value, whole.
                let top = match locale {
                    Locale::EnGb | Locale::Pseudo => "10000.00",
                    _ => "10000,00",
                };
                assert_eq!(
                    at_max,
                    texts.iter().any(|(text, _, _)| text == top),
                    "{locale:?}"
                );
            }
            // At 240 points some labels give way, so the cut is seen, not assumed.
            assert!(cut > 0, "{locale:?}: no label cut");
        }
    }

    /// The slider rows a definition draws, each control once, in its tabs, groups and
    /// modes.
    fn sliders(definition: &ProcessingDefinition) -> Vec<&Slider> {
        let mut seen = Vec::new();
        definition
            .rows()
            .filter_map(|row| match row {
                Row::Slider(slider)
                    if definition.control(slider.control).is_some()
                        && !seen.contains(&slider.control) =>
                {
                    seen.push(slider.control);
                    Some(slider)
                }
                _ => None,
            })
            .collect()
    }

    /// The tone equalizer's mask display, under every tab.
    const MASK_DISPLAY: &str = "Display exposure mask";

    /// The tone equalizer's graph of its bands, on its Advanced tab.
    const GRAPH: &str = "Tone equalizer, curve";

    /// The rows darktable shows only with a mask filter, in its order.
    const FILTER_ROWS: [&str; 4] = [
        "Filter diffusion",
        "Smoothing diameter",
        "Edges refinement/feathering",
        "Mask quantisation",
    ];

    /// The tops of the sliders named, which must all be drawn.
    fn tops(harness: &Harness<'_, Panel>, names: &[&str]) -> Vec<f32> {
        names
            .iter()
            .map(|name| {
                harness
                    .get_by_role_and_label(Role::Slider, name)
                    .rect()
                    .top()
            })
            .collect()
    }

    #[test]
    fn the_tone_equalizer_draws_darktable_s_three_tabs_and_the_mask_display_under_each() {
        let mut harness = panel(&ToneEqualizer, untouched(&ToneEqualizer));
        harness.run();
        let tabs: [(&str, &[&str]); 3] = [
            (
                "Simple",
                &[
                    "−8 EV, blacks",
                    "−7 EV, deep shadows",
                    "−6 EV, shadows",
                    "−5 EV, light shadows",
                    "−4 EV, mid-tones",
                    "−3 EV, dark highlights",
                    "−2 EV, highlights",
                    "−1 EV, whites",
                    "+0 EV, speculars",
                ],
            ),
            (
                "Advanced",
                &[
                    "Curve smoothing",
                    "Mask exposure compensation",
                    "Mask contrast compensation",
                ],
            ),
            ("Masking", &FILTER_ROWS),
        ];
        for (i, (tab, sliders)) in tabs.into_iter().enumerate() {
            let node = harness.get_by_role_and_label(Role::Tab, tab);
            assert!(!node.accesskit_node().is_disabled(), "{tab}");
            node.click();
            harness.run();
            harness.run();
            assert_eq!(harness.state().panels.tab("toneequal"), i, "{tab}");
            // darktable's rows, in its order, and none of the other tabs'; but on Advanced,
            // its graph of the bands first, its nodes the bands.
            let tops = tops(&harness, sliders);
            assert!(tops.is_sorted(), "{tab}: {tops:?}");
            let graph = harness.query_by_role_and_label(Role::Group, GRAPH);
            assert_eq!(graph.is_some(), tab == "Advanced", "{tab}");
            if let Some(graph) = graph {
                assert!(graph.rect().bottom() <= tops[0], "{tab}");
            }
            for (other_tab, other) in tabs.iter().filter(|(other, _)| *other != tab) {
                if tab == "Advanced" && *other_tab == "Simple" {
                    continue;
                }
                assert!(
                    harness
                        .query_by_role_and_label(Role::Slider, other[0])
                        .is_none(),
                    "{} under {tab}",
                    other[0]
                );
            }
            // The mask's display after them, drawn disabled until the mask preview comes.
            let mask = harness.get_by_role_and_label(Role::Switch, MASK_DISPLAY);
            assert!(mask.accesskit_node().is_disabled(), "{tab}");
            let last = harness
                .get_by_role_and_label(Role::Slider, sliders[sliders.len() - 1])
                .rect();
            assert!(mask.rect().top() >= last.bottom(), "{tab}");
        }
        // Each value as darktable's slider shows it: with its unit, or a number alone.
        for (name, value) in [
            ("Filter diffusion", 1.0),
            ("Edges refinement/feathering", 1.0),
        ] {
            let row = harness.get_by_role_and_label(Role::Slider, name);
            assert_eq!(row.accesskit_node().numeric_value(), Some(value), "{name}");
        }
        for (name, value) in [
            ("Smoothing diameter", "5.00%"),
            ("Mask quantisation", "0.00 EV"),
        ] {
            let row = harness.get_by_role_and_label(Role::Slider, name);
            assert_eq!(
                row.accesskit_node().value().as_deref(),
                Some(value),
                "{name}"
            );
        }
        for (name, value) in [
            ("Luminance estimator", "RGB euclidean norm"),
            ("Preserve details", "EIGF"),
        ] {
            let row = harness.get_by_role_and_label(Role::ComboBox, name);
            assert_eq!(
                row.accesskit_node().value().as_deref(),
                Some(value),
                "{name}"
            );
        }
        assert!(
            harness
                .state()
                .changes
                .iter()
                .all(|change| matches!(change, Change::Tab { .. })),
            "{:?}",
            harness.state().changes
        );
    }

    #[test]
    fn the_tone_equalizer_s_filter_rows_follow_preserve_details() {
        let mut harness = panel(&ToneEqualizer, untouched(&ToneEqualizer));
        harness.run();
        harness.get_by_role_and_label(Role::Tab, "Masking").click();
        harness.run();
        harness.run();
        // EIGF, darktable's default: the filter's rows show.
        tops(&harness, &FILTER_ROWS);
        let choose = |harness: &mut Harness<'_, Panel>, option: &str| {
            harness
                .get_by_role_and_label(Role::ComboBox, "Preserve details")
                .click();
            harness.run();
            harness.get_by_label(option).click();
            harness.run();
            harness.run();
        };
        // No filter: they hide, as darktable hides them.
        choose(&mut harness, "No");
        for name in FILTER_ROWS {
            assert!(
                harness
                    .query_by_role_and_label(Role::Slider, name)
                    .is_none(),
                "{name} without a filter"
            );
        }
        harness.get_by_role_and_label(Role::Switch, MASK_DISPLAY);
        // The guided filters are listed but not ported, so drawn disabled; Averaged EIGF,
        // a filter, brings the rows back.
        harness
            .get_by_role_and_label(Role::ComboBox, "Preserve details")
            .click();
        harness.run();
        for option in ["Averaged guided filter", "Guided filter"] {
            let node = harness.get_by_label(option);
            assert!(node.accesskit_node().is_disabled(), "{option}");
        }
        harness.get_by_label("Averaged EIGF").click();
        harness.run();
        harness.run();
        tops(&harness, &FILTER_ROWS);
        // The Advanced tab's contrast compensation follows the filter too: shown with EIGF,
        // as at first, but not with its averaged form, as darktable shows it.
        harness.get_by_role_and_label(Role::Tab, "Advanced").click();
        harness.run();
        harness.run();
        harness.get_by_role_and_label(Role::Slider, "Mask exposure compensation");
        assert!(
            harness
                .query_by_role_and_label(Role::Slider, "Mask contrast compensation")
                .is_none()
        );
        let details = |option| Change::Set {
            tool: "toneequal",
            control: "details",
            value: ControlValue::Choice(option),
            done: true,
        };
        let sets: Vec<&Change> = harness
            .state()
            .changes
            .iter()
            .filter(|change| matches!(change, Change::Set { .. }))
            .collect();
        assert_eq!(sets, [&details(0), &details(3)]);
    }

    /// The tone equalizer's section open on its Advanced tab, its bands at `bands`, and
    /// the graph's frame, less the room its nodes take round it.
    fn advanced(bands: [f32; 9]) -> (Harness<'static, Panel>, egui::Rect) {
        let mut state = untouched(&ToneEqualizer);
        for (band, value) in tone_equalizer::BANDS.iter().zip(bands) {
            if value != 0.0 {
                set(
                    &mut state,
                    band,
                    ControlValue::Number(value),
                    Some(Mark::Kept),
                );
                // A band set turns the switched tool on, as the session reports it.
                (state.enabled, state.edited) = (true, true);
            }
        }
        let mut harness = panel(&ToneEqualizer, state);
        harness.run();
        harness.get_by_role_and_label(Role::Tab, "Advanced").click();
        harness.run();
        harness.run();
        harness.state_mut().changes.clear();
        let graph = harness
            .get_by_role_and_label(Role::Group, GRAPH)
            .rect()
            .shrink(7.0);
        (harness, graph)
    }

    /// The bands' values the changes set, by band, and whether each set ended its gesture.
    fn bands_set(changes: &[Change]) -> Vec<(&'static str, f32, bool)> {
        changes
            .iter()
            .map(|change| match change {
                Change::Set {
                    tool: "toneequal",
                    control,
                    value: ControlValue::Number(value),
                    done,
                } => (*control, *value, *done),
                other => panic!("not a band's value: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn dragging_the_tone_equalizer_s_graph_moves_the_bands_as_darktable_s_does() {
        let (mut harness, graph) = advanced([0.0; 9]);
        // From the −4 EV node up a quarter of the graph, 1 EV: at darktable's default
        // smoothing each band rises by e^(−d²/2) of it, d its distance in EV, then, as
        // darktable's do, settles on the curve those bands give, its correction at each
        // node, rounded to the two decimals its slider shows: 0.96 where the falloff gives
        // 1, 0.65 for 0.61, 0.09 for 0.14, 0.04 for 0.01, and the ends, 0.0003, −0.01,
        // where darktable's projection, restated in f64, gives −0.0098.
        let from = pos2(graph.center().x, graph.center().y);
        let to = pos2(from.x, from.y - graph.height() / 4.0);
        harness.hover_at(from);
        harness.step();
        harness.drag_at(from);
        harness.step();
        harness.hover_at(to);
        harness.step();
        harness.drop_at(to);
        harness.step();
        let falloff = [
            ("noise", -0.01),
            ("ultra_deep_blacks", 0.04),
            ("deep_blacks", 0.09),
            ("blacks", 0.65),
            ("shadows", 0.96),
            ("midtones", 0.65),
            ("highlights", 0.09),
            ("whites", 0.04),
            ("speculars", -0.01),
        ];
        let moved = |done| -> Vec<(&str, f32, bool)> {
            falloff
                .iter()
                .map(|&(band, value)| (band, value, done))
                .collect()
        };
        // Every change while it moves, then once more on release, the bands' releases sent
        // as one step, which takes in the drag (decision 31).
        let changes = &harness.state().changes;
        assert_eq!(bands_set(&changes[..9]), moved(false));
        let [Change::Step(released)] = &changes[9..] else {
            panic!("the release as one step: {changes:?}");
        };
        assert_eq!(bands_set(released), moved(true));
        // The mirror shows them at once, the slider rows of the Simple tab as well.
        let shown = control(&harness.state().state, "shadows").unwrap();
        assert_eq!(shown.value, ControlValue::Number(0.96));
        let node = harness.get_by_role_and_label(Role::Slider, "−4 EV, mid-tones");
        assert_eq!(
            node.accesskit_node().numeric_value(),
            Some(f64::from(0.96f32))
        );
    }

    #[test]
    fn a_double_click_on_the_tone_equalizer_s_graph_resets_its_bands() {
        let (mut harness, _) = advanced([0.5, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.25]);
        // Half a second on, so the tab's click makes no double-click; then two clicks.
        for _ in 0..30 {
            harness.step();
        }
        for _ in 0..2 {
            harness.get_by_role_and_label(Role::Group, GRAPH).click();
            harness.step();
        }
        harness.step();
        assert_eq!(harness.state().changes, [bands_reset()]);
        let values: Vec<ControlValue> = tone_equalizer::BANDS
            .iter()
            .map(|band| control(&harness.state().state, band).unwrap().value.clone())
            .collect();
        assert_eq!(values, vec![ControlValue::Number(0.0); 9]);
    }

    #[test]
    fn the_tone_equalizer_s_graph_moves_and_resets_a_band_from_the_keys() {
        let (mut harness, _) = advanced([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5]);
        harness.get_by_role_and_label(Role::Group, GRAPH).focus();
        harness.run();
        // End chooses +0 EV; a step up is its slider's, 0.01 EV, ten with Shift.
        harness.key_press(Key::End);
        harness.run();
        harness.key_press(Key::ArrowUp);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowUp);
        harness.run();
        // Ctrl's tenth of a step, 0.001 EV, is finer than the two decimals the band's row
        // shows, so it moves by the last of them, as a slider row's does.
        harness.key_press_modifiers(Modifiers::CTRL | Modifiers::COMMAND, Key::ArrowDown);
        harness.run();
        assert_eq!(
            bands_set(&harness.state().changes),
            [
                ("speculars", 0.51, true),
                ("speculars", 0.61, true),
                ("speculars", 0.6, true)
            ]
        );
        // Delete resets it, Shift+Delete every band.
        harness.state_mut().changes.clear();
        harness.key_press(Key::Delete);
        harness.run();
        harness.key_press_modifiers(Modifiers::SHIFT, Key::Delete);
        harness.run();
        let changes = &harness.state().changes;
        let reset = Change::Reset {
            tool: "toneequal",
            control: "speculars",
        };
        assert_eq!(changes, &[reset, bands_reset()]);
    }

    /// The nine bands' reset, as one step.
    fn bands_reset() -> Change {
        let resets = tone_equalizer::BANDS.iter().map(|&control| Change::Reset {
            tool: "toneequal",
            control,
        });
        Change::Step(resets.collect())
    }

    /// The Tone equalizer, its curve or the curve's width panicking, as a tool's code may.
    struct Shaky {
        curve: bool,
        width: bool,
    }

    impl ProcessingTool for Shaky {
        fn definition(&self) -> &'static ProcessingDefinition {
            ToneEqualizer.definition()
        }

        fn owns(&self) -> &'static [apertrust_tools::Owned] {
            ToneEqualizer.owns()
        }

        fn bind(
            &self,
            values: &apertrust_tools::ToolValues,
            image: &ImageContext,
            base: &apertrust_tools::Base,
        ) -> Vec<apertrust_tools::Setting> {
            ToneEqualizer.bind(values, image, base)
        }

        fn curve(&self, control: &str, values: &Controls) -> Option<Box<dyn Fn(f32) -> f32>> {
            assert!(!self.curve, "the curve panics");
            ToneEqualizer.curve(control, values)
        }

        fn curve_width(&self, control: &str, values: &Controls) -> Option<f32> {
            assert!(!self.width, "the width panics");
            ToneEqualizer.curve_width(control, values)
        }
    }

    static CURVE_PANICS: Shaky = Shaky {
        curve: true,
        width: false,
    };
    static WIDTH_PANICS: Shaky = Shaky {
        curve: false,
        width: true,
    };

    /// Off, the tone equalizer corrects nothing and its graph is flat; on at its defaults
    /// it is the curve the operation renders, darktable's ripple of up to 0.025 EV.
    #[test]
    fn the_band_graph_is_flat_while_off_and_the_ripple_on_at_the_defaults() {
        let values = ToneEqualizer.definition().defaults();
        let first = tone_equalizer::BANDS[0];
        let off = equalizer_curve(&ToneEqualizer, false, first, &values, -8.0)
            .unwrap()
            .unwrap();
        assert!(off.iter().all(|&ev| ev == 0.0));
        let on = equalizer_curve(&ToneEqualizer, true, first, &values, -8.0)
            .unwrap()
            .unwrap();
        let highest = on.iter().copied().fold(f32::MIN, f32::max);
        assert!((highest - 0.0253).abs() < 1e-3, "{highest}");
    }

    #[test]
    fn a_graph_whose_tool_panics_draws_no_curve_and_moves_nothing() {
        for tool in [&CURVE_PANICS, &WIDTH_PANICS] {
            let mut state = untouched(tool);
            // On, so the drawing asks the tool for its curve, which panics.
            (state.enabled, state.edited) = (true, true);
            let mut harness = panel(tool, state);
            harness.run();
            harness.get_by_role_and_label(Role::Tab, "Advanced").click();
            harness.run();
            harness.run();
            harness.state_mut().changes.clear();
            let graph = harness
                .get_by_role_and_label(Role::Group, GRAPH)
                .rect()
                .shrink(7.0);
            let from = graph.center();
            let to = pos2(from.x, from.y - graph.height() / 4.0);
            harness.hover_at(from);
            harness.step();
            harness.drag_at(from);
            harness.step();
            harness.hover_at(to);
            harness.step();
            harness.drop_at(to);
            harness.step();
            assert!(
                harness.state().changes.is_empty(),
                "{:?}",
                harness.state().changes
            );
        }
    }

    #[test]
    fn the_tone_equalizer_s_graph_is_disabled_on_a_photo_open_for_viewing_only() {
        let mut state = untouched(&ToneEqualizer);
        for shown in &mut state.controls {
            shown.available = false;
        }
        let mut harness = panel(&ToneEqualizer, state);
        harness.run();
        harness.get_by_role_and_label(Role::Tab, "Advanced").click();
        harness.run();
        harness.run();
        let graph = harness.get_by_role_and_label(Role::Group, GRAPH);
        assert!(graph.accesskit_node().is_disabled());
        let rect = graph.rect();
        harness.hover_at(rect.center());
        harness.step();
        harness.drag_at(rect.center());
        harness.step();
        harness.hover_at(rect.center_top());
        harness.step();
        harness.drop_at(rect.center_top());
        harness.step();
        assert!(
            harness
                .state()
                .changes
                .iter()
                .all(|change| matches!(change, Change::Tab { .. })),
            "{:?}",
            harness.state().changes
        );
    }

    /// Tone curve's section with a curve drawn, on Luma, and open on Advanced, in every
    /// theme and the pseudo-locale, as wide as the Tools panel at its narrowest, where its
    /// six tabs are tightest.
    fn photograph_tone_curve(which: Builtin, locale: Locale, name: &str) {
        let mut luma = untouched(&ToneCurve);
        set(
            &mut luma,
            "luma_points",
            ControlValue::Points(vec![[0.17, 0.11], [0.5, 0.5], [0.83, 0.89]]),
            Some(Mark::Kept),
        );
        let mut panels = [Panels::default(), Panels::default()];
        panels[1].apply(&Change::Tab {
            tool: "tone_curve",
            tab: 5,
        });
        let states = [luma.clone(), luma];
        let mut harness = Harness::builder()
            .with_size(egui::vec2(320.0, 520.0))
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(which));
                apertrust_ui::set_locale(ctx, locale);
            })
            .build_ui(move |ui| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, (state, panels)) in states.iter().zip(&panels).enumerate() {
                    let groups = |_: &str| GROUP_OPEN;
                    ui.push_id(i, |ui| {
                        section(
                            ui,
                            &ToneCurve,
                            state,
                            Head {
                                open: true,
                                ..Head::default()
                            },
                            panels,
                            &groups,
                            true,
                            None,
                            &Pickers::NONE,
                        )
                    });
                }
            });
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn tone_curve_in_the_dark_theme() {
        photograph_tone_curve(Builtin::Dark, Locale::EnGb, "tone-curve-dark");
    }

    #[test]
    fn tone_curve_in_the_grey_theme() {
        photograph_tone_curve(Builtin::Grey, Locale::EnGb, "tone-curve-grey");
    }

    #[test]
    fn tone_curve_in_the_light_theme() {
        photograph_tone_curve(Builtin::Light, Locale::EnGb, "tone-curve-light");
    }

    #[test]
    fn tone_curve_in_the_high_contrast_theme() {
        photograph_tone_curve(
            Builtin::HighContrast,
            Locale::EnGb,
            "tone-curve-high-contrast",
        );
    }

    #[test]
    fn tone_curve_in_the_pseudo_locale() {
        photograph_tone_curve(Builtin::Dark, Locale::Pseudo, "tone-curve-pseudo");
    }

    /// Exposure's section as a photographer meets it, in every theme and the
    /// pseudo-locale: open with Exposure kept on a photo whose camera rows act, each with
    /// its value after its label, open with Black level correction set on the sample
    /// photo, which hides them, and closed. Picker target is drawn disabled at Middle
    /// grey, its pick not drawn here. What is not ported is disabled, as the session
    /// reports it.
    fn photograph(which: Builtin, locale: Locale, name: &str) {
        let mut kept = untouched(&Exposure);
        on_photo(&Exposure, &mut kept, &camera_photo());
        set(
            &mut kept,
            "exposure",
            ControlValue::Number(0.42),
            Some(Mark::Kept),
        );
        let definition = Exposure.definition();
        for control in &mut kept.controls {
            control.available = definition
                .control(control.control)
                .is_some_and(|found| !found.unported);
        }
        let mut black = kept.clone();
        set(
            &mut black,
            "black_level",
            ControlValue::Number(0.01),
            Some(Mark::Kept),
        );
        on_photo(&Exposure, &mut black, &testing::image());
        let panels = [Panels::default(), Panels::default(), Panels::default()];

        let states = [kept.clone(), black, kept];
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 620.0))
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(which));
                apertrust_ui::set_locale(ctx, locale);
            })
            .build_ui(move |ui| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, (state, panels)) in states.iter().zip(&panels).enumerate() {
                    let open = i < 2;
                    let groups = |_: &str| false;
                    ui.push_id(i, |ui| {
                        section(
                            ui,
                            &Exposure,
                            state,
                            Head {
                                open,
                                ..Head::default()
                            },
                            panels,
                            &groups,
                            true,
                            None,
                            &Pickers::NONE,
                        )
                    });
                }
            });
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn exposure_in_the_dark_theme() {
        photograph(Builtin::Dark, Locale::EnGb, "exposure-dark");
    }

    #[test]
    fn exposure_in_the_grey_theme() {
        photograph(Builtin::Grey, Locale::EnGb, "exposure-grey");
    }

    #[test]
    fn exposure_in_the_light_theme() {
        photograph(Builtin::Light, Locale::EnGb, "exposure-light");
    }

    #[test]
    fn exposure_in_the_high_contrast_theme() {
        photograph(
            Builtin::HighContrast,
            Locale::EnGb,
            "exposure-high-contrast",
        );
    }

    #[test]
    fn exposure_in_the_pseudo_locale() {
        photograph(Builtin::Dark, Locale::Pseudo, "exposure-pseudo");
    }

    /// Exposure's automatic mode as the board draws it (decision 88), open, one under
    /// another: on a raw with the exposure it computed; on an image that is not a raw
    /// holding Manual, Manual drawn as chosen and Automatic disabled; and there holding
    /// Automatic, pasted, chosen and disabled with Manual live.
    fn photograph_automatic(which: Builtin, name: &str) {
        let mut raw = untouched(&Exposure);
        set(&mut raw, "mode", ControlValue::Choice(1), Some(Mark::Kept));
        raw.computed = vec![apertrust_engine::session::ComputedState {
            id: "computed_exposure",
            value: Some(0.514),
        }];
        let states = [raw, not_a_raw(0), not_a_raw(1)];
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 740.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui(move |ui| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, state) in states.iter().enumerate() {
                    ui.push_id(i, |ui| {
                        section(
                            ui,
                            &Exposure,
                            state,
                            Head {
                                open: true,
                                ..Head::default()
                            },
                            &Panels::default(),
                            &|_: &str| false,
                            true,
                            None,
                            &Pickers::DRAWN,
                        )
                    });
                }
            });
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn exposure_s_automatic_mode_in_the_dark_theme() {
        photograph_automatic(Builtin::Dark, "exposure-automatic-dark");
    }

    #[test]
    fn exposure_s_automatic_mode_in_the_light_theme() {
        photograph_automatic(Builtin::Light, "exposure-automatic-light");
    }

    /// Exposure as the app draws it once its pick is built, its Picker target and
    /// Lightness live as the app keeps them (decision 88).
    struct Targeted {
        settings: ToolSettings,
        picked: Picked,
        changes: Vec<Change>,
        /// The picker on, the photo open, and whether a reference's photo is gone.
        on: Option<PickerRef>,
        photo: Option<std::path::PathBuf>,
        gone: bool,
        /// Exposure as the session reports it, where it is not its default.
        exposure: Option<f32>,
        /// A colour raw with a camera bias and a highlight-preserving mode, whose camera
        /// rows show, and Mode set to Manual as chosen.
        camera: bool,
        mode_set: bool,
    }

    /// The section in the dark theme with the tool settings at their defaults.
    fn targeted(picked: Picked) -> Harness<'static, Targeted> {
        targeted_with(picked, Builtin::Dark, ToolSettings::default())
    }

    /// The section in `which` with these tool settings, on its panel's colour.
    fn targeted_with(
        picked: Picked,
        which: Builtin,
        settings: ToolSettings,
    ) -> Harness<'static, Targeted> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 600.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui_state(
                |ui, targeted: &mut Targeted| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                    let gone = |_: &str| targeted.gone;
                    let pickers = Pickers {
                        drawn: PICKING,
                        on: targeted.on,
                        enabled: true,
                        why: None,
                        settings: &targeted.settings,
                        picked: &targeted.picked,
                        photo: targeted.photo.as_deref(),
                        gone: &gone,
                    };
                    let groups = |_: &str| true;
                    let mut state = untouched(&Exposure);
                    if targeted.camera {
                        on_photo(&Exposure, &mut state, &camera_photo());
                    }
                    if targeted.mode_set {
                        set(&mut state, "mode", ControlValue::Choice(0), None);
                    }
                    if let Some(exposure) = targeted.exposure {
                        set(&mut state, "exposure", ControlValue::Number(exposure), None);
                    }
                    let changes = section(
                        ui,
                        &Exposure,
                        &state,
                        Head {
                            open: true,
                            ..Head::default()
                        },
                        &Panels::default(),
                        &groups,
                        true,
                        None,
                        &pickers,
                    );
                    // Kept as the app keeps them.
                    for change in changes {
                        match change {
                            Change::Setting {
                                tool,
                                setting,
                                ref value,
                                ..
                            } => targeted.settings.set(tool, setting, value.clone()),
                            Change::ClearReference {
                                tool,
                                lightness,
                                photo,
                            } => {
                                (targeted.settings).clear_reference(tool, (lightness, photo));
                            }
                            _ => {}
                        }
                        targeted.changes.push(change);
                    }
                },
                Targeted {
                    settings,
                    picked,
                    changes: Vec::new(),
                    on: None,
                    photo: None,
                    gone: false,
                    exposure: None,
                    camera: false,
                    mode_set: false,
                },
            );
        harness.run();
        harness
    }

    /// The reading of Exposure's picker whose pick showed an input of L 45.2 and gave a
    /// lightness of 61 back, as a picker that measures a setting does: the reference
    /// picker's, to come (decision 88).
    fn exposure_reading() -> apertrust_engine::session::PickerReading {
        use apertrust_tools::pick::{Patch, Shown, Stats};
        let stats = Stats {
            mean: [0.15; 4],
            min: [0.15; 4],
            max: [0.15; 4],
        };
        apertrust_engine::session::PickerReading {
            through: 3,
            picker: PickerRef::Tool {
                tool: "exposure",
                picker: "exposure",
            },
            region: apertrust_engine::session::Region::Area([0.2, 0.2, 0.4, 0.4]),
            tool: Some(apertrust_engine::session::ToolReading {
                input: Some(stats),
                output: None,
                picked: Some(Shown {
                    patch: Some(Patch {
                        srgb: [0.42; 3],
                        lch: [45.2, 0.0, 0.0],
                    }),
                    settings: vec![("lightness", ControlValue::Number(61.0))],
                    ..Shown::default()
                }),
            }),
            frame: None,
        }
    }

    /// Whether Exposure's quad, named by its action `tip`, shows it as its tooltip too with
    /// the pointer resting on it: the button's name and the tooltip both say it.
    fn quad_tip(harness: &mut Harness<'static, Targeted>, tip: &str) -> bool {
        harness
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        harness.get_by_role_and_label(Role::Button, tip).hover();
        harness.run();
        harness.query_all_by_label(tip).count() > 1
    }

    /// Picker target is live, Middle grey by default with Lightness hidden; Lightness chosen
    /// shows its slider, live too, a drag a change of the setting, not the edit's, done on
    /// its release; Reference is listed but cannot be chosen yet. The quad's tooltip says
    /// what a pick will do by the target (decision 88).
    #[test]
    fn exposure_s_picker_target_and_lightness_are_live_and_say_what_a_pick_will_do() {
        let mut harness = targeted(Picked::default());
        let target = harness.get_by_role_and_label(Role::ComboBox, "Picker target");
        assert!(!target.accesskit_node().is_disabled());
        assert_eq!(
            target.accesskit_node().value().as_deref(),
            Some("Middle grey")
        );
        assert!(harness.query_by_label("Target lightness").is_none());
        assert!(quad_tip(&mut harness, "Make an area middle grey"));
        // A toggle button named by its action, pressed while the picker is on.
        let toggled = |harness: &Harness<'_, Targeted>| {
            harness
                .get_by_role_and_label(Role::Button, "Make an area middle grey")
                .accesskit_node()
                .toggled()
        };
        assert_eq!(toggled(&harness), Some(egui::accesskit::Toggled::False));
        harness.state_mut().on = Some(PickerRef::Tool {
            tool: "exposure",
            picker: "exposure",
        });
        harness.run();
        assert_eq!(toggled(&harness), Some(egui::accesskit::Toggled::True));
        harness.state_mut().on = None;
        harness.run();
        // Its list: each option with its own tooltip, each to be chosen.
        harness
            .get_by_role_and_label(Role::ComboBox, "Picker target")
            .click();
        harness.run();
        let reference = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Reference");
        assert!(!reference.accesskit_node().is_disabled());
        harness.get_by_label("Reference").hover();
        harness.run();
        harness.get_by_label("The picked area matches the one measured on another photo");
        harness.get_by_label("Lightness").hover();
        harness.run();
        harness.get_by_label("The picked area becomes the brightness you set");
        harness.get_by_label("Lightness").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Setting {
                tool: "exposure",
                setting: "target",
                value: ControlValue::Choice(1),
                done: true,
            })
        );
        harness.run();
        // Lightness under it, at darktable's 50 % in the percent the schema gives it.
        let lightness = harness.get_by_role_and_label(Role::Slider, "Target lightness");
        assert_eq!(lightness.accesskit_node().value().as_deref(), Some("50.0%"));
        let top = |harness: &Harness<'_, Targeted>, role, name| {
            harness.get_by_role_and_label(role, name).rect().top()
        };
        assert!(
            top(&harness, Role::ComboBox, "Picker target")
                < top(&harness, Role::Slider, "Target lightness")
        );
        harness.state_mut().changes.clear();
        let at = along(
            harness
                .get_by_role_and_label(Role::Slider, "Target lightness")
                .rect(),
            0.25,
        );
        harness.drag_at(at);
        harness.run();
        harness.drop_at(at);
        harness.run();
        let setting = |value, done| Change::Setting {
            tool: "exposure",
            setting: "lightness",
            value: ControlValue::Number(value),
            done,
        };
        let changes = &harness.state().changes;
        assert_eq!(changes.last(), Some(&setting(25.0, true)), "{changes:?}");
        assert!(
            changes
                .iter()
                .rev()
                .skip(1)
                .all(|c| *c == setting(25.0, false))
        );
        assert_eq!(
            harness.state().settings.get("exposure", "lightness"),
            Some(&ControlValue::Number(25.0))
        );
        assert!(quad_tip(&mut harness, "Make an area L 25 %"));
        // Its track is darktable's ramp of greys, black at its start.
        let track = harness
            .get_by_role_and_label(Role::Slider, "Target lightness")
            .rect();
        let shapes = harness.output().shapes.clone();
        let black = shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) => mesh
                .vertices
                .iter()
                .any(|v| track.contains(v.pos) && v.color.r() < 8 && v.color.r() == v.color.b()),
            _ => false,
        });
        assert!(black, "no ramp of greys on Lightness's track");
    }

    /// A pick that gives a setting back hands it to the app once: the readings after it
    /// carry the same pick. Its input patch stays as the picker turns off.
    #[test]
    fn a_setting_a_pick_gives_back_is_handed_to_the_app_once() {
        let mut picked = Picked::default();
        let reading = exposure_reading();
        assert_eq!(
            picked.apply(Some(&reading)),
            Some(("exposure", vec![("lightness", ControlValue::Number(61.0))]))
        );
        assert_eq!(picked.apply(Some(&reading)), None);
        picked.apply(None);
        assert_eq!(picked.patch("exposure").map(|p| p.lch[0]), Some(45.2));
    }

    /// A pick measured before a Clear Reference, still on its way, hands no reference back
    /// over the clear, so the cleared reference is not restored; one measured after it
    /// does (decision 88).
    #[test]
    fn a_pick_measured_before_a_clear_hands_no_setting_back() {
        let mut picked = Picked::default();
        picked.settings_cleared("exposure", 5);
        let before = apertrust_engine::session::PickerReading {
            through: 4,
            ..exposure_reading()
        };
        assert_eq!(picked.apply(Some(&before)), None);
        // Its patch still shows, as the clear takes no patch away.
        assert_eq!(picked.patch("exposure").map(|p| p.lch[0]), Some(45.2));
        let mut after = exposure_reading();
        after.through = 5;
        let shown = after.tool.as_mut().unwrap().picked.as_mut().unwrap();
        shown.settings = vec![("lightness", ControlValue::Number(70.0))];
        assert_eq!(
            picked.apply(Some(&after)),
            Some(("exposure", vec![("lightness", ControlValue::Number(70.0))]))
        );
        // The same values measured after the clear, as another area may give: the refused
        // pick was not taken as seen, so they are handed back.
        let mut picked = Picked::default();
        picked.settings_cleared("exposure", 5);
        assert_eq!(picked.apply(Some(&before)), None);
        let same = apertrust_engine::session::PickerReading {
            through: 5,
            ..exposure_reading()
        };
        assert_eq!(
            picked.apply(Some(&same)),
            Some(("exposure", vec![("lightness", ControlValue::Number(61.0))]))
        );
    }

    /// A tool setting `--conf` gives is drawn disabled, keeping its value for the run; the
    /// others stay live, and its rows follow the value it gives.
    #[test]
    fn a_tool_setting_conf_gives_is_drawn_disabled() {
        let given = vec!["tools.exposure.target=lightness".to_owned()];
        let overrides = crate::settings::overrides(&given).expect("read");
        let folder = crate::settings::testing::Folder::new("target-conf");
        let (_, settings) = crate::settings::Keeper::open_with(folder.storage(), &overrides);
        let harness = targeted_with(Picked::default(), Builtin::Dark, settings.tool_settings);
        let target = harness.get_by_role_and_label(Role::ComboBox, "Picker target");
        assert!(target.accesskit_node().is_disabled());
        assert_eq!(
            target.accesskit_node().value().as_deref(),
            Some("Lightness")
        );
        let lightness = harness.get_by_role_and_label(Role::Slider, "Target lightness");
        assert!(!lightness.accesskit_node().is_disabled());
    }

    /// The input patch goes as darktable's `gui_update` zeroes it (`exposure.c:707-710`):
    /// at undo, redo, a move through history, the edit's reset, and Compress, Truncate and
    /// Discard History, and at the tool's own reset, never another's, nor an edit of a
    /// control.
    #[test]
    fn the_input_patch_goes_as_the_tool_s_values_change_from_outside_a_pick() {
        use apertrust_engine::session::{Command, StepId};
        let picked = || {
            let mut picked = Picked::default();
            picked.apply(Some(&exposure_reading()));
            picked
        };
        let tool = |tool: &str| Command::ResetTool {
            tool: tool.to_owned(),
        };
        let set = Command::SetControl {
            tool: "exposure".to_owned(),
            control: "exposure".to_owned(),
            value: ControlValue::Number(1.0),
            done: true,
        };
        for (command, kept) in [
            (Command::Undo, false),
            (Command::Redo, false),
            (
                Command::GoTo {
                    step: Some(StepId(1)),
                },
                false,
            ),
            (Command::Reset, false),
            (Command::Compress { up_to: None }, false),
            (Command::Truncate { after: None }, false),
            (Command::DiscardHistory, false),
            (tool("exposure"), false),
            (tool("agx"), true),
            (set, true),
        ] {
            let mut picked = picked();
            picked.sent(5, Picked::clears(&command));
            assert_eq!(picked.patch("exposure").is_some(), kept, "{command:?}");
        }
    }

    /// The patch an undo took away stays away: the readings after it carry the same pick
    /// again, and a pick measured before the undo, still on its way as it was sent, shows
    /// none; the next pick measured after it does.
    #[test]
    fn a_patch_taken_away_is_not_brought_back_by_a_pick_from_before() {
        use apertrust_engine::session::Command;
        let mut picked = Picked::default();
        let first = exposure_reading();
        picked.apply(Some(&first));
        picked.sent(5, Picked::clears(&Command::Undo));
        // The same pick, carried by every reading after it.
        let again = apertrust_engine::session::PickerReading {
            through: 6,
            ..first.clone()
        };
        picked.apply(Some(&again));
        assert!(picked.patch("exposure").is_none());
        // Another pick, measured before the undo.
        let picked_as = |through: u64, lightness: f32| {
            let mut reading = first.clone();
            reading.through = through;
            let tool = reading.tool.as_mut().unwrap();
            let shown = tool.picked.as_mut().unwrap();
            shown.patch.as_mut().unwrap().lch[0] = lightness;
            reading
        };
        picked.apply(Some(&picked_as(4, 30.0)));
        assert!(picked.patch("exposure").is_none());
        // One measured after it.
        picked.apply(Some(&picked_as(7, 60.0)));
        assert_eq!(picked.patch("exposure").map(|p| p.lch[0]), Some(60.0));
        // A tool's reset takes only its own.
        picked.sent(
            8,
            Picked::clears(&Command::ResetTool {
                tool: "agx".to_owned(),
            }),
        );
        picked.apply(Some(&picked_as(7, 61.0)));
        assert_eq!(picked.patch("exposure").map(|p| p.lch[0]), Some(61.0));
        // A preset applied, inside its step, takes its own tool's away.
        let preset = crate::presets::apply("exposure", "Bright", Default::default());
        assert_eq!(Picked::clears(&preset), Some(Some("exposure".to_owned())));
        picked.sent(9, Picked::clears(&preset));
        assert!(picked.patch("exposure").is_none());
        // A step changing two tools takes every tool's, not the first's alone.
        let reset = |tool: &str| Command::ResetTool {
            tool: tool.to_owned(),
        };
        let step = |commands| Command::Step {
            source: apertrust_engine::session::StepSource::You,
            note: None,
            commands,
        };
        let two = step(vec![reset("agx"), reset("exposure"), reset("agx")]);
        assert_eq!(Picked::clears(&two), Some(None));
        let one = step(vec![reset("agx"), reset("agx")]);
        assert_eq!(Picked::clears(&one), Some(Some("agx".to_owned())));
        assert_eq!(Picked::clears(&step(Vec::new())), None);
    }

    /// `Settings` completes a change on the edit, whose write Quit waits for, whatever picker
    /// is on or none; a tool picker's completing commands do, the Colour picker's never.
    #[test]
    fn settings_and_a_tool_pick_complete_a_change_on_the_edit() {
        use crate::pickers::completes_edit;
        use apertrust_engine::session::PickerCommand;
        let settings = PickerCommand::Settings {
            tool: "exposure",
            settings: Vec::new(),
        };
        let exposure = PickerRef::Tool {
            tool: "exposure",
            picker: "exposure",
        };
        for active in [None, Some(exposure), Some(PickerRef::ColourPicker)] {
            assert!(completes_edit(&settings, active), "{active:?}");
        }
        assert!(completes_edit(&PickerCommand::Reset, Some(exposure)));
        assert!(!completes_edit(
            &PickerCommand::Reset,
            Some(PickerRef::ColourPicker)
        ));
        assert!(!completes_edit(&PickerCommand::Reset, None));
        assert!(!completes_edit(&PickerCommand::Off, Some(exposure)));
    }

    /// The themes the board's B states are snapshotted in, with their names' endings.
    const THEMES: [(Builtin, &str); 2] = [(Builtin::Dark, "dark"), (Builtin::Light, "light")];

    /// Exposure's picker, which the board's B states turn on.
    const EXPOSURE_PICKER: PickerRef = PickerRef::Tool {
        tool: "exposure",
        picker: "exposure",
    };

    /// Exposure's manual page on a colour raw with a camera bias and a held-back exposure,
    /// its picker drawn and Picker target holding `target`, in each theme (the board's B1
    /// to B4): Middle grey alone, or Lightness with its slider under it at `lightness`,
    /// on darktable's grey ramp. `on` turns the picker on; `last` shows a Last pick.
    fn photograph_target(target: u32, lightness: f32, on: bool, last: bool, name: &str) {
        let mut settings = ToolSettings::default();
        settings.set("exposure", "target", ControlValue::Choice(target));
        settings.set("exposure", "lightness", ControlValue::Number(lightness));
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let picked = if last {
                last_picked(false, false, false)
            } else {
                Picked::default()
            };
            let mut harness = targeted_with(picked, which, settings.clone());
            let state = harness.state_mut();
            state.camera = true;
            state.exposure = last.then_some(0.55);
            state.on = on.then_some(EXPOSURE_PICKER);
            harness.run();
            harness.snapshot(format!("{name}-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    /// Colour calibration's CAT tab as a section draws it, its pickers drawn, the area
    /// colour mapping open, with the tool settings and picks the app keeps.
    struct Calibrated {
        settings: ToolSettings,
        picked: Picked,
        changes: Vec<Change>,
        /// Its adaptation, where it is not CAT16.
        adaptation: Option<u32>,
    }

    /// The CAT tab in `which` with these tool settings and picks.
    fn calibrated(
        picked: Picked,
        which: Builtin,
        settings: ToolSettings,
    ) -> Harness<'static, Calibrated> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 900.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui_state(
                |ui, calibrated: &mut Calibrated| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                    let pickers = Pickers {
                        drawn: PICKING,
                        settings: &calibrated.settings,
                        picked: &calibrated.picked,
                        ..Pickers::NONE
                    };
                    let mut state = untouched(&ColourCalibration);
                    if let Some(adaptation) = calibrated.adaptation {
                        set(
                            &mut state,
                            "adaptation",
                            ControlValue::Choice(adaptation),
                            None,
                        );
                    }
                    report_readouts(&ColourCalibration, &mut state);
                    let groups = |_: &str| true;
                    let changes = section(
                        ui,
                        &ColourCalibration,
                        &state,
                        Head {
                            open: true,
                            ..Head::default()
                        },
                        &Panels::default(),
                        &groups,
                        true,
                        None,
                        &pickers,
                    );
                    // Kept as the app keeps them.
                    for change in changes {
                        if let Change::Setting {
                            tool,
                            setting,
                            ref value,
                            ..
                        } = change
                        {
                            calibrated.settings.set(tool, setting, value.clone());
                        }
                        calibrated.changes.push(change);
                    }
                },
                Calibrated {
                    settings,
                    picked,
                    changes: Vec::new(),
                    adaptation: None,
                },
            );
        harness.run();
        harness
    }

    /// A reading of Colour calibration's picker whose pick showed a brown input patch and,
    /// in measure mode, handed back the target it renders at.
    fn illuminant_reading(measured: bool) -> apertrust_engine::session::PickerReading {
        use apertrust_tools::pick::{Patch, Shown, Stats};
        let stats = Stats {
            mean: [0.2, 0.15, 0.1, 1.0],
            min: [0.2, 0.15, 0.1, 1.0],
            max: [0.2, 0.15, 0.1, 1.0],
        };
        let settings = if measured {
            vec![
                ("target_lightness", ControlValue::Number(45.2)),
                ("target_chroma", ControlValue::Number(18.4)),
                ("target_hue", ControlValue::Number(62.0)),
            ]
        } else {
            Vec::new()
        };
        apertrust_engine::session::PickerReading {
            through: 3,
            picker: PickerRef::Tool {
                tool: "channelmixerrgb",
                picker: "illuminant",
            },
            region: apertrust_engine::session::Region::Area([0.2, 0.2, 0.4, 0.4]),
            tool: Some(apertrust_engine::session::ToolReading {
                input: Some(stats),
                output: None,
                picked: Some(Shown {
                    patch: Some(Patch {
                        srgb: [0.52, 0.41, 0.32],
                        lch: [45.2, 18.4, 62.0],
                    }),
                    settings,
                    ..Shown::default()
                }),
            }),
            frame: None,
        }
    }

    /// The CAT tab's readout row, darktable's: the CCT the values give, with its tooltip,
    /// the light's colour, and the picker at the row's end; hidden with the adaptation
    /// none, as darktable's
    /// `_update_illuminants` hides all three.
    #[test]
    fn colour_calibration_s_readout_row_shows_the_cct_the_light_s_colour_and_its_picker() {
        let mut harness = calibrated(Picked::default(), Builtin::Dark, ToolSettings::default());
        let state = untouched(&ColourCalibration);
        let readout = state.readouts[0].readout.unwrap();
        assert_eq!(readout.text, "daylight");
        let words = translator(&harness.ctx);
        let mut args = Args::new();
        let value = f64::from(readout.value.unwrap());
        args.set("value", words.format_number(value, 0, false));
        let text = words.text_with("tool-channelmixerrgb-readout-cct-daylight", Some(&args));
        let cct = harness.get_by_label(&text);
        let swatch = "This is the colour of the scene illuminant before chromatic adaptation. \
                      This colour will be turned into pure white by the adaptation.";
        let colour = harness.get_by_role_and_label(Role::Image, swatch);
        let picker = harness.get_by_role_and_label(Role::Button, "Illuminant from an image area");
        // One line: the text, the swatch filling the row, the picker at its end, with the
        // illuminant under them.
        let (cct, colour, picker) = (cct.rect(), colour.rect(), picker.rect());
        assert!(cct.right() <= colour.left() && colour.right() <= picker.left());
        assert!(colour.width() >= 2.0 * colour.height(), "{colour:?}");
        let illuminant = harness.get_by_role_and_label(Role::ComboBox, "Illuminant");
        assert!(picker.bottom() <= illuminant.rect().top());
        // Its tooltip, darktable's for daylight.
        harness
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        harness.get_by_label(&text).hover();
        harness.run();
        harness.get_by_label(&words.text("tool-channelmixerrgb-readout-cct-daylight-promise"));
        // With no adaptation, none of it.
        harness.state_mut().adaptation = Some(4);
        harness.run();
        assert!(harness.query_by_label(&text).is_none());
        assert!(
            harness
                .query_by_role_and_label(Role::Image, swatch)
                .is_none()
        );
        let picker = "Illuminant from an image area";
        assert!(
            harness
                .query_by_role_and_label(Role::Button, picker)
                .is_none()
        );
    }

    /// The area colour mapping is live: its settings kept and sent as a gesture ends, the
    /// input patch with its L, h and c, "N/A" before a pick; a target measured is kept as the
    /// user's own, once; and its title reads "(active)" as darktable's does, in correction
    /// mode with the target off L 50 or chroma 0.
    #[test]
    fn colour_calibration_s_area_colour_mapping_is_live() {
        let mut harness = calibrated(Picked::default(), Builtin::Dark, ToolSettings::default());
        harness.get_by_role_and_label(Role::Button, "Area colour mapping");
        let mode = harness.get_by_role_and_label(Role::ComboBox, "Area mode");
        assert!(!mode.accesskit_node().is_disabled());
        for line in ["L: N/A", "h: N/A", "c: N/A"] {
            assert!(
                harness.query_all_by_label_contains(line).count() > 0,
                "{line}"
            );
        }
        // Measure chosen: sent as the tool's setting, kept.
        mode.click();
        harness.run();
        harness.get_by_label("Measure").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::Setting {
                tool: "channelmixerrgb",
                setting: "area_mode",
                value: ControlValue::Choice(1),
                done: true,
            })
        );
        // A target measured, handed to the app once, as a pick's settings are.
        let mut picked = Picked::default();
        let reading = illuminant_reading(true);
        let handed = picked.apply(Some(&reading));
        assert_eq!(
            handed,
            Some((
                "channelmixerrgb",
                vec![
                    ("target_lightness", ControlValue::Number(45.2)),
                    ("target_chroma", ControlValue::Number(18.4)),
                    ("target_hue", ControlValue::Number(62.0)),
                ]
            ))
        );
        assert_eq!(picked.apply(Some(&reading)), None);
        // The title: plain at middle grey, active toward a colour in correction mode only.
        let title = |harness: &Harness<'_, Calibrated>| {
            harness
                .query_by_role_and_label(Role::Button, "Area colour mapping (active)")
                .is_some()
        };
        let mut settings = ToolSettings::default();
        let harness = calibrated(Picked::default(), Builtin::Dark, settings.clone());
        assert!(!title(&harness));
        settings.set(
            "channelmixerrgb",
            "target_chroma",
            ControlValue::Number(18.4),
        );
        let harness = calibrated(Picked::default(), Builtin::Dark, settings.clone());
        assert!(title(&harness));
        settings.set("channelmixerrgb", "area_mode", ControlValue::Choice(1));
        let harness = calibrated(Picked::default(), Builtin::Dark, settings);
        assert!(!title(&harness));
        // After a pick, the input's L, h and c.
        let mut picked = Picked::default();
        picked.apply(Some(&illuminant_reading(false)));
        let harness = calibrated(picked, Builtin::Dark, ToolSettings::default());
        for line in ["L: 45.2%", "h: 62.0 °", "c: 18.4"] {
            assert!(
                harness.query_all_by_label_contains(line).count() > 0,
                "{line}"
            );
        }
    }

    /// The CAT tab after a pick toward (60, 20, 40°), its title active, in each theme.
    #[test]
    fn colour_calibration_s_cat_tab_after_a_pick() {
        let mut settings = ToolSettings::default();
        settings.set(
            "channelmixerrgb",
            "target_lightness",
            ControlValue::Number(60.0),
        );
        settings.set(
            "channelmixerrgb",
            "target_chroma",
            ControlValue::Number(20.0),
        );
        settings.set("channelmixerrgb", "target_hue", ControlValue::Number(40.0));
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut picked = Picked::default();
            picked.apply(Some(&illuminant_reading(false)));
            let mut harness = calibrated(picked, which, settings.clone());
            harness.run();
            harness.snapshot(format!("colour-calibration-mapping-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    /// Colour balance RGB as a section draws it, its pickers drawn, on its tab `tab`, the
    /// picker `on` on.
    struct Balanced {
        tab: usize,
        on: Option<PickerRef>,
        changes: Vec<Change>,
    }

    /// Colour balance RGB's section in `which` on its tab `tab`.
    fn balanced(which: Builtin, tab: usize) -> Harness<'static, Balanced> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 670.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui_state(
                |ui, balanced: &mut Balanced| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                    let pickers = Pickers {
                        drawn: PICKING,
                        on: balanced.on,
                        ..Pickers::NONE
                    };
                    let mut panels = Panels::default();
                    panels.apply(&Change::Tab {
                        tool: "colorbalancergb",
                        tab: balanced.tab,
                    });
                    let changes = section(
                        ui,
                        &ColourBalanceRgb,
                        &untouched(&ColourBalanceRgb),
                        Head {
                            open: true,
                            ..Head::default()
                        },
                        &panels,
                        &|_: &str| GROUP_OPEN,
                        true,
                        None,
                        &pickers,
                    );
                    balanced.changes.extend(changes);
                },
                Balanced {
                    tab,
                    on: None,
                    changes: Vec::new(),
                },
            );
        harness.run();
        harness
    }

    /// Colour balance RGB's six quads, darktable's: the four hues' on the 4 ways tab and
    /// the two fulcrums' on the Masks tab, each named by its picker, its tooltip darktable's
    /// words alone, as an area picker's has no line on choosing an area; a press sends `On`
    /// with the keys held, Ctrl (Cmd on the Mac) for a hue's own colour.
    #[test]
    fn colour_balance_rgb_draws_its_six_pickers_and_a_press_sends_on_with_the_keys() {
        let quads = |harness: &Harness<'_, Balanced>| {
            harness
                .query_all_by_role(Role::Button)
                .filter_map(|node| node.accesskit_node().label())
                .filter(|label| label.ends_with(": pick from the photo"))
                .collect::<Vec<_>>()
        };
        let mut harness = balanced(Builtin::Dark, 1);
        assert_eq!(
            quads(&harness),
            [
                "Global offset hue: pick from the photo",
                "Shadows lift hue: pick from the photo",
                "Highlights gain hue: pick from the photo",
                "Power hue: pick from the photo",
            ]
        );
        let quad =
            harness.get_by_role_and_label(Role::Button, "Global offset hue: pick from the photo");
        let tip = quad.accesskit_node().description().unwrap_or_default();
        assert_eq!(
            tip,
            "Picks the colour opposite the selected area's. Ctrl-click, Cmd-click on the Mac, \
             picks the area's own."
        );
        quad.click();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Power hue: pick from the photo")
            .click_modifiers(Modifiers::COMMAND);
        harness.run();
        let pressed = |picker, command| Change::Picker {
            tool: "colorbalancergb",
            picker,
            action: PickerAction::On { area: command },
            keys: PickKeys {
                command,
                shift: false,
            },
        };
        assert_eq!(
            harness.state().changes,
            [pressed("global_hue", false), pressed("midtones_hue", true)]
        );
        let harness = balanced(Builtin::Dark, 2);
        assert_eq!(
            quads(&harness),
            [
                "White fulcrum: pick from the photo",
                "Contrast grey fulcrum: pick from the photo",
            ]
        );
        let quad =
            harness.get_by_role_and_label(Role::Button, "White fulcrum: pick from the photo");
        let tip = quad.accesskit_node().description().unwrap_or_default();
        assert_eq!(tip, "Pick colour from the photo");
    }

    /// Colour balance RGB's 4 ways tab with its four quads, Global offset hue's on, in
    /// each theme.
    #[test]
    fn colour_balance_rgb_s_pickers() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = balanced(which, 1);
            harness.state_mut().on = Some(PickerRef::Tool {
                tool: "colorbalancergb",
                picker: "global_hue",
            });
            harness.run();
            harness.snapshot(format!("colour-balance-rgb-pickers-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    /// AgX as a section draws it, its pickers drawn, on its Settings tab, the picker `on`
    /// on.
    struct Toned {
        on: Option<PickerRef>,
        changes: Vec<Change>,
        /// AgX's values as the panel shows them.
        state: ToolState,
    }

    /// AgX's section in `which`, open on Settings.
    fn toned(which: Builtin) -> Harness<'static, Toned> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 620.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui_state(
                |ui, toned: &mut Toned| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                    let pickers = Pickers {
                        drawn: PICKING,
                        on: toned.on,
                        ..Pickers::NONE
                    };
                    let changes = section(
                        ui,
                        &Agx,
                        &toned.state,
                        Head {
                            open: true,
                            ..Head::default()
                        },
                        &Panels::default(),
                        &|_: &str| GROUP_OPEN,
                        true,
                        None,
                        &pickers,
                    );
                    toned.changes.extend(changes);
                },
                Toned {
                    on: None,
                    changes: Vec::new(),
                    state: untouched(&Agx),
                },
            );
        harness.run();
        harness
    }

    /// AgX's five pickers, darktable's: the quads on white and black relative exposure, on
    /// the pivot and on its target output, and Auto tune levels' button at the end of its
    /// labelled row, each named by its picker, its tooltip darktable's words; a press sends
    /// `On` with the keys held. Dynamic range scaling, the pickers' margin, is live, and
    /// Read exposure still drawn disabled.
    #[test]
    fn agx_draws_its_five_pickers_and_a_press_sends_on_with_the_keys() {
        let mut harness = toned(Builtin::Dark);
        let pickers: Vec<String> = harness
            .query_all_by_role(Role::Button)
            .filter_map(|node| node.accesskit_node().label())
            .filter(|label| label.ends_with(": pick from the photo"))
            .collect();
        assert_eq!(
            pickers,
            [
                "White relative exposure: pick from the photo",
                "Black relative exposure: pick from the photo",
                "Auto tune levels: pick from the photo",
                "Pivot relative exposure: pick from the photo",
                "Pivot target output: pick from the photo",
            ]
        );
        let tip = |harness: &Harness<'_, Toned>, label: &str| {
            let node = harness.get_by_role_and_label(Role::Button, label);
            node.accesskit_node().description().unwrap_or_default()
        };
        assert_eq!(
            tip(&harness, "White relative exposure: pick from the photo"),
            "Picks the white point."
        );
        assert_eq!(
            tip(&harness, "Auto tune levels: pick from the photo"),
            "Sets black and white relative exposure using the selected area."
        );
        assert!(harness.query_by_label("Auto tune levels").is_some());
        let slider = harness.get_by_role_and_label(Role::Slider, "Dynamic range scaling");
        assert!(!slider.accesskit_node().is_disabled());
        let read = harness.get_by_role_and_label(Role::Button, "Read exposure");
        assert!(!read.accesskit_node().is_disabled());

        harness
            .get_by_role_and_label(Role::Button, "White relative exposure: pick from the photo")
            .click();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Auto tune levels: pick from the photo")
            .click_modifiers(Modifiers::COMMAND);
        harness.run();
        let pressed = |picker, command| Change::Picker {
            tool: "agx",
            picker,
            action: PickerAction::On { area: command },
            keys: PickKeys {
                command,
                shift: false,
            },
        };
        assert_eq!(
            harness.state().changes,
            [pressed("white", false), pressed("range", true)]
        );
    }

    /// AgX's Primaries tab in `which`, its changes shown as the app's mirror shows them.
    fn primaried(which: Builtin) -> Harness<'static, Toned> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(400.0, 620.0))
            .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
            .build_ui_state(
                |ui, toned: &mut Toned| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter()
                        .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                    let mut panels = Panels::default();
                    panels.apply(&Change::Tab {
                        tool: "agx",
                        tab: 1,
                    });
                    let changes = section(
                        ui,
                        &Agx,
                        &untouched(&Agx),
                        Head {
                            open: true,
                            ..Head::default()
                        },
                        &panels,
                        &|_: &str| GROUP_OPEN,
                        true,
                        None,
                        &Pickers::NONE,
                    );
                    toned.changes.extend(changes);
                },
                Toned {
                    on: None,
                    changes: Vec::new(),
                    state: untouched(&Agx),
                },
            );
        harness.run();
        harness
    }

    /// Reset primaries is a line: its label, then a menu button that opens a menu of
    /// Blender-like, Smooth and Unmodified, reported expanded while open; choosing Smooth
    /// runs its action on the values shown, sent as one gesture, one history step.
    #[test]
    fn reset_primaries_opens_its_menu_and_smooth_sends_its_writes() {
        let mut harness = primaried(Builtin::Dark);
        // Its label, then its button, named by the same words.
        assert_eq!(harness.query_all_by_label("Reset primaries").count(), 2);
        let button = harness.get_by_role_and_label(Role::Button, "Reset primaries");
        assert_eq!(button.accesskit_node().data().is_expanded(), Some(false));
        button.click();
        harness.run();
        let menu = harness.get_by_role_and_label(Role::Menu, "Reset primaries");
        let items: Vec<String> = menu
            .query_all_by_role(Role::MenuItem)
            .filter_map(|node| node.accesskit_node().label())
            .collect();
        assert_eq!(items, ["Blender-like", "Smooth", "Unmodified"]);
        harness
            .get_by_role_and_label(Role::MenuItem, "Smooth")
            .click();
        harness.run();
        let Some(Change::Action {
            tool: "agx",
            action: "primaries_smooth",
            values,
        }) = harness.state().changes.last().cloned()
        else {
            panic!("{:?}", harness.state().changes);
        };
        let writes = action_writes(
            &[&Agx],
            "agx",
            "primaries_smooth",
            &values,
            &PublishedValues::new(),
            &testing::image(),
            false,
        );
        let image = testing::image();
        let read = PublishedValues::new();
        let smooth = Agx
            .action("primaries_smooth", &values, &read, &image)
            .unwrap();
        assert_eq!(writes.len(), smooth.iter().count());
        assert!(writes[..writes.len() - 1].iter().all(|(_, _, done)| !done));
        assert!(writes.last().unwrap().2);
        for (control, value, _) in &writes {
            assert_eq!(smooth.get(control), Some(value), "{control}");
        }
    }

    /// AgX's Primaries tab, Reset primaries' menu button and Set from above at Reverse
    /// all's end, in each theme, and the menu open in the dark one.
    #[test]
    fn agx_s_primaries() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = primaried(which);
            harness.snapshot(format!("agx-primaries-{theme}"));
            results.extend_harness(&mut harness);
        }
        let mut harness = primaried(Builtin::Dark);
        harness
            .get_by_role_and_label(Role::Button, "Reset primaries")
            .click();
        harness.run();
        harness.snapshot("agx-primaries-menu-dark");
        results.extend_harness(&mut harness);
    }

    /// Read exposure reads Exposure's effective exposure as the panel shows Exposure, and
    /// sets the range from it as darktable does: with Exposure at +1 EV and Compensate
    /// camera exposure on, on a photo whose camera biased it +0.7 EV, from 0.3 EV, one
    /// gesture. A press on its button sends its action.
    #[test]
    fn read_exposure_sets_the_range_from_exposure_s_effective_exposure() {
        let mut image = testing::image();
        let camera = image.camera.as_mut().unwrap();
        camera.exposure_bias = 0.7;
        camera.highlight_preservation = 0.0;
        let mut exposure = untouched(&Exposure);
        set(&mut exposure, "exposure", ControlValue::Number(1.0), None);
        set(&mut exposure, "camera_bias", ControlValue::Bool(true), None);
        let states = [untouched(&Agx), exposure];
        let tools: &[&dyn ProcessingTool] = &[&Agx, &Exposure];
        let read = published(tools, &states, "agx", "read_exposure", &image);
        let fact = apertrust_tools::Published {
            tool: "exposure",
            id: "effective_exposure",
        };
        let e = read.get(&fact).unwrap();
        assert!((e - 0.3).abs() < 1e-6, "{e}");
        let values = values(&states[0]);
        let writes = action_writes(tools, "agx", "read_exposure", &values, &read, &image, false);
        let scaling = 1.0 + values.number("dynamic_range_scaling") / 100.0;
        let black = (-8.0 + 0.5 * e) * scaling;
        let white = ((4.0 + 0.8 * f64::from(e)) * f64::from(scaling)) as f32;
        assert_eq!(
            writes,
            [
                (
                    "range_black_relative_ev",
                    ControlValue::Number(black),
                    false
                ),
                ("range_white_relative_ev", ControlValue::Number(white), true),
            ]
        );
        // Only what an action declares it reads: nothing for Set from above.
        let none = published(tools, &states, "agx", "set_from_above", &image);
        assert_eq!(none, PublishedValues::new());

        let mut harness = toned(Builtin::Dark);
        harness
            .get_by_role_and_label(Role::Button, "Read exposure")
            .click();
        harness.run();
        let pressed = harness.state().changes.last();
        assert!(
            matches!(
                pressed,
                Some(Change::Action {
                    tool: "agx",
                    action: "read_exposure",
                    ..
                })
            ),
            "{pressed:?}"
        );
    }

    /// On AgX's Settings tab, Tab meets Auto tune levels' picker and then Read exposure, the
    /// reading order of darktable's line.
    #[test]
    fn tab_meets_auto_tune_levels_then_read_exposure() {
        let mut harness = toned(Builtin::Dark);
        let mut met = Vec::new();
        for _ in 0..20 {
            harness.key_press(Key::Tab);
            harness.run();
            let focused = harness.get_by(|node| node.is_focused());
            met.push(focused.accesskit_node().label().unwrap_or_default());
        }
        let at = met
            .iter()
            .position(|label| label == "Auto tune levels: pick from the photo")
            .unwrap_or_else(|| panic!("{met:?}"));
        assert_eq!(met[at + 1], "Read exposure", "{met:?}");
    }

    /// Where a slider's value is drawn: the painted text `value`'s left edge.
    fn value_x(harness: &Harness<'_, impl Sized>, value: &str) -> f32 {
        fn find(shape: &egui::Shape, value: &str) -> Option<f32> {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, value)),
                egui::Shape::Text(text) if text.galley.text() == value => Some(text.pos.x),
                _ => None,
            }
        }
        (harness.output().shapes.iter())
            .find_map(|clipped| find(&clipped.shape, value))
            .unwrap_or_else(|| panic!("no {value}"))
    }

    /// Toe power's warning shows at once on a change pending that makes the curve fall
    /// back, its message darktable's, heard as the slider's description, and Shoulder
    /// power's value stays where it was drawn without it; the Settings tab with the toe's
    /// warning on, in each theme.
    #[test]
    fn toe_power_s_warning_follows_a_pending_change() {
        let mut harness = panel(&Agx, untouched(&Agx));
        harness.set_size(egui::vec2(400.0, 700.0));
        harness.run();
        let slider = harness.get_by_role_and_label(Role::Slider, "Toe power");
        let promise = slider.accesskit_node().description().unwrap_or_default();
        assert!(promise.starts_with("Contrast in shadows"), "{promise}");
        let (toe, shoulder) = (value_x(&harness, "1.50"), value_x(&harness, "3.30"));
        let state = &mut harness.state_mut().state;
        Pending::Control {
            tool: "agx",
            control: "range_black_relative_ev",
            value: ControlValue::Number(-2.0),
        }
        .show(&Agx, state, &testing::image());
        let warned = |id| state.warnings.iter().any(|w| w.id == id && w.on);
        assert!(warned("toe") && !warned("shoulder"));
        harness.run();
        let slider = harness.get_by_role_and_label(Role::Slider, "Toe power");
        let message = slider.accesskit_node().description().unwrap_or_default();
        assert!(
            message.starts_with("Toe power cannot be applied"),
            "{message}"
        );
        assert_eq!(value_x(&harness, "1.50"), toe);
        assert_eq!(value_x(&harness, "3.30"), shoulder);

        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = toned(which);
            Pending::Control {
                tool: "agx",
                control: "range_black_relative_ev",
                value: ControlValue::Number(-2.0),
            }
            .show(&Agx, &mut harness.state_mut().state, &testing::image());
            harness.run();
            harness.snapshot(format!("agx-settings-line-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    /// AgX's Settings tab with its pickers drawn, Auto tune levels' on, in each theme.
    #[test]
    fn agx_s_settings_with_its_pickers() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = toned(which);
            harness.state_mut().on = Some(PickerRef::Tool {
                tool: "agx",
                picker: "range",
            });
            harness.run();
            harness.snapshot(format!("agx-settings-pickers-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    #[test]
    fn exposure_s_picker_target_at_middle_grey() {
        photograph_target(0, 50.0, false, false, "exposure-middle-grey");
    }

    #[test]
    fn exposure_s_picker_target_at_lightness() {
        photograph_target(1, 92.0, false, false, "exposure-lightness");
    }

    /// The board's B4: Lightness chosen, a Last pick shown, the picker on.
    #[test]
    fn exposure_s_picker_on_at_lightness_after_a_pick() {
        photograph_target(1, 92.0, true, true, "exposure-lightness-last-pick");
    }

    /// The board's B1: Picker target's list open, its three options each with its tooltip.
    #[test]
    fn exposure_s_picker_target_list_open() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = targeted_with(Picked::default(), which, ToolSettings::default());
            harness.state_mut().camera = true;
            harness.run();
            harness
                .get_by_role_and_label(Role::ComboBox, "Picker target")
                .click();
            harness.run();
            harness.snapshot(format!("exposure-picker-target-open-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    /// The tool as the board's A1 and C2 draw it, on a colour raw with a camera bias and a
    /// held-back exposure and its picker drawn: at its defaults, and with Mode set to
    /// Manual.
    #[test]
    fn exposure_s_manual_page_with_its_picker_drawn() {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = targeted_with(Picked::default(), which, ToolSettings::default());
            harness.state_mut().camera = true;
            harness.run();
            harness.snapshot(format!("exposure-pickers-{theme}"));
            results.extend_harness(&mut harness);
            harness.state_mut().mode_set = true;
            harness.run();
            harness.snapshot(format!("exposure-manual-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    /// Exposure's pick that read L 41 % and lands at 50 %, +0.55 EV, with these warnings.
    fn last_picked(clipped: bool, short: bool, black: bool) -> Picked {
        use apertrust_tools::pick::{LastPick, Patch, PickWarnings, Short};
        let mut reading = exposure_reading();
        let shown = reading.tool.as_mut().unwrap().picked.as_mut().unwrap();
        shown.settings.clear();
        let (before, after) = if short { (2.0, 12.0) } else { (41.0, 50.0) };
        shown.last = Some(LastPick {
            before,
            after,
            exposure: if short { 17.2 } else { 0.55 },
            patches: [before, after].map(|l| Patch::of_lch([l, 0.0, 0.0])),
        });
        shown.warnings = PickWarnings {
            clipped,
            short: short.then_some(Short {
                exposure: 18.0,
                reached: 12.0,
                target: 50.0,
                black,
            }),
        };
        let mut picked = Picked::default();
        picked.apply(Some(&reading));
        picked
    }

    /// Last pick (decision 88): nothing before a pick; after one, the readout with its
    /// a patch before each value, "L 41 % → L 50 % · +0.55 EV", its tooltip its promise; each warning a
    /// line of its own above it, the clipped one first, its tooltip saying why, the short
    /// one's adding the black moved where the pick moved it.
    #[test]
    fn exposure_s_last_pick_shows_what_the_pick_did_and_its_warnings() {
        let harness = targeted(Picked::default());
        assert!(harness.query_by_label_contains("Last pick").is_none());
        let harness = targeted(last_picked(false, false, false));
        let row =
            harness.get_by_role_and_label(Role::Label, "Last pick L 41 % → L 50 % · +0.55 EV");
        assert!(
            harness
                .query_by_label("The area has clipped pixels")
                .is_none()
        );
        // Under Picker target, above the camera's rows and Black level correction.
        let target = harness.get_by_role_and_label(Role::ComboBox, "Picker target");
        let black = harness.get_by_role_and_label(Role::Slider, "Black level correction");
        assert!(target.rect().bottom() <= row.rect().top());
        assert!(row.rect().bottom() <= black.rect().top());
        let both = targeted(last_picked(true, true, true));
        let clipped = both.get_by_role_and_label(Role::Label, "The area has clipped pixels");
        let short = both.get_by_role_and_label(
            Role::Label,
            "Exposure at +18 EV: the area is L 12 %, not 50 %",
        );
        let last = both.get_by_role_and_label(Role::Label, "Last pick L 2 % → L 12 % · +17.20 EV");
        assert!(clipped.rect().top() < short.rect().top());
        assert!(short.rect().top() < last.rect().top());
        assert_eq!(
            short.accesskit_node().description().as_deref(),
            Some(
                "Exposure reached its end of +18 EV\nBlack level was moved to keep black below white"
            )
        );
        assert_eq!(
            clipped.accesskit_node().description().as_deref(),
            Some("Its lightness reads low: pick an area with no clipped highlights")
        );
        // Taken away as its patch is, by an undo.
        let mut picked = last_picked(false, false, false);
        picked.sent(9, Picked::clears(&apertrust_engine::session::Command::Undo));
        assert!(picked.last("exposure").is_none());
    }

    /// A new pick of the same picker with no Last pick of its own, as one toward L 0 that
    /// writes nothing, takes the shown one and its warnings away; another picker's pick,
    /// the reference's, leaves them (decision 88).
    #[test]
    fn a_pick_with_no_last_pick_takes_the_shown_one_away() {
        let mut picked = last_picked(true, true, false);
        assert!(picked.last("exposure").is_some());
        let mut reference = exposure_reading();
        reference.picker = PickerRef::Tool {
            tool: "exposure",
            picker: "reference",
        };
        reference.through = 4;
        let shown = reference.tool.as_mut().unwrap().picked.as_mut().unwrap();
        shown.settings = vec![("reference_lightness", ControlValue::Number(40.0))];
        picked.apply(Some(&reference));
        assert!(picked.last("exposure").is_some());
        let mut none = exposure_reading();
        none.through = 5;
        let shown = none.tool.as_mut().unwrap().picked.as_mut().unwrap();
        shown.settings.clear();
        shown.patch = Some(apertrust_tools::pick::Patch::of_lch([30.0, 0.0, 0.0]));
        picked.apply(Some(&none));
        assert!(picked.last("exposure").is_none());
    }

    /// Exposure's manual page after a pick, in each theme (the board's Last pick and
    /// warning variants), on a colour raw with a camera bias and a held-back exposure and
    /// the picker on: Last pick alone, with the clipped warning, short with the black
    /// moved, and both.
    fn photograph_last_pick(clipped: bool, short: bool, name: &str) {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = targeted_with(
                last_picked(clipped, short, short),
                which,
                ToolSettings::default(),
            );
            let state = harness.state_mut();
            state.camera = true;
            state.on = Some(EXPOSURE_PICKER);
            // Exposure where the pick put it: +18 EV held short, else 0.55 EV over 0.
            state.exposure = Some(if short { 18.0 } else { 0.55 });
            harness.run();
            harness.snapshot(format!("{name}-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    #[test]
    fn exposure_s_last_pick() {
        photograph_last_pick(false, false, "exposure-last-pick");
    }

    #[test]
    fn exposure_s_last_pick_of_clipped_pixels() {
        photograph_last_pick(true, false, "exposure-last-pick-clipped");
    }

    #[test]
    fn exposure_s_last_pick_short() {
        photograph_last_pick(false, true, "exposure-last-pick-short");
    }

    #[test]
    fn exposure_s_last_pick_clipped_and_short() {
        photograph_last_pick(true, true, "exposure-last-pick-both");
    }

    /// The settings with Picker target at Reference, and the reference at `lightness`
    /// measured on `photo`, or none.
    fn referenced(reference: Option<(f32, &str)>) -> ToolSettings {
        let mut settings = ToolSettings::default();
        settings.set("exposure", "target", ControlValue::Choice(2));
        if let Some((lightness, photo)) = reference {
            let lightness = ControlValue::Number(lightness);
            settings.set("exposure", "reference_lightness", lightness);
            settings.set_text("exposure", "reference_photo", photo.to_owned());
        }
        settings
    }

    const REFERENCE_PHOTO: &str = "/photos/IMG_2041.CR3";

    /// With Reference chosen and none set (decision 88): the row says none, Exposure's
    /// picker is disabled saying why, and ⋯'s Clear Reference waits for one; its own picker
    /// and Set Reference from an Area ask for the reference picker.
    #[test]
    fn with_no_reference_exposure_s_picker_waits_and_the_row_offers_its_own() {
        let mut harness = targeted_with(Picked::default(), Builtin::Dark, referenced(None));
        harness.get_by_role_and_label(Role::Label, "Reference none");
        let why = "Set a reference first, with the picker on the Reference row";
        let exposure = harness.get_by_role_and_label(Role::Button, why);
        assert!(exposure.accesskit_node().is_disabled());
        assert!(quad_tip(
            &mut harness,
            "Set a reference first, with the picker on the Reference row"
        ));
        harness
            .get_by_role_and_label(Role::Button, "Set the reference from an area")
            .click();
        harness.run();
        let asked = |change: Option<&Change>| {
            matches!(
                change,
                Some(Change::Picker {
                    tool: "exposure",
                    picker: "reference",
                    action: PickerAction::On { .. },
                    ..
                })
            )
        };
        assert!(asked(harness.state().changes.last()));
        harness
            .get_by_role_and_label(Role::Button, "Reference options")
            .click();
        harness.run();
        assert!(
            harness
                .get_by_label("Clear Reference")
                .accesskit_node()
                .is_disabled()
        );
        harness.get_by_label("Set Reference from an Area").click();
        harness.run();
        assert!(asked(harness.state().changes.last()));
    }

    /// A reference `--conf` gives keeps it for the run: the row's picker and ⋯ are drawn
    /// disabled, saying why, as the Settings dialog says of an overridden row.
    #[test]
    fn a_reference_conf_gives_is_drawn_disabled() {
        let given = vec!["tools.exposure.reference_lightness=60".to_owned()];
        let overrides = crate::settings::overrides(&given).expect("read");
        let folder = crate::settings::testing::Folder::new("reference-conf-row");
        let (_, settings) = crate::settings::Keeper::open_with(folder.storage(), &overrides);
        let mut settings = settings.tool_settings;
        settings.set("exposure", "target", ControlValue::Choice(2));
        let mut harness = targeted_with(Picked::default(), Builtin::Dark, settings);
        harness.get_by_role_and_label(Role::Label, "Reference L 60 %");
        for name in ["Set the reference from an area", "Reference options"] {
            let button = harness.get_by_role_and_label(Role::Button, name);
            assert!(button.accesskit_node().is_disabled(), "{name}");
        }
        harness
            .ctx
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        harness
            .get_by_role_and_label(Role::Button, "Set the reference from an area")
            .hover();
        harness.run();
        harness.get_by_label("Set by --conf for this run");
        harness
            .get_by_role_and_label(Role::Button, "Reference options")
            .click();
        harness.run();
        assert!(harness.query_by_label("Clear Reference").is_none());
        assert!(
            harness.state().changes.is_empty(),
            "{:?}",
            harness.state().changes
        );
    }

    /// A reference whose photo is not known shows its L alone, with no photo part, and
    /// Exposure's picker says it will match that L.
    #[test]
    fn a_reference_with_no_photo_known_shows_its_l_alone() {
        // As `--conf tools.exposure.reference_lightness=60` gives one.
        let mut settings = referenced(None);
        settings.set(
            "exposure",
            "reference_lightness",
            ControlValue::Number(60.0),
        );
        let mut harness = targeted_with(Picked::default(), Builtin::Dark, settings);
        harness.get_by_role_and_label(Role::Label, "Reference L 60 %");
        assert!(quad_tip(&mut harness, "Match an area to L 60 %"));
    }

    /// A reference set shows its L and its photo, "this photo" on the photo it was
    /// measured on, its name dimmed and said gone once the file is; Exposure's picker says
    /// what it will match; Clear Reference takes both settings away.
    #[test]
    fn a_reference_shows_its_photo_and_is_cleared_whole() {
        let settings = referenced(Some((62.0, REFERENCE_PHOTO)));
        let mut harness = targeted_with(Picked::default(), Builtin::Dark, settings);
        harness.get_by_role_and_label(Role::Label, "Reference L 62 % · IMG_2041");
        let action = "Match an area to IMG_2041 (L 62 %)";
        let exposure = harness.get_by_role_and_label(Role::Button, action);
        assert!(!exposure.accesskit_node().is_disabled());
        assert!(quad_tip(&mut harness, action));
        harness.state_mut().photo = Some(REFERENCE_PHOTO.into());
        harness.run();
        // The reference measured on the open photo says so, the button's name and its tip.
        assert!(quad_tip(
            &mut harness,
            "Match an area to this photo (L 62 %)"
        ));
        harness.get_by_role_and_label(Role::Label, "Reference L 62 % · this photo");
        harness.state_mut().photo = None;
        harness.state_mut().gone = true;
        harness.run();
        let row = harness.get_by_role_and_label(Role::Label, "Reference L 62 % · IMG_2041");
        assert_eq!(
            row.accesskit_node().description().as_deref(),
            Some("The photo it was measured on is no longer there")
        );
        harness
            .get_by_role_and_label(Role::Button, "Reference options")
            .click();
        harness.run();
        harness.get_by_label("Clear Reference").click();
        harness.run();
        assert_eq!(
            harness.state().changes.last(),
            Some(&Change::ClearReference {
                tool: "exposure",
                lightness: "reference_lightness",
                photo: "reference_photo",
            })
        );
        harness.run();
        harness.get_by_role_and_label(Role::Label, "Reference none");
    }

    /// Exposure's manual page with Picker target at Reference, in each theme, as the
    /// board's reference variants, on a colour raw with a camera bias and a held-back
    /// exposure: `reference` set or none, measuring while its picker is on, on the photo
    /// open, or on a photo gone.
    fn photograph_reference(
        reference: Option<(f32, &str)>,
        measuring: bool,
        this_photo: bool,
        gone: bool,
        name: &str,
    ) {
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness = targeted_with(Picked::default(), which, referenced(reference));
            let state = harness.state_mut();
            state.camera = true;
            state.on = measuring.then_some(PickerRef::Tool {
                tool: "exposure",
                picker: "reference",
            });
            state.photo = this_photo.then(|| REFERENCE_PHOTO.into());
            state.gone = gone;
            harness.run();
            harness.snapshot(format!("{name}-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    #[test]
    fn exposure_s_reference_none() {
        photograph_reference(None, false, false, false, "exposure-reference-none");
    }

    /// The reference picker measuring, with a reference already set: the row says
    /// measuring in place of its value.
    #[test]
    fn exposure_s_reference_measuring() {
        let reference = Some((62.0, REFERENCE_PHOTO));
        photograph_reference(
            reference,
            true,
            false,
            false,
            "exposure-reference-measuring",
        );
    }

    /// The board's B8: Exposure's picker on, matching the reference, a Last pick shown.
    #[test]
    fn exposure_s_picker_matching_a_reference() {
        let settings = referenced(Some((62.0, REFERENCE_PHOTO)));
        let mut results = egui_kittest::SnapshotResults::new();
        for (which, theme) in THEMES {
            let mut harness =
                targeted_with(last_picked(false, false, false), which, settings.clone());
            let state = harness.state_mut();
            state.camera = true;
            state.on = Some(EXPOSURE_PICKER);
            state.exposure = Some(0.55);
            harness.run();
            harness.snapshot(format!("exposure-reference-matching-{theme}"));
            results.extend_harness(&mut harness);
        }
    }

    #[test]
    fn exposure_s_reference_on_this_photo() {
        let reference = Some((62.0, REFERENCE_PHOTO));
        photograph_reference(
            reference,
            false,
            true,
            false,
            "exposure-reference-this-photo",
        );
    }

    #[test]
    fn exposure_s_reference_on_another_photo() {
        let reference = Some((62.0, REFERENCE_PHOTO));
        photograph_reference(reference, false, false, false, "exposure-reference");
    }

    #[test]
    fn exposure_s_reference_on_a_photo_gone() {
        let reference = Some((62.0, REFERENCE_PHOTO));
        photograph_reference(reference, false, false, true, "exposure-reference-gone");
    }

    /// The reference row's ⋯ open, its two items, Clear Reference live with one set.
    #[test]
    fn exposure_s_reference_menu_in_the_dark_theme() {
        let settings = referenced(Some((62.0, REFERENCE_PHOTO)));
        let mut harness = targeted_with(Picked::default(), Builtin::Dark, settings);
        harness
            .get_by_role_and_label(Role::Button, "Reference options")
            .click();
        harness.run();
        harness.snapshot("exposure-reference-menu-dark");
    }
}
