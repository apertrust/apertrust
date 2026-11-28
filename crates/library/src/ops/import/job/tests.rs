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

//! Import's copy through its body on temporary trees, events in and files out: the photos
//! in path order with their sidecars and times, names taken, copies there already, a
//! diverged edit, a file appearing as a photo lands, the source removed, a full disk, a
//! cancel at every file, and Try again.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use apertrust_base::variables::Places;
use tempfile::TempDir;

use super::*;
use crate::copy::At;
use crate::ops::{Fingerprints, SourceState, Trees};
use crate::volumes::FreeSpace;

const NOW: DateTime = DateTime {
    year: 2026,
    month: 11,
    day: 15,
    hour: 18,
    minute: 30,
    second: 0,
    microsecond: 0,
};

/// A file's bytes and modified time, by its path relative to a tree's root; a folder's
/// bytes are `None`.
type Tree = BTreeMap<PathBuf, (Option<Vec<u8>>, SystemTime)>;

/// Everything under `root`, hidden entries included.
fn tree(root: &Path) -> Tree {
    let mut found = Tree::new();
    let mut queue = vec![root.to_owned()];
    while let Some(folder) = queue.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            let meta = fs::metadata(&path).unwrap();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if meta.is_dir() {
                queue.push(path.clone());
                found.insert(relative, (None, meta.modified().unwrap()));
            } else {
                let bytes = fs::read(&path).unwrap();
                found.insert(relative, (Some(bytes), meta.modified().unwrap()));
            }
        }
    }
    found
}

/// A card of raws, each its own bytes and a modified time long past, and a Pictures folder
/// holding `Apertrust`, there before.
struct Setup {
    root: TempDir,
}

impl Setup {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("card/DCIM/100CANON")).unwrap();
        fs::create_dir_all(root.path().join("Pictures/Apertrust")).unwrap();
        Setup { root }
    }

    fn dcim(&self) -> PathBuf {
        self.root.path().join("card/DCIM")
    }

    fn apertrust(&self) -> PathBuf {
        self.root.path().join("Pictures/Apertrust")
    }

    /// A file on the card, under `100CANON`, its time long past.
    fn card_file(&self, name: &str, bytes: &[u8]) {
        let path = self.dcim().join("100CANON").join(name);
        fs::write(&path, bytes).unwrap();
        let then = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 + bytes.len() as u64);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(then)
            .unwrap();
    }

    /// `count` raws, `IMG_0001.CR2` on, in an order other than their names'.
    fn raws(&self, count: usize) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for i in (1..=count).rev() {
            let name = format!("IMG_{i:04}.CR2");
            self.card_file(
                &name,
                format!("raw number {i}, {}", "x".repeat(i * 100)).as_bytes(),
            );
            files.push(PathBuf::from("100CANON").join(name));
        }
        files
    }

    fn there(&self, path: &str, bytes: &[u8]) -> PathBuf {
        let path = self.apertrust().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    fn settings(&self, subfolder: &str) -> ImportSettings {
        ImportSettings {
            base_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
            default_folder: "$(PICTURES_FOLDER)/Apertrust".to_owned(),
            subfolder: subfolder.to_owned(),
            job_code: "Wedding".to_owned(),
            places: Places {
                home: Some(self.root.path().to_owned()),
                pictures: Some(self.root.path().join("Pictures")),
                desktop: None,
                user_name: "ana".to_owned(),
            },
        }
    }

    /// The import of `files` with `settings`, its events, and how it ended: each file landed
    /// through the copy with `hook`, told the photo's place, `before` each photo.
    fn import(
        &self,
        files: &[PathBuf],
        settings: &ImportSettings,
        hook: &mut dyn FnMut(usize, At<'_>) -> io::Result<()>,
        before: &dyn Fn(usize),
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> (Vec<ImportedPhoto>, ImportEnd) {
        let card = self.root.path().join("card");
        let mount = move |path: &Path| path.starts_with(&card).then(|| card.clone());
        let world = World {
            now: NOW,
            mount: &mount,
            free_space: &|_: &Path| None::<FreeSpace>,
        };
        let photo = AtomicUsize::new(0);
        let at = |index: usize| photo.store(index, Ordering::Relaxed);
        let before = |index: usize| {
            at(index);
            before(index);
        };
        let mut land = |source: &PhotoFiles, landing: &Landing<'_>, made: &mut Vec<PathBuf>| {
            let index = photo.load(Ordering::Relaxed);
            let mut hook = |step: At<'_>| hook(index, step);
            let how = (Check::None, 4096);
            copy::land_with(source, landing, how, cancel, &mut |_| {}, made, &mut hook)
        };
        let how = Lands {
            world: &world,
            land: &mut land,
            before: &before,
        };
        let heard = Mutex::new(Vec::new());
        let photos = ImportPhotos {
            folder: self.dcim(),
            files: files.to_vec(),
        };
        let end = import_in(
            3,
            &photos,
            settings,
            "test",
            how,
            &|event| heard.lock().unwrap().push(event),
            &|_| {},
            cancel,
        );
        let photos = (heard.into_inner().unwrap().into_iter())
            .flat_map(|event| match event {
                LibraryEvent::Imported { open: 3, photos } => photos,
                other => panic!("{other:?}"),
            })
            .collect();
        (photos, end)
    }

    /// The import with no fault.
    fn plain(
        &self,
        files: &[PathBuf],
        settings: &ImportSettings,
    ) -> (Vec<ImportedPhoto>, ImportEnd) {
        self.import(files, settings, &mut |_, _| Ok(()), &|_| {}, &|| false)
    }
}

fn names(photos: &[ImportedPhoto]) -> Vec<String> {
    (photos.iter())
        .map(|photo| {
            photo
                .source
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn copy_of(photo: &ImportedPhoto) -> &Path {
    match &photo.result {
        ImportResult::Copied { copy } | ImportResult::There { copy } => copy,
        other => panic!("{other:?}"),
    }
}

/// No hidden temporary is left anywhere under `root`.
fn assert_no_temporary(root: &Path) {
    for path in tree(root).keys() {
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(!apertrust_base::fs::is_temporary_name(&name), "{path:?}");
    }
}

/// Five raws, two with an `.apxmp` and darktable's `.xmp`: all copied in the order of their
/// paths, the sidecars carried, the modified times kept, the source as it was.
#[test]
fn the_photos_are_copied_in_path_order_with_their_sidecars_and_times() {
    let setup = Setup::new();
    let files = setup.raws(5);
    for i in [2, 4] {
        setup.card_file(&format!("IMG_000{i}.CR2.apxmp"), b"<edit/>");
        setup.card_file(&format!("IMG_000{i}.CR2.xmp"), b"<darktable/>");
    }
    let card = tree(&setup.dcim());
    let (photos, end) = setup.plain(&files, &setup.settings("$(IMPORT_NAME)"));
    assert_eq!(
        names(&photos),
        [
            "IMG_0001.CR2",
            "IMG_0002.CR2",
            "IMG_0003.CR2",
            "IMG_0004.CR2",
            "IMG_0005.CR2"
        ]
    );
    assert_eq!((end.copied, end.failed, end.not_reached), (5, 0, 0));
    assert!(end.left.is_empty() && end.landed);
    let wedding = setup.apertrust().join("Wedding");
    assert_eq!(end.destination.as_deref(), Some(wedding.as_path()));
    let copied = tree(&wedding);
    let source = tree(&setup.dcim().join("100CANON"));
    assert_eq!(copied, source, "the same names, bytes and times");
    assert_eq!(tree(&setup.dcim()), card, "the source as it was");
    assert_no_temporary(setup.root.path());
}

/// A name taken: `_01`, then `_02`, each `.apxmp` keeping the camera name, the source's edit
/// with it, or a new one holding it alone.
#[test]
fn a_name_taken_is_continued_and_the_camera_name_kept() {
    let setup = Setup::new();
    let files = setup.raws(2);
    let mut edits = crate::sidecar::Edits::new("0.8.0");
    edits.photo = Some("IMG_0002.CR2".to_owned());
    setup.card_file("IMG_0002.CR2.apxmp", edits.to_xml().as_bytes());
    setup.there("IMG_0001.CR2", b"not the first");
    setup.there("IMG_0002.CR2", b"not the second");
    setup.there("IMG_0002_01.CR2.xmp", b"a sidecar a crash left");
    let (photos, end) = setup.plain(&files, &setup.settings(""));
    assert_eq!(end.copied, 2);
    let copies: Vec<&Path> = photos.iter().map(copy_of).collect();
    assert_eq!(
        copies,
        [
            setup.apertrust().join("IMG_0001_01.CR2"),
            setup.apertrust().join("IMG_0002_02.CR2")
        ]
    );
    for (copy, camera) in copies.iter().zip(["IMG_0001.CR2", "IMG_0002.CR2"]) {
        let edits = crate::sidecar::read_edits(&crate::sidecar::path_for(copy)).unwrap();
        assert_eq!(edits.preserved_file_name().as_deref(), Some(camera));
        let name = copy.file_name().unwrap().to_str().unwrap();
        assert_eq!(edits.photo.as_deref(), Some(name));
    }
    assert_eq!(
        fs::read(setup.apertrust().join("IMG_0002_01.CR2.xmp")).unwrap(),
        b"a sidecar a crash left"
    );
}

/// A copy there byte for byte is skipped, its missing sidecar copied alone beside it; one
/// of the same size whose bytes differ is no copy, and the photo lands as `_01`.
#[test]
fn a_duplicate_there_is_skipped_and_its_missing_sidecar_copied() {
    let setup = Setup::new();
    let files = setup.raws(2);
    setup.card_file("IMG_0001.CR2.xmp", b"<darktable/>");
    let first = fs::read(setup.dcim().join("100CANON/IMG_0001.CR2")).unwrap();
    let mut second = fs::read(setup.dcim().join("100CANON/IMG_0002.CR2")).unwrap();
    let at = setup.there("IMG_0001.CR2", &first);
    *second.last_mut().unwrap() ^= 1;
    setup.there("IMG_0002.CR2", &second);
    let (photos, end) = setup.plain(&files, &setup.settings(""));
    assert_eq!((end.there, end.copied), (1, 1));
    assert_eq!(photos[0].result, ImportResult::There { copy: at.clone() });
    assert_eq!(fs::read(&at).unwrap(), first);
    assert_eq!(
        fs::read(setup.apertrust().join("IMG_0001.CR2.xmp")).unwrap(),
        b"<darktable/>"
    );
    assert_eq!(
        copy_of(&photos[1]),
        setup.apertrust().join("IMG_0002_01.CR2")
    );
}

/// A diverged `.apxmp` there: the photo is skipped, nothing written, and named.
#[test]
fn a_diverged_edit_is_skipped_and_named_with_nothing_written() {
    let setup = Setup::new();
    let files = setup.raws(1);
    setup.card_file("IMG_0001.CR2.apxmp", b"the card's edit");
    setup.card_file("IMG_0001.CR2.xmp", b"<darktable/>");
    let raw = fs::read(setup.dcim().join("100CANON/IMG_0001.CR2")).unwrap();
    let copy = setup.there("IMG_0001.CR2", &raw);
    let sidecar = setup.there("IMG_0001.CR2.apxmp", b"an edit of its own");
    let before = tree(&setup.apertrust());
    let (photos, end) = setup.plain(&files, &setup.settings(""));
    assert_eq!(photos[0].result, ImportResult::Diverged { copy, sidecar });
    assert_eq!((end.diverged, end.copied, end.landed), (1, 0, false));
    assert_eq!(tree(&setup.apertrust()), before);
}

/// A file appearing at the destination as a photo lands: that photo fails alone, the file
/// there untouched, the others copied, and Try again copies it alone, once.
#[test]
fn a_file_appearing_fails_its_photo_alone_and_try_again_copies_what_is_left() {
    let setup = Setup::new();
    let files = setup.raws(3);
    let settings = setup.settings("");
    let theirs = setup.apertrust().join("IMG_0002.CR2");
    let (photos, end) = setup.import(
        &files,
        &settings,
        &mut |photo, step| {
            if photo == 1
                && let At::Written { path, .. } = step
            {
                fs::write(path, b"theirs")?;
            }
            Ok(())
        },
        &|_| {},
        &|| false,
    );
    assert!(matches!(
        photos[1].result,
        ImportResult::Failed {
            cause: ImportCause::Exists,
            ..
        }
    ));
    assert_eq!((end.copied, end.failed, end.not_reached), (2, 1, 0));
    assert_eq!(end.left, [PathBuf::from("100CANON/IMG_0002.CR2")]);
    assert_eq!(
        end.failure.as_ref().map(|f| f.cause),
        Some(ImportCause::Exists)
    );
    assert_eq!(fs::read(&theirs).unwrap(), b"theirs");
    assert_no_temporary(setup.root.path());

    let before = tree(&setup.apertrust());
    let (photos, end) = setup.plain(&end.left, &settings);
    assert_eq!((photos.len(), end.copied), (1, 1));
    assert_eq!(
        copy_of(&photos[0]),
        setup.apertrust().join("IMG_0002_01.CR2")
    );
    let after = tree(&setup.apertrust());
    let added: Vec<&PathBuf> = after
        .keys()
        .filter(|path| !before.contains_key(*path))
        .collect();
    assert_eq!(
        added,
        [
            &PathBuf::from("IMG_0002_01.CR2"),
            &PathBuf::from("IMG_0002_01.CR2.apxmp")
        ]
    );
}

/// The source removed mid-job: the job stops, the copies made stay, no temporary, and what
/// is left is offered again.
#[test]
fn the_source_removed_stops_the_job_and_keeps_the_copies() {
    let setup = Setup::new();
    let files = setup.raws(4);
    let card = setup.dcim();
    let (photos, end) = setup.import(
        &files,
        &setup.settings("$(IMPORT_NAME)"),
        &mut |_, _| Ok(()),
        &|photo| {
            if photo == 2 {
                fs::remove_dir_all(&card).unwrap();
            }
        },
        &|| false,
    );
    assert_eq!(
        names(&photos),
        ["IMG_0001.CR2", "IMG_0002.CR2", "IMG_0003.CR2"]
    );
    assert_eq!(
        end.failure.as_ref().map(|f| f.cause),
        Some(ImportCause::Gone)
    );
    assert_eq!((end.copied, end.failed, end.not_reached), (2, 1, 1));
    assert_eq!(end.left.len(), 2);
    let wedding = setup.apertrust().join("Wedding");
    let landed: Vec<PathBuf> = tree(&wedding).into_keys().collect();
    assert_eq!(
        landed,
        [PathBuf::from("IMG_0001.CR2"), PathBuf::from("IMG_0002.CR2")]
    );
    assert_no_temporary(setup.root.path());
}

/// A full disk at the third photo stops the job, the third absent whole, its sidecars too.
#[test]
fn a_full_disk_stops_the_job_with_the_photo_absent_whole() {
    let setup = Setup::new();
    let files = setup.raws(4);
    setup.card_file("IMG_0003.CR2.apxmp", b"<edit/>");
    let (_, end) = setup.import(
        &files,
        &setup.settings(""),
        &mut |photo, step| match step {
            At::Write { file: 1, .. } if photo == 2 => Err(io::ErrorKind::StorageFull.into()),
            _ => Ok(()),
        },
        &|_| {},
        &|| false,
    );
    assert_eq!(
        end.failure.as_ref().map(|f| f.cause),
        Some(ImportCause::DiskFull)
    );
    assert_eq!((end.copied, end.failed, end.not_reached), (2, 1, 1));
    let landed: Vec<PathBuf> = tree(&setup.apertrust()).into_keys().collect();
    assert_eq!(
        landed,
        [PathBuf::from("IMG_0001.CR2"), PathBuf::from("IMG_0002.CR2")]
    );
    assert_no_temporary(setup.root.path());
}

/// A cancel at every place a three-photo job asks for one: no temporary, every photo
/// whole or absent, a folder the import made and left empty removed, the folder there
/// before kept, and nothing offered again.
#[test]
fn a_cancel_anywhere_leaves_whole_photos_and_no_empty_folder_made() {
    let setup = Setup::new();
    let files = setup.raws(3);
    setup.card_file("IMG_0002.CR2.apxmp", b"<edit/>");
    let settings = setup.settings("$(IMPORT_NAME)/Day");
    let wedding = setup.apertrust().join("Wedding");
    let mut at = 0;
    loop {
        let asked = AtomicUsize::new(0);
        let cancel = || asked.fetch_add(1, Ordering::Relaxed) >= at;
        let (_, end) = setup.import(&files, &settings, &mut |_, _| Ok(()), &|_| {}, &cancel);
        assert_no_temporary(setup.root.path());
        assert!(
            setup.apertrust().is_dir(),
            "the folder there before is kept"
        );
        let landed = tree(&setup.apertrust());
        if end.copied == 0 {
            assert!(!wedding.exists(), "{at}: {landed:?}");
        }
        let day = wedding.join("Day");
        let raws = (1..=3)
            .filter(|i| day.join(format!("IMG_000{i}.CR2")).exists())
            .count();
        assert_eq!(raws, end.copied, "{at}");
        assert_eq!(
            day.join("IMG_0002.CR2.apxmp").exists(),
            day.join("IMG_0002.CR2").exists(),
            "{at}: a photo whole or absent"
        );
        if !end.cancelled {
            assert_eq!(end.copied, 3);
            break;
        }
        assert!(end.left.is_empty());
        if wedding.exists() {
            fs::remove_dir_all(&wedding).unwrap();
        }
        at += 1;
    }
    assert!(at > 6, "{at} places");
}

/// A cancel after a photo failed offers nothing again, that photo included.
#[test]
fn a_cancel_after_a_failure_leaves_nothing_to_try_again() {
    let setup = Setup::new();
    let files = setup.raws(3);
    let failed = AtomicUsize::new(0);
    let (_, end) = setup.import(
        &files,
        &setup.settings(""),
        &mut |photo, step| match step {
            At::Write { .. } if photo == 0 => {
                failed.store(1, Ordering::Relaxed);
                Err(io::ErrorKind::Other.into())
            }
            _ => Ok(()),
        },
        &|_| {},
        &|| failed.load(Ordering::Relaxed) == 1,
    );
    assert!(end.cancelled && end.failed == 1, "{end:?}");
    assert!(end.left.is_empty(), "{:?}", end.left);
}

/// A name taken whose `_01` name's sidecar is longer than the system allows fails that
/// photo alone, as the destination's, never trying every `_01` name in turn.
#[cfg(unix)]
#[test]
fn a_name_the_system_cannot_look_up_fails_its_photo_alone() {
    let setup = Setup::new();
    let mut files = setup.raws(1);
    // 248 bytes and `.apdata` fit a name, 255 bytes; with `_01` they do not.
    let long = format!("{}.CR2", "L".repeat(244));
    setup.card_file(&long, b"a raw with a long name");
    setup.there(&long, b"another photo");
    files.push(PathBuf::from("100CANON").join(&long));
    let (photos, end) = setup.plain(&files, &setup.settings(""));
    assert_eq!(
        (end.copied, end.failed, end.not_reached),
        (1, 1, 0),
        "{photos:?}"
    );
    assert!(
        matches!(
            photos[1].result,
            ImportResult::Failed {
                cause: ImportCause::Write,
                ..
            }
        ),
        "{photos:?}"
    );
    assert_eq!(
        fs::read(setup.apertrust().join(&long)).unwrap(),
        b"another photo"
    );
}

/// The check for photos imported before finds a copy named `_01` by the camera name its
/// `.apxmp` keeps.
#[test]
fn a_renamed_copy_is_found_imported_before() {
    let setup = Setup::new();
    let files = setup.raws(1);
    setup.there("IMG_0001.CR2", b"not the first");
    let (photos, _) = setup.plain(&files, &setup.settings(""));
    let copy = copy_of(&photos[0]).to_owned();
    assert!(copy.ends_with("IMG_0001_01.CR2"));
    let photos = ImportPhotos {
        folder: setup.dcim(),
        files,
    };
    let listed = preview::listed(&photos, &photos.files);
    let settings = setup.settings("");
    let trees = Trees {
        base_folder: settings.base_folder,
        default_folder: settings.default_folder,
        places: settings.places,
    };
    let fingerprints = Arc::new(Fingerprints::default());
    let states = preview::check(&listed, &trees, &fingerprints, &|| false).unwrap();
    assert_eq!(states, [SourceState::ImportedBefore { copy }]);
}

/// A destination on the source, or a pattern that cannot be read, refuses the import before
/// any photo, nothing written and nothing offered again.
#[test]
fn a_destination_on_the_source_or_a_bad_pattern_is_refused() {
    let setup = Setup::new();
    let files = setup.raws(1);
    let card = tree(&setup.dcim());
    let inside = ImportSettings {
        base_folder: setup.dcim().to_string_lossy().replace('\\', "/"),
        ..setup.settings("Copies")
    };
    let relative = ImportSettings {
        base_folder: "Pictures/Apertrust".to_owned(),
        ..setup.settings("")
    };
    let dcim = setup.dcim();
    let (parent, name) = (dcim.parent().unwrap(), dcim.file_name().unwrap());
    let through = ImportSettings {
        base_folder: parent
            .join("missing/..")
            .join(name)
            .join("Copies")
            .to_string_lossy()
            .replace('\\', "/"),
        ..setup.settings("")
    };
    let cases = [
        (inside, ImportCause::OnSource),
        (through, ImportCause::Folder),
        (setup.settings("$(MONT)"), ImportCause::Pattern),
        (setup.settings("../$(IMPORT_NAME)"), ImportCause::Pattern),
        (relative, ImportCause::Folder),
    ];
    for (settings, cause) in cases {
        let (photos, end) = setup.plain(&files, &settings);
        assert!(photos.is_empty());
        assert_eq!(
            end.refused.as_ref().map(|r| r.cause),
            Some(cause),
            "{end:?}"
        );
        assert!(end.left.is_empty());
        assert_eq!(end.not_reached, 1);
    }
    assert_eq!(tree(&setup.dcim()), card);
    assert!(!Path::new("Pictures/Apertrust").exists());
}

/// A destination unplugged as photos land: a write error after which the deepest folder of
/// the destination there as the import started can no longer be listed stops the job, gone,
/// though a folder above it, as `/Volumes`, still can; the copies made stay.
#[test]
fn a_destination_unplugged_stops_the_job() {
    let setup = Setup::new();
    let files = setup.raws(3);
    let volume = setup.root.path().join("NAS");
    fs::create_dir_all(volume.join("Photos")).unwrap();
    let settings = ImportSettings {
        base_folder: volume.join("Photos").to_string_lossy().replace('\\', "/"),
        ..setup.settings("$(IMPORT_NAME)")
    };
    let gone = volume.clone();
    let (photos, end) = setup.import(
        &files,
        &settings,
        &mut |photo, step| match step {
            At::Write { .. } if photo == 1 => {
                let _ = fs::remove_dir_all(&gone);
                Err(io::ErrorKind::Other.into())
            }
            _ => Ok(()),
        },
        &|_| {},
        &|| false,
    );
    assert_eq!(names(&photos), ["IMG_0001.CR2", "IMG_0002.CR2"]);
    assert_eq!(
        end.failure.as_ref().map(|f| f.cause),
        Some(ImportCause::Gone)
    );
    assert_eq!((end.copied, end.failed, end.not_reached), (1, 1, 1));
    assert_eq!(end.left.len(), 2);
}

/// The deepest folder of the destination there as the import started gone between two
/// photos, its disk unplugged, its mount point's folder left: the job stops, gone, before
/// the next photo's folders are made where the path now leads.
#[test]
fn a_destination_gone_between_photos_stops_the_job_with_nothing_made() {
    let setup = Setup::new();
    let files = setup.raws(3);
    let volume = setup.root.path().join("NAS");
    fs::create_dir_all(volume.join("Photos")).unwrap();
    let settings = ImportSettings {
        base_folder: volume.join("Photos").to_string_lossy().replace('\\', "/"),
        ..setup.settings("$(IMPORT_NAME)")
    };
    let gone = volume.join("Photos");
    let (photos, end) = setup.import(
        &files,
        &settings,
        &mut |_, _| Ok(()),
        &|photo| {
            if photo == 1 {
                fs::remove_dir_all(&gone).unwrap();
            }
        },
        &|| false,
    );
    assert_eq!(names(&photos), ["IMG_0001.CR2", "IMG_0002.CR2"]);
    assert_eq!(
        end.failure.as_ref().map(|f| f.cause),
        Some(ImportCause::Gone)
    );
    assert_eq!((end.copied, end.failed, end.not_reached), (1, 1, 1));
    assert!(
        volume.is_dir() && !gone.exists(),
        "nothing made for the next photo"
    );
}

/// A write error on a destination still there fails that photo alone.
#[test]
fn a_write_error_with_the_destination_there_fails_the_photo_alone() {
    let setup = Setup::new();
    let files = setup.raws(3);
    let (_, end) = setup.import(
        &files,
        &setup.settings(""),
        &mut |photo, step| match step {
            At::Write { .. } if photo == 1 => Err(io::ErrorKind::Other.into()),
            _ => Ok(()),
        },
        &|_| {},
        &|| false,
    );
    assert_eq!(
        end.failure.as_ref().map(|f| f.cause),
        Some(ImportCause::Write)
    );
    assert_eq!((end.copied, end.failed, end.not_reached), (2, 1, 0));
}

/// A renamed photo whose `.apxmp` is no XMP, not even text, still copies, its sidecar as
/// it is.
#[test]
fn a_renamed_photo_with_a_sidecar_that_is_no_xmp_still_copies() {
    let setup = Setup::new();
    let files = setup.raws(1);
    let junk = b"\xc3\x28 no XMP \xff";
    setup.card_file("IMG_0001.CR2.apxmp", junk);
    setup.there("IMG_0001.CR2", b"not the first");
    let (photos, end) = setup.plain(&files, &setup.settings(""));
    assert_eq!(end.copied, 1, "{photos:?}");
    let copy = setup.apertrust().join("IMG_0001_01.CR2.apxmp");
    assert_eq!(fs::read(copy).unwrap(), junk);
}
