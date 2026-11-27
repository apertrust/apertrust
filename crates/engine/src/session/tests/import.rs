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

//! What the Import sheet reads through the session before a photo is copied (decision 85):
//! the source's listing, a job with no progress in its command's number, whatever folder
//! is open, which the next listing or a `CancelListing` stops; and its checks against the
//! destination, each numbered within its listing.

use super::*;

/// `folder` as a folder pattern names it, with forward slashes, which a pattern never
/// takes as an escape.
fn pattern(folder: &Path) -> String {
    folder.to_string_lossy().replace('\\', "/")
}

/// Lists `source` for Import, checked against the tree of `destination`.
fn list_against(source: &Path, destination: &Path) -> Command {
    Command::Library(LibraryCommand::ListSource {
        source: source.to_owned(),
        include_subfolders: false,
        base_folder: pattern(destination),
        default_folder: pattern(destination),
        places: Places::default(),
    })
}

/// Lists `source` for Import, checked against a tree that is not there.
fn list_source(source: &Path) -> Command {
    list_against(source, &source.join("not there"))
}

/// Checks the listing numbered `listing` again, against the tree of `destination`.
fn check_against(listing: u64, destination: &Path) -> Command {
    Command::Library(LibraryCommand::CheckSource {
        listing,
        base_folder: pattern(destination),
        default_folder: pattern(destination),
        places: Places::default(),
    })
}

/// Whether `event` is the last chunk of check `check` of the listing numbered `listing`.
fn checked(listing: u64, check: u64) -> impl Fn(&Event) -> bool {
    move |e| {
        matches!(e, Event::Library(LibraryEvent::SourceChecked { open, check: c, done: true, .. })
            if *open == listing && *c == check)
    }
}

/// Each check in `events` of the listing numbered `listing`, by its number, with the trees
/// it walked and its states.
fn checks(events: &[Event], listing: u64) -> Vec<(u64, Vec<PathBuf>, Vec<library::SourceState>)> {
    (library_of(events, listing).into_iter())
        .filter_map(|event| match event {
            LibraryEvent::SourceChecked {
                check,
                folders,
                states,
                ..
            } => Some((*check, folders.clone(), states.clone())),
            _ => None,
        })
        .collect()
}

/// Whether `event` is the capture times of the listing numbered `listing`, its last.
fn taken(listing: u64) -> impl Fn(&Event) -> bool {
    move |e| matches!(e, Event::Library(LibraryEvent::SourceTaken { open, .. }) if *open == listing)
}

/// A source's listing comes in its command's number, its raws then their capture times,
/// with no folder open; its job has no progress, so no job event shows it in the spinner
/// or the jobs window. What follows the listing, its check, may come in the same batch.
#[test]
fn a_source_is_listed_by_a_job_with_no_progress_in_its_number() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-source-listed");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let listing = session.send(list_source(&folder));
    let events = until(&session, &wakes, taken(listing));
    let library: Vec<&LibraryEvent> = (library_of(&events, listing).into_iter())
        .filter(|event| {
            matches!(
                event,
                LibraryEvent::SourceListed { .. } | LibraryEvent::SourceTaken { .. }
            )
        })
        .collect();
    let [
        LibraryEvent::SourceListed {
            folder: listed,
            files,
            done: true,
            ..
        },
        LibraryEvent::SourceTaken { shots, .. },
    ] = library.as_slice()
    else {
        panic!("{library:?}");
    };
    assert_eq!(listed, &folder);
    let names: Vec<&Path> = files.iter().map(|file| file.path.as_path()).collect();
    assert_eq!(names, [Path::new("A.ARW"), Path::new("B.ARW")]);
    assert_eq!(shots.len(), 2);
    assert!(of_job(&events, listing).is_none(), "{events:?}");
    drop(session);
    remove(&path);
}

/// A second listing and a `CancelListing` each stop the listing before them: nothing of
/// the first comes once it is cancelled, though its job was under way. The held job is let
/// go only once the next listing has answered, so the engine has applied the cancel.
#[test]
fn a_new_listing_or_a_cancel_stops_the_one_before() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-source-cancel");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    for cancel in [false, true] {
        let (hold, begun, permit) = export_job::testing::Hold::new();
        session.send(Command::HoldFolder(hold));
        let first = session.send(list_source(&folder));
        begun.recv_timeout(TEST_WAIT).expect("the listing begins");
        if cancel {
            let listing = first;
            session.send(Command::Library(LibraryCommand::CancelListing { listing }));
        }
        // The next listing, after the cancel; its answer comes only once both are applied,
        // the held one ending as its cancel comes.
        let next = session.send(list_source(&folder));
        let events = until(&session, &wakes, taken(next));
        drop(permit);
        assert!(
            library_of(&events, first).is_empty(),
            "cancel {cancel}: {events:?}"
        );
        // A quiet while for anything of the first still on its way.
        let quiet = Instant::now() + Duration::from_millis(200);
        while Instant::now() < quiet {
            let _ = wakes.recv_timeout(Duration::from_millis(20));
            let late = session.events();
            assert!(
                library_of(&late, first).is_empty(),
                "cancel {cancel}: {late:?}"
            );
        }
    }
    drop(session);
    remove(&path);
}

/// A `CancelListing` naming another listing than the latest leaves the latest alone.
#[test]
fn a_cancel_of_an_earlier_listing_leaves_the_latest() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-source-earlier");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold));
    let latest = session.send(list_source(&folder));
    begun.recv_timeout(TEST_WAIT).expect("the listing begins");
    let listing = latest - 1;
    session.send(Command::Library(LibraryCommand::CancelListing { listing }));
    drop(permit);
    let events = until(&session, &wakes, taken(latest));
    assert!(!library_of(&events, latest).is_empty());
    drop(session);
    remove(&path);
}

/// The listing ends with its own check, number 1, against its trees; a `CheckSource` that
/// comes while it lists replaces it, its number the next, and one after it is a job of its
/// own; each names the trees it walked, and one of an earlier listing is not checked.
#[test]
fn each_check_of_a_listing_is_numbered_and_names_its_trees() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-source-checks");
    let destination = folder.join("Day 1");
    std::fs::copy(folder.join("A.ARW"), destination.join("A.ARW")).unwrap();
    let elsewhere = folder.join("not there");
    let (session, wakes) = start_with(&gpu, &[], synthetic);

    // Its own check.
    let listing = session.send(list_against(&folder, &destination));
    let events = until(&session, &wakes, checked(listing, 1));
    let new = library::SourceState::New;
    let before = library::SourceState::ImportedBefore {
        copy: PathBuf::from(pattern(&destination)).join("A.ARW"),
    };
    let trees = vec![PathBuf::from(pattern(&destination))];
    assert_eq!(
        checks(&events, listing),
        [(1, trees.clone(), vec![before.clone(), new.clone()])]
    );
    assert!(of_job(&events, listing).is_none(), "{events:?}");

    // Asked again once listed: its next check, a job of its own.
    session.send(check_against(listing, &elsewhere));
    let events = until(&session, &wakes, checked(listing, 2));
    let elsewhere_trees = vec![PathBuf::from(pattern(&elsewhere))];
    assert_eq!(
        checks(&events, listing),
        [(2, elsewhere_trees, vec![new.clone(), new.clone()])]
    );

    // Asked while it lists: the check it ends with, the listing's own never sent.
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold));
    let held = session.send(list_against(&folder, &elsewhere));
    begun.recv_timeout(TEST_WAIT).expect("the listing begins");
    session.send(check_against(held, &destination));
    // One of the listing before it: not checked.
    session.send(check_against(listing, &elsewhere));
    // A close, answered on the engine thread, comes once both checks are applied: only
    // then is the held listing let go.
    let close = session.send(Command::Library(LibraryCommand::Close));
    let closed =
        |e: &Event| matches!(e, Event::Library(LibraryEvent::Closed { open }) if *open == close);
    let mut events = until(&session, &wakes, closed);
    drop(permit);
    events.extend(until(&session, &wakes, checked(held, 2)));
    assert_eq!(checks(&events, held), [(2, trees, vec![before, new])]);
    assert!(checks(&events, listing).is_empty(), "{events:?}");
    drop(session);
    remove(&path);
}

/// A source of two raws of their own bytes, `x.ARW` and `y.ARW`, a raw outside it, and a
/// disk cache holding x's embedded picture by its fingerprint, as a thumbnail job keeps
/// it: the source, the cache and the scratch path.
fn source_with_cache(name: &str) -> (PathBuf, CacheOptions, PathBuf) {
    let path = scratch(name);
    let root = path.parent().unwrap().to_owned();
    let source = root.join("card");
    std::fs::create_dir(&source).unwrap();
    for raw in ["x.ARW", "y.ARW"] {
        std::fs::write(source.join(raw), format!("no raw: {raw}")).unwrap();
    }
    std::fs::write(root.join("outside.ARW"), b"no raw outside").unwrap();
    let cache = Arc::new(apertrust_base::cache::Cache::create(root.join("cache")).unwrap());
    let x = source.join("x.ARW");
    let state = library::raw_state(&x).expect("x's state");
    let fingerprints = library::Fingerprints::default();
    let identity = library::Identity::read(&x, state, &fingerprints).expect("x's identity");
    let key = library::thumbnail_key(&identity, 256);
    let picture = library::Thumbnail {
        width: 4,
        height: 3,
        pixels: vec![200; 48].into(),
    };
    let jpeg = thumbnail::encode(&picture).expect("a JPEG");
    (cache.write(
        apertrust_base::cache::THUMBNAILS,
        &key.slot,
        &key.version,
        &jpeg,
    ))
    .unwrap();
    let options = CacheOptions {
        cache,
        limit: apertrust_base::cache::Limit::Unlimited,
    };
    (source, options, path)
}

/// The thumbnails among `events` in the number `open`, by photo, with their pictures'
/// sizes.
fn thumbnails_of(events: &[Event], open: u64) -> Vec<(String, Option<(u32, u32)>)> {
    (library_of(events, open).into_iter())
        .filter_map(|event| match event {
            LibraryEvent::Thumbnail { photo, image, .. } => Some((
                photo.clone(),
                image.as_ref().map(|image| (image.width, image.height)),
            )),
            _ => None,
        })
        .collect()
}

/// A listing's photos' thumbnails come in its number, each named by the path asked, from
/// the cache by fingerprint or read from its file, with no folder open; a path outside the
/// source is refused and reads nothing. Once the photo's copy lands in a folder opened,
/// the folder's thumbnail of it comes from the same entry, its embedded picture not read.
#[test]
fn a_source_s_thumbnails_come_by_its_paths_and_are_kept_for_the_copies() {
    let gpu = gpu();
    let (source, cache, path) = source_with_cache("import-source-thumbnails");
    let root = source.parent().unwrap().to_owned();
    let on_source = || {
        let mut files: Vec<(PathBuf, Vec<u8>)> = (std::fs::read_dir(&source).unwrap())
            .map(|entry| entry.unwrap().path())
            .map(|path| (path.clone(), std::fs::read(&path).unwrap()))
            .collect();
        files.sort();
        files
    };
    let unchanged = on_source();
    let (session, wakes) = spawned(&gpu, &[], Some(cache), loading(synthetic));
    let listing = session.send(list_source(&source));
    until(&session, &wakes, taken(listing));
    let x = source.join("x.ARW");
    let read_before = apertrust_library::testing::reads(&x);
    let asked = ["x.ARW", "y.ARW", "../outside.ARW"].map(PathBuf::from);
    let mut photos = asked.to_vec();
    photos.push(root.join("outside.ARW"));
    session.send(Command::Library(LibraryCommand::SourceThumbnails {
        listing,
        photos,
        size: 256,
    }));
    let two = |e: &Event| matches!(e, Event::Library(LibraryEvent::Thumbnail { photo, .. }) if photo == "y.ARW");
    let mut events = until(&session, &wakes, two);
    if thumbnails_of(&events, listing).len() < 2 {
        let x = |e: &Event| matches!(e, Event::Library(LibraryEvent::Thumbnail { photo, .. }) if photo == "x.ARW");
        events.extend(until(&session, &wakes, x));
    }
    let mut told = thumbnails_of(&events, listing);
    told.sort();
    assert_eq!(
        told,
        [
            ("x.ARW".to_owned(), Some((4, 3))),
            ("y.ARW".to_owned(), None)
        ]
    );
    assert_eq!(
        apertrust_library::testing::reads(&x) - read_before,
        1,
        "its fingerprint alone"
    );
    assert_eq!(
        apertrust_library::testing::reads(&root.join("outside.ARW")),
        0
    );
    assert_eq!(on_source(), unchanged, "nothing written on the source");

    // Copied with its modified time, as the checked copy copies it, into a folder opened.
    let landed = root.join("landed");
    std::fs::create_dir(&landed).unwrap();
    let copy = landed.join("x.ARW");
    std::fs::copy(&x, &copy).unwrap();
    let modified = std::fs::metadata(&x).unwrap().modified().unwrap();
    (std::fs::File::options().write(true).open(&copy).unwrap())
        .set_modified(modified)
        .unwrap();
    let open = session.send(open_folder(&landed));
    until(&session, &wakes, taken_folder(open));
    session.send(Command::Library(LibraryCommand::Thumbnails {
        photos: vec!["x.ARW".to_owned()],
        size: 256,
    }));
    let told = |e: &Event| matches!(e, Event::Library(LibraryEvent::Thumbnail { open: n, .. }) if *n == open);
    let events = until(&session, &wakes, told);
    assert_eq!(
        thumbnails_of(&events, open),
        [("x.ARW".to_owned(), Some((4, 3)))]
    );
    // Its capture time and its fingerprint; no embedded picture.
    assert_eq!(apertrust_library::testing::reads(&copy), 2);
    drop(session);
    remove(&path);
}

/// Whether `event` is the capture times of the folder's open numbered `open`.
fn taken_folder(open: u64) -> impl Fn(&Event) -> bool {
    move |e| matches!(e, Event::Library(LibraryEvent::Taken { open: n, .. }) if *n == open)
}

/// A source photo whose fingerprint an edited photo shares gets the camera's embedded
/// picture alone, never the edit's render the cache holds in that slot, which stays.
#[test]
fn a_source_s_thumbnail_is_never_an_edited_copy_s_render() {
    let gpu = gpu();
    let (source, options, path) = source_with_cache("import-source-no-render");
    let cache = Arc::clone(&options.cache);
    let x = source.join("x.ARW");
    let state = library::raw_state(&x).expect("x's state");
    let fingerprints = library::Fingerprints::default();
    let identity = library::Identity::read(&x, state, &fingerprints).expect("x's identity");
    let slot = library::thumbnail_key(&identity, 256).slot;
    let render = library::rendered_version(&[7; 32]);
    let picture = library::Thumbnail {
        width: 8,
        height: 6,
        pixels: vec![90; 192].into(),
    };
    let jpeg = thumbnail::encode(&picture).expect("a JPEG");
    let thumbnails = apertrust_base::cache::THUMBNAILS;
    cache.write(thumbnails, &slot, &render, &jpeg).unwrap();

    let (session, wakes) = spawned(&gpu, &[], Some(options), loading(synthetic));
    let listing = session.send(list_source(&source));
    until(&session, &wakes, taken(listing));
    session.send(Command::Library(LibraryCommand::SourceThumbnails {
        listing,
        photos: vec![PathBuf::from("x.ARW")],
        size: 256,
    }));
    let x_told = |e: &Event| matches!(e, Event::Library(LibraryEvent::Thumbnail { photo, .. }) if photo == "x.ARW");
    let events = until(&session, &wakes, x_told);
    // Its own file holds no picture: none, not the render.
    assert_eq!(
        thumbnails_of(&events, listing),
        [("x.ARW".to_owned(), None)]
    );
    assert_eq!(cache.version(thumbnails, &slot).unwrap(), Some(render));
    drop(session);
    remove(&path);
}

/// A new listing cancels the thumbnails the one before asked for, not yet sent; a photo
/// asked for again while its thumbnail is read is read once.
#[test]
fn a_new_listing_cancels_its_thumbnails_and_one_asked_twice_is_read_once() {
    let gpu = gpu();
    let (source, cache, path) = source_with_cache("import-source-thumbnails-cancel");
    let (session, wakes) = spawned(&gpu, &[], Some(cache), loading(synthetic));
    let ask = |listing: u64, photo: &str| {
        session.send(Command::Library(LibraryCommand::SourceThumbnails {
            listing,
            photos: vec![PathBuf::from(photo)],
            size: 256,
        }))
    };

    // Asked twice while the first is held: one thumbnail.
    let listing = session.send(list_source(&source));
    until(&session, &wakes, taken(listing));
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldThumbnails(hold));
    ask(listing, "y.ARW");
    begun
        .recv_timeout(TEST_WAIT)
        .expect("the thumbnail is read");
    ask(listing, "y.ARW");
    // A close, answered on the engine thread, which leaves a source's thumbnails running,
    // comes once the second ask is applied: only then is the first let go.
    let close = session.send(Command::Library(LibraryCommand::Close));
    let closed =
        |e: &Event| matches!(e, Event::Library(LibraryEvent::Closed { open }) if *open == close);
    let mut events = until(&session, &wakes, closed);
    drop(permit);
    let y = |e: &Event| matches!(e, Event::Library(LibraryEvent::Thumbnail { photo, .. }) if photo == "y.ARW");
    events.extend(until(&session, &wakes, y));
    let quiet = Instant::now() + Duration::from_millis(300);
    while Instant::now() < quiet {
        let _ = wakes.recv_timeout(Duration::from_millis(20));
        events.extend(session.events());
    }
    assert_eq!(thumbnails_of(&events, listing).len(), 1, "{events:?}");

    // Held, then a new listing: nothing of the first's.
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldThumbnails(hold));
    ask(listing, "x.ARW");
    begun
        .recv_timeout(TEST_WAIT)
        .expect("the thumbnail is read");
    let next = session.send(list_source(&source));
    let events = until(&session, &wakes, taken(next));
    drop(permit);
    let mut events = events;
    let quiet = Instant::now() + Duration::from_millis(300);
    while Instant::now() < quiet {
        let _ = wakes.recv_timeout(Duration::from_millis(20));
        events.extend(session.events());
    }
    assert!(thumbnails_of(&events, listing).is_empty(), "{events:?}");
    drop(session);
    remove(&path);
}

/// The volumes mounted come as one event with no number, from a job with no progress, so
/// no job event shows it; the boot volume among them on a Mac, each with its space.
#[test]
fn the_volumes_come_unnumbered_from_a_job_with_no_progress() {
    let gpu = gpu();
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let asked = session.send(Command::Library(LibraryCommand::ListVolumes));
    let told = |e: &Event| matches!(e, Event::Library(LibraryEvent::Volumes { .. }));
    let events = until(&session, &wakes, told);
    let volumes: Vec<&library::Volume> = (events.iter())
        .filter_map(|event| match event {
            Event::Library(LibraryEvent::Volumes { volumes }) => Some(volumes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(of_job(&events, asked).is_none(), "{events:?}");
    assert!(
        volumes
            .iter()
            .all(|volume| volume.available <= volume.total)
    );
    if cfg!(target_os = "macos") {
        let root = volumes
            .iter()
            .any(|volume| volume.mount_point == Path::new("/"));
        assert!(root, "{volumes:?}");
    }
}

/// Lists `folder`'s subfolders for the Import tool's Folders tree.
fn place(folder: &Path) -> Command {
    Command::Library(LibraryCommand::SourceSubfolders {
        folder: folder.to_owned(),
    })
}

/// The subfolders answered among `events` in the number `asked`: the folder, its names and
/// why it could not be read.
fn subfolders_of(events: &[Event], asked: u64) -> Vec<(PathBuf, Vec<String>, Option<String>)> {
    (library_of(events, asked).into_iter())
        .filter_map(|event| match event {
            LibraryEvent::SubfoldersOf {
                folder,
                names,
                unreadable,
                ..
            } => Some((folder.clone(), names.clone(), unreadable.clone())),
            _ => None,
        })
        .collect()
}

/// A place's subfolders are listed in their own command's number with no folder open and
/// with one open elsewhere; a relative folder and one with a `..` are refused, and one that
/// cannot be read says why.
#[test]
fn a_place_s_subfolders_are_listed_whatever_is_open() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-source-places");
    let (elsewhere, other_path) = photo_folder("import-source-places-elsewhere");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let answered = |asked: u64| move |e: &Event| matches!(e, Event::Library(LibraryEvent::SubfoldersOf { open, .. }) if *open == asked);

    let asked = session.send(place(&folder));
    let events = until(&session, &wakes, answered(asked));
    let day = vec!["Day 1".to_owned()];
    assert_eq!(
        subfolders_of(&events, asked),
        [(folder.clone(), day.clone(), None)]
    );

    let open = session.send(open_folder(&elsewhere));
    follow(&session, &wakes, &mut Vec::new(), open, ended);
    session.send(place(Path::new("Day 1")));
    session.send(place(&folder.join("Day 1").join("..")));
    let gone = folder.join("gone");
    let asked = session.send(place(&gone));
    let events = until(&session, &wakes, answered(asked));
    let told = subfolders_of(&events, asked);
    let [(at, names, Some(why))] = told.as_slice() else {
        panic!("{events:?}");
    };
    assert_eq!((at, names.len()), (&gone, 0));
    assert!(!why.is_empty());
    let any = |e: &Event| matches!(e, Event::Library(LibraryEvent::SubfoldersOf { .. }));
    assert_eq!(events.iter().filter(|e| any(e)).count(), 1, "{events:?}");
    let after = session.send(place(&folder));
    let events = until(&session, &wakes, answered(after));
    assert_eq!(subfolders_of(&events, after), [(folder.clone(), day, None)]);
    drop(session);
    remove(&path);
    remove(&other_path);
}

/// A `CancelListing`, as the sheet closes, drops a place's subfolders still listed: no
/// answer comes.
#[test]
fn a_cancel_listing_drops_a_place_s_subfolders() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-source-places-cancel");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold));
    let asked = session.send(place(&folder));
    begun.recv_timeout(TEST_WAIT).expect("the listing begins");
    session.send(Command::Library(LibraryCommand::CancelListing {
        listing: 0,
    }));
    // A listing after it answers only once the cancel is applied, the held job ending as
    // its cancel comes: only then is it let go.
    let listing = session.send(list_source(&folder));
    let mut events = until(&session, &wakes, taken(listing));
    drop(permit);
    let quiet = Instant::now() + Duration::from_millis(200);
    while Instant::now() < quiet {
        let _ = wakes.recv_timeout(Duration::from_millis(20));
        events.extend(session.events());
    }
    assert!(subfolders_of(&events, asked).is_empty(), "{events:?}");
    drop(session);
    remove(&path);
}

/// Import's settings sending the photos straight into `folder`.
fn into(folder: &Path) -> library::ImportSettings {
    library::ImportSettings {
        base_folder: pattern(folder),
        default_folder: pattern(folder),
        subfolder: String::new(),
        job_code: "Test".to_owned(),
        places: Places::default(),
    }
}

/// A preview comes in its command's number by a job with no progress, whatever folder is
/// open, its photos in the order of their paths, and makes nothing.
#[test]
fn an_import_s_preview_comes_in_its_number_and_makes_nothing() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-preview");
    // Beside the source, never inside it.
    let mut name = folder.file_name().unwrap().to_owned();
    name.push(" imported");
    let destination = folder.with_file_name(name);
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let photos = library::ImportPhotos {
        folder: folder.clone(),
        files: vec![PathBuf::from("B.ARW"), PathBuf::from("A.ARW")],
    };
    let preview = session.send(Command::Library(LibraryCommand::PreviewImport {
        photos,
        settings: into(&destination),
    }));
    let previewed = |e: &Event| matches!(e, Event::Library(LibraryEvent::ImportPreview { open, .. }) if *open == preview);
    let events = until(&session, &wakes, previewed);
    let [LibraryEvent::ImportPreview { preview: found, .. }] = library_of(&events, preview)[..]
    else {
        panic!("{events:?}");
    };
    let to: Vec<_> = (found.photos.iter())
        .map(|photo| match &photo.fate {
            library::ImportFate::Copy { to, .. } => to.clone(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(to, [destination.join("A.ARW"), destination.join("B.ARW")]);
    assert!(found.problems.is_empty(), "{:?}", found.problems);
    assert!(of_job(&events, preview).is_none(), "{events:?}");
    assert!(!destination.exists());
    drop(session);
    remove(&path);
}

/// A folder beside `folder`, never inside it, for an import's destination.
fn beside(folder: &Path, what: &str) -> PathBuf {
    let mut name = folder.file_name().unwrap().to_owned();
    name.push(format!(" {what}"));
    folder.with_file_name(name)
}

/// The photos an `Imported` of the import numbered `import` names among `events`, with
/// what became of each.
fn imported(events: &[Event], import: u64) -> Vec<library::ImportedPhoto> {
    (library_of(events, import).into_iter())
        .flat_map(|event| match event {
            LibraryEvent::Imported { photos, .. } => photos.clone(),
            other => panic!("{other:?}"),
        })
        .collect()
}

/// An import is a job of kind `import` on `UserForeground`, cancellable, its events in
/// order from `Added`, with its title and the photos as its total, to `Ended`, its
/// destination the folder the photos go to, each photo told in its number.
#[test]
fn an_import_is_a_job_from_added_to_ended() {
    let gpu = gpu();
    let (folder, path) = photo_folder("import-job");
    let destination = beside(&folder, "imported");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let photos = library::ImportPhotos {
        folder: folder.clone(),
        files: vec![PathBuf::from("B.ARW"), PathBuf::from("A.ARW")],
    };
    let import = session.send(Command::Library(LibraryCommand::Import {
        photos,
        settings: into(&destination),
    }));
    let mut events = Vec::new();
    let (job, own) = follow(&session, &wakes, &mut events, import, ended);
    let JobEvent::Added {
        kind,
        queue,
        title,
        cancellable,
        progress,
        retries,
        ..
    } = &own[0]
    else {
        panic!("{own:?}");
    };
    assert_eq!(*kind, "import");
    assert_eq!(*queue, Some(Queue::UserForeground));
    assert_eq!(*title, jobs::Message::new("job-import").with("count", 2));
    assert!(*cancellable && retries.is_none());
    assert_eq!(progress.total, Some(2));
    let destinations: Vec<_> = (own.iter())
        .filter_map(|event| match event {
            JobEvent::Destination { destination, .. } => Some(destination.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        destinations,
        [jobs::Destination::Folder(destination.clone())]
    );
    let Some(JobEvent::Ended {
        state,
        outcome,
        failure,
        offers,
        ..
    }) = own.last()
    else {
        panic!("{own:?}");
    };
    assert_eq!(*state, jobs::State::Finished);
    assert!(failure.is_none() && offers.is_empty(), "{own:?}");
    let finished = jobs::Message::new("job-import-finished")
        .with("copied", 2)
        .with("skipped", 0)
        .with("failed", 0)
        .with("total", 2);
    assert_eq!(outcome.as_ref(), Some(&finished));
    let copies: Vec<_> = imported(&events, import)
        .into_iter()
        .map(|photo| (photo.source, photo.result))
        .collect();
    let copied = |name: &str| library::ImportResult::Copied {
        copy: destination.join(name),
    };
    assert_eq!(
        copies,
        [
            (PathBuf::from("A.ARW"), copied("A.ARW")),
            (PathBuf::from("B.ARW"), copied("B.ARW"))
        ]
    );
    assert!(destination.join("A.ARW").is_file() && destination.join("B.ARW").is_file());
    assert_eq!(job_of(&own[0]), job);
    drop(session);
    remove(&path);
    let _ = std::fs::remove_dir_all(&destination);
}

/// An import with a photo that could not be read offers Try again, which copies that photo
/// alone, once, its job naming the one it tries again.
#[cfg(unix)]
#[test]
fn try_again_copies_the_photo_that_failed_once() {
    use std::os::unix::fs::PermissionsExt;
    let gpu = gpu();
    let (folder, path) = photo_folder("import-retry");
    let destination = beside(&folder, "imported");
    let locked = folder.join("B.ARW");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let photos = library::ImportPhotos {
        folder: folder.clone(),
        files: vec![PathBuf::from("A.ARW"), PathBuf::from("B.ARW")],
    };
    let import = session.send(Command::Library(LibraryCommand::Import {
        photos,
        settings: into(&destination),
    }));
    let mut events = Vec::new();
    let (job, own) = follow(&session, &wakes, &mut events, import, ended);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();
    let Some(JobEvent::Ended {
        failure: Some(failure),
        offers,
        ..
    }) = own.last()
    else {
        panic!("{own:?}");
    };
    assert_eq!(failure.cause, "unreadable");
    assert_eq!(failure.item.as_deref(), Some(locked.as_path()));
    assert_eq!(offers, &[jobs::Offer::Retry { items: 1 }]);

    let retry = session.send(Command::RetryJob { job });
    let mut events = Vec::new();
    let (_, own) = follow(&session, &wakes, &mut events, retry, ended);
    let JobEvent::Added { retries, title, .. } = &own[0] else {
        panic!("{own:?}");
    };
    assert_eq!(*retries, Some(job));
    assert_eq!(*title, jobs::Message::new("job-import").with("count", 1));
    let copies: Vec<_> = (imported(&events, retry).into_iter())
        .map(|photo| photo.source)
        .collect();
    assert_eq!(copies, [PathBuf::from("B.ARW")]);
    assert!(destination.join("B.ARW").is_file());

    let again = session.send(Command::RetryJob { job });
    let refused = |e: &Event| {
        matches!(e, Event::Failed(SessionError::Refused { command, error })
            if *command == again && matches!(**error, SessionError::NotRetryable { .. }))
    };
    until(&session, &wakes, refused);
    drop(session);
    remove(&path);
    let _ = std::fs::remove_dir_all(&destination);
}
