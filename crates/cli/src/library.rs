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

//! `library`: the library's commands, run in this process (decision 54).

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use apertrust_library::{Counts, Index, Options, Summary};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List one folder, not its subfolders: its raws, the `.apxmp` sidecars beside them
    /// and the sidecars whose raw is gone, read 16 at a time, and print the counts: the
    /// photos with an edit, those that open for viewing only and the sidecars that could
    /// not be read. The times of the listing and the reading are a `-d perf` line.
    Scan { dir: PathBuf },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Scan { dir } => scan(dir),
    }
}

fn scan(dir: PathBuf) -> Result<()> {
    let index = Index::build(&dir, Options::default(), &|| false).map_err(|e| anyhow!("{e}"))?;
    print!("{}", counts(&index.summary, &index.counts()));
    if index.cancelled {
        println!("cancelled: the counts are of what was reached");
    }
    Ok(())
}

/// The counts, one to a line.
fn counts(summary: &Summary, sidecars: &Counts) -> String {
    let mut text = format!(
        "raws: {}\nwith sidecars: {}\norphan sidecars: {}\nother files skipped: {}\n",
        summary.raws, summary.with_sidecar, summary.orphans, summary.other
    );
    text.push_str(&format!(
        "with edits: {}\nview-only: {}\nunreadable sidecars: {}\n",
        sidecars.edited, sidecars.view_only, sidecars.unreadable
    ));
    if summary.hidden > 0 {
        text.push_str(&format!(
            "hidden files and folders skipped: {}\n",
            summary.hidden
        ));
    }
    if summary.stale_removed > 0 {
        text.push_str(&format!(
            "old temporary files removed: {}\n",
            summary.stale_removed
        ));
    }
    if summary.not_text > 0 {
        text.push_str(&format!(
            "names that are not text, not listed: {}\n",
            summary.not_text
        ));
    }
    if summary.folders > 0 {
        text.push_str(&format!("subfolders not listed: {}\n", summary.folders));
    }
    if summary.unreadable > 0 {
        text.push_str(&format!(
            "files that could not be read: {}\n",
            summary.unreadable
        ));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_name_what_is_skipped_only_when_there_is_some() {
        let sidecars = Counts {
            edited: 2,
            view_only: 1,
            unreadable: 0,
        };
        let summary = Summary {
            raws: 3,
            with_sidecar: 2,
            orphans: 1,
            other: 4,
            ..Summary::default()
        };
        assert_eq!(
            counts(&summary, &sidecars),
            "raws: 3\nwith sidecars: 2\norphan sidecars: 1\nother files skipped: 4\n\
             with edits: 2\nview-only: 1\nunreadable sidecars: 0\n"
        );
        let summary = Summary {
            hidden: 5,
            stale_removed: 3,
            folders: 2,
            unreadable: 1,
            ..summary
        };
        let text = counts(&summary, &sidecars);
        assert!(text.contains("hidden files and folders skipped: 5\n"));
        assert!(text.contains("old temporary files removed: 3\n"));
        assert!(text.contains("subfolders not listed: 2\n"));
        assert!(text.contains("files that could not be read: 1\n"));
    }
}
