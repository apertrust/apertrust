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

//! What a colour picker reads and where (decision 79; `docs/pickers.md`): its point or
//! area on the photo, the statistics of a reading and the keys held as it reads. The
//! engine's session takes and gives them, and `Module::pick` is to; plain data, so they
//! move into the protocol unchanged.

/// A picker's point or area in the photo's coordinates: 0 to 1 across the photo as export
/// renders it, after crop and the geometry, (0, 0) its top left, x to the right and y
/// down. They do not depend on the zoom, the window or the photo's size, so a script and
/// the app name the same place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Region {
    Point([f32; 2]),
    /// Left, top, right, bottom.
    Area([f32; 4]),
}

/// darktable's `lib_colorpicker_stats` (`src/common/color_picker.c`): per channel over a
/// region, the mean, the sum over its pixels divided by their number, and the exact
/// minimum and maximum. In the picker's space; the fourth channel a hue moved half a turn
/// where the space has a hue, else the stage's alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub mean: [f32; 4],
    pub min: [f32; 4],
    pub max: [f32; 4],
}

/// The keys held, which some modules read as darktable reads `dt_key_modifier_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PickKeys {
    /// Ctrl, Cmd on the Mac.
    pub command: bool,
    pub shift: bool,
}
