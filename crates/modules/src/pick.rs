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
//! area on the photo, the statistics of a reading and the keys held as it reads, and what
//! a module's pick makes of a reading. The engine's session takes and gives them, and
//! [`crate::Module::pick`] takes a [`Reading`] and gives what it writes and shows; plain
//! data, so they move into the protocol unchanged.

use crate::{ControlValue, Controls};

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

/// What a module's picker measured at its operation, as [`crate::Module::pick`] reads it:
/// the operation's input and, with the picker's [`crate::Picker::output`], its output,
/// each in the picker's space. A region that misses the operation's image measures
/// nothing, and gives no reading to pick from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub input: Stats,
    /// With the picker's `output` only.
    pub output: Option<Stats>,
}

/// What a module's pick makes of a reading ([`crate::Module::pick`]): darktable's
/// `color_picker_apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    /// Each control it writes with its value, among the picker's
    /// [`crate::Picker::writes`]: every control darktable's `color_picker_apply` writes,
    /// those it keeps consistent included, since no follow write runs after a pick.
    pub writes: Controls,
    pub shown: Shown,
}

/// What the module's panel shows of a reading.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shown {
    /// The input patch, where the module shows one.
    pub patch: Option<Patch>,
    /// The tool settings measure mode set, as darktable's measure mode writes its target
    /// sliders; the client keeps them in its settings.
    pub tools: Vec<(&'static str, ControlValue)>,
}

/// A patch of picked colour, as Exposure and Colour calibration show the area they read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    /// Its colour for the screen: sRGB, encoded, each channel 0 to 1.
    pub srgb: [f32; 3],
    /// Its L, C and h: CIE LCh of Lab D50, L 0 to 100, h in degrees from 0 to 360.
    pub lch: [f32; 3],
}
