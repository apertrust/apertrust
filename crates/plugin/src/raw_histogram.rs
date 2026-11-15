// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2009-2026  darktable developers (ported from src/iop/exposure.c)
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

//! The raw's histogram (decision 87): one count per 16-bit value of the decoded mosaic,
//! which the engine counts on the GPU, once per photo, for an operation that reads it,
//! and hands to `prepare` in the image context. Exposure's automatic mode, darktable's
//! deflicker, meters it; the search and the conversion to EV are darktable's
//! (`src/iop/exposure.c`, `_compute_correction` and `_raw_to_ev`), here so that the
//! operation and its tool's readout compute the same number.

use std::sync::Arc;

/// One bin per 16-bit value: darktable's `DEFLICKER_BINS_COUNT`.
pub const RAW_HISTOGRAM_BINS: usize = 1 << 16;

/// How many photosites of the raw hold each 16-bit value, over the region the engine
/// meters: the declared image, the one Apertrust shows (`docs/ports/exposure.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct RawHistogram {
    counts: Arc<[u32]>,
    pixels: u64,
}

impl RawHistogram {
    /// The histogram of these counts, one per 16-bit value; `None` for any other number.
    pub fn new(counts: Vec<u32>) -> Option<Self> {
        (counts.len() == RAW_HISTOGRAM_BINS).then(|| Self {
            pixels: counts.iter().map(|&count| u64::from(count)).sum(),
            counts: counts.into(),
        })
    }

    /// The counts, one per 16-bit value, darkest first.
    pub fn counts(&self) -> &[u32] {
        &self.counts
    }

    /// The photosites counted.
    pub fn pixels(&self) -> u64 {
        self.pixels
    }

    /// The value at `percentile`, in % of the photosites, as darktable's
    /// `_compute_correction` finds it: the first value, from 0 up, at which the
    /// photosites counted reach `pixels · percentile / 100`, held to 0 to `pixels`, in
    /// `f64`. At 0 that is 0; at 100 the highest value a photosite holds.
    pub fn value_at(&self, percentile: f32) -> u32 {
        let pixels = self.pixels as f64;
        let threshold = (pixels * f64::from(percentile) / 100.0).clamp(0.0, pixels);
        let mut counted = 0u64;
        for (value, &count) in self.counts.iter().enumerate() {
            counted += u64::from(count);
            if counted as f64 >= threshold {
                return value as u32;
            }
        }
        // Only a NaN threshold gets here, which the schema's range keeps out; darktable
        // leaves the value at 0.
        0
    }

    /// darktable's `_raw_to_ev`: a raw value in EV below the white point,
    /// `log2(max(value − black, 1)) − log2(white − black)`, in `f64`; at most 0 at the
    /// white point, about −16 at the darkest. A white at or below the black counts as one
    /// level above it, as rawprepare's divisor does.
    pub fn ev(value: u32, black: u32, white: u32) -> f64 {
        let range = white.saturating_sub(black).max(1);
        let above = (i64::from(value) - i64::from(black)).max(1);
        -f64::from(range).log2() + (above as f64).log2()
    }

    /// The exposure that puts `percentile` at `target` EV below the white point, as
    /// darktable's deflicker computes it: `target − ev`, in `f64`, rounded once to `f32`.
    pub fn exposure(&self, percentile: f32, target: f32, black: u32, white: u32) -> f32 {
        (f64::from(target) - Self::ev(self.value_at(percentile), black, white)) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A histogram with `count` photosites at each of `values`.
    fn histogram(values: &[(usize, u32)]) -> RawHistogram {
        let mut counts = vec![0; RAW_HISTOGRAM_BINS];
        for &(value, count) in values {
            counts[value] += count;
        }
        RawHistogram::new(counts).unwrap()
    }

    #[test]
    fn only_one_count_per_16_bit_value_is_a_histogram() {
        assert!(RawHistogram::new(vec![0; 256]).is_none());
        assert_eq!(histogram(&[(3, 2), (9, 5)]).pixels(), 7);
    }

    /// darktable's `_compute_correction` in `f64`, the reference.
    fn darktable_value(counts: &[u32], percentile: f32) -> u32 {
        let pixels: u64 = counts.iter().map(|&c| u64::from(c)).sum();
        let thr = (pixels as f64 * percentile as f64 / 100.0).clamp(0.0, pixels as f64);
        let mut n = 0u64;
        for (i, &c) in counts.iter().enumerate() {
            n += u64::from(c);
            if n as f64 >= thr {
                return i as u32;
            }
        }
        0
    }

    #[test]
    fn the_percentile_is_darktable_s_first_value_reaching_it() {
        // 10 photosites at 100, 10 at 200, 20 at 300.
        let h = histogram(&[(100, 10), (200, 10), (300, 20)]);
        // At 0, the first value of all, 0, whatever it holds.
        assert_eq!(h.value_at(0.0), 0);
        // 25 % is 10 photosites, reached exactly at 100.
        assert_eq!(h.value_at(25.0), 100);
        // Just above it, the next value.
        assert_eq!(h.value_at(25.01), 200);
        assert_eq!(h.value_at(50.0), 200);
        // 100 %: the highest value held.
        assert_eq!(h.value_at(100.0), 300);
        for p in [0.0, 1.0, 10.0, 25.0, 33.3, 50.0, 62.5, 90.0, 99.99, 100.0] {
            assert_eq!(h.value_at(p), darktable_value(h.counts(), p), "{p}");
        }
    }

    #[test]
    fn values_at_or_below_the_black_meter_one_level_above_it() {
        // 14 bits above a black of 512.
        let (black, white) = (512, 512 + 16384);
        assert_eq!(RawHistogram::ev(white, black, white), 0.0);
        assert_eq!(RawHistogram::ev(512 + 8192, black, white), -1.0);
        assert_eq!(RawHistogram::ev(513, black, white), -14.0);
        assert_eq!(RawHistogram::ev(512, black, white), -14.0);
        assert_eq!(RawHistogram::ev(0, black, white), -14.0);
        // A white at or below the black is one level above it.
        assert_eq!(RawHistogram::ev(600, 512, 500), 88f64.log2());
    }

    #[test]
    fn the_exposure_puts_the_percentile_at_the_target() {
        // Every photosite at a quarter of the range above the black: −2 EV.
        let h = histogram(&[(1024 + 4096, 1000)]);
        let exposure = h.exposure(50.0, -4.0, 1024, 1024 + 16384);
        assert_eq!(exposure, -2.0);
        // darktable's arithmetic: the target in f64, less the EV, then to f32.
        let h = histogram(&[(1500, 7), (2700, 3)]);
        let ev = -(15000f64).log2() + (2700f64 - 600.0).log2();
        assert_eq!(
            h.exposure(80.0, -3.7, 600, 15600),
            (f64::from(-3.7f32) - ev) as f32
        );
    }
}
