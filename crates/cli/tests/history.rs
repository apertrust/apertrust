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

//! `apertrust-cli edit --note` and `history` (decision 78) as a script runs them: what
//! each prints and its exit status, on a photo of the test's own.

use std::path::PathBuf;
use std::process::{Command, Output};

/// A folder of a test's own holding a photo, `IMG.ARW`, which is no raw, and an empty user
/// data folder, removed when the test ends.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path = std::env::temp_dir().join(format!(
            "apertrust-cli-history-run-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join("user")).expect("a temporary folder");
        std::fs::write(path.join("IMG.ARW"), b"not read").unwrap();
        Folder(path)
    }

    /// The CLI with `args`, then the photo: its stdout, its stderr and its exit status.
    fn run(&self, args: &[&str]) -> (String, String, Option<i32>) {
        let Output {
            status,
            stdout,
            stderr,
        } = Command::new(env!("CARGO_BIN_EXE_apertrust-cli"))
            .arg("--user-data-dir")
            .arg(self.0.join("user"))
            .args(args)
            .arg(self.0.join("IMG.ARW"))
            .env_remove("RUST_LOG")
            .env_remove("APERTRUST_USER_DATA_DIR")
            .output()
            .expect("the CLI runs");
        let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
        (text(stdout), text(stderr), status.code())
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `history` lists the steps `edit` adds with their note, after the default steps `edit`
/// gave the photo first, moves the end, says why a move changes nothing, and fails with 1
/// on a step the history lacks and 2 on two moves at once, writing nothing; `edit` says a
/// note it did not use.
#[test]
fn history_lists_and_moves_with_the_cli_s_exit_statuses() {
    let folder = Folder::new("statuses");
    let note = ["edit", "--set", "exposure.exposure=1", "--note", "Brighter"];
    let (_, stderr, status) = folder.run(&note);
    assert_eq!(status, Some(0), "{stderr}");

    let (stdout, stderr, status) = folder.run(&["history"]);
    assert_eq!(status, Some(0), "{stderr}");
    assert!(
        stdout.starts_with("IMG.ARW: 10 steps, the end at step 10; times in UTC\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains(">  10  exposure ↑ · \"Brighter\"  "),
        "{stdout}"
    );
    assert!(
        stdout.contains("\n  1–9  defaults: 9 modules every photo runs\n"),
        "{stdout}"
    );
    assert!(stdout.ends_with("   0  Original\n"), "{stdout}");

    let (stdout, stderr, status) = folder.run(&["history", "--undo"]);
    assert_eq!(status, Some(0), "{stderr}");
    assert!(
        stdout.starts_with("Undid step 10: the end at step 9\nWrote "),
        "{stdout}"
    );
    // Never over a default step.
    let (stdout, _, status) = folder.run(&["history", "--undo"]);
    assert_eq!(status, Some(0));
    assert!(stdout.starts_with("Nothing to undo\nIMG.ARW: "), "{stdout}");

    let sidecar = folder.0.join("IMG.ARW.apxmp");
    let before = std::fs::read(&sidecar).unwrap();
    let (_, stderr, status) = folder.run(&["history", "--go-to", "11"]);
    assert_eq!(status, Some(1));
    assert!(
        stderr.contains("no step 11: the history has 10 steps; 0 is Original"),
        "{stderr}"
    );
    let (_, stderr, status) = folder.run(&["history", "--undo", "--redo"]);
    assert_eq!(status, Some(2), "{stderr}");
    assert_eq!(std::fs::read(&sidecar).unwrap(), before);

    // An empty note is a usage error too; a note with nothing to change is not used.
    let (_, _, status) = folder.run(&["edit", "--set", "exposure.exposure=2", "--note", ""]);
    assert_eq!(status, Some(2));
    let (stdout, _, status) = folder.run(&["edit", "--note", "Nothing"]);
    assert_eq!(status, Some(0));
    assert!(
        stdout.starts_with("Nothing changed: the note was not used\n"),
        "{stdout}"
    );
    assert_eq!(std::fs::read(&sidecar).unwrap(), before);
}
