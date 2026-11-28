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

//! Import's preview on temporary trees, against a fixed import time and capture times
//! that are the raws' modified times, since their bytes hold no EXIF: the folders each
//! Subfolder choice makes, the names taken, the photos there already, a diverged edit,
//! each problem, and nothing made or written.

use std::time::SystemTime;

use apertrust_base::variables::Places;
use chrono::TimeZone;
use tempfile::TempDir;

use super::*;

/// The import's time in the tests: 15 November 2026, 18:30, local.
const NOW: DateTime = DateTime {
    year: 2026,
    month: 11,
    day: 15,
    hour: 18,
    minute: 30,
    second: 0,
    microsecond: 0,
};

/// Noon of a day in November 2026 on the local clock.
fn november(day: u32) -> SystemTime {
    chrono::Local
        .with_ymd_and_hms(2026, 11, day, 12, 0, 0)
        .single()
        .unwrap()
        .into()
}

fn set_modified(path: &Path, time: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// `path` as a folder pattern names it, with forward slashes, which a pattern never takes
/// as an escape.
fn pattern(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A card, `card/DCIM/100CANON`, and a Pictures folder, under one temporary folder.
struct Setup {
    root: TempDir,
}

impl Setup {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("card/DCIM/100CANON")).unwrap();
        fs::create_dir_all(root.path().join("Pictures")).unwrap();
        Setup { root }
    }

    fn dcim(&self) -> PathBuf {
        self.root.path().join("card/DCIM")
    }

    fn apertrust(&self) -> PathBuf {
        self.root.path().join("Pictures/Apertrust")
    }

    /// A raw on the card, of its own bytes, taken on November `day`.
    fn raw(&self, name: &str, bytes: &[u8], day: u32) -> PathBuf {
        let path = self.dcim().join("100CANON").join(name);
        fs::write(&path, bytes).unwrap();
        set_modified(&path, november(day));
        PathBuf::from("100CANON").join(name)
    }

    /// A file at the destination, `path` under Pictures/Apertrust, its folders made.
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

    /// The preview of `files` with `settings`, the card's mount point the card's folder,
    /// `free` bytes free.
    fn preview(&self, files: &[PathBuf], settings: &ImportSettings, free: u64) -> ImportPreview {
        let card = self.root.path().join("card");
        let mount = move |path: &Path| path.starts_with(&card).then(|| card.clone());
        let free_space = move |_: &Path| {
            Some(FreeSpace {
                volume: "Macintosh HD".to_owned(),
                available: free,
                total: free.saturating_mul(2),
            })
        };
        let world = World {
            now: NOW,
            mount: &mount,
            free_space: &free_space,
        };
        let photos = ImportPhotos {
            folder: self.dcim(),
            files: files.to_vec(),
        };
        let fingerprints = Arc::new(Fingerprints::default());
        preview_in(&photos, settings, &world, &fingerprints, &|| false).unwrap()
    }
}

/// Where each photo goes, as the folder it goes into relative to Pictures/Apertrust, and
/// its name there.
fn destinations(setup: &Setup, preview: &ImportPreview) -> Vec<String> {
    (preview.photos.iter())
        .map(|photo| match &photo.fate {
            ImportFate::Copy { to, .. } => {
                let to = to.strip_prefix(setup.apertrust()).unwrap();
                pattern(to)
            }
            other => panic!("{other:?}"),
        })
        .collect()
}

/// Each folder relative to Pictures/Apertrust, its photos and what it holds there.
fn folders(setup: &Setup, preview: &ImportPreview) -> Vec<(String, usize, Option<usize>)> {
    (preview.folders.iter())
        .map(|folder| {
            let name = folder.folder.strip_prefix(setup.apertrust()).unwrap();
            (pattern(name), folder.photos, folder.there)
        })
        .collect()
}

/// Each Subfolder choice for a card of two photos taken on two days, a folder of today's
/// there already holding a raw: the paths and folders each gives, new or there, and the
/// dates it reads; `$(JOBCODE)` the same as `$(IMPORT_NAME)`.
#[test]
fn each_subfolder_choice_gives_its_paths_and_folders() {
    let setup = Setup::new();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 15),
    ];
    setup.there("20261115_Wedding/OTHER.CR2", b"someone's");
    let today = Uses {
        today: true,
        ..Uses::default()
    };
    let taken = Uses {
        taken: true,
        ..Uses::default()
    };
    type Folders = &'static [(&'static str, usize, Option<usize>)];
    let cases: [(&str, [&str; 2], Folders, Uses); 5] = [
        (
            "$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)",
            [
                "20261115_Wedding/IMG_0001.CR2",
                "20261115_Wedding/IMG_0002.CR2",
            ],
            &[("20261115_Wedding", 2, Some(1))],
            today,
        ),
        (
            "$(YEAR)$(MONTH)$(DAY)_$(JOBCODE)",
            [
                "20261115_Wedding/IMG_0001.CR2",
                "20261115_Wedding/IMG_0002.CR2",
            ],
            &[("20261115_Wedding", 2, Some(1))],
            today,
        ),
        (
            "$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)",
            [
                "20261114_Wedding/IMG_0001.CR2",
                "20261115_Wedding/IMG_0002.CR2",
            ],
            &[
                ("20261114_Wedding", 1, None),
                ("20261115_Wedding", 1, Some(1)),
            ],
            taken,
        ),
        (
            "$(IMPORT_NAME)",
            ["Wedding/IMG_0001.CR2", "Wedding/IMG_0002.CR2"],
            &[("Wedding", 2, None)],
            Uses::default(),
        ),
        (
            "",
            ["IMG_0001.CR2", "IMG_0002.CR2"],
            &[("", 2, Some(0))],
            Uses::default(),
        ),
    ];
    for (subfolder, paths, expected, uses) in cases {
        let preview = setup.preview(&files, &setup.settings(subfolder), u64::MAX);
        assert_eq!(destinations(&setup, &preview), paths, "{subfolder}");
        let expected: Vec<_> = (expected.iter())
            .map(|(name, photos, there)| ((*name).to_owned(), *photos, *there))
            .collect();
        assert_eq!(folders(&setup, &preview), expected, "{subfolder}");
        assert_eq!(preview.subfolder_uses, uses, "{subfolder}");
        assert_eq!(preview.folder_uses, Uses::default());
        assert!(preview.problems.is_empty(), "{:?}", preview.problems);
        assert_eq!(preview.size, 11);
    }
}

/// A card spanning two days, imported one folder for each day taken: two new folders.
#[test]
fn a_card_of_two_days_makes_two_new_folders_by_the_day_taken() {
    let setup = Setup::new();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 14),
        setup.raw("IMG_0003.CR2", b"third", 15),
    ];
    let settings = setup.settings("$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)");
    let preview = setup.preview(&files, &settings, u64::MAX);
    assert_eq!(
        folders(&setup, &preview),
        [
            ("20261114_Wedding".to_owned(), 2, None),
            ("20261115_Wedding".to_owned(), 1, None)
        ]
    );
}

/// A Subfolder of the file name reads no photo's EXIF, the listing giving the name; one of
/// the camera reads each photo's.
#[test]
fn only_the_capture_time_or_camera_reads_a_photo_s_exif() {
    for (subfolder, reads) in [("$(FILE_NAME)", 0), ("$(MODEL)", 1)] {
        let setup = Setup::new();
        let files = [setup.raw("IMG_0001.CR2", b"first", 14)];
        setup.preview(&files, &setup.settings(subfolder), u64::MAX);
        let raw = setup.dcim().join(&files[0]);
        assert_eq!(crate::testing::reads(&raw), reads, "{subfolder}");
    }
}

/// A name taken by the raw, by a sidecar alone, and by `_01` already: `_01`, `_01` and
/// `_02`; and two photos of one name in two folders of the card take two names.
#[test]
fn a_name_taken_is_continued_with_01() {
    let setup = Setup::new();
    fs::create_dir_all(setup.dcim().join("101CANON")).unwrap();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 14),
        setup.raw("IMG_0003.CR2", b"third", 14),
        setup.raw("IMG_0004.CR2", b"fourth", 14),
    ];
    let other = setup.dcim().join("101CANON/IMG_0004.CR2");
    fs::write(&other, b"another fourth").unwrap();
    let mut files = files.to_vec();
    files.push(PathBuf::from("101CANON/IMG_0004.CR2"));
    setup.there("IMG_0001.CR2", b"not the first");
    setup.there("IMG_0002.CR2.xmp", b"a sidecar a crash left");
    setup.there("IMG_0003.CR2", b"not the third");
    setup.there("IMG_0003_01.CR2.apxmp", b"left");
    let preview = setup.preview(&files, &setup.settings(""), u64::MAX);
    assert_eq!(
        destinations(&setup, &preview),
        [
            "IMG_0001_01.CR2",
            "IMG_0002_01.CR2",
            "IMG_0003_02.CR2",
            "IMG_0004.CR2",
            "IMG_0004_01.CR2"
        ]
    );
    let counts = preview.counts();
    assert_eq!((counts.copied, counts.renamed), (5, 4));
}

/// A file of a photo's size at its destination, under its name or a `_01` one, is to be
/// skipped if identical and left out of the count of photos copied again, its bytes still
/// in the size, since the copy writes it under `_01` where the files differ; a photo whose
/// copy is elsewhere in the tree is copied again and counted.
#[test]
fn a_photo_there_already_is_skipped_and_not_counted_as_copied_again() {
    let setup = Setup::new();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 14),
        setup.raw("IMG_0003.CR2", b"third", 14),
    ];
    // Its copy where it goes, and one renamed there.
    let at = setup.there("Wedding/IMG_0001.CR2", b"first");
    setup.there("Wedding/IMG_0002.CR2", b"not mine");
    let renamed = setup.there("Wedding/IMG_0002_01.CR2", b"second");
    // Imported before into another folder of the tree.
    setup.there("20261101_Studio/IMG_0003.CR2", b"third");
    let preview = setup.preview(&files, &setup.settings("$(IMPORT_NAME)"), u64::MAX);
    let fates: Vec<&ImportFate> = preview.photos.iter().map(|photo| &photo.fate).collect();
    assert_eq!(fates[0], &ImportFate::There { at });
    assert_eq!(fates[1], &ImportFate::There { at: renamed });
    assert!(matches!(fates[2], ImportFate::Copy { renamed: false, .. }));
    assert_eq!(preview.imported_before, 1);
    assert_eq!(preview.size, 5 + 6 + 5);
}

/// A photo there with an `.apxmp` that differs from the source's is skipped and named; one
/// whose sidecar there is the same, or missing, is there; both found by their size alone,
/// their files' bytes counted.
#[test]
fn a_diverged_edit_is_counted_and_named() {
    let setup = Setup::new();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 14),
    ];
    for name in ["IMG_0001.CR2.apxmp", "IMG_0002.CR2.apxmp"] {
        fs::write(setup.dcim().join("100CANON").join(name), b"the card's edit").unwrap();
    }
    let at = setup.there("IMG_0001.CR2", b"first");
    let sidecar = setup.there("IMG_0001.CR2.apxmp", b"an edit of its own");
    setup.there("IMG_0002.CR2", b"second");
    let preview = setup.preview(&files, &setup.settings(""), u64::MAX);
    assert_eq!(preview.photos[0].fate, ImportFate::Diverged { at, sidecar });
    assert!(matches!(preview.photos[1].fate, ImportFate::There { .. }));
    let counts = preview.counts();
    assert_eq!((counts.diverged, counts.there, counts.copied), (1, 1, 0));
    let edit = "the card's edit".len() as u64;
    assert_eq!(preview.size, (5 + edit) + (6 + edit));
}

/// An unknown variable, with Use $(MONTH) for its fix, and nothing else worked out.
#[test]
fn an_unknown_variable_is_a_problem_with_its_fix() {
    let setup = Setup::new();
    let files = [setup.raw("IMG_0001.CR2", b"first", 14)];
    let preview = setup.preview(&files, &setup.settings("$(YEAR)$(MONT)"), u64::MAX);
    let [problem] = preview.problems.as_slice() else {
        panic!("{:?}", preview.problems);
    };
    assert_eq!(problem.row, Row::Subfolder);
    assert_eq!(problem.fix, Some(Fix::Use { variable: "MONTH" }));
    assert!(matches!(
        &problem.problem,
        Problem::Pattern(PatternError::Unknown { name, .. }) if name == "MONT"
    ));
    assert!(preview.photos.is_empty() && preview.folders.is_empty());
}

/// A Subfolder that leads out of Folder is Subfolder's problem, and nothing else is
/// worked out.
#[test]
fn a_subfolder_leading_out_of_the_folder_is_a_problem() {
    let setup = Setup::new();
    let files = [setup.raw("IMG_0001.CR2", b"first", 14)];
    let preview = setup.preview(&files, &setup.settings("../$(IMPORT_NAME)"), u64::MAX);
    assert_eq!(
        preview.problems,
        [ImportProblem {
            row: Row::Subfolder,
            problem: Problem::OutsideFolder {
                subfolder: "../Wedding".to_owned()
            },
            fix: None,
        }]
    );
    assert!(preview.photos.is_empty() && preview.folders.is_empty());
}

/// A folder under a read-only folder cannot be made: Folder's problem, Choose a folder….
#[cfg(unix)]
#[test]
fn a_folder_under_a_read_only_folder_is_a_problem() {
    use std::os::unix::fs::PermissionsExt;
    let setup = Setup::new();
    let files = [setup.raw("IMG_0001.CR2", b"first", 14)];
    let locked = setup.root.path().join("Pictures");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let preview = setup.preview(&files, &setup.settings("$(IMPORT_NAME)"), u64::MAX);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        preview.problems,
        [ImportProblem {
            row: Row::Folder,
            problem: Problem::Unwritable {
                at: locked,
                why: Unwritable::ReadOnly
            },
            fix: Some(Fix::ChooseFolder),
        }]
    );
}

/// A destination that is the source, lies inside it, or is elsewhere on the card the source
/// is on.
#[test]
fn a_destination_on_the_source_is_a_problem() {
    let setup = Setup::new();
    let files = [setup.raw("IMG_0001.CR2", b"first", 14)];
    let card = setup.root.path().join("card");
    let cases = [
        (setup.dcim(), OnSource::IsSource),
        (setup.dcim().join("Copies"), OnSource::InsideSource),
        (card.join("Backup"), OnSource::OnCard { card: card.clone() }),
    ];
    for (folder, why) in cases {
        let settings = ImportSettings {
            base_folder: pattern(&folder),
            ..setup.settings("")
        };
        let preview = setup.preview(&files, &settings, u64::MAX);
        assert_eq!(
            preview.problems,
            [ImportProblem {
                row: Row::Folder,
                problem: Problem::OnSource {
                    folder: folder.clone(),
                    why: why.clone()
                },
                fix: Some(Fix::ChooseFolder),
            }],
            "{folder:?}"
        );
    }
}

/// Less free space than the photos' bytes: "Needs 11 B; 10 B is free on Macintosh HD."
#[test]
fn too_little_space_is_a_problem() {
    let setup = Setup::new();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 14),
    ];
    let preview = setup.preview(&files, &setup.settings(""), 10);
    assert_eq!(
        preview.problems,
        [ImportProblem {
            row: Row::Folder,
            problem: Problem::Space {
                needed: 11,
                available: 10,
                volume: "Macintosh HD".to_owned(),
            },
            fix: Some(Fix::ChooseFolder),
        }]
    );
    assert_eq!(preview.free.as_ref().map(|free| free.available), Some(10));
    assert!(
        setup
            .preview(&files, &setup.settings(""), 11)
            .problems
            .is_empty()
    );
}

/// Photos going to two volumes: neither's free space is compared with the whole import's
/// bytes, the copy's stop on a full disk standing in.
#[test]
fn photos_going_to_two_volumes_are_not_held_to_one_s_space() {
    let setup = Setup::new();
    let nas = setup.root.path().join("14");
    fs::create_dir_all(&nas).unwrap();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 15),
    ];
    // Folder by the day taken: the 14th's a NAS mounted there, the 15th's on the home
    // volume.
    let settings = ImportSettings {
        base_folder: pattern(setup.root.path()) + "/$(EXIF.DAY)",
        ..setup.settings("")
    };
    let free_space = move |path: &Path| {
        let volume = if path.starts_with(&nas) {
            "NAS"
        } else {
            "Macintosh HD"
        };
        Some(FreeSpace {
            volume: volume.to_owned(),
            available: 1,
            total: 2,
        })
    };
    let world = World {
        now: NOW,
        mount: &|_: &Path| None,
        free_space: &free_space,
    };
    let photos = ImportPhotos {
        folder: setup.dcim(),
        files: files.to_vec(),
    };
    let fingerprints = Arc::new(Fingerprints::default());
    let preview = preview_in(&photos, &settings, &world, &fingerprints, &|| false).unwrap();
    assert_eq!(preview.folders.len(), 2, "{:?}", preview.folders);
    assert_eq!(preview.free, None);
    assert!(preview.problems.is_empty(), "{:?}", preview.problems);
}

/// Everything under `root`, each folder and each file with its bytes and modified time.
fn snapshot(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>, SystemTime)> {
    let mut found = Vec::new();
    let mut queue = vec![root.to_owned()];
    while let Some(folder) = queue.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::metadata(&path).unwrap();
            let modified = meta.modified().unwrap();
            if meta.is_dir() {
                queue.push(path.clone());
                found.push((path, None, modified));
            } else {
                found.push((path.clone(), Some(fs::read(&path).unwrap()), modified));
            }
        }
    }
    found.sort();
    found
}

/// The preview makes and writes nothing, on the source or the destination, whatever it
/// finds there.
#[test]
fn a_preview_makes_and_writes_nothing() {
    let setup = Setup::new();
    let files = [
        setup.raw("IMG_0001.CR2", b"first", 14),
        setup.raw("IMG_0002.CR2", b"second", 15),
    ];
    fs::write(setup.dcim().join("100CANON/IMG_0001.CR2.apxmp"), b"edit").unwrap();
    setup.there("20261114_Wedding/IMG_0001.CR2", b"first");
    let before = snapshot(setup.root.path());
    let settings = setup.settings("$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)");
    let preview = setup.preview(&files, &settings, u64::MAX);
    assert_eq!(preview.folders.len(), 2);
    assert_eq!(snapshot(setup.root.path()), before);
}

/// The event names its command's number, and a cancel sends nothing.
#[test]
fn the_preview_answers_in_its_number_and_a_cancel_sends_nothing() {
    let setup = Setup::new();
    let files = vec![setup.raw("IMG_0001.CR2", b"first", 14)];
    let photos = ImportPhotos {
        folder: setup.dcim(),
        files,
    };
    let settings = setup.settings("");
    let fingerprints = Arc::new(Fingerprints::default());
    let heard = std::sync::Mutex::new(Vec::new());
    let tell = |event| heard.lock().unwrap().push(event);
    let ended = preview_import(4, &photos, &settings, NOW, &fingerprints, &tell, &|| false);
    assert_eq!(ended, Ended::Listed);
    let ended = preview_import(5, &photos, &settings, NOW, &fingerprints, &tell, &|| true);
    assert_eq!(ended, Ended::Cancelled);
    let heard = heard.into_inner().unwrap();
    let [LibraryEvent::ImportPreview { open: 4, preview }] = heard.as_slice() else {
        panic!("{heard:?}");
    };
    assert_eq!(preview.photos.len(), 1);
}
