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

//! The banner a binary prints on stderr as it starts with a debug domain on, as
//! darktable's: the program, its version and commit, what it was built with, where to
//! read and report, and the command line.
//!
//! ```text
//! Apertrust 0.1.0+120~gf212be7-dirty [macos arm64 release]
//! Copyright (C) 2026 Apertrust developers
//! Licence: GPL-3.0-or-later
//!
//! Build:
//!   wgpu                   -> 30.0.1   - Metal (Vulkan, DX12, GL not built for macOS)
//!   rawler                 -> 0.8.0    - raw decoding
//!   ONNX Runtime           -> DISABLED - AI masks are not available yet
//!
//! See https://github.com/apertrust/apertrust for documentation.
//! See https://github.com/apertrust/apertrust/issues to report bugs.
//!
//! [starting] as: apertrust -d gpu B13A0729.CR2
//! ```
//!
//! Versions come from each binary's own build (decision 44): its `build.rs` reads the
//! workspace's `Cargo.lock`, the Slang version `tools/fetch-slang.sh` pins, and
//! Apertrust's own, from git's release tags as HEAD last moved, with
//! `apertrust_base::build::version`, and [`program!`](crate::program) hands them over
//! as a [`Program`]. In the binary, not here, so that a commit rebuilds no library.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

/// The repository: its documentation, and its issues for bug reports.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

const COPYRIGHT: &str = "Copyright (C) 2026 Apertrust developers";

/// Whether [`starting`] prints the banner: set by `init` when a domain is on.
pub(crate) static SHOWN: AtomicBool = AtomicBool::new(false);

/// A binary, as [`program!`](crate::program) names it: its package's name, and what its
/// build read: Apertrust's version, the commit count beside it, and the versions the
/// banner names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Program {
    pub name: &'static str,
    /// Apertrust's version, the same for every binary of a build, read from git as HEAD
    /// last moved (decision 44): `X.Y.Z` on a commit tagged `release-X.Y.Z`,
    /// `X.Y.Z+N~gHASH` N commits after it, `0.0.0+N~gHASH` before any release tag, N then
    /// every commit, and `-dirty` after it when tracked files had changes; `0.0.0` where
    /// git cannot say, a shallow clone or no git, with a warning from the build; or
    /// `APERTRUST_VERSION` as a packager set it. Not `Cargo.toml`'s.
    pub version: &'static str,
    /// How many commits lead to the one built, beside the version: a number that only
    /// grows along main, for what takes numbers only, as macOS's `CFBundleVersion`.
    /// `None` where the version is `0.0.0` or a packager's.
    pub build_number: Option<u32>,
    /// Whether this is a development build, Apertrust Dev, which installs and keeps its
    /// settings beside Apertrust (docs/settings.md, Development builds): a nightly
    /// package, or, once the history has a release tag, a build not on one.
    pub dev: bool,
    /// The packages the banner can name, as the build's `Cargo.lock` locks them:
    /// `wgpu=30.0.1,naga=30.0.1`.
    packages: &'static str,
    /// The Slang version the committed kernels were generated with; empty where unknown.
    slang: &'static str,
}

impl Program {
    /// A binary from what its build read, as [`program!`](crate::program) gives it: an
    /// empty build number is none, and an empty `dev` is not a development build.
    pub const fn from_build(
        name: &'static str,
        version: &'static str,
        build_number: &'static str,
        dev: &'static str,
        packages: &'static str,
        slang: &'static str,
    ) -> Self {
        let build_number = match u32::from_str_radix(build_number, 10) {
            Ok(number) => Some(number),
            Err(_) => None,
        };
        Self {
            name,
            version,
            build_number,
            dev: !dev.is_empty(),
            packages,
            slang,
        }
    }

    /// The app's name, never translated: `Apertrust`, or `Apertrust Dev` for a
    /// development build. The window's title, and the banner's first word.
    pub const fn app_name(&self) -> &'static str {
        if self.dev {
            "Apertrust Dev"
        } else {
            "Apertrust"
        }
    }

    /// The app's id, as every package names it (packaging/): a development build's has
    /// `.Dev` after it.
    pub const fn app_id(&self) -> &'static str {
        if self.dev {
            "org.apertrust.Apertrust.Dev"
        } else {
            "org.apertrust.Apertrust"
        }
    }

    /// A package's version in the build's `Cargo.lock`, for the packages the build reads.
    pub fn version_of(&self, package: &str) -> Option<&'static str> {
        self.packages
            .split(',')
            .filter_map(|entry| entry.split_once('='))
            .find(|(name, _)| *name == package)
            .map(|(_, version)| version)
    }
}

/// The calling binary as a [`Program`]: its package's name, and Apertrust's version, not
/// its package's, with the versions its banner names, as its own build read them. The
/// binary's `build.rs` calls `apertrust_base::build::version()`; without it, the
/// binary does not compile.
#[macro_export]
macro_rules! program {
    () => {
        $crate::log::Program::from_build(
            env!("CARGO_PKG_NAME"),
            env!("APERTRUST_BUILT_VERSION"),
            env!("APERTRUST_BUILT_NUMBER"),
            env!("APERTRUST_BUILT_DEV"),
            env!("APERTRUST_BUILT_PACKAGES"),
            env!("APERTRUST_BUILT_SLANG"),
        )
    };
}

/// One line of the banner's build list: a library or a feature, its version or
/// whether it is built, and what it is for or why it is missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    name: &'static str,
    value: String,
    detail: Option<String>,
}

impl Item {
    pub fn new(name: &'static str, value: impl Into<String>) -> Self {
        Self {
            name,
            value: value.into(),
            detail: None,
        }
    }

    /// A library at the version the program's build locked.
    pub fn package(program: &Program, name: &'static str, package: &str) -> Self {
        Self::new(name, program.version_of(package).unwrap_or("unknown"))
    }

    /// Something this build does not have, and what is missing for it.
    pub fn disabled(name: &'static str, why: impl Into<String>) -> Self {
        Self::new(name, "DISABLED").detail(why)
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// wgpu, with the backends `built` lists, such as `Metal (Vulkan, DX12, GL not built for
/// macOS)`.
pub fn wgpu(program: &Program, built: impl Into<String>) -> Item {
    Item::package(program, "wgpu", "wgpu").detail(built)
}

/// eframe and egui, one version when they share it.
pub fn egui(program: &Program) -> Item {
    let (eframe, egui) = (program.version_of("eframe"), program.version_of("egui"));
    let value = match (eframe, egui) {
        (Some(eframe), Some(egui)) if eframe != egui => format!("{eframe} / {egui}"),
        _ => eframe.or(egui).unwrap_or("unknown").to_owned(),
    };
    Item::new("eframe / egui", value)
}

pub fn rawler(program: &Program) -> Item {
    Item::package(program, "rawler", "rawler").detail("raw decoding")
}

/// The Slang compiler the committed WGSL was generated with, and the naga that checks it.
pub fn kernels(program: &Program) -> Item {
    let slang = Some(program.slang).filter(|slang| !slang.is_empty());
    Item::new("Slang kernels", slang.unwrap_or("unknown")).detail(format!(
        "generated WGSL, checked with naga {}",
        program.version_of("naga").unwrap_or("unknown")
    ))
}

pub fn accesskit(program: &Program) -> Item {
    Item::package(program, "AccessKit", "accesskit").detail("screen readers")
}

/// The interface's languages, by their identifiers.
pub fn languages(ids: &[&str]) -> Item {
    Item::new("UI languages", ids.join(", "))
}

/// ONNX Runtime, which AI masks will load at run time (AGENTS.md, Rust, not C or C++).
pub fn onnx_runtime() -> Item {
    Item::disabled("ONNX Runtime", "AI masks are not available yet")
}

/// Prints the banner on stderr when `init` switched a domain on, as the program starts;
/// `build` lists what it was built with, asked only then.
pub fn starting(program: Program, build: impl FnOnce(&Program) -> Vec<Item>) {
    if SHOWN.load(Ordering::Relaxed) {
        let text = banner(program, &build(&program), &command_line());
        let _ = std::io::stderr().lock().write_all(text.as_bytes());
    }
}

/// Whether the program was asked for its version: `--version` or `-V` as its first
/// argument, as clap reads them.
pub fn version_asked() -> bool {
    matches!(
        std::env::args_os()
            .nth(1)
            .as_deref()
            .and_then(|a| a.to_str()),
        Some("--version" | "-V")
    )
}

/// Prints the banner on stdout, without the command line, as darktable's `--version`
/// does (decision 44): one command gives a bug report the whole build.
pub fn print_version(program: Program, build: &[Item]) {
    let _ = std::io::stdout()
        .lock()
        .write_all(about(program, build).as_bytes());
}

/// The program's version with the commit, as the banner's first line gives it,
/// `0.1.0+197~g9201df0`: what About shows, so it and a log name the same build.
pub fn full_version(program: Program) -> String {
    program.version.to_owned()
}

/// The banner's text, every line ending in a newline.
pub fn banner(program: Program, build: &[Item], command: &str) -> String {
    format!("{}\n[starting] as: {command}\n", about(program, build))
}

/// The banner up to where to report bugs: what `--version` prints.
pub fn about(program: Program, build: &[Item]) -> String {
    let mut text = format!(
        "{} {} [{}]\n{COPYRIGHT}\nLicence: {}\n\nBuild:\n",
        display_name(&program),
        full_version(program),
        platform(),
        env!("CARGO_PKG_LICENSE"),
    );
    for item in build {
        text.push_str(&item_line(item));
        text.push('\n');
    }
    text.push_str(&format!(
        "\nSee {REPOSITORY} for documentation.\nSee {REPOSITORY}/issues to report bugs.\n"
    ));
    text
}

/// The program as a person writes it: `Apertrust` for the app, `Apertrust CLI` for
/// `apertrust-cli`, and `Apertrust golden` for `apertrust-golden`; `Apertrust Dev`,
/// `Apertrust Dev CLI` and so on for a development build.
fn display_name(program: &Program) -> String {
    let app = program.app_name();
    match program.name.strip_prefix("apertrust") {
        Some("") => app.to_owned(),
        Some("-cli") => format!("{app} CLI"),
        Some(rest) if rest.starts_with('-') => format!("{app} {}", &rest[1..]),
        _ => program.name.to_owned(),
    }
}

/// darktable's columns: the name to 23 characters, then the value, to 8 when a detail
/// follows.
fn item_line(item: &Item) -> String {
    match &item.detail {
        Some(detail) => format!("  {:<23}-> {:<8} - {detail}", item.name, item.value),
        None => format!("  {:<23}-> {}", item.name, item.value),
    }
}

/// `macos arm64 release`, as darktable's `mac arm neon release`.
fn platform() -> String {
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        arch => arch,
    };
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!("{} {arch} {profile}", std::env::consts::OS)
}

/// The command line as given, an argument with a space or none quoted: the banner's
/// `[starting] as:`, which the app's kept log starts with too.
pub fn command_line() -> String {
    std::env::args_os()
        .map(|arg| {
            let arg = arg.to_string_lossy();
            if arg.is_empty() || arg.contains(char::is_whitespace) {
                format!("'{arg}'")
            } else {
                arg.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The packages a build reads, as `apertrust_base::build::version` hands them over.
    const PACKAGES: &str =
        "wgpu=30.0.1,naga=30.0.1,eframe=0.37.0,egui=0.37.0,rawler=0.8.0,accesskit=0.25.2";

    /// A binary has what its build read, never its package's version: an empty build
    /// number or Slang version is none, and a package the build did not read has none.
    #[test]
    fn programs_have_what_their_build_read() {
        let program = Program::from_build(
            "apertrust-cli",
            "0.1.0+3~gabc1234",
            "3",
            "",
            PACKAGES,
            "2026.20",
        );
        assert_eq!(program.build_number, Some(3));
        assert!(!program.dev);
        assert_eq!(program.app_name(), "Apertrust");
        assert_eq!(program.app_id(), "org.apertrust.Apertrust");
        assert_eq!(full_version(program), "0.1.0+3~gabc1234");
        assert_eq!(program.version_of("naga"), Some("30.0.1"));
        assert_eq!(program.version_of("tracing"), None);
        let packaged = Program::from_build("apertrust", "1.2.3-1.fc43", "", "", "", "");
        assert_eq!(packaged.build_number, None);
        assert_eq!(
            kernels(&packaged),
            Item::new("Slang kernels", "unknown")
                .detail("generated WGSL, checked with naga unknown")
        );
        assert_eq!(egui(&packaged), Item::new("eframe / egui", "unknown"));
    }

    /// A development build names itself Apertrust Dev, in the banner as in its id.
    #[test]
    fn a_development_build_is_apertrust_dev() {
        let dev = |name| Program::from_build(name, "0.1.0+3~gabc1234", "3", "1", "", "");
        assert!(dev("apertrust").dev);
        assert_eq!(dev("apertrust").app_name(), "Apertrust Dev");
        assert_eq!(dev("apertrust").app_id(), "org.apertrust.Apertrust.Dev");
        assert_eq!(display_name(&dev("apertrust")), "Apertrust Dev");
        assert_eq!(display_name(&dev("apertrust-cli")), "Apertrust Dev CLI");
        assert_eq!(
            display_name(&dev("apertrust-golden")),
            "Apertrust Dev golden"
        );
        let text = about(dev("apertrust-cli"), &[]);
        assert!(
            text.starts_with("Apertrust Dev CLI 0.1.0+3~gabc1234 ["),
            "{text}"
        );
    }

    #[test]
    fn the_banner_names_the_program_its_build_and_where_to_report() {
        let program = Program::from_build("apertrust-cli", "0.1.0", "", "", PACKAGES, "2026.20");
        let build = [
            wgpu(&program, "Metal (Vulkan, DX12, GL not built for macOS)"),
            rawler(&program),
            kernels(&program),
            languages(&["en-GB", "uk", "pl"]),
            onnx_runtime(),
        ];
        let text = banner(program, &build, "apertrust-cli -d all info B13A0729.CR2");
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("Apertrust CLI 0.1.0"), "{text}");
        assert!(lines[0].ends_with(&format!("[{}]", platform())), "{text}");
        assert_eq!(lines[1], "Copyright (C) 2026 Apertrust developers");
        assert_eq!(lines[2], "Licence: GPL-3.0-or-later");
        assert_eq!(lines[4], "Build:");
        let wgpu = program.version_of("wgpu").unwrap();
        assert_eq!(
            lines[5],
            format!(
                "  wgpu                   -> {wgpu:<8} - Metal (Vulkan, DX12, GL not built for macOS)"
            )
        );
        assert!(
            lines[6].starts_with("  rawler                 -> "),
            "{text}"
        );
        assert!(
            lines[7].contains("- generated WGSL, checked with naga "),
            "{text}"
        );
        assert_eq!(lines[8], "  UI languages           -> en-GB, uk, pl");
        assert_eq!(
            lines[9],
            "  ONNX Runtime           -> DISABLED - AI masks are not available yet"
        );
        assert!(text.contains(&format!("See {REPOSITORY}/issues to report bugs.")));
        assert!(
            REPOSITORY.starts_with("https://github.com/"),
            "{REPOSITORY}"
        );
        assert_eq!(
            lines.last(),
            Some(&"[starting] as: apertrust-cli -d all info B13A0729.CR2")
        );
    }

    /// The whole banner byte for byte, blank lines included, as `apertrust-cli -d all gpu`
    /// printed it with a packager's version before the log moved into this crate, with
    /// the versions that build read: what bug reports carry.
    #[test]
    fn the_banner_keeps_its_bytes() {
        let program = Program::from_build(
            "apertrust-cli",
            "9.8.7-compare",
            "",
            "",
            "wgpu=30.0.1,naga=30.0.1,rawler=0.8.0",
            "2026.20",
        );
        let build = [
            wgpu(&program, "Metal (Vulkan, DX12, GL not built for macOS)"),
            rawler(&program),
            kernels(&program),
            onnx_runtime(),
        ];
        let first = format!("Apertrust CLI 9.8.7-compare [{}]", platform());
        let expected = [
            first.as_str(),
            "Copyright (C) 2026 Apertrust developers",
            "Licence: GPL-3.0-or-later",
            "",
            "Build:",
            "  wgpu                   -> 30.0.1   - Metal (Vulkan, DX12, GL not built for macOS)",
            "  rawler                 -> 0.8.0    - raw decoding",
            "  Slang kernels          -> 2026.20  - generated WGSL, checked with naga 30.0.1",
            "  ONNX Runtime           -> DISABLED - AI masks are not available yet",
            "",
            "See https://github.com/apertrust/apertrust for documentation.",
            "See https://github.com/apertrust/apertrust/issues to report bugs.",
            "",
            "[starting] as: ./target/debug/apertrust-cli -d all gpu",
            "",
        ]
        .join("\n");
        let command = "./target/debug/apertrust-cli -d all gpu";
        assert_eq!(banner(program, &build, command), expected);
    }
}
