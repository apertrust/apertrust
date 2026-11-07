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

//! The built-in operations (architecture decision 20), listed explicitly. The binaries
//! hand this list to the engine, which never depends on an operation crate itself.

pub use apertrust_op_agx as agx;
pub use apertrust_op_bilat as bilat;
pub use apertrust_op_bloom as bloom;
pub use apertrust_op_channelmixerrgb as channelmixerrgb;
pub use apertrust_op_colorbalancergb as colorbalancergb;
pub use apertrust_op_colorequal as colorequal;
pub use apertrust_op_colorin as colorin;
pub use apertrust_op_colorout as colorout;
pub use apertrust_op_demosaic as demosaic;
pub use apertrust_op_exposure as exposure;
pub use apertrust_op_filmicrgb as filmicrgb;
pub use apertrust_op_flip as flip;
pub use apertrust_op_grain as grain;
pub use apertrust_op_hazeremoval as hazeremoval;
pub use apertrust_op_highlights as highlights;
pub use apertrust_op_primaries as primaries;
pub use apertrust_op_rawprepare as rawprepare;
pub use apertrust_op_rgbcurve as rgbcurve;
pub use apertrust_op_rgblevels as rgblevels;
pub use apertrust_op_sigmoid as sigmoid;
pub use apertrust_op_soften as soften;
pub use apertrust_op_temperature as temperature;
pub use apertrust_op_toneequal as toneequal;
pub use apertrust_op_vignette as vignette;
use apertrust_plugin::{Kernel, Operation};

/// The operations the engine can run, in pipeline order, darktable's (decision 22, `docs/engine.md`, Order). An unedited photo runs those whose manifest has
/// `default_enabled`, decision 22's eight, colour calibration (decision 27) and
/// orientation, which darktable enables on every image at auto; the
/// others run only while a module sets one of their parameters (decision 28). Nothing
/// takes this list as the default pipeline: `apertrust_engine::edit::instances` makes one
/// from an edit.
pub const OPERATIONS: &[&dyn Operation] = &[
    &rawprepare::Rawprepare,
    &temperature::Temperature,
    &highlights::Highlights,
    &demosaic::Demosaic,
    &hazeremoval::HazeRemoval,
    &flip::Flip,
    &exposure::Exposure,
    &toneequal::ToneEqual,
    &colorin::Colorin,
    &channelmixerrgb::ChannelMixerRgb,
    &primaries::Primaries,
    &colorequal::ColorEqual,
    &colorbalancergb::ColorBalanceRgb,
    &rgbcurve::RgbCurve,
    &rgblevels::RgbLevels,
    &sigmoid::Sigmoid,
    &agx::Agx,
    &filmicrgb::FilmicRgb,
    &bilat::Bilat,
    &bloom::Bloom,
    &grain::Grain,
    &soften::Soften,
    &vignette::Vignette,
    &colorout::Colorout,
];

/// Every built-in operation's kernel, in the order of [`OPERATIONS`].
pub const KERNELS: &[Kernel] = &[
    rawprepare::KERNEL,
    temperature::KERNEL,
    highlights::KERNEL,
    demosaic::KERNEL,
    hazeremoval::KERNEL,
    flip::KERNEL,
    exposure::KERNEL,
    toneequal::KERNEL,
    colorin::KERNEL,
    channelmixerrgb::KERNEL,
    primaries::KERNEL,
    colorequal::KERNEL,
    colorbalancergb::KERNEL,
    rgbcurve::KERNEL,
    rgblevels::KERNEL,
    sigmoid::KERNEL,
    agx::KERNEL,
    filmicrgb::KERNEL,
    bilat::KERNEL,
    bloom::KERNEL,
    grain::KERNEL,
    soften::KERNEL,
    vignette::KERNEL,
    colorout::KERNEL,
];

/// The operation with this ID, if the engine can run it.
pub fn operation(id: &str) -> Option<&'static dyn Operation> {
    OPERATIONS
        .iter()
        .copied()
        .find(|operation| operation.manifest().id == id)
}

/// The kernel of the operation with this ID, if it is built in.
pub fn kernel(operation: &str) -> Option<&'static Kernel> {
    KERNELS.iter().find(|kernel| kernel.name == operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turning another operation on changes every photo's default render and the
    /// whole-pipeline test's, which a decision has to change first: decision 22's eight,
    /// colour calibration since decision 27, and orientation, darktable's on every image,
    /// which a photo the right way up leaves out of its plan.
    #[test]
    fn an_unedited_photo_runs_exactly_the_base_ten() {
        let enabled: Vec<&str> = OPERATIONS
            .iter()
            .map(|operation| operation.manifest())
            .filter(|manifest| manifest.default_enabled)
            .map(|manifest| manifest.id)
            .collect();
        assert_eq!(
            enabled,
            [
                "rawprepare",
                "temperature",
                "highlights",
                "demosaic",
                "flip",
                "exposure",
                "colorin",
                "channelmixerrgb",
                "agx",
                "colorout"
            ]
        );
    }
}
