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

//! Cursors (decision 104, `docs/cursors.md`, The API): what a press on the photo does,
//! one chosen at a time in the Toolbar's well. Each is data, as a tool's context menu
//! items are, so a tool declares its picker cursor beside its pickers and runs no code of
//! its own for it; the app's owners, the well and the gestures are the app's.

use std::collections::BTreeSet;

use super::{DefinitionError, ProcessingDefinition, Row, is_id};

/// One cursor of the well.
#[derive(Debug, Clone, Copy)]
pub struct CursorDef {
    /// snake_case, unique among its owner's cursors: `spot`, `select`, `brush`.
    pub id: &'static str,
    pub group: CursorGroup,
    /// One of `apertrust-ui`'s icons by name, as a toolbar item names its icon.
    pub icon: &'static str,
    /// Its default key, the trigger as text in Qt's portable form ("K"), or none.
    pub key: Option<&'static str>,
    /// Its place in its group, 100 apart.
    pub order: u16,
    /// What it takes on the photo; none for a picker cursor, whose picker's kinds decide.
    pub gestures: &'static [Gesture],
    pub surfaces: &'static [Surface],
    /// Its amounts: setting rows of the settings schema, as a tool's `Row::Setting`.
    pub options: &'static [Row],
}

/// The groups, in the well's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CursorGroup {
    Select,
    Pan,
    Pick,
    Mask,
    Retouch,
    Annotate,
    CopyApply,
}

/// A group's own settings, toggles and choices, declared once by its owner.
#[derive(Debug, Clone, Copy)]
pub struct CursorGroupDef {
    pub group: CursorGroup,
    /// `Row::Setting`s naming settings of `[cursors.<group>]`.
    pub settings: &'static [Row],
}

/// What a cursor takes on the photo (`docs/cursors.md`, On the photo).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// A press and release that moves less than the drag threshold.
    Click,
    /// A drag, from the press to the release.
    Area,
    /// A drag whose direction counts: a gradient.
    Shape,
    /// The pointer's path while pressed.
    Stroke,
    /// Clicks and drags that build one shape until it is closed: a path.
    Points,
}

/// Where a cursor acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Viewer,
    Browser,
}

/// A tool's picker cursor: chosen exactly while `picker` is on; choosing it turns `picker`
/// on, as the picker's button does, and a press on the photo is the picker's.
#[derive(Debug, Clone, Copy)]
pub struct ToolCursor {
    /// Its declaration, with no gestures: its picker's kinds decide what a press does.
    pub cursor: CursorDef,
    /// One of the tool's pickers ([`ProcessingDefinition::pickers`]).
    pub picker: &'static str,
}

impl ProcessingDefinition {
    /// Whether the tool's cursors hold together (`docs/cursors.md`, The API): each has an
    /// id of its own, turns on one of the tool's pickers, each picker with one cursor at
    /// most, is in Pick, declares no gestures, acts on the Viewer alone and has no amounts.
    pub(super) fn check_cursors(&self) -> Result<(), DefinitionError> {
        let mut ids = BTreeSet::new();
        let mut pickers = BTreeSet::new();
        for ToolCursor { cursor, picker } in self.tool.cursors {
            if !is_id(cursor.id) {
                return Err(DefinitionError::Id(cursor.id));
            }
            if !ids.insert(cursor.id) {
                return Err(DefinitionError::DuplicateCursor(cursor.id));
            }
            if self.picker(picker).is_none() {
                return Err(DefinitionError::CursorPicker(cursor.id));
            }
            if !pickers.insert(*picker) {
                return Err(DefinitionError::CursorsOfPicker(picker));
            }
            if cursor.group != CursorGroup::Pick {
                return Err(DefinitionError::CursorGroup(cursor.id));
            }
            if !cursor.gestures.is_empty() {
                return Err(DefinitionError::CursorGestures(cursor.id));
            }
            if cursor.surfaces != [Surface::Viewer] {
                return Err(DefinitionError::CursorSurfaces(cursor.id));
            }
            if !cursor.options.is_empty() {
                return Err(DefinitionError::CursorOptions(cursor.id));
            }
        }
        Ok(())
    }
}
