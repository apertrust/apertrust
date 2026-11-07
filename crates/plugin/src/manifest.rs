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

//! What the engine needs to know about an operation before running it.

use std::cmp::Ordering;
use std::fmt;

/// An operation's place and properties (architecture decision 20).
#[derive(Debug)]
pub struct Manifest {
    /// darktable's operation name for a port, such as `exposure`; edits and the CLI
    /// name the operation by it.
    pub id: &'static str,
    /// The operation's own version, raised when its output changes.
    pub version: u32,
    pub slot: Slot,
    pub domain: Domain,
    /// Whether the operation can run on a masked layer.
    pub layers: bool,
    /// Whether its kernel exports the derivative entry points.
    pub differentiable: bool,
    /// Its neighbourhood radius at full resolution, in pixels; 0 for a per-pixel
    /// operation. Tiles overlap by it.
    pub radius: u32,
    /// Whether an unedited photo runs it, darktable's name for the same idea (decision
    /// 28): decision 22's eight and colour calibration (decision 27) do. Any other
    /// operation is planned only while a module's binding sets one of its parameters, so
    /// it needs no identity setting, and none of its parameters can be set or saved at
    /// operation level.
    pub default_enabled: bool,
    pub provenance: Option<Provenance>,
}

/// Where an operation runs: the stage whose data it reads, and its number in the
/// pipeline order within that stage (decision 22: darktable's place in its v5.0 order,
/// `src/common/iop_order.c`, where Apertrust keeps it, otherwise a number between the new
/// neighbours). An operation cannot reorder the pipeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub stage: Stage,
    pub order: f32,
}

impl Slot {
    /// Pipeline order: by stage, then by order number.
    pub fn pipeline_order(&self, other: &Self) -> Ordering {
        self.stage
            .cmp(&other.stage)
            .then(self.order.total_cmp(&other.order))
    }
}

/// A stretch of the pipeline with one data format (`docs/engine.md`, Data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// The mosaic, one value per photosite, up to demosaic.
    Raw,
    /// RGBA in the camera's own primaries, from demosaic to the input profile.
    CameraRgb,
    /// RGBA in the working space, linear Rec2020, up to the tone mapper.
    WorkingSpace,
    /// RGBA after the tone mapper.
    Display,
    /// RGBA in the output's colour space, from output colour on.
    Output,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Stage::Raw => "raw",
            Stage::CameraRgb => "camera RGB",
            Stage::WorkingSpace => "working space",
            Stage::Display => "display",
            Stage::Output => "output",
        })
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.stage, self.order)
    }
}

/// Whether an operation works on scene light or on the image as displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Scene,
    Display,
}

/// Where a darktable port comes from, for traceability and GPL attribution.
#[derive(Debug, Clone, Copy)]
pub struct Provenance {
    /// darktable's module, by its operation name.
    pub darktable_module: &'static str,
    /// The release it was ported from, and that release's commit.
    pub release: &'static str,
    pub commit: &'static str,
    /// The darktable source files it ports.
    pub sources: &'static [&'static str],
}
