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

//! The raw.pixls.us corpus for the rows that run locally (`docs/modules/README.md`, Test
//! images), with `APERTRUST_RAW_CORPUS` naming it: each raw the pipeline takes, decoded
//! and uploaded, and benches over them. A test file includes it after `common` with
//! `mod corpus;`.

use std::path::{Path, PathBuf};

use apertrust_engine::compute::GpuImage;
use apertrust_engine::pipeline::Pipe;
use apertrust_plugin::ImageContext;

use crate::common::{Bench, gpu};

/// The raw.pixls.us corpus, if `APERTRUST_RAW_CORPUS` names it.
fn corpus() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("APERTRUST_RAW_CORPUS")?);
    dir.is_dir().then_some(dir)
}

/// Every raw the corpus's manifest names, `<Make>/<Model>/<file>`.
fn corpus_names() -> Vec<&'static str> {
    include_str!("../../../raw/tests/corpus/files.sha256")
        .lines()
        .filter_map(|line| line.split_once("  ").map(|(_, name)| name.trim()))
        .collect()
}

/// A raw decoded and uploaded, with its context; `None` for one that is not a colour
/// mosaic of integers, which the pipeline does not take yet; an error for one that does
/// not decode or upload.
pub fn decode(
    path: &Path,
) -> Result<Option<(apertrust_raw::RawImage, ImageContext, GpuImage)>, String> {
    let raw = apertrust_raw::decode_file(path).map_err(|error| format!("decoding: {error}"))?;
    let Some(input) = apertrust_engine::raw::upload(gpu(), &raw) else {
        return Ok(None);
    };
    let input = input.map_err(|error| format!("uploading: {error}"))?;
    let image = apertrust_engine::raw::image_context(&raw);
    Ok(Some((raw, image, input)))
}

/// A raw the pipeline takes, decoded and uploaded, or a panic saying why not.
pub fn decoded(path: &Path) -> (apertrust_raw::RawImage, ImageContext, GpuImage) {
    match decode(path) {
        Ok(Some(decoded)) => decoded,
        Ok(None) => panic!("{}: not a colour mosaic", path.display()),
        Err(error) => panic!("{}: {error}", path.display()),
    }
}

/// Each corpus raw the pipeline takes, decoded and uploaded, with what is printed of it.
/// A raw that fails to decode or upload is reported apart from one the pipeline does not
/// take.
pub fn each_corpus_raw(mut check: impl FnMut(&str, ImageContext, GpuImage)) {
    let dir = corpus().expect("APERTRUST_RAW_CORPUS names the raw.pixls.us corpus");
    for name in corpus_names() {
        let (raw, image, input) = match decode(&dir.join(name)) {
            Ok(Some(decoded)) => decoded,
            Ok(None) => {
                println!("{name}: skipped, not a colour mosaic the pipeline takes");
                continue;
            }
            Err(error) => {
                println!("{name}: skipped, failed {error}");
                continue;
            }
        };
        let bias = image.camera.as_ref().map_or(0.0, |c| c.exposure_bias);
        let label = format!(
            "{} {} (bias {bias:+.2} EV)",
            raw.camera.make, raw.camera.model
        );
        check(&label, image, input);
    }
}

pub fn export_bench(image: ImageContext, input: GpuImage) -> Bench {
    Bench::new(image, input, false, Pipe::Export)
}

/// The Canon EOS 90D, with an exposure bias of −0.67 EV, the raw rows name first.
pub const NINETY_D: &str = "Canon/EOS 90D/Canon-90D-RAW-ISO-100.CR3";

pub fn corpus_bench(name: &str, pipe: Pipe) -> Bench {
    let dir = corpus().expect("APERTRUST_RAW_CORPUS names the raw.pixls.us corpus");
    let (_, image, input) = decoded(&dir.join(name));
    Bench::new(image, input, false, pipe)
}

/// The viewport at a size where a 61 MP raw still renders quickly.
pub const CORPUS_VIEWPORT: Pipe = Pipe::Viewport {
    width: 1600,
    height: 1600,
};
