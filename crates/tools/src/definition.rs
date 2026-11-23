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

//! A tool's definition (decision 28, `docs/gui.md`, Tool panels from definitions):
//! static data the app draws a panel from and MCP describes. Controls, what an edit
//! stores, are kept apart from rows, how a tab shows them, so validation, edits and MCP
//! read the controls without walking the panel.
//!
//! The rows are darktable's page as darktable has it (decision 29): its tabs, its section
//! labels and collapsible sections as [`Row::Group`]s, plain and collapsible (decision 100),
//! the rows it shows only in some modes as [`Row::When`], the rows it draws outside its notebook in [`ToolDefinition::before`] and
//! [`ToolDefinition::after`], every option of a choice, those not ported marked, its own
//! settings, which no edit stores, as [`Row::Setting`], and its buttons,
//! actions no edit stores either, as a [`Row::Line`]'s [`End::Action`]s. Which rows show
//! and which options can be chosen is the panel's, from the values, so the edit and the
//! render never depend on it.
//!
//! Two kinds of row store nothing (decision 2): a preset row, whose options each write
//! several controls ([`PresetRow`]), and a lossless view's coordinate, a second view of
//! some stored controls ([`View`]), shown by a [`Row::Slider`] that names it.
//!
//! A tool's colour pickers (decision 79) are declared apart from the rows too
//! ([`Picker`]), and each shown in one place: a slider's end item ([`End::Picker`]), a
//! line's ([`Row::Line`]), a reference row ([`Row::Reference`]), or an option of a preset
//! row that turns it on ([`PresetRow::pickers`]).

use std::collections::BTreeSet;

use apertrust_base::wire::Id;
use serde::{Deserialize, Serialize};

use crate::values::{ControlValue, Controls};

mod cursor;

pub use cursor::{CursorDef, CursorGroup, CursorGroupDef, Gesture, Surface, ToolCursor};

/// What every tool declares, a processing tool's and a utility tool's alike (decision 93):
/// the parent each kind's definition embeds, as darktable's `dt_action_t` begins both its
/// processing and its utility modules. Its words are message ids that follow from its id
/// ([`crate::messages`]), so it holds no text.
#[derive(Debug, Clone, Copy)]
pub struct ToolDefinition {
    /// snake_case ASCII and permanent: every saved edit, the CLI and scripts name it.
    pub id: &'static str,
    /// One untitled tab draws no tab strip.
    pub tabs: &'static [Tab],
    /// Rows darktable draws outside its notebook, shown whichever tab is open: these
    /// before the tab's rows, as the colour equalizer's node placement.
    pub before: &'static [Row],
    /// And these after them, as the tone equalizer's mask display.
    pub after: &'static [Row],
    /// What its header's ↺ does, or none for a header with no ↺.
    pub reset: Reset,
    /// Whether its header has the presets button, opening its presets (decision 106):
    /// every processing tool's; a utility's only where it has presets of the settings its
    /// page stores, as Export's, named `<id>-presets`.
    pub presets: bool,
    /// Whether its section is open at first, until the person opens or closes it, which the
    /// app then keeps in `[edit.sections]` and which wins over this (decision 93): false
    /// for every processing tool, as darktable's processing modules start collapsed, and
    /// for Export, which Export… and Ctrl+E open; true for Collections, the way to the
    /// photos, Scopes and History.
    pub open_at_first: bool,
    /// The items it adds to context menus (decision 91; `docs/gui.md`, Context menus), each
    /// one of its own actions at a place; none for a tool with nothing to add. The app
    /// builds the menus, every item, the app's included, where its section and order put
    /// it ([`Section`]).
    pub menus: &'static [MenuItem],
    /// The actions it offers for the Toolbar (decision 92; `docs/gui.md`, Customising the
    /// Toolbar), each one of its own, on the toolbar only where a person places it;
    /// none for a tool with nothing to offer. A processing tool offers two at most
    /// ([`MOST_ON_TOOLBAR`]), a utility any number.
    pub toolbar: &'static [ToolbarItem],
    /// Its picker cursors (decision 104; `docs/cursors.md`, The API), each the second door
    /// to one of its pickers, which the app's well offers in Pick; none for a tool with none,
    /// and for every utility until the Colour readout's design.
    pub cursors: &'static [ToolCursor],
}

impl ToolDefinition {
    /// A processing tool's declaration with a processing tool's defaults: its page `tabs`
    /// and no rows before or after them, its own reset, the presets button, no context menu
    /// items, toolbar items or cursors, and closed at first, as every processing tool is
    /// unless it sets `open_at_first` (decision 93). A tool that differs writes the fields
    /// it changes over it,
    /// `ToolDefinition { menus: MENUS, ..ToolDefinition::processing(id, TABS) }`.
    pub const fn processing(id: &'static str, tabs: &'static [Tab]) -> ToolDefinition {
        ToolDefinition {
            id,
            tabs,
            before: &[],
            after: &[],
            reset: Reset::Tool,
            presets: true,
            open_at_first: false,
            menus: &[],
            toolbar: &[],
            cursors: &[],
        }
    }

    /// Its setting row `id` ([`Row::Setting`]), wherever its page shows it: before its tabs,
    /// on one of them or after them, in a group or among rows following a value.
    pub fn setting(&self, id: &str) -> Option<&'static SettingRow> {
        fn find(rows: &'static [Row], id: &str) -> Option<&'static SettingRow> {
            rows.iter().find_map(|row| match row {
                Row::Setting(found) if found.id == id => Some(found),
                _ => find(row.rows(), id),
            })
        }
        let tabs = self.tabs.iter().map(|tab| tab.rows);
        std::iter::once(self.before)
            .chain(tabs)
            .chain(std::iter::once(self.after))
            .find_map(|rows| find(rows, id))
    }

    /// Every row in the order the panel lists them, each before the rows inside it, with
    /// where it shows: its tab, its group, its modes and the settings it follows. A lone
    /// tab draws no tab strip, so its rows show beside those before and after it.
    fn placed(&self) -> Vec<Placed<'static>> {
        let mut placed = Vec::new();
        let strip = self.tabs.len() > 1;
        let tabs = self.tabs.iter().enumerate();
        let all = std::iter::once((None, self.before))
            .chain(tabs.map(|(i, tab)| (strip.then_some(i), tab.rows)))
            .chain(std::iter::once((None, self.after)));
        for (tab, rows) in all {
            let at = Where {
                tab,
                ..Where::default()
            };
            place(rows, &at, &mut placed);
        }
        placed
    }

    /// The first setting that rows follow ([`Row::WhenSetting`]) but that is not sure to
    /// show wherever they do, as two settings each among the rows the other hides; none
    /// where every such setting shows. A setting the page lacks is no concern of this.
    pub(crate) fn hidden_setting(&self) -> Option<&'static str> {
        let placed = self.placed();
        placed.iter().find_map(|Placed { row, at }| {
            let Row::WhenSetting { setting, .. } = row else {
                return None;
            };
            let its = placed.iter().find_map(|its| match its.row {
                Row::Setting(found) if found.id == *setting => Some(&its.at),
                _ => None,
            })?;
            (!its.shows_wherever(at)).then_some(*setting)
        })
    }
}

/// What a tool's header's ↺ does (decision 93).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reset {
    /// A processing tool's own reset, `ResetTool`.
    Tool,
    /// A registry action of the app's, by its id, as History's `reset`, Reset Edit, which
    /// Edit › Reset Edit runs too.
    Action(&'static str),
    /// The utility's own, `UtilityTool::reset` in the app, named `<id>-reset`, as Export's
    /// Default settings.
    Own,
    /// No ↺, as Scopes' and Collections' headers have none.
    None,
}

/// A processing tool, as data: the shared [`ToolDefinition`] and what editing needs.
#[derive(Debug)]
pub struct ProcessingDefinition {
    /// Its id, page and context menu items, as every tool's.
    pub tool: ToolDefinition,
    /// From 1, raised whenever a control's meaning or the binding changes, with a
    /// migration ([`crate::ProcessingTool::migrate`]).
    pub version: u32,
    /// Whether the tool works on a masked layer.
    pub layers: bool,
    /// Whether the tool has an Auto. The header draws no button for it.
    pub auto: bool,
    /// Whether the tool is switched (decision 32): one darktable leaves off on a new
    /// photo, off until a command writes one of its controls, then on with darktable's
    /// defaults for the rest until its reset turns it off. It owns no parameter of an
    /// operation every photo runs ([`crate::check::accept`]), and its defaults bind its
    /// operations' own ([`crate::check::switched_defaults`]).
    pub switched: bool,
    pub controls: &'static [Control],
    /// The preset rows its [`Row::Preset`]s draw (decision 2): options that each write
    /// several of its controls, as White balance's Mode. Nothing is stored under them.
    pub preset_rows: &'static [PresetRow],
    /// Its lossless views (decision 2): second views of some of its controls, as White
    /// balance shows its light as temperature and tint, and as hue and chroma. Nothing is
    /// stored under them.
    pub views: &'static [View],
    /// Its colour pickers (decision 79), each darktable's `dt_color_picker_new` call, shown
    /// by a slider's end item ([`End::Picker`]), a line's ([`Row::Line`]), a reference row
    /// ([`Row::Reference`]) or a preset row's option that turns it on
    /// ([`PresetRow::pickers`]), as White balance's Settings' from image area.
    /// Nothing is stored under them: the session measures the photo and the tool's
    /// [`crate::ProcessingTool::pick`] writes its controls.
    pub pickers: &'static [Picker],
    /// darktable's facts about the module the tool is, from its `description()` at the
    /// reference release (`docs/gui.md`, The header's tooltip): what its header's tooltip
    /// says after the tool's sentence, `tool-<tool>-description`. `None` for a tool that
    /// is not one of darktable's.
    pub facts: Option<crate::about::Facts>,
    /// Whether Copy Edit takes it (decision 106): false for the tools darktable flags
    /// `IOP_FLAGS_UNSAFE_COPY` at the reference release, settings that belong to one
    /// photo; true for every other.
    pub copy_safe: bool,
    /// Whether Create a Style… starts with it checked (decision 106): true for the tools
    /// darktable flags `IOP_FLAGS_INCLUDE_IN_STYLES` at the reference release, those that
    /// make a look rather than correct one photo; false for every other.
    pub in_styles: bool,
    /// The facts it publishes for other tools' actions to read, each snake_case and once
    /// (decision 107), each given by its [`crate::ProcessingTool::publish`]: Exposure's
    /// `effective_exposure`; none for every other tool.
    pub publishes: &'static [&'static str],
}

/// A fact another tool publishes, which an action reads ([`ActionEnd::reads`]; decision
/// 107): the publishing tool's id and the fact's, one of its
/// [`ProcessingDefinition::publishes`]. Named apart from the header tooltip's facts
/// ([`crate::about::Fact`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Published {
    pub tool: &'static str,
    pub id: &'static str,
}

/// One of the tool's colour pickers (decision 79; `docs/pickers.md`, Declaring
/// pickers): darktable's `dt_color_picker_new` call, its flags and colour space, and what
/// its pick may write and read. It is in exactly one place: a slider's end item, a line's,
/// a reference row, or a preset row's option that turns it on
/// ([`PresetRow::pickers`]).
///
/// Its words come with its button and the viewer bar (`docs/pickers.md`, The picker
/// button, On the photo), in the controls' and actions' scheme:
/// `tool-<tool>-picker-<id>-label`, its name; `-full`, what screen readers say;
/// `-promise`, its tooltip; and, for a point picker, `-what`, the viewer bar's sentence
/// ([`crate::messages::picker`]). A picker whose words follow a setting
/// ([`Picker::words_by`]) has, in place of `-promise`, `-promise-<option>` and
/// `-hint-<option>` for each of its options. [`crate::messages::of`] implies them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picker {
    /// snake_case ASCII and permanent, unique among the tool's pickers: clients name it
    /// with the tool's id. Pickers are a namespace of their own, as a picker at a
    /// slider's end takes its control's id (Exposure's `exposure`).
    pub id: &'static str,
    /// The operation whose input, and with `output` output, it reads: one the tool owns
    /// a parameter of ([`crate::check::accept`]).
    pub operation: &'static str,
    /// darktable's `DT_COLOR_PICKER_POINT`, `_AREA` or `_POINT_AREA`.
    pub kind: PickerKind,
    /// darktable's `DT_COLOR_PICKER_DENOISE`: a B-spline blur before the statistics, never
    /// on a mosaic.
    pub denoise: bool,
    /// darktable's `DT_COLOR_PICKER_IO`: the operation's output read as well as its input.
    pub output: bool,
    /// darktable's `picker_cst`: the space each pixel is read in.
    pub space: Space,
    /// darktable's "keep-active": it stays on when its tool changes, as RGB curve's
    /// show colour does.
    pub keep: bool,
    /// The controls its pick may write, each once; none for a picker that only shows.
    pub writes: &'static [&'static str],
    /// The tool's settings its pick reads ([`Row::Setting`]), each once.
    pub settings: &'static [&'static str],
    /// The choice setting of the tool ([`Row::Setting`]) whose option says what a pick will
    /// do, and so chooses the picker's tooltip and its hint over the photo (decision 88):
    /// `tool-<tool>-picker-<picker>-promise-<option>` and `-hint-<option>`
    /// ([`crate::messages::picker_option`]), as Exposure's Picker target does. Each
    /// option of that setting has a tooltip of its own in the list
    /// ([`crate::messages::setting_option_promise`]). `None` for a picker with one
    /// promise.
    pub words_by: Option<&'static str>,
}

/// A context menu tools add items to (decision 91; `docs/gui.md`, Context menus). The
/// list is closed: the app's other places, as the surround's, hold the app's items alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Place {
    /// The photo in the Viewer.
    Photo,
    /// A thumbnail: a cell of the Browser, or a thumbnail in the Viewer's Photo layout while it
    /// shows several photos.
    Thumbnail,
    /// The tool's own options, ⋯, in its own section only.
    Options,
}

impl Place {
    /// The most items one tool or utility adds here: two in the photo's and a thumbnail's
    /// menus, four in ⋯.
    pub const fn most(self) -> usize {
        match self {
            Place::Photo | Place::Thumbnail => 2,
            Place::Options => 4,
        }
    }
}

/// Where an item stands in a context menu (decision 91): the menu lists its items by
/// section, lowest first, then by [`MenuItem::order`] within one, a line wherever the
/// section changes. The app's items and the tools' take the same sections, so a tool's
/// Reset sits in Edit's whoever adds the rest of it.
///
/// The named sections are 1000 apart, and a tool may take one of its own value: a new
/// section goes between two existing ones, the gaps leaving room to insert without
/// renumbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Section(pub u32);

impl Section {
    /// What changes the edit: Reset, and Copy Edit and Apply Edit once built.
    pub const EDIT: Section = Section(1000);
    /// What writes the photo out: Export….
    pub const EXPORT: Section = Section(2000);
    /// How the photo is shown: Background ▸.
    pub const VIEW: Section = Section(3000);
    /// The edit's history.
    pub const HISTORY: Section = Section(4000);
    /// Items about the panel's layout, which always come last; nothing is in it yet.
    pub const LAYOUT: Section = Section(9000);
}

/// One item of a context menu, a tool's, a utility's or the app's: an action or a
/// submenu of actions, at a place, where its section and order put it. Every action is
/// one of the app's registry, which builds the menu and runs it; the tool runs no code
/// there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    pub place: Place,
    /// A tool's or a utility's from [`Section::EDIT`] up to, not including,
    /// [`Section::LAYOUT`] ([`check_section`]); the app's anywhere.
    pub section: Section,
    /// Its place within its section, lowest first, every item's in a place's section its
    /// own, the app's included. Orders go 100 apart (100, 200, 300…): a new item goes
    /// between two existing ones, the gaps leaving room to insert without renumbering. In
    /// `EDIT`, 100 is Reset's, so a tool's there take 200 or more.
    pub order: u32,
    pub entry: MenuEntry,
}

/// What a context menu item holds: an action, or a submenu of actions, one level deep.
///
/// An action is a tool's own, an [`End::Action`]'s id, which
/// the app's registry holds as `tool-<tool>-<action>`; a utility's or the app's, the
/// registry action's id, as History's Reset is `reset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEntry {
    Action(&'static str),
    /// Its title, a message id, and its actions, from one up to [`MOST_IN_SUBMENU`], each
    /// once, in order. It counts as one item towards its place's limit.
    Submenu {
        title: &'static str,
        actions: &'static [&'static str],
    },
}

impl MenuEntry {
    /// The actions it names: its own, or the submenu's, in order.
    pub fn actions(&self) -> &[&'static str] {
        match self {
            MenuEntry::Action(action) => std::slice::from_ref(action),
            MenuEntry::Submenu { actions, .. } => actions,
        }
    }

    /// What names it in an error: its action, or the submenu's title.
    pub fn name(&self) -> &'static str {
        match self {
            MenuEntry::Action(action) => action,
            MenuEntry::Submenu { title, .. } => title,
        }
    }
}

/// The most actions a context menu's submenu holds.
pub const MOST_IN_SUBMENU: usize = 8;

/// Whether `items`, one tool's or one utility's, hold together: no action listed twice in
/// a place, a submenu's included, each submenu holding one to [`MOST_IN_SUBMENU`] actions,
/// and no place holding more items than it takes ([`Place::most`]), a submenu one.
pub fn check_menus(items: &[MenuItem]) -> Result<(), DefinitionError> {
    for (i, item) in items.iter().enumerate() {
        let actions = item.entry.actions();
        if let MenuEntry::Submenu { title, .. } = item.entry
            && !(1..=MOST_IN_SUBMENU).contains(&actions.len())
        {
            return Err(DefinitionError::SubmenuItems(title));
        }
        // Its actions at its place, whatever its section and order.
        let earlier = items[..i].iter().filter(|other| other.place == item.place);
        let earlier: Vec<&str> = earlier
            .flat_map(|other| other.entry.actions())
            .copied()
            .collect();
        for (j, action) in actions.iter().enumerate() {
            if earlier.contains(action) || actions[..j].contains(action) {
                return Err(DefinitionError::DuplicateMenuItem(action));
            }
        }
    }
    for place in [Place::Photo, Place::Thumbnail, Place::Options] {
        let held = items.iter().filter(|item| item.place == place).count();
        if held > place.most() {
            return Err(DefinitionError::MenuItems(place));
        }
    }
    Ok(())
}

/// Whether a tool's or a utility's item stands where they may (decision 91): in a section
/// from [`Section::EDIT`] up to, not including, [`Section::LAYOUT`], the rest being the
/// app's, and in `EDIT` at order 200 or more, after Reset. The app's own items are exempt,
/// and so is History's Reset, at `EDIT`, 100.
pub fn check_section(item: &MenuItem) -> Result<(), DefinitionError> {
    if !(Section::EDIT..Section::LAYOUT).contains(&item.section) {
        return Err(DefinitionError::MenuSection(item.entry.name()));
    }
    if item.section == Section::EDIT && item.order < 200 {
        return Err(DefinitionError::MenuOrder(item.entry.name()));
    }
    Ok(())
}

/// An action a tool offers for the Toolbar (decision 92; `docs/gui.md`, Customising the
/// Toolbar): a candidate the app lists, on the toolbar only where a person places it. The
/// app runs it as it runs the action from a menu; the tool runs no code there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolbarItem {
    /// A processing tool's own action as its menu items name it, an [`End::Action`]'s id,
    /// as Orientation's `rotate_ccw`, which the app's registry
    /// holds as `tool-<tool>-<action>`; a utility's, the registry action it names, as
    /// History's `undo`.
    pub action: &'static str,
    /// One of `apertrust-ui`'s icons by its name, as `export`; none for an icon-faced
    /// [`End::Action`]'s action, which takes its button's icon.
    pub icon: Option<&'static str>,
}

/// The most items a processing tool offers for the Toolbar (decision 92). A utility's have
/// no limit, as History offers the edit's six.
pub const MOST_ON_TOOLBAR: usize = 2;

/// Whether `items`, one tool's toolbar items, hold together: no action empty or offered
/// twice, none with no icon but the actions of `buttons`, its icon-faced [`End::Action`]s',
/// which take their buttons' (none for a utility), and no more items than `most`, a
/// processing tool's [`MOST_ON_TOOLBAR`], where there is a limit.
pub fn check_toolbar(
    items: &[ToolbarItem],
    most: Option<usize>,
    buttons: &[&str],
) -> Result<(), DefinitionError> {
    for (i, item) in items.iter().enumerate() {
        if item.action.is_empty() {
            return Err(DefinitionError::UnknownToolbarAction(item.action));
        }
        if items[..i].iter().any(|other| other.action == item.action) {
            return Err(DefinitionError::DuplicateToolbarItem(item.action));
        }
        if item.icon.is_none() && !buttons.contains(&item.action) {
            return Err(DefinitionError::ToolbarIcon(item.action));
        }
    }
    if most.is_some_and(|most| items.len() > most) {
        return Err(DefinitionError::ToolbarItems);
    }
    Ok(())
}

/// Whether a picker takes a point, an area or either: darktable's
/// `DT_COLOR_PICKER_POINT`, `_AREA` and `_POINT_AREA`. One with both turns on as an area
/// with a right-click or Ctrl-click, else as a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickerKind {
    Point,
    Area,
    PointArea,
}

/// The space a picker reads each pixel in, darktable's `picker_cst`
/// (`dt_color_picker_helper`). In a space with a hue the fourth channel is the third moved
/// half a turn, so a hue round 0 has a minimum and a maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// The stage's own values: the mosaic in the raw stage, per CFA colour, camera RGB,
    /// the working space.
    Operation,
    /// LCh, for a Lab stage (`dt_Lab_2_LCH`).
    Lch,
    /// HSL, for an RGB stage (`dt_RGB_2_HSL`).
    Hsl,
    /// JzCzhz, through the working profile's matrix.
    Jzczhz,
}

/// Options that each write several of the tool's controls, as White balance's Mode
/// writes its light (decision 2). Nothing is stored under it: the tool gives each
/// option's values for a photo ([`crate::ProcessingTool::preset`]) and says which option
/// the values hold ([`crate::ProcessingTool::matching`]), and the session applies one as
/// one change.
#[derive(Debug)]
pub struct PresetRow {
    /// snake_case ASCII and permanent, as a control's, and unique among the tool's
    /// controls and preset rows.
    pub id: &'static str,
    /// Option ids, snake_case, in the row's order.
    pub options: &'static [&'static str],
    /// The controls every option writes.
    pub controls: &'static [&'static str],
    /// The options that turn a picker of [`ProcessingDefinition::pickers`] on instead of writing
    /// values, each an option of the row and a picker's id, as White balance's Settings'
    /// from image area turns its `spot` picker on, as darktable's `_preset_tune_callback`
    /// presses the picker's button. Such an option is its picker's one place where the
    /// row is drawn, and its pick writes the values; [`crate::ProcessingTool::preset`] gives `None`
    /// for it, so it is never applied.
    pub pickers: &'static [(&'static str, &'static str)],
}

/// A lossless view of some of the tool's controls, every one a number (decision 2):
/// coordinates that are an exact, invertible change of coordinates of them, or fewer
/// coordinates than controls where the tool's model drops a scale of them, as White
/// balance's temperature and tint drop its three coefficients' (decision 27), `unview`
/// then the inverse up to that scale. Nothing is stored under it: the tool gives the
/// coordinates for the controls' values ([`crate::ProcessingTool::view`]) and the values
/// for coordinates ([`crate::ProcessingTool::unview`]), and the session writes the controls
/// when a coordinate is set, holding the view's other coordinates through the gesture.
#[derive(Debug)]
pub struct View {
    /// snake_case ASCII and permanent, unique among the tool's views.
    pub id: &'static str,
    /// The stored controls it covers, each a number of the tool, each once.
    pub controls: &'static [&'static str],
    /// As many as the controls it covers, or fewer for a model of them, at least one, in
    /// the order [`crate::ProcessingTool::view`] gives them.
    pub coordinates: &'static [Coordinate],
}

/// One coordinate of a [`View`], addressed like a control: its id names it in a tool
/// command, and its words are a control's ([`crate::messages`]).
#[derive(Debug)]
pub struct Coordinate {
    /// snake_case ASCII and permanent, unique among the tool's controls, preset rows
    /// and coordinates.
    pub id: &'static str,
    /// What a command may set it to, `min..=max`. The controls' values may give it one
    /// outside, which a slider shows at its end.
    pub min: f32,
    pub max: f32,
    /// What a key or a scroll step moves it by.
    pub step: f32,
    pub unit: Option<Unit>,
}

impl Coordinate {
    /// `value` as this coordinate's number, if it is a number in its range.
    pub fn check(&self, value: &ControlValue) -> Result<f32, ControlError> {
        let control = self.id;
        match *value {
            // Also refuses NaN, which no range contains.
            ControlValue::Number(number) if (self.min..=self.max).contains(&number) => Ok(number),
            ControlValue::Number(number) => Err(ControlError::OutOfRange {
                control,
                value: f64::from(number),
                min: f64::from(self.min),
                max: f64::from(self.max),
            }),
            _ => Err(ControlError::WrongKind {
                control,
                expected: "a number",
            }),
        }
    }
}

/// What an edit stores for a tool: one value of one kind.
#[derive(Debug)]
pub struct Control {
    /// snake_case ASCII and permanent, as the tool's.
    pub id: &'static str,
    pub kind: ControlKind,
    /// A number's unit; `None` for a plain number and every other kind.
    pub unit: Option<Unit>,
    /// Whether its parameter is not ported (decision 29). darktable shows it, so it is a
    /// control, but it takes only its default, as a choice takes no option not ported:
    /// any other value is refused, so no edit stores a value the render ignores and
    /// nothing needs migrating once it is ported. The session reports it unavailable, so
    /// a panel draws it disabled.
    pub unported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlKind {
    /// A number in `min..=max`. `step` is what a key or a scroll step moves it by; a
    /// command may set any value in the range.
    Number {
        min: f32,
        max: f32,
        step: f32,
        default: f32,
    },
    /// One of `options`, snake_case ids, by index: every option darktable lists
    /// (decision 29). Those its operation does not port are `unported`, by id: the panel
    /// draws them disabled and a value naming one is refused, so the default is never
    /// one.
    Choice {
        options: &'static [&'static str],
        default: u32,
        unported: &'static [&'static str],
    },
    Bool {
        default: bool,
    },
    /// A curve's points between its ends, (0, 0) and (1, 1), which the curve holds and
    /// the control does not store: up to [`MAX_POINTS`] of them, each coordinate from 0 to
    /// 1, x increasing from left to right by at least [`MIN_GAP`], from the ends too. None
    /// is the straight curve.
    Points {
        default: &'static [[f32; 2]],
    },
}

/// The most points a curve takes between its ends: darktable's RGB curve's 20 nodes
/// (`DT_IOP_RGBCURVE_MAXNODES`), less the two ends.
pub const MAX_POINTS: usize = 18;

/// How close in x two of a curve's points may come: darktable's RGB curve keeps its nodes
/// this far apart (`DT_IOP_RGBCURVE_MIN_X_DISTANCE`), and so does the design system's
/// curve, measuring in f32 as [`ControlKind::Points`] does.
pub const MIN_GAP: f32 = 0.0025;

/// A number's unit, from the closed list the translator has messages for: `unit-<id>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    Ev,
    Percent,
    Degrees,
    Kelvin,
    Pixels,
    /// A film's speed, as darktable's grain shows its coarseness.
    Iso,
}

impl Unit {
    pub const ALL: [Unit; 6] = [
        Unit::Ev,
        Unit::Percent,
        Unit::Degrees,
        Unit::Kelvin,
        Unit::Pixels,
        Unit::Iso,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Unit::Ev => "ev",
            Unit::Percent => "percent",
            Unit::Degrees => "degrees",
            Unit::Kelvin => "kelvin",
            Unit::Pixels => "pixels",
            Unit::Iso => "iso",
        }
    }
}

/// One tab of the panel. A titled tab with no rows has nothing behind it yet: the panel
/// draws it disabled, as an action that cannot run is, saying it is to come.
#[derive(Debug)]
pub struct Tab {
    /// The tab's id, for its title; `None` for a tool's only tab, which draws no tab
    /// strip.
    pub id: Option<&'static str>,
    pub rows: &'static [Row],
}

/// How a tab shows one control, a group of rows, plain or collapsible, rows darktable shows
/// only in some modes, a preset row, a lossless view's coordinate, one of darktable's settings
/// for the tool, or a line of pickers or actions. Each control is in exactly one row, or in one
/// row per mode where darktable labels it differently by mode ([`Slider::label`]) or shows
/// it under conditions one [`Row::When`] cannot join, as highlight reconstruction's Noise
/// level, or only in the rows of the views covering it; each coordinate is in one row at
/// most, and each picker in exactly one place: one row, or a drawn preset row's option
/// that turns it on ([`PresetRow::pickers`]).
#[derive(Debug, Clone)]
pub enum Row {
    Slider(Slider),
    /// A choice control, as a list or as segments. Only a list draws an option disabled,
    /// so a choice with unported options is a list.
    Choice {
        control: &'static str,
        style: ChoiceStyle,
        /// Its end items, before the option (decision 107); none until a choice's are
        /// drawn ([`DefinitionError::EndNotDrawn`]).
        end: &'static [End],
    },
    /// A boolean control.
    Toggle {
        control: &'static str,
        /// Its end items, after the switch (decision 107), as AgX's Reverse all takes Set
        /// from above.
        end: &'static [End],
    },
    /// A titled set of rows, darktable's section label or its collapsible section
    /// (decision 100): a [`Group`] of either kind, holding its rows.
    Group(Group),
    /// Rows darktable shows only while another control of the tool, a choice or a
    /// toggle, holds one of the values `is` names as that control writes them
    /// ([`Control::text`]): option ids, or `true` or `false`. Otherwise they are hidden,
    /// which a control that merely cannot act never is (decision 30). The control is
    /// shown wherever they are, or on another tab outside any group or mode, as the tone
    /// equalizer's filter on its Masking tab is for a row of its Advanced tab.
    When {
        control: &'static str,
        is: &'static [&'static str],
        rows: &'static [Row],
    },
    /// Rows shown while one of the tool's choice settings ([`Row::Setting`]) holds one of
    /// the options `is` names, as [`Row::When`] shows rows for a control's values, as
    /// Exposure's Lightness shows under Picker target while Lightness is chosen
    /// (decision 88). A setting is no control: what it holds is the app's, never the
    /// edit's, so these rows hold no control either, only settings and rows of no
    /// control. The setting shows wherever they do.
    WhenSetting {
        setting: &'static str,
        is: &'static [&'static str],
        rows: &'static [Row],
    },
    /// A widget from the design system's closed registry, with the controls it edits. The
    /// equalizer, over controls that have rows of their own, is a second view of them, the
    /// tone equalizer's graph of its bands (decision 2), and stores nothing of its own;
    /// no other widget may be a second row, until one needs it.
    Custom {
        widget: Widget,
        controls: &'static [&'static str],
    },
    /// A preset row of [`ProcessingDefinition::preset_rows`], by its id. It shows no
    /// control of its own: each of its options writes several. It is a list: segments
    /// cannot draw an option the photo cannot have, nor Custom, yet, so
    /// [`ProcessingDefinition::check`] refuses them.
    Preset {
        preset: &'static str,
        style: ChoiceStyle,
    },
    /// One of darktable's own settings for the tool, which darktable keeps in its
    /// darktablerc and never in an edit: the colour checker's chart, an area mode
    /// (decision 29; `docs/tools/README.md`, rule 12). It is no control: an app setting
    /// that comes with what uses it, never stored with the photo. Until that is built the
    /// panel draws it disabled at darktable's default, so the page reads as darktable's and
    /// shows what is missing.
    Setting(SettingRow),
    /// Two or three compact fields on one line, sharing its width, with no label of its
    /// own (decision 107), as Export's `[w] × [h] px`: below the width its fields need, it
    /// folds into one labelled row each. Its fields are a utility's settings, by their
    /// keys; [`ProcessingDefinition::check`] refuses it until Lens correction's port brings
    /// fields of controls ([`DefinitionError::Fields`]). It is no control.
    Fields(Fields),
    /// One line, darktable's widgets packed in a box (decision 107): a start, a readout in
    /// the label's place, and end items, of any shape:
    ///
    /// - pickers after a label or a readout, as AgX's Auto tune levels and Colour
    ///   calibration's CCT, the illuminant's colour and its picker;
    /// - a control's label, double-clicked to reset it, as darktable's reset label, with
    ///   icon actions after it that write the control from its value
    ///   ([`crate::ProcessingTool::action`]), as orientation's rotate and flip, the control
    ///   shown only here;
    /// - a line with no start and no readout whose one item is a text action, drawn at the
    ///   line's start as a button of its own.
    ///
    /// A click on an action is one change, a `SetControl` of each control it writes; one
    /// not ported is drawn disabled, so the page reads as darktable's and shows what is
    /// missing. It is no control, but for a control at its start. An action writes the
    /// tool's controls and is never stored (`docs/tools/README.md`, rule 10).
    Line {
        start: LineStart,
        /// A text and a swatch the tool computes from its values
        /// ([`crate::ProcessingTool::readout`]), at the row's start in place of a label,
        /// the swatch filling the row up to the end items.
        readout: Option<ReadoutDef>,
        end: &'static [End],
    },
    /// darktable's input and target patches of a picker's area mapping, side by side
    /// under their headings, as darktable's area mappings show them: the input,
    /// the area as the picker's last pick showed it ([`crate::pick::Shown::patch`]), with
    /// its L readout, and the target, the colour of the settings `target` names, its
    /// L, then its C and h where given, as darktable's `_target_color_draw` paints it
    /// ([`crate::pick::Patch::of_lch`]). It is no control; its words are shared
    /// ([`crate::messages::PATCHES`]), the target patch's tooltip its first setting's
    /// promise. Until its picker is drawn the panel draws it disabled, with no input.
    Patches {
        picker: &'static str,
        target: &'static [&'static str],
    },
    /// A computed value: a number the tool's operation computes from the photo for the
    /// panel, darktable's label beside a value it fills in as the image renders, as
    /// Exposure's "computed EC" (decision 87). One line, its label at the start and the
    /// value at the end, read-only. It is no control; its value comes from the session with
    /// the tool's state (`ToolState::computed`), none while the operation computes none.
    Computed(Computed),
    /// A reference kept across photos (decision 88), as Exposure's under Picker target's
    /// Reference: one line, its label the picker's, the reference's patch, its L and the
    /// name of the photo it was measured on, the picker that measures it, and ⋯ with Clear
    /// Reference and Set Reference from an Area; its words are shared
    /// ([`crate::messages::REFERENCE`]). The picker writes no control: its pick hands
    /// `lightness` back ([`crate::pick::Shown::settings`]), and the client keeps it with
    /// the open photo's path as `photo`. Both are settings of the tool, set and cleared
    /// together, optional, a number and a text, which no other row shows. It is no control.
    Reference {
        picker: &'static str,
        lightness: &'static str,
        photo: &'static str,
    },
    /// What the last pick of a picker that writes controls did (decision 88), one line,
    /// "L 41 % → L 50 % · +0.55 EV", the area before and after as two patches, from the
    /// pick's [`crate::pick::Shown::last`], and above it a warning line for each warning
    /// the pick gave ([`crate::pick::Shown::warnings`]), the clipped one first; nothing
    /// before a pick. Its words are shared ([`crate::messages::LAST_PICK`]) but for the
    /// picker's own on a black moved ([`crate::messages::short_black`]). It is no control.
    LastPick {
        picker: &'static str,
    },
    /// darktable's guides widget, which it adds to a module with
    /// `IOP_FLAGS_GUIDES_WIDGET` (`dt_guides_init_module_widget`): its show guides
    /// toggle, the overlay while the module has focus, kept in darktablerc and never in
    /// an edit, and the global guide settings button at its end. Its words are shared
    /// ([`crate::messages::GUIDES`]). Until the tool's guides come the panel draws it
    /// disabled, off.
    Guides,
    /// A read-only fact of a utility (decision 110), as the Metadata tool's rows: a label and
    /// its value, which the utility gives, with its words `tool-<tool>-fact-<id>-label`,
    /// `-tag` and `-none` ([`crate::messages::fact_words`]). It is no control and edits
    /// nothing: [`ProcessingDefinition::check`] refuses it, and a utility holds each fact id
    /// once among its facts.
    Fact(Fact),
    /// An editable field of a utility (decision 110), as the Metadata tool's Title: a label
    /// and a well holding the photo's value, which writes library data, never the edit, its
    /// words `tool-<tool>-field-<id>-label` and `-tag` ([`crate::messages::field_words`]).
    /// It is no control: [`ProcessingDefinition::check`] refuses it, and a utility holds each
    /// row id once among its facts and fields.
    Field(FieldRow),
}

/// A [`Row::Field`]: its id, the field it edits by its property's qualified name, as
/// `apertrust_library::fields::FIELDS` gives it (`dc:title`), or `apertrust:name` for the
/// copy's name, and whether it shows until the person hides it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldRow {
    pub id: &'static str,
    pub field: &'static str,
    pub shown: bool,
}

/// A [`Row::Fact`]: its id, whether it shows until the person hides it, where its value is
/// cut when it is too long, and the item at its end, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fact {
    pub id: &'static str,
    pub shown: bool,
    pub cut: Cut,
    pub end: Option<FactEnd>,
}

/// Where a fact's value too long for its row is cut with …: in its middle, a path or a file
/// name, its extension kept; or at its end, a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    Middle,
    End,
}

/// The item at a fact's end, an icon button its utility carries out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactEnd {
    /// Show in Finder (Explorer, Files): the photo's file in the platform's file manager,
    /// or the folder several photos share.
    ShowInFolder,
}

/// A readout at a line's start ([`Row::Line::readout`]): a text the tool
/// computes from its values, one of several by what they describe, with a number, and a
/// swatch of a colour they give, as darktable's "CCT: 5003 K (daylight)" beside the
/// illuminant's colour (`channelmixerrgb.c:4472-4491`). Its id is unique among the tool's
/// readouts. Each text's words are `tool-<tool>-readout-<id>-<text>`, its number the
/// variable `$value`, and its tooltip `-promise` after them; the swatch's tooltip is
/// `tool-<tool>-readout-<id>-swatch` ([`crate::messages::readout`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadoutDef {
    pub id: &'static str,
    /// Its texts' ids, snake_case, one of which the tool gives ([`crate::pick::Readout`]).
    pub texts: &'static [&'static str],
    /// Decimals its number shows.
    pub decimals: u8,
    /// Whether a swatch follows the text.
    pub swatch: bool,
}

/// A titled set of rows inside a tool's tab (decision 100), of one of two kinds, as a tool
/// is a processing tool or a utility (decision 93): darktable's section label, a plain
/// group, or its collapsible section, a collapsible group. Both hold their rows, so changing
/// a group's kind is a one-word change to its `kind`. A plain group may sit inside a
/// collapsible one; a collapsible group never sits inside another group, and a plain group
/// never inside a plain one. Its id is unique among the tool's groups of both kinds and
/// its lines of pickers' labels; its title is a message its id and kind imply
/// ([`crate::messages::group_title`]).
#[derive(Debug, Clone, Copy)]
pub struct Group {
    pub id: &'static str,
    pub kind: GroupKind,
    pub rows: &'static [Row],
}

/// A [`Group`]'s kind (decision 100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    /// darktable's section label: a title over its rows, always open, with no chevron. It
    /// has rows.
    Plain,
    /// darktable's collapsible section (decision 29): its title with a chevron, its rows
    /// under it, closed at first, its open state the app's. One with no rows has nothing
    /// behind it yet, as AgX's Show curve until a read-only graph row exists: the panel
    /// draws it disabled, closed, as a titled tab with no rows is, saying it is to come.
    Collapsible,
}

/// A [`Row::Computed`]: its id, unique among the tool's computed values, whose words are
/// `tool-<tool>-computed-<id>-label` and `-promise`, the operation that computes it
/// ([`apertrust_plugin::Operation::computed`], asked with the same id), one whose
/// parameters the tool owns, and how its value shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Computed {
    pub id: &'static str,
    pub operation: &'static str,
    pub unit: Option<Unit>,
    /// Decimals the value shows.
    pub decimals: u8,
}

/// What starts a [`Row::Line`] (decision 107).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStart {
    None,
    /// A plain group's words, by its id, unique among the tool's groups: its words are
    /// [`crate::messages::group`]'s.
    Label(&'static str),
    /// A control, its label darktable's reset label, double-clicked to reset it.
    Control(&'static str),
}

/// What sits at a row's end (decision 107), a closed list; the next steps add the other
/// kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// A picker of [`ProcessingDefinition::pickers`], its button turning it on and off.
    Picker(&'static str),
    /// One of the tool's actions, [`crate::ProcessingTool::action`].
    Action(ActionEnd),
    /// A read-only warning sign, shown while the tool's check reports it
    /// ([`crate::ProcessingTool::warning`]), as AgX's toe and shoulder power quads: on a
    /// slider's end alone, its id snake_case and unique among the tool's warnings, its
    /// message `tool-<tool>-warning-<id>` ([`crate::messages::warning`]).
    Warning(&'static str),
}

impl End {
    /// The picker's id, the action's or the warning's.
    pub fn id(&self) -> &'static str {
        match self {
            End::Picker(id) | End::Action(ActionEnd { id, .. }) | End::Warning(id) => id,
        }
    }

    /// The picker it turns on and off, if it is a picker's.
    pub fn picker(&self) -> Option<&'static str> {
        match self {
            End::Picker(id) => Some(id),
            End::Action(_) | End::Warning(_) => None,
        }
    }

    /// The action it runs, if it is an action's.
    pub fn action(&self) -> Option<&ActionEnd> {
        match self {
            End::Action(action) => Some(action),
            End::Picker(_) | End::Warning(_) => None,
        }
    }

    /// The warning it shows, if it is a warning's.
    pub fn warning(&self) -> Option<&'static str> {
        match self {
            End::Warning(id) => Some(id),
            End::Picker(_) | End::Action(_) => None,
        }
    }
}

/// An [`End::Action`]: one of the tool's actions, whose words are an action's
/// (`tool-<tool>-action-<id>-label`, `-full` and `-promise`), and its face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionEnd {
    /// snake_case, unique among the tool's actions; `tool-<tool>-<id>` in the registry.
    pub id: &'static str,
    pub face: Face,
    /// Empty for a button that runs its action. Otherwise an icon button that opens a menu
    /// of these actions in place of running one, as AgX's Reset primaries offers
    /// Blender-like, Smooth and Unmodified (`agx.c:2100-2121`): 2 to [`MOST_IN_SUBMENU`],
    /// each one of the tool's own, snake_case and unique among its actions, with an
    /// action's words, its registry action `tool-<tool>-<id>` and its availability, and
    /// none with a menu of its own. The button's words name the menu and it runs nothing,
    /// so it is no registry action.
    pub menu: &'static [&'static str],
    /// The facts other tools publish that it reads, and nothing else of the edit, as AgX's
    /// Read exposure reads Exposure's `effective_exposure`: each a built tool's published
    /// fact (`crates/tools/tests/builtin.rs`). A menu's items read their button's.
    pub reads: &'static [Published],
    /// Drawn disabled and never run until what it runs is built, as [`Control::unported`]
    /// is for a control (decision 29).
    pub unported: bool,
}

/// An action's face: an icon, or its label as a text button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Icon(ButtonIcon),
    Text,
}

/// The design system's icons an action's face can name, a closed registry as [`Widget`]
/// is: darktable's own shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonIcon {
    /// darktable's circular arrow turning anticlockwise (`dtgtk_cairo_paint_refresh`).
    RotateLeft,
    /// Its mirror image, clockwise.
    RotateRight,
    /// darktable's two triangles mirrored across a vertical line (`dtgtk_cairo_paint_flip`).
    FlipHorizontal,
    /// The same across a horizontal line.
    FlipVertical,
    /// A list with a turning arrow: AgX's menu of primaries to start again from.
    PrimariesMenu,
    /// darktable's camera (`dtgtk_cairo_paint_camera`): AgX's Read exposure.
    Camera,
}

impl Row {
    /// The controls the row shows itself, or the coordinate a view's row shows, ids being
    /// unique together; a group's and a mode's are their rows', a line's the control at
    /// its start, and a preset row shows none.
    pub fn controls(&self) -> &[&'static str] {
        match self {
            Row::Slider(slider) => std::slice::from_ref(&slider.control),
            Row::Choice { control, .. }
            | Row::Toggle { control, .. }
            | Row::Line {
                start: LineStart::Control(control),
                ..
            } => std::slice::from_ref(control),
            Row::Group(_)
            | Row::When { .. }
            | Row::WhenSetting { .. }
            | Row::Setting(_)
            | Row::Fields(_)
            | Row::Preset { .. }
            | Row::Line { .. }
            | Row::Patches { .. }
            | Row::Computed(_)
            | Row::Reference { .. }
            | Row::LastPick { .. }
            | Row::Guides
            | Row::Fact(_)
            | Row::Field(_) => &[],
            Row::Custom { controls, .. } => controls,
        }
    }

    /// The row's end items (decision 107): a slider's one, a line's, a choice's, a
    /// toggle's or a setting's; none for any other row.
    pub fn end(&self) -> &[End] {
        match self {
            Row::Slider(slider) => slider.end.as_slice(),
            Row::Line { end, .. }
            | Row::Choice { end, .. }
            | Row::Toggle { end, .. }
            | Row::Setting(SettingRow { end, .. }) => end,
            _ => &[],
        }
    }

    /// The pickers the row shows: its end items', or a reference's.
    pub fn pickers(&self) -> impl Iterator<Item = &'static str> + use<> {
        let (one, end): (Option<&'static str>, Vec<&'static str>) = match self {
            Row::Reference { picker, .. } => (Some(picker), Vec::new()),
            _ => (None, self.end().iter().filter_map(End::picker).collect()),
        };
        one.into_iter().chain(end)
    }

    /// The setting rows the row shows itself: a setting row's, or a line of fields' one a
    /// field; none for any other row, a group's and a mode's being their rows'.
    pub fn setting_rows(&self) -> impl Iterator<Item = &SettingRow> {
        let (one, fields): (Option<&SettingRow>, &[Field]) = match self {
            Row::Setting(setting) => (Some(setting), &[]),
            Row::Fields(fields) => (None, fields.fields),
            _ => (None, &[]),
        };
        one.into_iter()
            .chain(fields.iter().map(|field| &field.setting))
    }

    /// The rows inside a group or a mode's rows, a control's or a setting's; none for any
    /// other row.
    pub fn rows(&self) -> &'static [Row] {
        match self {
            Row::Group(Group { rows, .. })
            | Row::When { rows, .. }
            | Row::WhenSetting { rows, .. } => rows,
            _ => &[],
        }
    }
}

/// A number control, or a view's coordinate, as a slider row
/// (`apertrust_ui::components::SliderRow`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slider {
    /// A number control's id, or a coordinate's.
    pub control: &'static str,
    /// Decimals the value shows.
    pub decimals: u8,
    /// darktable's soft range (`dt_bauhaus_slider_set_soft_range`), `(min, max)` in the
    /// control's units, inside its range: what the track spans at rest. The control's
    /// range stays what a value may hold, so a typed value, a script or a preset goes
    /// beyond it, and the track widens to show it, as darktable's does. `None` where
    /// darktable sets none: the track spans the whole range.
    pub soft: Option<(f32, f32)>,
    pub track: Track,
    /// Whether the track fills from 0, held to its ends, to the value, as darktable's.
    pub fill: bool,
    /// The item at the row's end, if any, darktable's quad (decision 107): one, as
    /// darktable's is, and a picker until the other kinds are drawn there
    /// ([`DefinitionError::EndNotDrawn`]).
    pub end: Option<End>,
    /// The row's own words, by an id, where darktable labels the control otherwise in
    /// the mode the row shows for; `None` for the control's own.
    pub label: Option<&'static str>,
}

impl Slider {
    /// Two decimals, the track over the whole range, plain, with its fill, no end item, the
    /// control's own words.
    pub const fn new(control: &'static str) -> Self {
        Self {
            control,
            decimals: 2,
            soft: None,
            track: Track::Plain,
            fill: true,
            end: None,
            label: None,
        }
    }

    /// Words of the row's own, `words` a snake_case id: one row per mode where darktable
    /// shows one parameter under two labels, as local contrast's `sigma_r` is Contrast
    /// with the bilateral grid and Highlights with the local Laplacian filter, each row
    /// in a [`Row::When`] of its mode ([`crate::messages::row_words`]).
    pub const fn label(mut self, words: &'static str) -> Self {
        self.label = Some(words);
        self
    }

    pub const fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// darktable's soft range, in the control's units ([`Self::soft`]).
    pub const fn soft(mut self, min: f32, max: f32) -> Self {
        self.soft = Some((min, max));
        self
    }

    pub const fn track(mut self, track: Track) -> Self {
        self.track = track;
        self
    }

    pub const fn no_fill(mut self) -> Self {
        self.fill = false;
        self
    }

    /// The item at the row's end ([`Self::end`]).
    pub const fn end(mut self, end: End) -> Self {
        self.end = Some(end);
        self
    }
}

/// A number control's range for the tool's values (decision 30), as darktable's slider
/// has them: what a value may hold, `min..=max`; what the track spans at rest, `soft`,
/// inside it; and what a key moves, `step`. Most are the definition's
/// ([`ProcessingDefinition::range`]); where darktable's `gui_changed` moves a slider's
/// range with other controls, as AgX's pivot spans black to white relative exposure, the
/// tool gives it ([`crate::ProcessingTool::range`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub soft: (f32, f32),
    pub step: f32,
}

impl Range {
    /// Whether `value` lies in `min..=max`.
    pub fn contains(&self, value: f32) -> bool {
        (self.min..=self.max).contains(&value)
    }

    /// `value` at the range's nearer end, if it lies outside it.
    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }

    /// Whether it holds together inside `within`, the definition's range: finite, `min`
    /// below `max`, inside `within`'s `min..=max`, its track in order inside its own, and
    /// its step above 0.
    pub fn holds_inside(&self, within: &Range) -> bool {
        let (low, high) = self.soft;
        [self.min, self.max, low, high, self.step]
            .iter()
            .all(|v| v.is_finite())
            && self.min < self.max
            && within.min <= self.min
            && self.max <= within.max
            && self.min <= low
            && low < high
            && high <= self.max
            && self.step > 0.0
    }
}

/// darktable's step for a slider given none (`dt_bauhaus_slider_get_step`), in the units
/// it shows: from `min` to `max`, the span at rest in the parameter's units, and `factor`,
/// what it shows a parameter by. 1 in the parameter's units where the span's top reaches
/// 100, else a hundredth of it in the units shown, taken to the power of ten at or just
/// below it (within a tenth of a decade above), times 5 where it lies more than half a
/// decade above that power: 0.04 EV gives 0.05, 0.2 % gives 0.1 %. In f32, as darktable
/// computes it.
pub fn slider_step(min: f32, max: f32, factor: f32) -> f32 {
    let top = (max - min).min(min.abs().max(max.abs()));
    let factor = factor.abs();
    if top >= 100.0 {
        return factor;
    }
    let log10step = (top * factor / 100.0).log10();
    let digits = (log10step + 0.1).floor();
    let step = 10f32.powf(digits);
    if log10step - digits > 0.5 {
        step * 5.0
    } else {
        step
    }
}

/// One of darktable's own settings for a tool, as a row ([`Row::Setting`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingRow {
    /// snake_case, unique among the tool's settings: its words follow from it
    /// ([`crate::messages::setting`]) unless it declares its own.
    pub id: &'static str,
    /// A number, a choice or a boolean, with darktable's range, options and default.
    pub kind: ControlKind,
    /// Decimals a number shows.
    pub decimals: u8,
    /// A number's track, as a slider's ([`Slider::track`]).
    pub track: Track,
    /// Its key in the settings in full, where a utility keeps it under a key of its own
    /// rather than a tool's `tools.<tool>.<id>`, as Export's `export.jpeg.quality`
    /// (decision 93). A test holds every key to the settings schema. `None` for a
    /// processing tool's, and for a utility's setting not kept yet, drawn disabled at its
    /// default.
    pub key: Option<&'static str>,
    /// Its words, where it declares its own rather than those its id implies, as Export's
    /// `export-*` (decision 93): a utility's, whose key in full implies no message.
    pub words: Option<&'static SettingWords>,
    /// Its end items, before its value (decision 107); none until a setting's are drawn
    /// ([`DefinitionError::EndNotDrawn`]).
    pub end: &'static [End],
}

/// A line of fields ([`Row::Fields`], decision 107): two or three of a utility's number
/// settings, a separator before each but the first and a unit after it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fields {
    /// snake_case, unique among the tool's fields rows.
    pub id: &'static str,
    /// Its words, the group's name to screen readers, a utility's own message, as Export's
    /// `export-size-pixels`.
    pub words: &'static str,
    pub fields: &'static [Field],
}

/// One field of a line of fields ([`Fields`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    /// The setting it edits, by its key in full, with its kind, decimals and words, as a
    /// setting row has them; its label is the field's name and its folded row's label.
    pub setting: SettingRow,
    /// Drawn before it, after the field before; none on the first.
    pub before: Option<Separator>,
    /// Its unit, in its name to screen readers, and drawn after it, once after the last of
    /// fields side by side that share it, as `[w] × [h] px`.
    pub unit: Option<Unit>,
}

/// What stands between two fields of a line ([`Field::before`]), written alike in every
/// language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separator {
    /// "×", between a width and a height.
    Times,
    /// "@", before a resolution.
    At,
}

impl Separator {
    /// How it is written.
    pub fn text(self) -> &'static str {
        match self {
            Separator::Times => "×",
            Separator::At => "@",
        }
    }
}

impl Fields {
    /// Whether it holds together (decision 107): a snake_case id and words, two or three
    /// fields, each a number setting kept under a key, holding together as a setting row
    /// does, with a separator before each but the first.
    pub(crate) fn check(&self) -> Result<(), DefinitionError> {
        let fails = DefinitionError::Fields(self.id);
        if !is_id(self.id) {
            return Err(DefinitionError::Id(self.id));
        }
        if self.words.is_empty() || !(2..=3).contains(&self.fields.len()) {
            return Err(fails);
        }
        for (at, field) in self.fields.iter().enumerate() {
            let setting = &field.setting;
            let number = matches!(setting.kind, ControlKind::Number { .. });
            if !number || setting.key.is_none() || field.before.is_some() != (at > 0) {
                return Err(fails);
            }
            setting.check()?;
        }
        Ok(())
    }
}

/// The words a setting row declares ([`SettingRow::words`]): message ids, but for an
/// option's, which may be a name written alike in every language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingWords {
    pub label: &'static str,
    /// What screen readers call it; `None` for its label.
    pub full: Option<&'static str>,
    /// Its tooltip; `None` for none.
    pub promise: Option<&'static str>,
    /// A choice's options, one for each, in their order. A boolean's two, No and Yes,
    /// make it darktable's choice of No and Yes (decision 90), false the first; with none
    /// it is a toggle. A number has none.
    pub options: &'static [Word],
}

/// A word a setting row declares ([`SettingWords`]), or one its id implies
/// ([`crate::messages::setting_row_options`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word<T = &'static str> {
    /// A message id.
    Message(T),
    /// A name written alike in every language, as a file format's "JPEG (8-bit)".
    Name(T),
}

impl SettingRow {
    /// Whether it is a boolean drawn as darktable's choice of No and Yes: one whose words
    /// name its two options (decision 90).
    pub fn no_or_yes(&self) -> bool {
        matches!(self.kind, ControlKind::Bool { .. })
            && self.words.is_some_and(|words| words.options.len() == 2)
    }

    /// Its default, darktable's.
    pub fn default(&self) -> ControlValue {
        self.control().default()
    }

    /// It as a control of its kind, which no edit stores.
    fn control(&self) -> Control {
        Control {
            id: self.id,
            kind: self.kind,
            unit: None,
            unported: false,
        }
    }

    /// Whether the setting holds together: its id, and its kind as a control's; curve
    /// points are no setting darktable has; a key in full dotted snake_case parts;
    /// declared words naming each of a choice's options, a boolean's two or none, and none
    /// for a number; and no end items, until a setting's are drawn.
    pub(crate) fn check(&self) -> Result<(), DefinitionError> {
        if !is_id(self.id) {
            return Err(DefinitionError::Id(self.id));
        }
        if !self.end.is_empty() {
            return Err(DefinitionError::EndNotDrawn(self.id));
        }
        if matches!(self.kind, ControlKind::Points { .. }) {
            return Err(DefinitionError::Row(self.id));
        }
        if let Some(key) = self.key
            && !key.split('.').all(is_id)
        {
            return Err(DefinitionError::Key(key));
        }
        if let Some(words) = self.words {
            let named = words.options.len();
            let holds = match self.kind {
                ControlKind::Choice { options, .. } => named == options.len(),
                ControlKind::Bool { .. } => named == 0 || named == 2,
                ControlKind::Number { .. } | ControlKind::Points { .. } => named == 0,
            };
            if !holds {
                return Err(DefinitionError::SettingWords(self.id));
            }
        }
        self.control().check_kind()
    }
}

/// A slider's track: a plain line, or a colour band where colour is the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    Plain,
    Temperature,
    Tint,
    Hue,
    Saturation,
    Luminance,
    /// darktable's ramp of the greys of each CIE L along the track, as it paints a target
    /// lightness's slider (`_paint_hue`, `exposure.c:1123-1151`), for a number in L.
    Lightness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceStyle {
    /// A drop-down list.
    List,
    /// Segments, for a few short options.
    Segmented,
}

/// The design system's painted widgets a definition can name; a third-party tool
/// chooses among them and ships no UI code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Widget {
    Curve,
    ColourWheels,
    /// darktable's tone equalizer graph: its controls are bands in EV, numbers 1 EV apart
    /// up to +0 EV, the darkest first; it draws the tool's curve through them
    /// ([`crate::ProcessingTool::curve`] of the first), and a drag reaches as far as the
    /// curve is wide ([`crate::ProcessingTool::curve_width`]).
    Equalizer,
    /// darktable's colour equalizer graph: its controls are a curve's nodes, numbers
    /// evenly spaced around the hue circle, red first; it draws them over their range,
    /// the tool's curve through them ([`crate::ProcessingTool::curve`] of the first) from
    /// where the first sits ([`crate::ProcessingTool::curve_offset`]), and what the tool
    /// paints behind it ([`crate::ProcessingTool::curve_paint`]).
    HueCurve,
    Swatches,
    /// History's steps, the edit's history as a list (decision 93).
    History,
    /// Scopes' graph with its buttons (decision 93): which graph, the scale, the channels
    /// and the clipping toggles are the graph's own state, kept in the app's settings.
    Scopes,
    /// Collections' top line: what is open, the recents to choose from, and ＋ (decision
    /// 93).
    CollectionsTopLine,
    /// Collections' Folders: its group, holding the open folder's tree in a list that
    /// grows with it up to a cap, then scrolls under the top line (decision 93).
    CollectionsGroups,
    /// The Styles tool's page (decision 106): the styles applied to the active photo, Find
    /// style, and the tree of the person's styles.
    Styles,
    /// Export's Target storage, File on disk the one that can be chosen until the other
    /// storages come, with no setting yet (decision 93).
    ExportStorage,
    /// Export's path line: the path field on one line with the folder button at its end
    /// and no visible label, the resolved file name its tooltip.
    ExportPath,
    /// Export's print and scale lines for Set size's mode, `[w] × [h] cm @ [dpi] dpi` with
    /// no label, or the scale's field; its line in pixels is a [`Row::Fields`].
    ExportSize,
    /// Export's groups still to come, Metadata and Multi-preset export, drawn disabled.
    ExportLater,
    /// Export's end: the Summary, the warnings, the Export button and its job's line.
    ExportEnd,
    /// The Metadata tool's Rating row in Basic, its five stars, drawn disabled until the
    /// global rating actions are built (decision 110).
    MetadataRating,
    /// Its Colour labels row in Basic, darktable's five label dots, drawn disabled the same
    /// way.
    MetadataLabels,
    /// Show on Map after its GPS group's facts, drawn disabled until the map comes.
    MetadataMap,
    /// The Info tool's card: the active photo's name, date, camera, lens, size and its five
    /// exposure values.
    Info,
}

impl Widget {
    /// Whether it is a utility's own widget, drawn by its `UtilityTool::body`, which only a
    /// utility's page shows; the others are a processing tool's, drawn over its controls
    /// (decision 93).
    pub fn is_utility(self) -> bool {
        match self {
            Widget::History
            | Widget::Scopes
            | Widget::CollectionsTopLine
            | Widget::CollectionsGroups
            | Widget::Styles
            | Widget::ExportStorage
            | Widget::ExportPath
            | Widget::ExportSize
            | Widget::ExportLater
            | Widget::ExportEnd
            | Widget::MetadataRating
            | Widget::MetadataLabels
            | Widget::MetadataMap
            | Widget::Info => true,
            Widget::Curve
            | Widget::ColourWheels
            | Widget::Equalizer
            | Widget::HueCurve
            | Widget::Swatches => false,
        }
    }
}

/// A value a control cannot take.
#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ControlError {
    #[error("{control} takes {expected}")]
    WrongKind {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        expected: Id,
    },
    #[error("{control} is {value}, outside {min} to {max}")]
    OutOfRange {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
        value: f64,
        min: f64,
        max: f64,
    },
    #[error("{control} has no option {option}")]
    NoOption {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
        option: String,
    },
    #[error(
        "{control} takes up to {MAX_POINTS} points between its ends, left to right, {MIN_GAP} apart"
    )]
    Points {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
    },
    #[error("{control}: cannot read {text:?}")]
    Invalid {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
        text: String,
    },
    #[error("{control}: {option} is not ported yet")]
    Unported {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        option: Id,
    },
    #[error("{control} is not ported yet: it takes only its default")]
    NotPorted {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
    },
}

/// What is wrong with a definition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DefinitionError {
    #[error("{0:?} is not a snake_case id")]
    Id(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a tool's version starts at 1")]
    Version,
    #[error("two controls are called {0}")]
    DuplicateControl(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0}: its range, step and default do not hold together")]
    Range(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0}: its slider's soft range is not in order inside its range")]
    SoftRange(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "{0}: its options are missing, repeated or not snake_case, or its default is none of them"
    )]
    Options(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0}: an option marked unported is none of its options, or is its default")]
    Unported(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "{0}: its default points are not up to {MAX_POINTS} between its ends, left to right, {MIN_GAP} apart"
    )]
    Points(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0}: only a number has a unit")]
    Unit(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a tool has one untitled tab, or every tab titled, each once")]
    Tabs,
    #[error("a row shows {0}, which the tool lacks")]
    UnknownControl(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "{control} is in {rows} rows; each control is in exactly one, or in one per mode, no two shown at once, or only in the rows of a view covering it, besides one equalizer's at most"
    )]
    Rows {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        control: Id,
        rows: usize,
    },
    #[error("{0} is in a row that cannot show its kind")]
    Row(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0} has options not ported, which only a list draws disabled")]
    Segmented(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two groups, or a group and a line of pickers' label, are called {0}")]
    DuplicateGroup(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the group {0} is inside a group it cannot be in: only a plain group sits inside another group, a collapsible one"
    )]
    GroupInGroup(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the plain group {0} has no rows")]
    EmptyGroup(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two settings are called {0}")]
    DuplicateSetting(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the patches of the picker {0}: their target is not one to three numbers among the tool's settings"
    )]
    UnknownSetting(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two actions are called {0}")]
    DuplicateAction(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    /// `tabs` is the tool's tabs' action, `tool-<tool>-tabs` (decision 98), so no
    /// action end item may take it.
    #[error("{0} is reserved for the tool's tabs")]
    ReservedAction(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the action {0} is called as a control is; one availability rule takes either's id")]
    ActionIsControl(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the action {0}'s menu is no icon button's, or holds fewer than 2 or more than {MOST_IN_SUBMENU} actions"
    )]
    ActionMenu(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the tool publishes {0} twice")]
    DuplicatePublished(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the warning {0} is at the end of a row other than a slider's")]
    WarningPlace(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two warnings are called {0}")]
    DuplicateWarning(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two computed values are called {0}")]
    DuplicateComputed(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "rows follow {0}: not a choice or a toggle of the tool, no value or one it cannot hold, or among those rows"
    )]
    Condition(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "rows follow {0} where none of its rows is sure to show: on their tab, outside its group or outside its modes; on another tab, in a group or a mode"
    )]
    Hidden(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0}: rows following it, with no row")]
    Empty(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "rows follow the setting {0}: not a choice setting of the tool, no option or one it lacks, among those rows, or rows holding a control"
    )]
    SettingCondition(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a preset row is called {0}, as a control or another preset row is")]
    DuplicatePreset(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the preset row {0}: its options are missing, repeated or not snake_case")]
    PresetOptions(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the preset row {0} writes no control, one twice or one the tool lacks")]
    PresetControls(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a row shows the preset row {0}, which the tool lacks")]
    UnknownPreset(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the preset row {0} is segmented; only a list draws Custom and options disabled")]
    SegmentedPreset(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the preset row {preset} is in {rows} rows; each is in one at most")]
    PresetRows {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        preset: Id,
        rows: usize,
    },
    #[error("two views are called {0}")]
    DuplicateView(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the view {0} covers no control, one twice, one the tool lacks or one that is no number"
    )]
    ViewControls(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the view {0} has no coordinate, or more than the controls it covers")]
    ViewCoordinates(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a coordinate is called {0}, as a control, a preset row or another coordinate is")]
    DuplicateCoordinate(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the coordinate {coordinate} is in {rows} rows; each is in one at most")]
    CoordinateRows {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        coordinate: Id,
        rows: usize,
    },
    #[error("two pickers are called {0}")]
    DuplicatePicker(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the picker {0} writes a control the tool lacks, or one twice")]
    PickerWrites(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the picker {0} reads a setting the tool lacks, or one twice")]
    PickerSettings(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the reference row's picker {0} writes controls; a reference's picker only measures")]
    ReferenceWrites(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a last pick row names {0}: no picker of the tool that writes controls, or one twice")]
    LastPick(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the picker {0}'s words follow no choice setting of the tool")]
    WordsBy(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a row shows the picker {0}, which the tool lacks")]
    UnknownPicker(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a line holds no end item")]
    EmptyLine,
    #[error(
        "the row {0} has end items not drawn there yet: a choice and a setting take none, and a slider's is a picker or a warning"
    )]
    EndNotDrawn(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the readout {0}: a line with a label too, no texts, an id named twice or not snake_case"
    )]
    Readout(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the picker {picker} is in {rows} places, rows and preset options; each is in exactly one"
    )]
    PickerRows {
        #[serde(deserialize_with = "apertrust_base::wire::id")]
        picker: Id,
        rows: usize,
    },
    #[error(
        "the preset row {0}: an option turning a picker on is none of its options, is named twice, or names a picker the tool lacks"
    )]
    PresetPickers(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a context menu item names {0}, none of the tool's actions")]
    UnknownMenuAction(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the context menu item {0} is listed twice in its place")]
    DuplicateMenuItem(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("more items in the {0:?} menu than one tool may add there")]
    MenuItems(Place),
    #[error("the context menu's submenu {0} holds no action, or more than {MOST_IN_SUBMENU}")]
    SubmenuItems(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the context menu item {0} is in a section of the app's: a tool's are from EDIT up to LAYOUT"
    )]
    MenuSection(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "the context menu item {0} comes before Reset: a tool's in EDIT take order 200 or more"
    )]
    MenuOrder(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a toolbar item names {0:?}, an empty action or one the tool lacks")]
    UnknownToolbarAction(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the toolbar item {0} is offered twice")]
    DuplicateToolbarItem(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the toolbar item {0} names no icon, which only an icon action leaves to its button")]
    ToolbarIcon(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("more than {MOST_ON_TOOLBAR} toolbar items on a processing tool")]
    ToolbarItems,
    #[error("two cursors are called {0}")]
    DuplicateCursor(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the cursor {0} names no picker of the tool")]
    CursorPicker(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two cursors pick with the picker {0}; a picker has one cursor at most")]
    CursorsOfPicker(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the cursor {0} is in a group other than Pick, a tool's cursors' one group")]
    CursorGroup(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the picker cursor {0} declares gestures, which its picker's kinds decide")]
    CursorGestures(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the cursor {0} acts somewhere other than the Viewer alone")]
    CursorSurfaces(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the picker cursor {0} has amounts, which a picker cursor has none of")]
    CursorOptions(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a utility declares cursors, which a utility has none of until the Colour readout's")]
    UtilityCursors,
    #[error(
        "a utility's row edits, by {0}: a utility holds settings, labels, groups and custom rows naming no control"
    )]
    Edits(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a custom row shows {0:?}, the other kind of tool's widget")]
    Widget(Widget),
    #[error(
        "a utility's action row {0}: its rows name no action until they are bound to the registry"
    )]
    UtilityAction(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("a utility's ↺ is a registry action's, its own or none, never a processing tool's")]
    UtilityHeader,
    #[error(
        "the fields row {0}: not two or three number settings kept under keys, each once, with a separator before each but the first, in a utility"
    )]
    Fields(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the fact {0} is a utility's read-only row, never a processing tool's")]
    Fact(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("two facts are called {0}")]
    DuplicateFact(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the field {0} is a utility's, never a processing tool's")]
    FieldRow(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0:?} is no key of dotted snake_case parts")]
    Key(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the setting {0}'s words name not each of its options")]
    SettingWords(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the setting {0} names a key or words of its own, which only a utility's do")]
    ToolSettingKey(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error(
        "{0} is titled but is no group of the utility's page, or opens at first but is no collapsible group of it"
    )]
    UnknownTitle(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("{0} is given more than one title")]
    DuplicateTitle(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
    #[error("the group {0} opens at first, but has no rows, so it is drawn closed")]
    OpensEmpty(#[serde(deserialize_with = "apertrust_base::wire::id")] Id),
}

/// snake_case ASCII: a lowercase letter, then lowercase letters, digits and single
/// underscores, not ending in one. Message ids join ids with hyphens, so none can
/// contain one.
pub(crate) fn is_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !id.contains("__")
        && !id.ends_with('_')
}

impl ProcessingDefinition {
    pub fn control(&self, id: &str) -> Option<&Control> {
        self.controls.iter().find(|control| control.id == id)
    }

    pub fn preset(&self, id: &str) -> Option<&PresetRow> {
        self.preset_rows.iter().find(|preset| preset.id == id)
    }

    pub fn view(&self, id: &str) -> Option<&'static View> {
        self.views.iter().find(|view| view.id == id)
    }

    pub fn picker(&self, id: &str) -> Option<&'static Picker> {
        self.pickers.iter().find(|picker| picker.id == id)
    }

    /// The readouts of its lines ([`Row::Line`]), in the rows' order.
    pub fn readouts(&self) -> impl Iterator<Item = &ReadoutDef> {
        self.rows().filter_map(|row| match row {
            Row::Line {
                readout: Some(readout),
                ..
            } => Some(readout),
            _ => None,
        })
    }

    /// The coordinate called `id`: its view, and its place among the view's coordinates.
    pub fn coordinate(&self, id: &str) -> Option<(&'static View, usize)> {
        self.views.iter().find_map(|view| {
            let at = view.coordinates.iter().position(|c| c.id == id)?;
            Some((view, at))
        })
    }

    /// A number control's range as the definition gives it (decision 30): its range and
    /// step, and the soft range of the first slider row showing it, else its range, so its
    /// track. `None` for a control that is no number, or that the tool lacks.
    pub fn range(&self, control: &str) -> Option<Range> {
        let ControlKind::Number { min, max, step, .. } = self.control(control)?.kind else {
            return None;
        };
        let soft = self.rows().find_map(|row| match row {
            Row::Slider(slider) if slider.control == control => Some(slider.soft),
            _ => None,
        });
        Some(Range {
            min,
            max,
            soft: soft.flatten().unwrap_or((min, max)),
            step,
        })
    }

    /// Every control at the definition's default.
    pub fn defaults(&self) -> Controls {
        let mut defaults = Controls::new();
        for control in self.controls {
            defaults.set(control.id, control.default());
        }
        defaults
    }

    /// Every computed value's row, in the panel's order, with whether it shows while the
    /// controls hold `values`: each mode it is in holding one of the values it shows for,
    /// as [`Control::text`] writes them.
    pub fn computed(&self, values: &Controls) -> Vec<(&Computed, bool)> {
        self.placed()
            .into_iter()
            .filter_map(|placed| match placed.row {
                Row::Computed(computed) => {
                    let shown = placed.at.modes.iter().all(|(control, is)| {
                        let text = self
                            .control(control)
                            .zip(values.get(control))
                            .map(|(control, value)| control.text(value));
                        text.is_some_and(|text| is.contains(&text.as_str()))
                    });
                    Some((computed, shown))
                }
                _ => None,
            })
            .collect()
    }

    /// Every row in the order the panel lists them, each before the rows inside it: those
    /// before every tab's, the tabs', then those after, with the rows of groups and modes.
    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.placed().into_iter().map(|placed| placed.row)
    }

    /// Every row, as [`Self::rows`], with where it shows: its tab, its group, its modes.
    fn placed(&self) -> Vec<Placed<'_>> {
        self.tool.placed()
    }

    /// Whether the definition holds together: ids, ranges, options, tabs, groups inside
    /// a group only as [`Group`] allows, each plain group with rows, modes of a choice or a
    /// toggle with rows of their own, each
    /// where the control it follows is sure to show, or following a control that another
    /// tab shows outside any collapsible group or mode, each control in exactly one row
    /// that can show it, or in one per mode, no two of them shown at once, or in none
    /// and only in the rows of the views covering it or of a preset row writing it,
    /// besides, for a number, in one equalizer's row at most, the one widget allowed
    /// beside a control's own rows, each preset row with options,
    /// writing controls the tool has, in one row at most and a list, each view
    /// covering number controls of the tool, each once, with a coordinate for each or
    /// fewer,
    /// each coordinate with its range in one slider row at most, each slider's soft range
    /// in order inside its control's or coordinate's range, each setting once, its
    /// kind as a control's, and each action once. Control, preset row and coordinate ids
    /// are unique together. Each picker has an id of its own, writes controls of the
    /// tool and reads its settings, each once, its words following a choice setting of the
    /// tool if any, and is in exactly one place: a row's end item, a reference row, or a
    /// drawn preset row's option that turns it on, an option of the row named once. A
    /// line holds at least one end item, each action at a row's end has a snake_case id
    /// unique among the tool's actions and no control's, a slider's end item is a picker
    /// or a warning, a warning's id snake_case and unique among them and on a slider
    /// alone, and a choice and a setting have none yet. Rows following a setting follow a choice setting of the tool,
    /// shown wherever they are, for options it has, and hold no control and not it. A
    /// reference row's picker writes no control, and its two settings are the tool's,
    /// each once. A last
    /// pick row names a picker of the tool that writes controls, once. Each context menu
    /// item names one of its actions, once in its place, and no place holds more of
    /// them than it takes ([`check_menus`]). Each toolbar item names one of its actions,
    /// once, with an icon unless it is an icon action's, two at most ([`check_toolbar`]). Each cursor has an id of its own and turns
    /// on a picker of the tool, in Pick, no picker with two, on the Viewer alone, with no
    /// gestures and no amounts ([`ToolCursor`]).
    pub fn check(&self) -> Result<(), DefinitionError> {
        if !is_id(self.tool.id) {
            return Err(DefinitionError::Id(self.tool.id));
        }
        if self.version == 0 {
            return Err(DefinitionError::Version);
        }
        let mut published = BTreeSet::new();
        for id in self.publishes {
            if !is_id(id) {
                return Err(DefinitionError::Id(id));
            }
            if !published.insert(*id) {
                return Err(DefinitionError::DuplicatePublished(id));
            }
        }
        let mut ids = BTreeSet::new();
        for control in self.controls {
            if !is_id(control.id) {
                return Err(DefinitionError::Id(control.id));
            }
            if !ids.insert(control.id) {
                return Err(DefinitionError::DuplicateControl(control.id));
            }
            control.check_kind()?;
        }
        for preset in self.preset_rows {
            if !is_id(preset.id) {
                return Err(DefinitionError::Id(preset.id));
            }
            // A preset row's id is in the controls' namespace (`docs/engine.md`, Preset
            // rows and lossless views).
            if !ids.insert(preset.id) {
                return Err(DefinitionError::DuplicatePreset(preset.id));
            }
            self.check_preset(preset)?;
        }
        let mut views = BTreeSet::new();
        for view in self.views {
            if !is_id(view.id) {
                return Err(DefinitionError::Id(view.id));
            }
            if !views.insert(view.id) {
                return Err(DefinitionError::DuplicateView(view.id));
            }
            self.check_view(view)?;
            for coordinate in view.coordinates {
                if !is_id(coordinate.id) {
                    return Err(DefinitionError::Id(coordinate.id));
                }
                // Addressed like a control, so in the controls' namespace too.
                if !ids.insert(coordinate.id) {
                    return Err(DefinitionError::DuplicateCoordinate(coordinate.id));
                }
                if !range_holds(coordinate.min, coordinate.max, coordinate.step) {
                    return Err(DefinitionError::Range(coordinate.id));
                }
            }
        }
        let titled = self.tool.tabs.iter().filter(|tab| tab.id.is_some()).count();
        let tab_ids: BTreeSet<_> = self.tool.tabs.iter().filter_map(|tab| tab.id).collect();
        let tabs_hold = match self.tool.tabs.len() {
            0 => false,
            1 => true,
            n => titled == n && tab_ids.len() == n,
        };
        if !tabs_hold {
            return Err(DefinitionError::Tabs);
        }
        if let Some(id) = tab_ids.iter().find(|id| !is_id(id)) {
            return Err(DefinitionError::Id(id));
        }
        let placed = self.placed();
        let (mut groups, mut settings, mut actions, mut computed) = (
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        let mut last_picks = BTreeSet::new();
        let mut readouts = BTreeSet::new();
        // The actions that run, which a context menu or the toolbar may name.
        let mut runnable = BTreeSet::new();
        let mut warnings = BTreeSet::new();
        for &Placed { row, ref at } in &placed {
            // A group with no rows has nothing behind it yet; rows following a control
            // must be rows.
            if let Row::When {
                control: id, rows, ..
            }
            | Row::WhenSetting {
                setting: id, rows, ..
            } = row
                && rows.is_empty()
            {
                return Err(DefinitionError::Empty(id));
            }
            // A utility's widget is drawn by its own body, which a processing tool has none
            // of (decision 93).
            if let Row::Custom { widget, .. } = row
                && widget.is_utility()
            {
                return Err(DefinitionError::Widget(*widget));
            }
            // A warning is a slider's alone, its id snake_case and the tool's once.
            if let Some(id) = row.end().iter().find_map(End::warning) {
                if !matches!(row, Row::Slider(_)) {
                    return Err(DefinitionError::WarningPlace(id));
                }
                if !is_id(id) {
                    return Err(DefinitionError::Id(id));
                }
                if !warnings.insert(id) {
                    return Err(DefinitionError::DuplicateWarning(id));
                }
            }
            // Each action at a row's end, and each of a menu's, has an id of its own among
            // the tool's actions, never a control's: availability asks one rule with
            // either. A menu is an icon button's, of 2 to `MOST_IN_SUBMENU` actions, and
            // its button runs nothing.
            for ActionEnd { id, face, menu, .. } in row.end().iter().filter_map(End::action) {
                let fits = menu.is_empty()
                    || (matches!(face, Face::Icon(_))
                        && (2..=MOST_IN_SUBMENU).contains(&menu.len()));
                if !fits {
                    return Err(DefinitionError::ActionMenu(id));
                }
                for (id, runs) in std::iter::once((id, menu.is_empty()))
                    .chain(menu.iter().map(|item| (item, true)))
                {
                    if !is_id(id) {
                        return Err(DefinitionError::Id(id));
                    }
                    if *id == "tabs" {
                        return Err(DefinitionError::ReservedAction(id));
                    }
                    if !actions.insert(*id) {
                        return Err(DefinitionError::DuplicateAction(id));
                    }
                    if self.control(id).is_some() {
                        return Err(DefinitionError::ActionIsControl(id));
                    }
                    if runs {
                        runnable.insert(*id);
                    }
                }
            }
            match row {
                Row::Line { start, end, .. } => {
                    if end.is_empty() {
                        return Err(DefinitionError::EmptyLine);
                    }
                    // A line's label shares a plain group's words, so its id is one.
                    if let LineStart::Label(id) = start {
                        if !is_id(id) {
                            return Err(DefinitionError::Id(id));
                        }
                        if !groups.insert(*id) {
                            return Err(DefinitionError::DuplicateGroup(id));
                        }
                    }
                }
                // A slider's one end item is a picker or a warning until the other kinds
                // are drawn there.
                Row::Slider(Slider {
                    control,
                    end: Some(End::Action(_)),
                    ..
                }) => return Err(DefinitionError::EndNotDrawn(control)),
                Row::Choice { control, end, .. } if !end.is_empty() => {
                    return Err(DefinitionError::EndNotDrawn(control));
                }
                Row::Group(group) => {
                    if !is_id(group.id) {
                        return Err(DefinitionError::Id(group.id));
                    }
                    if !groups.insert(group.id) {
                        return Err(DefinitionError::DuplicateGroup(group.id));
                    }
                    group.check()?;
                }
                Row::When { control, is, rows } => {
                    self.check_mode(control, is, rows)?;
                    let shown = placed.iter().any(|its| {
                        its.row.controls().contains(control)
                            && (its.at.shows_wherever(at) || its.at.shows_on_another_tab(at))
                    });
                    if !shown {
                        return Err(DefinitionError::Hidden(control));
                    }
                }
                Row::WhenSetting { setting, is, rows } => {
                    let found = placed.iter().find_map(|its| match its.row {
                        Row::Setting(row) if row.id == *setting => Some((row, &its.at)),
                        _ => None,
                    });
                    let Some((found, its)) = found else {
                        return Err(DefinitionError::SettingCondition(setting));
                    };
                    let ControlKind::Choice { options, .. } = found.kind else {
                        return Err(DefinitionError::SettingCondition(setting));
                    };
                    let holds = !is.is_empty() && is.iter().all(|value| options.contains(value));
                    if !holds || holds_control(rows) || shows_setting(rows, setting) {
                        return Err(DefinitionError::SettingCondition(setting));
                    }
                    if !its.shows_wherever(at) {
                        return Err(DefinitionError::Hidden(setting));
                    }
                }
                Row::Slider(Slider {
                    label: Some(words), ..
                }) if !is_id(words) => return Err(DefinitionError::Id(words)),
                Row::Slider(Slider {
                    control,
                    soft: Some(soft),
                    ..
                }) => {
                    // A control or a coordinate the tool lacks is refused below.
                    if let Some(range) = self.number_range(control)
                        && !soft_holds(*soft, range)
                    {
                        return Err(DefinitionError::SoftRange(control));
                    }
                }
                Row::Preset { preset, .. } if self.preset(preset).is_none() => {
                    return Err(DefinitionError::UnknownPreset(preset));
                }
                Row::Preset {
                    preset,
                    style: ChoiceStyle::Segmented,
                } => return Err(DefinitionError::SegmentedPreset(preset)),
                // A processing tool's fields come with Lens correction's, of controls.
                Row::Fields(fields) => return Err(DefinitionError::Fields(fields.id)),
                // Facts are a utility's, never the edit's.
                Row::Fact(fact) => return Err(DefinitionError::Fact(fact.id)),
                Row::Field(field) => return Err(DefinitionError::FieldRow(field.id)),
                Row::Setting(setting) => {
                    if !settings.insert(setting.id) {
                        return Err(DefinitionError::DuplicateSetting(setting.id));
                    }
                    // A processing tool's are its own, `tools.<tool>.<id>`, with the words
                    // their ids imply.
                    if setting.key.is_some() || setting.words.is_some() {
                        return Err(DefinitionError::ToolSettingKey(setting.id));
                    }
                    setting.check()?;
                }
                // Its two settings are the tool's, each once, and its picker writes no
                // control.
                Row::Reference {
                    picker,
                    lightness,
                    photo,
                } => {
                    for id in [lightness, photo] {
                        if !is_id(id) {
                            return Err(DefinitionError::Id(id));
                        }
                        if !settings.insert(*id) {
                            return Err(DefinitionError::DuplicateSetting(id));
                        }
                    }
                    if self.picker(picker).is_some_and(|p| !p.writes.is_empty()) {
                        return Err(DefinitionError::ReferenceWrites(picker));
                    }
                }
                // What it shows is what its picker wrote: one of the tool's that writes.
                Row::LastPick { picker } => {
                    if !self.picker(picker).is_some_and(|p| !p.writes.is_empty()) {
                        return Err(DefinitionError::LastPick(picker));
                    }
                    if !last_picks.insert(*picker) {
                        return Err(DefinitionError::LastPick(picker));
                    }
                }
                Row::Computed(Computed { id, .. }) => {
                    if !is_id(id) {
                        return Err(DefinitionError::Id(id));
                    }
                    if !computed.insert(*id) {
                        return Err(DefinitionError::DuplicateComputed(id));
                    }
                }
                _ => {}
            }
            // A readout stands in the label's place, its id and its texts' ids its words'.
            if let Row::Line {
                start,
                readout: Some(readout),
                ..
            } = row
            {
                let ReadoutDef { id, texts, .. } = *readout;
                let named =
                    |text: &&str| is_id(text) && texts.iter().filter(|t| *t == text).count() == 1;
                if matches!(start, LineStart::Label(_))
                    || !is_id(id)
                    || texts.is_empty()
                    || !texts.iter().all(named)
                    || !readouts.insert(id)
                {
                    return Err(DefinitionError::Readout(id));
                }
            }
            if let Row::Patches { picker, target } = row {
                if self.picker(picker).is_none() {
                    return Err(DefinitionError::UnknownPicker(picker));
                }
                // Its target's L, then C and h, each a number among the settings.
                let number = |id: &&str| {
                    placed.iter().any(|its| {
                        matches!(its.row, Row::Setting(setting)
                            if setting.id == *id
                                && matches!(setting.kind, ControlKind::Number { .. }))
                    })
                };
                if target.is_empty() || target.len() > 3 || !target.iter().all(number) {
                    return Err(DefinitionError::UnknownSetting(picker));
                }
            }
            if let Some(picker) = row.pickers().find(|id| self.picker(id).is_none()) {
                return Err(DefinitionError::UnknownPicker(picker));
            }
            for &id in row.controls() {
                let Some(control) = self.control(id) else {
                    // A view's coordinate, which only a slider shows.
                    match (row, self.coordinate(id)) {
                        (Row::Slider(_), Some(_)) => continue,
                        (_, Some(_)) => return Err(DefinitionError::Row(id)),
                        (_, None) => return Err(DefinitionError::UnknownControl(id)),
                    }
                };
                let fits = match (row, control.kind) {
                    (Row::Slider(_), ControlKind::Number { .. })
                    | (Row::Toggle { .. }, ControlKind::Bool { .. })
                    | (
                        Row::Custom {
                            widget: Widget::Equalizer | Widget::HueCurve,
                            ..
                        },
                        ControlKind::Number { .. },
                    ) => true,
                    (
                        Row::Custom {
                            widget: Widget::Equalizer | Widget::HueCurve,
                            ..
                        },
                        _,
                    ) => false,
                    (Row::Custom { .. }, _) => true,
                    // Its actions write it, whatever its kind.
                    (Row::Line { .. }, _) => true,
                    (Row::Choice { style, .. }, ControlKind::Choice { unported, .. }) => {
                        if *style == ChoiceStyle::Segmented && !unported.is_empty() {
                            return Err(DefinitionError::Segmented(id));
                        }
                        true
                    }
                    _ => false,
                };
                if !fits {
                    return Err(DefinitionError::Row(id));
                }
            }
        }
        let rows_of = |id: &str| -> Vec<&Placed> {
            placed
                .iter()
                .filter(|placed| placed.row.controls().contains(&id))
                .collect()
        };
        for control in self.controls {
            let all = rows_of(control.id);
            // A band graph over controls with rows of their own is a second view of them,
            // the tone equalizer's graph of its bands or the colour equalizer's of its
            // nodes, so only their own rows count; one at most. They are the only widgets
            // allowed so.
            let (custom, own): (Vec<&Placed>, Vec<&Placed>) = all.iter().partition(|placed| {
                matches!(
                    placed.row,
                    Row::Custom {
                        widget: Widget::Equalizer | Widget::HueCurve,
                        ..
                    }
                )
            });
            if custom.len() > 1 {
                return Err(DefinitionError::Rows {
                    control: control.id,
                    rows: all.len(),
                });
            }
            let rows = if own.is_empty() { custom } else { own };
            let apart = rows
                .iter()
                .enumerate()
                .all(|(i, a)| rows[i + 1..].iter().all(|b| a.apart(b)));
            // A control in no row of its own shows through the views covering it, or
            // through a preset row that writes it, as White balance's `preset` does
            // through Settings (decision 27).
            let viewed = || {
                self.views
                    .iter()
                    .filter(|view| view.controls.contains(&control.id))
                    .flat_map(|view| view.coordinates)
                    .any(|coordinate| !rows_of(coordinate.id).is_empty())
            };
            let preset_shown = || {
                self.preset_rows
                    .iter()
                    .filter(|preset| preset.controls.contains(&control.id))
                    .any(|preset| {
                        placed.iter().any(|placed| {
                            matches!(placed.row, Row::Preset { preset: id, .. } if *id == preset.id)
                        })
                    })
            };
            if (rows.is_empty() && !viewed() && !preset_shown()) || !apart {
                return Err(DefinitionError::Rows {
                    control: control.id,
                    rows: rows.len(),
                });
            }
        }
        for coordinate in self.views.iter().flat_map(|view| view.coordinates) {
            let rows = rows_of(coordinate.id).len();
            if rows > 1 {
                return Err(DefinitionError::CoordinateRows {
                    coordinate: coordinate.id,
                    rows,
                });
            }
        }
        for preset in self.preset_rows {
            let rows = placed
                .iter()
                .filter(|placed| match placed.row {
                    Row::Preset { preset: id, .. } => *id == preset.id,
                    _ => false,
                })
                .count();
            if rows > 1 {
                return Err(DefinitionError::PresetRows {
                    preset: preset.id,
                    rows,
                });
            }
        }
        let mut pickers = BTreeSet::new();
        for picker in self.pickers {
            if !is_id(picker.id) {
                return Err(DefinitionError::Id(picker.id));
            }
            if !pickers.insert(picker.id) {
                return Err(DefinitionError::DuplicatePicker(picker.id));
            }
            self.check_picker(picker, &settings)?;
            // Counted where they are named, so a row naming one twice shows it twice; a
            // drawn preset row's option that turns it on is a place too.
            let in_rows = placed
                .iter()
                .flat_map(|placed| placed.row.pickers())
                .filter(|&id| id == picker.id)
                .count();
            let by_options = self
                .preset_rows
                .iter()
                .filter(|preset| {
                    placed.iter().any(|placed| {
                        matches!(placed.row, Row::Preset { preset: id, .. } if *id == preset.id)
                    })
                })
                .flat_map(|preset| preset.pickers)
                .filter(|(_, id)| *id == picker.id)
                .count();
            let rows = in_rows + by_options;
            if rows != 1 {
                return Err(DefinitionError::PickerRows {
                    picker: picker.id,
                    rows,
                });
            }
        }
        for item in self.tool.menus {
            let entry = item.entry.actions();
            if let Some(action) = entry.iter().find(|action| !runnable.contains(*action)) {
                return Err(DefinitionError::UnknownMenuAction(action));
            }
            check_section(item)?;
        }
        check_menus(self.tool.menus)?;
        let unknown = (self.tool.toolbar.iter()).find(|item| !runnable.contains(item.action));
        if let Some(item) = unknown {
            return Err(DefinitionError::UnknownToolbarAction(item.action));
        }
        let buttons: Vec<&str> = (self.rows())
            .flat_map(|row| row.end().iter().filter_map(End::action))
            .filter(|action| matches!(action.face, Face::Icon(_)) && action.menu.is_empty())
            .map(|action| action.id)
            .collect();
        check_toolbar(self.tool.toolbar, Some(MOST_ON_TOOLBAR), &buttons)?;
        self.check_cursors()
    }

    /// Whether a picker's writes are controls of the tool and the settings it reads
    /// are the tool's, `settings` being their ids, each once.
    fn check_picker(
        &self,
        picker: &Picker,
        settings: &BTreeSet<&str>,
    ) -> Result<(), DefinitionError> {
        let writes: BTreeSet<_> = picker.writes.iter().collect();
        let writes_hold = writes.len() == picker.writes.len()
            && picker.writes.iter().all(|id| self.control(id).is_some());
        if !writes_hold {
            return Err(DefinitionError::PickerWrites(picker.id));
        }
        let reads: BTreeSet<_> = picker.settings.iter().collect();
        let reads_hold = reads.len() == picker.settings.len()
            && picker.settings.iter().all(|id| settings.contains(id));
        if !reads_hold {
            return Err(DefinitionError::PickerSettings(picker.id));
        }
        if let Some(words_by) = picker.words_by
            && (self.choice_setting(words_by).is_none() || !reads.contains(&words_by))
        {
            return Err(DefinitionError::WordsBy(picker.id));
        }
        Ok(())
    }

    /// Every action at its rows' ends, in the page's order (decision 107).
    pub fn action_ends(&self) -> impl Iterator<Item = &ActionEnd> {
        self.rows()
            .flat_map(|row| row.end().iter().filter_map(End::action))
    }

    /// Every warning at its sliders' ends, in the page's order (decision 107).
    pub fn warnings(&self) -> Vec<&'static str> {
        (self.rows())
            .flat_map(|row| row.end().iter().filter_map(End::warning))
            .collect()
    }

    /// Every action of the tool with whether it is not ported, in the page's order: each
    /// action at its rows' ends, a menu button's own and then each of its menu's, which is
    /// as ported as its button (decision 107).
    pub fn action_ids(&self) -> Vec<(&'static str, bool)> {
        let mut ids = Vec::new();
        for end in self.action_ends() {
            ids.push((end.id, end.unported));
            ids.extend(end.menu.iter().map(|item| (*item, end.unported)));
        }
        ids
    }

    /// The facts the action `id`, at a row's end or in a menu, reads ([`ActionEnd::reads`]).
    pub fn reads(&self, id: &str) -> &'static [Published] {
        let found = self
            .action_ends()
            .find(|end| end.id == id || end.menu.contains(&id));
        found.map_or(&[], |end| end.reads)
    }

    /// The tool's actions that run, with whether each is not ported, in the page's order:
    /// each action at its rows' ends with no menu, and each item of a menu in its button's
    /// place, the button running nothing (decision 107).
    pub fn own_actions(&self) -> Vec<(&'static str, bool)> {
        let mut ids = Vec::new();
        for end in self.action_ends() {
            match end.menu {
                [] => ids.push((end.id, end.unported)),
                items => ids.extend(items.iter().map(|item| (*item, end.unported))),
            }
        }
        ids
    }

    /// The options of the tool's choice setting `id` ([`Row::Setting`]), if it has one.
    pub fn choice_setting(&self, id: &str) -> Option<&'static [&'static str]> {
        self.rows().find_map(|row| match row {
            Row::Setting(SettingRow {
                id: found,
                kind: ControlKind::Choice { options, .. },
                ..
            }) if *found == id => Some(*options),
            _ => None,
        })
    }

    /// The range of a number control or a coordinate called `id`.
    fn number_range(&self, id: &str) -> Option<(f32, f32)> {
        match self.control(id).map(|control| control.kind) {
            Some(ControlKind::Number { min, max, .. }) => Some((min, max)),
            Some(_) => None,
            None => self.coordinate(id).map(|(view, at)| {
                let coordinate = &view.coordinates[at];
                (coordinate.min, coordinate.max)
            }),
        }
    }

    /// Whether a preset row holds together: options that are ids, each once, and controls
    /// of the tool, each once.
    fn check_preset(&self, preset: &PresetRow) -> Result<(), DefinitionError> {
        let options: BTreeSet<_> = preset.options.iter().collect();
        let options_hold = !preset.options.is_empty()
            && options.len() == preset.options.len()
            && preset.options.iter().all(|option| is_id(option));
        if !options_hold {
            return Err(DefinitionError::PresetOptions(preset.id));
        }
        let controls: BTreeSet<_> = preset.controls.iter().collect();
        let controls_hold = !preset.controls.is_empty()
            && controls.len() == preset.controls.len()
            && preset.controls.iter().all(|id| self.control(id).is_some());
        if !controls_hold {
            return Err(DefinitionError::PresetControls(preset.id));
        }
        // An option that turns a picker on is one of the row's, once, and names a picker
        // of the tool.
        let turning: BTreeSet<_> = preset.pickers.iter().map(|(option, _)| option).collect();
        let pickers_hold = turning.len() == preset.pickers.len()
            && preset.pickers.iter().all(|(option, picker)| {
                preset.options.contains(option) && self.picker(picker).is_some()
            });
        if !pickers_hold {
            return Err(DefinitionError::PresetPickers(preset.id));
        }
        Ok(())
    }

    /// Whether a view holds together: number controls of the tool, each once, and a
    /// coordinate for each, so it can be an invertible change of their coordinates, or
    /// fewer, where the tool's model drops a scale of them, as White balance's
    /// temperature and tint drop the coefficients' (decision 27).
    fn check_view(&self, view: &View) -> Result<(), DefinitionError> {
        let controls: BTreeSet<_> = view.controls.iter().collect();
        let numbers = view.controls.iter().all(|id| {
            self.control(id)
                .is_some_and(|control| matches!(control.kind, ControlKind::Number { .. }))
        });
        if view.controls.is_empty() || controls.len() != view.controls.len() || !numbers {
            return Err(DefinitionError::ViewControls(view.id));
        }
        if view.coordinates.is_empty() || view.coordinates.len() > view.controls.len() {
            return Err(DefinitionError::ViewCoordinates(view.id));
        }
        Ok(())
    }

    /// Whether rows can follow `control` for the values `is` names: a choice or a toggle
    /// of the tool, values it holds, and none of the rows its own.
    fn check_mode(
        &self,
        control: &'static str,
        is: &[&str],
        rows: &[Row],
    ) -> Result<(), DefinitionError> {
        let values: &[&str] = match self.control(control).map(|found| found.kind) {
            Some(ControlKind::Choice { options, .. }) => options,
            Some(ControlKind::Bool { .. }) => &["false", "true"],
            _ => return Err(DefinitionError::Condition(control)),
        };
        let holds = !is.is_empty() && is.iter().all(|value| values.contains(value));
        if !holds || shows(rows, control) {
            return Err(DefinitionError::Condition(control));
        }
        Ok(())
    }
}

/// A row, and where its definition puts it.
struct Placed<'d> {
    row: &'d Row,
    at: Where,
}

/// Where a row shows: its tab, `None` for the rows before and after every tab's; its
/// collapsible group, a plain one being always open; each mode it shows in, as the control it follows and the values it
/// shows for, outermost first; and each setting's options it shows for
/// ([`Row::WhenSetting`]), kept apart, as settings and controls are.
#[derive(Clone, Default)]
struct Where {
    tab: Option<usize>,
    group: Option<&'static str>,
    modes: Vec<Condition>,
    settings: Vec<Condition>,
}

/// A control or a setting rows follow, and the values they show for.
type Condition = (&'static str, &'static [&'static str]);

impl Placed<'_> {
    /// Whether the two rows are never shown at once: one follows a control for values
    /// the other follows it for none of.
    fn apart(&self, other: &Placed) -> bool {
        let apart = |ours: &[Condition], theirs: &[Condition]| {
            ours.iter().any(|(control, is)| {
                theirs
                    .iter()
                    .any(|(other, its)| control == other && is.iter().all(|v| !its.contains(v)))
            })
        };
        apart(&self.at.modes, &other.at.modes) || apart(&self.at.settings, &other.at.settings)
    }
}

impl Where {
    /// Whether a row here shows whenever one at `other` does: on every tab or on its tab,
    /// outside any group or in its group, and in each of these modes because `other` is
    /// in it too, for some of its values only.
    fn shows_wherever(&self, other: &Where) -> bool {
        let tab = self.tab.is_none() || self.tab == other.tab;
        let group = self.group.is_none() || self.group == other.group;
        let within = |ours: &[Condition], theirs: &[Condition]| {
            ours.iter().all(|(control, is)| {
                theirs
                    .iter()
                    .any(|(other, its)| control == other && its.iter().all(|v| is.contains(v)))
            })
        };
        tab && group && within(&self.modes, &other.modes) && within(&self.settings, &other.settings)
    }

    /// Whether a row here is on a tab other than `other`'s and shows whenever its own tab
    /// is open, outside any group or mode: a control that rows on another tab may follow,
    /// as darktable's tone equalizer shows its contrast compensation, on the Advanced tab,
    /// only for some of the filters chosen on the Masking tab.
    fn shows_on_another_tab(&self, other: &Where) -> bool {
        self.tab.is_some()
            && self.tab != other.tab
            && self.group.is_none()
            && self.modes.is_empty()
            && self.settings.is_empty()
    }
}

/// Places `rows` and the rows inside them, at `at` and inside the groups and modes they
/// are in.
fn place<'d>(rows: &'d [Row], at: &Where, placed: &mut Vec<Placed<'d>>) {
    for row in rows {
        placed.push(Placed {
            row,
            at: at.clone(),
        });
        let mut inside = at.clone();
        match row {
            // A plain group is always open, so only a collapsible one hides its rows.
            Row::Group(Group {
                id,
                kind: GroupKind::Collapsible,
                ..
            }) => inside.group = Some(id),
            Row::When { control, is, .. } => inside.modes.push((control, is)),
            Row::WhenSetting { setting, is, .. } => inside.settings.push((setting, is)),
            _ => {}
        }
        place(row.rows(), &inside, placed);
    }
}

impl Group {
    /// Whether the group holds together (decision 100): a plain one with rows, and no group
    /// inside it that cannot be there, any group in a plain one and a collapsible one in a
    /// collapsible one.
    pub(crate) fn check(&self) -> Result<(), DefinitionError> {
        if self.kind == GroupKind::Plain && self.rows.is_empty() {
            return Err(DefinitionError::EmptyGroup(self.id));
        }
        match misplaced(self.kind, self.rows) {
            Some(inner) => Err(DefinitionError::GroupInGroup(inner)),
            None => Ok(()),
        }
    }
}

/// The first group among `rows` or inside them that cannot sit in a group of `kind`.
fn misplaced(kind: GroupKind, rows: &[Row]) -> Option<&'static str> {
    rows.iter().find_map(|row| match row {
        Row::Group(inner) if kind == GroupKind::Plain || inner.kind == GroupKind::Collapsible => {
            Some(inner.id)
        }
        _ => misplaced(kind, row.rows()),
    })
}

/// Whether `rows`, or the rows inside them, show `control`.
fn shows(rows: &[Row], control: &str) -> bool {
    rows.iter()
        .any(|row| row.controls().contains(&control) || shows(row.rows(), control))
}

/// Whether `rows`, or the rows inside them, show any control or a preset row, which writes
/// controls.
fn holds_control(rows: &[Row]) -> bool {
    rows.iter().any(|row| {
        matches!(row, Row::Preset { .. }) || !row.controls().is_empty() || holds_control(row.rows())
    })
}

/// Whether `rows`, or the rows inside them, show the setting `setting`.
pub(crate) fn shows_setting(rows: &[Row], setting: &str) -> bool {
    rows.iter().any(|row| {
        matches!(row, Row::Setting(found) if found.id == setting)
            || shows_setting(row.rows(), setting)
    })
}

/// Whether a number's range holds together: finite ends in order, and a step that fits.
fn range_holds(min: f32, max: f32, step: f32) -> bool {
    min.is_finite() && max.is_finite() && min < max && step > 0.0 && step <= max - min
}

/// Whether a slider's soft range holds together: finite ends in order, inside the range,
/// as darktable's `dt_bauhaus_slider_set_soft_range` keeps it.
fn soft_holds((low, high): (f32, f32), (min, max): (f32, f32)) -> bool {
    low.is_finite() && high.is_finite() && min <= low && low < high && high <= max
}

/// Whether `points` lie between a curve's ends: up to [`MAX_POINTS`], each coordinate
/// from 0 to 1, x increasing by at least [`MIN_GAP`] from the end at 0, through the
/// points, to the end at 1, measured in f32 as the curve widget measures it.
fn points_hold(points: &[[f32; 2]]) -> bool {
    let xs: Vec<f32> = std::iter::once(0.0)
        .chain(points.iter().map(|point| point[0]))
        .chain(std::iter::once(1.0))
        .collect();
    points.len() <= MAX_POINTS
        && points
            .iter()
            .flatten()
            .all(|coordinate| (0.0..=1.0).contains(coordinate))
        && xs.windows(2).all(|pair| pair[1] - pair[0] >= MIN_GAP)
}

impl Control {
    pub fn default(&self) -> ControlValue {
        match self.kind {
            ControlKind::Number { default, .. } => ControlValue::Number(default),
            ControlKind::Choice { default, .. } => ControlValue::Choice(default),
            ControlKind::Bool { default } => ControlValue::Bool(default),
            ControlKind::Points { default } => ControlValue::Points(default.to_vec()),
        }
    }

    /// Whether the kind holds together, for [`ProcessingDefinition::check`].
    fn check_kind(&self) -> Result<(), DefinitionError> {
        let id = self.id;
        match self.kind {
            ControlKind::Number {
                min,
                max,
                step,
                default,
            } => {
                if !range_holds(min, max, step) || !(min..=max).contains(&default) {
                    return Err(DefinitionError::Range(id));
                }
            }
            ControlKind::Choice {
                options,
                default,
                unported,
            } => {
                let unique: BTreeSet<_> = options.iter().collect();
                let holds = !options.is_empty()
                    && unique.len() == options.len()
                    && options.iter().all(|option| is_id(option))
                    && (default as usize) < options.len();
                if !holds {
                    return Err(DefinitionError::Options(id));
                }
                let marked = unported.iter().all(|option| options.contains(option))
                    && !unported.contains(&options[default as usize]);
                if !marked {
                    return Err(DefinitionError::Unported(id));
                }
            }
            ControlKind::Bool { .. } => {}
            ControlKind::Points { default } => {
                if !points_hold(default) {
                    return Err(DefinitionError::Points(id));
                }
            }
        }
        if self.unit.is_some() && !matches!(self.kind, ControlKind::Number { .. }) {
            return Err(DefinitionError::Unit(id));
        }
        Ok(())
    }

    /// Whether `value` has this control's kind and lies in its range: for a choice, one
    /// of its options that is ported; for a control that is not ported, its default.
    pub fn check(&self, value: &ControlValue) -> Result<(), ControlError> {
        self.check_kind_of(value)?;
        if self.unported && *value != self.default() {
            return Err(ControlError::NotPorted { control: self.id });
        }
        Ok(())
    }

    /// Whether `value` has this control's kind and lies in its range: for a choice, one
    /// of its options that is ported.
    fn check_kind_of(&self, value: &ControlValue) -> Result<(), ControlError> {
        let control = self.id;
        match (self.kind, value) {
            (ControlKind::Number { min, max, .. }, ControlValue::Number(value)) => {
                // Also refuses NaN, which no range contains.
                if (min..=max).contains(value) {
                    Ok(())
                } else {
                    Err(ControlError::OutOfRange {
                        control,
                        value: f64::from(*value),
                        min: f64::from(min),
                        max: f64::from(max),
                    })
                }
            }
            (
                ControlKind::Choice {
                    options, unported, ..
                },
                ControlValue::Choice(index),
            ) => match options.get(*index as usize) {
                Some(option) if unported.contains(option) => {
                    Err(ControlError::Unported { control, option })
                }
                Some(_) => Ok(()),
                None => Err(ControlError::NoOption {
                    control,
                    option: index.to_string(),
                }),
            },
            (ControlKind::Bool { .. }, ControlValue::Bool(_)) => Ok(()),
            (ControlKind::Points { .. }, ControlValue::Points(points)) => {
                if points_hold(points) {
                    Ok(())
                } else {
                    Err(ControlError::Points { control })
                }
            }
            (kind, _) => Err(ControlError::WrongKind {
                control,
                expected: match kind {
                    ControlKind::Number { .. } => "a number",
                    ControlKind::Choice { .. } => "one of its options",
                    ControlKind::Bool { .. } => "true or false",
                    ControlKind::Points { .. } => "points",
                },
            }),
        }
    }

    /// Reads a value from text, as edits and the CLI write it: a number, an option by id
    /// or index, `true` or `false`, or points as `x,y` pairs separated by spaces. The
    /// result still needs [`Self::check`].
    pub fn parse(&self, text: &str) -> Result<ControlValue, ControlError> {
        let control = self.id;
        let invalid = || ControlError::Invalid {
            control,
            text: text.to_owned(),
        };
        let text = text.trim();
        match self.kind {
            ControlKind::Number { .. } => text
                .parse()
                .map(ControlValue::Number)
                .map_err(|_| invalid()),
            ControlKind::Choice { options, .. } => options
                .iter()
                .position(|option| option.eq_ignore_ascii_case(text))
                .map(|index| index as u32)
                .or_else(|| text.parse().ok())
                .map(ControlValue::Choice)
                .ok_or_else(|| ControlError::NoOption {
                    control,
                    option: text.to_owned(),
                }),
            ControlKind::Bool { .. } => text.parse().map(ControlValue::Bool).map_err(|_| invalid()),
            ControlKind::Points { .. } => text
                .split_whitespace()
                .map(|pair| {
                    let (x, y) = pair.split_once(',')?;
                    Some([x.parse().ok()?, y.parse().ok()?])
                })
                .collect::<Option<Vec<_>>>()
                .map(ControlValue::Points)
                .ok_or_else(invalid),
        }
    }

    /// The value as text that [`Self::parse`] reads back exactly: a number in its
    /// shortest exact form, an option by its id, `true` or `false`, points as `x,y`
    /// pairs.
    pub fn text(&self, value: &ControlValue) -> String {
        match (self.kind, value) {
            (ControlKind::Choice { options, .. }, ControlValue::Choice(index)) => options
                .get(*index as usize)
                .map_or_else(|| index.to_string(), |option| (*option).to_owned()),
            (_, ControlValue::Number(value)) => value.to_string(),
            (_, ControlValue::Choice(index)) => index.to_string(),
            (_, ControlValue::Bool(value)) => value.to_string(),
            (_, ControlValue::Points(points)) => points
                .iter()
                .map(|[x, y]| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// Whether `value` is one of `values`, as this control writes them ([`Self::text`]):
    /// whether the rows of a [`Row::When`] following this control show.
    pub fn is_one_of(&self, value: &ControlValue, values: &[&str]) -> bool {
        values.contains(&self.text(value).as_str())
    }

    /// A value read from a saved edit, made usable as decision 21 asks: a number or a
    /// point's coordinate outside its range is brought to its nearer end. An option the
    /// control lacks or does not port, a value of the wrong kind, NaN and points out of
    /// order stay refused.
    pub fn clamp(&self, value: ControlValue) -> Result<ControlValue, ControlError> {
        let value = match (self.kind, value) {
            (ControlKind::Number { min, max, .. }, ControlValue::Number(v)) if !v.is_nan() => {
                ControlValue::Number(v.clamp(min, max))
            }
            (ControlKind::Points { .. }, ControlValue::Points(points)) => ControlValue::Points(
                points
                    .into_iter()
                    .map(|point| point.map(|v| if v.is_nan() { v } else { v.clamp(0.0, 1.0) }))
                    .collect(),
            ),
            (_, value) => value,
        };
        self.check(&value).map(|()| value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of one text action not ported, `id`, as AgX's Read exposure.
    macro_rules! action_line {
        ($id:expr) => {
            Row::Line {
                start: LineStart::None,
                readout: None,
                end: &[End::Action(ActionEnd {
                    id: $id,
                    face: Face::Text,
                    menu: &[],
                    reads: &[],
                    unported: true,
                })],
            }
        };
    }

    /// An icon action that runs, `id`, as Orientation's buttons.
    const fn button(id: &'static str) -> End {
        End::Action(ActionEnd {
            id,
            face: Face::Icon(ButtonIcon::RotateLeft),
            menu: &[],
            reads: &[],
            unported: false,
        })
    }

    static CONTROLS: &[Control] = &[
        Control {
            id: "exposure",
            kind: ControlKind::Number {
                min: -3.0,
                max: 4.0,
                step: 0.01,
                default: 0.0,
            },
            unit: Some(Unit::Ev),
            unported: false,
        },
        Control {
            id: "mode",
            kind: ControlKind::Choice {
                options: &["soft", "hard"],
                default: 0,
                unported: &[],
            },
            unit: None,
            unported: false,
        },
        Control {
            id: "preserve",
            kind: ControlKind::Bool { default: true },
            unit: None,
            unported: false,
        },
        Control {
            id: "curve",
            kind: ControlKind::Points { default: &[] },
            unit: None,
            unported: false,
        },
    ];

    static ROWS: &[Row] = &[
        Row::Slider(Slider::new("exposure")),
        Row::Group(Group {
            id: "look",
            kind: GroupKind::Plain,
            rows: &[
                Row::Choice {
                    control: "mode",
                    style: ChoiceStyle::Segmented,
                    end: &[],
                },
                Row::Toggle {
                    control: "preserve",
                    end: &[],
                },
                Row::Custom {
                    widget: Widget::Curve,
                    controls: &["curve"],
                },
            ],
        }),
    ];

    static DEFINITION: ProcessingDefinition = ProcessingDefinition {
        tool: ToolDefinition {
            id: "look",
            tabs: &[Tab {
                id: None,
                rows: ROWS,
            }],
            before: &[],
            after: &[],
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
        controls: CONTROLS,
        preset_rows: &[],
        views: &[],
        pickers: &[],
        facts: None,
    };

    fn control(id: &str) -> &'static Control {
        DEFINITION.control(id).unwrap()
    }

    /// A number from 0 to 1, at 0.
    const fn number(id: &'static str) -> Control {
        Control {
            id,
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        }
    }

    /// A page as darktable draws one, as local contrast and demosaic do: a mode whose
    /// third option is not ported, a parameter under another label in each mode, a
    /// collapsible group with rows shown while a toggle is on, and rows before and after
    /// every tab's.
    static PAGE_CONTROLS: &[Control] = &[
        number("amount"),
        Control {
            id: "mode",
            kind: ControlKind::Choice {
                options: &["bilateral", "laplacian", "neural"],
                default: 0,
                unported: &["neural"],
            },
            unit: None,
            unported: false,
        },
        number("detail"),
        number("contrast"),
        Control {
            id: "smooth",
            kind: ControlKind::Bool { default: false },
            unit: None,
            unported: false,
        },
        number("radius"),
        Control {
            id: "mask",
            kind: ControlKind::Bool { default: false },
            unit: None,
            unported: false,
        },
    ];

    const MODE: Row = Row::Choice {
        control: "mode",
        style: ChoiceStyle::List,
        end: &[],
    };

    /// `contrast`'s row, and its row under its other label.
    const CONTRAST: &[Row] = &[Row::Slider(Slider::new("contrast"))];
    const HIGHLIGHTS: &[Row] = &[Row::Slider(Slider::new("contrast").label("highlights"))];

    static PER_MODE: &[Row] = &[
        Row::When {
            control: "mode",
            is: &["bilateral"],
            rows: CONTRAST,
        },
        Row::When {
            control: "mode",
            is: &["laplacian", "neural"],
            rows: HIGHLIGHTS,
        },
    ];

    const FINE_TUNING: Row = Row::Group(Group {
        id: "fine_tuning",
        kind: GroupKind::Collapsible,
        rows: &[
            Row::Toggle {
                control: "smooth",
                end: &[],
            },
            Row::When {
                control: "smooth",
                is: &["true"],
                rows: &[Row::Slider(Slider::new("radius"))],
            },
        ],
    });

    static PAGE_TABS: &[Tab] = &[
        Tab {
            id: Some("main"),
            rows: &[
                MODE,
                Row::Slider(Slider::new("detail")),
                Row::When {
                    control: "mode",
                    is: &["bilateral"],
                    rows: CONTRAST,
                },
                Row::When {
                    control: "mode",
                    is: &["laplacian", "neural"],
                    rows: HIGHLIGHTS,
                },
            ],
        },
        Tab {
            id: Some("fine"),
            rows: &[FINE_TUNING],
        },
    ];

    static PAGE: ProcessingDefinition = ProcessingDefinition {
        tool: ToolDefinition {
            id: "page",
            tabs: PAGE_TABS,
            before: &[Row::Slider(Slider::new("amount"))],
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
        controls: PAGE_CONTROLS,
        preset_rows: &[],
        views: &[],
        pickers: &[],
        facts: None,
    };

    /// [`PAGE`] with its first tab's rows in place of its own.
    fn page(rows: &'static [Row]) -> ProcessingDefinition {
        ProcessingDefinition {
            tool: ToolDefinition {
                tabs: Box::leak(Box::new([
                    Tab {
                        id: Some("main"),
                        rows,
                    },
                    Tab {
                        id: Some("fine"),
                        rows: &[FINE_TUNING],
                    },
                ])),
                ..PAGE.tool
            },
            ..PAGE
        }
    }

    /// `rows` with the mode and the rows `contrast` needs around them.
    fn with_mode(rows: &[Row]) -> &'static [Row] {
        let mut all = vec![MODE.clone(), Row::Slider(Slider::new("detail"))];
        all.extend(rows.iter().cloned());
        Box::leak(all.into_boxed_slice())
    }

    #[test]
    fn a_definition_holds_together() {
        DEFINITION.check().unwrap();
        assert_eq!(
            DEFINITION.defaults().get("curve"),
            Some(&ControlValue::Points(Vec::new()))
        );
    }

    /// A utility's widget, drawn by its own body, is never a processing tool's row.
    #[test]
    fn a_processing_tool_shows_no_utility_s_widget() {
        for widget in [
            Widget::Scopes,
            Widget::History,
            Widget::CollectionsTopLine,
            Widget::CollectionsGroups,
            Widget::Styles,
            Widget::Info,
        ] {
            let rows = Box::leak(Box::new([Row::Custom {
                widget,
                controls: &[],
            }]));
            assert_eq!(page(rows).check(), Err(DefinitionError::Widget(widget)));
        }
    }

    #[test]
    fn a_page_as_darktable_draws_it_holds_together() {
        PAGE.check().unwrap();
        // Before every tab's rows, the tabs', then after them, each row before those it
        // holds.
        let shown: Vec<Vec<&str>> = PAGE.rows().map(|row| row.controls().to_vec()).collect();
        assert_eq!(
            shown,
            [
                vec!["amount"],
                vec!["mode"],
                vec!["detail"],
                vec![],
                vec!["contrast"],
                vec![],
                vec!["contrast"],
                vec![],
                vec!["smooth"],
                vec![],
                vec!["radius"],
                vec!["mask"],
            ]
        );
        assert_eq!(page(with_mode(PER_MODE)).check(), Ok(()));
    }

    #[test]
    fn a_control_in_two_rows_is_one_per_mode_never_two_at_once() {
        let twice = |rows: &[Row]| page(with_mode(rows)).check();
        let rows = |n| {
            Err(DefinitionError::Rows {
                control: "contrast",
                rows: n,
            })
        };
        let laplacian = Row::When {
            control: "mode",
            is: &["laplacian", "bilateral"],
            rows: HIGHLIGHTS,
        };
        // Both shown with the bilateral grid.
        assert_eq!(twice(&[PER_MODE[0].clone(), laplacian]), rows(2));
        // One shown in every mode.
        assert_eq!(
            twice(&[PER_MODE[0].clone(), Row::Slider(Slider::new("contrast"))]),
            rows(2)
        );
        // Apart on another control than the mode: shown together when both hold.
        let mask = Row::When {
            control: "mask",
            is: &["true"],
            rows: CONTRAST,
        };
        assert_eq!(twice(&[PER_MODE[0].clone(), mask]), rows(2));
        // Under every tab and in one.
        const UNDER: &[Row] = &[
            Row::Toggle {
                control: "mask",
                end: &[],
            },
            Row::Slider(Slider::new("detail")),
        ];
        let under = ProcessingDefinition {
            tool: ToolDefinition {
                after: UNDER,
                ..PAGE.tool
            },
            ..PAGE
        };
        assert_eq!(
            under.check(),
            Err(DefinitionError::Rows {
                control: "detail",
                rows: 2
            })
        );
    }

    #[test]
    fn rows_follow_only_a_choice_or_a_toggle_for_values_it_holds() {
        let follow = |control, is| {
            let rows: &'static [Row] = Box::leak(Box::new([Row::When {
                control,
                is,
                rows: CONTRAST,
            }]));
            page(with_mode(rows)).check()
        };
        assert_eq!(
            follow("mode", &["neural"]),
            Ok(()),
            "an unported mode's rows"
        );
        for (control, is) in [
            ("missing", &["true"][..]),
            ("detail", &["0"]),
            ("mode", &["guided"]),
            ("mode", &[]),
            ("smooth", &["on"]),
        ] {
            assert_eq!(
                follow(control, is),
                Err(DefinitionError::Condition(control)),
                "{control} {is:?}"
            );
        }
        // Rows that hide the control they follow.
        const ITSELF: &[Row] = &[MODE, Row::Slider(Slider::new("contrast"))];
        let itself = Row::When {
            control: "mode",
            is: &["bilateral"],
            rows: ITSELF,
        };
        let rows: &'static [Row] =
            Box::leak(Box::new([Row::Slider(Slider::new("detail")), itself]));
        assert_eq!(page(rows).check(), Err(DefinitionError::Condition("mode")));
    }

    #[test]
    fn rows_follow_a_control_on_another_tab_shown_outside_any_group_or_mode() {
        // Contrast on the Fine tab, following the Main tab's mode, as darktable's tone
        // equalizer's contrast compensation follows its filter.
        let two_tabs = |main: &'static [Row], fine: &'static [Row], after: &'static [Row]| {
            ProcessingDefinition {
                tool: ToolDefinition {
                    tabs: Box::leak(Box::new([
                        Tab {
                            id: Some("main"),
                            rows: main,
                        },
                        Tab {
                            id: Some("fine"),
                            rows: fine,
                        },
                    ])),
                    after,
                    ..PAGE.tool
                },
                ..PAGE
            }
            .check()
        };
        const FOLLOWING_MODE: &[Row] = &[
            FINE_TUNING,
            Row::When {
                control: "mode",
                is: &["bilateral"],
                rows: CONTRAST,
            },
        ];
        const MASK: &[Row] = &[Row::Toggle {
            control: "mask",
            end: &[],
        }];
        assert_eq!(two_tabs(with_mode(&[]), FOLLOWING_MODE, MASK), Ok(()));
        // The mask's toggle in a mode of the Main tab: rows on the Fine tab cannot follow
        // it, as it may be hidden there.
        let main = with_mode(&[
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: MASK,
            },
            Row::When {
                control: "mode",
                is: &["bilateral"],
                rows: CONTRAST,
            },
        ]);
        const FOLLOWING_MASK: &[Row] = &[
            FINE_TUNING,
            Row::When {
                control: "mask",
                is: &["true"],
                rows: &[action_line!("masked")],
            },
        ];
        assert_eq!(
            two_tabs(main, FOLLOWING_MASK, &[]),
            Err(DefinitionError::Hidden("mask"))
        );
    }

    #[test]
    fn rows_follow_a_control_only_where_one_of_its_rows_is_sure_to_show() {
        // The smooth toggle is in the Fine tab's group: rows on the Main tab, or in that
        // tab outside the group, would show while it is out of sight.
        assert_eq!(
            page(with_mode(&[Row::When {
                control: "smooth",
                is: &["true"],
                rows: CONTRAST,
            }]))
            .check(),
            Err(DefinitionError::Hidden("smooth"))
        );
        const OUTSIDE: &[Row] = &[
            FINE_TUNING,
            Row::When {
                control: "smooth",
                is: &["true"],
                rows: CONTRAST,
            },
        ];
        let outside = ProcessingDefinition {
            tool: ToolDefinition {
                tabs: Box::leak(Box::new([
                    Tab {
                        id: Some("main"),
                        rows: with_mode(&[]),
                    },
                    Tab {
                        id: Some("fine"),
                        rows: OUTSIDE,
                    },
                ])),
                ..PAGE.tool
            },
            ..PAGE
        };
        assert_eq!(outside.check(), Err(DefinitionError::Hidden("smooth")));
        // The mask's toggle shown only with the local Laplacian filter: rows that follow
        // it must be in that mode too, for those values or fewer.
        const IN_MODE: &[Row] = &[Row::Toggle {
            control: "mask",
            end: &[],
        }];
        let follow_mask = |is: &'static [&'static str]| {
            let following: &'static [Row] = Box::leak(Box::new([Row::When {
                control: "mask",
                is: &["true"],
                rows: CONTRAST,
            }]));
            let rows = [
                Row::When {
                    control: "mode",
                    is: &["laplacian"],
                    rows: IN_MODE,
                },
                Row::When {
                    control: "mode",
                    is,
                    rows: following,
                },
            ];
            ProcessingDefinition {
                tool: ToolDefinition {
                    after: &[],
                    ..page(with_mode(&rows)).tool
                },
                ..page(with_mode(&rows))
            }
            .check()
        };
        assert_eq!(follow_mask(&["laplacian"]), Ok(()));
        assert_eq!(
            follow_mask(&["laplacian", "bilateral"]),
            Err(DefinitionError::Hidden("mask"))
        );
        assert_eq!(
            follow_mask(&["bilateral"]),
            Err(DefinitionError::Hidden("mask"))
        );
    }

    #[test]
    fn a_mode_holds_rows_and_a_group_may_have_none_yet() {
        // A group with nothing behind it yet, as AgX's Show curve before its graph.
        const EMPTY_GROUP: &[Row] = &[
            Row::Slider(Slider::new("contrast")),
            Row::Group(Group {
                id: "empty",
                kind: GroupKind::Collapsible,
                rows: &[],
            }),
        ];
        assert_eq!(page(with_mode(EMPTY_GROUP)).check(), Ok(()));
        const EMPTY_MODE: &[Row] = &[
            Row::Slider(Slider::new("contrast")),
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: &[],
            },
        ];
        assert_eq!(
            page(with_mode(EMPTY_MODE)).check(),
            Err(DefinitionError::Empty("mode"))
        );
    }

    /// A group of `kind` called `id`, holding `rows`.
    fn group(id: &'static str, kind: GroupKind, rows: &[Row]) -> Row {
        Row::Group(Group {
            id,
            kind,
            rows: Box::leak(rows.to_vec().into_boxed_slice()),
        })
    }

    #[test]
    fn a_group_holds_its_rows_whatever_its_kind() {
        for kind in [GroupKind::Plain, GroupKind::Collapsible] {
            let held = group("tuning", kind, CONTRAST);
            assert_eq!(held.rows().len(), 1, "{kind:?}");
            assert!(held.controls().is_empty(), "{kind:?}");
            let definition = page(with_mode(std::slice::from_ref(&held)));
            assert_eq!(definition.check(), Ok(()), "{kind:?}");
            // Its rows are listed after it, the control shown once, inside it.
            let rows: Vec<&Row> = definition.rows().collect();
            let at = rows
                .iter()
                .position(|row| matches!(row, Row::Group(Group { id: "tuning", .. })))
                .unwrap();
            assert_eq!(rows[at + 1].controls(), ["contrast"], "{kind:?}");
        }
    }

    #[test]
    fn a_collapsible_group_inside_another_is_refused() {
        // At any depth, as inside a mode's rows, and inside a plain group too.
        let inner = group("inner", GroupKind::Collapsible, CONTRAST);
        let mode = Row::When {
            control: "mode",
            is: &["bilateral"],
            rows: Box::leak(Box::new([inner.clone()])),
        };
        for kind in [GroupKind::Collapsible, GroupKind::Plain] {
            let outer = group("outer", kind, std::slice::from_ref(&mode));
            assert_eq!(
                page(with_mode(&[outer])).check(),
                Err(DefinitionError::GroupInGroup("inner")),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_plain_group_inside_a_collapsible_one_is_allowed() {
        // As Colour calibration's Target in its area colour mapping.
        let plain = group("target", GroupKind::Plain, CONTRAST);
        let outer = group(
            "mapping",
            GroupKind::Collapsible,
            std::slice::from_ref(&plain),
        );
        assert_eq!(page(with_mode(&[outer])).check(), Ok(()));
        // Two plain groups never nest.
        let outer = group("mapping", GroupKind::Plain, &[plain]);
        assert_eq!(
            page(with_mode(&[outer])).check(),
            Err(DefinitionError::GroupInGroup("target"))
        );
    }

    #[test]
    fn an_empty_plain_group_is_refused() {
        // A collapsible one with no rows has nothing behind it yet; a plain one is only its
        // rows' title.
        let rows = [
            Row::Slider(Slider::new("contrast")),
            group("empty", GroupKind::Plain, &[]),
        ];
        assert_eq!(
            page(with_mode(&rows)).check(),
            Err(DefinitionError::EmptyGroup("empty"))
        );
    }

    #[test]
    fn groups_have_ids_of_their_own_whatever_their_kind() {
        // FINE_TUNING is on the page's other tab.
        for kind in [GroupKind::Collapsible, GroupKind::Plain] {
            let again = group("fine_tuning", kind, CONTRAST);
            assert_eq!(
                page(with_mode(&[again])).check(),
                Err(DefinitionError::DuplicateGroup("fine_tuning")),
                "{kind:?}"
            );
        }
        let twice = [
            group("look", GroupKind::Plain, CONTRAST),
            group("look", GroupKind::Plain, &[action_line!("auto")]),
        ];
        assert_eq!(
            page(with_mode(&twice)).check(),
            Err(DefinitionError::DuplicateGroup("look"))
        );
        let named = group("Look", GroupKind::Plain, CONTRAST);
        assert_eq!(
            page(with_mode(&[named])).check(),
            Err(DefinitionError::Id("Look"))
        );
        let words = [Row::Slider(Slider::new("contrast").label("High lights"))];
        assert_eq!(
            page(with_mode(&words)).check(),
            Err(DefinitionError::Id("High lights"))
        );
    }

    #[test]
    fn a_slider_s_soft_range_lies_in_order_inside_its_range() {
        // `contrast` holds 0 to 1.
        let soft = |min, max| {
            let rows: &'static [Row] = Box::leak(Box::new([Row::Slider(
                Slider::new("contrast").soft(min, max),
            )]));
            page(with_mode(rows)).check()
        };
        assert_eq!(soft(0.0, 0.5), Ok(()));
        assert_eq!(soft(0.0, 1.0), Ok(()), "the whole range");
        for (min, max) in [
            (0.5, 0.5),
            (0.5, 0.2),
            (-0.1, 0.5),
            (0.5, 1.5),
            (0.0, f32::NAN),
        ] {
            assert_eq!(
                soft(min, max),
                Err(DefinitionError::SoftRange("contrast")),
                "{min} to {max}"
            );
        }
        // Its default may lie outside it, as darktable's may.
        assert_eq!(soft(0.2, 0.5), Ok(()));
        // A coordinate's slider is held to the coordinate's range, −1 to 2.
        let total = |min, max| Row::Slider(Slider::new("total").soft(min, max));
        assert_eq!(viewed(tone(), &[AMOUNT, total(-0.5, 1.5)]), Ok(()));
        assert_eq!(
            viewed(tone(), &[AMOUNT, total(0.0, 3.0)]),
            Err(DefinitionError::SoftRange("total"))
        );
    }

    /// A tool's setting, darktable's chart for its colour checker.
    const CHART: SettingRow = SettingRow {
        id: "chart",
        kind: ControlKind::Choice {
            options: &["classic", "passport"],
            default: 0,
            unported: &[],
        },
        decimals: 0,
        track: Track::Plain,
        key: None,
        words: None,
        end: &[],
    };

    #[test]
    fn a_tool_s_settings_are_rows_of_no_control_each_once_and_whole() {
        // In a group of their own, as darktable's colour checker: a group of settings
        // alone holds, and the settings show no control.
        let checker = Row::Group(Group {
            id: "checker",
            kind: GroupKind::Collapsible,
            rows: Box::leak(Box::new([
                Row::Setting(CHART),
                Row::Setting(SettingRow {
                    id: "patch_scale",
                    kind: ControlKind::Number {
                        min: 0.0,
                        max: 1.0,
                        step: 0.01,
                        default: 0.5,
                    },
                    decimals: 3,
                    track: Track::Plain,
                    key: None,
                    words: None,
                    end: &[],
                }),
            ])),
        });
        let rows = [Row::Slider(Slider::new("contrast")), checker];
        assert_eq!(page(with_mode(&rows)).check(), Ok(()));
        assert!(Row::Setting(CHART).controls().is_empty());
        let twice = [
            Row::Slider(Slider::new("contrast")),
            Row::Setting(CHART),
            Row::Setting(CHART),
        ];
        assert_eq!(
            page(with_mode(&twice)).check(),
            Err(DefinitionError::DuplicateSetting("chart"))
        );
        let setting = |setting| {
            let rows = [Row::Slider(Slider::new("contrast")), Row::Setting(setting)];
            page(with_mode(&rows)).check()
        };
        let named = |id| SettingRow { id, ..CHART };
        assert_eq!(setting(named("Chart")), Err(DefinitionError::Id("Chart")));
        let default_outside = SettingRow {
            kind: ControlKind::Choice {
                options: &["classic"],
                default: 1,
                unported: &[],
            },
            ..CHART
        };
        assert_eq!(
            setting(default_outside),
            Err(DefinitionError::Options("chart"))
        );
        let points = SettingRow {
            kind: ControlKind::Points { default: &[] },
            ..CHART
        };
        assert_eq!(setting(points), Err(DefinitionError::Row("chart")));
        // A key in full and declared words are a utility's (decision 93).
        let keyed = SettingRow {
            key: Some("tools.sample.chart"),
            ..CHART
        };
        let own = Err(DefinitionError::ToolSettingKey("chart"));
        assert_eq!(setting(keyed), own);
    }

    #[test]
    fn an_action_is_a_row_of_no_control_each_once() {
        // In a mode, as AgX's Set from above shows only while adjustments are on.
        let rows = [
            Row::Slider(Slider::new("contrast")),
            action_line!("auto_tune"),
            Row::When {
                control: "mode",
                is: &["laplacian"],
                rows: &[action_line!("set_from_above")],
            },
        ];
        assert_eq!(page(with_mode(&rows)).check(), Ok(()));
        assert!(action_line!("auto_tune").controls().is_empty());
        // A line of buttons shows its control, and its buttons' ids are actions'.
        let buttons = Row::Line {
            start: LineStart::Control("contrast"),
            readout: None,
            end: Box::leak(Box::new([button("auto_tune")])),
        };
        assert_eq!(buttons.controls(), ["contrast"]);
        // `tabs` is the tool's tabs' action, never an end item's.
        let reserved = [action_line!("tabs")];
        assert_eq!(
            page(with_mode(&reserved)).check(),
            Err(DefinitionError::ReservedAction("tabs"))
        );
        let clash = [buttons, action_line!("auto_tune")];
        assert_eq!(
            page(with_mode(&clash)).check(),
            Err(DefinitionError::DuplicateAction("auto_tune"))
        );
        let twice = [
            Row::Slider(Slider::new("contrast")),
            action_line!("auto_tune"),
            action_line!("auto_tune"),
        ];
        assert_eq!(
            page(with_mode(&twice)).check(),
            Err(DefinitionError::DuplicateAction("auto_tune"))
        );
        let named = [
            Row::Slider(Slider::new("contrast")),
            action_line!("Auto tune"),
        ];
        assert_eq!(
            page(with_mode(&named)).check(),
            Err(DefinitionError::Id("Auto tune"))
        );
    }

    /// [`page`] with a line of buttons and action lines, five actions in all, adding
    /// `menus` to context menus.
    fn with_menus(menus: &[MenuItem]) -> ProcessingDefinition {
        let rows = [
            Row::Line {
                start: LineStart::Control("contrast"),
                readout: None,
                end: Box::leak(Box::new([button("rotate")])),
            },
            action_line!("auto_tune"),
            action_line!("set_from_above"),
            action_line!("pick"),
            action_line!("clear"),
        ];
        ProcessingDefinition {
            tool: ToolDefinition {
                menus: Box::leak(menus.to_vec().into_boxed_slice()),
                ..page(with_mode(&rows)).tool
            },
            ..page(with_mode(&rows))
        }
    }

    /// The item naming `action` at `place`, in View's section.
    const fn item(place: Place, action: &'static str) -> MenuItem {
        MenuItem {
            place,
            section: Section::VIEW,
            order: 100,
            entry: MenuEntry::Action(action),
        }
    }

    /// The submenu `title` of `actions` at `place`, in View's section.
    fn submenu(place: Place, title: &'static str, actions: &[&'static str]) -> MenuItem {
        MenuItem {
            entry: MenuEntry::Submenu {
                title,
                actions: Box::leak(actions.to_vec().into_boxed_slice()),
            },
            ..item(place, "")
        }
    }

    /// Decision 91: a tool's context menu items name its own actions, a row's or a
    /// button's, each once in its place though in several places, and at most two in the
    /// photo's and a thumbnail's menus and four in ⋯.
    #[test]
    fn a_tool_s_menu_items_name_its_actions_once_each_within_each_place_s_limit() {
        assert_eq!(with_menus(&[]).check(), Ok(()));
        let each = with_menus(&[
            item(Place::Photo, "auto_tune"),
            item(Place::Photo, "rotate"),
            item(Place::Thumbnail, "auto_tune"),
            item(Place::Options, "auto_tune"),
        ]);
        assert_eq!(each.check(), Ok(()));
        // An item naming none of its actions, as one naming a registry action of the app's.
        let unknown = with_menus(&[item(Place::Photo, "reset")]);
        assert_eq!(
            unknown.check(),
            Err(DefinitionError::UnknownMenuAction("reset"))
        );
        let twice = with_menus(&[
            item(Place::Options, "rotate"),
            item(Place::Options, "rotate"),
        ]);
        assert_eq!(
            twice.check(),
            Err(DefinitionError::DuplicateMenuItem("rotate"))
        );
        // The same action at the same place is listed twice whatever its order or section.
        let moved = |section, order| MenuItem {
            section,
            order,
            ..item(Place::Photo, "rotate")
        };
        for other in [moved(Section::VIEW, 200), moved(Section::EXPORT, 100)] {
            let twice = [moved(Section::VIEW, 100), other];
            assert_eq!(
                check_menus(&twice),
                Err(DefinitionError::DuplicateMenuItem("rotate")),
                "{other:?}"
            );
        }
        // Up to each place's limit, and one more.
        let actions = ["auto_tune", "rotate", "set_from_above", "pick", "clear"];
        for (place, most) in [
            (Place::Photo, 2),
            (Place::Thumbnail, 2),
            (Place::Options, 4),
        ] {
            assert_eq!(place.most(), most);
            let items: Vec<MenuItem> = actions.iter().map(|a| item(place, a)).collect();
            assert_eq!(with_menus(&items[..most]).check(), Ok(()), "{place:?}");
            assert_eq!(
                with_menus(&items[..=most]).check(),
                Err(DefinitionError::MenuItems(place))
            );
        }
        // The same limits hold a utility's items, which name the registry's actions.
        let history = [
            item(Place::Photo, "reset"),
            item(Place::Thumbnail, "reset"),
            item(Place::Options, "reset"),
        ];
        assert_eq!(check_menus(&history), Ok(()));
        let crowded = [
            item(Place::Photo, "reset"),
            item(Place::Photo, "undo"),
            item(Place::Photo, "redo"),
        ];
        assert_eq!(
            check_menus(&crowded),
            Err(DefinitionError::MenuItems(Place::Photo))
        );
    }

    /// Decision 91: a tool's submenu holds its own actions, one to eight, each once and
    /// none listed beside it in its place too, and counts as one item towards the limit.
    #[test]
    fn a_tool_s_submenu_holds_one_to_eight_of_its_own_actions_and_counts_as_one() {
        let turn = |actions: &[&'static str]| submenu(Place::Photo, "tool-page-title", actions);
        let both = with_menus(&[
            turn(&["rotate", "auto_tune", "pick", "clear"]),
            item(Place::Photo, "set_from_above"),
        ]);
        assert_eq!(both.check(), Ok(()));
        let unknown = with_menus(&[turn(&["rotate", "spin"])]);
        assert_eq!(
            unknown.check(),
            Err(DefinitionError::UnknownMenuAction("spin"))
        );
        let empty = with_menus(&[turn(&[])]);
        assert_eq!(
            empty.check(),
            Err(DefinitionError::SubmenuItems("tool-page-title"))
        );
        let nine = [&["rotate"; 1][..], &["clear"; 8]].concat();
        assert_eq!(
            check_menus(&[turn(&nine)]),
            Err(DefinitionError::SubmenuItems("tool-page-title"))
        );
        let inside = with_menus(&[turn(&["rotate", "pick", "rotate"])]);
        assert_eq!(
            inside.check(),
            Err(DefinitionError::DuplicateMenuItem("rotate"))
        );
        let beside = with_menus(&[turn(&["rotate", "pick"]), item(Place::Photo, "pick")]);
        assert_eq!(
            beside.check(),
            Err(DefinitionError::DuplicateMenuItem("pick"))
        );
        // Beside it in another place, as on a thumbnail.
        let elsewhere = with_menus(&[turn(&["rotate"]), item(Place::Thumbnail, "rotate")]);
        assert_eq!(elsewhere.check(), Ok(()));
        let crowded = with_menus(&[
            turn(&["rotate"]),
            item(Place::Photo, "pick"),
            item(Place::Photo, "clear"),
        ]);
        assert_eq!(
            crowded.check(),
            Err(DefinitionError::MenuItems(Place::Photo))
        );
    }

    /// [`with_menus`]'s page, its five actions offering `items` for the Toolbar.
    fn with_toolbar(items: &[ToolbarItem]) -> ProcessingDefinition {
        let page = with_menus(&[]);
        ProcessingDefinition {
            tool: ToolDefinition {
                toolbar: Box::leak(items.to_vec().into_boxed_slice()),
                ..page.tool
            },
            ..page
        }
    }

    /// The toolbar item offering `action`, with no icon of its own.
    const fn offered(action: &'static str) -> ToolbarItem {
        ToolbarItem { action, icon: None }
    }

    /// Decision 92: a processing tool's toolbar items name its own actions, a row's or a
    /// button's, each once, with an icon unless a button's, two at most; an empty action,
    /// one of the app's, a repeat, an action row's with no icon and a third item are
    /// refused.
    #[test]
    fn a_tool_s_toolbar_items_name_its_actions_once_each_two_at_most() {
        assert_eq!(with_toolbar(&[]).check(), Ok(()));
        let with_icon = |action| ToolbarItem {
            icon: Some("wand"),
            ..offered(action)
        };
        let both = [offered("rotate"), with_icon("auto_tune")];
        assert_eq!(with_toolbar(&both).check(), Ok(()));
        assert_eq!(with_toolbar(&[with_icon("rotate")]).check(), Ok(()));
        for (items, why) in [
            (
                vec![offered("spin")],
                DefinitionError::UnknownToolbarAction("spin"),
            ),
            // A registry action of the app's, which only a utility names.
            (
                vec![offered("undo")],
                DefinitionError::UnknownToolbarAction("undo"),
            ),
            (vec![offered("")], DefinitionError::UnknownToolbarAction("")),
            (
                vec![offered("rotate"), with_icon("rotate")],
                DefinitionError::DuplicateToolbarItem("rotate"),
            ),
            // An action row's, which has no button to take an icon from.
            (
                vec![offered("auto_tune")],
                DefinitionError::ToolbarIcon("auto_tune"),
            ),
            (
                vec![offered("rotate"), with_icon("pick"), with_icon("clear")],
                DefinitionError::ToolbarItems,
            ),
        ] {
            assert_eq!(with_toolbar(&items).check(), Err(why), "{items:?}");
        }
        assert_eq!(MOST_ON_TOOLBAR, 2);
    }

    /// Decision 91: a tool's item stands from Edit's section up to, not including, the
    /// layout's, and in Edit's after Reset, at order 200 or more.
    #[test]
    fn a_tool_s_item_stands_between_edit_and_layout_after_reset() {
        let at = |section, order| MenuItem {
            section: Section(section),
            order,
            ..item(Place::Photo, "pick")
        };
        for fine in [at(1000, 200), at(2000, 100), at(4500, 100), at(8999, 100)] {
            assert_eq!(with_menus(&[fine]).check(), Ok(()), "{fine:?}");
        }
        for (outside, why) in [
            (at(999, 100), DefinitionError::MenuSection("pick")),
            (at(0, 100), DefinitionError::MenuSection("pick")),
            (at(9000, 100), DefinitionError::MenuSection("pick")),
            (at(1000, 100), DefinitionError::MenuOrder("pick")),
            (at(1000, 199), DefinitionError::MenuOrder("pick")),
        ] {
            assert_eq!(
                with_menus(&[outside]).check(),
                Err(why.clone()),
                "{outside:?}"
            );
            assert_eq!(check_section(&outside), Err(why));
        }
        // A submenu is named by its title.
        let turn = MenuItem {
            section: Section::LAYOUT,
            ..submenu(Place::Photo, "tool-page-title", &["pick"])
        };
        assert_eq!(
            check_section(&turn),
            Err(DefinitionError::MenuSection("tool-page-title"))
        );
    }

    /// A picker writing `contrast` and `detail` and reading the chart, as Exposure's
    /// writes its exposure and black and reads its area mode and target.
    const LEVEL: Picker = Picker {
        id: "level",
        operation: "bilat",
        kind: PickerKind::Area,
        denoise: false,
        output: false,
        space: Space::Operation,
        keep: false,
        writes: &["contrast", "detail"],
        settings: &["chart"],
        words_by: None,
    };

    /// [`PAGE`] with `pickers`, the chart's setting and `rows` on its first tab.
    fn picking(pickers: &[Picker], rows: &[Row]) -> Result<(), DefinitionError> {
        let mut all = vec![Row::Setting(CHART)];
        all.extend(rows.iter().cloned());
        ProcessingDefinition {
            pickers: Box::leak(pickers.into()),
            ..page(with_mode(&all))
        }
        .check()
    }

    /// A picker cursor for [`LEVEL`], an area picker that writes controls.
    const LEVEL_CURSOR: ToolCursor = ToolCursor {
        cursor: CursorDef {
            id: "level",
            group: CursorGroup::Pick,
            icon: "pick",
            key: Some("L"),
            order: 100,
            gestures: &[],
            surfaces: &[Surface::Viewer],
            options: &[],
        },
        picker: "level",
    };

    /// [`PAGE`] with `cursors`, [`LEVEL`] on contrast's quad, and a point picker, `dot`,
    /// and one that only shows, `shown`, in a line of pickers.
    fn with_cursors(cursors: &[ToolCursor]) -> Result<(), DefinitionError> {
        const DOT: Picker = Picker {
            id: "dot",
            kind: PickerKind::Point,
            ..LEVEL
        };
        const SHOWN: Picker = Picker {
            id: "shown",
            writes: &[],
            ..LEVEL
        };
        let rows = [
            Row::Setting(CHART),
            Row::Slider(Slider::new("contrast").end(End::Picker("level"))),
            Row::Line {
                start: LineStart::None,
                readout: None,
                end: &[End::Picker("dot"), End::Picker("shown")],
            },
        ];
        let page = page(with_mode(&rows));
        ProcessingDefinition {
            tool: ToolDefinition {
                toolbar: &[],
                cursors: Box::leak(cursors.into()),
                ..page.tool
            },
            pickers: &[LEVEL, DOT, SHOWN],
            ..page
        }
        .check()
    }

    /// A tool's cursor turns one of its pickers on, one that writes no control among them,
    /// one cursor a picker, in Pick, on the Viewer alone, with no gestures, which its
    /// picker's kinds decide, and no amounts (`docs/cursors.md`, The API).
    #[test]
    fn a_tool_s_cursor_turns_one_of_its_pickers_on() {
        let dot = ToolCursor {
            cursor: CursorDef {
                id: "dot",
                key: None,
                ..LEVEL_CURSOR.cursor
            },
            picker: "dot",
        };
        let shown = ToolCursor {
            cursor: CursorDef {
                id: "shown",
                ..dot.cursor
            },
            picker: "shown",
        };
        assert_eq!(with_cursors(&[]), Ok(()));
        assert_eq!(with_cursors(&[LEVEL_CURSOR, dot, shown]), Ok(()));
        let with = |change: fn(&mut CursorDef)| {
            let mut cursor = LEVEL_CURSOR;
            change(&mut cursor.cursor);
            cursor
        };
        let naming = |picker| ToolCursor {
            picker,
            ..LEVEL_CURSOR
        };
        let refused: Vec<(&str, Vec<ToolCursor>, DefinitionError)> = vec![
            (
                "no snake_case id",
                vec![with(|c| c.id = "Level")],
                DefinitionError::Id("Level"),
            ),
            (
                "an id twice",
                vec![
                    LEVEL_CURSOR,
                    ToolCursor {
                        cursor: CursorDef {
                            id: "level",
                            ..dot.cursor
                        },
                        ..dot
                    },
                ],
                DefinitionError::DuplicateCursor("level"),
            ),
            (
                "a picker the tool lacks",
                vec![naming("spot")],
                DefinitionError::CursorPicker("level"),
            ),
            (
                "two cursors for one picker",
                vec![LEVEL_CURSOR, with(|c| c.id = "again")],
                DefinitionError::CursorsOfPicker("level"),
            ),
            (
                "a group other than Pick",
                vec![with(|c| c.group = CursorGroup::Mask)],
                DefinitionError::CursorGroup("level"),
            ),
            (
                "gestures",
                vec![with(|c| c.gestures = &[Gesture::Click, Gesture::Area])],
                DefinitionError::CursorGestures("level"),
            ),
            (
                "no surface",
                vec![with(|c| c.surfaces = &[])],
                DefinitionError::CursorSurfaces("level"),
            ),
            (
                "the Browser too",
                vec![with(|c| c.surfaces = &[Surface::Viewer, Surface::Browser])],
                DefinitionError::CursorSurfaces("level"),
            ),
            (
                "amounts",
                vec![with(|c| c.options = &[Row::Setting(CHART)])],
                DefinitionError::CursorOptions("level"),
            ),
        ];
        for (case, cursors, error) in refused {
            assert_eq!(with_cursors(&cursors), Err(error), "{case}");
        }
    }

    /// A readout stands in a line of pickers' label's place: none with a label, one with
    /// texts, each named once, its id unique among the tool's readouts; and its words follow
    /// from its ids.
    #[test]
    fn a_readout_takes_a_line_s_label_s_place() {
        const CONTRAST: Row = Row::Slider(Slider::new("contrast"));
        const CCT: ReadoutDef = ReadoutDef {
            id: "cct",
            texts: &["daylight", "undefined"],
            decimals: 0,
            swatch: true,
        };
        let row = |start, readout| Row::Line {
            start,
            readout: Some(readout),
            end: &[End::Picker("level")],
        };
        let none = LineStart::None;
        assert_eq!(picking(&[LEVEL], &[CONTRAST, row(none, CCT)]), Ok(()));
        let refused = |rows: &[Row]| picking(&[LEVEL], rows);
        assert_eq!(
            refused(&[CONTRAST, row(LineStart::Label("auto_tune"), CCT)]),
            Err(DefinitionError::Readout("cct"))
        );
        for readout in [
            ReadoutDef { texts: &[], ..CCT },
            ReadoutDef {
                texts: &["daylight", "daylight"],
                ..CCT
            },
            ReadoutDef {
                texts: &["Daylight"],
                ..CCT
            },
            ReadoutDef { id: "CCT", ..CCT },
        ] {
            assert_eq!(
                refused(&[CONTRAST, row(none, readout)]),
                Err(DefinitionError::Readout(readout.id))
            );
        }
        // Two lines of pickers, two readouts by one id.
        let spot = Picker {
            id: "spot",
            ..LEVEL
        };
        let other = Row::Line {
            start: LineStart::None,
            readout: Some(CCT),
            end: &[End::Picker("spot")],
        };
        assert_eq!(
            picking(&[LEVEL, spot], &[CONTRAST, row(none, CCT), other]),
            Err(DefinitionError::Readout("cct"))
        );
        // Its words: each text's and its tooltip, and the swatch's.
        let rows: &'static [Row] = Box::leak(Box::new([CONTRAST, row(none, CCT)]));
        let definition = ProcessingDefinition {
            pickers: &[LEVEL],
            ..page(with_mode(rows))
        };
        let ids = crate::messages::of(&definition);
        for id in [
            "tool-page-readout-cct-daylight",
            "tool-page-readout-cct-daylight-promise",
            "tool-page-readout-cct-undefined",
            "tool-page-readout-cct-undefined-promise",
            "tool-page-readout-cct-swatch",
        ] {
            assert!(ids.iter().any(|found| found == id), "{id}: {ids:?}");
        }
    }

    #[test]
    fn a_picker_is_in_exactly_one_row_a_slider_s_end_or_a_line() {
        const CONTRAST: Row = Row::Slider(Slider::new("contrast"));
        const QUAD: Row = Row::Slider(Slider::new("contrast").end(End::Picker("level")));
        assert!(QUAD.pickers().eq(["level"]));
        assert_eq!(picking(&[LEVEL], &[QUAD]), Ok(()));
        let buttons = |start, pickers: &[&'static str]| Row::Line {
            start,
            readout: None,
            end: Box::leak(pickers.iter().map(|&id| End::Picker(id)).collect()),
        };
        // A line of pickers shows no control, with a label or none.
        let row = buttons(LineStart::Label("auto_tune"), &["level"]);
        assert!(row.controls().is_empty());
        assert!(row.pickers().eq(["level"]));
        assert_eq!(picking(&[LEVEL], &[CONTRAST, row.clone()]), Ok(()));
        let unlabelled = buttons(LineStart::None, &["level", "spot"]);
        assert!(unlabelled.pickers().eq(["level", "spot"]));
        let spot = Picker {
            id: "spot",
            ..LEVEL
        };
        assert_eq!(picking(&[LEVEL, spot], &[CONTRAST, unlabelled]), Ok(()));
        // Pickers are a namespace of their own: one takes its slider's control's id.
        let contrast = Picker {
            id: "contrast",
            ..LEVEL
        };
        let own = Row::Slider(Slider::new("contrast").end(End::Picker("contrast")));
        assert_eq!(picking(&[contrast], &[own]), Ok(()));
        let rows = |rows| {
            Err(DefinitionError::PickerRows {
                picker: "level",
                rows,
            })
        };
        assert_eq!(picking(&[LEVEL], &[CONTRAST]), rows(0));
        assert_eq!(picking(&[LEVEL], &[QUAD, row.clone()]), rows(2));
        let twice = buttons(LineStart::None, &["level", "level"]);
        assert_eq!(picking(&[LEVEL], &[CONTRAST, twice]), rows(2));
        assert_eq!(
            picking(&[], &[QUAD]),
            Err(DefinitionError::UnknownPicker("level"))
        );
        assert_eq!(
            picking(&[LEVEL], &[QUAD, buttons(LineStart::None, &[])]),
            Err(DefinitionError::EmptyLine)
        );
        let label = LineStart::Label("Auto");
        assert_eq!(
            picking(&[LEVEL], &[CONTRAST, buttons(label, &["level"])]),
            Err(DefinitionError::Id("Auto"))
        );
        // Its label is a plain group's title, sharing its words, so never a group's id.
        let label = Row::Group(Group {
            id: "auto_tune",
            kind: GroupKind::Plain,
            rows: &[CONTRAST],
        });
        assert_eq!(
            picking(&[LEVEL], &[label, row]),
            Err(DefinitionError::DuplicateGroup("auto_tune"))
        );
    }

    /// An action at a line's end: `id`, its face and whether it is ported.
    fn action(id: &'static str, face: Face, unported: bool) -> End {
        End::Action(ActionEnd {
            id,
            face,
            menu: &[],
            reads: &[],
            unported,
        })
    }

    /// A line of `end` after `start`, with `readout`.
    fn line(start: LineStart, readout: Option<ReadoutDef>, end: &[End]) -> Row {
        Row::Line {
            start,
            readout,
            end: Box::leak(end.into()),
        }
    }

    /// A line of any shape is drawn (decision 107): pickers, icon or text actions, after
    /// no start, a label, a control or a readout, ported or not, and a toggle takes end
    /// items after its switch.
    #[test]
    fn a_line_and_a_toggle_take_end_items_of_any_shape() {
        const CCT: ReadoutDef = ReadoutDef {
            id: "cct",
            texts: &["daylight"],
            decimals: 0,
            swatch: true,
        };
        let icon = Face::Icon(ButtonIcon::RotateLeft);
        let control = LineStart::Control("contrast");
        let label = LineStart::Label("auto_tune");
        let lines = [
            line(label, None, &[End::Picker("level")]),
            line(LineStart::None, Some(CCT), &[End::Picker("level")]),
            line(control, None, &[action("rotate", icon, false)]),
            line(LineStart::None, None, &[action("auto", Face::Text, true)]),
            // Mixed lines, pickers and actions of either face, ported or not.
            line(
                label,
                None,
                &[End::Picker("level"), action("rotate", icon, false)],
            ),
            line(
                LineStart::None,
                Some(CCT),
                &[action("auto", Face::Text, false), End::Picker("level")],
            ),
            line(control, None, &[action("rotate", icon, true)]),
            line(
                LineStart::None,
                None,
                &[
                    action("auto", Face::Text, false),
                    action("again", Face::Text, true),
                ],
            ),
        ];
        for row in lines {
            // Contrast shown, and the picker in one place.
            let mut rows = vec![row.clone()];
            if row.controls().is_empty() {
                rows.push(Row::Slider(Slider::new("contrast")));
            }
            if row.pickers().next().is_none() {
                rows.push(line(LineStart::None, None, &[End::Picker("level")]));
            }
            assert_eq!(picking(&[LEVEL], &rows), Ok(()), "{rows:?}");
        }
        // A toggle with a text action after its switch, its fine tuning's.
        const FINE: Row = Row::Group(Group {
            id: "fine_tuning",
            kind: GroupKind::Collapsible,
            rows: &[
                Row::Toggle {
                    control: "smooth",
                    end: &[End::Action(ActionEnd {
                        id: "auto",
                        face: Face::Text,
                        menu: &[],
                        reads: &[],
                        unported: false,
                    })],
                },
                Row::When {
                    control: "smooth",
                    is: &["true"],
                    rows: &[Row::Slider(Slider::new("radius"))],
                },
            ],
        });
        let toggled = ProcessingDefinition {
            tool: ToolDefinition {
                tabs: Box::leak(Box::new([
                    Tab {
                        id: Some("main"),
                        rows: with_mode(CONTRAST),
                    },
                    Tab {
                        id: Some("fine"),
                        rows: &[FINE],
                    },
                ])),
                ..PAGE.tool
            },
            ..PAGE
        };
        assert_eq!(toggled.check(), Ok(()));
        // A slider's one end item is a picker.
        let ends = Row::Slider(Slider::new("contrast").end(action("rotate", icon, false)));
        assert_eq!(
            picking(&[], &[ends]),
            Err(DefinitionError::EndNotDrawn("contrast"))
        );
        // A line holds at least one end item.
        assert_eq!(
            picking(&[], &[line(control, None, &[])]),
            Err(DefinitionError::EmptyLine)
        );
        // An action's id is the tool's once, a line's or a toggle's.
        let twice = [
            line(control, None, &[action("rotate", icon, false)]),
            line(
                LineStart::None,
                None,
                &[action("rotate", Face::Text, false)],
            ),
        ];
        assert_eq!(
            picking(&[], &twice),
            Err(DefinitionError::DuplicateAction("rotate"))
        );
    }

    /// An action is never called as a control is: one availability rule takes either's id
    /// (decision 107).
    #[test]
    fn an_action_is_never_called_as_a_control() {
        let rows = [
            line(
                LineStart::None,
                None,
                &[action("detail", Face::Text, false)],
            ),
            Row::Slider(Slider::new("contrast")),
        ];
        assert_eq!(
            picking(&[], &rows),
            Err(DefinitionError::ActionIsControl("detail"))
        );
    }

    /// A menu is an icon button's, of 2 to `MOST_IN_SUBMENU` actions, each snake_case and
    /// unique among the tool's actions, none a control's; its button runs nothing, so no
    /// context menu or toolbar item may name it (decision 107).
    #[test]
    fn a_menu_is_an_icon_button_s_of_two_to_eight_actions() {
        let icon = Face::Icon(ButtonIcon::PrimariesMenu);
        let menu = |face, menu: &'static [&'static str]| {
            line(
                LineStart::Label("reset"),
                None,
                &[End::Action(ActionEnd {
                    id: "reset",
                    face,
                    menu,
                    reads: &[],
                    unported: false,
                })],
            )
        };
        let contrast = Row::Slider(Slider::new("contrast"));
        let check = |row: Row| picking(&[], &[row, contrast.clone()]);
        assert_eq!(check(menu(icon, &["soft", "plain"])), Ok(()));
        let refused = |id| Err(DefinitionError::ActionMenu(id));
        assert_eq!(
            check(menu(Face::Text, &["soft", "plain"])),
            refused("reset")
        );
        assert_eq!(check(menu(icon, &["soft"])), refused("reset"));
        let nine: &'static [&'static str] = &["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        assert_eq!(check(menu(icon, nine)), refused("reset"));
        assert_eq!(
            check(menu(icon, &["soft", "Plain"])),
            Err(DefinitionError::Id("Plain"))
        );
        assert_eq!(
            check(menu(icon, &["soft", "soft"])),
            Err(DefinitionError::DuplicateAction("soft"))
        );
        // Its button's id among them, which a menu's item would name twice.
        assert_eq!(
            check(menu(icon, &["soft", "reset"])),
            Err(DefinitionError::DuplicateAction("reset"))
        );
        assert_eq!(
            check(menu(icon, &["soft", "detail"])),
            Err(DefinitionError::ActionIsControl("detail"))
        );
    }

    /// A warning is a slider's end item alone, its id snake_case and the tool's once
    /// (decision 107).
    #[test]
    fn a_warning_is_a_slider_s_alone() {
        let warned = Row::Slider(Slider::new("contrast").end(End::Warning("flat")));
        assert_eq!(picking(&[], &[warned]), Ok(()));
        let on_line = line(LineStart::Label("look"), None, &[End::Warning("flat")]);
        let contrast = Row::Slider(Slider::new("contrast"));
        assert_eq!(
            picking(&[], &[on_line, contrast]),
            Err(DefinitionError::WarningPlace("flat"))
        );
        let mode = Row::Choice {
            control: "mode",
            style: ChoiceStyle::List,
            end: &[End::Warning("flat")],
        };
        assert_eq!(
            page(with_mode(&[mode])).check(),
            Err(DefinitionError::WarningPlace("flat"))
        );
        let named = Row::Slider(Slider::new("contrast").end(End::Warning("Flat")));
        assert_eq!(picking(&[], &[named]), Err(DefinitionError::Id("Flat")));
        // A warned slider's other checks hold too: its soft range and its own words.
        let soft = Row::Slider(
            Slider::new("contrast")
                .soft(0.8, 0.2)
                .end(End::Warning("flat")),
        );
        assert_eq!(
            picking(&[], &[soft]),
            Err(DefinitionError::SoftRange("contrast"))
        );
        let words = Row::Slider(
            Slider::new("contrast")
                .label("Words")
                .end(End::Warning("flat")),
        );
        assert_eq!(picking(&[], &[words]), Err(DefinitionError::Id("Words")));
    }

    /// The facts a tool publishes are snake_case, each once (decision 107).
    #[test]
    fn a_tool_publishes_each_fact_once() {
        let publishing = |publishes: &'static [&'static str]| {
            ProcessingDefinition {
                publishes,
                ..DEFINITION
            }
            .check()
        };
        assert_eq!(publishing(&["level"]), Ok(()));
        assert_eq!(
            publishing(&["level", "level"]),
            Err(DefinitionError::DuplicatePublished("level"))
        );
        assert_eq!(publishing(&["Level"]), Err(DefinitionError::Id("Level")));
    }

    /// A choice's and a setting's end items wait for decision 107's next steps.
    #[test]
    fn a_choice_and_a_setting_take_no_end_item_yet() {
        let end: &'static [End] = &[End::Picker("level")];
        let choice = Row::Choice {
            control: "mode",
            style: ChoiceStyle::List,
            end,
        };
        let setting = Row::Setting(SettingRow { end, ..CHART });
        for (row, id) in [(choice, "mode"), (setting, "chart")] {
            assert_eq!(
                page(with_mode(&[row])).check(),
                Err(DefinitionError::EndNotDrawn(id))
            );
        }
    }

    /// A processing tool's fields wait for Lens correction's port, which brings fields of
    /// controls: a line of fields of settings is a utility's alone.
    #[test]
    fn a_processing_tool_takes_no_fields_row_yet() {
        const WIDTH: SettingRow = SettingRow {
            id: "width",
            kind: ControlKind::Number {
                min: 0.0,
                max: 100.0,
                step: 1.0,
                default: 0.0,
            },
            key: Some("sample.width"),
            ..CHART
        };
        const HEIGHT: SettingRow = SettingRow {
            id: "height",
            key: Some("sample.height"),
            ..WIDTH
        };
        let fields = Row::Fields(Fields {
            id: "size",
            words: "sample-size",
            fields: &[
                Field {
                    setting: WIDTH,
                    before: None,
                    unit: None,
                },
                Field {
                    setting: HEIGHT,
                    before: Some(Separator::Times),
                    unit: None,
                },
            ],
        });
        assert_eq!(fields.setting_rows().count(), 2);
        assert!(fields.controls().is_empty());
        assert_eq!(
            page(with_mode(&[fields])).check(),
            Err(DefinitionError::Fields("size"))
        );
    }

    /// A fact is a utility's read-only row (decision 110), never a processing tool's.
    #[test]
    fn a_processing_tool_takes_no_fact() {
        let fact = Row::Fact(Fact {
            id: "make",
            shown: true,
            cut: Cut::End,
            end: None,
        });
        assert!(fact.controls().is_empty());
        assert_eq!(
            page(with_mode(&[fact])).check(),
            Err(DefinitionError::Fact("make"))
        );
    }

    /// What each shape of row shows: a line its control at its start and its pickers, a
    /// slider its control and its end's picker.
    #[test]
    fn a_line_shows_its_start_s_control_and_its_pickers() {
        let icon = Face::Icon(ButtonIcon::RotateLeft);
        let buttons = line(
            LineStart::Control("orientation"),
            None,
            &[action("rotate", icon, false), action("flip", icon, false)],
        );
        assert_eq!(buttons.controls(), ["orientation"]);
        assert!(buttons.pickers().next().is_none());
        let pickers = line(
            LineStart::Label("auto_tune"),
            None,
            &[End::Picker("black"), End::Picker("white")],
        );
        assert!(pickers.controls().is_empty());
        assert!(pickers.pickers().eq(["black", "white"]));
        let text = line(LineStart::None, None, &[action("auto", Face::Text, true)]);
        assert!(text.controls().is_empty() && text.pickers().next().is_none());
        let slider = Row::Slider(Slider::new("contrast").end(End::Picker("level")));
        assert_eq!(slider.controls(), ["contrast"]);
        assert!(slider.pickers().eq(["level"]));
        let plain = Row::Slider(Slider::new("contrast"));
        assert!(plain.pickers().next().is_none());
    }

    /// A preset row's option that turns a picker on, as White balance's Settings' from
    /// image area turns `spot` on, is that picker's one place while the row is drawn: an
    /// option of the row, named once, and a picker of the tool.
    #[test]
    fn a_preset_row_s_option_is_a_picker_s_one_place() {
        const LOOK: Row = Row::Preset {
            preset: "look",
            style: ChoiceStyle::List,
        };
        const CONTRAST: Row = Row::Slider(Slider::new("contrast"));
        const QUAD: Row = Row::Slider(Slider::new("contrast").end(End::Picker("level")));
        let with = |turning: &[(&'static str, &'static str)], rows: &[Row]| {
            let mut all = vec![Row::Setting(CHART)];
            all.extend(rows.iter().cloned());
            ProcessingDefinition {
                pickers: &[LEVEL],
                preset_rows: Box::leak(Box::new([PresetRow {
                    id: "look",
                    options: &["soft", "spot"],
                    controls: &["amount"],
                    pickers: Box::leak(turning.into()),
                }])),
                ..page(with_mode(&all))
            }
            .check()
        };
        assert_eq!(with(&[("spot", "level")], &[LOOK, CONTRAST]), Ok(()));
        let rows = |rows| {
            Err(DefinitionError::PickerRows {
                picker: "level",
                rows,
            })
        };
        // Placed both by an option and in a row, or by an option of a row not drawn.
        assert_eq!(with(&[("spot", "level")], &[LOOK, QUAD]), rows(2));
        assert_eq!(with(&[("spot", "level")], &[CONTRAST]), rows(0));
        // The option is the row's, once, and the picker the tool's.
        for turning in [
            &[("sharp", "level")][..],
            &[("spot", "level"), ("spot", "level")],
            &[("spot", "missing")],
        ] {
            assert_eq!(
                with(turning, &[LOOK, QUAD]),
                Err(DefinitionError::PresetPickers("look")),
                "{turning:?}"
            );
        }
    }

    #[test]
    fn a_picker_writes_the_tool_s_controls_and_reads_its_settings_each_once() {
        let quad = |picker| Row::Slider(Slider::new("contrast").end(End::Picker(picker)));
        let one = |picker: Picker| picking(&[picker], &[quad(picker.id)]);
        // One that only shows writes nothing and reads nothing.
        let shows = Picker {
            writes: &[],
            settings: &[],
            ..LEVEL
        };
        assert_eq!(one(shows), Ok(()));
        let named = Picker {
            id: "Level",
            ..LEVEL
        };
        assert_eq!(one(named), Err(DefinitionError::Id("Level")));
        assert_eq!(
            picking(&[LEVEL, LEVEL], &[quad("level")]),
            Err(DefinitionError::DuplicatePicker("level"))
        );
        let writes = |writes| one(Picker { writes, ..LEVEL });
        let wrong = Err(DefinitionError::PickerWrites("level"));
        assert_eq!(writes(&["gamma"]), wrong);
        assert_eq!(writes(&["contrast", "contrast"]), wrong);
        let settings = |settings| one(Picker { settings, ..LEVEL });
        let wrong = Err(DefinitionError::PickerSettings("level"));
        // A setting is a row's, never a control.
        assert_eq!(settings(&["mode"]), wrong);
        assert_eq!(settings(&["chart", "chart"]), wrong);
    }

    /// A number setting, as Exposure's Lightness.
    const SCALE: SettingRow = SettingRow {
        id: "scale",
        kind: ControlKind::Number {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: 50.0,
        },
        decimals: 1,
        track: Track::Lightness,
        key: None,
        words: None,
        end: &[],
    };

    /// `rows` after Contrast's row and the chart's setting, on [`PAGE`]'s first tab.
    fn following(rows: &[Row]) -> Result<(), DefinitionError> {
        let mut all = vec![Row::Slider(Slider::new("contrast")), Row::Setting(CHART)];
        all.extend(rows.iter().cloned());
        page(with_mode(&all)).check()
    }

    /// Rows following the chart's setting for `is`.
    fn when_chart(is: &'static [&'static str], rows: &[Row]) -> Row {
        Row::WhenSetting {
            setting: "chart",
            is,
            rows: Box::leak(rows.into()),
        }
    }

    #[test]
    fn rows_follow_a_choice_setting_for_options_it_has_and_hold_no_control() {
        // As Exposure's Lightness follows Picker target (decision 88).
        let scale = [Row::Setting(SCALE)];
        let row = when_chart(&["passport"], &scale);
        assert_eq!(following(std::slice::from_ref(&row)), Ok(()));
        assert!(row.controls().is_empty());
        assert_eq!(row.rows().len(), 1);
        // For several options, and in a mode or a group with its setting.
        let both = when_chart(&["classic", "passport"], &scale);
        assert_eq!(following(&[both]), Ok(()));
        let grouped = Row::Group(Group {
            id: "checker",
            kind: GroupKind::Collapsible,
            rows: Box::leak(Box::new([Row::Setting(CHART), row.clone()])),
        });
        let rows = [Row::Slider(Slider::new("contrast")), grouped];
        assert_eq!(page(with_mode(&rows)).check(), Ok(()));

        let wrong = Err(DefinitionError::SettingCondition("chart"));
        // An option the setting lacks, or none.
        assert_eq!(following(&[when_chart(&["pocket"], &scale)]), wrong);
        assert_eq!(following(&[when_chart(&[], &scale)]), wrong);
        // A control among the rows: what shows on the edit never follows the app's.
        let contrast = [Row::Slider(Slider::new("contrast"))];
        let rows = [Row::Setting(CHART), when_chart(&["passport"], &contrast)];
        assert_eq!(page(with_mode(&rows)).check(), wrong);
        // The setting among its own rows.
        let itself = [Row::Setting(CHART)];
        assert_eq!(following(&[when_chart(&["passport"], &itself)]), wrong);
        // Not a choice setting of the tool: a number, a control, or none.
        let scaled = Row::WhenSetting {
            setting: "scale",
            is: &["passport"],
            rows: Box::leak(Box::new([action_line!("reset")])),
        };
        assert_eq!(
            following(&[Row::Setting(SCALE), scaled]),
            Err(DefinitionError::SettingCondition("scale"))
        );
        let mode = Row::WhenSetting {
            setting: "mode",
            is: &["bilateral"],
            rows: Box::leak(Box::new([Row::Setting(SCALE)])),
        };
        assert_eq!(
            following(&[mode]),
            Err(DefinitionError::SettingCondition("mode"))
        );
        // No rows.
        assert_eq!(
            following(&[when_chart(&["passport"], &[])]),
            Err(DefinitionError::Empty("chart"))
        );
        // Where its setting is not sure to show: in a group it is outside, or in a mode.
        let grouped = Row::Group(Group {
            id: "checker",
            kind: GroupKind::Collapsible,
            rows: Box::leak(Box::new([Row::Setting(CHART)])),
        });
        let rows = [
            Row::Slider(Slider::new("contrast")),
            grouped,
            when_chart(&["passport"], &scale),
        ];
        assert_eq!(
            page(with_mode(&rows)).check(),
            Err(DefinitionError::Hidden("chart"))
        );
        let moded = Row::When {
            control: "mode",
            is: &["bilateral"],
            rows: Box::leak(Box::new([Row::Setting(CHART)])),
        };
        let rows = [
            Row::Slider(Slider::new("contrast")),
            moded,
            when_chart(&["passport"], &scale),
        ];
        assert_eq!(
            page(with_mode(&rows)).check(),
            Err(DefinitionError::Hidden("chart"))
        );
    }

    #[test]
    fn a_picker_s_words_follow_a_choice_setting_of_the_tool() {
        let quad = Row::Slider(Slider::new("contrast").end(End::Picker("level")));
        let by = |words_by, more: &[Row]| {
            let mut rows = vec![quad.clone()];
            rows.extend(more.iter().cloned());
            picking(&[Picker { words_by, ..LEVEL }], &rows)
        };
        assert_eq!(by(None, &[]), Ok(()));
        assert_eq!(by(Some("chart"), &[]), Ok(()));
        assert_eq!(DEFINITION.choice_setting("chart"), None);
        let wrong = Err(DefinitionError::WordsBy("level"));
        // A setting the tool lacks, a control, or a number.
        assert_eq!(by(Some("gamma"), &[]), wrong);
        assert_eq!(by(Some("mode"), &[]), wrong);
        assert_eq!(by(Some("scale"), &[Row::Setting(SCALE)]), wrong);
    }

    /// A reference row (decision 88): its picker one that writes no control, shown there
    /// alone, and its two settings the tool's, which a picker may then read.
    #[test]
    fn a_reference_row_names_a_picker_that_writes_nothing_and_two_settings() {
        const MEASURE: Picker = Picker {
            id: "measure",
            writes: &[],
            settings: &["reference_l"],
            ..LEVEL
        };
        let reference = |picker, lightness, photo| Row::Reference {
            picker,
            lightness,
            photo,
        };
        let row = reference("measure", "reference_l", "reference_photo");
        assert!(row.controls().is_empty());
        assert!(row.pickers().eq(["measure"]));
        let contrast = Row::Slider(Slider::new("contrast"));
        assert_eq!(
            picking(&[MEASURE], &[contrast.clone(), row.clone()]),
            Ok(())
        );
        // Another picker may read its L, as Exposure's does.
        let quad = Row::Slider(Slider::new("contrast").end(End::Picker("level")));
        let reads = Picker {
            settings: &["chart", "reference_l"],
            ..LEVEL
        };
        assert_eq!(picking(&[reads, MEASURE], &[quad, row.clone()]), Ok(()));
        // Its picker writes no control, is the tool's, and shows nowhere else.
        let writes = Picker {
            writes: &["detail"],
            ..MEASURE
        };
        assert_eq!(
            picking(&[writes], &[contrast.clone(), row.clone()]),
            Err(DefinitionError::ReferenceWrites("measure"))
        );
        assert_eq!(
            picking(&[], &[contrast.clone(), row.clone()]),
            Err(DefinitionError::UnknownPicker("measure"))
        );
        let twice = Row::Slider(Slider::new("contrast").end(End::Picker("measure")));
        assert_eq!(
            picking(&[MEASURE], &[twice, row]),
            Err(DefinitionError::PickerRows {
                picker: "measure",
                rows: 2
            })
        );
        // Its settings are ids of their own among the tool's.
        let clash = reference("measure", "chart", "reference_photo");
        assert_eq!(
            picking(&[MEASURE], &[contrast.clone(), clash]),
            Err(DefinitionError::DuplicateSetting("chart"))
        );
        let named = reference("measure", "reference_l", "Photo");
        assert_eq!(
            picking(&[MEASURE], &[contrast, named]),
            Err(DefinitionError::Id("Photo"))
        );
    }

    /// A last pick row (decision 88) names a picker of the tool that writes controls, once;
    /// it shows no control and no picker of its own.
    #[test]
    fn a_last_pick_row_names_a_picker_that_writes_once() {
        let quad = Row::Slider(Slider::new("contrast").end(End::Picker("level")));
        let last = Row::LastPick { picker: "level" };
        assert!(last.controls().is_empty() && last.pickers().next().is_none());
        assert_eq!(picking(&[LEVEL], &[quad.clone(), last.clone()]), Ok(()));
        let wrong = Err(DefinitionError::LastPick("level"));
        assert_eq!(
            picking(&[LEVEL], &[quad.clone(), last.clone(), last.clone()]),
            wrong
        );
        let shows = Picker {
            writes: &[],
            ..LEVEL
        };
        assert_eq!(picking(&[shows], &[quad.clone(), last.clone()]), wrong);
        let other = Row::LastPick { picker: "spot" };
        assert_eq!(
            picking(&[LEVEL], &[quad, other]),
            Err(DefinitionError::LastPick("spot"))
        );
    }

    #[test]
    fn rows_that_follow_a_setting_hold_no_preset_row() {
        // A preset row writes controls, so it never follows the app's settings either:
        // a control it alone shows would vanish with the setting's other option.
        let look = Row::Preset {
            preset: "look",
            style: ChoiceStyle::List,
        };
        let definition = ProcessingDefinition {
            tool: ToolDefinition {
                before: Box::leak(Box::new([
                    Row::Setting(CHART),
                    when_chart(&["passport"], &[look]),
                ])),
                ..PAGE.tool
            },
            preset_rows: &[PresetRow {
                id: "look",
                options: &["soft"],
                controls: &["amount"],
                pickers: &[],
            }],
            ..PAGE
        };
        assert_eq!(
            definition.check(),
            Err(DefinitionError::SettingCondition("chart"))
        );
    }

    #[test]
    fn a_preset_row_writes_controls_the_tool_has_and_is_in_one_row_at_most() {
        const AMOUNT: Row = Row::Slider(Slider::new("amount"));
        const LOOK: Row = Row::Preset {
            preset: "look",
            style: ChoiceStyle::List,
        };
        let with = |presets: &'static [PresetRow], before: &'static [Row]| {
            ProcessingDefinition {
                tool: ToolDefinition {
                    before,
                    ..PAGE.tool
                },
                preset_rows: presets,
                ..PAGE
            }
            .check()
        };
        let look = |options: &'static [&'static str], controls: &'static [&'static str]| {
            let preset = PresetRow {
                id: "look",
                options,
                controls,
                pickers: &[],
            };
            with(Box::leak(Box::new([preset])), &[LOOK, AMOUNT])
        };
        assert_eq!(
            look(&["soft", "crisp"], &["amount", "detail", "mode"]),
            Ok(())
        );
        // In no row, as darktable's presets in the presets menu will be (Phase 3).
        let presets = &[PresetRow {
            id: "look",
            options: &["soft"],
            controls: &["amount"],
            pickers: &[],
        }];
        assert_eq!(with(presets, &[AMOUNT]), Ok(()));
        // A control in no row of its own shows through a preset row that writes it, once
        // the row is placed, as White balance's `preset` shows through Settings.
        assert_eq!(with(presets, &[LOOK]), Ok(()));
        assert_eq!(
            with(presets, &[]),
            Err(DefinitionError::Rows {
                control: "amount",
                rows: 0
            })
        );
        assert_eq!(
            with(presets, &[LOOK, AMOUNT, LOOK]),
            Err(DefinitionError::PresetRows {
                preset: "look",
                rows: 2
            })
        );
        assert_eq!(
            with(&[], &[LOOK, AMOUNT]),
            Err(DefinitionError::UnknownPreset("look"))
        );
        // A list: segments draw neither Custom nor an option the photo cannot have.
        let segmented = Row::Preset {
            preset: "look",
            style: ChoiceStyle::Segmented,
        };
        assert_eq!(
            with(presets, Box::leak(Box::new([segmented, AMOUNT]))),
            Err(DefinitionError::SegmentedPreset("look"))
        );
        // Its controls are the tool's, each once.
        for controls in [&[][..], &["amount", "missing"], &["amount", "amount"]] {
            assert_eq!(
                look(&["soft"], controls),
                Err(DefinitionError::PresetControls("look")),
                "{controls:?}"
            );
        }
        for options in [&[][..], &["soft", "soft"], &["Soft"]] {
            assert_eq!(
                look(options, &["amount"]),
                Err(DefinitionError::PresetOptions("look")),
                "{options:?}"
            );
        }
        // Its id is unique together with the controls'.
        let named = |id| PresetRow {
            id,
            options: &["soft"],
            controls: &["amount"],
            pickers: &[],
        };
        let twice = Box::leak(Box::new([named("look"), named("look")]));
        assert_eq!(
            with(twice, &[AMOUNT]),
            Err(DefinitionError::DuplicatePreset("look"))
        );
        let detail = Box::leak(Box::new([named("detail")]));
        assert_eq!(
            with(detail, &[AMOUNT]),
            Err(DefinitionError::DuplicatePreset("detail"))
        );
        let words = Box::leak(Box::new([named("Look")]));
        assert_eq!(with(words, &[AMOUNT]), Err(DefinitionError::Id("Look")));
    }

    /// A coordinate from −1 to 2.
    const fn coordinate(id: &'static str) -> Coordinate {
        Coordinate {
            id,
            min: -1.0,
            max: 2.0,
            step: 0.01,
            unit: None,
        }
    }

    /// A view of detail and contrast, as their total and their balance.
    static TONE: View = View {
        id: "tone",
        controls: &["detail", "contrast"],
        coordinates: &[coordinate("total"), coordinate("balance")],
    };

    const AMOUNT: Row = Row::Slider(Slider::new("amount"));
    const TOTAL: Row = Row::Slider(Slider::new("total"));
    const BALANCE: Row = Row::Slider(Slider::new("balance"));

    /// [`PAGE`] with these views and these rows before every tab's.
    fn viewed(views: Vec<View>, before: &[Row]) -> Result<(), DefinitionError> {
        ProcessingDefinition {
            tool: ToolDefinition {
                before: Box::leak(before.to_vec().into_boxed_slice()),
                ..PAGE.tool
            },
            views: Box::leak(views.into_boxed_slice()),
            ..PAGE
        }
        .check()
    }

    fn tone() -> Vec<View> {
        vec![View { ..TONE }]
    }

    #[test]
    fn a_view_covers_number_controls_each_once_with_a_coordinate_for_each_or_fewer() {
        assert_eq!(viewed(tone(), &[AMOUNT, TOTAL, BALANCE]), Ok(()));
        // Fewer coordinates than controls, a model of them that drops a scale, as White
        // balance's temperature and tint of three coefficients (decision 27); none, never.
        let model = View {
            controls: &["detail", "contrast", "amount"],
            ..TONE
        };
        assert_eq!(viewed(vec![model], &[AMOUNT, TOTAL, BALANCE]), Ok(()));
        let none = View {
            coordinates: &[],
            ..TONE
        };
        assert_eq!(
            viewed(vec![none], &[AMOUNT]),
            Err(DefinitionError::ViewCoordinates("tone"))
        );
        for controls in [
            &[][..],
            &["detail", "detail"],
            &["detail", "mode"],
            &["detail", "missing"],
        ] {
            let view = View { controls, ..TONE };
            assert_eq!(
                viewed(vec![view], &[AMOUNT]),
                Err(DefinitionError::ViewControls("tone")),
                "{controls:?}"
            );
        }
        let one = View {
            controls: &["detail"],
            ..TONE
        };
        assert_eq!(
            viewed(vec![one], &[AMOUNT]),
            Err(DefinitionError::ViewCoordinates("tone"))
        );
        // Views have ids of their own.
        let named = |id| View { id, ..TONE };
        assert_eq!(
            viewed(vec![named("Tone")], &[AMOUNT]),
            Err(DefinitionError::Id("Tone"))
        );
        let other = View {
            coordinates: Box::leak(Box::new([coordinate("lift"), coordinate("gain")])),
            ..TONE
        };
        assert_eq!(
            viewed(vec![named("tone"), other], &[AMOUNT]),
            Err(DefinitionError::DuplicateView("tone"))
        );
    }

    #[test]
    fn a_coordinate_has_an_id_of_its_own_a_range_and_one_slider_row_at_most() {
        // In no row, as a view a panel does not draw yet.
        assert_eq!(viewed(tone(), &[AMOUNT]), Ok(()));
        assert_eq!(
            viewed(tone(), &[AMOUNT, TOTAL, BALANCE, TOTAL]),
            Err(DefinitionError::CoordinateRows {
                coordinate: "total",
                rows: 2
            })
        );
        let toggle = Row::Toggle {
            control: "total",
            end: &[],
        };
        assert_eq!(
            viewed(tone(), &[AMOUNT, toggle]),
            Err(DefinitionError::Row("total"))
        );
        // Unique among the controls, the preset rows and the other coordinates.
        let with = |coordinates: [Coordinate; 2]| {
            ProcessingDefinition {
                views: Box::leak(Box::new([View {
                    coordinates: Box::leak(Box::new(coordinates)),
                    ..TONE
                }])),
                preset_rows: &[PresetRow {
                    id: "look",
                    options: &["soft"],
                    controls: &["amount"],
                    pickers: &[],
                }],
                ..PAGE
            }
            .check()
        };
        for (coordinates, twice) in [
            ([coordinate("detail"), coordinate("balance")], "detail"),
            ([coordinate("look"), coordinate("balance")], "look"),
            ([coordinate("total"), coordinate("total")], "total"),
        ] {
            assert_eq!(
                with(coordinates),
                Err(DefinitionError::DuplicateCoordinate(twice))
            );
        }
        assert_eq!(
            with([coordinate("Total"), coordinate("balance")]),
            Err(DefinitionError::Id("Total"))
        );
        let backwards = Coordinate {
            min: 2.0,
            max: -1.0,
            ..coordinate("total")
        };
        assert_eq!(
            with([backwards, coordinate("balance")]),
            Err(DefinitionError::Range("total"))
        );
    }

    #[test]
    fn a_control_in_no_row_of_its_own_shows_through_a_view_covering_it() {
        let percent = || View {
            id: "percent",
            controls: &["amount"],
            coordinates: Box::leak(Box::new([coordinate("amount_percent")])),
        };
        let shown = Row::Slider(Slider::new("amount_percent"));
        assert_eq!(viewed(vec![percent()], &[shown]), Ok(()));
        // Its view's coordinate in no row, or only a view not covering it in one: it shows
        // nowhere.
        let nowhere = Err(DefinitionError::Rows {
            control: "amount",
            rows: 0,
        });
        assert_eq!(viewed(vec![percent()], &[]), nowhere);
        assert_eq!(viewed(tone(), &[TOTAL]), nowhere);
    }

    #[test]
    fn two_views_may_cover_the_same_controls() {
        // As White balance shows its light, x and y, as temperature and tint and as hue
        // and chroma.
        let lift = || View {
            id: "lift",
            coordinates: Box::leak(Box::new([coordinate("lift"), coordinate("gain")])),
            ..TONE
        };
        let rows = [
            AMOUNT,
            TOTAL,
            BALANCE,
            Row::Slider(Slider::new("lift")),
            Row::Slider(Slider::new("gain")),
        ];
        assert_eq!(viewed(vec![View { ..TONE }, lift()], &rows), Ok(()));
        // A control in no row of its own, covered by two views, one of them in no row.
        let percent = |id, named| View {
            id,
            controls: &["amount"],
            coordinates: Box::leak(Box::new([coordinate(named)])),
        };
        let views = vec![percent("hidden", "fraction"), percent("shown", "percent")];
        let shown = Row::Slider(Slider::new("percent"));
        assert_eq!(viewed(views, &[shown]), Ok(()));
    }

    #[test]
    fn a_coordinate_takes_numbers_in_its_range() {
        let total = coordinate("total");
        assert_eq!(total.check(&ControlValue::Number(2.0)), Ok(2.0));
        assert!(matches!(
            total.check(&ControlValue::Number(2.01)),
            Err(ControlError::OutOfRange {
                control: "total",
                ..
            })
        ));
        assert!(total.check(&ControlValue::Number(f32::NAN)).is_err());
        assert!(matches!(
            total.check(&ControlValue::Bool(true)),
            Err(ControlError::WrongKind {
                control: "total",
                ..
            })
        ));
    }

    #[test]
    fn an_unported_option_is_never_the_default_and_never_segments() {
        let mode = |default, unported| {
            let controls: Vec<Control> = PAGE_CONTROLS
                .iter()
                .map(|control| match control.kind {
                    ControlKind::Choice { options, .. } => Control {
                        kind: ControlKind::Choice {
                            options,
                            default,
                            unported,
                        },
                        ..*control
                    },
                    _ => Control { ..*control },
                })
                .collect();
            ProcessingDefinition {
                controls: Box::leak(controls.into_boxed_slice()),
                ..PAGE
            }
            .check()
        };
        assert_eq!(mode(1, &["neural"]), Ok(()));
        assert_eq!(mode(2, &["neural"]), Err(DefinitionError::Unported("mode")));
        assert_eq!(mode(0, &["guided"]), Err(DefinitionError::Unported("mode")));
        let segments = [Row::Choice {
            control: "mode",
            style: ChoiceStyle::Segmented,
            end: &[],
        }];
        let rows: Vec<Row> = segments
            .into_iter()
            .chain(with_mode(PER_MODE)[1..].iter().cloned())
            .collect();
        assert_eq!(
            page(Box::leak(rows.into_boxed_slice())).check(),
            Err(DefinitionError::Segmented("mode"))
        );
    }

    #[test]
    fn an_unported_option_is_refused_as_a_value() {
        let mode = PAGE.control("mode").unwrap();
        assert_eq!(mode.check(&ControlValue::Choice(1)), Ok(()));
        let unported = ControlError::Unported {
            control: "mode",
            option: "neural",
        };
        assert_eq!(mode.check(&ControlValue::Choice(2)), Err(unported.clone()));
        // Whether typed, as the CLI takes it, or read from an edit.
        let typed = mode.parse("neural").unwrap();
        assert_eq!(mode.check(&typed), Err(unported.clone()));
        assert_eq!(mode.clamp(ControlValue::Choice(2)), Err(unported));
    }

    #[test]
    fn a_control_not_ported_takes_only_its_default() {
        let unported = |kind| Control {
            id: "hold",
            kind,
            unit: None,
            unported: true,
        };
        let refused = ControlError::NotPorted { control: "hold" };
        let not_ported = Err(refused.clone());
        let level = unported(ControlKind::Number {
            min: -18.0,
            max: 18.0,
            step: 0.1,
            default: -4.0,
        });
        assert_eq!(level.check(&ControlValue::Number(-4.0)), Ok(()));
        assert_eq!(level.check(&ControlValue::Number(-3.9)), not_ported);
        // Whether typed, as the CLI takes it, or read from an edit, clamped first.
        assert_eq!(level.check(&level.parse("0").unwrap()), not_ported);
        assert_eq!(level.clamp(ControlValue::Number(40.0)), Err(refused));
        // Its kind still comes first.
        assert!(matches!(
            level.check(&ControlValue::Bool(true)),
            Err(ControlError::WrongKind { .. })
        ));
        let toggle = unported(ControlKind::Bool { default: false });
        assert_eq!(toggle.check(&ControlValue::Bool(false)), Ok(()));
        assert_eq!(toggle.check(&ControlValue::Bool(true)), not_ported);
    }

    #[test]
    fn a_mode_s_rows_show_for_the_values_it_names() {
        let mode = PAGE.control("mode").unwrap();
        let laplacian = ControlValue::Choice(1);
        assert!(mode.is_one_of(&laplacian, &["laplacian", "neural"]));
        assert!(!mode.is_one_of(&laplacian, &["bilateral"]));
        let smooth = PAGE.control("smooth").unwrap();
        assert!(smooth.is_one_of(&ControlValue::Bool(true), &["true"]));
        assert!(!smooth.is_one_of(&ControlValue::Bool(false), &["true"]));
    }

    #[test]
    fn a_control_in_no_row_or_two_or_the_wrong_one_is_refused() {
        static TWICE: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Slider(Slider::new("exposure")),
        ];
        static WRONG: &[Row] = &[Row::Toggle {
            control: "exposure",
            end: &[],
        }];
        let with = |rows: &'static [Row]| ProcessingDefinition {
            tool: ToolDefinition {
                id: "look",
                tabs: Box::leak(Box::new([Tab { id: None, rows }])),
                before: &[],
                after: &[],
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
            controls: &CONTROLS[..1],
            preset_rows: &[],
            views: &[],
            pickers: &[],
            facts: None,
        };
        assert_eq!(
            with(TWICE).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 2
            })
        );
        assert_eq!(
            with(&[]).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 0
            })
        );
        assert_eq!(with(WRONG).check(), Err(DefinitionError::Row("exposure")));
        assert_eq!(with(&ROWS[..1]).check(), Ok(()), "one control, one slider");
    }

    #[test]
    fn only_the_band_graphs_are_a_second_view_of_number_controls() {
        // As the tone equalizer's graph over its bands' sliders: one equalizer besides
        // a control's own row, never two; or the widget alone; and only over numbers. The
        // colour equalizer's hue curve likewise.
        static VIEWED: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["exposure"],
            },
        ];
        static ALONE: &[Row] = &[Row::Custom {
            widget: Widget::Equalizer,
            controls: &["exposure"],
        }];
        static TWICE: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["exposure"],
            },
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["exposure"],
            },
        ];
        static OF_A_CHOICE: &[Row] = &[
            Row::Choice {
                control: "mode",
                style: ChoiceStyle::List,
                end: &[],
            },
            Row::Custom {
                widget: Widget::Equalizer,
                controls: &["mode"],
            },
        ];
        let with = |controls: &'static [Control], rows: &'static [Row]| ProcessingDefinition {
            tool: ToolDefinition {
                id: "look",
                tabs: Box::leak(Box::new([Tab { id: None, rows }])),
                before: &[],
                after: &[],
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
            controls,
            preset_rows: &[],
            views: &[],
            pickers: &[],
            facts: None,
        };
        assert_eq!(with(&CONTROLS[..1], VIEWED).check(), Ok(()));
        assert_eq!(with(&CONTROLS[..1], ALONE).check(), Ok(()));
        assert_eq!(
            with(&CONTROLS[..1], TWICE).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 3
            })
        );
        assert_eq!(
            with(&CONTROLS[1..2], OF_A_CHOICE).check(),
            Err(DefinitionError::Row("mode"))
        );
        static HUE_VIEWED: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::HueCurve,
                controls: &["exposure"],
            },
        ];
        static HUE_OF_A_CHOICE: &[Row] = &[
            Row::Choice {
                control: "mode",
                style: ChoiceStyle::List,
                end: &[],
            },
            Row::Custom {
                widget: Widget::HueCurve,
                controls: &["mode"],
            },
        ];
        assert_eq!(with(&CONTROLS[..1], HUE_VIEWED).check(), Ok(()));
        assert_eq!(
            with(&CONTROLS[1..2], HUE_OF_A_CHOICE).check(),
            Err(DefinitionError::Row("mode"))
        );
        // No other widget is a second row: a curve beside its control's own row is two.
        static CURVE_TOO: &[Row] = &[
            Row::Slider(Slider::new("exposure")),
            Row::Custom {
                widget: Widget::Curve,
                controls: &["exposure"],
            },
        ];
        assert_eq!(
            with(&CONTROLS[..1], CURVE_TOO).check(),
            Err(DefinitionError::Rows {
                control: "exposure",
                rows: 2
            })
        );
    }

    #[test]
    fn ids_are_snake_case() {
        for id in ["exposure", "white_balance", "zone_3"] {
            assert!(is_id(id), "{id}");
        }
        for id in ["", "Exposure", "white-balance", "_x", "x_", "a__b", "3d"] {
            assert!(!is_id(id), "{id}");
        }
    }

    #[test]
    fn values_out_of_range_or_of_another_kind_are_refused() {
        let exposure = control("exposure");
        assert!(exposure.check(&ControlValue::Number(4.0)).is_ok());
        assert!(matches!(
            exposure.check(&ControlValue::Number(4.01)),
            Err(ControlError::OutOfRange { .. })
        ));
        assert!(exposure.check(&ControlValue::Number(f32::NAN)).is_err());
        assert!(matches!(
            exposure.check(&ControlValue::Bool(true)),
            Err(ControlError::WrongKind { .. })
        ));
        assert!(control("mode").check(&ControlValue::Choice(2)).is_err());
        let curve = control("curve");
        let full: Vec<[f32; 2]> = (1..=MAX_POINTS).map(|k| [k as f32 * 0.05, 0.5]).collect();
        let mut over = full.clone();
        over.push([0.97, 0.5]);
        // The ends are the curve's: a point at either, or closer to one than the gap.
        for points in [
            vec![[0.0, 0.0]],
            vec![[1.0, 1.0]],
            vec![[0.002, 0.5]],
            vec![[0.998, 0.5]],
            vec![[0.5, 0.0], [0.4, 1.0]],
            vec![[0.5, 1.5]],
            vec![[0.5, 0.5], [0.502, 0.5]],
            over,
        ] {
            assert!(
                curve.check(&ControlValue::Points(points.clone())).is_err(),
                "{points:?}"
            );
        }
        for points in [Vec::new(), vec![[0.0025, 0.0]], vec![[0.5, 0.5]], full] {
            assert!(
                curve.check(&ControlValue::Points(points.clone())).is_ok(),
                "{points:?}"
            );
        }
    }

    #[test]
    fn text_reads_back_exactly() {
        for (id, value) in [
            ("exposure", ControlValue::Number(0.1 + 0.2)),
            ("exposure", ControlValue::Number(-2.999_999_8)),
            ("mode", ControlValue::Choice(1)),
            ("preserve", ControlValue::Bool(false)),
            (
                "curve",
                ControlValue::Points(vec![[0.25, 0.2], [0.75, 0.8]]),
            ),
            ("curve", ControlValue::Points(Vec::new())),
        ] {
            let control = control(id);
            let text = control.text(&value);
            assert_eq!(control.parse(&text), Ok(value), "{id}: {text}");
        }
        assert_eq!(control("mode").text(&ControlValue::Choice(1)), "hard");
        assert_eq!(control("mode").parse("1"), Ok(ControlValue::Choice(1)));
        assert_eq!(
            control("curve").text(&ControlValue::Points(vec![[0.0, 0.0], [0.5, 0.25]])),
            "0,0 0.5,0.25"
        );
        assert!(control("curve").parse("0,0 0.5").is_err());
        assert!(control("mode").parse("medium").is_err());
    }

    #[test]
    fn a_saved_value_out_of_range_is_clamped() {
        let exposure = control("exposure");
        assert_eq!(
            exposure.clamp(ControlValue::Number(9.0)),
            Ok(ControlValue::Number(4.0))
        );
        assert!(exposure.clamp(ControlValue::Number(f32::NAN)).is_err());
        assert_eq!(
            control("curve").clamp(ControlValue::Points(vec![[0.25, -0.1], [0.75, 1.2]])),
            Ok(ControlValue::Points(vec![[0.25, 0.0], [0.75, 1.0]]))
        );
        // A point brought onto an end is still refused.
        assert!(
            control("curve")
                .clamp(ControlValue::Points(vec![[-0.1, 0.0]]))
                .is_err()
        );
        assert!(control("mode").clamp(ControlValue::Choice(5)).is_err());
    }
}
