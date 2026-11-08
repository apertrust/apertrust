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

//! The hidden temporary file a write goes through: its name, and the removal of the ones a
//! crash left behind.
//!
//! The name is `.~NAME.XXXXXX.tmp`: a dot, so every file manager hides it, a tilde as
//! editors mark their scratch files, the name of the file it is to become so a stray one
//! is told by its name, six random characters so two writers never meet, and `.tmp`.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use tempfile::NamedTempFile;

/// What stands before the final name in a temporary's name.
const PREFIX: &str = ".~";
/// What stands after it: a dot, `RANDOM` random characters and the suffix.
const SUFFIX: &str = ".tmp";
/// How many random characters `tempfile` adds, which [`is_temporary_name`] expects.
const RANDOM: usize = 6;

/// The most bytes of a final name the temporary's name can repeat: the longest name a file
/// system takes, 255 bytes, less what is added round it, so a final name of any length that
/// fits has a temporary file too, and one that does not fit is that file's error when it is
/// renamed.
const NAME_PART: usize = 255 - (PREFIX.len() + 1 + RANDOM + SUFFIX.len());

/// How old a temporary must be before [`remove_if_stale`] removes it: a write that takes a
/// day is a write that is gone.
pub const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// `name` cut at a character to at most [`NAME_PART`] bytes.
fn shortened(name: &str) -> &str {
    let mut end = name.len().min(NAME_PART);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    &name[..end]
}

/// Makes the hidden temporary file for the file `name` in `folder`, to be written and
/// renamed over `folder/name`. Dropped, it removes itself, so a failed write leaves
/// nothing. A file made in the usual way has the permissions the process's umask gives it,
/// not `tempfile`'s 0600.
pub fn create(folder: &Path, name: &str) -> io::Result<NamedTempFile> {
    let mut builder = tempfile::Builder::new();
    let prefix = format!("{PREFIX}{}.", shortened(name));
    builder.prefix(&prefix).suffix(SUFFIX).rand_bytes(RANDOM);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o666));
    }
    // Its path, which the rename hands Windows as it is, reaches any length.
    builder.tempfile_in(super::long_path(folder))
}

/// Whether `name` is shaped like a temporary [`create`] makes: `.~`, a name, a dot, six
/// letters or digits and `.tmp`. Nothing else is: `.~notes.tmp` and `.notes.AbC123.tmp` are
/// someone's files.
pub fn is_temporary_name(name: &str) -> bool {
    let Some(rest) = name
        .strip_prefix(PREFIX)
        .and_then(|rest| rest.strip_suffix(SUFFIX))
    else {
        return false;
    };
    let rest = rest.as_bytes();
    let Some(split) = rest.len().checked_sub(RANDOM + 1) else {
        return false;
    };
    split > 0 && rest[split] == b'.' && rest[split + 1..].iter().all(u8::is_ascii_alphanumeric)
}

/// Removes the file `name` in `folder` if it is a temporary left by a write that never
/// ended: [`is_temporary_name`], a plain file (not a folder or a link), and last modified
/// more than [`STALE_AFTER`] before `now`. True if it was removed; a file that is not
/// stale, is not shaped like a temporary or has gone is left, and false.
pub fn remove_if_stale(folder: &Path, name: &str, now: SystemTime) -> io::Result<bool> {
    if !is_temporary_name(name) {
        return Ok(false);
    }
    let path = folder.join(name);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let stale = meta.is_file()
        && meta
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > STALE_AFTER);
    if !stale {
        return Ok(false);
    }
    match fs::remove_file(&path) {
        Ok(()) => Ok(true),
        // Gone since it was looked at: another process swept it.
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aged(path: &Path, age: Duration) -> SystemTime {
        let now = SystemTime::now();
        fs::File::options()
            .write(true)
            .open(path)
            .expect("the file")
            .set_modified(now - age)
            .expect("a time");
        now
    }

    #[test]
    fn the_temporary_name_never_passes_what_a_file_system_takes() {
        assert_eq!(shortened("IMG.jpg"), "IMG.jpg");
        let long = "a".repeat(255);
        assert_eq!(shortened(&long).len(), NAME_PART);
        // Cut at a character: 'é' is two bytes, and 242 is not a boundary of 'éé…'.
        let accented = "é".repeat(130);
        let cut = shortened(&accented);
        assert_eq!(cut.len(), 242);
        assert!(cut.chars().all(|c| c == 'é'));
        let odd = format!("a{}", "é".repeat(130));
        assert_eq!(shortened(&odd).len(), 241);
        assert_eq!(2 + NAME_PART + 1 + 6 + ".tmp".len(), 255);
    }

    #[test]
    fn a_temporary_is_named_after_its_file_and_told_by_its_name() {
        let folder = tempfile::tempdir().expect("a folder");
        let temp = create(folder.path(), "IMG_2041.ARW.apxmp").expect("a temporary");
        let name = temp.path().file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".~IMG_2041.ARW.apxmp."), "{name}");
        assert!(name.ends_with(".tmp"));
        assert!(is_temporary_name(name), "{name}");
        // A name of any length that fits has a temporary that fits.
        let long = "x".repeat(255);
        let temp = create(folder.path(), &long).expect("a temporary for a long name");
        assert!(temp.path().file_name().unwrap().len() <= 255);
        assert!(is_temporary_name(
            temp.path().file_name().unwrap().to_str().unwrap()
        ));
    }

    #[test]
    fn only_the_hidden_temporarys_naming_is_recognised() {
        for name in [
            ".~IMG.ARW.apxmp.AbC123.tmp",
            ".~a.000000.tmp",
            ".~.hidden.zZ9aB0.tmp",
            ".~é.AbC123.tmp",
        ] {
            assert!(is_temporary_name(name), "{name}");
        }
        for name in [
            "",
            ".tmp",
            ".~.tmp",
            ".~notes.tmp",
            // The name part is empty.
            ".~.AbC123.tmp",
            // Five and seven random characters.
            ".~IMG.AbC12.tmp",
            ".~IMG.AbC1234.tmp",
            // Not letters or digits.
            ".~IMG.AbC12-.tmp",
            ".~IMG.AbC12é.tmp",
            // No dot, no tilde, no suffix, the wrong case of the suffix.
            ".~IMGAbC123.tmp",
            ".IMG.ARW.AbC123.tmp",
            "~IMG.ARW.AbC123.tmp",
            "IMG.ARW.AbC123.tmp",
            ".~IMG.ARW.AbC123.temp",
            ".~IMG.ARW.AbC123.TMP",
            ".~IMG.ARW.AbC123.tmp.bak",
            // Another program's.
            "~$report.docx",
            ".DSCF0001.RAF.apxmp.0123abcd.tmp",
        ] {
            assert!(!is_temporary_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_stale_temporary_is_removed_and_a_young_one_kept() {
        let folder = tempfile::tempdir().expect("a folder");
        let old = folder.path().join(".~OLD.ARW.apxmp.AbC123.tmp");
        let young = folder.path().join(".~YOUNG.ARW.apxmp.AbC123.tmp");
        let edge = folder.path().join(".~EDGE.ARW.apxmp.AbC123.tmp");
        for path in [&old, &young, &edge] {
            fs::write(path, b"half a file").expect("written");
        }
        let now = aged(&old, STALE_AFTER + Duration::from_secs(60));
        aged(&young, Duration::from_secs(60 * 60));
        // Not yet a whole day old.
        aged(&edge, STALE_AFTER - Duration::from_secs(60));

        for path in [&old, &young, &edge] {
            let name = path.file_name().unwrap().to_str().unwrap();
            let removed = remove_if_stale(folder.path(), name, now).expect("swept");
            assert_eq!(removed, path == &old, "{name}");
        }
        assert!(!old.exists() && young.exists() && edge.exists());
        // A name that has gone is no error.
        assert!(!remove_if_stale(folder.path(), ".~OLD.ARW.apxmp.AbC123.tmp", now).unwrap());
    }

    #[test]
    fn a_file_that_merely_looks_similar_is_never_removed() {
        let folder = tempfile::tempdir().expect("a folder");
        let names = [
            "notes.tmp",
            ".~notes.tmp",
            ".~IMG.AbC12.tmp",
            ".IMG.ARW.AbC123.tmp",
            ".~IMG.ARW.AbC123.tmp.bak",
            "IMG.ARW.apxmp",
            ".DSCF0001.RAF.apxmp.0123abcd.tmp",
        ];
        let mut now = SystemTime::now();
        for name in names {
            let path = folder.path().join(name);
            fs::write(&path, b"mine").expect("written");
            now = aged(&path, STALE_AFTER * 30);
        }
        for name in names {
            assert!(
                !remove_if_stale(folder.path(), name, now).unwrap(),
                "{name}"
            );
            assert_eq!(fs::read(folder.path().join(name)).unwrap(), b"mine");
        }
    }

    #[test]
    fn a_folder_or_a_link_named_like_a_temporary_is_never_removed() {
        let folder = tempfile::tempdir().expect("a folder");
        let directory = folder.path().join(".~DIR.AbC123.tmp");
        fs::create_dir(&directory).expect("a folder named like one");
        let now = aged_dir(&directory);
        assert!(!remove_if_stale(folder.path(), ".~DIR.AbC123.tmp", now).unwrap());
        assert!(directory.is_dir());

        #[cfg(unix)]
        {
            let target = folder.path().join("target.ARW");
            fs::write(&target, b"the photo").expect("a target");
            let link = folder.path().join(".~LINK.AbC123.tmp");
            std::os::unix::fs::symlink(&target, &link).expect("a link");
            aged(&target, STALE_AFTER * 30);
            assert!(!remove_if_stale(folder.path(), ".~LINK.AbC123.tmp", now).unwrap());
            assert!(link.symlink_metadata().is_ok());
            assert_eq!(fs::read(&target).unwrap(), b"the photo");
        }
    }

    /// A time a month after the folder was made.
    fn aged_dir(path: &Path) -> SystemTime {
        fs::metadata(path).unwrap().modified().unwrap() + STALE_AFTER * 30
    }
}
