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

//! File on disk (decision 56): the pattern's names, On conflict's four modes, the folder's
//! creation and the write beside the name, each in a temporary folder.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use apertrust_export::disk::{Disk, Error, Prepared, Skipped, Stored, Why};
use apertrust_export::settings::OnConflict;
use apertrust_export::variables::{PatternError, Values};
use apertrust_export::{Format, Image, jpeg, png};
use tempfile::TempDir;

/// A pattern in `folder`, which a pattern reads with `/` only: a `\` would escape.
fn pattern(folder: &TempDir, rest: &str) -> String {
    let folder = folder.path().to_string_lossy().replace('\\', "/");
    format!("{folder}/{rest}")
}

fn photo(path: &str, sequence: u32) -> Values {
    Values {
        path: path.to_owned(),
        sequence,
        ..Values::default()
    }
}

/// `disk`'s write of `bytes` for `values`, as an export does it.
fn store(disk: &Disk, values: &Values, bytes: &[u8]) -> Result<Stored, Error> {
    match disk.prepare(values, "jpg", None)? {
        Prepared::Skip(skipped) => Ok(Stored::Skipped(skipped)),
        Prepared::Write(mut target) => {
            target.file().write_all(bytes).expect("written");
            target.finish()
        }
    }
}

fn written(stored: Result<Stored, Error>) -> PathBuf {
    match stored.expect("stored") {
        Stored::Written(path) => path,
        other => panic!("not written: {other:?}"),
    }
}

/// The names in `folder`, sorted: temporary files among them.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(folder)
        .expect("the folder")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn disk(pattern: &str, on_conflict: OnConflict, photos: usize) -> Disk {
    Disk::new(pattern, on_conflict, photos, None).expect("a pattern")
}

#[test]
fn the_name_is_the_pattern_s_with_the_format_s_extension() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(
        &pattern(&folder, "$(FILE_NAME)_edit"),
        OnConflict::Overwrite,
        1,
    );
    let name = disk.name(&photo("/photos/IMG_0001.ARW", 1), "png").unwrap();
    assert_eq!(name, folder.path().join("IMG_0001_edit.png"));
}

#[test]
fn several_photos_with_no_variable_take_a_sequence_number() {
    let folder = tempfile::tempdir().unwrap();
    let plain = pattern(&folder, "out");
    let several = disk(&plain, OnConflict::Overwrite, 3);
    let names: Vec<_> = (1..=3)
        .map(|n| several.name(&photo("/p/a.ARW", n), "jpg").unwrap())
        .collect();
    let expected: Vec<_> = ["out_0001.jpg", "out_0002.jpg", "out_0003.jpg"]
        .iter()
        .map(|name| folder.path().join(name))
        .collect();
    assert_eq!(names, expected);
    // One photo, or any `$` in the pattern, adds nothing.
    let one = disk(&plain, OnConflict::Overwrite, 1);
    assert_eq!(
        one.name(&photo("/p/a.ARW", 1), "jpg").unwrap(),
        folder.path().join("out.jpg")
    );
    let named = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 3);
    assert_eq!(
        named.name(&photo("/p/a.ARW", 2), "jpg").unwrap(),
        folder.path().join("a.jpg")
    );
}

#[test]
fn a_leading_tilde_is_the_home_folder() {
    let home = tempfile::tempdir().unwrap();
    let disk = Disk::new(
        "~/out/$(FILE_NAME)",
        OnConflict::Overwrite,
        1,
        Some(home.path()),
    )
    .expect("a pattern");
    let name = disk.name(&photo("/p/a.ARW", 1), "jpg").unwrap();
    assert_eq!(name, home.path().join("out").join("a.jpg"));
}

#[test]
fn a_pattern_that_expands_to_a_folder_takes_the_file_name() {
    let folder = tempfile::tempdir().unwrap();
    // The lens is empty, so the pattern expands to `shots/`: a folder, which takes the
    // original pattern, then `/$(FILE_NAME)`.
    let disk = disk(
        &pattern(&folder, "shots/$(LENS-)"),
        OnConflict::Overwrite,
        1,
    );
    let name = disk.name(&photo("/p/a.ARW", 1), "jpg").unwrap();
    assert_eq!(name, folder.path().join("shots").join("a.jpg"));
}

#[test]
fn a_path_that_is_empty_or_unreadable_is_refused_before_any_photo() {
    assert!(matches!(
        Disk::new("", OnConflict::Overwrite, 1, None),
        Err(Error::Empty)
    ));
    assert!(matches!(
        Disk::new("$(NONEXISTANT)", OnConflict::Overwrite, 1, None),
        Err(Error::Pattern(PatternError::Unknown { .. }))
    ));
    assert!(matches!(
        Disk::new("out/$(FILE_NAME", OnConflict::Overwrite, 1, None),
        Err(Error::Pattern(PatternError::Unclosed { .. }))
    ));
    // A pattern that expands to nothing for a photo is that photo's.
    let nothing = disk("$(LENS-)", OnConflict::Overwrite, 1);
    assert!(matches!(
        nothing.name(&photo("/p/a.ARW", 1), "jpg"),
        Err(Error::NoName(_))
    ));
}

#[test]
fn the_folder_is_made_with_its_parents() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(
        &pattern(&folder, "a/b/c/$(FILE_NAME)"),
        OnConflict::Overwrite,
        1,
    );
    let path = written(store(&disk, &photo("/p/IMG.ARW", 1), b"x"));
    assert_eq!(path, folder.path().join("a/b/c/IMG.jpg"));
    assert_eq!(fs::read(path).unwrap(), b"x");
}

#[test]
fn create_unique_filename_numbers_as_darktable_does() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(
        &pattern(&folder, "$(FILE_NAME)"),
        OnConflict::CreateUniqueFilename,
        1,
    );
    let values = photo("/p/IMG.ARW", 1);
    for expected in ["IMG.jpg", "IMG_01.jpg", "IMG_02.jpg"] {
        assert_eq!(
            written(store(&disk, &values, b"x")),
            folder.path().join(expected)
        );
    }
    // The first that is free, not the next after the last: a gap is filled.
    fs::remove_file(folder.path().join("IMG_01.jpg")).unwrap();
    assert_eq!(
        written(store(&disk, &values, b"x")),
        folder.path().join("IMG_01.jpg")
    );
    // Two digits to 99, then as many as the number takes, as `_%.2d` writes it.
    for number in 3..=100 {
        fs::write(folder.path().join(format!("IMG_{number:02}.jpg")), b"x").unwrap();
    }
    assert_eq!(
        written(store(&disk, &values, b"x")),
        folder.path().join("IMG_101.jpg")
    );
}

#[test]
fn a_unique_name_taken_meanwhile_takes_the_next_free_one() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(
        &pattern(&folder, "$(FILE_NAME)"),
        OnConflict::CreateUniqueFilename,
        1,
    );
    let Prepared::Write(mut target) = disk.prepare(&photo("/p/IMG.ARW", 1), "jpg", None).unwrap()
    else {
        panic!("a free name is written");
    };
    assert_eq!(target.path(), folder.path().join("IMG.jpg"));
    // Another program or export takes IMG.jpg and IMG_01.jpg before this one is done.
    fs::write(folder.path().join("IMG.jpg"), b"theirs").unwrap();
    fs::write(folder.path().join("IMG_01.jpg"), b"theirs too").unwrap();
    target.file().write_all(b"ours").unwrap();
    let stored = written(Ok(target.finish().unwrap()));
    assert_eq!(stored, folder.path().join("IMG_02.jpg"));
    assert_eq!(fs::read(&stored).unwrap(), b"ours");
    // Theirs are as they were, and no temporary file remains.
    assert_eq!(fs::read(folder.path().join("IMG.jpg")).unwrap(), b"theirs");
    assert_eq!(
        fs::read(folder.path().join("IMG_01.jpg")).unwrap(),
        b"theirs too"
    );
    assert_eq!(
        names(folder.path()),
        ["IMG.jpg", "IMG_01.jpg", "IMG_02.jpg"]
    );
}

#[test]
fn overwrite_replaces_the_file() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    let values = photo("/p/IMG.ARW", 1);
    written(store(&disk, &values, b"first"));
    let path = written(store(&disk, &values, b"second"));
    assert_eq!(path, folder.path().join("IMG.jpg"));
    assert_eq!(fs::read(&path).unwrap(), b"second");
    assert_eq!(names(folder.path()), ["IMG.jpg"]);
}

#[test]
fn overwrite_if_changed_keeps_a_file_newer_than_the_edit() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(
        &pattern(&folder, "$(FILE_NAME)"),
        OnConflict::OverwriteIfChanged,
        1,
    );
    let values = photo("/p/IMG.ARW", 1);
    let path = folder.path().join("IMG.jpg");
    let prepare = |changed| disk.prepare(&values, "jpg", changed).unwrap();
    let at = |secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);

    // No file: exported, whatever the edit.
    let Prepared::Write(mut target) = prepare(Some(at(5000))) else {
        panic!("no file, no skip");
    };
    target.file().write_all(b"old").unwrap();
    target.finish().unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(at(1000))
        .unwrap();

    // Edited after the file: exported again, replacing it.
    let Prepared::Write(mut target) = prepare(Some(at(2000))) else {
        panic!("an edit newer than the file is exported");
    };
    target.file().write_all(b"new").unwrap();
    assert_eq!(target.finish().unwrap(), Stored::Written(path.clone()));
    assert_eq!(fs::read(&path).unwrap(), b"new");

    // The file, now newer than an edit at 2000, and than a photo never edited, is kept.
    for changed in [Some(at(2000)), None] {
        let Prepared::Skip(skipped) = prepare(changed) else {
            panic!("a file newer than the edit is kept");
        };
        assert_eq!(
            skipped,
            Skipped {
                path: path.clone(),
                why: Why::NotChanged
            }
        );
    }
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert_eq!(names(folder.path()), ["IMG.jpg"]);
}

#[test]
fn skip_keeps_a_file_that_is_there() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Skip, 1);
    let values = photo("/p/IMG.ARW", 1);
    let path = written(store(&disk, &values, b"first"));
    let Stored::Skipped(skipped) = store(&disk, &values, b"second").unwrap() else {
        panic!("a file there is skipped");
    };
    assert_eq!(
        (skipped.path.as_path(), skipped.why),
        (path.as_path(), Why::Exists)
    );
    assert_eq!(fs::read(&path).unwrap(), b"first");
    assert_eq!(names(folder.path()), ["IMG.jpg"]);
}

#[test]
fn skip_and_a_name_taken_meanwhile_skips_without_replacing() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Skip, 1);
    let Prepared::Write(mut target) = disk.prepare(&photo("/p/IMG.ARW", 1), "jpg", None).unwrap()
    else {
        panic!("a free name is written");
    };
    fs::write(folder.path().join("IMG.jpg"), b"theirs").unwrap();
    target.file().write_all(b"ours").unwrap();
    let Stored::Skipped(skipped) = target.finish().unwrap() else {
        panic!("a name taken meanwhile is skipped");
    };
    assert_eq!(skipped.why, Why::Exists);
    assert_eq!(fs::read(folder.path().join("IMG.jpg")).unwrap(), b"theirs");
    assert_eq!(names(folder.path()), ["IMG.jpg"]);
}

#[test]
fn a_temporary_file_is_beside_its_name_and_never_under_it_until_done() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    let Prepared::Write(mut target) = disk.prepare(&photo("/p/IMG.ARW", 1), "jpg", None).unwrap()
    else {
        panic!("written");
    };
    target.file().write_all(b"half").unwrap();
    // Only the temporary file, named after the file it will be and ending in .tmp.
    let held = names(folder.path());
    assert_eq!(held.len(), 1);
    assert!(
        held[0].starts_with(".~IMG.jpg.") && held[0].ends_with(".tmp"),
        "{held:?}"
    );
    target.finish().unwrap();
    assert_eq!(names(folder.path()), ["IMG.jpg"]);
}

#[test]
fn a_target_dropped_leaves_no_temporary_file() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    let Prepared::Write(mut target) = disk.prepare(&photo("/p/IMG.ARW", 1), "jpg", None).unwrap()
    else {
        panic!("written");
    };
    // A write that fails halfway: the caller drops the target.
    target.file().write_all(b"half").unwrap();
    drop(target);
    assert!(names(folder.path()).is_empty());
}

#[test]
fn a_rename_that_fails_leaves_no_temporary_file_and_the_name_as_it_was() {
    let folder = tempfile::tempdir().unwrap();
    // A folder is under the photo's name: no file replaces it.
    let blocking = folder.path().join("IMG.jpg");
    fs::create_dir(&blocking).unwrap();
    fs::write(blocking.join("inside"), b"x").unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    let error = store(&disk, &photo("/p/IMG.ARW", 1), b"x").unwrap_err();
    assert!(matches!(error, Error::Write { .. }), "{error}");
    assert!(error.to_string().contains("IMG.jpg"), "{error}");
    assert_eq!(names(folder.path()), ["IMG.jpg"]);
    assert_eq!(fs::read(blocking.join("inside")).unwrap(), b"x");
}

#[test]
fn a_folder_that_cannot_be_created_is_the_destination_s_fault() {
    let folder = tempfile::tempdir().unwrap();
    // A file where a folder should be.
    fs::write(folder.path().join("file"), b"x").unwrap();
    let disk = disk(
        &pattern(&folder, "file/inside/$(FILE_NAME)"),
        OnConflict::Overwrite,
        1,
    );
    let error = disk
        .prepare(&photo("/p/IMG.ARW", 1), "jpg", None)
        .unwrap_err();
    assert!(matches!(error, Error::CreateFolder { .. }), "{error}");
    assert!(error.destination());
    assert_eq!(names(folder.path()), ["file"]);
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_written_is_the_destination_s_fault() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().unwrap();
    let locked = folder.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let restore = || fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    // Root writes anywhere: nothing to see.
    if fs::write(locked.join("probe"), b"x").is_ok() {
        restore();
        return;
    }
    let disk = disk(
        &pattern(&folder, "locked/$(FILE_NAME)"),
        OnConflict::Overwrite,
        1,
    );
    let error = disk
        .prepare(&photo("/p/IMG.ARW", 1), "jpg", None)
        .unwrap_err();
    restore();
    assert!(matches!(error, Error::WriteFolder { .. }), "{error}");
    assert!(error.destination());
    assert!(names(&locked).is_empty());
}

#[cfg(unix)]
#[test]
fn the_file_has_the_permissions_a_file_made_in_the_usual_way_has() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().unwrap();
    let plain = folder.path().join("plain");
    fs::File::create(&plain).unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    let path = written(store(&disk, &photo("/p/IMG.ARW", 1), b"x"));
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&path), mode(&plain));
}

#[test]
fn several_photos_with_one_pattern_each_get_a_file() {
    let folder = tempfile::tempdir().unwrap();
    // darktable's: no variable, so the sequence number is added.
    let disk = disk(
        &pattern(&folder, "set/shot"),
        OnConflict::CreateUniqueFilename,
        3,
    );
    let paths: Vec<_> = (1..=3)
        .map(|n| written(store(&disk, &photo(&format!("/p/{n}.ARW"), n), &[n as u8])))
        .collect();
    assert_eq!(
        names(&folder.path().join("set")),
        ["shot_0001.jpg", "shot_0002.jpg", "shot_0003.jpg"]
    );
    for (n, path) in paths.iter().enumerate() {
        assert_eq!(fs::read(path).unwrap(), [n as u8 + 1]);
    }
    // And a pattern of the file's own name, two photos of one name: the second is unique.
    let by_name = self::disk(
        &pattern(&folder, "names/$(FILE_NAME)"),
        OnConflict::CreateUniqueFilename,
        2,
    );
    let first = written(store(&by_name, &photo("/one/IMG.ARW", 1), b"1"));
    let second = written(store(&by_name, &photo("/two/IMG.ARW", 2), b"2"));
    assert_eq!(first, folder.path().join("names/IMG.jpg"));
    assert_eq!(second, folder.path().join("names/IMG_01.jpg"));
}

#[test]
fn a_name_of_the_longest_length_has_a_temporary_file() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    // 255 bytes with the extension: the temporary file's name is 13 bytes more than a
    // plain copy of it, so its name part is cut.
    for length in [243, 255] {
        let stem = "x".repeat(length - ".jpg".len());
        let stored = written(store(&disk, &photo(&format!("/p/{stem}.ARW"), 1), b"x"));
        assert_eq!(stored.file_name().unwrap().len(), length);
        assert_eq!(fs::read(&stored).unwrap(), b"x");
    }
}

#[test]
fn a_name_too_long_for_the_file_system_is_the_photo_s_error() {
    let folder = tempfile::tempdir().unwrap();
    let disk = disk(&pattern(&folder, "$(FILE_NAME)"), OnConflict::Overwrite, 1);
    let stem = "x".repeat(300);
    let error = store(&disk, &photo(&format!("/p/{stem}.ARW"), 1), b"x").unwrap_err();
    // It is the rename that finds the name too long, not the folder, which is fine.
    assert!(matches!(error, Error::Write { .. }), "{error}");
    assert!(!error.destination(), "{error}");
    assert!(names(folder.path()).is_empty());
}

/// A writer whose disk is full.
struct Full;

impl Write for Full {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::StorageFull.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::StorageFull.into())
    }
}

#[test]
fn a_full_disk_found_by_the_encoder_is_the_destination_s_fault() {
    let pixels = vec![[0.5f32; 4]; 64 * 64];
    let image = Image::new(&pixels, 64, 64).unwrap();
    for format in [
        Format::Jpeg(jpeg::Options::default()),
        Format::Png(png::Options::default()),
    ] {
        let error = format.write(image, Full).unwrap_err();
        assert!(error.destination(), "{}: {error}", format.name());
    }
    // An error of the image or the options is the photo's.
    let error = Image::new(&pixels, 63, 64).unwrap_err();
    assert!(!error.destination(), "{error}");
    let other = apertrust_export::Error::Io(io::ErrorKind::PermissionDenied.into());
    assert!(!other.destination());
}
