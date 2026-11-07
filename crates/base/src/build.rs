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

//! What the startup banner reports about the build, read from the build rather than
//! typed by hand (decision 44): the versions of the libraries it names, from the
//! workspace's `Cargo.lock`, each left out when it cannot be read; the Slang version the
//! committed kernels were generated with, the one `tools/fetch-slang.sh` pins; and
//! Apertrust's own version, from git's `release-X.Y.Z` tags (`src/build/version.rs`), or
//! as a packager gives it in `APERTRUST_VERSION`. A build never fails over it: where git
//! cannot say (a shallow clone, no git), Cargo warns why and the version is `0.0.0`.
//!
//! Each binary's build script reads them, with [`version`], behind the `build` feature,
//! which only build-dependencies turn on, and the binary names itself with
//! `apertrust_base::program!()`, which reads what the script handed its compiler. The
//! binaries' builds, not this crate's: the version changes with every commit, and only
//! what reads it rebuilds, so a commit rebuilds the binaries and no library.
//!
//! ```ignore
//! // build.rs
//! fn main() {
//!     apertrust_base::build::version();
//! }
//! ```

use std::path::{Path, PathBuf};
use std::{env, fs};

mod version;

pub use version::Version;
use version::git;

/// The packages whose versions the banner can name: a banner line for another library
/// adds its package here.
const PACKAGES: &[&str] = &["wgpu", "naga", "eframe", "egui", "rawler", "accesskit"];

/// Reads the versions in a binary's build script and hands them to its compiler, for
/// `apertrust_base::program!()`, each an empty value where it has none:
/// `APERTRUST_BUILT_VERSION`, `APERTRUST_BUILT_NUMBER`, `APERTRUST_BUILT_PACKAGES`
/// (`wgpu=30.0.1,naga=30.0.1`) and `APERTRUST_BUILT_SLANG`. The script runs again only
/// when one can change ([`watch_checkout`]). Returns Apertrust's version, for what else
/// the script stamps with it: the Windows executable's version resource.
pub fn version() -> Version {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
    // A binary's package is a folder of `crates`, as every crate of the workspace.
    let root = manifest.join("../..");
    let lock = root.join("Cargo.lock");
    let slang = root.join("tools/fetch-slang.sh");
    for path in [&lock, &slang] {
        watch(path);
    }
    let lockfile = fs::read_to_string(&lock).unwrap_or_default();
    let slang = fs::read_to_string(&slang)
        .ok()
        .and_then(|script| slang_pin(&script));

    // Also covers the variable being unset later, which sends the build back to git.
    println!("cargo:rerun-if-env-changed=APERTRUST_VERSION");
    let given = env::var("APERTRUST_VERSION").ok();
    if given.as_deref().is_none_or(str::is_empty) {
        watch_checkout(&root);
    }
    let apertrust = version::read(&root, given.as_deref()).unwrap_or_else(|why| {
        // Cargo reads its instructions line by line.
        println!(
            "cargo:warning=the version is 0.0.0: {}",
            why.replace('\n', " ")
        );
        Version {
            full: "0.0.0".into(),
            build: None,
        }
    });

    // Cargo rebuilds the binary's package whenever this runs, whatever it hands over:
    // hence the few paths it watches.
    let built = [
        ("APERTRUST_BUILT_VERSION", apertrust.full.clone()),
        (
            "APERTRUST_BUILT_NUMBER",
            apertrust.build.map(|n| n.to_string()).unwrap_or_default(),
        ),
        ("APERTRUST_BUILT_PACKAGES", packages(&lockfile)),
        ("APERTRUST_BUILT_SLANG", slang.unwrap_or_default()),
    ];
    for (name, value) in built {
        // One line each, whatever a packager's version holds.
        println!(
            "cargo:rustc-env={name}={}",
            value.replace(['\n', '\r'], " ")
        );
    }
    apertrust
}

/// The packages the banner can name, as `Cargo.lock` locks them: `wgpu=30.0.1,naga=…`,
/// one left out when it cannot be read.
fn packages(lockfile: &str) -> String {
    PACKAGES
        .iter()
        .filter_map(|package| {
            locked(lockfile, package).map(|version| format!("{package}={version}"))
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The Slang version `tools/fetch-slang.sh` pins.
fn slang_pin(script: &str) -> Option<String> {
    script
        .lines()
        .find_map(|line| line.strip_prefix("VERSION=").map(str::to_owned))
        .filter(|version| !version.is_empty())
}

/// The highest version of `package` in a `Cargo.lock`.
fn locked(lockfile: &str, package: &str) -> Option<String> {
    let name = format!("name = \"{package}\"");
    let mut lines = lockfile.lines();
    let mut found: Vec<String> = Vec::new();
    while let Some(line) = lines.next() {
        if line == name
            && let Some(version) = lines
                .next()
                .and_then(|next| next.strip_prefix("version = \""))
                .and_then(|rest| rest.strip_suffix('"'))
        {
            found.push(version.to_owned());
        }
    }
    let key = |version: &String| -> Vec<u64> {
        version
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|part| part.parse().ok())
            .collect()
    };
    found.into_iter().max_by_key(key)
}

/// Runs the build script again when the version can change: when HEAD moves (a commit, a
/// checkout, a reset or a rebase, which HEAD's reflog records), when a tag comes or goes
/// (loose, packed or in a reftable), and when a shallow clone gets its history. Not on
/// an edit: the binary's package would rebuild with it, its library and test binaries
/// too, even when nothing here changes. So `-dirty` says whether tracked files had
/// changes when HEAD last moved, or at the first build. Nor on other branches, other
/// worktrees or fetches, as far as git's files allow.
fn watch_checkout(root: &Path) {
    // Its own `.git` only, as `version::read` reads it.
    if !root.join(".git").exists() {
        return;
    }
    let dir = |args: &[&str]| git(root, args).ok().map(|dir| root.join(dir));
    // A worktree has its own HEAD; the branches and tags are the repository's.
    if let Some(git_dir) = dir(&["rev-parse", "--git-dir"]) {
        watch(&git_dir.join("HEAD"));
        watch(&git_dir.join("logs/HEAD"));
        watch(&git_dir.join("reftable"));
    }
    if let Some(common) = dir(&["rev-parse", "--git-common-dir"]) {
        watch(&common.join("packed-refs"));
        watch(&common.join("refs/tags"));
        // Every ref of a reftable repository but a worktree's HEAD, which is in its own.
        watch(&common.join("reftable"));
        // Gone once the clone has its history, which Cargo notices.
        watch(&common.join("shallow"));
        if let Ok(head) = git(root, &["symbolic-ref", "-q", "HEAD"]) {
            watch(&common.join(head));
        }
    }
}

/// Watches a path that exists, a file or a folder: one that does not would run the
/// script on every build.
fn watch(path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The versions are the build's: each package the banner names, as `Cargo.lock`
    /// locks it, and the Slang that `tools/fetch-slang.sh` pins.
    #[test]
    fn versions_come_from_the_lockfile_and_the_slang_pin() {
        // As a Windows checkout may end its lines.
        let read = |path: &str| {
            let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
            std::fs::read_to_string(format!("{root}/{path}"))
                .unwrap()
                .replace("\r\n", "\n")
        };
        let lockfile = read("Cargo.lock");
        let packages = packages(&lockfile);
        let listed: Vec<(&str, &str)> = packages
            .split(',')
            .filter_map(|entry| entry.split_once('='))
            .collect();
        let names: Vec<&str> = listed.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, PACKAGES, "{packages}");
        for (package, version) in listed {
            let entry = format!("name = \"{package}\"\nversion = \"{version}\"");
            assert!(lockfile.contains(&entry), "{entry}");
        }
        let script = read("tools/fetch-slang.sh");
        let slang = slang_pin(&script).unwrap();
        assert!(script.contains(&format!("\nVERSION={slang}\n")));
    }
}
