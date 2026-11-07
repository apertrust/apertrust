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

//! Black measured from the sensor's masked borders, as darktable's decoder, rawspeed,
//! measures it (`RawImageDataU16::calculateBlackAreas`) for a raw that stores no
//! levels. It explains darktable's levels where they differ from Apertrust's; the
//! defaults use the levels the raw stores (decision 23).
//!
//! rawspeed's measurement has a known bug (rawspeed issue 389): each row of a masked
//! area contributes one value, its photosite in a single column, counted once per
//! column of the crop or of the area, instead of every photosite. darktable
//! uses the result anyway, so this does the same, bug and all. Its histogram counts
//! are 16-bit and wrap, as rawspeed's do.

use crate::{BlackLevel, Pixels, RawImage, Rect};

impl RawImage {
    /// The black level per position of the 2×2 pattern of row and column parities on
    /// the sensor, measured from the masked areas as rawspeed measures it; `None` when
    /// the raw has no masked areas or is not a 16-bit mosaic.
    pub fn masked_black(&self) -> Option<BlackLevel> {
        let Pixels::U16(pixels) = &self.pixels else {
            return None;
        };
        if self.components != 1 || self.masked_areas.is_empty() {
            return None;
        }
        let crop = self.active_area;
        let width = self.width as usize;
        let at = |x: u32, y: u32| pixels[y as usize * width + x as usize];
        // Four histograms, one per parity position, of 16-bit counts.
        let mut histograms = vec![[0u16; 4]; 1 << 16];
        let mut add = |value: u16, y: u32, x_start: u32, columns: u32| {
            let (even, odd) = parity_counts(x_start, columns);
            let row = ((y & 1) << 1) as usize;
            let bins = &mut histograms[usize::from(value)];
            bins[row] = bins[row].wrapping_add(even as u16);
            bins[row + 1] = bins[row + 1].wrapping_add(odd as u16);
        };
        let mut total: u64 = 0;
        for area in &self.masked_areas {
            match Area::of(area, &crop) {
                Some(Area::Horizontal { top, rows }) => {
                    // The photosite in the crop's first column, counted across the
                    // crop's width.
                    for y in top..top + rows {
                        add(at(crop.x, y), y, crop.x, crop.width);
                    }
                    total += u64::from(rows) * u64::from(crop.width);
                }
                Some(Area::Vertical { left, columns }) => {
                    // The photosite in the area's first column, counted across the
                    // area's width, for every row of the crop.
                    for y in crop.y..crop.y + crop.height {
                        add(at(left, y), y, left, columns);
                    }
                    total += u64::from(columns) * u64::from(crop.height);
                }
                None => {}
            }
        }
        if total == 0 {
            return None;
        }
        // The median of each histogram: half its share of the photosites.
        let half = total / 8;
        let mut values: Vec<f32> = (0..4)
            .map(|position| {
                let mut seen = u64::from(histograms[0][position]);
                let mut value = 0;
                while seen <= half && value < u16::MAX as usize {
                    value += 1;
                    seen += u64::from(histograms[value][position]);
                }
                value as f32
            })
            .collect();
        if self.cfa.is_none() {
            let sum: f32 = values.iter().sum();
            values = vec![((sum as u32 + 2) >> 2) as f32; 4];
        }
        Some(BlackLevel {
            width: 2,
            height: 2,
            values,
        })
    }
}

/// A masked area as rawspeed describes one: rows across the crop's width, or columns
/// down the crop's height. rawler builds its areas the same way, from the same camera
/// data.
enum Area {
    Horizontal { top: u32, rows: u32 },
    Vertical { left: u32, columns: u32 },
}

impl Area {
    fn of(area: &Rect, crop: &Rect) -> Option<Self> {
        // rawspeed uses an even number of rows or columns, the same for each parity.
        if area.x == crop.x && area.width == crop.width {
            Some(Area::Horizontal {
                top: area.y,
                rows: area.height & !1,
            })
        } else if area.y == crop.y && area.height == crop.height {
            Some(Area::Vertical {
                left: area.x,
                columns: area.width & !1,
            })
        } else {
            None
        }
    }
}

/// How many of the columns `start..start + count` are even, and how many odd.
fn parity_counts(start: u32, count: u32) -> (u32, u32) {
    let first_even = start.is_multiple_of(2);
    let half = count / 2;
    let extra = count % 2;
    if first_even {
        (half + extra, half)
    } else {
        (half, half + extra)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Camera, Cfa, CfaColor, Orientation};

    /// A 12×10 sensor: two masked rows at the top, two masked columns at the left, the
    /// crop from (2, 2), every photosite 100 plus its parity position.
    fn sensor(width: u32, height: u32, value: impl Fn(u32, u32) -> u16) -> RawImage {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| value(x, y))
            .collect();
        let crop = Rect {
            x: 2,
            y: 2,
            width: width - 2,
            height: height - 2,
        };
        RawImage {
            camera: Camera {
                make: "Test".into(),
                model: "Sensor".into(),
                alias: "Sensor".into(),
            },
            width,
            height,
            components: 1,
            pixels: Pixels::U16(pixels),
            active_area: crop,
            default_crop: None,
            masked_areas: vec![
                Rect {
                    x: crop.x,
                    y: 0,
                    width: crop.width,
                    height: 2,
                },
                Rect {
                    x: 0,
                    y: crop.y,
                    width: 2,
                    height: crop.height,
                },
            ],
            cfa: Some(Cfa {
                width: 2,
                height: 2,
                colors: vec![
                    CfaColor::Red,
                    CfaColor::Green,
                    CfaColor::Green,
                    CfaColor::Blue,
                ],
            }),
            black_level: BlackLevel {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white_levels: vec![4095],
            white_balance: None,
            color_matrices: Vec::new(),
            dng_color: None,
            linear_response_limit: None,
            orientation: Orientation::Normal,
            exposure_bias: None,
            iso: None,
            lens_correction: None,
        }
    }

    fn parity(x: u32, y: u32) -> u16 {
        (((y & 1) << 1) | (x & 1)) as u16
    }

    #[test]
    fn measures_each_parity_position() {
        let raw = sensor(12, 10, |x, y| 100 + parity(x, y));
        // Every row of an area is sampled in one column, so the column parity of a
        // sample is that column's: both areas start at an even column.
        let black = raw.masked_black().unwrap();
        assert_eq!((black.width, black.height), (2, 2));
        assert_eq!(black.values, [100.0, 100.0, 102.0, 102.0]);
    }

    #[test]
    fn samples_one_column_per_row_as_rawspeed_does() {
        // Hot photosites everywhere but the sampled columns change nothing.
        let raw = sensor(12, 10, |x, y| {
            if x == 2 || x == 0 {
                64
            } else {
                900 + parity(x, y)
            }
        });
        assert_eq!(raw.masked_black().unwrap().values, [64.0; 4]);
    }

    #[test]
    fn takes_the_median_of_the_samples() {
        // In the masked columns alone, one even and one odd row of six read high: the
        // median stays at the low value, where the mean would not.
        let mut raw = sensor(12, 14, |_, y| if y == 4 || y == 7 { 140 } else { 100 });
        raw.masked_areas.remove(0);
        assert_eq!(raw.masked_black().unwrap().values, [100.0; 4]);
    }

    #[test]
    fn weighs_each_masked_row_by_the_crop_width_as_rawspeed_does() {
        // Two masked rows read 140 and eight masked columns' rows read 100, but each
        // row's sample counts once per column of the crop, ten times, against twice
        // for a column's: the rows win.
        let raw = sensor(12, 10, |x, y| if y < 2 && x >= 2 { 140 } else { 100 });
        assert_eq!(raw.masked_black().unwrap().values, [140.0; 4]);
    }

    #[test]
    fn averages_a_raw_without_a_colour_filter() {
        let mut raw = sensor(12, 10, |x, y| 100 + parity(x, y));
        raw.cfa = None;
        // (100 + 100 + 102 + 102 + 2) >> 2
        assert_eq!(raw.masked_black().unwrap().values, [101.0; 4]);
    }

    #[test]
    fn needs_masked_areas() {
        let mut raw = sensor(12, 10, |_, _| 100);
        raw.masked_areas.clear();
        assert_eq!(raw.masked_black(), None);
    }

    #[test]
    fn counts_column_parities() {
        assert_eq!(parity_counts(0, 5), (3, 2));
        assert_eq!(parity_counts(1, 5), (2, 3));
        assert_eq!(parity_counts(2, 4), (2, 2));
        assert_eq!(parity_counts(3, 0), (0, 0));
    }
}
