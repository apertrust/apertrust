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

//! The utilities (`docs/gui.md`, The side panel): side-panel sections that edit nothing,
//! Scopes and History, with no [`ProcessingDefinition`](crate::ProcessingDefinition). Each
//! is listed in [`UTILITIES`] with its words, its sentence `tool-<id>-description` as a
//! tool's, what it measures, which its header's tooltip says ([`crate::about`]), and the
//! items it adds to context menus (decision 91).

use crate::about::WorksOn;
use crate::{MenuEntry, MenuItem, Place, Section, ToolDefinition};

/// A utility section, which edits nothing (decision 93): the [`ToolDefinition`] every tool
/// embeds, its page empty, and what it measures, if anything.
#[derive(Debug, Clone, Copy)]
pub struct UtilityDefinition {
    /// Its id, whose sentence is `tool-<id>-description`, and the items it adds to context
    /// menus (decision 91), each the registry action that runs it, by its id, at a place,
    /// held to a tool's limits ([`crate::check_menus`]); none for a utility with nothing to
    /// add. The app builds the menus, every item where its section and order put it
    /// ([`crate::Section`]).
    pub tool: ToolDefinition,
    pub measures: Option<WorksOn>,
}

/// The utilities built: Scopes, which measures the finished picture, and History, which adds
/// Reset to the photo's and a thumbnail's menus.
pub const UTILITIES: &[UtilityDefinition] = &[
    UtilityDefinition {
        tool: ToolDefinition {
            id: "scopes",
            tabs: &[],
            before: &[],
            after: &[],
            menus: &[],
        },
        measures: Some(WorksOn::Display),
    },
    UtilityDefinition {
        tool: ToolDefinition {
            id: "history",
            tabs: &[],
            before: &[],
            after: &[],
            // Reset in the photo's and a thumbnail's menus (decision 91).
            menus: &[
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
            ],
        },
        measures: None,
    },
];

/// The utility `id`, if it is one.
pub fn utility(id: &str) -> Option<&'static UtilityDefinition> {
    UTILITIES.iter().find(|utility| utility.tool.id == id)
}
