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

//! The startup banner as `apertrust-cli` prints it: on stderr with a debug domain on,
//! never on stdout, which carries only results, and not at all without one.

use std::process::{Command, Output};

/// The CLI with `args`, on a raw that is not there: it fails before it needs a GPU. Its
/// user data folder is one of its own, removed after, never a person's.
fn run(args: &[&str]) -> Output {
    let folder = std::env::temp_dir().join(format!(
        "apertrust-cli-banner-{}-{}",
        args.join("-").replace(['.', '/'], "_"),
        std::process::id()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"))
        .args(args)
        .env_remove("RUST_LOG")
        .env("APERTRUST_USER_DATA_DIR", &folder)
        .output()
        .expect("the CLI runs");
    let _ = std::fs::remove_dir_all(&folder);
    output
}

#[test]
fn a_debug_domain_puts_the_banner_on_stderr_only() {
    let with = run(&["-d", "pipe", "edit", "no-such-raw.CR2"]);
    let stderr = String::from_utf8_lossy(&with.stderr);
    let first = format!("Apertrust CLI {} [", apertrust_base::program!().version);
    assert!(stderr.starts_with(&first), "{stderr}");
    for line in [
        "\nCopyright (C) 2026 Apertrust developers\n",
        "\nLicence: GPL-3.0-or-later\n",
        "\nBuild:\n  wgpu                   -> ",
        "\n  rawler                 -> ",
        "\n  Slang kernels          -> ",
        "/issues to report bugs.\n",
        " -d pipe edit no-such-raw.CR2\n",
    ] {
        assert!(stderr.contains(line), "{line:?} in:\n{stderr}");
    }
    assert!(stderr.contains("\n[starting] as: "), "{stderr}");
    assert!(
        with.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&with.stdout)
    );

    let without = run(&["edit", "no-such-raw.CR2"]);
    let stderr = String::from_utf8_lossy(&without.stderr);
    assert!(!stderr.contains("Build:"), "{stderr}");
    assert!(without.stdout.is_empty());
}

#[test]
fn version_prints_the_banner_on_stdout() {
    for flag in ["--version", "-V"] {
        let output = run(&[flag]);
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first = format!("Apertrust CLI {} [", apertrust_base::program!().version);
        assert!(stdout.starts_with(&first), "{stdout}");
        assert!(stdout.contains("\nBuild:\n  wgpu "), "{stdout}");
        assert!(stdout.ends_with("/issues to report bugs.\n"), "{stdout}");
        assert!(!stdout.contains("[starting]"), "{stdout}");
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
