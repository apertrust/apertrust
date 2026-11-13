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

//! The safe write (decision 53), behind the `fs` feature, with no `unsafe`.
//!
//! Every file Apertrust writes into a person's folder is written whole to a hidden
//! temporary file beside its name and renamed into place, so a NAS, a cloud folder or a
//! crash only ever shows a complete file under a photo's name:
//!
//! - [`write()`] replaces a file with new bytes: written, flushed, synced and renamed, not
//!   at all when the file already holds them, with nothing left behind and the old file
//!   untouched on an error;
//! - [`create`] makes the hidden temporary (`.~NAME.XXXXXX.tmp`, `NAME` cut to fit a file
//!   system's 255 bytes), for a writer that fills the file itself, as the export's does;
//!   [`is_temporary_name`] tells such a name, and [`remove_if_stale`] removes those a
//!   crash left;
//! - [`persist_retrying`] is the rename, tried again for about a second where Windows
//!   refuses it because an indexer or a virus scanner holds the file.
//!
//! Bulk listing and cloud placeholders' flags, which need foreign calls, are planned.

use std::borrow::Cow;
use std::path::Path;

mod temporary;
mod write;

pub use temporary::{STALE_AFTER, create, is_temporary_name, remove_if_stale};
pub use write::{Retry, Written, persist_retrying, write, write_metadata};

/// `path` as `tempfile` must be given it to reach any length: on Windows absolute, with
/// `\\?\` before it, since `tempfile` hands its paths to Windows as they are, where a path
/// of 260 characters or more is not found (std's own calls add the prefix themselves);
/// elsewhere, or where it cannot be made absolute, `path` as it is.
fn long_path(path: &Path) -> Cow<'_, Path> {
    #[cfg(windows)]
    if let Some(verbatim) = verbatim(path) {
        return Cow::Owned(verbatim);
    }
    Cow::Borrowed(path)
}

/// `path` made absolute and verbatim, `\\?\C:\…` or `\\?\UNC\server\share\…`; `None` for one
/// that is verbatim already or cannot be made absolute.
#[cfg(windows)]
fn verbatim(path: &Path) -> Option<std::path::PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Component, Prefix};
    // `absolute` resolves `.`, `..` and `/`, which a verbatim path would take literally.
    let full = std::path::absolute(path).ok()?;
    let Some(Component::Prefix(prefix)) = full.components().next() else {
        return None;
    };
    let wide = full.as_os_str().encode_wide();
    let long: Vec<u16> = match prefix.kind() {
        Prefix::Disk(_) => r"\\?\".encode_utf16().chain(wide).collect(),
        // `\\server\share\…` with its first backslash taken by `UNC`.
        Prefix::UNC(..) => r"\\?\UNC".encode_utf16().chain(wide.skip(1)).collect(),
        _ => return None,
    };
    Some(OsString::from_wide(&long).into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn a_windows_path_is_made_verbatim_and_resolved() {
        let long = |path: &str| long_path(Path::new(path)).into_owned();
        assert_eq!(long(r"C:\a\..\b/c.jpg"), Path::new(r"\\?\C:\b\c.jpg"));
        assert_eq!(
            long(r"\\nas\photos\out\a.jpg"),
            Path::new(r"\\?\UNC\nas\photos\out\a.jpg")
        );
        assert_eq!(long(r"\\?\C:\x\y"), Path::new(r"\\?\C:\x\y"));
        // A relative path starts in the current folder.
        let here = std::env::current_dir().expect("the current folder");
        let expected = format!(r"\\?\{}", here.join("a.jpg").display());
        assert_eq!(long("a.jpg"), Path::new(&expected));
    }
}
