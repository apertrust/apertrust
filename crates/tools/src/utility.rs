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

//! The utilities (`docs/gui.md`, The Tools panel, One tool model): the Tools
//! panel's tools that edit nothing, Scopes, History, Collections, Styles and Export, with
//! no [`ProcessingDefinition`](crate::ProcessingDefinition). Each is listed in [`UTILITIES`]
//! with its words, its sentence `tool-<id>-description` as a tool's, what it measures,
//! which its header's tooltip says ([`crate::about`]), what its header's ↺ does, its page
//! and the items it adds to context menus and its ⋯ (decision 91). The app draws each
//! one's page through its `UtilityTool`, its widgets the page's custom rows (decision 93).

pub mod export;

use std::collections::BTreeSet;

use crate::about::WorksOn;
use crate::definition::{is_id, shows_setting};
use crate::{
    ControlKind, DefinitionError, GroupKind, LineStart, MenuEntry, MenuItem, Place, Reset, Row,
    Section, Tab, ToolDefinition, ToolbarItem, Widget, check_menus, check_toolbar,
};

/// A utility tool, which edits nothing (decision 93): the [`ToolDefinition`] every tool
/// embeds, and what it measures, if anything.
#[derive(Debug, Clone, Copy)]
pub struct UtilityDefinition {
    /// Its id, whose sentence is `tool-<id>-description`; its header's ↺, a registry
    /// action's, its own or none; its page, rows that edit nothing ([`Self::check`]), its
    /// own widgets custom rows its app module draws; and the items it adds to context menus
    /// and its ⋯ (decision 91), each the registry action that runs it, by its id, at a
    /// place, held to a tool's limits ([`crate::check_menus`]); none for a utility with
    /// nothing to add. The app builds the menus, every item where its section and order put
    /// it ([`Section`]). The actions it offers for the Toolbar are registry actions too, by
    /// their ids, any number of them (decision 92).
    pub tool: ToolDefinition,
    pub measures: Option<WorksOn>,
    /// Its groups' titles where it declares its own, by their ids, as Export's
    /// `export-storage-options` (decision 93); the others' follow from their ids and kinds
    /// ([`crate::messages::group`], [`crate::messages::collapsible`]).
    pub titles: &'static [(&'static str, &'static str)],
    /// Its collapsible groups open at first, as Export's Global options (decision 90);
    /// every other is closed at first (decision 29).
    pub open: &'static [&'static str],
}

impl UtilityDefinition {
    /// Whether the utility holds together (decision 93): its id; its header's ↺ a registry
    /// action's, its own or none, never a processing tool's, a presets button allowed; its
    /// page one untitled tab, since
    /// its header draws no tab strip; no row that edits, so its rows are settings, groups,
    /// rows following a setting and custom rows naming no control, each a utility's own
    /// widget; no action row until a utility's are bound to the registry; each group and
    /// setting once by its id, groups inside a group only as [`crate::Group`] allows, each
    /// plain group with rows, each setting's kind as a control's; rows following a setting
    /// following a choice setting of its own, shown outside them and wherever they show,
    /// for options it has, with rows of their own; what it titles a group of its page, and
    /// what opens at first a collapsible group with rows; no cursors, until the Colour
    /// readout's design; its toolbar items each naming an action, once, with an icon, any
    /// number of them ([`check_toolbar`]); and its context menu items held to a tool's limits
    /// ([`check_menus`]).
    pub fn check(&self) -> Result<(), DefinitionError> {
        let tool = &self.tool;
        if !is_id(tool.id) {
            return Err(DefinitionError::Id(tool.id));
        }
        if tool.reset == Reset::Tool {
            return Err(DefinitionError::UtilityHeader);
        }
        if !matches!(tool.tabs, [Tab { id: None, .. }]) {
            return Err(DefinitionError::Tabs);
        }
        let mut seen = Seen::default();
        for rows in [tool.before, tool.tabs[0].rows, tool.after] {
            self.check_rows(rows, &mut seen)?;
        }
        // Each id is titled once, as a title is found by its id alone.
        let mut titled_once = std::collections::BTreeSet::new();
        if let Some((id, _)) = self.titles.iter().find(|(id, _)| !titled_once.insert(*id)) {
            return Err(DefinitionError::DuplicateTitle(id));
        }
        // What it titles is a group of its page, and what opens at first a collapsible one.
        let titled = self.titles.iter().map(|(id, _)| id);
        if let Some(id) = titled.copied().find(|id| !seen.groups.contains(id)) {
            return Err(DefinitionError::UnknownTitle(id));
        }
        if let Some(id) = self.open.iter().find(|id| !seen.collapsible.contains(*id)) {
            return Err(DefinitionError::UnknownTitle(id));
        }
        // A group with no rows is drawn closed and disabled, so it never opens at first.
        if let Some(id) = self.open.iter().find(|id| seen.empty.contains(*id)) {
            return Err(DefinitionError::OpensEmpty(id));
        }
        // Each setting rows follow shows wherever they do, never hidden by rows it hides.
        if let Some(setting) = tool.hidden_setting() {
            return Err(DefinitionError::Hidden(setting));
        }
        // A utility's cursors wait for the Colour readout's design (`docs/cursors.md`).
        if !tool.cursors.is_empty() {
            return Err(DefinitionError::UtilityCursors);
        }
        // Its toolbar items have no limit: History offers the edit's six (decision 92).
        check_toolbar(tool.toolbar, None, &[])?;
        check_menus(tool.menus)
    }

    /// Checks `rows` and the rows inside them, as [`Self::check`] says.
    fn check_rows(&self, rows: &'static [Row], seen: &mut Seen) -> Result<(), DefinitionError> {
        for row in rows {
            if let Some(by) = edits(row) {
                return Err(DefinitionError::Edits(by));
            }
            match row {
                Row::Group(group) => {
                    once(&mut seen.groups, group.id, DefinitionError::DuplicateGroup)?;
                    group.check()?;
                    if group.kind == GroupKind::Collapsible {
                        seen.collapsible.insert(group.id);
                        if group.rows.is_empty() {
                            seen.empty.insert(group.id);
                        }
                    }
                }
                Row::Setting(setting) => {
                    setting.check()?;
                    once(
                        &mut seen.settings,
                        setting.id,
                        DefinitionError::DuplicateSetting,
                    )?;
                }
                Row::WhenSetting { setting, is, rows } => {
                    if rows.is_empty() {
                        return Err(DefinitionError::Empty(setting));
                    }
                    let options = match self.tool.setting(setting).map(|found| found.kind) {
                        Some(ControlKind::Choice { options, .. }) => options,
                        _ => &[],
                    };
                    // The setting shows outside the rows it hides, at any depth.
                    let hidden = shows_setting(rows, setting);
                    if is.is_empty() || hidden || !is.iter().all(|option| options.contains(option))
                    {
                        return Err(DefinitionError::SettingCondition(setting));
                    }
                }
                Row::Custom { widget, .. } if !widget.is_utility() => {
                    return Err(DefinitionError::Widget(*widget));
                }
                // A line of actions alone, as `edits` leaves it.
                Row::Line {
                    end: [first, ..], ..
                } => return Err(DefinitionError::UtilityAction(first.id())),
                _ => {}
            }
            self.check_rows(row.rows(), seen)?;
        }
        Ok(())
    }
}

/// What edits in `row`, if anything, for a utility, which edits nothing (decision 93): a
/// control it names, a slider's, a choice's, a toggle's, the one rows follow, a line's at
/// its start or a custom row's; a preset row, which writes controls; a picker's rows, a
/// line with a picker or with no end item; a computed value, which an operation computes;
/// and the guides, a processing tool's. None for a setting, a group, rows following a
/// setting, a line of actions alone and a custom row naming no control.
fn edits(row: &Row) -> Option<&'static str> {
    match row {
        Row::Slider(slider) => Some(slider.control),
        Row::Choice { control, .. }
        | Row::Toggle { control, .. }
        | Row::When { control, .. }
        | Row::Line {
            start: LineStart::Control(control),
            ..
        } => Some(control),
        Row::Custom { controls, .. } => controls.first().copied(),
        Row::Preset { preset, .. } => Some(preset),
        Row::Line { end, .. } if end.is_empty() || end.iter().any(|e| e.picker().is_some()) => {
            Some("pickers")
        }
        Row::Patches { picker, .. } | Row::Reference { picker, .. } | Row::LastPick { picker } => {
            Some(picker)
        }
        Row::Computed(computed) => Some(computed.id),
        Row::Guides => Some("guides"),
        Row::Group(_) | Row::WhenSetting { .. } | Row::Setting(_) | Row::Line { .. } => None,
    }
}

/// The ids a utility's page has used, by kind, each once.
#[derive(Default)]
struct Seen {
    /// Its groups of both kinds.
    groups: BTreeSet<&'static str>,
    collapsible: BTreeSet<&'static str>,
    /// The groups with no rows yet, placeholders drawn disabled and closed.
    empty: BTreeSet<&'static str>,
    settings: BTreeSet<&'static str>,
}

/// Keeps `id` in `ids`, a snake_case id not kept before, else the error `twice` makes of
/// it.
fn once(
    ids: &mut BTreeSet<&'static str>,
    id: &'static str,
    twice: fn(&'static str) -> DefinitionError,
) -> Result<(), DefinitionError> {
    if !is_id(id) {
        return Err(DefinitionError::Id(id));
    }
    if !ids.insert(id) {
        return Err(twice(id));
    }
    Ok(())
}

/// A utility's toolbar item, the registry action `action` with `apertrust-ui`'s icon `icon`
/// (decision 92).
pub(crate) const fn item(action: &'static str, icon: &'static str) -> ToolbarItem {
    ToolbarItem {
        action,
        icon: Some(icon),
    }
}

/// History's ⋯ item `action` at `order` in `section` (decision 93).
const fn option(section: Section, order: u32, action: &'static str) -> MenuItem {
    MenuItem {
        place: Place::Options,
        section,
        order,
        entry: MenuEntry::Action(action),
    }
}

/// The utilities built: Scopes, which measures the finished picture; History, whose ↺ is
/// Reset Edit, which it adds to the photo's and a thumbnail's menus, and whose ⋯ holds
/// Compress History, Truncate History and Create a Style…, then Discard History… apart, as
/// the destructive one; Collections, the Photos tab's first; Styles, whose ⋯ holds Create
/// a Style… (decision 106); and Export, its last ([`export::EXPORT`]). The pages of the
/// first four are their own widgets as custom rows, which keep what each drew before
/// (decision 93): Scopes' the graph with its buttons, History's its steps, Collections' its
/// recents line and its Folders, Styles' its lists; Export's are setting rows in groups,
/// with custom rows of its own. Scopes, History and Collections are open at first, Styles
/// and Export closed, as every processing tool is ([`ToolDefinition::open_at_first`]).
pub const UTILITIES: &[UtilityDefinition] = &[
    UtilityDefinition {
        tool: ToolDefinition {
            id: "scopes",
            tabs: &[Tab {
                id: None,
                rows: &[Row::Custom {
                    widget: Widget::Scopes,
                    controls: &[],
                }],
            }],
            before: &[],
            after: &[],
            reset: Reset::None,
            presets: false,
            open_at_first: true,
            menus: &[],
            toolbar: &[],
            cursors: &[],
        },
        measures: Some(WorksOn::Display),
        titles: &[],
        open: &[],
    },
    UtilityDefinition {
        tool: ToolDefinition {
            id: "history",
            tabs: &[Tab {
                id: None,
                rows: &[Row::Custom {
                    widget: Widget::History,
                    controls: &[],
                }],
            }],
            before: &[],
            after: &[],
            reset: Reset::Action("reset"),
            presets: false,
            open_at_first: true,
            menus: &[
                // Reset in the photo's and a thumbnail's menus (decision 91).
                MenuItem {
                    place: Place::Photo,
                    section: Section::EDIT,
                    order: 100,
                    entry: MenuEntry::Action("reset"),
                },
                MenuItem {
                    place: Place::Thumbnail,
                    section: Section::EDIT,
                    order: 100,
                    entry: MenuEntry::Action("reset"),
                },
                // Its ⋯, four, the most it holds (decision 93).
                option(Section::HISTORY, 100, "compress-history"),
                option(Section::HISTORY, 200, "truncate-history"),
                option(Section::HISTORY, 300, "create-style"),
                option(Section(5000), 100, "discard-history"),
            ],
            // The edit's actions for the Toolbar, the default's among them (decision 92).
            toolbar: &[
                item("reset", "reset"),
                item("undo", "undo"),
                item("redo", "redo"),
                item("copy-edit", "copy"),
                item("apply-edit", "apply"),
                item("copy-apply-parts", "copy"),
            ],
            cursors: &[],
        },
        measures: None,
        titles: &[],
        open: &[],
    },
    UtilityDefinition {
        tool: ToolDefinition {
            id: "collections",
            tabs: &[Tab {
                id: None,
                rows: &[
                    Row::Custom {
                        widget: Widget::CollectionsTopLine,
                        controls: &[],
                    },
                    Row::Custom {
                        widget: Widget::CollectionsGroups,
                        controls: &[],
                    },
                ],
            }],
            before: &[],
            after: &[],
            reset: Reset::None,
            presets: false,
            open_at_first: true,
            menus: &[],
            // View › Collections, its tab chosen (decision 92).
            toolbar: &[item("show-collections", "library")],
            cursors: &[],
        },
        measures: None,
        titles: &[],
        open: &[],
    },
    UtilityDefinition {
        tool: ToolDefinition {
            id: "styles",
            tabs: &[Tab {
                id: None,
                rows: &[Row::Custom {
                    widget: Widget::Styles,
                    controls: &[],
                }],
            }],
            before: &[],
            after: &[],
            reset: Reset::None,
            presets: false,
            open_at_first: false,
            // Its ⋯ begins with Create a Style…; Mode ▸, Import Style… and Export Style…
            // are the app's own items after it (decision 106).
            menus: &[option(Section::HISTORY, 100, "create-style")],
            toolbar: &[],
            cursors: &[],
        },
        measures: None,
        titles: &[],
        open: &[],
    },
    export::EXPORT,
];

/// The utility `id`, if it is one.
pub fn utility(id: &str) -> Option<&'static UtilityDefinition> {
    UTILITIES.iter().find(|utility| utility.tool.id == id)
}

impl UtilityDefinition {
    /// The message id of the title of its group `id` of the kind `kind`: its own where it
    /// declares one ([`Self::titles`]), else the one the id and kind imply
    /// ([`crate::messages::group_title`]).
    pub fn title(&self, id: &str, kind: GroupKind) -> String {
        match self.titles.iter().find(|(titled, _)| *titled == id) {
            Some((_, title)) => (*title).to_owned(),
            None => crate::messages::group_title(self.tool.id, id, kind),
        }
    }

    /// Whether its collapsible group `group` is open at first ([`Self::open`]).
    pub fn opens(&self, group: &str) -> bool {
        self.open.contains(&group)
    }
}

/// Whether the collapsible group `group` of the tool `tool` is open at first: a utility's
/// that it declares open (decision 90); every other closed (decision 29).
pub fn opens(tool: &str, group: &str) -> bool {
    utility(tool).is_some_and(|utility| utility.opens(group))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionEnd, ChoiceStyle, Computed, End, Face, Group, SettingRow, Slider, Track, Word,
    };

    /// A row that edits nothing, Scopes' graph.
    const GRAPH_ROW: Row = Row::Custom {
        widget: Widget::Scopes,
        controls: &[],
    };

    /// A utility whose page is `rows`, on one untitled tab.
    fn with(rows: &'static [Row]) -> UtilityDefinition {
        UtilityDefinition {
            tool: ToolDefinition {
                id: "sample",
                tabs: Box::leak(Box::new([Tab { id: None, rows }])),
                before: &[],
                after: &[],
                reset: Reset::None,
                presets: false,
                open_at_first: false,
                menus: &[],
                toolbar: &[],
                cursors: &[],
            },
            measures: None,
            titles: &[],
            open: &[],
        }
    }

    const GRAPH: SettingRow = SettingRow {
        id: "graph",
        kind: ControlKind::Choice {
            options: &["bars", "lines"],
            default: 0,
            unported: &[],
        },
        decimals: 0,
        track: Track::Plain,
        key: None,
        words: None,
        end: &[],
    };

    const SLIDER: Slider = Slider {
        control: "brightness",
        decimals: 2,
        soft: None,
        track: Track::Plain,
        fill: false,
        end: None,
        label: None,
    };

    const COMPUTED: Computed = Computed {
        id: "level",
        operation: "exposure",
        unit: None,
        decimals: 2,
    };

    #[test]
    fn every_utility_holds_together() {
        for utility in UTILITIES {
            assert_eq!(utility.check(), Ok(()), "{}", utility.tool.id);
        }
    }

    /// Every processing tool is closed at first; of the utilities, Collections, Scopes and
    /// History are open and Export closed (decision 93), until the person opens or closes
    /// one.
    #[test]
    fn processing_tools_start_closed_and_the_utilities_as_declared() {
        for tool in crate::TOOLS {
            let tool = &tool.definition().tool;
            assert!(!tool.open_at_first, "{}", tool.id);
        }
        let open: Vec<(&str, bool)> = UTILITIES
            .iter()
            .map(|utility| (utility.tool.id, utility.tool.open_at_first))
            .collect();
        let expected = [
            ("scopes", true),
            ("history", true),
            ("collections", true),
            ("styles", false),
            ("export", false),
        ];
        assert_eq!(open, expected);
    }

    /// Settings, groups of both kinds, rows following a setting shown beside them, and
    /// custom rows of a utility's widgets naming no control: what a utility holds.
    #[test]
    fn a_utility_holds_settings_groups_and_custom_rows() {
        let utility = with(&[
            Row::Setting(GRAPH),
            Row::WhenSetting {
                setting: "graph",
                is: &["lines"],
                rows: &[Row::Group(Group {
                    id: "lines",
                    kind: GroupKind::Plain,
                    rows: &[GRAPH_ROW],
                })],
            },
            Row::Group(Group {
                id: "more",
                kind: GroupKind::Collapsible,
                rows: &[Row::Custom {
                    widget: Widget::Scopes,
                    controls: &[],
                }],
            }),
        ]);
        assert_eq!(utility.check(), Ok(()));
    }

    /// An action row waits for a utility's to be bound to the registry; a processing
    /// tool's widget is drawn over its controls, which a utility has none of.
    #[test]
    fn a_utility_has_no_action_row_yet_and_no_processing_tool_s_widget() {
        let action = with(&[Row::Line {
            start: LineStart::None,
            readout: None,
            end: &[End::Action(ActionEnd {
                id: "refresh",
                face: Face::Text,
                unported: true,
            })],
        }]);
        assert_eq!(
            action.check(),
            Err(DefinitionError::UtilityAction("refresh"))
        );
        for widget in [
            Widget::Curve,
            Widget::ColourWheels,
            Widget::Equalizer,
            Widget::HueCurve,
            Widget::Swatches,
        ] {
            let rows = Box::leak(Box::new([Row::Custom {
                widget,
                controls: &[],
            }]));
            assert_eq!(with(rows).check(), Err(DefinitionError::Widget(widget)));
        }
    }

    /// A utility's ↺ is never a processing tool's; it may have a presets button.
    #[test]
    fn a_utility_s_header_has_no_tool_reset_and_may_have_presets() {
        let mut reset = with(&[]);
        reset.tool.reset = Reset::Tool;
        assert_eq!(reset.check(), Err(DefinitionError::UtilityHeader));
        let mut presets = with(&[]);
        presets.tool.presets = true;
        assert_eq!(presets.check(), Ok(()));
    }

    /// A utility's cursors wait for the Colour readout's design (`docs/cursors.md`).
    #[test]
    fn a_utility_has_no_cursor_yet() {
        let mut picking = with(&[]);
        picking.tool.cursors = crate::ProcessingTool::definition(&crate::exposure::Exposure)
            .tool
            .cursors;
        assert_eq!(picking.check(), Err(DefinitionError::UtilityCursors));
    }

    /// Decision 92: a utility offers any number of registry actions for the Toolbar,
    /// each once, none empty, each with an icon.
    #[test]
    fn a_utility_offers_any_number_of_toolbar_items_each_once() {
        const THREE: &[ToolbarItem] = &[
            item("undo", "undo"),
            item("redo", "redo"),
            item("reset", "reset"),
        ];
        const TWICE: &[ToolbarItem] = &[item("undo", "undo"), item("undo", "redo")];
        const EMPTY: &[ToolbarItem] = &[item("", "undo")];
        const NO_ICON: &[ToolbarItem] = &[ToolbarItem {
            action: "undo",
            icon: None,
        }];
        let mut offering = with(&[]);
        offering.tool.toolbar = THREE;
        assert_eq!(offering.check(), Ok(()));
        offering.tool.toolbar = TWICE;
        assert_eq!(
            offering.check(),
            Err(DefinitionError::DuplicateToolbarItem("undo"))
        );
        offering.tool.toolbar = EMPTY;
        assert_eq!(
            offering.check(),
            Err(DefinitionError::UnknownToolbarAction(""))
        );
        // A utility has no button to take an icon from.
        offering.tool.toolbar = NO_ICON;
        assert_eq!(offering.check(), Err(DefinitionError::ToolbarIcon("undo")));
    }

    /// Every row that edits is refused, by what edits in it, inside a group too.
    #[test]
    fn a_row_that_edits_is_refused() {
        let refused: [(&'static [Row], &str); 15] = [
            (&[Row::Slider(SLIDER)], "brightness"),
            (
                &[Row::Choice {
                    control: "mode",
                    style: ChoiceStyle::List,
                    end: &[],
                }],
                "mode",
            ),
            (
                &[Row::Toggle {
                    control: "on",
                    end: &[],
                }],
                "on",
            ),
            (
                &[Row::When {
                    control: "mode",
                    is: &["fast"],
                    rows: &[GRAPH_ROW],
                }],
                "mode",
            ),
            (
                &[Row::Line {
                    start: LineStart::Control("turn"),
                    readout: None,
                    end: &[],
                }],
                "turn",
            ),
            (
                &[Row::Custom {
                    widget: Widget::Equalizer,
                    controls: &["band"],
                }],
                "band",
            ),
            (
                &[Row::Preset {
                    preset: "look",
                    style: ChoiceStyle::List,
                }],
                "look",
            ),
            (
                &[Row::Line {
                    start: LineStart::None,
                    readout: None,
                    end: &[End::Picker("spot")],
                }],
                "pickers",
            ),
            // A line holding none still edits.
            (
                &[Row::Line {
                    start: LineStart::None,
                    readout: None,
                    end: &[],
                }],
                "pickers",
            ),
            (
                &[Row::Patches {
                    picker: "area",
                    target: &[],
                }],
                "area",
            ),
            (
                &[Row::Reference {
                    picker: "measure",
                    lightness: "lightness",
                    photo: "photo",
                }],
                "measure",
            ),
            (&[Row::LastPick { picker: "spot" }], "spot"),
            (&[Row::Computed(COMPUTED)], "level"),
            (&[Row::Guides], "guides"),
            (
                &[Row::Group(Group {
                    id: "more",
                    kind: GroupKind::Collapsible,
                    rows: &[Row::Toggle {
                        control: "on",
                        end: &[],
                    }],
                })],
                "on",
            ),
        ];
        for (rows, by) in refused {
            assert_eq!(with(rows).check(), Err(DefinitionError::Edits(by)), "{by}");
        }
    }

    /// Its page is one untitled tab; its groups and settings each once, a collapsible group
    /// holding no collapsible one; rows following a setting follow an option of one it has; its menus
    /// are a tool's.
    #[test]
    fn its_page_is_one_untitled_tab_its_ids_once_and_its_menus_a_tool_s() {
        let mut titled = with(&[]);
        titled.tool.tabs = &[Tab {
            id: Some("main"),
            rows: &[],
        }];
        assert_eq!(titled.check(), Err(DefinitionError::Tabs));
        const ONE: Row = Row::Group(Group {
            id: "one",
            kind: GroupKind::Plain,
            rows: &[GRAPH_ROW],
        });
        let twice = with(&[ONE, ONE]);
        assert_eq!(twice.check(), Err(DefinitionError::DuplicateGroup("one")));
        let empty = with(&[Row::Group(Group {
            id: "empty",
            kind: GroupKind::Plain,
            rows: &[],
        })]);
        assert_eq!(empty.check(), Err(DefinitionError::EmptyGroup("empty")));
        let nested = with(&[Row::Group(Group {
            id: "outer",
            kind: GroupKind::Collapsible,
            rows: &[Row::Group(Group {
                id: "inner",
                kind: GroupKind::Collapsible,
                rows: &[],
            })],
        })]);
        assert_eq!(nested.check(), Err(DefinitionError::GroupInGroup("inner")));
        let settings = with(&[Row::Setting(GRAPH), Row::Setting(GRAPH)]);
        let twice = Err(DefinitionError::DuplicateSetting("graph"));
        assert_eq!(settings.check(), twice);
        let follows = Err(DefinitionError::SettingCondition("graph"));
        let unknown = with(&[Row::WhenSetting {
            setting: "graph",
            is: &["lines"],
            rows: &[GRAPH_ROW],
        }]);
        assert_eq!(unknown.check(), follows);
        let no_option = with(&[
            Row::Setting(GRAPH),
            Row::WhenSetting {
                setting: "graph",
                is: &["dots"],
                rows: &[GRAPH_ROW],
            },
        ]);
        assert_eq!(no_option.check(), follows);
        // Its own setting among the rows it hides, at any depth, would hide itself.
        let hides_itself = with(&[Row::WhenSetting {
            setting: "graph",
            is: &["lines"],
            rows: &[Row::Group(Group {
                id: "more",
                kind: GroupKind::Collapsible,
                rows: &[Row::Setting(GRAPH)],
            })],
        }]);
        assert_eq!(hides_itself.check(), follows);
        // Two settings, each among the rows the other hides, both at their defaults
        // hiding the other: neither ever shows.
        const LINES: SettingRow = SettingRow {
            id: "lines",
            ..GRAPH
        };
        let hidden = with(&[
            Row::WhenSetting {
                setting: "lines",
                is: &["lines"],
                rows: &[Row::Setting(GRAPH)],
            },
            Row::WhenSetting {
                setting: "graph",
                is: &["lines"],
                rows: &[Row::Setting(LINES)],
            },
        ]);
        assert_eq!(hidden.check(), Err(DefinitionError::Hidden("lines")));
        // One shown outside any condition, the other among the rows it hides, beside the
        // rows that follow it: both show where those rows do.
        let shown = with(&[
            Row::Setting(GRAPH),
            Row::WhenSetting {
                setting: "graph",
                is: &["lines"],
                rows: &[
                    Row::Setting(LINES),
                    Row::WhenSetting {
                        setting: "lines",
                        is: &["lines"],
                        rows: &[GRAPH_ROW],
                    },
                ],
            },
        ]);
        assert_eq!(shown.check(), Ok(()));
        // A lone tab draws no tab strip, so a setting on it shows beside the rows after it.
        let mut beside = with(&[Row::Setting(GRAPH)]);
        beside.tool.after = &[Row::WhenSetting {
            setting: "graph",
            is: &["lines"],
            rows: &[GRAPH_ROW],
        }];
        assert_eq!(beside.check(), Ok(()));
        let mut menus = with(&[]);
        const TWICE: &[MenuItem] = &[
            option(Section::HISTORY, 100, "compress-history"),
            option(Section::HISTORY, 200, "compress-history"),
        ];
        menus.tool.menus = TWICE;
        let listed_twice = Err(DefinitionError::DuplicateMenuItem("compress-history"));
        assert_eq!(menus.check(), listed_twice);
    }

    /// Declared words for a choice of two, Bars and a name.
    const SHOWN: crate::SettingWords = crate::SettingWords {
        label: "sample-graph",
        full: None,
        promise: Some("sample-graph-tip"),
        options: &[Word::Message("sample-bars"), Word::Name("RGB")],
    };

    /// GRAPH kept under a key of its own, with its words declared.
    const KEYED: SettingRow = SettingRow {
        key: Some("sample.view.graph"),
        words: Some(&SHOWN),
        ..GRAPH
    };

    /// A utility's setting may name its key in full, dotted snake_case, and declare its
    /// words, one for each option; a processing tool's may do neither.
    #[test]
    fn a_utility_s_setting_names_its_key_in_full_with_its_words() {
        assert_eq!(with(&[Row::Setting(KEYED)]).check(), Ok(()));
        const SPACED: SettingRow = SettingRow {
            key: Some("sample.View"),
            ..KEYED
        };
        let bad_key = Err(DefinitionError::Key("sample.View"));
        assert_eq!(with(&[Row::Setting(SPACED)]).check(), bad_key);
        const ONE: crate::SettingWords = crate::SettingWords {
            options: &[Word::Message("sample-bars")],
            ..SHOWN
        };
        const SHORT: SettingRow = SettingRow {
            words: Some(&ONE),
            ..KEYED
        };
        let short = Err(DefinitionError::SettingWords("graph"));
        assert_eq!(with(&[Row::Setting(SHORT)]).check(), short);
        // A boolean names No and Yes, or nothing, as a toggle.
        const NO_YES: crate::SettingWords = crate::SettingWords {
            options: &[Word::Message("no"), Word::Message("yes")],
            ..SHOWN
        };
        const ON: SettingRow = SettingRow {
            id: "on",
            kind: ControlKind::Bool { default: false },
            words: Some(&NO_YES),
            ..KEYED
        };
        assert!(ON.no_or_yes());
        assert!(!GRAPH.no_or_yes());
        assert_eq!(with(&[Row::Setting(ON)]).check(), Ok(()));
    }

    /// Its titles and the groups open at first are its page's own.
    #[test]
    fn its_titles_and_open_groups_are_its_page_s() {
        const ROWS: &[Row] = &[
            Row::Group(Group {
                id: "first",
                kind: GroupKind::Plain,
                rows: &[GRAPH_ROW],
            }),
            Row::Group(Group {
                id: "more",
                kind: GroupKind::Collapsible,
                rows: &[Row::Setting(GRAPH)],
            }),
        ];
        let mut titled = with(ROWS);
        titled.titles = &[("first", "sample-first"), ("more", "sample-more")];
        titled.open = &["more"];
        assert_eq!(titled.check(), Ok(()));
        assert_eq!(titled.title("first", GroupKind::Plain), "sample-first");
        assert_eq!(titled.title("more", GroupKind::Collapsible), "sample-more");
        assert!(titled.opens("more"));
        let plain = with(ROWS);
        assert_eq!(
            plain.title("first", GroupKind::Plain),
            "tool-sample-group-first"
        );
        assert_eq!(
            plain.title("more", GroupKind::Collapsible),
            "tool-sample-collapsible-more"
        );
        assert!(!plain.opens("more"));
        let mut unknown = with(ROWS);
        unknown.titles = &[("last", "sample-last")];
        assert_eq!(unknown.check(), Err(DefinitionError::UnknownTitle("last")));
        let mut twice = with(ROWS);
        twice.titles = &[("more", "sample-more"), ("more", "sample-other")];
        assert_eq!(twice.check(), Err(DefinitionError::DuplicateTitle("more")));
        // A plain group is always open, so never opens at first.
        let mut plain_open = with(ROWS);
        plain_open.open = &["first"];
        assert_eq!(
            plain_open.check(),
            Err(DefinitionError::UnknownTitle("first"))
        );
        // A group with no rows yet is a placeholder, drawn closed: it may be on the page,
        // but never open at first.
        const PLACEHOLDER: &[Row] = &[Row::Group(Group {
            id: "later",
            kind: GroupKind::Collapsible,
            rows: &[],
        })];
        assert_eq!(with(PLACEHOLDER).check(), Ok(()));
        let mut empty = with(PLACEHOLDER);
        empty.open = &["later"];
        assert_eq!(empty.check(), Err(DefinitionError::OpensEmpty("later")));
        // A title is found by its id alone, so two groups never share one, whatever their
        // kinds.
        let shared = with(&[
            Row::Group(Group {
                id: "more",
                kind: GroupKind::Plain,
                rows: &[GRAPH_ROW],
            }),
            Row::Group(Group {
                id: "more",
                kind: GroupKind::Collapsible,
                rows: &[Row::Setting(GRAPH)],
            }),
        ]);
        assert_eq!(shared.check(), Err(DefinitionError::DuplicateGroup("more")));
    }

    /// Every key a utility's setting names in full is a setting of the settings schema
    /// (decision 93), so a key there is never misspelt here: an id its parser reads, never
    /// a table that only holds settings.
    #[test]
    fn every_key_a_utility_names_is_in_the_settings_schema() {
        const SCHEMA: &str = include_str!("../../../data/settings.schema.toml");
        let schema = apertrust_base::settings::codegen::parse(SCHEMA, "settings.schema.toml");
        let schema = schema.unwrap_or_else(|errors| panic!("the schema: {}", errors.len()));
        let ids: std::collections::BTreeSet<&str> = schema.ids().collect();
        fn keys(rows: &[Row], into: &mut Vec<&'static str>) {
            for row in rows {
                if let Row::Setting(SettingRow { key: Some(key), .. }) = row {
                    into.push(key);
                }
                keys(row.rows(), into);
            }
        }
        let mut named = Vec::new();
        for utility in UTILITIES {
            let tool = &utility.tool;
            for rows in [tool.before, tool.tabs[0].rows, tool.after] {
                keys(rows, &mut named);
            }
        }
        let declared = |key: &str| ids.contains(key);
        let missing: Vec<&str> = named.into_iter().filter(|key| !declared(key)).collect();
        assert!(missing.is_empty(), "not in the schema: {missing:?}");
        assert!(declared("export.format") && !declared("export.formats"));
        // A table that only holds settings is none.
        assert!(!declared("export") && !declared("export.jpeg"));
    }
}
