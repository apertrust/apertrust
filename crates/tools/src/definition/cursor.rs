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

use super::{DefinitionError, PickerKind, ProcessingDefinition, Row, is_id};

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

/// A tool's picker cursor: a click or a drag picks once with `picker`.
#[derive(Debug, Clone, Copy)]
pub struct ToolCursor {
    pub cursor: CursorDef,
    /// One of the tool's pickers ([`ProcessingDefinition::pickers`]) that writes controls.
    pub picker: &'static str,
}

impl ProcessingDefinition {
    /// Whether the tool's cursors hold together (`docs/cursors.md`, The API): each has an
    /// id of its own, picks with one of the tool's pickers that writes controls, each
    /// picker with one cursor at most, is in Pick, takes a click, an area or both, an area
    /// only with a picker that takes one, acts on the Viewer alone and has no amounts.
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
            let Some(found) = self.picker(picker).filter(|found| !found.writes.is_empty()) else {
                return Err(DefinitionError::CursorPicker(cursor.id));
            };
            if !pickers.insert(*picker) {
                return Err(DefinitionError::CursorsOfPicker(picker));
            }
            if cursor.group != CursorGroup::Pick {
                return Err(DefinitionError::CursorGroup(cursor.id));
            }
            let takes = |gesture: &Gesture| match gesture {
                Gesture::Click => true,
                Gesture::Area => found.kind != PickerKind::Point,
                _ => false,
            };
            let gestures = cursor.gestures;
            let once = (gestures.iter().enumerate()).all(|(i, g)| !gestures[..i].contains(g));
            let gestures_hold = !gestures.is_empty() && once && gestures.iter().all(takes);
            if !gestures_hold {
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
