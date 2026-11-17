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

//! The utilities (`docs/gui.md`, The side panel, One tool model): side-panel tools that
//! edit nothing, Scopes, History and Browse, with no
//! [`ProcessingDefinition`](crate::ProcessingDefinition). Each is listed in [`UTILITIES`]
//! with its words, its sentence `tool-<id>-description` as a tool's, what it measures,
//! which its header's tooltip says ([`crate::about`]), what its header's ↺ does, its page
//! and the items it adds to context menus and its ⋯ (decision 91). The app draws each
//! one's page through its `UtilityTool`, its widgets the page's custom rows (decision 93).

use crate::about::WorksOn;
use crate::{MenuEntry, MenuItem, Place, Reset, Row, Section, Tab, ToolDefinition, Widget};

/// A utility tool, which edits nothing (decision 93): the [`ToolDefinition`] every tool
/// embeds, and what it measures, if anything.
#[derive(Debug, Clone, Copy)]
pub struct UtilityDefinition {
    /// Its id, whose sentence is `tool-<id>-description`; its header's ↺, a registry
    /// action's, its own or none; its page, one custom row its app module draws whole until
    /// the page is rows (step 93.4); and the items it adds to context menus and its ⋯
    /// (decision 91), each the registry action that runs it, by its id, at a place, held to
    /// a tool's limits ([`crate::check_menus`]); none for a utility with nothing to add. The
    /// app builds the menus, every item where its section and order put it ([`Section`]).
    pub tool: ToolDefinition,
    pub measures: Option<WorksOn>,
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
/// the destructive one; and Browse, the Photos tab's first. Each page is one custom row
/// until step 93.4 makes it rows.
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
            id: "browse",
            tabs: &[Tab {
                id: None,
                rows: &[Row::Custom {
                    widget: Widget::Browse,
                    controls: &[],
                }],
            }],
            before: &[],
            after: &[],
            reset: Reset::None,
            presets: false,
            menus: &[],
        },
        measures: None,
    },
];

/// The utility `id`, if it is one.
pub fn utility(id: &str) -> Option<&'static UtilityDefinition> {
    UTILITIES.iter().find(|utility| utility.tool.id == id)
}
