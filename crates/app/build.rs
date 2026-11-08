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

//! Generates the app's settings from the settings schema, `data/settings.schema.toml`,
//! as typed statics in `OUT_DIR/settings.rs`, which `src/settings.rs` includes as its
//! `registry` (decision 43). The generator embeds the schema, so a change to it reruns
//! this script; one it refuses fails the build, naming the line.
//!
//! It reads Apertrust's version and the versions the startup banner names, for
//! `apertrust_base::program!()` (decision 44): in the binary's own build, so that a
//! commit rebuilds the binaries and no library.
//!
//! On macOS, it also embeds `assets/macos/Info.plist` in the app's binary, in the section
//! macOS reads a binary's information from when it runs without a bundle
//! (`__TEXT,__info_plist`). It names the app Apertrust in the menu bar: run with
//! `cargo run`, macOS would otherwise title the app menu with the binary's name,
//! "apertrust". The Dock and the app switcher take the binary's name all the same, from
//! LaunchServices, until the app is packaged with the bundle's own Info.plist,
//! `packaging/macos/Info.plist`, whose keys agree with these (`tests/packaging.rs`).
//!
//! On Windows, it compiles the executable's resources with the SDK's resource compiler,
//! through winresource: the app icon, `assets/icons/apertrust.ico`, which Explorer, the
//! taskbar and the installer's shortcuts show, and the version resource, the build's
//! version as File Explorer's Details tab and the installer read it (docs/packaging.md).

use std::env;
use std::path::Path;

fn main() {
    apertrust_base::settings::codegen::build("app");
    #[cfg_attr(not(windows), allow(unused_variables))]
    let version = apertrust_base::build::version();

    #[cfg(windows)]
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_resources(&version);
    }

    let plist = Path::new("assets/macos/Info.plist");
    println!("cargo:rerun-if-changed={}", plist.display());
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let manifest = env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
        let plist = Path::new(&manifest).join(plist);
        println!(
            "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            plist.display()
        );
    }
}

/// The icon and the version resource of `apertrust.exe`: its four numbers the release's
/// and the count of commits, its strings the full version, as the banner gives it, and
/// the name Apertrust Dev for a development build.
#[cfg(windows)]
fn windows_resources(version: &apertrust_base::build::Version) {
    let icon = "assets/icons/apertrust.ico";
    println!("cargo:rerun-if-changed={icon}");
    let [x, y, z, build] = version.numbers().map(u64::from);
    let numbers = x << 48 | y << 32 | z << 16 | build;
    let name = if version.dev {
        "Apertrust Dev"
    } else {
        "Apertrust"
    };
    let mut resources = winresource::WindowsResource::new();
    resources
        .set_icon(icon)
        .set("ProductName", name)
        .set("FileDescription", name)
        .set("CompanyName", "Apertrust developers")
        .set("LegalCopyright", "Copyright (C) 2026 Apertrust developers")
        .set("InternalName", "apertrust")
        .set("OriginalFilename", "apertrust.exe")
        .set("ProductVersion", &version.full)
        .set("FileVersion", &version.full)
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, numbers)
        .set_version_info(winresource::VersionInfo::FILEVERSION, numbers);
    if let Err(failed) = resources.compile() {
        panic!("the Windows resources (the SDK's rc.exe): {failed}");
    }
}
