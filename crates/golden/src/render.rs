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

//! Rendering a case's reference pair, cached under `target/golden`.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::cases::Case;
use crate::darktable::{DarktableCli, REFERENCE_VERSION};
use crate::xmp::{copy_blend_params, disable_operation};

/// Bump when the darktable-cli options in `DarktableCli::render` change.
const RENDER_OPTIONS_VERSION: u32 = 1;

/// A case's module input and expected output, both linear Rec2020 PFM.
#[derive(Debug, Clone)]
pub struct References {
    pub input: PathBuf,
    pub expected: PathBuf,
}

pub fn cache_root() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2);
            workspace.expect("crate sits in crates/").join("target")
        })
        .join("golden")
}

/// The raw.pixls.us corpus, `<Make>/<Model>/<file>`: `test-images/raw.pixls.us` in the
/// workspace, where `tools/fetch-raw-corpus.sh` puts it, or `APERTRUST_RAW_CORPUS`,
/// as for the raw crate's corpus test.
pub fn corpus_dir() -> Result<PathBuf> {
    let dir = std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2);
            let workspace = workspace.expect("crate sits in crates/");
            workspace.join("test-images").join("raw.pixls.us")
        },
        PathBuf::from,
    );
    ensure!(
        dir.is_dir(),
        "no raw corpus at {}: run tools/fetch-raw-corpus.sh",
        dir.display()
    );
    Ok(dir)
}

/// The darktable integration-test images, from `APERTRUST_TEST_IMAGES`.
pub fn images_dir() -> Result<PathBuf> {
    let dir: PathBuf = std::env::var_os("APERTRUST_TEST_IMAGES")
        .context(
            "set APERTRUST_TEST_IMAGES to the darktable integration-test images \
             (darktable/src/tests/integration/images)",
        )?
        .into();
    ensure!(
        dir.is_dir(),
        "APERTRUST_TEST_IMAGES is not a directory: {}",
        dir.display()
    );
    Ok(dir)
}

/// Renders the case's input and expected output, or returns them from the cache when
/// the darktable version, sidecars and raw are unchanged.
pub fn render_case(darktable: &DarktableCli, case: &Case, images: &Path) -> Result<References> {
    let image = images.join(case.image);
    let raw = std::fs::read(&image).with_context(|| format!("reading {}", image.display()))?;

    let mut xmp_on = case.xmp.to_owned();
    if let Some(from) = case.unmask_like {
        xmp_on = copy_blend_params(&xmp_on, from, case.operation)?;
    }
    for operation in case.disable {
        xmp_on = disable_operation(&xmp_on, operation)?;
    }
    let xmp_off = disable_operation(&xmp_on, case.operation)?;

    let mut hasher = DefaultHasher::new();
    (
        REFERENCE_VERSION,
        RENDER_OPTIONS_VERSION,
        &xmp_on,
        &xmp_off,
        &raw,
    )
        .hash(&mut hasher);
    let dir = cache_root()
        .join(case.name)
        .join(format!("{:016x}", hasher.finish()));
    let references = References {
        input: dir.join("input.pfm"),
        expected: dir.join("expected.pfm"),
    };
    if references.input.is_file() && references.expected.is_file() {
        tracing::trace!(target: "golden", "{}: cached, {}", case.name, dir.display());
        return Ok(references);
    }

    darktable.ensure_reference_version()?;
    let work = dir.join("work");
    if work.exists() {
        std::fs::remove_dir_all(&work)?;
    }
    std::fs::create_dir_all(&work)?;
    for (xmp, name, target) in [
        (&xmp_off, "off", &references.input),
        (&xmp_on, "on", &references.expected),
    ] {
        let sidecar = work.join(format!("{name}.xmp"));
        std::fs::write(&sidecar, xmp)?;
        let output = work.join(format!("{name}.pfm"));
        darktable
            .render(
                &image,
                &sidecar,
                &output,
                &work.join(format!("config-{name}")),
                "LIN_REC2020",
                &[],
                None,
            )
            .with_context(|| format!("darktable-cli failed to render {}", sidecar.display()))?;
        std::fs::rename(&output, target)?;
    }
    std::fs::remove_dir_all(&work)?;
    Ok(references)
}
