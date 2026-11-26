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

//! The checked copy (decisions 85 and 101): one photo's files copied from one folder to
//! another, whole or not at all, never replacing a file, and optionally read back and
//! checked against the source. Import and Move to…/Copy to… stand on it.
//!
//! [`copy_photo`] copies a photo's [`PhotoFiles`], its sidecars first and its main file
//! last, so a photo that has landed is never a raw without its sidecar. Each file:
//!
//! 1. is streamed from the source, opened read-only and never written, in chunks of
//!    [`CHUNK`], hashed with BLAKE3 as it is read, so it is read once;
//! 2. into a hidden temporary in the destination folder, `.~NAME.XXXXXX.tmp`, the safe
//!    write's ([`apertrust_base::fs::create`]);
//! 3. which, with [`Check::ReadBack`], is synced, read back past the system's cache
//!    ([`apertrust_base::fs::read_uncached`]) and compared with the source's hash, then
//!    given the source's modified time and synced: until then it is as young as its write,
//!    so the listing never takes it for one a crash left;
//! 4. then renamed into place by a rename that refuses to replace a file
//!    ([`persist_noclobber`], on Windows `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`;
//!    on a share with no such rename, as macOS's SMB client, a look just before a plain
//!    rename), tried again where Windows refuses it for a moment ([`persist_retrying`]).
//!
//! Any failure, a cancel included, removes the temporary and the photo's files that had
//! landed, so the destination holds the whole photo or nothing of it; a file that was there
//! before is never touched.
//!
//! A crash is not a failure the copy sees. It can leave the temporary, hidden and told by
//! its name ([`apertrust_base::fs::is_temporary_name`]), which the listing removes as it
//! lists the folder a day after it was last written ([`crate::listing`]); and, once some of
//! the photo's files had landed, those files under their names, its sidecars without its
//! main file, which a copy of the photo again into that folder meets as
//! [`CopyError::Exists`]. They are copies whose source is still there: remove them and
//! copy again, or copy into another folder.
//!
//! The function always copies, on one volume too: Move to…'s rename there is its own.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use apertrust_base::fs::{Retry, persist_noclobber, persist_retrying};

/// What the binary sidecar's name adds to its photo's (decision 49).
pub const DATA_SUFFIX: &str = ".apdata";

/// The most bytes the copy reads, writes or reads back at a time: the memory it holds,
/// whatever a file's size, and how often it asks its cancel.
pub const CHUNK: usize = 1 << 20;

/// One photo's files in one folder: its main file and the files that go with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotoFiles {
    pub folder: PathBuf,
    /// The main file's name: the raw, or the photo's only image file.
    pub main: String,
    /// The names of the files that go with it, copied before it, in this order.
    pub companions: Vec<String>,
}

impl PhotoFiles {
    /// The photo whose main file is `main` in `folder`, with the files beside it that carry
    /// its whole name, as the listing pairs them, byte for byte: its `.apxmp`, `.apdata`
    /// and darktable's `.xmp`, each where it is a plain file.
    pub fn find(folder: &Path, main: &str) -> io::Result<Self> {
        let mut companions = Vec::new();
        for suffix in [
            crate::sidecar::SUFFIX,
            DATA_SUFFIX,
            crate::darktable::SUFFIX,
        ] {
            let name = format!("{main}{suffix}");
            match fs::symlink_metadata(folder.join(&name)) {
                Ok(meta) if meta.is_file() => companions.push(name),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(Self {
            folder: folder.to_owned(),
            main: main.to_owned(),
            companions,
        })
    }

    /// Every name, in the order they are copied: the companions, then the main file.
    fn names(&self) -> impl Iterator<Item = &str> {
        self.companions
            .iter()
            .map(String::as_str)
            .chain([self.main.as_str()])
    }
}

/// Whether each copy is read back before it is renamed into place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// Renamed once synced.
    None,
    /// Read back past the system's cache and compared with the source's hash.
    ReadBack,
}

/// A file that landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedFile {
    pub path: PathBuf,
    pub len: u64,
    /// The source's BLAKE3 hash, which the copy holds too.
    pub hash: blake3::Hash,
}

/// What [`copy_photo`] copied: each file, in the order they landed, the main file last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copied {
    pub files: Vec<CopiedFile>,
}

/// Why a photo was not copied, naming the file. Nothing of the photo is at the destination
/// then, but the files that were there before, and a file of the photo that landed and
/// could not be removed again, which is logged as a warning and stays.
#[derive(Debug, thiserror::Error)]
pub enum CopyError {
    /// Cancelled before or while `file`, the source's, was copied.
    #[error("{}: the copy was cancelled", file.display())]
    Cancelled { file: PathBuf },
    /// `file` is at the destination already, or appeared there during the copy: never
    /// replaced.
    #[error("{}: a file of that name is there already", file.display())]
    Exists { file: PathBuf },
    /// The source's `file` could not be read: a card pulled out, say.
    #[error("cannot read {}: {source}", file.display())]
    Read { file: PathBuf, source: io::Error },
    /// The destination's `file` could not be written, synced or read back: a full disk,
    /// say.
    #[error("cannot write {}: {source}", file.display())]
    Write { file: PathBuf, source: io::Error },
    /// The copy of `file`, the destination's, read back, differs from its source.
    #[error("{}: the copy read back differs from its source", file.display())]
    Mismatch { file: PathBuf },
}

impl CopyError {
    /// The file it names: the source's for a cancel or a read, else the destination's.
    pub fn file(&self) -> &Path {
        match self {
            Self::Cancelled { file }
            | Self::Exists { file }
            | Self::Read { file, .. }
            | Self::Write { file, .. }
            | Self::Mismatch { file } => file,
        }
    }
}

/// Copies the photo `source` into the folder `into`, under the same names, whole or not at
/// all (the module's steps). `into` is made, with its parents, as the first file is about
/// to land; removing a folder the copy made and left empty is the caller's. `cancel` is
/// asked before each file and each chunk; `progress` is given the bytes of each chunk
/// copied. A blocking call, made by a job's body.
pub fn copy_photo(
    source: &PhotoFiles,
    into: &Path,
    check: Check,
    cancel: &(dyn Fn() -> bool + Sync),
    progress: &mut dyn FnMut(u64),
) -> Result<Copied, CopyError> {
    copy_with(
        source,
        into,
        check,
        CHUNK,
        cancel,
        progress,
        &mut |_| Ok(()),
    )
}

/// Where the copy calls a test's hook, which fails the step with its error: before each
/// chunk is read and written, and once the temporary holds the bytes (synced, where it is
/// read back), before its read-back and its rename.
#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
enum At<'a> {
    Read {
        file: usize,
    },
    Write {
        file: usize,
    },
    Written {
        file: usize,
        temp: &'a Path,
        path: &'a Path,
    },
}

type Hook<'h> = dyn FnMut(At<'_>) -> io::Result<()> + 'h;

/// [`copy_photo`] in chunks of `chunk`, with a test's hook.
fn copy_with(
    source: &PhotoFiles,
    into: &Path,
    check: Check,
    chunk: usize,
    cancel: &(dyn Fn() -> bool + Sync),
    progress: &mut dyn FnMut(u64),
    hook: &mut Hook<'_>,
) -> Result<Copied, CopyError> {
    let mut files = Vec::new();
    for (index, name) in source.names().enumerate() {
        let file = Part {
            index,
            from: source.folder.join(name),
            to: into.join(name),
        };
        let copied = if cancel() {
            Err(CopyError::Cancelled { file: file.from })
        } else {
            copy_file(&file, into, check, chunk, cancel, progress, hook)
        };
        match copied {
            Ok(copied) => files.push(copied),
            Err(error) => {
                remove_landed(&files);
                return Err(error);
            }
        }
    }
    Ok(Copied { files })
}

/// One of a photo's files: its place in the copy, its source and its destination.
struct Part {
    index: usize,
    from: PathBuf,
    to: PathBuf,
}

/// Removes the files of a photo that landed before one of its files failed: the copy's
/// own, each made by a rename that replaced nothing.
fn remove_landed(files: &[CopiedFile]) {
    for file in files {
        if let Err(error) = fs::remove_file(&file.path) {
            let path = file.path.display();
            tracing::warn!("{path}: copied, then not removed as its photo failed: {error}");
        }
    }
}

/// Copies one file, the module's steps.
fn copy_file(
    file: &Part,
    into: &Path,
    check: Check,
    chunk: usize,
    cancel: &(dyn Fn() -> bool + Sync),
    progress: &mut dyn FnMut(u64),
    hook: &mut Hook<'_>,
) -> Result<CopiedFile, CopyError> {
    let read = |source| CopyError::Read {
        file: file.from.clone(),
        source,
    };
    let write = |source| CopyError::Write {
        file: file.to.clone(),
        source,
    };
    let cancelled = || CopyError::Cancelled {
        file: file.from.clone(),
    };
    if fs::symlink_metadata(&file.to).is_ok() {
        return Err(CopyError::Exists {
            file: file.to.clone(),
        });
    }
    let mut from = fs::File::open(&file.from).map_err(read)?;
    let modified = from.metadata().and_then(|m| m.modified()).map_err(read)?;
    let name = file.to.file_name().map(|n| n.to_string_lossy());
    let name = name.ok_or_else(|| read(io::ErrorKind::InvalidInput.into()))?;
    fs::create_dir_all(into).map_err(write)?;
    // Dropped on any error below, which removes it.
    let mut temp = apertrust_base::fs::create(into, &name).map_err(write)?;
    if check == Check::ReadBack {
        apertrust_base::fs::uncache_writes(temp.as_file()).map_err(write)?;
    }
    let mut buffer = vec![0u8; chunk];
    let mut hasher = blake3::Hasher::new();
    let mut len = 0u64;
    loop {
        if cancel() {
            return Err(cancelled());
        }
        hook(At::Read { file: file.index }).map_err(read)?;
        let got = match from.read(&mut buffer) {
            Ok(0) => break,
            Ok(got) => got,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(read(error)),
        };
        let bytes = &buffer[..got];
        hasher.update(bytes);
        hook(At::Write { file: file.index }).map_err(write)?;
        temp.write_all(bytes).map_err(write)?;
        len += got as u64;
        progress(got as u64);
    }
    drop(buffer);
    let hash = hasher.finalize();
    if check == Check::ReadBack {
        apertrust_base::fs::sync(temp.as_file()).map_err(write)?;
    }
    hook(At::Written {
        file: file.index,
        temp: temp.path(),
        path: &file.to,
    })
    .map_err(write)?;
    if check == Check::ReadBack {
        let mut back = blake3::Hasher::new();
        let read_back = apertrust_base::fs::read_uncached(temp.path(), chunk, |bytes| {
            if cancel() {
                return Err(io::Error::other(Cancelled));
            }
            back.update(bytes);
            Ok(())
        });
        match read_back {
            Ok(_) => {}
            Err(error) if error.get_ref().is_some_and(|e| e.is::<Cancelled>()) => {
                return Err(cancelled());
            }
            Err(error) => return Err(write(error)),
        }
        if back.finalize() != hash {
            return Err(CopyError::Mismatch {
                file: file.to.clone(),
            });
        }
    }
    // The source's time only now: until here the temporary is as young as its write, so a
    // listing of the folder meanwhile never takes it for one a crash left a day ago.
    temp.as_file().set_modified(modified).map_err(write)?;
    apertrust_base::fs::sync(temp.as_file()).map_err(write)?;
    persist_retrying(temp, &file.to, Retry::platform(), persist_noclobber).map_err(|failed| {
        match failed.error.kind() {
            io::ErrorKind::AlreadyExists => CopyError::Exists {
                file: file.to.clone(),
            },
            _ => write(failed.error),
        }
    })?;
    Ok(CopiedFile {
        path: file.to.clone(),
        len,
        hash,
    })
}

/// The read-back's error where its cancel answers true.
#[derive(Debug)]
struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

#[cfg(test)]
mod tests;
