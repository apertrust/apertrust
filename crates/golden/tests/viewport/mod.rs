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

//! Decision 15's viewport target (`docs/engine.md`, Pipes), shared by the tests that
//! measure an operation's viewport against its export: the sizes and their targets, one
//! size's result, what it misses and its line on stderr.

use golden::delta_e::{Stats, Tolerance};

/// The viewport's boxes and their targets: at most 0.15 at 2400×1600 and at 1/2 of a
/// `width` × `height` image, 0.4 at 1/4 and 1/8.
pub fn boxes(width: u32, height: u32) -> [([u32; 2], f64); 4] {
    [
        ([2400, 1600], 0.15),
        ([width / 2, height / 2], 0.15),
        ([width / 4, height / 4], 0.4),
        ([width / 8, height / 8], 0.4),
    ]
}

/// One viewport size of decision 15's measure: the viewport against the export
/// downscaled by the viewport's box filter, mean ΔE2000, and the operation's per-step
/// baseline.
pub struct Scale {
    /// The viewport's box, in which the image is fitted.
    pub box_size: [u32; 2],
    pub width: usize,
    pub height: usize,
    pub stats: Stats,
    pub baseline: f64,
    /// The mean less the baseline, which the target bounds.
    pub share: f64,
    pub target: f64,
}

impl Scale {
    /// The box `box_size` with its `target`, the image fitted at `width` × `height`, the
    /// viewport's `stats` and the baseline's.
    pub fn new(
        (box_size, target): ([u32; 2], f64),
        (width, height): (usize, usize),
        stats: Stats,
        baseline: &Stats,
    ) -> Self {
        Self {
            box_size,
            width,
            height,
            share: stats.mean - baseline.mean,
            baseline: baseline.mean,
            stats,
            target,
        }
    }
}

/// What a scale misses of decision 15's target, if anything: its share over the target,
/// or at 2400×1600 a mean over `Tolerance::DARKTABLE`'s.
pub fn missed(scale: &Scale) -> Option<String> {
    let over_mean = scale.box_size == [2400, 1600] && scale.stats.mean > Tolerance::DARKTABLE.mean;
    (scale.share > scale.target || over_mean).then(|| {
        format!(
            "at {}×{}: share {:.4} against {}, mean {:.4}",
            scale.width, scale.height, scale.share, scale.target, scale.stats.mean
        )
    })
}

/// The scale's line on stderr, its baseline and share named as `baseline` and `share`.
pub fn report(what: &str, [baseline, share]: [&str; 2], scale: &Scale) {
    eprintln!(
        "{what} at {}×{}: mean {:.4}, p95 {:.4}, max {:.3}; {baseline} {:.4}; {share} {:.4}",
        scale.width,
        scale.height,
        scale.stats.mean,
        scale.stats.p95,
        scale.stats.max,
        scale.baseline,
        scale.share
    );
}
