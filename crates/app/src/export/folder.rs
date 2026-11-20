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

//! The path's folder button (`docs/gui.md`, The Export tool): darktable's "select
//! directory" (`src/imageio/storage/disk.c`, `button_clicked`, at the reference release).
//!
//! - **Where the dialog opens** ([`look`]): at the path as written, `~` expanded, where it
//!   is a folder, its file part then `$(FILE_NAME)`; else at the folder part before its
//!   last separator, where that is a folder; else at the home folder. The file part is
//!   what follows the last separator, kept as written.
//! - **What the path becomes** ([`destination`]): the folder chosen, under the home folder
//!   written `~/`, then the file part kept. On Windows the folder's backslashes become `/`;
//!   elsewhere a backslash in a folder's name is escaped as `\\`, darktable's rule, so
//!   variables stay unambiguous.
//!
//! Whether a path is a folder is asked of the disk on a thread of its own, never the
//! interface's, as drops are, so a slow share never holds a frame; the dialog opens a
//! later frame ([`Looking`]).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use apertrust_export::disk::home_for_tilde;
use eframe::egui;

/// Whether this is Windows, whose paths take `\` as a separator and whose chosen folders'
/// backslashes become `/`.
pub const WINDOWS: bool = cfg!(windows);

/// The file part of a path that is a folder alone, darktable's `$(FILE.NAME)`.
pub const FILE_NAME: &str = "$(FILE_NAME)";

/// What the folder button found of the path as it was pressed: the folder its dialog
/// opens at, and the file part the chosen folder is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub start: Option<PathBuf>,
    pub file: String,
}

/// Where the folder dialog opens for `path`, and the file part it keeps ([`Look`]):
/// darktable's rule, with `home` for a leading `~` and where nothing else is a folder, and
/// `is_dir` asking the disk. `windows` takes `\` as a separator too, as glib does there.
pub fn look(
    path: &str,
    home: Option<&Path>,
    windows: bool,
    is_dir: impl Fn(&Path) -> bool,
) -> Look {
    let expanded = home_for_tilde(path, home);
    let whole = PathBuf::from(literal(&expanded, windows));
    if !expanded.is_empty() && is_dir(&whole) {
        return Look {
            start: Some(whole),
            file: FILE_NAME.to_owned(),
        };
    }
    let (folder, file) = split(&expanded, windows);
    let folder = folder
        .map(|folder| PathBuf::from(literal(folder, windows)))
        .filter(|folder| is_dir(folder));
    Look {
        start: folder.or_else(|| home.map(Path::to_owned)),
        file: file.to_owned(),
    }
}

/// `path` at its last separator: the folder part before it, none where there is no
/// separator, and the file part after it. A separator inside a variable, as in
/// `$(FILE_NAME/a/b)`, is the variable's; elsewhere than Windows a `\` escapes the
/// character after it. The root's own separator is kept with it, `/` for `/IMG`, and on
/// Windows a drive's, `C:/` for `C:/IMG`, since `C:` alone is the drive's working folder.
fn split(path: &str, windows: bool) -> (Option<&str>, &str) {
    let mut last = None;
    let (mut depth, mut chars) = (0usize, path.char_indices().peekable());
    while let Some((at, c)) = chars.next() {
        match c {
            '\\' if !windows => {
                chars.next();
            }
            '$' if chars.peek().is_some_and(|(_, next)| *next == '(') => {
                chars.next();
                depth += 1;
            }
            ')' if depth > 0 => depth -= 1,
            '/' if depth == 0 => last = Some(at),
            '\\' if depth == 0 => last = Some(at),
            _ => {}
        }
    }
    let Some(at) = last else {
        return (None, path);
    };
    let root = at == 0 || (windows && path[..at].ends_with(':'));
    let folder = if root { &path[..=at] } else { &path[..at] };
    (Some(folder), &path[at + 1..])
}

/// The path once `folder` is chosen, with `file` the file part kept ([`look`]): under
/// `home` written `~/`, as the Summary shows it; on Windows (`windows`) its backslashes
/// `/`, elsewhere each backslash escaped as `\\`.
/// `text`, a folder part of a path pattern, as the pattern reads it off Windows: a
/// backslash takes the character after it as it is, as `apertrust_export::variables`
/// parses it. On Windows, where the backslash is a separator, `text` as it is. The
/// filesystem is asked about this, never the escaped text.
fn literal(text: &str, windows: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let escaped = !windows && c == '\\' && chars.peek().is_some();
        if escaped {
            out.extend(chars.next());
        } else {
            out.push(c);
        }
    }
    out
}

pub fn destination(folder: &Path, file: &str, home: Option<&Path>, windows: bool) -> String {
    let under = home.and_then(|home| folder.strip_prefix(home).ok());
    let text = under.unwrap_or(folder).to_string_lossy();
    let text = if windows {
        text.replace('\\', "/")
    } else {
        text.replace('\\', "\\\\")
    };
    // Off Windows, a `$` the folder's name holds is escaped too, so `$(` there is never
    // read as a variable. On Windows the backslash is a separator, so there is no escape,
    // as darktable has none.
    let text = if windows {
        text
    } else {
        text.replace('$', "\\$")
    };
    let folder = match under {
        Some(_) if text.is_empty() => "~".to_owned(),
        Some(_) => format!("~/{text}"),
        None => text,
    };
    format!("{}/{file}", folder.trim_end_matches('/'))
}

/// The folder button's look at the disk ([`look`]), on a thread of its own: the latest
/// press's, answered a later frame. A share that hangs keeps its thread until it answers;
/// a later press starts a thread of its own and is the one taken.
#[derive(Debug, Default)]
pub struct Looking {
    answer: Option<Receiver<Look>>,
}

impl Looking {
    /// Looks at `path` as the folder button was pressed with it, `home` the person's, and
    /// asks `ctx` for a frame when the answer comes.
    pub fn start(&mut self, path: String, home: Option<PathBuf>, ctx: &egui::Context) {
        let (send, answer) = mpsc::channel();
        let ctx = ctx.clone();
        let started = std::thread::Builder::new()
            .name("export folder".into())
            .spawn(move || {
                let found = look(&path, home.as_deref(), WINDOWS, Path::is_dir);
                let _ = send.send(found);
                ctx.request_repaint();
            });
        match started {
            Ok(_) => self.answer = Some(answer),
            Err(e) => {
                tracing::warn!(target: "settings", "the folder dialog not opened: no thread: {e}")
            }
        }
    }

    /// What the look found, once it has; nothing while it looks.
    pub fn looked(&mut self) -> Option<Look> {
        match self.answer.as_ref()?.try_recv() {
            Ok(found) => {
                self.answer = None;
                Some(found)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.answer = None;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "/home/anna";

    /// `path` looked at with Anna's home, where `folders` are the folders there.
    fn look_at(path: &str, folders: &[&str]) -> Look {
        let is_dir = |p: &Path| folders.iter().any(|f| Path::new(f) == p);
        look(path, Some(Path::new(HOME)), false, is_dir)
    }

    fn at(start: &str, file: &str) -> Look {
        Look {
            start: Some(PathBuf::from(start)),
            file: file.to_owned(),
        }
    }

    /// A folder chosen with a backslash or `$(` in its name is written escaped, and found
    /// again under its own name the next time the button is pressed.
    #[test]
    fn a_folder_with_a_backslash_or_a_dollar_is_found_again() {
        let home = Some(Path::new(HOME));
        for folder in ["/srv/a\\b", "/srv/$(YEAR)"] {
            let path = destination(Path::new(folder), FILE_NAME, home, false);
            assert_eq!(look_at(&path, &[folder]), at(folder, FILE_NAME), "{path}");
        }
        assert_eq!(
            destination(Path::new("/srv/$(YEAR)"), FILE_NAME, home, false),
            "/srv/\\$(YEAR)/$(FILE_NAME)"
        );
        // On Windows the backslash is a separator, so a `$` is written as it is.
        assert_eq!(
            destination(Path::new("D:\\$(x)"), FILE_NAME, None, true),
            "D:/$(x)/$(FILE_NAME)"
        );
    }

    #[test]
    fn the_dialog_opens_at_the_path_its_folder_part_or_the_home_folder() {
        let folders = [HOME, "/home/anna/Desktop", "/srv/out"];
        // A path that is a folder: there, its file part $(FILE_NAME).
        assert_eq!(
            look_at("/home/anna/Desktop", &folders),
            at("/home/anna/Desktop", FILE_NAME)
        );
        assert_eq!(
            look_at("~/Desktop", &folders),
            at("/home/anna/Desktop", FILE_NAME)
        );
        // The folder part before the last separator, the file part kept as written.
        assert_eq!(
            look_at("/srv/out/$(FILE_NAME)_web", &folders),
            at("/srv/out", "$(FILE_NAME)_web")
        );
        assert_eq!(
            look_at("~/Desktop/$(YEAR)", &folders),
            at("/home/anna/Desktop", "$(YEAR)")
        );
        // A folder part with variables, or one not there: the home folder.
        assert_eq!(
            look_at("$(FILE_FOLDER)/darktable_exported/$(FILE_NAME)", &folders),
            at(HOME, "$(FILE_NAME)")
        );
        assert_eq!(look_at("/gone/away/IMG", &folders), at(HOME, "IMG"));
        // No separator: no folder part, the whole path the file part.
        assert_eq!(look_at("$(FILE_NAME)", &folders), at(HOME, "$(FILE_NAME)"));
        // With no home, no folder to open at.
        let none = look("$(FILE_NAME)", None, false, |_| false);
        assert_eq!(none.start, None);
    }

    #[test]
    fn a_separator_inside_a_variable_or_escaped_is_not_the_last() {
        assert_eq!(
            split("/srv/out/$(FILE_NAME/a/b)", false),
            (Some("/srv/out"), "$(FILE_NAME/a/b)")
        );
        assert_eq!(
            split("/srv/$(FILE_FOLDER/x/y)/$(FILE_NAME)", false),
            (Some("/srv/$(FILE_FOLDER/x/y)"), "$(FILE_NAME)")
        );
        // An escaped backslash ends with the next character, so a separator after it is one.
        assert_eq!(
            split("/srv/a\\\\b/IMG", false),
            (Some("/srv/a\\\\b"), "IMG")
        );
        assert_eq!(split("/IMG", false), (Some("/"), "IMG"));
        // Windows takes the backslash as a separator, as glib does there.
        assert_eq!(
            split("C:\\out\\$(FILE_NAME)", true),
            (Some("C:\\out"), "$(FILE_NAME)")
        );
        assert_eq!(
            split("C:/$(FILE_NAME)", true),
            (Some("C:/"), "$(FILE_NAME)")
        );
        assert_eq!(split("a\\b", false), (None, "a\\b"));
    }

    #[test]
    fn the_chosen_folder_is_followed_by_the_file_part_under_home_as_tilde() {
        let home = Some(Path::new(HOME));
        let chosen = |folder: &str, file: &str| destination(Path::new(folder), file, home, false);
        assert_eq!(
            chosen("/home/anna/Desktop", FILE_NAME),
            "~/Desktop/$(FILE_NAME)"
        );
        assert_eq!(chosen(HOME, FILE_NAME), "~/$(FILE_NAME)");
        assert_eq!(
            chosen("/srv/out", "$(FILE_NAME)_web"),
            "/srv/out/$(FILE_NAME)_web"
        );
        assert_eq!(chosen("/", "IMG"), "/IMG");
        // Another user's home is not under Anna's.
        assert_eq!(
            chosen("/home/annabel", FILE_NAME),
            "/home/annabel/$(FILE_NAME)"
        );
        assert_eq!(
            destination(Path::new("/home/anna/Desktop"), FILE_NAME, None, false),
            "/home/anna/Desktop/$(FILE_NAME)"
        );
    }

    #[test]
    fn a_backslash_becomes_a_slash_on_windows_and_is_escaped_elsewhere() {
        let folder = Path::new("C:\\Users\\anna\\Desktop");
        assert_eq!(
            destination(folder, FILE_NAME, None, true),
            "C:/Users/anna/Desktop/$(FILE_NAME)"
        );
        assert_eq!(
            destination(Path::new("C:\\"), FILE_NAME, None, true),
            "C:/$(FILE_NAME)"
        );
        let home = Some(Path::new(HOME));
        assert_eq!(
            destination(Path::new("/home/anna/a\\b"), FILE_NAME, home, false),
            "~/a\\\\b/$(FILE_NAME)"
        );
    }

    #[test]
    fn a_look_on_its_thread_answers_a_later_frame() {
        let folder = std::env::temp_dir();
        let ctx = egui::Context::default();
        let mut looking = Looking::default();
        assert_eq!(looking.looked(), None);
        looking.start(folder.to_string_lossy().into_owned(), None, &ctx);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let found = loop {
            if let Some(found) = looking.looked() {
                break found;
            }
            assert!(std::time::Instant::now() < deadline, "never answered");
            std::thread::yield_now();
        };
        assert_eq!(
            found,
            Look {
                start: Some(folder),
                file: FILE_NAME.to_owned()
            }
        );
        assert_eq!(looking.looked(), None);
    }
}
