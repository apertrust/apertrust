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
//! a tool's pick makes of a reading. The engine's session takes and gives them, and
//! [`crate::Tool::pick`] takes a [`Reading`] and gives what it writes and shows; plain
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

/// The keys held, which some tools read as darktable reads `dt_key_modifier_state`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PickKeys {
    /// Ctrl, Cmd on the Mac.
    pub command: bool,
    pub shift: bool,
}

/// What a tool's picker measured at its operation, as [`crate::Tool::pick`] reads it:
/// the operation's input and, with the picker's [`crate::Picker::output`], its output,
/// each in the picker's space. A region that misses the operation's image measures
/// nothing, and gives no reading to pick from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub input: Stats,
    /// With the picker's `output` only.
    pub output: Option<Stats>,
    /// What each of red, green and blue of the operation's input reaches where the
    /// sensor saturated (`ImageDesc::maximum`): 1 after rawprepare, times the white
    /// balance coefficients after temperature, so each channel's own. The session fills it
    /// in, so a pick can tell an area of clipped pixels, a channel at a time (decision 88).
    pub input_maximum: [f32; 3],
}

/// What a tool's pick makes of a reading ([`crate::Tool::pick`]): darktable's
/// `color_picker_apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    /// Each control it writes with its value, among the picker's
    /// [`crate::Picker::writes`]: every control darktable's `color_picker_apply` writes,
    /// those it keeps consistent included, since no follow write runs after a pick.
    pub writes: Controls,
    pub shown: Shown,
}

/// What the tool's panel shows of a reading.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shown {
    /// The input patch, where the tool shows one.
    pub patch: Option<Patch>,
    /// The tool's settings measure mode set, as darktable's measure mode writes its target
    /// sliders; the client keeps them in its settings.
    pub settings: Vec<(&'static str, ControlValue)>,
    /// What the pick did, for a picker that writes controls and a tool that says so: its
    /// Last pick (decision 88).
    pub last: Option<LastPick>,
    /// What went wrong with the pick, each on its own (decision 88).
    pub warnings: PickWarnings,
}

/// What a pick did (decision 88), as Exposure's Last pick shows it, "L 41 % → L 50 % ·
/// +0.55 EV": the area's L as it rendered before the pick and as it renders after, and
/// the change of the control it set, in its unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LastPick {
    pub before: f32,
    pub after: f32,
    /// The change of the control the pick set: Exposure's, in EV.
    pub exposure: f32,
    /// The area as a grey of its L before and after.
    pub patches: [Patch; 2],
}

/// The warnings a pick gave, independent (decision 88): an area of clipped pixels can also
/// fall short, often because it is clipped.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PickWarnings {
    /// Some channel's maximum over the area reaches 0.99 × that channel's maximum at the
    /// input ([`Reading::input_maximum`]): its lightness reads low.
    pub clipped: bool,
    /// The control the pick set is held at the end of its range and the area falls short
    /// of its target.
    pub short: Option<Short>,
}

/// A pick held short of its target (decision 88): "Exposure at +18 EV: the area is L 12 %,
/// not 50 %".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Short {
    /// The value the control is held at, the end of its range.
    pub exposure: f32,
    /// The L the area reaches there.
    pub reached: f32,
    /// The L it was to reach.
    pub target: f32,
    /// Whether the pick moved the black too, to keep it below the new white.
    pub black: bool,
}

/// A patch of picked colour, as Exposure and Colour calibration show the area they read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Patch {
    /// Its colour for the screen: sRGB, encoded, each channel 0 to 1.
    pub srgb: [f32; 3],
    /// Its L, C and h: CIE LCh of Lab D50, L 0 to 100, h in degrees from 0 to 360.
    pub lch: [f32; 3],
}

impl Patch {
    /// The patch of CIE LCh `lch`, h in degrees, as darktable paints a picker's patches
    /// (`_target_color_draw`, `exposure.c:1045-1086`): `dt_LCH_2_Lab`, `dt_Lab_to_XYZ`
    /// and `dt_XYZ_to_sRGB`, held to 0 to 1 as cairo holds a colour.
    pub fn of_lch(lch: [f32; 3]) -> Patch {
        let [l, c, h] = lch;
        let h = h.to_radians();
        let xyz = apertrust_color::lab_to_xyz_d50([l, h.cos() * c, h.sin() * c]);
        Patch {
            srgb: apertrust_color::xyz_d50_to_srgb(xyz).map(|v| v.clamp(0.0, 1.0)),
            lch,
        }
    }
}
