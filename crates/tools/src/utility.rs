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
//! panel's tools that edit nothing, Scopes, History, Collections and Export, with no
//! [`ProcessingDefinition`](crate::ProcessingDefinition). Each is listed in [`UTILITIES`]
//! with its words, its sentence `tool-<id>-description` as a tool's, what it measures,
//! which its header's tooltip says ([`crate::about`]), what its header's ↺ does, its page
//! and the items it adds to context menus and its ⋯ (decision 91). The app draws each
//! one's page through its `UtilityTool`, its widgets the page's custom rows (decision 93).

use std::collections::BTreeSet;

use crate::about::WorksOn;
use crate::definition::{is_id, shows_setting};
use crate::{
    ControlKind, DefinitionError, MenuEntry, MenuItem, Place, Reset, Row, Section, Tab,
    ToolDefinition, Widget, check_menus,
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
    /// it ([`Section`]).
    pub tool: ToolDefinition,
    pub measures: Option<WorksOn>,
}

impl UtilityDefinition {
    /// Whether the utility holds together (decision 93): its id; its header's ↺ a registry
    /// action's, its own or none, never a processing tool's, a presets button allowed, drawn
    /// disabled until its presets are built (step 93.6); its page one untitled tab, since
    /// its header draws no tab strip; no row that edits, so its rows are settings, labels,
    /// groups, rows following a setting and custom rows naming no control, each a utility's
    /// own widget; no action row until a utility's are bound to the registry; each label,
    /// group and setting once by its id, a group holding no group, each setting's kind as a
    /// control's; rows following a setting following a choice setting of its own, shown
    /// outside them and wherever they show, for options it has, with rows of their own; and
    /// its context menu items held to a tool's limits ([`check_menus`]).
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
            self.check_rows(rows, false, &mut seen)?;
        }
        // Each setting rows follow shows wherever they do, never hidden by rows it hides.
        if let Some(setting) = tool.hidden_setting() {
            return Err(DefinitionError::Hidden(setting));
        }
        check_menus(tool.menus)
    }

    /// Checks `rows`, `grouped` while inside a group, as [`Self::check`] says.
    fn check_rows(
        &self,
        rows: &'static [Row],
        grouped: bool,
        seen: &mut Seen,
    ) -> Result<(), DefinitionError> {
        for row in rows {
            if let Some(by) = edits(row) {
                return Err(DefinitionError::Edits(by));
            }
            match row {
                Row::Label { id } => once(&mut seen.labels, id, DefinitionError::DuplicateLabel)?,
                Row::Group { id, .. } => {
                    if grouped {
                        return Err(DefinitionError::GroupInGroup(id));
                    }
                    once(&mut seen.groups, id, DefinitionError::DuplicateGroup)?;
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
                Row::Action { id } => return Err(DefinitionError::UtilityAction(id)),
                _ => {}
            }
            let grouped = grouped || matches!(row, Row::Group { .. });
            self.check_rows(row.rows(), grouped, seen)?;
        }
        Ok(())
    }
}

/// What edits in `row`, if anything, for a utility, which edits nothing (decision 93): a
/// control it names, a slider's, a choice's, a toggle's, the one rows follow, a button
/// row's or a custom row's; a preset row, which writes controls; a picker's rows, a row of
/// pickers whatever it holds; a computed value, which an operation computes; and the
/// guides, a processing tool's. None for a setting, a label, a group, rows following a
/// setting, an action and a custom row naming no control.
fn edits(row: &Row) -> Option<&'static str> {
    match row {
        Row::Slider(slider) => Some(slider.control),
        Row::Choice { control, .. }
        | Row::Toggle { control }
        | Row::When { control, .. }
        | Row::Buttons { control, .. } => Some(control),
        Row::Custom { controls, .. } => controls.first().copied(),
        Row::Preset { preset, .. } => Some(preset),
        Row::Pickers { .. } => Some("pickers"),
        Row::Patches { picker, .. } | Row::Reference { picker, .. } | Row::LastPick { picker } => {
            Some(picker)
        }
        Row::Computed(computed) => Some(computed.id),
        Row::Guides => Some("guides"),
        Row::Label { .. }
        | Row::Group { .. }
        | Row::WhenSetting { .. }
        | Row::Setting(_)
        | Row::Action { .. } => None,
    }
}

/// The ids a utility's page has used, by kind, each once.
#[derive(Default)]
struct Seen {
    labels: BTreeSet<&'static str>,
    groups: BTreeSet<&'static str>,
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
/// the destructive one; Collections, the Photos tab's first; and Export, its last, whose ↺
/// is its own Default settings and whose presets button is drawn disabled until its presets
/// come. Their pages are their own widgets as custom rows, which keep what each drew before
/// (decision 93): Scopes' the graph with its buttons, History's its steps, Collections' its
/// recents line and its Folders, and Export's its groups and its end, until its rows are
/// setting rows (step 93.6).
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
            menus: &[],
        },
        measures: Some(WorksOn::Display),
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
        },
        measures: None,
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
            menus: &[],
        },
        measures: None,
    },
    UtilityDefinition {
        tool: ToolDefinition {
            id: "export",
            tabs: &[Tab {
                id: None,
                rows: &[
                    Row::Custom {
                        widget: Widget::ExportStorage,
                        controls: &[],
                    },
                    Row::Custom {
                        widget: Widget::ExportFormat,
                        controls: &[],
                    },
                    Row::Custom {
                        widget: Widget::ExportGlobal,
                        controls: &[],
                    },
                    Row::Custom {
                        widget: Widget::ExportLater,
                        controls: &[],
                    },
                    Row::Custom {
                        widget: Widget::ExportEnd,
                        controls: &[],
                    },
                ],
            }],
            before: &[],
            after: &[],
            reset: Reset::Own,
            presets: true,
            menus: &[],
        },
        measures: None,
    },
];

/// The utility `id`, if it is one.
pub fn utility(id: &str) -> Option<&'static UtilityDefinition> {
    UTILITIES.iter().find(|utility| utility.tool.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChoiceStyle, Computed, SettingRow, Slider, Track};

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
                menus: &[],
            },
            measures: None,
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
    };

    const SLIDER: Slider = Slider {
        control: "brightness",
        decimals: 2,
        soft: None,
        track: Track::Plain,
        fill: false,
        quad: None,
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

    /// Settings, labels, groups, rows following a setting shown beside them, and custom
    /// rows of a utility's widgets naming no control: what a utility holds.
    #[test]
    fn a_utility_holds_settings_labels_groups_and_custom_rows() {
        let utility = with(&[
            Row::Setting(GRAPH),
            Row::WhenSetting {
                setting: "graph",
                is: &["lines"],
                rows: &[Row::Label { id: "lines" }],
            },
            Row::Group {
                id: "more",
                rows: &[Row::Custom {
                    widget: Widget::Scopes,
                    controls: &[],
                }],
            },
        ]);
        assert_eq!(utility.check(), Ok(()));
    }

    /// An action row waits for a utility's to be bound to the registry; a processing
    /// tool's widget is drawn over its controls, which a utility has none of.
    #[test]
    fn a_utility_has_no_action_row_yet_and_no_processing_tool_s_widget() {
        let action = with(&[Row::Action { id: "refresh" }]);
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

    /// A utility's ↺ is never a processing tool's; it may have a presets button, drawn
    /// disabled until its presets are built.
    #[test]
    fn a_utility_s_header_has_no_tool_reset_and_may_have_presets() {
        let mut reset = with(&[]);
        reset.tool.reset = Reset::Tool;
        assert_eq!(reset.check(), Err(DefinitionError::UtilityHeader));
        let mut presets = with(&[]);
        presets.tool.presets = true;
        assert_eq!(presets.check(), Ok(()));
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
                }],
                "mode",
            ),
            (&[Row::Toggle { control: "on" }], "on"),
            (
                &[Row::When {
                    control: "mode",
                    is: &["fast"],
                    rows: &[Row::Label { id: "fast" }],
                }],
                "mode",
            ),
            (
                &[Row::Buttons {
                    control: "turn",
                    buttons: &[],
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
                &[Row::Pickers {
                    label: None,
                    pickers: &["spot"],
                }],
                "pickers",
            ),
            // A row of pickers holding none still edits.
            (
                &[Row::Pickers {
                    label: None,
                    pickers: &[],
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
                &[Row::Group {
                    id: "more",
                    rows: &[Row::Toggle { control: "on" }],
                }],
                "on",
            ),
        ];
        for (rows, by) in refused {
            assert_eq!(with(rows).check(), Err(DefinitionError::Edits(by)), "{by}");
        }
    }

    /// Its page is one untitled tab; its labels, groups and settings each once, a group
    /// holding none; rows following a setting follow an option of one it has; its menus
    /// are a tool's.
    #[test]
    fn its_page_is_one_untitled_tab_its_ids_once_and_its_menus_a_tool_s() {
        let mut titled = with(&[]);
        titled.tool.tabs = &[Tab {
            id: Some("main"),
            rows: &[],
        }];
        assert_eq!(titled.check(), Err(DefinitionError::Tabs));
        let twice = with(&[Row::Label { id: "one" }, Row::Label { id: "one" }]);
        assert_eq!(twice.check(), Err(DefinitionError::DuplicateLabel("one")));
        let nested = with(&[Row::Group {
            id: "outer",
            rows: &[Row::Group {
                id: "inner",
                rows: &[],
            }],
        }]);
        assert_eq!(nested.check(), Err(DefinitionError::GroupInGroup("inner")));
        let settings = with(&[Row::Setting(GRAPH), Row::Setting(GRAPH)]);
        let twice = Err(DefinitionError::DuplicateSetting("graph"));
        assert_eq!(settings.check(), twice);
        let follows = Err(DefinitionError::SettingCondition("graph"));
        let unknown = with(&[Row::WhenSetting {
            setting: "graph",
            is: &["lines"],
            rows: &[Row::Label { id: "lines" }],
        }]);
        assert_eq!(unknown.check(), follows);
        let no_option = with(&[
            Row::Setting(GRAPH),
            Row::WhenSetting {
                setting: "graph",
                is: &["dots"],
                rows: &[Row::Label { id: "dots" }],
            },
        ]);
        assert_eq!(no_option.check(), follows);
        // Its own setting among the rows it hides, at any depth, would hide itself.
        let hides_itself = with(&[Row::WhenSetting {
            setting: "graph",
            is: &["lines"],
            rows: &[Row::Group {
                id: "more",
                rows: &[Row::Setting(GRAPH)],
            }],
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
                        rows: &[Row::Label { id: "both" }],
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
            rows: &[Row::Label { id: "lines" }],
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
}
