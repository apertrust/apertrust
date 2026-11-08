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

//! The safe write (decision 53): a file replaced whole through a hidden temporary beside
//! it, and the rename that Windows sometimes refuses, tried again.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use tempfile::{NamedTempFile, PersistError};

use super::temporary;

/// How often a rename Windows refuses is tried again, and how long to wait between: an
/// indexer or a virus scanner often holds a file just written for a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retry {
    /// Tries after the first.
    pub attempts: u32,
    pub pause: Duration,
}

impl Retry {
    /// A refused rename is final.
    pub const NONE: Self = Self {
        attempts: 0,
        pause: Duration::ZERO,
    };

    /// The platform's: on Windows ten more tries 100 ms apart, about a second; elsewhere a
    /// refused rename is final.
    pub const fn platform() -> Self {
        if cfg!(windows) {
            Self {
                attempts: 10,
                pause: Duration::from_millis(100),
            }
        } else {
            Self::NONE
        }
    }
}

/// A refusal Windows gives while something else holds the file for a moment: access
/// denied, a sharing violation and a lock violation.
fn transient(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied
        || matches!(error.raw_os_error(), Some(5 | 32 | 33))
}

/// Renames `temp` to `path` with `rename` (`NamedTempFile::persist` or
/// `persist_noclobber`), trying again after `retry.pause` while it fails with a refusal
/// that passes, up to `retry.attempts` times. The error of the last try comes back with the
/// temporary file, which the caller may use again or drop. `rename` is given `path` in the
/// form that reaches any length on Windows.
pub fn persist_retrying(
    mut temp: NamedTempFile,
    path: &Path,
    retry: Retry,
    mut rename: impl FnMut(NamedTempFile, &Path) -> Result<File, PersistError>,
) -> Result<File, PersistError> {
    let path = super::long_path(path);
    let mut attempt = 0;
    loop {
        match rename(temp, &path) {
            Err(failed) if attempt < retry.attempts && transient(&failed.error) => {
                attempt += 1;
                temp = failed.file;
                std::thread::sleep(retry.pause);
            }
            done => return done,
        }
    }
}

/// What [`write()`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Written {
    /// The file now holds the bytes.
    Replaced,
    /// It held them already, and was not touched: its modified time is as it was.
    Unchanged,
}

/// Replaces the file at `path` with `bytes`, or makes it:
///
/// - nothing is written, and the file not touched, when it already holds `bytes`
///   ([`Written::Unchanged`]);
/// - otherwise they are written to a hidden temporary in its folder
///   ([`temporary::create`]), flushed, synced and renamed over `path`, the rename tried
///   again for about a second where Windows refuses it ([`Retry::platform`]);
/// - on an error the temporary file is removed, and the old file is as it was.
///
/// A link at `path` is replaced by the file, not written through. The folder must exist.
pub fn write(path: &Path, bytes: &[u8]) -> io::Result<Written> {
    write_with(
        path,
        bytes,
        Retry::platform(),
        |file, bytes| file.write_all(bytes),
        |temp, path| temp.persist(path),
    )
}

/// [`write()`] with its two steps given, so that a test can fail the one and refuse the
/// other: `put` writes the bytes into the temporary, `rename` moves it over `path`.
fn write_with(
    path: &Path,
    bytes: &[u8],
    retry: Retry,
    put: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    rename: impl FnMut(NamedTempFile, &Path) -> Result<File, PersistError>,
) -> io::Result<Written> {
    if holds(path, bytes) {
        return Ok(Written::Unchanged);
    }
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?;
    let folder = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    // Dropped on any error below, which removes it.
    let mut temp = temporary::create(folder, &name.to_string_lossy())?;
    put(temp.as_file_mut(), bytes)?;
    temp.as_file_mut().flush()?;
    temp.as_file().sync_all()?;
    persist_retrying(temp, path, retry, rename).map_err(|failed| failed.error)?;
    Ok(Written::Replaced)
}

/// Whether the file at `path` is there and holds exactly `bytes`.
fn holds(path: &Path, bytes: &[u8]) -> bool {
    fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() == bytes.len() as u64)
        && fs::read(path).is_ok_and(|found| found == bytes)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::time::SystemTime;

    use super::*;

    /// The names in `folder`, sorted.
    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(folder)
            .expect("the folder")
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn modified(path: &Path) -> SystemTime {
        fs::metadata(path).unwrap().modified().unwrap()
    }

    fn quickly(attempts: u32) -> Retry {
        Retry {
            attempts,
            pause: Duration::from_millis(1),
        }
    }

    fn plain(
        path: &Path,
        bytes: &[u8],
        retry: Retry,
        rename: impl FnMut(NamedTempFile, &Path) -> Result<File, PersistError>,
    ) -> io::Result<Written> {
        write_with(path, bytes, retry, |f, b| f.write_all(b), rename)
    }

    fn refused(kind: io::ErrorKind) -> impl Fn(NamedTempFile, &Path) -> Result<File, PersistError> {
        move |file, _| {
            Err(PersistError {
                error: kind.into(),
                file,
            })
        }
    }

    #[test]
    fn a_file_is_made_and_then_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("IMG.ARW.apxmp");
        assert_eq!(write(&path, b"one").unwrap(), Written::Replaced);
        assert_eq!(fs::read(&path).unwrap(), b"one");
        assert_eq!(write(&path, b"another, longer").unwrap(), Written::Replaced);
        assert_eq!(fs::read(&path).unwrap(), b"another, longer");
        assert_eq!(write(&path, b"").unwrap(), Written::Replaced);
        assert_eq!(fs::read(&path).unwrap(), b"");
        // Only the file: no temporary stays.
        assert_eq!(names(folder.path()), ["IMG.ARW.apxmp"]);
    }

    #[test]
    fn the_same_bytes_are_not_written_and_the_modified_time_is_kept() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("IMG.ARW.apxmp");
        fs::write(&path, b"same").unwrap();
        let then = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(then)
            .unwrap();

        assert_eq!(write(&path, b"same").unwrap(), Written::Unchanged);
        assert_eq!(modified(&path), then);
        assert_eq!(fs::read(&path).unwrap(), b"same");
        // One byte of difference writes.
        assert_eq!(write(&path, b"sama").unwrap(), Written::Replaced);
        assert_ne!(modified(&path), then);
        // The same bytes under another length: the old file is longer.
        assert_eq!(write(&path, b"sam").unwrap(), Written::Replaced);
        assert_eq!(fs::read(&path).unwrap(), b"sam");
    }

    #[test]
    fn a_write_that_fails_leaves_the_old_file_and_no_temporary() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("IMG.ARW.apxmp");
        fs::write(&path, b"the old edit").unwrap();

        // Half the bytes written, then the disk fills.
        let result = write_with(
            &path,
            b"the new edit",
            Retry::NONE,
            |file, bytes| {
                file.write_all(&bytes[..4])?;
                Err(io::ErrorKind::StorageFull.into())
            },
            |temp, path| temp.persist(path),
        );
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::StorageFull);
        assert_eq!(fs::read(&path).unwrap(), b"the old edit");
        assert_eq!(names(folder.path()), ["IMG.ARW.apxmp"]);

        // The rename fails for good.
        let result = plain(
            &path,
            b"the new edit",
            Retry::NONE,
            refused(io::ErrorKind::ReadOnlyFilesystem),
        );
        assert_eq!(
            result.unwrap_err().kind(),
            io::ErrorKind::ReadOnlyFilesystem
        );
        assert_eq!(fs::read(&path).unwrap(), b"the old edit");
        assert_eq!(names(folder.path()), ["IMG.ARW.apxmp"]);
    }

    #[test]
    fn a_write_into_a_missing_folder_is_an_error_and_makes_nothing() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("missing").join("IMG.ARW.apxmp");
        assert_eq!(
            write(&path, b"x").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(names(folder.path()).is_empty());
    }

    // These stand in for Windows with an injected refusal and no wait to speak of: the
    // retry's logic is tested, not a virus scanner's timing.
    #[test]
    fn a_rename_refused_for_a_moment_is_tried_again() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("IMG.ARW.apxmp");
        fs::write(&path, b"old").unwrap();
        let tries = Cell::new(0);
        let result = plain(&path, b"new", quickly(5), |temp, path| {
            tries.set(tries.get() + 1);
            if tries.get() <= 3 {
                // Access denied, then a sharing violation, then a lock violation.
                let error = match tries.get() {
                    1 => io::Error::from(io::ErrorKind::PermissionDenied),
                    2 => io::Error::from_raw_os_error(32),
                    _ => io::Error::from_raw_os_error(33),
                };
                Err(PersistError { error, file: temp })
            } else {
                temp.persist(path)
            }
        });
        assert_eq!(result.unwrap(), Written::Replaced);
        assert_eq!(tries.get(), 4);
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(names(folder.path()), ["IMG.ARW.apxmp"]);
    }

    #[test]
    fn a_rename_refused_for_good_gives_up_after_its_tries_and_changes_nothing() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("IMG.ARW.apxmp");
        fs::write(&path, b"old").unwrap();
        let tries = Cell::new(0);
        let result = plain(&path, b"new", quickly(4), |temp, _| {
            tries.set(tries.get() + 1);
            Err(PersistError {
                error: io::ErrorKind::PermissionDenied.into(),
                file: temp,
            })
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        // The first try and four more.
        assert_eq!(tries.get(), 5);
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert_eq!(names(folder.path()), ["IMG.ARW.apxmp"]);
    }

    #[test]
    fn a_refusal_that_cannot_pass_is_not_tried_again() {
        let folder = tempfile::tempdir().expect("a folder");
        let path = folder.path().join("IMG.ARW.apxmp");
        let tries = Cell::new(0);
        let result = plain(&path, b"new", quickly(5), |temp, _| {
            tries.set(tries.get() + 1);
            Err(PersistError {
                error: io::ErrorKind::StorageFull.into(),
                file: temp,
            })
        });
        assert!(result.is_err());
        assert_eq!(tries.get(), 1);
        assert!(names(folder.path()).is_empty());
    }

    #[test]
    fn no_retry_without_attempts_and_a_second_where_windows_asks_for_it() {
        let tries = Cell::new(0);
        let folder = tempfile::tempdir().expect("a folder");
        let result = plain(&folder.path().join("a"), b"x", Retry::NONE, |temp, _| {
            tries.set(tries.get() + 1);
            Err(PersistError {
                error: io::ErrorKind::PermissionDenied.into(),
                file: temp,
            })
        });
        assert!(result.is_err());
        assert_eq!(tries.get(), 1);

        let platform = Retry::platform();
        if cfg!(windows) {
            assert_eq!(platform.pause * platform.attempts, Duration::from_secs(1));
        } else {
            assert_eq!(platform, Retry::NONE);
        }
    }
}
