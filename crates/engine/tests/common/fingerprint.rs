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

//! What a plan dispatches and binds, as a fingerprint no platform changes, and its keys
//! restated, for the tests that hold plans to what they were.

use std::fmt::Write as _;
use std::hash::{DefaultHasher, Hash, Hasher};

use apertrust_engine::pipeline::{Pipe, Plan};
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{Format, ImageDesc, Pass};

/// FNV-1a, 64 bits: a hash that no Rust release changes.
pub fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// What a plan dispatches and binds, but for the bytes `prepare` gave its passes, which
/// may differ between platforms' maths libraries in their last bits, and the keys,
/// which hash them.
pub fn structure(plan: &Plan) -> String {
    let desc = |d: &ImageDesc| {
        format!(
            "{}x{} {:?} {:?}",
            d.width, d.height, d.format, d.sensor_origin
        )
    };
    // A tile's pipe as it printed before an export at a size gave tiles a resample
    // (decision 56), so a tile at full size keeps its fingerprint.
    let pipe = match plan.pipe {
        Pipe::Tile {
            region,
            resample: None,
        } => format!("Tile {{ region: {region:?} }}"),
        pipe => format!("{pipe:?}"),
    };
    let mut text = format!("{pipe} {}\n", desc(&plan.input));
    for op in &plan.operations {
        writeln!(
            text,
            "{} {:?}->{:?} {} {:?} {} {:?} {:?}",
            op.id,
            op.input,
            op.output,
            desc(&op.output_desc),
            op.scratch,
            op.scale.to_bits(),
            op.origin,
            op.whole_image.map(|whole| whole.passes),
        )
        .unwrap();
        for (pass, dispatches) in op.passes.iter().zip(&op.dispatches) {
            let tables: Vec<usize> = pass.tables.iter().map(Vec::len).collect();
            writeln!(
                text,
                " {} {:?} {:?} {:?} {:?} {} {tables:?} {dispatches:?}",
                pass.entry_point,
                pass.kind,
                pass.size,
                pass.strips,
                pass.bindings,
                pass.params.len(),
            )
            .unwrap();
        }
    }
    let work: Vec<String> = plan.work.iter().map(desc).collect();
    let used: Vec<String> = plan.work_used.iter().map(desc).collect();
    writeln!(
        text,
        "{work:?} {used:?} {:?} {:?} {:?} {:?} {:?}",
        plan.scratch,
        plan.scratch_used,
        plan.cached,
        plan.edit_cache.as_ref().map(desc),
        plan.tiled
    )
    .unwrap();
    text
}

/// `hash_desc` in the pipeline, as it was before geometry: an image's size, format,
/// place on the sensor and colour bookkeeping, never its region of its stage.
pub fn hash_desc(desc: &ImageDesc, hasher: &mut DefaultHasher) {
    let format = match desc.format {
        Format::Mosaic16 => 0u8,
        Format::Mosaic => 1,
        Format::Rgba => 2,
    };
    (
        desc.width,
        desc.height,
        format,
        desc.sensor_origin,
        desc.late_correction,
    )
        .hash(hasher);
    for value in desc.maximum.iter().chain(&desc.white_balance) {
        value.to_bits().hash(hasher);
    }
}

/// Each operation's key and whole-image key as the pipeline made them before geometry:
/// the previous key, its ID, version, scale and origin, its output image and its passes'
/// prepared bytes; and for the whole-image passes, their input's key and their own.
pub fn keys_before_geometry(plan: &Plan) -> Vec<(u64, Option<u64>)> {
    let version = |id: &str| {
        OPERATIONS
            .iter()
            .find(|op| op.manifest().id == id)
            .map_or(1, |op| op.manifest().version)
    };
    let mut hasher = DefaultHasher::new();
    hash_desc(&plan.input, &mut hasher);
    let mut previous = hasher.finish();
    let mut keys = Vec::new();
    for op in &plan.operations {
        let mut hasher = DefaultHasher::new();
        (
            previous,
            op.id,
            version(op.id),
            op.scale.to_bits(),
            op.origin,
        )
            .hash(&mut hasher);
        hash_desc(&op.output_desc, &mut hasher);
        for pass in &op.passes {
            (pass.entry_point, pass.size, &pass.params, &pass.tables).hash(&mut hasher);
        }
        let key = hasher.finish();
        let whole_image = op.whole_image.map(|whole| {
            let scratch: Vec<u64> = op.scratch.iter().map(|&i| plan.scratch[i]).collect();
            let mut hasher = DefaultHasher::new();
            (
                previous,
                op.id,
                version(op.id),
                op.scale.to_bits(),
                whole.passes,
                scratch,
            )
                .hash(&mut hasher);
            hash_desc(&op.output_desc, &mut hasher);
            for pass in &op.passes[..whole.passes] {
                let Pass {
                    entry_point,
                    kind,
                    params,
                    tables,
                    bindings,
                    size,
                    strips: _,
                } = pass;
                (entry_point, kind, params, tables, bindings, size).hash(&mut hasher);
            }
            hasher.finish()
        });
        keys.push((key, whole_image));
        previous = key;
    }
    keys
}
