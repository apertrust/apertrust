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

//! Driving darktable-cli to render reference images.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};

/// The darktable release every reference is rendered with: the reference release, which is
/// darktable's latest stable release (decision 3), 5.6.2 today. The harness and the tools
/// read its number here; each operation's manifest records the release its port was written
/// from. Moving to a new release changes the references: follow AGENTS.md's steps and
/// re-check every module's tolerance.
pub const REFERENCE_VERSION: &str = "5.6.2";

/// darktable-cli's size options for an export at a size (decision 56): `--width`,
/// `--height`, `--hq` and `--upscale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: u32,
    pub height: u32,
    pub high_quality: bool,
    pub upscale: bool,
}

#[derive(Debug, Clone)]
pub struct DarktableCli {
    path: PathBuf,
}

impl DarktableCli {
    /// Finds darktable-cli: `DARKTABLE_CLI`, then `PATH`, then the platform's default
    /// install location.
    pub fn locate() -> Result<Self> {
        if let Some(path) = std::env::var_os("DARKTABLE_CLI") {
            return Ok(Self { path: path.into() });
        }
        let exe = format!("darktable-cli{}", std::env::consts::EXE_SUFFIX);
        let on_path = std::env::var_os("PATH")
            .into_iter()
            .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
            .map(|dir| dir.join(&exe));
        let defaults = [
            "/Applications/darktable.app/Contents/MacOS/darktable-cli",
            "/opt/darktable/bin/darktable-cli",
            r"C:\Program Files\darktable\bin\darktable-cli.exe",
        ]
        .map(PathBuf::from);
        on_path
            .chain(defaults)
            .find(|path| path.is_file())
            .map(|path| Self { path })
            .context("darktable-cli not found: install darktable or set DARKTABLE_CLI")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The version darktable-cli reports, for example the constant below.
    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.path)
            .arg("--version")
            .output()
            .with_context(|| format!("running {}", self.path.display()))?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.lines()
            .next()
            .and_then(|line| line.strip_prefix("darktable "))
            .map(|v| v.trim().to_owned())
            .with_context(|| format!("unexpected darktable-cli --version output: {text:?}"))
    }

    /// Fails unless this is the darktable release the references are pinned to.
    pub fn ensure_reference_version(&self) -> Result<()> {
        let version = self.version()?;
        ensure!(
            version == REFERENCE_VERSION,
            "{} is darktable {version}; references need {REFERENCE_VERSION}",
            self.path.display()
        );
        Ok(())
    }

    /// Renders `image` with the history in `xmp` to a file in the output profile
    /// `icc_type` (darktable's name, such as `LIN_REC2020` or `SRGB`), its format from
    /// `output`'s extension: at full resolution, or at `size`.
    ///
    /// Options mirror darktable's integration tests (CPU path, fixed memory and thread
    /// settings, no auto-applied presets), except that the output is resized only to a
    /// `size` given. `conf` adds darktable settings, `key=value`.
    #[allow(
        clippy::too_many_arguments,
        reason = "darktable-cli's command line, one part each"
    )]
    pub fn render(
        &self,
        image: &Path,
        xmp: &Path,
        output: &Path,
        config_dir: &Path,
        icc_type: &str,
        conf: &[&str],
        size: Option<Size>,
    ) -> Result<()> {
        let name = |path: &Path| {
            path.file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into())
        };
        tracing::debug!(target: "golden", "darktable-cli: rendering {}, xmp: {}", name(image), name(xmp));
        tracing::trace!(target: "golden", "xmp: {}", xmp.display());
        let start = std::time::Instant::now();
        let size = size.unwrap_or(Size {
            width: 0,
            height: 0,
            high_quality: true,
            upscale: false,
        });
        let flag = |on: bool| if on { "true" } else { "false" };
        let result = Command::new(&self.path)
            .arg(image)
            .arg(xmp)
            .arg(output)
            .args(["--apply-custom-presets", "false"])
            .args(["--width", &size.width.to_string()])
            .args(["--height", &size.height.to_string()])
            .args([
                "--hq",
                flag(size.high_quality),
                "--upscale",
                flag(size.upscale),
            ])
            .args(["--icc-type", icc_type])
            .arg("--core")
            .arg("--disable-opencl")
            .arg("--configdir")
            .arg(config_dir)
            .args(["-t", "4"])
            .args(
                [
                    "host_memory_limit=8192",
                    "resourcelevel=reference",
                    "worker_threads=4",
                    "plugins/lighttable/export/force_lcms2=FALSE",
                    "plugins/lighttable/export/iccintent=0",
                ]
                .iter()
                .chain(conf)
                .flat_map(|conf| ["--conf", conf]),
            )
            .env("OMP_THREAD_LIMIT", "4")
            .output()
            .with_context(|| format!("running {}", self.path.display()))?;

        tracing::debug!(
            target: "perf",
            "darktable-cli rendered {} in {:.1} s",
            name(image),
            start.elapsed().as_secs_f64()
        );
        if !result.status.success() || !output.is_file() {
            let log = format!(
                "{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            if let Some(line) = loader_rejection(&log, &name(image)) {
                return Err(LoaderRejected(line.to_owned()).into());
            }
            bail!(
                "darktable-cli failed to render {} with {}:\n{log}",
                image.display(),
                xmp.display()
            );
        }
        Ok(())
    }
}

/// The line of darktable-cli's log that shows its loader rejected the raw `file`, a
/// file name, if one does: rawspeed's `Unsupported camera model for {file}` or
/// `{file} corrupt: …`, exactly as darktable prints them
/// (`src/imageio/imageio_rawspeed.cc`). Any other failure shows none: rawspeed's
/// errors that do not stop a decode, a crash, running out of memory or disk, a bad
/// option, or a history darktable cannot open.
///
/// When the line appears but a fallback loader, such as libraw, then loads the file and
/// the export fails for another reason, the log cannot tell this from a rejection. The
/// whole-pipeline test limits that case to the samples it expects to skip.
pub fn loader_rejection<'a>(log: &'a str, file: &str) -> Option<&'a str> {
    let unsupported = format!("[rawspeed] Unsupported camera model for {file}");
    let corrupt = format!("[rawspeed] {file} corrupt: ");
    log.lines().find_map(|line| {
        let rest = line[line.find("[rawspeed]")?..].trim_end();
        (rest == unsupported || rest.starts_with(&corrupt)).then_some(rest)
    })
}

/// darktable-cli's loader rejected the raw: rawspeed does not support the camera or
/// finds the file corrupt. Holds that line of the log.
#[derive(Debug)]
pub struct LoaderRejected(String);

impl std::fmt::Display for LoaderRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoaderRejected {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loader_rejection_is_told_from_other_failures() {
        // The X-E4's lossy RAF, as darktable-cli logs it.
        let corrupt = "     2.2108 [xmp_import] creating tag: darktable|exported
     2.2139 [rawspeed] fuji_xe4_32_lossy.RAF corrupt: rawspeed::FujiDecompressor::FujiDecompressor(RawImage, ByteStream), line 854: compressed RAF header check
     2.2176 [imageio_storage_disk] could not export to file: `output.png'!";
        assert_eq!(
            loader_rejection(corrupt, "fuji_xe4_32_lossy.RAF"),
            Some(
                "[rawspeed] fuji_xe4_32_lossy.RAF corrupt: rawspeed::FujiDecompressor::FujiDecompressor(RawImage, ByteStream), line 854: compressed RAF header check"
            )
        );
        // The IIQ.
        let unsupported = "  1.0 [rawspeed] Unsupported camera model for x.IIQ\n";
        assert_eq!(
            loader_rejection(unsupported, "x.IIQ"),
            Some("[rawspeed] Unsupported camera model for x.IIQ")
        );
        // Anything else is a failure: rawspeed's errors that do not stop a decode, a file
        // name with "corrupt" in it, another file's rejection, darktable-cli's own
        // messages, a crash, a full disk, no log at all.
        for (log, file) in [
            (
                "  1.0 [rawspeed] (x.PEF) Data corrupt: v1[0]=13, expected [1..12]\n",
                "x.PEF",
            ),
            (
                "  1.0 [rawspeed] (corrupt.NEF) I/O error: read past end\n",
                "corrupt.NEF",
            ),
            (
                "  1.0 [rawspeed] (x.NEF) I/O error: corrupt.NEF\n",
                "corrupt.NEF",
            ),
            ("  1.0 [rawspeed] y.RAF corrupt: header check\n", "x.RAF"),
            (
                "  1.0 [rawspeed] Unsupported camera model for x.IIQ.bak\n",
                "x.IIQ",
            ),
            ("error: can't open file /raws/x.3FR\n", "x.3FR"),
            ("error: can't open XMP file /work/raw-stage.xmp\n", "x.NEF"),
            ("Segmentation fault: 11\n", "x.NEF"),
            (
                "[imageio_storage_disk] could not export to file: `output.png'!\n",
                "x.NEF",
            ),
            ("", "x.NEF"),
        ] {
            assert_eq!(loader_rejection(log, file), None, "{log}");
        }
    }
}
