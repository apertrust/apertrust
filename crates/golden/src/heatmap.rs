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

//! Difference heatmaps: black where images match, brightening towards the tolerance,
//! red where a pixel exceeds it.

use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use crate::delta_e::Tolerance;

pub fn write(
    path: &Path,
    width: usize,
    height: usize,
    differences: &[f32],
    tolerance: Tolerance,
) -> Result<()> {
    ensure!(differences.len() == width * height, "heatmap size mismatch");
    let data: Vec<u8> = differences
        .iter()
        .flat_map(|&d| {
            let ratio = f64::from(d) / tolerance.max;
            if ratio > 1.0 {
                [255, 40, 40]
            } else {
                [(ratio * 200.0) as u8; 3]
            }
        })
        .collect();

    let file =
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&data)?;
    Ok(())
}
