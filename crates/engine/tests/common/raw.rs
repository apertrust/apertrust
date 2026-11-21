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

//! The synthetic raws the geometry and detail tests develop: a colour gradient with a
//! highlight that clips, Bayer or X-Trans.

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{Exported, Tiling, export};
use apertrust_engine::gpu::GpuContext;
use apertrust_engine::pipeline::Instance;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{Format, ImageContext, ImageDesc, Pattern};

use super::X_TRANS;

/// A raw `sensor` photosites across and down, its active area `crop` and its declared
/// image `declared`: a colour gradient with a highlight that clips.
pub struct Raw {
    pub cfa: Pattern<u32>,
    pub sensor: (u32, u32),
    pub crop: [u32; 4],
    pub declared: [u32; 4],
}

impl Raw {
    pub fn bayer(sensor: (u32, u32), crop: [u32; 4], declared: [u32; 4]) -> Self {
        Self {
            cfa: Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            },
            sensor,
            crop,
            declared,
        }
    }

    #[rustfmt::skip]
    pub fn x_trans(sensor: (u32, u32), crop: [u32; 4], declared: [u32; 4]) -> Self {
        Self {
            cfa: Pattern {
                width: 6,
                height: 6,
                values: X_TRANS.to_vec(),
            },
            sensor,
            crop,
            declared,
        }
    }

    pub fn values(&self) -> Vec<u16> {
        let (width, height) = self.sensor;
        let cfa = &self.cfa;
        (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                let c = cfa.values[((y % cfa.height) * cfa.width + x % cfa.width) as usize];
                let (fx, fy) = (x as f32 / width as f32, y as f32 / height as f32);
                let r2 = (fx - 0.6).powi(2) + (fy - 0.4).powi(2);
                let level = 0.15 + 0.5 * fx + 0.1 * c as f32 + 0.2 * fy + 1.2 * (-r2 / 0.01).exp();
                (level.min(1.0) * 4000.0).round() as u16
            })
            .collect()
    }

    pub fn image(&self) -> ImageContext {
        super::image(self.sensor, &self.cfa, self.crop, self.declared)
    }

    pub fn sensor(&self) -> ImageDesc {
        ImageDesc::new(self.sensor.0, self.sensor.1, Format::Mosaic16)
    }

    pub fn upload(&self, gpu: &GpuContext) -> GpuImage {
        GpuImage::upload_mosaic(gpu, self.sensor.0, self.sensor.1, &self.values()).unwrap()
    }

    /// The default pipeline an unedited photo runs.
    pub fn instances(&self) -> Vec<Instance<'static>> {
        edit::instances(&Edit::default(), OPERATIONS, &[], &self.image())
            .all()
            .unwrap()
    }

    pub fn export(&self, gpu: &GpuContext, instances: &[Instance], tiling: Tiling) -> Exported {
        pollster::block_on(export(
            gpu,
            instances,
            &self.image(),
            self.upload(gpu),
            tiling,
        ))
        .unwrap()
    }
}
