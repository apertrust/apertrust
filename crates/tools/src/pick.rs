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
//! [`crate::ProcessingTool::pick`] takes a [`Reading`] and gives what it writes and shows;
//! plain data, so they move into the protocol unchanged.

use apertrust_base::settings::Choice;

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

/// darktable's half side across of the area a press on the photo starts, as a share of
/// the photo's width (`src/views/darkroom.c:4064-4073`), at every zoom.
pub const PRESSED: f32 = 0.02;

impl Region {
    /// The area a press at `at` starts on the photo for an area picker, as darktable's
    /// press away from an area's corners starts one (`src/views/darkroom.c:4064-4073`):
    /// 2 % of the photo's width each way of the point and as tall in its pixels, `aspect`
    /// its width over its height, clamped to the photo. The app's press and the session's
    /// one-shot pick both make it here, so the two never differ. Left, top, right, bottom,
    /// as [`Region::Area`] holds them.
    pub fn pressed(at: [f32; 2], aspect: f32) -> [f32; 4] {
        Region::around(at, [PRESSED, PRESSED * aspect])
    }

    /// The area of half sides `half` round `at`, across as a share of the photo's width and
    /// down of its height, clamped to the photo as [`Region::pressed`]'s: the app's click's
    /// area at its size on screen at Fit (decision 105). Left, top, right, bottom.
    pub fn around(at: [f32; 2], [across, down]: [f32; 2]) -> [f32; 4] {
        // A coordinate that is not a number at 0, as the session clamps a region.
        let clamped = |v: f32| if v >= 0.0 { v.min(1.0) } else { 0.0 };
        [
            clamped(at[0] - across),
            clamped(at[1] - down),
            clamped(at[0] + across),
            clamped(at[1] + down),
        ]
    }
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

/// What a tool's picker measured at its operation, as [`crate::ProcessingTool::pick`] reads
/// it: the operation's input and, with the picker's [`crate::Picker::output`], its output,
/// each in the picker's space. A region that misses the operation's image measures nothing,
/// and gives no reading to pick from.
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

/// What a tool's pick makes of a reading ([`crate::ProcessingTool::pick`]): darktable's
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

/// What a readout at a line of pickers' start shows ([`crate::ProcessingTool::readout`],
/// [`crate::ReadoutDef`]): one of its texts, with its number, and its swatch's colour, as
/// Colour calibration's "CCT: 5003 K (daylight)" and the illuminant's colour beside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Readout {
    /// Which of its texts, by id.
    pub text: &'static str,
    /// The number its text shows, its `$value`; `None` for a text with none.
    pub value: Option<f32>,
    /// The swatch's colour, each channel 0 to 1, handed to the screen as it is, as cairo
    /// paints darktable's; `None` for a readout with no swatch.
    pub swatch: Option<[f32; 3]>,
}

/// darktable's area mode (`dt_spot_mode_t`), what an area mapping's picker does with the
/// area it reads, a tool's setting: Colour calibration's `tools.channelmixerrgb.area_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AreaMode {
    /// darktable's correction, its default: the pick sets what maps the area to the
    /// target.
    #[default]
    Correction,
    /// darktable's measure: the pick writes no control and hands back the target the area
    /// renders at.
    Measure,
}

impl AreaMode {
    /// The option a setting's value names, correction where it names none.
    pub fn of(value: Option<&ControlValue>) -> Self {
        match value {
            Some(&ControlValue::Choice(at)) => Self::OPTIONS
                .get(at as usize)
                .map_or(Self::Correction, |(mode, _)| *mode),
            _ => Self::Correction,
        }
    }
}

impl Choice for AreaMode {
    const OPTIONS: &'static [(Self, &'static str)] =
        &[(Self::Correction, "correction"), (Self::Measure, "measure")];
}

#[cfg(test)]
mod tests {
    use super::*;

    /// darktable's area modes, correction first and the default, and a setting's value
    /// read as one, correction where it names none.
    #[test]
    fn area_mode_is_darktable_s_correction_or_measure() {
        let ids: Vec<&str> = AreaMode::OPTIONS.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, ["correction", "measure"]);
        assert_eq!(AreaMode::default(), AreaMode::Correction);
        let of = |value| AreaMode::of(value);
        assert_eq!(of(Some(&ControlValue::Choice(1))), AreaMode::Measure);
        assert_eq!(of(Some(&ControlValue::Choice(0))), AreaMode::Correction);
        assert_eq!(of(Some(&ControlValue::Choice(7))), AreaMode::Correction);
        assert_eq!(of(None), AreaMode::Correction);
    }

    /// A press's area against darktable's `fbox` (`src/views/darkroom.c:4064-4073`), `dx`
    /// 0.02 and `dy` scaled by the processed width over the height, at a few sizes, and
    /// clamped to the photo at its edges.
    #[test]
    fn a_press_starts_darktable_s_area_clamped_to_the_photo() {
        #[rustfmt::skip]
        let cases: [([u32; 2], [f32; 2], [f32; 4]); 6] = [
            // Landscape, 3:2: as tall as 3 % of the height.
            ([6000, 4000], [0.5, 0.5], [0.48, 0.47, 0.52, 0.53]),
            // Portrait, 2:3.
            ([4000, 6000], [0.25, 0.75], [0.23, 0.75 - 0.04 / 3.0, 0.27, 0.75 + 0.04 / 3.0]),
            // Square.
            ([1000, 1000], [0.5, 0.3], [0.48, 0.28, 0.52, 0.32]),
            // Wide, 16:9.
            ([1920, 1080], [0.7, 0.4], [0.68, 0.4 - 0.32 / 9.0, 0.72, 0.4 + 0.32 / 9.0]),
            // Near the edges, held to the photo.
            ([6000, 4000], [0.01, 0.99], [0.0, 0.96, 0.03, 1.0]),
            ([6000, 4000], [1.0, 0.0], [0.98, 0.0, 1.0, 0.03]),
        ];
        for ([width, height], at, want) in cases {
            let aspect = width as f32 / height as f32;
            let area = Region::pressed(at, aspect);
            let near = area.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6);
            assert!(near, "{width}×{height} at {at:?}: {area:?}, not {want:?}");
            // The same area as `around` at darktable's share, to the bit.
            assert_eq!(Region::around(at, [PRESSED, PRESSED * aspect]), area);
        }
    }

    /// An area round a point, of any half sides, clamped to the photo at each edge.
    #[test]
    fn around_makes_an_area_of_its_half_sides_clamped_to_the_photo() {
        let cases: [([f32; 2], [f32; 2], [f32; 4]); 3] = [
            ([0.5, 0.5], [0.25, 0.125], [0.25, 0.375, 0.75, 0.625]),
            // Held to the photo at its left and bottom edges, then its right and top.
            ([0.05, 0.98], [0.1, 0.05], [0.0, 0.93, 0.15, 1.0]),
            ([0.95, 0.02], [0.1, 0.05], [0.85, 0.0, 1.0, 0.07]),
        ];
        for (at, half, want) in cases {
            let area = Region::around(at, half);
            let near = area.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6);
            assert!(near, "{at:?} by {half:?}: {area:?}, not {want:?}");
        }
        // A coordinate that is not a number at 0.
        assert_eq!(Region::around([f32::NAN, 0.5], [0.1, 0.0])[..2], [0.0, 0.5]);
    }
}
