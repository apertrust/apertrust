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

//! The Metadata tool's summary of several photos (decision 110): `Summarise` answered by
//! `Summary`, the facts it lacks read by its job and kept for the next, a later one
//! cancelling the one before, the facts read by then kept.

use apertrust_library::testing::reads;

use super::*;
use crate::session::facts::FactSummary;

fn two_photos(name: &str) -> PathBuf {
    let folder = scratch(name).parent().unwrap().to_owned();
    for raw in ["A.ARW", "B.ARW"] {
        std::fs::write(folder.join(raw), [0u8; 4]).unwrap();
    }
    folder
}

/// The folder at `folder` opened, its listing done.
fn opened(session: &Session, wakes: &Receiver<()>, folder: &Path) {
    let open = session.send(Command::Library(LibraryCommand::Open(
        library::Source::Folder(folder.to_owned()),
    )));
    let taken =
        |e: &Event| matches!(e, Event::Library(LibraryEvent::Taken { open: n, .. }) if *n == open);
    until(session, wakes, taken);
}

/// The summaries in `events` answering the command numbered `through`.
fn answering(events: &[Event], through: u64) -> Vec<Arc<facts::Summary>> {
    (events.iter())
        .filter_map(|event| match event {
            Event::Summary {
                through: n,
                summary,
            } if *n == through => Some(summary.clone()),
            _ => None,
        })
        .collect()
}

/// Whether `summary` waits for no photo's facts.
fn read(summary: &facts::Summary) -> bool {
    !matches!(summary.get("lens"), Some(FactSummary::Waiting { .. }))
}

/// The answers to `through`, up to the first that waits for no facts.
fn answers(session: &Session, wakes: &Receiver<()>, through: u64) -> Vec<Arc<facts::Summary>> {
    let done = |e: &Event| matches!(e, Event::Summary { through: n, summary } if *n == through && read(summary));
    answering(&until(session, wakes, done), through)
}

/// A summary of photos whose facts are not read yet reads them, answering first with
/// their rows waiting, then with them; a photo outside the collection has none. The next
/// summary of them reads nothing again.
#[test]
fn a_summary_reads_the_facts_it_lacks_and_the_next_reuses_them() {
    let gpu = gpu();
    let folder = two_photos("summary-read");
    let (a, b) = (folder.join("A.ARW"), folder.join("B.ARW"));
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    opened(&session, &wakes, &folder);
    // The open reads the capture times, not the facts.
    assert_eq!((reads(&a), reads(&b)), (1, 1));

    let photos = vec![a.clone(), b.clone(), folder.join("Gone.ARW")];
    let asked = session.send(Command::Summarise {
        photos: photos.clone(),
    });
    let answered = answers(&session, &wakes, asked);
    assert_eq!(
        answered[0].get("lens"),
        Some(&FactSummary::Waiting { read: 1, of: 3 })
    );
    let last = answered.last().unwrap();
    assert_eq!(last.photos, 3);
    assert_eq!(
        last.get("size_on_disk"),
        Some(&FactSummary::Total { bytes: 8 })
    );
    // Four bytes are no raw: read, and none.
    assert_eq!(last.get("lens"), Some(&FactSummary::None));
    assert_eq!((reads(&a), reads(&b)), (2, 2));

    let again = session.send(Command::Summarise { photos });
    let answered = answers(&session, &wakes, again);
    assert_eq!(answered.len(), 1, "answered at once, nothing to read");
    assert_eq!((reads(&a), reads(&b)), (2, 2));
}

/// A `Summarise` sent while the one before is still reading cancels it, which answers no
/// more; the facts it read by then are kept, so a summary of them reads nothing.
#[test]
fn a_later_summarise_cancels_the_one_before_which_keeps_what_it_read() {
    let gpu = gpu();
    let folder = two_photos("summary-cancel");
    let (a, b) = (folder.join("A.ARW"), folder.join("B.ARW"));
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    opened(&session, &wakes, &folder);
    let (hold, begun, permit) = crate::export_job::testing::Hold::new();
    session.send(Command::HoldSummary(hold));
    let first = session.send(Command::Summarise {
        photos: vec![a.clone(), b.clone()],
    });
    // A read, the job held before B.
    begun
        .recv_timeout(TEST_WAIT)
        .expect("the first summary reads its first photo");
    let second = session.send(Command::Summarise {
        photos: vec![b.clone()],
    });
    // The second taken, the first cancelled, as a selection sent after it tells; then the
    // first let go, the second waiting for it on the same queue.
    let selected = session.send(Command::Selection(SelectionCommand::Add(a.clone())));
    let told = |e: &Event| matches!(e, Event::Selection(s) if s.through >= selected);
    let mut events = until(&session, &wakes, told);
    let answered = |e: &Event| matches!(e, Event::Summary { through, .. } if *through == second);
    if !events.iter().any(answered) {
        events.extend(until(&session, &wakes, answered));
    }
    // Let the first go on: it sees the cancel, keeps what it read and answers nothing.
    permit.send(()).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    events.extend(session.events());
    // The first answered once, as it began, and no more.
    assert_eq!(answering(&events, first).len(), 1);
    assert_eq!(reads(&a), 2, "the open's and the first summary's");
    let third = session.send(Command::Summarise {
        photos: vec![a.clone()],
    });
    let answered = answers(&session, &wakes, third);
    assert_eq!(answered.len(), 1);
    assert_eq!(reads(&a), 2, "kept, not read again");
}

/// Each photo's `.apxmp` titled `title`.
fn titled(photos: &[&Path], title: &str) {
    use apertrust_library::clock::{Clock, MachineId};
    use apertrust_library::fields::{self, FieldValue};
    for photo in photos {
        let mut file = sidecar::Edits::new("test");
        let copy = crate::session::fields::copy_mut(&mut file);
        let clock = Clock::at(1, None, MachineId::UNKNOWN);
        let value = FieldValue::LangAlt(vec![("x-default".to_owned(), title.to_owned())]);
        fields::set(&mut copy.fields, "dc:title", Some(value), clock);
        copy.clocks.push(clock);
        std::fs::write(sidecar::path_for(photo), file.to_xml()).unwrap();
    }
}

/// The photos' fields as their listing, or the reading of photos opened alone, read them:
/// a summary of two photos titled alike, opened in their folder or alone, shows the title.
#[test]
fn a_summary_shows_the_fields_the_listing_or_the_photos_alone_read() {
    use crate::session::facts::FieldSummary;
    let gpu = gpu();
    let folder = two_photos("summary-fields");
    let (a, b) = (folder.join("A.ARW"), folder.join("B.ARW"));
    titled(&[&a, &b], "Tram 28");
    let (session, wakes) = start_with(&gpu, &[], synthetic);
    let title = |summary: &facts::Summary| match summary.field("dc:title") {
        Some(FieldSummary::Same(value)) => Some(value.shown()),
        _ => None,
    };
    opened(&session, &wakes, &folder);
    let asked = session.send(Command::Summarise {
        photos: vec![a.clone(), b.clone()],
    });
    let answered = answers(&session, &wakes, asked);
    assert_eq!(title(&answered[0]).as_deref(), Some("Tram 28"));
    // Opened alone: their reading, which comes after the open, carries the fields.
    session.send(Command::Library(LibraryCommand::Open(
        library::Source::Photos(vec![a.clone(), b.clone()]),
    )));
    let deadline = Instant::now() + TEST_WAIT;
    loop {
        let asked = session.send(Command::Summarise {
            photos: vec![a.clone(), b.clone()],
        });
        let answered = answers(&session, &wakes, asked);
        if title(answered.last().unwrap()).as_deref() == Some("Tram 28") {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the photos alone show their title"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
