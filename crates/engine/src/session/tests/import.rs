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
//! is open, which the next listing or a `CancelListing` stops.

use super::*;

/// Lists `source` for Import.
fn list_source(source: &Path) -> Command {
    Command::Library(LibraryCommand::ListSource {
        source: source.to_owned(),
        include_subfolders: false,
    })
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
