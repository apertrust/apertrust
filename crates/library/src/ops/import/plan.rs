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

//! Under which name a photo lands in its destination folder, as the preview foresees it and
//! the copy works it out again as it goes (decision 85; `docs/library.md`, Import in folder
//! mode, The copy):
//!
//! - **A name taken:** where any of a photo's files, its main file or one of its sidecars,
//!   is at the destination under a name, the copy takes the next, `_01`, then `_02`, before
//!   the extension, so a sidecar a crash left is never overwritten ([`place`]).
//! - **Already there:** on the way, a main file there of the source's that is the same
//!   ([`place`]'s `same`: of the same size for the preview, the same byte for byte for the
//!   copy) is the photo's copy, which is skipped, its missing sidecars copied alone
//!   ([`beside`]), unless one there differs from the source's, a diverged edit, when
//!   nothing is copied.
//! - **A copy named otherwise** keeps its camera name in its `.apxmp`
//!   ([`renamed_sidecar`]).

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::copy::{CopyError, DATA_SUFFIX, PhotoFiles};

/// What a photo's sidecars add to its main file's name, in the order the copy lands them.
pub(crate) const SIDECARS: [&str; 3] = [
    crate::sidecar::SUFFIX,
    DATA_SUFFIX,
    crate::darktable::SUFFIX,
];

/// `main` numbered `n`, `_01` and on before its extension, as darktable's import continues a
/// name taken; itself for 0.
pub(crate) fn numbered(main: &str, n: u32) -> String {
    if n == 0 {
        return main.to_owned();
    }
    match main.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => format!("{stem}_{n:02}.{extension}"),
        _ => format!("{main}_{n:02}"),
    }
}

/// Every name a photo whose main file is `main` takes: the main file's, then its
/// sidecars'.
pub(crate) fn names(main: &str) -> impl Iterator<Item = String> + '_ {
    std::iter::once(main.to_owned())
        .chain(SIDECARS.iter().map(move |suffix| format!("{main}{suffix}")))
}

/// Whether `path` names anything, a file, a folder or a link that leads nowhere; an error
/// where the system cannot say, so nothing is ever written over what may be there.
pub(crate) fn there(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

/// Where a photo lands in its folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Place {
    /// Under this main file's name, none of its names taken.
    Free(String),
    /// Its copy is there under this name: the main file there is the same.
    There(String),
}

/// The name the photo whose main file is `main` lands under in `folder`: its own, or the
/// first `_01` name none of whose files is there or `claimed`, by the photos before it in
/// the same import; or the name, its own or one of its `_01` names on the way, whose main
/// file there `same` says is the photo's. A name the system cannot say is there or not, a
/// folder it may not search or a name too long, is [`CopyError::Write`]: the photo's
/// failure, never a reason to try the next name.
pub(crate) fn place(
    folder: &Path,
    main: &str,
    claimed: &HashSet<PathBuf>,
    same: &mut dyn FnMut(&Path) -> Result<bool, CopyError>,
) -> Result<Place, CopyError> {
    for n in 0.. {
        let name = numbered(main, n);
        let path = folder.join(&name);
        if !claimed.contains(&path) && path.is_file() && same(&path)? {
            return Ok(Place::There(name));
        }
        let mut taken = false;
        for name in names(&name) {
            let path = folder.join(name);
            let lookup = |source| CopyError::Write {
                file: path.clone(),
                source,
            };
            if claimed.contains(&path) || there(&path).map_err(lookup)? {
                taken = true;
                break;
            }
        }
        if !taken {
            return Ok(Place::Free(name));
        }
    }
    unreachable!("a folder holds fewer files than u32 numbers")
}

/// The `.apxmp` a copy of `source` named `name` writes in place of the source's, where it is
/// named otherwise than its camera name: the source's with `xmpMM:PreservedFileName` and
/// naming its new photo, or a new one holding the name alone ([`crate::sidecar::preserving`]).
/// `None` for a copy under its own name, which copies its sidecar as it is, and for a
/// source's that is no XMP, copied as it is too, logged. An error where the source's names
/// its photo otherwise than as plain text, so it opens for viewing only: the photo's
/// failure, its sidecar never rewritten without that field.
pub(crate) fn renamed_sidecar(
    source: &PhotoFiles,
    name: &str,
    build: &str,
) -> io::Result<Option<Vec<u8>>> {
    if name == source.main {
        return Ok(None);
    }
    let own = format!("{}{}", source.main, crate::sidecar::SUFFIX);
    let xml = if source.companions.contains(&own) {
        Some(fs::read(source.folder.join(&own))?)
    } else {
        None
    };
    match crate::sidecar::preserving(xml.as_deref(), &source.main, name, build) {
        Ok(Some(xml)) => Ok(Some(xml.into_bytes())),
        Ok(None) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: its apertrust:photo is no plain text, so it opens for viewing only",
                source.folder.join(&own).display()
            ),
        )),
        Err(error) => {
            let path = source.folder.join(&own);
            tracing::warn!(
                "{}: no XMP, copied as it is, without the camera name: {error}",
                path.display()
            );
            Ok(None)
        }
    }
}

/// What of the source photo's sidecars is beside its copy there under `name`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Beside {
    /// The source's companions missing beside it, to copy under the copy's name; and
    /// whether the `.apxmp` written in place of the source's, `sidecar`, is missing.
    pub missing: Vec<String>,
    pub sidecar_missing: bool,
    /// The sidecar there that differs from what the copy would write: a diverged edit.
    pub diverged: Option<PathBuf>,
}

/// The source photo's sidecars beside its copy under `name` in `folder`: each the source
/// has, or the `.apxmp` it writes in place of the source's, `sidecar`, missing there or the
/// same, or one there that differs, a diverged edit. A sidecar there that the source has
/// none of is the copy's own, and stays.
pub(crate) fn beside(
    source: &PhotoFiles,
    folder: &Path,
    name: &str,
    sidecar: Option<&[u8]>,
) -> io::Result<Beside> {
    let mut found = Beside::default();
    for suffix in SIDECARS {
        let own = format!("{}{suffix}", source.main);
        let theirs = folder.join(format!("{name}{suffix}"));
        let has = source.companions.contains(&own);
        let written = (suffix == crate::sidecar::SUFFIX)
            .then_some(sidecar)
            .flatten();
        if !has && written.is_none() {
            continue;
        }
        if !there(&theirs)? {
            match written {
                Some(_) => found.sidecar_missing = true,
                None => found.missing.push(own),
            }
            continue;
        }
        // A new `.apxmp` holding the camera name alone is never compared with the copy's
        // own edit there.
        if !has {
            continue;
        }
        let expected = match written {
            Some(bytes) => bytes.to_vec(),
            None => fs::read(source.folder.join(&own))?,
        };
        if fs::read(&theirs)? != expected {
            found.diverged = Some(theirs);
            return Ok(found);
        }
    }
    Ok(found)
}
