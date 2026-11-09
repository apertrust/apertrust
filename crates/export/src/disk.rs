// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2026  darktable developers (ported from src/imageio/storage/disk.c)
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

//! File on disk (decision 56; `docs/engine.md`, Export jobs, Writing the file): where an
//! export's files go and how they get there, as darktable's disk storage
//! (`src/imageio/storage/disk.c`, `store`) names and writes them, with the one change
//! decision 56 makes: a file is written beside its name and renamed into place, so a NAS
//! or a cloud folder only ever holds a complete file under a photo's name.
//!
//! A [`Disk`] is made once for an export, from the path pattern, On conflict and the
//! number of photos. For each photo, [`Disk::prepare`] expands the pattern, creates the
//! folder, applies On conflict and either says to [skip](Prepared::Skip) the photo, before
//! anything is rendered, or hands back a [`Target`]: a temporary file in the photo's
//! folder, which proves the folder can be written. The caller writes the file's bytes into
//! it and calls [`Target::finish`], which syncs it and renames it into place. A target
//! dropped before that leaves nothing behind, a failed write included.
//!
//! The rename never replaces a file for Create unique filename and Skip, so a name taken
//! between [`Disk::prepare`] and [`Target::finish`], by another program or another
//! export, takes the next free number or skips the photo; for Overwrite and Overwrite if
//! changed it replaces. Where the file system has no rename that refuses to replace (FAT,
//! some SMB, CIFS and NFS mounts answer "not supported" or "invalid"), [`Target::finish`]
//! checks for the name and then renames, which is darktable's own window.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use apertrust_base::fs::{Retry, persist_retrying};
use tempfile::{NamedTempFile, PersistError};
use thiserror::Error;

use crate::settings::OnConflict;
use crate::variables::{Pattern, PatternError, Values};

/// An export's path pattern and On conflict.
#[derive(Debug, Clone)]
pub struct Disk {
    /// The pattern, with `_$(SEQUENCE)` added where several photos would otherwise share
    /// a name, and a leading `~` the home folder.
    pattern: Pattern,
    /// The pattern as the person wrote it with `/$(FILE_NAME)` after it, for a pattern
    /// that expands to a folder.
    folder: Pattern,
    on_conflict: OnConflict,
    /// Whether `_$(SEQUENCE)` was added to the pattern.
    sequence_added: bool,
}

/// What stops a photo's file, or, where [`Error::destination`] says so, every photo's.
#[derive(Debug, Error)]
pub enum Error {
    /// An empty path (D8).
    #[error("the path is empty")]
    Empty,
    #[error(transparent)]
    Pattern(#[from] PatternError),
    /// A pattern that expands to nothing, or still to a folder after `$(FILE_NAME)`
    /// follows it: a photo's name is empty.
    #[error("the path expands to no file name: \"{0}\"")]
    NoName(String),
    #[error("could not create the folder {}: {source}", path.display())]
    CreateFolder { path: PathBuf, source: io::Error },
    #[error("could not write to the folder {}: {source}", path.display())]
    WriteFolder { path: PathBuf, source: io::Error },
    /// The file could not be synced or renamed into place, or was refused its name.
    #[error("could not write {}: {source}", path.display())]
    Write { path: PathBuf, source: io::Error },
}

impl Error {
    /// Whether the destination as a whole is at fault, a folder that cannot be created
    /// or written, a disk full, read-only or gone, so that every later photo of the
    /// export would fail the same way and the export stops, as darktable's export job
    /// stops at a failed store; otherwise the error is this photo's.
    pub fn destination(&self) -> bool {
        match self {
            Self::CreateFolder { .. } | Self::WriteFolder { .. } => true,
            Self::Write { source, .. } => destination_kind(source.kind()),
            Self::Empty | Self::Pattern(_) | Self::NoName(_) => false,
        }
    }
}

/// Whether a failed write of that kind is the destination's, not the photo's: the disk
/// full, over quota, read-only or the folder gone. It is [`Error::destination`]'s test for
/// a sync or a rename, and [`crate::Error::destination`]'s for the encoder's own writes.
pub(crate) fn destination_kind(kind: io::ErrorKind) -> bool {
    use io::ErrorKind::{NotFound, QuotaExceeded, ReadOnlyFilesystem, StorageFull};
    matches!(
        kind,
        StorageFull | QuotaExceeded | ReadOnlyFilesystem | NotFound
    )
}

/// Why a photo is not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The file is there and On conflict is Skip.
    Exists,
    /// Overwrite if changed, and the file is newer than the photo's edit.
    NotChanged,
}

/// A photo that is left out, and the file it would have been.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub path: PathBuf,
    pub why: Why,
}

/// What [`Disk::prepare`] decides for a photo.
#[derive(Debug)]
pub enum Prepared {
    /// Leave the photo out, before it is rendered.
    Skip(Skipped),
    Write(Target),
}

/// What [`Target::finish`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Stored {
    Written(PathBuf),
    /// Skip, and the name was taken after [`Disk::prepare`] looked.
    Skipped(Skipped),
}

impl Disk {
    /// The storage for an export of `photos` photos to `pattern`, as File on disk's path
    /// holds it, with `on_conflict`. A leading `~` is `home`, as darktable's
    /// `dt_util_fix_path` has it, but for `~name`, another user's, which stays as it is.
    /// An empty path, an unknown variable and a variable not closed are refused here, before
    /// any photo (D8).
    pub fn new(
        pattern: &str,
        on_conflict: OnConflict,
        photos: usize,
        home: Option<&Path>,
    ) -> Result<Self, Error> {
        if pattern.is_empty() {
            return Err(Error::Empty);
        }
        // Several photos would overwrite each other at random: darktable's own words.
        let mut numbered = pattern.to_owned();
        let sequence_added = photos > 1 && !pattern.contains('$');
        if sequence_added {
            numbered.push_str("_$(SEQUENCE)");
        }
        Ok(Self {
            pattern: Pattern::parse(&home_for_tilde(&numbered, home))?,
            // After the original pattern, darktable's `d->filename`, whatever was added.
            // Always `/`, which Windows takes too and which `\` would escape.
            folder: Pattern::parse(&home_for_tilde(&format!("{pattern}/$(FILE_NAME)"), home))?,
            on_conflict,
            sequence_added,
        })
    }

    /// Whether `_$(SEQUENCE)` was added to the pattern: several photos, and no variable in
    /// it to tell their names apart, darktable's rule.
    pub fn sequence_added(&self) -> bool {
        self.sequence_added
    }

    /// The pattern as it expands, with `_$(SEQUENCE)` added where it was: for the dialog's
    /// line of variables with no value here (D7).
    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    /// The file `values`' photo is written to before On conflict looks, with
    /// `extension` after a dot: the pattern expanded, `$(FILE_NAME)` after it where it
    /// expands to a folder. Nothing is created or looked at on disk.
    pub fn name(&self, values: &Values, extension: &str) -> Result<PathBuf, Error> {
        Ok(PathBuf::from(format!(
            "{}.{extension}",
            self.expand(values)?
        )))
    }

    fn expand(&self, values: &Values) -> Result<String, Error> {
        let ends_in_separator = |text: &str| text.ends_with(['/', '\\']);
        let mut name = self.pattern.expand(values);
        if ends_in_separator(&name) {
            name = self.folder.expand(values);
        }
        if name.is_empty() || ends_in_separator(&name) {
            return Err(Error::NoName(name));
        }
        Ok(name)
    }

    /// Makes the photo's folder, applies On conflict and gives the file to write, or says
    /// to skip the photo:
    ///
    /// - Create unique filename: the first of `NAME.EXT`, `NAME_01.EXT`, `NAME_02.EXT` and
    ///   on that is free;
    /// - Overwrite: the name as it is;
    /// - Overwrite if changed: the name as it is, unless a file there is newer than
    ///   `changed`, the photo's last edit, taken as it is now and to the whole second as
    ///   the file's time is, where `None` is a photo never edited, whose file is kept;
    /// - Skip: the name as it is, unless a file is there.
    ///
    /// The folder is made with its parents, and an error here, [`Error::destination`], is
    /// the export's, not the photo's.
    pub fn prepare(
        &self,
        values: &Values,
        extension: &str,
        changed: Option<SystemTime>,
    ) -> Result<Prepared, Error> {
        let base = self.expand(values)?;
        let folder = folder_of(&base, extension);
        fs::create_dir_all(&folder).map_err(|source| Error::CreateFolder {
            path: folder.clone(),
            source,
        })?;
        let (number, why) = self.resolve(
            &base,
            extension,
            |path| path.exists(),
            |path| older_than(path, changed),
        );
        let mut target = Target {
            // Not yet made: the folder's check comes after On conflict's skips.
            temp: None,
            base,
            extension: extension.to_owned(),
            number,
            on_conflict: self.on_conflict,
        };
        if let Some(why) = why {
            return Ok(Prepared::Skip(Skipped {
                path: target.path(),
                why,
            }));
        }
        // A file in the photo's folder, where it can be written, as darktable's `g_access`
        // asks; named after the file it is to become, so a stray one is told by its name
        // (`apertrust_base::fs::create`).
        let name = target.path();
        target.temp = Some(
            apertrust_base::fs::create(&folder, file_name(&name).unwrap_or_default()).map_err(
                |source| Error::WriteFolder {
                    path: folder,
                    source,
                },
            )?,
        );
        Ok(Prepared::Write(target))
    }

    /// What [`prepare`](Self::prepare) would do with the photo, looked at without writing
    /// anything: no folder made, no file created, only the names tested. `claimed` holds
    /// the files the photos before this one have taken, which exist for it as they will
    /// once they are written, and are newer than any edit.
    pub fn foresee(
        &self,
        values: &Values,
        extension: &str,
        changed: Option<SystemTime>,
        claimed: &Claimed,
    ) -> Result<Foreseen, Error> {
        let base = self.expand(values)?;
        let plain = numbered(&base, extension, 0);
        let taken = |path: &Path| claimed.contains(path) || path.exists();
        let (number, why) = self.resolve(&base, extension, taken, |path| {
            !claimed.contains(path) && older_than(path, changed)
        });
        let path = numbered(&base, extension, number);
        let exists = taken(&plain);
        let (path, action) = match (why, self.on_conflict) {
            (Some(why), _) => (path, Action::Skip(why)),
            (None, OnConflict::CreateUniqueFilename) if number > 0 => (path, Action::Number),
            (None, OnConflict::Overwrite | OnConflict::OverwriteIfChanged) if exists => {
                (path, Action::Overwrite)
            }
            (None, _) => (path, Action::Write),
        };
        Ok(Foreseen {
            path,
            exists,
            action,
        })
    }

    /// On conflict's rule for the name `base` and `extension` make: the number the file
    /// takes, from 1, or 0 for none, and why it is left out, if it is. `taken` says whether
    /// a file is there and `older` whether it is not newer than the photo's edit.
    fn resolve(
        &self,
        base: &str,
        extension: &str,
        taken: impl Fn(&Path) -> bool,
        older: impl Fn(&Path) -> bool,
    ) -> (u32, Option<Why>) {
        let plain = numbered(base, extension, 0);
        match self.on_conflict {
            OnConflict::CreateUniqueFilename => {
                let mut number = 0;
                while taken(&numbered(base, extension, number)) {
                    number += 1;
                }
                (number, None)
            }
            OnConflict::Overwrite => (0, None),
            OnConflict::OverwriteIfChanged => {
                let skip = taken(&plain) && !older(&plain);
                (0, skip.then_some(Why::NotChanged))
            }
            OnConflict::Skip => (0, taken(&plain).then_some(Why::Exists)),
        }
    }
}

/// The files the photos of a preview have taken so far ([`Disk::foresee`]).
///
/// Names are compared as the file system does when it can say: case-folded on macOS and
/// Windows, whose volumes are case-insensitive by default, so that two photos whose files
/// differ only in case are one file there, as the export's `exists()` sees them, and
/// exact elsewhere. It is chosen by platform, not asked of the volume, which may be the
/// other kind (a case-sensitive APFS volume, a case-insensitive ext4 directory); the
/// preview is advice, and the export decides on what is actually there. Unicode
/// normalisation, which APFS also ignores, is not folded.
#[derive(Debug, Clone)]
pub struct Claimed {
    names: BTreeSet<PathBuf>,
    case_insensitive: bool,
}

impl Default for Claimed {
    fn default() -> Self {
        Self::new(cfg!(any(target_os = "macos", target_os = "windows")))
    }
}

impl Claimed {
    /// An empty set that folds case or does not, whatever the platform.
    pub fn new(case_insensitive: bool) -> Self {
        Self {
            names: BTreeSet::new(),
            case_insensitive,
        }
    }

    /// Notes that a photo's file takes `path`.
    pub fn insert(&mut self, path: &Path) {
        let key = self.key(path);
        self.names.insert(key);
    }

    /// Whether an earlier photo's file takes `path`.
    pub fn contains(&self, path: &Path) -> bool {
        self.names.contains(&self.key(path))
    }

    fn key(&self, path: &Path) -> PathBuf {
        if self.case_insensitive {
            PathBuf::from(path.to_string_lossy().to_lowercase())
        } else {
            path.to_owned()
        }
    }
}

/// What On conflict would do with a photo's file ([`Disk::foresee`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing is there: the file is written under its name.
    Write,
    /// Create unique filename, and the name is taken: the file takes a number, `_01`.
    Number,
    /// Overwrite, or Overwrite if changed with the photo changed: the file there is
    /// replaced.
    Overwrite,
    /// The photo is left out.
    Skip(Why),
}

/// A photo's file as [`Disk::foresee`] sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Foreseen {
    /// Where the file goes, its number included.
    pub path: PathBuf,
    /// Whether a file is already under the name the pattern gives, before any number.
    pub exists: bool,
    pub action: Action,
}

/// `base` and `extension` as a name, with `_NN` before the extension from number 1.
fn numbered(base: &str, extension: &str, number: u32) -> PathBuf {
    if number == 0 {
        PathBuf::from(format!("{base}.{extension}"))
    } else {
        PathBuf::from(format!("{base}_{number:02}.{extension}"))
    }
}

/// The photo's file, written beside its name until [`finish`](Self::finish) renames it into
/// place. Dropped, it removes the temporary file.
#[derive(Debug)]
pub struct Target {
    /// `None` only inside [`Disk::prepare`].
    temp: Option<NamedTempFile>,
    /// The name as the pattern gives it, without the extension.
    base: String,
    extension: String,
    /// The number the name has now, from 1, or 0 for none: `NAME_02.EXT` is 2.
    number: u32,
    on_conflict: OnConflict,
}

impl Target {
    /// The name the file will have if it is renamed now: [`finish`](Self::finish) takes
    /// the next free one where a unique name was taken meanwhile.
    pub fn path(&self) -> PathBuf {
        numbered(&self.base, &self.extension, self.number)
    }

    /// The temporary file, to write the bytes into; flush any buffer before
    /// [`finish`](Self::finish).
    pub fn file(&mut self) -> &mut File {
        self.temp
            .as_mut()
            .expect("a prepared target has its file")
            .as_file_mut()
    }

    /// Syncs the file and renames it into place, as [`Disk::prepare`]'s On conflict says
    /// (the module's documentation). On an error the temporary file is removed and nothing
    /// is under the photo's name that was not there before.
    pub fn finish(self) -> Result<Stored, Error> {
        self.finish_with(|temp, path| temp.persist_noclobber(path))
    }

    /// [`finish`](Self::finish) with the rename that never replaces given, so that a test
    /// can stand in for a file system that refuses it.
    fn finish_with(
        mut self,
        noclobber: impl Fn(NamedTempFile, &Path) -> Result<File, PersistError>,
    ) -> Result<Stored, Error> {
        let mut temp = self.temp.take().expect("a prepared target has its file");
        let failed = |path: &Path, source| Error::Write {
            path: path.to_owned(),
            source,
        };
        temp.as_file()
            .sync_all()
            .map_err(|source| failed(&self.path(), source))?;
        let replaces = matches!(
            self.on_conflict,
            OnConflict::Overwrite | OnConflict::OverwriteIfChanged
        );
        // Set once the file system has refused the rename that never replaces.
        let mut checks = false;
        // Each rename is tried again where Windows refuses it for a moment.
        let retry = Retry::platform();
        loop {
            let path = self.path();
            let renamed = if replaces {
                persist_retrying(temp, &path, retry, |temp, path| temp.persist(path))
            } else if checks {
                // Anything under the name, a dangling link included, is taken.
                if fs::symlink_metadata(&path).is_ok() {
                    Err(PersistError {
                        error: io::ErrorKind::AlreadyExists.into(),
                        file: temp,
                    })
                } else {
                    persist_retrying(temp, &path, retry, |temp, path| temp.persist(path))
                }
            } else {
                persist_retrying(temp, &path, retry, &noclobber)
            };
            let PersistError { error, file } = match renamed {
                Ok(_) => return Ok(Stored::Written(path)),
                Err(error) => error,
            };
            temp = file;
            match (error.kind(), self.on_conflict) {
                (io::ErrorKind::AlreadyExists, OnConflict::CreateUniqueFilename) => {
                    // Taken since `prepare` looked: the next free one.
                    self.number += 1;
                    while self.path().exists() {
                        self.number += 1;
                    }
                }
                (io::ErrorKind::AlreadyExists, OnConflict::Skip) => {
                    return Ok(Stored::Skipped(Skipped {
                        path,
                        why: Why::Exists,
                    }));
                }
                // FAT, and some network mounts, have no rename that refuses to replace
                // ("not supported", "invalid", or a refused hard link): look, then
                // rename, as darktable does, with a window between. If the file system
                // refuses the plain rename too, that error is the photo's.
                _ if !replaces && !checks => {
                    tracing::debug!(
                        target: "pipe",
                        "{}: no rename that keeps a file ({error}), checking the name first",
                        file_name(&path).unwrap_or_default()
                    );
                    checks = true;
                }
                _ => return Err(failed(&path, error)),
            }
        }
    }
}

/// Whether the file at `path` is not newer than `changed`: darktable's test that the
/// file's time is later than the change's, turned round, where a file whose time cannot be
/// read counts as 0, and a photo never changed as 0 too.
fn older_than(path: &Path, changed: Option<SystemTime>) -> bool {
    let Some(modified) = fs::metadata(path).ok().and_then(|m| m.modified().ok()) else {
        return true;
    };
    // glib reads the time in whole seconds.
    let whole = modified
        .duration_since(UNIX_EPOCH)
        .map_or(modified, |since| {
            UNIX_EPOCH + Duration::from_secs(since.as_secs())
        });
    changed.is_some_and(|changed| whole <= changed)
}

/// The folder of the file `base` and `extension` name: glib's `g_path_get_dirname`, with
/// `.` for a name with no folder.
fn folder_of(base: &str, extension: &str) -> PathBuf {
    let path = PathBuf::from(format!("{base}.{extension}"));
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_owned(),
        _ => PathBuf::from("."),
    }
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}

/// `text` with a leading `~` the home folder, as `dt_util_fix_path`: `~` alone or `~/…`
/// (and `~\…` on Windows); `~name` and a system with no home are left as they are. Public
/// for the folder an export job names as its destination, read as [`Disk::new`] reads it.
pub fn home_for_tilde(text: &str, home: Option<&Path>) -> String {
    let (Some(rest), Some(home)) = (text.strip_prefix('~'), home) else {
        return text.to_owned();
    };
    let separator = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    if !rest.is_empty() && !rest.starts_with(separator) {
        return text.to_owned();
    }
    // Forward slashes, which a pattern reads as it reads any other: a `\` would escape.
    let home = home.to_string_lossy().replace('\\', "/");
    format!("{}{rest}", home.trim_end_matches('/'))
}

/// Whether an export's photos can go to a folder, asked without making it or writing
/// anything ([`preflight`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Preflight {
    /// The folder is there and may be written.
    Ready,
    /// The folder is missing and the export makes it, with its parents, in `from`, the
    /// nearest folder that is there.
    Creates { from: PathBuf },
    /// The export would fail at `at`.
    Blocked { at: PathBuf, why: Blocked },
}

/// Why a folder cannot be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    /// The folder is on a disk that is not connected: a volume under `/Volumes` on the
    /// Mac, a drive letter on Windows, a folder under `/media` or `/run/media` on Linux.
    NotConnected,
    /// A file is where a folder is wanted.
    NotAFolder,
    /// The folder has no write permission for anyone.
    ReadOnly,
    /// The system would not say, for this reason.
    Unreadable(io::ErrorKind),
}

/// Asks whether the folder `folder`, which an export makes if it is missing, can be
/// written: the nearest folder on its path that exists must be a folder and writable, and
/// the volume it is on must be connected. It looks at metadata only, so it creates
/// nothing, and it advises: it sees the permission bits, not an access control list or a
/// read-only mount, and the export reports what actually fails. A relative folder whose
/// first part is missing is made in the current folder, which is asked as any other, and a
/// link that leads nowhere on the way is a [`Blocked::NotAFolder`].
pub fn preflight(folder: &Path) -> Preflight {
    preflight_from(folder, Path::new("."))
}

/// [`preflight`], with `here` the folder a relative path starts in; a test gives another.
fn preflight_from(folder: &Path, here: &Path) -> Preflight {
    if let Some(volume) = volume_of(folder)
        && !volume.exists()
    {
        return Preflight::Blocked {
            at: volume,
            why: Blocked::NotConnected,
        };
    }
    let mut at = folder;
    let metadata = loop {
        match fs::metadata(at) {
            Ok(metadata) => break metadata,
            // A missing folder, or a file in the way, which its parent finds.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                // A link that leads nowhere is there, so the export's `create_dir_all`
                // finds the name taken and fails: it is not a folder to make.
                if fs::symlink_metadata(at).is_ok() {
                    return Preflight::Blocked {
                        at: at.to_owned(),
                        why: Blocked::NotAFolder,
                    };
                }
                match at.parent() {
                    Some(parent) if !parent.as_os_str().is_empty() => at = parent,
                    // A relative path whose first folder is missing starts in the
                    // current folder, which must be there and writable itself.
                    _ if at != here => at = here,
                    _ => {
                        return Preflight::Blocked {
                            at: at.to_owned(),
                            why: Blocked::Unreadable(error.kind()),
                        };
                    }
                }
            }
            Err(error) => {
                return Preflight::Blocked {
                    at: at.to_owned(),
                    why: Blocked::Unreadable(error.kind()),
                };
            }
        }
    };
    let blocked = |why| Preflight::Blocked {
        at: at.to_owned(),
        why,
    };
    if !metadata.is_dir() {
        return blocked(Blocked::NotAFolder);
    }
    if !writable(&metadata) {
        return blocked(Blocked::ReadOnly);
    }
    if at == folder {
        Preflight::Ready
    } else {
        Preflight::Creates {
            from: at.to_owned(),
        }
    }
}

/// Whether anyone may write the folder `metadata` describes: its write bits on Unix. On
/// Windows a folder's read-only attribute means something else, so it is not asked.
fn writable(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o222 != 0
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        true
    }
}

/// The removable volume `path` is on, where it names one by the place systems mount them:
/// `/Volumes/NAME` on the Mac, `/media/USER/NAME` and `/run/media/USER/NAME` on Linux, a
/// drive letter on Windows; `None` for a path anywhere else.
fn volume_of(path: &Path) -> Option<PathBuf> {
    use std::path::Component;
    #[cfg(windows)]
    if let Some(Component::Prefix(prefix)) = path.components().next() {
        let mut volume = PathBuf::from(prefix.as_os_str());
        volume.push(std::path::MAIN_SEPARATOR_STR);
        return Some(volume);
    }
    let mut components = path.components();
    if components.next()? != Component::RootDir {
        return None;
    }
    let names: Vec<_> = components
        .map_while(|component| match component {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();
    // How many names make the volume's folder, counted from the root.
    let depth = match names.as_slice() {
        [volumes, ..] if *volumes == "Volumes" => 2,
        [media, ..] if *media == "media" => 3,
        [run, media, ..] if *run == "run" && *media == "media" => 4,
        _ => return None,
    };
    (names.len() >= depth).then(|| {
        std::iter::once(Component::RootDir.as_os_str())
            .chain(names[..depth].iter().copied())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_tilde_is_the_home_folder() {
        let home = Some(Path::new("/home/ana"));
        let fixed = |text| home_for_tilde(text, home);
        assert_eq!(fixed("~"), "/home/ana");
        assert_eq!(fixed("~/out/$(FILE_NAME)"), "/home/ana/out/$(FILE_NAME)");
        // Another user's is left, and so is a tilde anywhere else.
        assert_eq!(fixed("~bob/out"), "~bob/out");
        assert_eq!(fixed("out/~/x"), "out/~/x");
        assert_eq!(home_for_tilde("~/out", None), "~/out");
        assert_eq!(
            home_for_tilde("~/out", Some(Path::new("/home/ana/"))),
            "/home/ana/out"
        );
    }

    #[test]
    fn the_mtime_in_whole_seconds_is_compared_with_the_change_time() {
        let at = |secs: u64, millis: u64| UNIX_EPOCH + Duration::from_millis(secs * 1000 + millis);
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join("file");
        let file = File::create(&path).expect("a file");
        file.set_modified(at(1000, 900)).expect("a time");
        // 1000.9 s reads as 1000 s: not newer than a change at 1000.5 s.
        assert!(older_than(&path, Some(at(1000, 500))));
        assert!(older_than(&path, Some(at(1000, 0))));
        assert!(!older_than(&path, Some(at(999, 999))));
        // A photo never changed keeps its file.
        assert!(!older_than(&path, None));
        // A file with no time, or no file, is exported again.
        assert!(older_than(&dir.path().join("missing"), None));
    }

    /// A target for a photo in `folder` under `on_conflict`, with `bytes` written.
    fn target(folder: &Path, on_conflict: OnConflict, bytes: &[u8]) -> Target {
        use std::io::Write;
        let pattern = format!(
            "{}/$(FILE_NAME)",
            folder.to_string_lossy().replace('\\', "/")
        );
        let disk = Disk::new(&pattern, on_conflict, 1, None).expect("a pattern");
        let values = Values {
            path: "/p/IMG.ARW".to_owned(),
            sequence: 1,
            ..Values::default()
        };
        let Prepared::Write(mut target) = disk.prepare(&values, "jpg", None).expect("prepared")
        else {
            panic!("a free name is written");
        };
        target.file().write_all(bytes).expect("written");
        target
    }

    /// A rename that never replaces, as FAT and some network mounts answer it: with an
    /// error of `kind`, and the file handed back.
    fn refused(kind: io::ErrorKind) -> impl Fn(NamedTempFile, &Path) -> Result<File, PersistError> {
        move |file, _| {
            Err(PersistError {
                error: kind.into(),
                file,
            })
        }
    }

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(folder)
            .expect("the folder")
            .map(|e| {
                e.expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    // These stand in for the file system with `finish_with`: the fallback's logic is
    // tested, not a FAT volume's answers, which only such a volume gives.
    #[test]
    fn a_rename_the_file_system_refuses_falls_back_to_checking_then_renaming() {
        let folder = tempfile::tempdir().expect("a folder");
        for kind in [
            io::ErrorKind::Unsupported,
            io::ErrorKind::InvalidInput,
            io::ErrorKind::PermissionDenied,
        ] {
            let stored = target(folder.path(), OnConflict::CreateUniqueFilename, b"ours")
                .finish_with(refused(kind))
                .expect("stored");
            assert_eq!(
                stored,
                Stored::Written(folder.path().join("IMG.jpg")),
                "{kind}"
            );
            fs::remove_file(folder.path().join("IMG.jpg")).expect("removed");
        }
    }

    #[test]
    fn a_unique_name_taken_when_no_rename_keeps_a_file_takes_the_next_free_one() {
        let folder = tempfile::tempdir().expect("a folder");
        let target = target(folder.path(), OnConflict::CreateUniqueFilename, b"ours");
        fs::write(folder.path().join("IMG.jpg"), b"theirs").expect("theirs");
        fs::write(folder.path().join("IMG_01.jpg"), b"theirs too").expect("theirs");
        let stored = target
            .finish_with(refused(io::ErrorKind::Unsupported))
            .expect("stored");
        assert_eq!(stored, Stored::Written(folder.path().join("IMG_02.jpg")));
        assert_eq!(
            fs::read(folder.path().join("IMG_02.jpg")).expect("ours"),
            b"ours"
        );
        assert_eq!(
            fs::read(folder.path().join("IMG.jpg")).expect("theirs"),
            b"theirs"
        );
        assert_eq!(
            names(folder.path()),
            ["IMG.jpg", "IMG_01.jpg", "IMG_02.jpg"]
        );
    }

    #[test]
    fn skip_when_no_rename_keeps_a_file_skips_a_name_that_is_taken_and_writes_a_free_one() {
        let folder = tempfile::tempdir().expect("a folder");
        let free = target(folder.path(), OnConflict::Skip, b"ours");
        let taken = target(folder.path(), OnConflict::Skip, b"ours too");
        let stored = free
            .finish_with(refused(io::ErrorKind::Unsupported))
            .expect("stored");
        assert_eq!(stored, Stored::Written(folder.path().join("IMG.jpg")));
        let stored = taken
            .finish_with(refused(io::ErrorKind::Unsupported))
            .expect("stored");
        assert!(matches!(
            stored,
            Stored::Skipped(Skipped {
                why: Why::Exists,
                ..
            })
        ));
        // The first photo's file is as it was written, and no temporary file remains.
        assert_eq!(
            fs::read(folder.path().join("IMG.jpg")).expect("ours"),
            b"ours"
        );
        assert_eq!(names(folder.path()), ["IMG.jpg"]);
    }

    #[test]
    fn overwrite_never_asks_for_the_rename_that_keeps_a_file() {
        let folder = tempfile::tempdir().expect("a folder");
        fs::write(folder.path().join("IMG.jpg"), b"old").expect("old");
        let target = target(folder.path(), OnConflict::Overwrite, b"new");
        let stored = target
            .finish_with(|_, _| panic!("Overwrite replaces"))
            .expect("stored");
        assert_eq!(stored, Stored::Written(folder.path().join("IMG.jpg")));
        assert_eq!(
            fs::read(folder.path().join("IMG.jpg")).expect("new"),
            b"new"
        );
    }

    /// A disk for `pattern` under `on_conflict`, one photo.
    fn disk(pattern: &Path, on_conflict: OnConflict) -> Disk {
        let pattern = format!(
            "{}/$(FILE_NAME)",
            pattern.to_string_lossy().replace('\\', "/")
        );
        Disk::new(&pattern, on_conflict, 1, None).expect("a pattern")
    }

    fn photo() -> Values {
        Values {
            path: "/p/IMG.ARW".to_owned(),
            sequence: 1,
            ..Values::default()
        }
    }

    const MODES: [OnConflict; 4] = [
        OnConflict::CreateUniqueFilename,
        OnConflict::Overwrite,
        OnConflict::OverwriteIfChanged,
        OnConflict::Skip,
    ];

    #[test]
    fn foresee_says_what_prepare_does_for_each_on_conflict() {
        let folder = tempfile::tempdir().expect("a folder");
        let now = SystemTime::now();
        let hour = Duration::from_secs(3600);
        for (taken, changed) in [
            (&[][..], None),
            (&["IMG.jpg"][..], None),
            (&["IMG.jpg", "IMG_01.jpg"][..], None),
            // The file is newer than the edit, and older.
            (&["IMG.jpg"][..], Some(now - hour)),
            (&["IMG.jpg"][..], Some(now + hour)),
        ] {
            for mode in MODES {
                for name in ["IMG.jpg", "IMG_01.jpg"] {
                    let _ = fs::remove_file(folder.path().join(name));
                }
                for name in taken {
                    fs::write(folder.path().join(name), b"there").expect("a file");
                }
                let disk = disk(folder.path(), mode);
                let foreseen = disk
                    .foresee(&photo(), "jpg", changed, &Claimed::default())
                    .expect("foreseen");
                let prepared = disk.prepare(&photo(), "jpg", changed).expect("prepared");
                match prepared {
                    Prepared::Skip(skipped) => {
                        assert_eq!(
                            foreseen.action,
                            Action::Skip(skipped.why),
                            "{mode:?} {taken:?}"
                        );
                        assert_eq!(foreseen.path, skipped.path);
                    }
                    Prepared::Write(target) => {
                        assert_eq!(foreseen.path, target.path(), "{mode:?} {taken:?}");
                        let expected = match (mode, taken.len()) {
                            (OnConflict::CreateUniqueFilename, 0) => Action::Write,
                            (OnConflict::CreateUniqueFilename, _) => Action::Number,
                            (_, 0) => Action::Write,
                            _ => Action::Overwrite,
                        };
                        assert_eq!(foreseen.action, expected, "{mode:?} {taken:?}");
                    }
                }
                assert_eq!(foreseen.exists, !taken.is_empty(), "{mode:?} {taken:?}");
            }
        }
    }

    #[test]
    fn foresee_makes_no_folder_and_no_file() {
        let folder = tempfile::tempdir().expect("a folder");
        let deeper = folder.path().join("not/yet/made");
        for mode in MODES {
            let foreseen = disk(&deeper, mode)
                .foresee(&photo(), "jpg", None, &Claimed::default())
                .expect("foreseen");
            assert_eq!(foreseen.path, deeper.join("IMG.jpg"));
            assert_eq!(foreseen.action, Action::Write);
        }
        assert!(names(folder.path()).is_empty(), "nothing was made");
    }

    #[test]
    fn a_file_an_earlier_photo_takes_counts_as_there_for_the_next() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut claimed = Claimed::default();
        claimed.insert(&folder.path().join("IMG.jpg"));
        let changed = Some(SystemTime::now() + Duration::from_secs(3600));
        let action = |mode| {
            disk(folder.path(), mode)
                .foresee(&photo(), "jpg", changed, &claimed)
                .expect("foreseen")
        };
        let unique = action(OnConflict::CreateUniqueFilename);
        assert_eq!(unique.action, Action::Number);
        assert_eq!(unique.path, folder.path().join("IMG_01.jpg"));
        assert_eq!(action(OnConflict::Overwrite).action, Action::Overwrite);
        assert_eq!(action(OnConflict::Skip).action, Action::Skip(Why::Exists));
        // Written moments ago, so newer than the edit.
        assert_eq!(
            action(OnConflict::OverwriteIfChanged).action,
            Action::Skip(Why::NotChanged)
        );
    }

    #[test]
    fn the_sequence_rule_is_told() {
        let said = |pattern, photos| {
            Disk::new(pattern, OnConflict::Skip, photos, None)
                .expect("a pattern")
                .sequence_added()
        };
        assert!(said("/out/IMG", 3));
        assert!(!said("/out/IMG", 1));
        assert!(!said("/out/$(FILE_NAME)", 3));
    }

    #[test]
    fn a_folder_is_ready_or_made_or_blocked() {
        let folder = tempfile::tempdir().expect("a folder");
        assert_eq!(preflight(folder.path()), Preflight::Ready);
        // Missing folders are made, from the nearest that is there.
        assert_eq!(
            preflight(&folder.path().join("a/b")),
            Preflight::Creates {
                from: folder.path().to_owned()
            }
        );
        // A file where a folder is wanted, or on the way to it.
        let file = folder.path().join("file");
        fs::write(&file, b"x").expect("a file");
        let blocked = Preflight::Blocked {
            at: file.clone(),
            why: Blocked::NotAFolder,
        };
        assert_eq!(preflight(&file), blocked);
        assert_eq!(preflight(&file.join("sub")), blocked);
        assert!(names(folder.path()) == ["file"], "nothing was made");
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_nobody_may_write_is_blocked_and_one_below_it_too() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().expect("a folder");
        let locked = folder.path().join("locked");
        fs::create_dir(&locked).expect("a folder");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("locked");
        let blocked = Preflight::Blocked {
            at: locked.clone(),
            why: Blocked::ReadOnly,
        };
        assert_eq!(preflight(&locked), blocked);
        assert_eq!(preflight(&locked.join("new/deeper")), blocked);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("unlocked");
        assert_eq!(preflight(&locked), Preflight::Ready);
    }

    #[test]
    fn a_relative_folder_is_made_in_the_current_folder_if_that_is_there() {
        let here = tempfile::tempdir().expect("a folder");
        let missing = Path::new("zz-no-such-folder/deeper");
        assert_eq!(
            preflight_from(missing, here.path()),
            Preflight::Creates {
                from: here.path().to_owned()
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_relative_folder_in_a_current_folder_nobody_may_write_is_blocked() {
        use std::os::unix::fs::PermissionsExt;
        let folder = tempfile::tempdir().expect("a folder");
        let locked = folder.path().join("locked");
        fs::create_dir(&locked).expect("a folder");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("locked");
        assert_eq!(
            preflight_from(Path::new("zz-no-such-folder"), &locked),
            Preflight::Blocked {
                at: locked.clone(),
                why: Blocked::ReadOnly
            }
        );
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("unlocked");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_leads_nowhere_is_blocked_on_the_way_and_as_the_folder() {
        let folder = tempfile::tempdir().expect("a folder");
        let link = folder.path().join("link");
        std::os::unix::fs::symlink(folder.path().join("nowhere"), &link).expect("a link");
        let blocked = Preflight::Blocked {
            at: link.clone(),
            why: Blocked::NotAFolder,
        };
        assert_eq!(preflight(&link), blocked);
        assert_eq!(preflight(&link.join("sub/deeper")), blocked);
        // A link to a folder is that folder.
        let real = folder.path().join("real");
        fs::create_dir(&real).expect("a folder");
        let good = folder.path().join("good");
        std::os::unix::fs::symlink(&real, &good).expect("a link");
        assert_eq!(preflight(&good), Preflight::Ready);
        assert_eq!(
            preflight(&good.join("new")),
            Preflight::Creates { from: good }
        );
    }

    #[test]
    fn names_differing_in_case_are_one_file_where_the_system_folds_them() {
        let folder = tempfile::tempdir().expect("a folder");
        let upper = folder.path().join("IMG.jpg");
        let lower = folder.path().join("img.jpg");
        let mut folded = Claimed::new(true);
        folded.insert(&upper);
        assert!(folded.contains(&lower));
        assert!(folded.contains(&upper));
        let mut exact = Claimed::new(false);
        exact.insert(&upper);
        assert!(!exact.contains(&lower));
        assert!(exact.contains(&upper));
        // The next photo's name is numbered as the export would number it.
        let mut claimed = Claimed::new(true);
        claimed.insert(&folder.path().join("img.JPG"));
        let foreseen = disk(folder.path(), OnConflict::CreateUniqueFilename)
            .foresee(&photo(), "jpg", None, &claimed)
            .expect("foreseen");
        assert_eq!(foreseen.action, Action::Number);
        // The platform's own choice is the file systems' usual one there.
        let mut platform = Claimed::default();
        platform.insert(&upper);
        assert_eq!(
            platform.contains(&lower),
            cfg!(any(target_os = "macos", target_os = "windows"))
        );
    }

    #[test]
    fn a_volume_that_is_not_connected_is_told() {
        for (path, volume) in [
            ("/Volumes/Card/out", "/Volumes/Card"),
            ("/media/ana/CARD/out", "/media/ana/CARD"),
            ("/run/media/ana/CARD/out/x", "/run/media/ana/CARD"),
        ] {
            // Names no disk has.
            let path = path
                .replace("Card", "apertrust-no-disk")
                .replace("CARD", "apertrust-no-disk");
            let volume = volume
                .replace("Card", "apertrust-no-disk")
                .replace("CARD", "apertrust-no-disk");
            assert_eq!(
                preflight(Path::new(&path)),
                Preflight::Blocked {
                    at: PathBuf::from(volume),
                    why: Blocked::NotConnected
                },
                "{path}"
            );
        }
    }

    #[test]
    fn a_volume_is_found_by_where_systems_mount_them() {
        let volume = |path: &str| volume_of(Path::new(path));
        assert_eq!(
            volume("/Volumes/Photos/a/b"),
            Some("/Volumes/Photos".into())
        );
        assert_eq!(volume("/Volumes/Photos"), Some("/Volumes/Photos".into()));
        assert_eq!(volume("/media/ana/CARD/x"), Some("/media/ana/CARD".into()));
        assert_eq!(
            volume("/run/media/ana/CARD"),
            Some("/run/media/ana/CARD".into())
        );
        // Not a volume: the folder above them, or anywhere else.
        for path in [
            "/Volumes",
            "/media/ana",
            "/run/media/ana",
            "/run/user",
            "/home/ana",
            "out/x",
        ] {
            assert_eq!(volume(path), None, "{path}");
        }
    }
}
