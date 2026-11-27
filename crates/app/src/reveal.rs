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

//! Showing a file or a folder in the platform's file manager: Finder, File Explorer or the
//! desktop's, which the Jobs window, Collections and Settings › Storage ask for, and the words for it, "Show in Finder", "Show in Explorer" or "Show in Files".

use std::path::PathBuf;

use apertrust_ui::components::FileManager;

/// What Show in Finder reveals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reveal {
    /// A file written, selected in its folder.
    File(PathBuf),
    /// A folder, opened.
    Folder(PathBuf),
}

/// The program and arguments that show `what` in the platform's file manager.
pub fn reveal_command(what: &Reveal) -> (&'static str, Vec<std::ffi::OsString>) {
    let (file, path) = match what {
        Reveal::File(path) => (true, path),
        Reveal::Folder(path) => (false, path),
    };
    // Absolute, so a name that starts with `-` is never read as an option.
    let path = &std::path::absolute(path).unwrap_or_else(|_| path.clone());
    if cfg!(target_os = "macos") {
        // Finder opens the folder with the file selected, or the folder itself.
        let mut args = Vec::new();
        if file {
            args.push("-R".into());
        }
        args.push(path.into());
        ("open", args)
    } else if cfg!(target_os = "windows") {
        let mut arg = std::ffi::OsString::from(if file { "/select," } else { "" });
        arg.push(path);
        ("explorer", vec![arg])
    } else {
        // The desktop's file manager opens a folder; a file shows by its folder.
        let folder = if file {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        ("xdg-open", vec![folder.into()])
    }
}

/// Shows `what` in the platform's file manager, started and left to its own: the UI thread
/// never waits for it.
pub fn reveal(what: &Reveal) {
    let (program, args) = reveal_command(what);
    match std::process::Command::new(program).args(args).spawn() {
        Ok(mut child) => {
            // Reaped by a thread of its own, so it never lingers as a zombie; a failed
            // call is logged.
            std::thread::spawn(move || match child.wait() {
                Ok(status) if !status.success() => {
                    tracing::warn!(target: "pipe", "the file manager call failed: {status}");
                }
                Err(e) => tracing::warn!(target: "pipe", "the file manager call failed: {e}"),
                Ok(_) => {}
            });
        }
        Err(e) => tracing::warn!(target: "pipe", "cannot show the export in the file manager: {e}"),
    }
}

/// The platform's file manager, for the button's words.
pub const FILE_MANAGER: FileManager = if cfg!(target_os = "macos") {
    FileManager::Finder
} else if cfg!(target_os = "windows") {
    FileManager::Explorer
} else {
    FileManager::Files
};

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn finder_selects_a_file_and_opens_a_folder() {
        let file = Reveal::File(PathBuf::from("/out/a b.jpg"));
        let (program, args) = reveal_command(&file);
        assert_eq!(
            (program, args),
            ("open", vec!["-R".into(), "/out/a b.jpg".into()])
        );
        let folder = Reveal::Folder(PathBuf::from("/out"));
        let (program, args) = reveal_command(&folder);
        assert_eq!((program, args), ("open", vec!["/out".into()]));
        // A relative name starting with a dash is made absolute, never read as an option.
        let dashed = Reveal::Folder(PathBuf::from("-p"));
        let (_, args) = reveal_command(&dashed);
        assert!(Path::new(&args[0]).is_absolute(), "{args:?}");
    }
}
