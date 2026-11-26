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

//! The checked copy's tests: a round trip, and a fault injected at every place the copy
//! loop can meet one, each leaving the destination as it was.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::{Seek, SeekFrom};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use super::*;

/// The chunk the tests copy in, so a file of a few chunks is cut mid-file.
const SMALL: usize = 4096;

/// A file's bytes and modified time.
type Contents = BTreeMap<String, (Vec<u8>, SystemTime)>;

/// Every entry of `folder` with its bytes and modified time, hidden ones included; empty
/// where the folder is not there.
fn contents(folder: &Path) -> Contents {
    let Ok(entries) = fs::read_dir(folder) else {
        return Contents::new();
    };
    entries
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let modified = fs::metadata(&path).unwrap().modified().unwrap();
            (name, (fs::read(&path).unwrap(), modified))
        })
        .collect()
}

fn set_modified(path: &Path, time: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// A photo of a raw and its three sidecars, each a few chunks long, given modified times
/// long past.
fn photo(folder: &Path) -> PhotoFiles {
    let files = [
        ("DSCF0001.RAF", 5 * SMALL + 123),
        ("DSCF0001.RAF.apxmp", 2 * SMALL + 1),
        ("DSCF0001.RAF.apdata", SMALL),
        ("DSCF0001.RAF.xmp", 300),
    ];
    for (i, (name, len)) in files.into_iter().enumerate() {
        let bytes: Vec<u8> = (0..len).map(|b| (b * 7 + i * 13) as u8).collect();
        let path = folder.join(name);
        fs::write(&path, bytes).unwrap();
        let then = SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000 + i as u64 * 60);
        set_modified(&path, then);
    }
    PhotoFiles::find(folder, "DSCF0001.RAF").unwrap()
}

/// Two folders, a photo in the one and someone's file in the other, which every copy must
/// leave alone.
struct Setup {
    _root: tempfile::TempDir,
    source: PhotoFiles,
    into: PathBuf,
    before: Contents,
    there: Contents,
}

fn setup() -> Setup {
    let root = tempfile::tempdir().unwrap();
    let from = root.path().join("card");
    let into = root.path().join("Pictures");
    fs::create_dir_all(&from).unwrap();
    fs::create_dir_all(&into).unwrap();
    fs::write(into.join("DSCF0002.RAF"), b"someone's photo").unwrap();
    let source = photo(&from);
    Setup {
        before: contents(&from),
        there: contents(&into),
        _root: root,
        source,
        into,
    }
}

impl Setup {
    fn copy(
        &self,
        check: Check,
        cancel: &(dyn Fn() -> bool + Sync),
        hook: &mut Hook<'_>,
    ) -> Result<Copied, CopyError> {
        copy_with(
            &self.source,
            &self.into,
            check,
            SMALL,
            cancel,
            &mut |_| {},
            hook,
        )
    }

    /// The source as it was, and the destination as it was: no temporary, no file of the
    /// photo, someone's file untouched.
    fn assert_untouched(&self) {
        assert_eq!(contents(&self.source.folder), self.before, "the source");
        assert_eq!(contents(&self.into), self.there, "the destination");
    }

    fn source_path(&self, index: usize) -> PathBuf {
        let name = self.source.names().nth(index).unwrap();
        self.source.folder.join(name)
    }

    fn destination_path(&self, index: usize) -> PathBuf {
        self.into.join(self.source.names().nth(index).unwrap())
    }
}

#[test]
fn a_photo_lands_whole_with_its_times_and_the_source_unchanged() {
    for check in [Check::None, Check::ReadBack] {
        let root = tempfile::tempdir().unwrap();
        let card = root.path().join("card");
        fs::create_dir(&card).unwrap();
        let source = photo(&card);
        assert_eq!(
            source.companions,
            [
                "DSCF0001.RAF.apxmp",
                "DSCF0001.RAF.apdata",
                "DSCF0001.RAF.xmp"
            ]
        );
        let before = contents(&card);
        // A folder that is not there yet, with its parent.
        let into = root.path().join("2026").join("Lisbon");
        let mut copied_bytes = 0;
        let copied = copy_photo(&source, &into, check, &|| false, &mut |bytes| {
            assert!(bytes as usize <= CHUNK);
            copied_bytes += bytes;
        })
        .unwrap();

        assert_eq!(contents(&card), before, "the source is unchanged");
        assert_eq!(contents(&into), before, "the same names, bytes and times");
        let names: Vec<_> = copied
            .files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names.last(), Some(&"DSCF0001.RAF"), "the main file last");
        assert_eq!(names.len(), 4);
        for file in &copied.files {
            let bytes = fs::read(&file.path).unwrap();
            assert_eq!(file.len, bytes.len() as u64);
            assert_eq!(file.hash, blake3::hash(&bytes));
        }
        let total: u64 = copied.files.iter().map(|f| f.len).sum();
        assert_eq!(copied_bytes, total);
    }
}

#[test]
fn only_the_files_beside_the_photo_by_its_whole_name_go_with_it() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("IMG_1.CR3"), b"raw").unwrap();
    fs::write(folder.path().join("IMG_1.CR3.apxmp"), b"edit").unwrap();
    // darktable's sidecar of another name, a folder named like one, another case.
    fs::write(folder.path().join("IMG_1.xmp"), b"not it").unwrap();
    fs::create_dir(folder.path().join("IMG_1.CR3.xmp")).unwrap();
    fs::write(folder.path().join("IMG_1.CR3.APDATA"), b"not it").unwrap();
    let files = PhotoFiles::find(folder.path(), "IMG_1.CR3").unwrap();
    if fs::symlink_metadata(folder.path().join("IMG_1.CR3.apdata")).is_ok() {
        // A file system that ignores case finds the upper-case one under the name.
        assert_eq!(files.companions, ["IMG_1.CR3.apxmp", "IMG_1.CR3.apdata"]);
    } else {
        assert_eq!(files.companions, ["IMG_1.CR3.apxmp"]);
    }
    assert_eq!(files.main, "IMG_1.CR3");
}

/// A cancel at every call of it, between files and in each chunk, the read-back's too:
/// each leaves nothing, and names the file it stopped at.
#[test]
fn a_cancel_anywhere_leaves_nothing() {
    let mut places = Vec::new();
    for check in [Check::None, Check::ReadBack] {
        let setup = setup();
        let mut at = 0;
        loop {
            let asked = AtomicUsize::new(0);
            let cancel = || asked.fetch_add(1, Ordering::Relaxed) >= at;
            let result = setup.copy(check, &cancel, &mut |_| Ok(()));
            if result.is_ok() {
                break;
            }
            match result.unwrap_err() {
                CopyError::Cancelled { file: named } => {
                    let names: Vec<_> = (0..4).map(|i| setup.source_path(i)).collect();
                    assert!(names.contains(&named), "{named:?}");
                }
                other => panic!("{other}"),
            }
            setup.assert_untouched();
            at += 1;
        }
        // Each file's start and its chunks.
        assert!(at > 12, "{check:?}: {at} places");
        places.push(at);
        fs::remove_dir_all(&setup.into).unwrap();
    }
    // The read-back asks at each of its chunks too.
    assert!(places[1] > places[0], "{places:?}");
}

/// Which step's error a fault test injects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    Read,
    Write,
}

/// A read error (a card pulled out) and a write error (a full disk) at each chunk of each
/// file: each leaves nothing, and names the file, the source's or the destination's.
#[test]
fn a_read_or_write_error_anywhere_leaves_nothing() {
    for fault in [Fault::Read, Fault::Write] {
        let setup = setup();
        let mut at = 0;
        loop {
            let calls = Cell::new(0);
            let failed_at = Cell::new(None);
            let result = setup.copy(Check::ReadBack, &|| false, &mut |step| {
                let index = match (fault, step) {
                    (Fault::Read, At::Read { file }) | (Fault::Write, At::Write { file }) => file,
                    _ => return Ok(()),
                };
                calls.set(calls.get() + 1);
                if calls.get() > at {
                    failed_at.set(Some(index));
                    Err(match fault {
                        Fault::Read => io::ErrorKind::UnexpectedEof.into(),
                        Fault::Write => io::ErrorKind::StorageFull.into(),
                    })
                } else {
                    Ok(())
                }
            });
            let Some(index) = failed_at.get() else {
                result.unwrap();
                break;
            };
            match (fault, result.unwrap_err()) {
                (Fault::Read, CopyError::Read { file, source }) => {
                    assert_eq!(file, setup.source_path(index));
                    assert_eq!(source.kind(), io::ErrorKind::UnexpectedEof);
                }
                (Fault::Write, CopyError::Write { file, source }) => {
                    assert_eq!(file, setup.destination_path(index));
                    assert_eq!(source.kind(), io::ErrorKind::StorageFull);
                }
                (_, other) => panic!("{fault:?}: {other}"),
            }
            setup.assert_untouched();
            at += 1;
        }
        assert!(at > 10, "{fault:?}: {at} places");
    }
}

/// A file of the photo's that appears at the destination after the copy started, before
/// its rename: refused, never replaced, and the photo's files that landed removed again.
#[test]
fn a_file_that_appears_at_the_destination_is_never_replaced() {
    for index in 0..4 {
        let setup = setup();
        let result = setup.copy(Check::ReadBack, &|| false, &mut |step| {
            if let At::Written { file, path, .. } = step
                && file == index
            {
                fs::write(path, b"theirs")?;
            }
            Ok(())
        });
        let path = setup.destination_path(index);
        match result.unwrap_err() {
            CopyError::Exists { file } => assert_eq!(file, path),
            other => panic!("{other}"),
        }
        assert_eq!(fs::read(&path).unwrap(), b"theirs");
        fs::remove_file(&path).unwrap();
        setup.assert_untouched();
    }
}

/// A photo whose file is at the destination already is refused before a byte is copied.
#[test]
fn a_file_there_before_is_refused_and_untouched() {
    let setup = setup();
    let path = setup.destination_path(1);
    fs::write(&path, b"an older edit").unwrap();
    let there = contents(&setup.into);
    let result = setup.copy(Check::None, &|| false, &mut |_| Ok(()));
    match result.unwrap_err() {
        CopyError::Exists { file } => assert_eq!(file, path),
        other => panic!("{other}"),
    }
    assert_eq!(contents(&setup.into), there);
    assert_eq!(contents(&setup.source.folder), setup.before);
}

/// A copy that went wrong on its way to the disk: the read-back finds it, and nothing is
/// left; without the check it would have landed.
#[test]
fn a_copy_that_differs_from_its_source_fails_the_read_back() {
    for index in 0..4 {
        let setup = setup();
        let mut flip = |step: At<'_>| {
            if let At::Written { file, temp, .. } = step
                && file == index
            {
                flip_byte(temp, 1)?;
            }
            Ok(())
        };
        match setup
            .copy(Check::ReadBack, &|| false, &mut flip)
            .unwrap_err()
        {
            CopyError::Mismatch { file } => assert_eq!(file, setup.destination_path(index)),
            other => panic!("{other}"),
        }
        setup.assert_untouched();
        // Unchecked, the same fault lands.
        setup.copy(Check::None, &|| false, &mut flip).unwrap();
        let landed = fs::read(setup.destination_path(index)).unwrap();
        assert_ne!(landed, fs::read(setup.source_path(index)).unwrap());
    }
}

/// A file of 200 MB is streamed in chunks of at most [`CHUNK`], never read whole.
#[test]
fn a_large_file_is_streamed_in_chunks() {
    const LEN: u64 = 200 << 20;
    let root = tempfile::tempdir().unwrap();
    let from = root.path().join("card");
    fs::create_dir(&from).unwrap();
    let raw = from.join("BIG.IIQ");
    let mut file = fs::File::create(&raw).unwrap();
    let block: Vec<u8> = (0..CHUNK).map(|i| (i % 251) as u8).collect();
    let mut source_hash = blake3::Hasher::new();
    for i in 0..(LEN / CHUNK as u64) {
        let mut block = block.clone();
        block[0] = i as u8;
        file.write_all(&block).unwrap();
        source_hash.update(&block);
    }
    drop(file);
    let source = PhotoFiles::find(&from, "BIG.IIQ").unwrap();
    let into = root.path().join("Pictures");
    let mut largest = 0;
    let copied = copy_photo(&source, &into, Check::ReadBack, &|| false, &mut |bytes| {
        largest = largest.max(bytes);
    })
    .unwrap();
    assert!(largest as usize <= CHUNK, "{largest}");
    assert_eq!(copied.files[0].len, LEN);
    assert_eq!(copied.files[0].hash, source_hash.finalize());
}

/// Flips the byte at `offset` of the file at `path`: a copy gone wrong on its way to the
/// disk.
fn flip_byte(path: &Path, offset: u64) -> io::Result<()> {
    let mut file = fs::File::options().read(true).write(true).open(path)?;
    let mut byte = [0u8];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut byte)?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(&[!byte[0]])?;
    file.sync_all()
}
