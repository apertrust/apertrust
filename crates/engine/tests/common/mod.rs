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

//! What several of the engine's tests share: the synthetic raws and their camera, the
//! default pipeline with settings, the plans' fingerprints, and the GPU opened to bind
//! less. Each test binary uses part of it.

#![allow(dead_code)]

pub mod fingerprint;
pub mod raw;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::Instance;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{Camera, ImageContext, Kind, ParamValues, Pattern, Value};

/// The Fujifilm X-Trans colour filter array, 6 × 6, red 0, green 1 and blue 2: the
/// X-T5's and the synthetic raws'.
#[rustfmt::skip]
pub const X_TRANS: [u32; 36] = [
    1, 1, 0, 1, 1, 2,
    1, 1, 2, 1, 1, 0,
    2, 0, 1, 0, 2, 1,
    1, 1, 2, 1, 1, 0,
    1, 1, 0, 1, 1, 2,
    0, 2, 1, 2, 0, 1,
];

/// A raw's image context: a readout of `sensor` photosites across and down with `cfa`,
/// its active area `crop` and its declared image `declared`, black at 0 and white at 4000,
/// as-shot multipliers of 2, 1 and 1.5 and an sRGB-like matrix. A test of another camera
/// sets those of its own on it.
pub fn image(
    sensor: (u32, u32),
    cfa: &Pattern<u32>,
    crop: [u32; 4],
    declared: [u32; 4],
) -> ImageContext {
    let (width, height) = sensor;
    ImageContext {
        camera: Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width,
            height,
            cfa: Some(cfa.clone()),
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 4000.0,
            white_balance: Some(vec![2.0, 1.0, 1.5]),
            xyz_to_camera: Some([
                [3.2406, -1.5372, -0.4986],
                [-0.9689, 1.8758, 0.0415],
                [0.0557, -0.2040, 1.0570],
            ]),
            embedded_matrix: false,
            crop,
            declared,
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            highlight_preservation: 0.0,
            orientation: 0,
        }),
        ..ImageContext::full(declared[2], declared[3])
    }
}

/// The default pipeline on `image` with these parameters set, as an edit sets them, and
/// every operation an unedited photo does not run that a setting names, with its settings,
/// as a tool's binding adds it (decision 28); no tools, so the tests stay about the
/// pipeline. A choice or a whole number is given by its index or value.
pub fn instances(image: &ImageContext, settings: &[(&str, &str, f32)]) -> Vec<Instance<'static>> {
    let optional = |id: &str| {
        OPERATIONS
            .iter()
            .copied()
            .find(|op| op.manifest().id == id && !op.manifest().default_enabled)
    };
    let mut edit = Edit::default();
    // An edit refuses an optional operation's parameters, which only tools set.
    let mut joined: Vec<Instance<'static>> = Vec::new();
    for &(id, param, value) in settings {
        // A choice or a whole number by its index or value.
        let operation = (OPERATIONS.iter()).find(|op| op.manifest().id == id);
        let kind = operation
            .and_then(|op| op.schema().params.iter().find(|p| p.name == param))
            .map(|p| p.kind);
        let value = match kind {
            Some(Kind::Choice { .. }) => Value::Choice(value as u32),
            Some(Kind::Int { .. }) => Value::Int(value as i64),
            _ => Value::Float(value),
        };
        let Some(operation) = optional(id) else {
            edit.set_param(OPERATIONS, &[], id, param, value).unwrap();
            continue;
        };
        let at = match joined.iter().position(|i| i.operation.manifest().id == id) {
            Some(at) => at,
            None => {
                joined.push(Instance {
                    operation,
                    params: ParamValues::new(),
                });
                joined.len() - 1
            }
        };
        joined[at].params.set(param, value);
    }
    let mut instances = edit::instances(&edit, OPERATIONS, &[], image)
        .all()
        .unwrap();
    assert!(
        joined.iter().all(|j| instances
            .iter()
            .all(|i| i.operation.manifest().id != j.operation.manifest().id)),
        "an optional operation in the default pipeline"
    );
    instances.extend(joined);
    instances
}

/// The adapter, hardware or software, opened to bind at most `binding` bytes of a buffer
/// at once, at offsets aligned to 256 bytes, WebGPU's largest alignment, as lavapipe binds
/// less than a large photo's images (decision 14).
pub fn small_gpu(binding: u64) -> GpuContext {
    let options = GpuOptions {
        max_storage_binding: Some(binding),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    };
    let gpu =
        pollster::block_on(GpuContext::new(&options)).expect("a GPU adapter, hardware or software");
    assert_eq!(gpu.device.limits().max_storage_buffer_binding_size, binding);
    gpu
}
