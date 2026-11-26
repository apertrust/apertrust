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

//! The selection through a session (decision 37): the commands, the photo opened, the
//! folder's listing and the `Selection` fact.

use super::*;

/// A folder of its own holding the raws A, B and C, which [`remove`] takes with it.
fn three_photos(name: &str) -> PathBuf {
    let folder = scratch(name).parent().unwrap().to_owned();
    for raw in ["A.ARW", "B.ARW", "C.ARW"] {
        std::fs::write(folder.join(raw), [0u8; 4]).unwrap();
    }
    folder
}

/// Waits for the selection the session tells next whose photos are `photos`, in `folder`,
/// with `active`, the active photo, through at least the command numbered `through`.
fn told(
    session: &Session,
    wakes: &Receiver<()>,
    folder: &Path,
    photos: &[&str],
    active: Option<&str>,
    through: u64,
) -> Vec<Event> {
    let photos: Vec<PathBuf> = photos.iter().map(|photo| folder.join(photo)).collect();
    let active = active.map(|photo| folder.join(photo));
    until(session, wakes, |event| {
        matches!(event, Event::Selection(selection)
            if selection.photos == photos
                && selection.active == active
                && selection.through >= through)
    })
}

/// Waits until the photo the open numbered `open` opens is live, unless `events`,
/// drained already, say so.
fn go_live(session: &Session, wakes: &Receiver<()>, events: &[Event], open: u64) {
    let live =
        |e: &Event| matches!(e, Event::Values(values) if values.live && values.through >= open);
    if !events.iter().any(live) {
        until(session, wakes, live);
    }
}

fn select(command: SelectionCommand) -> Command {
    Command::Selection(command)
}

/// Each command, the rule that the selection holds the active photo, opening a photo
/// outside the selection and a selected one, Unedited by the edit the open photo's values
/// report, a photo outside the folder refused, and a folder listed again.
#[test]
fn a_session_keeps_the_selection_of_the_folder_s_photos() {
    let gpu = gpu();
    let folder = three_photos("selection");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let open = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);

    // Opened, A is selected alone; C added.
    open_live(&session, &wakes, &folder.join("A.ARW"));
    let add = session.send(select(SelectionCommand::Add(folder.join("C.ARW"))));
    told(
        &session,
        &wakes,
        &folder,
        &["A.ARW", "C.ARW"],
        Some("A.ARW"),
        add,
    );
    // A photo of another folder is refused, naming its command.
    let elsewhere = scratch("selection-elsewhere");
    let refused = session.send(select(SelectionCommand::Add(elsewhere.clone())));
    let events = until(&session, &wakes, |e| matches!(e, Event::Failed(_)));
    assert!(
        events.iter().any(|e| matches!(e,
            Event::Failed(SessionError::Refused { command, error })
                if *command == refused
                    && matches!(&**error, SessionError::NotInCollection { photo } if *photo == elsewhere))),
        "{events:?}"
    );
    // B, opened from outside the selection, is selected alone.
    let b = session.send(Command::Open(folder.join("B.ARW")));
    told(&session, &wakes, &folder, &["B.ARW"], Some("B.ARW"), b);
    let all = session.send(select(SelectionCommand::All));
    let every = ["A.ARW", "B.ARW", "C.ARW"];
    told(&session, &wakes, &folder, &every, Some("B.ARW"), all);
    // C, selected, made active: the selection stays.
    let c = session.send(Command::Open(folder.join("C.ARW")));
    let events = told(&session, &wakes, &folder, &every, Some("C.ARW"), c);
    go_live(&session, &wakes, &events, c);

    // C edited, as its values say, is not among the unedited: it closes, its edit saved.
    let edit = session.send(set("brightness", ControlValue::Number(0.2), true));
    until(
        &session,
        &wakes,
        |e| matches!(e, Event::Values(values) if values.through >= edit && values.edited),
    );
    let unedited = session.send(select(SelectionCommand::Unedited));
    let events = told(
        &session,
        &wakes,
        &folder,
        &["A.ARW", "B.ARW"],
        None,
        unedited,
    );
    let mut events = events.into_iter().chain(session.events());
    assert!(
        !events.any(|e| matches!(e, Event::Values(values) if values.through >= unedited)),
        "no photo is open"
    );
    // B opens, selected, keeping the selection; Invert leaves C, B closing.
    let b = session.send(Command::Open(folder.join("B.ARW")));
    let events = told(
        &session,
        &wakes,
        &folder,
        &["A.ARW", "B.ARW"],
        Some("B.ARW"),
        b,
    );
    go_live(&session, &wakes, &events, b);
    let invert = session.send(select(SelectionCommand::Invert));
    told(&session, &wakes, &folder, &["C.ARW"], None, invert);

    // A range from the active photo in the client's order, then Select None.
    let a = session.send(Command::Open(folder.join("A.ARW")));
    told(&session, &wakes, &folder, &["A.ARW"], Some("A.ARW"), a);
    let order = ["C.ARW", "A.ARW", "B.ARW"].map(|photo| folder.join(photo));
    let range = session.send(select(SelectionCommand::Range {
        to: folder.join("C.ARW"),
        order: order.to_vec(),
    }));
    told(
        &session,
        &wakes,
        &folder,
        &["A.ARW", "C.ARW"],
        Some("A.ARW"),
        range,
    );
    let none = session.send(select(SelectionCommand::Clear));
    told(&session, &wakes, &folder, &[], None, none);

    // Listed again with C gone, the selection keeps A and B by photo and drops C.
    let a = session.send(Command::Open(folder.join("A.ARW")));
    let set = select(SelectionCommand::Set(
        every.map(|p| folder.join(p)).to_vec(),
    ));
    let set = session.send(set);
    told(&session, &wakes, &folder, &every, Some("A.ARW"), set.max(a));
    std::fs::remove_file(folder.join("C.ARW")).unwrap();
    let again = session.send(open_folder(&folder));
    told(
        &session,
        &wakes,
        &folder,
        &["A.ARW", "B.ARW"],
        Some("A.ARW"),
        again,
    );
    // Another folder: A alone, the active photo.
    let other = three_photos("selection-other");
    let other_open = session.send(open_folder(&other));
    told(
        &session,
        &wakes,
        &folder,
        &["A.ARW"],
        Some("A.ARW"),
        other_open,
    );
    drop(session);
    remove(&folder.join("A.ARW"));
    remove(&other.join("A.ARW"));
    remove(&elsewhere);
}

/// The command that tells the session the photos at `paths` opened alone.
fn alone(paths: &[PathBuf]) -> Command {
    Command::Library(LibraryCommand::Open(library::Source::Photos(
        paths.to_vec(),
    )))
}

/// The first selection the session tells through at least the command numbered `through`,
/// which must be `photos`, with `active`: photos of any folders.
fn told_paths(
    session: &Session,
    wakes: &Receiver<()>,
    photos: &[&PathBuf],
    active: Option<&PathBuf>,
    through: u64,
) {
    let after = |e: &Event| matches!(e, Event::Selection(s) if s.through >= through);
    let events = until(session, wakes, after);
    let Some(Event::Selection(selection)) = events.into_iter().find(after) else {
        unreachable!("`until` stops at it");
    };
    let photos: Vec<&PathBuf> = photos.to_vec();
    assert_eq!(
        selection.photos.iter().collect::<Vec<_>>(),
        photos,
        "{through}"
    );
    assert_eq!(selection.active.as_ref(), active, "{through}");
}

/// Photos opened alone, from two folders, are the collection, in the order given: each
/// command acts on them as on a folder's photos, a photo of their folders not among them
/// refused, and `Unedited` reads them by their sidecars, which their job reads.
/// Photos opened alone again replace them, and a folder opened after replaces those.
#[test]
fn photos_opened_alone_are_the_collection() {
    let gpu = gpu();
    let here = three_photos("selection-alone");
    let there = three_photos("selection-alone-there");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let (c, a, b) = (here.join("C.ARW"), there.join("A.ARW"), here.join("B.ARW"));
    let here_a = here.join("A.ARW");
    let photos = [c.clone(), a.clone(), here_a.clone()];
    session.send(alone(&photos));
    // C opens, selected alone; inverted, the other two, and C closes.
    open_live(&session, &wakes, &c);
    let invert = session.send(select(SelectionCommand::Invert));
    told_paths(&session, &wakes, &[&a, &here_a], None, invert);
    let all = session.send(select(SelectionCommand::All));
    told_paths(&session, &wakes, &[&c, &a, &here_a], None, all);
    let removed = session.send(select(SelectionCommand::Remove(a.clone())));
    told_paths(&session, &wakes, &[&c, &here_a], None, removed);
    let given = session.send(select(SelectionCommand::Set(vec![a.clone()])));
    told_paths(&session, &wakes, &[&a], None, given);
    let add = session.send(select(SelectionCommand::Add(c.clone())));
    told_paths(&session, &wakes, &[&c, &a], None, add);
    // B, of their folder but not among them, is refused.
    let refused = session.send(select(SelectionCommand::Add(b.clone())));
    let events = until(&session, &wakes, |e| matches!(e, Event::Failed(_)));
    assert!(
        events.iter().any(|e| matches!(e,
            Event::Failed(SessionError::Refused { command, error })
                if *command == refused
                    && matches!(&**error, SessionError::NotInCollection { photo } if *photo == b))),
        "{events:?}"
    );
    // C, selected, opens keeping the selection; a range from it in the client's order, then
    // Select None.
    let open = session.send(Command::Open(c.clone()));
    told_paths(&session, &wakes, &[&c, &a], Some(&c), open);
    let order = vec![here_a.clone(), a.clone(), c.clone()];
    let to = here_a.clone();
    let range = session.send(select(SelectionCommand::Range { to, order }));
    told_paths(&session, &wakes, &[&c, &a, &here_a], Some(&c), range);
    let none = session.send(select(SelectionCommand::Clear));
    told_paths(&session, &wakes, &[], None, none);

    // C edited, saved; opened alone again with A of here, C reads as its sidecar holds it,
    // once their job has read them, and A of there is no longer among them.
    open_live(&session, &wakes, &c);
    let edit = session.send(set("brightness", ControlValue::Number(0.2), true));
    let saved = |e: &Event| matches!(e, Event::Saved { through, .. } if *through >= edit);
    until(&session, &wakes, saved);
    // Closed, C stays selected.
    let close = session.send(Command::Close);
    told_paths(&session, &wakes, &[&c], None, close);
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold));
    session.send(alone(&[c.clone(), here_a.clone()]));
    begun
        .recv_timeout(TEST_WAIT)
        .expect("their edit states are sent");
    let unedited = session.send(select(SelectionCommand::Unedited));
    told_paths(&session, &wakes, &[&here_a], None, unedited);
    drop(permit);
    let refused = session.send(select(SelectionCommand::Add(a.clone())));
    let events = until(&session, &wakes, |e| matches!(e, Event::Failed(_)));
    assert!(
        events.iter().any(|e| matches!(e,
            Event::Failed(SessionError::Refused { command, .. }) if *command == refused)),
        "{events:?}"
    );

    // A folder opened after them is the collection in their place.
    let open = session.send(open_folder(&there));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    let all = session.send(select(SelectionCommand::All));
    let every = ["A.ARW", "B.ARW", "C.ARW"];
    told(&session, &wakes, &there, &every, None, all);
    drop(session);
    remove(&c);
    remove(&a);
}

/// A photo opened alone whose sidecar's read is stuck leaves the others read: each state
/// comes as it is read, so `Unedited` selects the unedited photo before it, the stuck one
/// not known yet and left out.
#[test]
fn photos_alone_read_before_a_stuck_sidecar_count_at_once() {
    use crate::session::selection::stall;

    let gpu = gpu();
    let folder = three_photos("selection-alone-partial");
    let (a, b) = (folder.join("A.ARW"), folder.join("B.ARW"));
    let reached = stall::hold(&b);
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    let release = Release(b.clone());
    session.send(alone(&[a.clone(), b.clone()]));
    // A's state was sent before B's read began.
    reached.recv_timeout(TEST_WAIT).expect("B's read is stuck");
    let unedited = session.send(select(SelectionCommand::Unedited));
    told_paths(&session, &wakes, &[&a], None, unedited);
    drop(release);
    drop(session);
    remove(&a);
}

/// Releases a photo held by `selection::stall` as it drops.
struct Release(PathBuf);

impl Drop for Release {
    fn drop(&mut self) {
        crate::session::selection::stall::release(&self.0);
    }
}

/// The job reading the edits of photos opened alone never holds the session's end: with a
/// sidecar's read stuck, as on a share that does not answer, the session's end cancels the
/// job, which returns at once, the read left to end on its own thread.
#[test]
fn a_session_ends_at_once_while_a_photo_alone_s_sidecar_read_is_stuck() {
    use crate::session::selection::stall;

    let gpu = gpu();
    let photo = scratch("selection-alone-stuck");
    let reached = stall::hold(&photo);
    let (session, _wakes) = start_with(&gpu, &[&Sample], synthetic);
    // Released before the session drops, should the test fail before the end is measured.
    let _release = Release(photo.clone());
    session.send(alone(std::slice::from_ref(&photo)));
    reached.recv_timeout(TEST_WAIT).expect("the read is stuck");
    // Should the end wait for the read after all, it ends once the photo is released, 20 s
    // after the end began, and the test fails on the time rather than hanging.
    let (done, finished) = std::sync::mpsc::channel::<()>();
    let releaser = {
        let photo = photo.clone();
        std::thread::spawn(move || {
            let _ = finished.recv_timeout(Duration::from_secs(20));
            stall::release(&photo);
        })
    };
    let started = Instant::now();
    drop(session);
    let took = started.elapsed();
    assert!(took < Duration::from_secs(10), "ended after {took:?}");
    drop(done);
    releaser.join().unwrap();
    remove(&photo);
}

/// The first selection the session tells through at least the command numbered `through`,
/// which must be `photos`, in `folder`, with `active`.
fn told_next(
    session: &Session,
    wakes: &Receiver<()>,
    folder: &Path,
    photos: &[&str],
    active: Option<&str>,
    through: u64,
) {
    let after = |e: &Event| matches!(e, Event::Selection(s) if s.through >= through);
    let events = until(session, wakes, after);
    let Some(Event::Selection(selection)) = events.into_iter().find(after) else {
        unreachable!("`until` stops at it");
    };
    let photos: Vec<PathBuf> = photos.iter().map(|photo| folder.join(photo)).collect();
    assert_eq!(selection.photos, photos);
    assert_eq!(selection.active, active.map(|photo| folder.join(photo)));
}

/// Holds the engine thread at the submit gate, rendering the photo at `width`, which
/// changes no edit, until what this returns is dropped: the commands sent meanwhile are
/// then taken as one batch.
fn one_batch<'a>(gpu: &'a GpuContext, session: &Session, width: u32) -> crate::gpu::Held<'a> {
    let held = gpu.gate.hold();
    session.send(Command::Resize { width, height: 48 });
    // The test's hold and the engine's turn, waiting.
    gpu.gate.until_asked(2);
    held
}

/// A change to the open photo and `Unedited` in one batch: `Unedited` reads the photo as
/// the change left it, so it closes; and a photo changed in the batch that closes it, whose
/// values that batch never sends, is read as it left too.
#[test]
fn unedited_reads_the_photo_as_its_own_batch_changed_it() {
    let gpu = gpu();
    let folder = three_photos("selection-batch");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let open = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    // A edited and Unedited sent together, never waiting for the values between them.
    open_live(&session, &wakes, &folder.join("A.ARW"));
    let held = one_batch(&gpu, &session, 60);
    session.send(set("brightness", ControlValue::Number(0.2), true));
    let unedited = session.send(select(SelectionCommand::Unedited));
    drop(held);
    let left = ["B.ARW", "C.ARW"];
    told_next(&session, &wakes, &folder, &left, None, unedited);
    // B edited and closed by Select None in one batch, which sends none of its values.
    open_live(&session, &wakes, &folder.join("B.ARW"));
    let held = one_batch(&gpu, &session, 56);
    session.send(set("brightness", ControlValue::Number(0.2), true));
    let none = session.send(select(SelectionCommand::Clear));
    drop(held);
    told(&session, &wakes, &folder, &[], None, none);
    let unedited = session.send(select(SelectionCommand::Unedited));
    told_next(&session, &wakes, &folder, &["C.ARW"], None, unedited);
    drop(session);
    remove(&folder.join("A.ARW"));
}

/// A turn that panics closes the photo, unsaved, and the selection says so in the same
/// burst, with no active photo, through every command the turn took, those it never
/// reached included.
#[test]
fn a_panicked_turn_tells_the_selection_with_no_active_photo() {
    let path = scratch("selection-panic");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let opened = driven.engine.selection_event(&[]);
    assert!(
        matches!(&opened, Some(Event::Selection(s)) if s.active.as_deref() == Some(&*path)),
        "{opened:?}"
    );
    let payload: Box<dyn std::any::Any + Send> = Box::new("a bug");
    let taken = driven.engine.applied + 3;
    let events = driven.engine.recover(payload.as_ref(), taken);
    assert!(
        matches!(&events[..], [
            Event::Failed(SessionError::Panicked(_)),
            Event::View(ViewState { placement: None, .. }),
            Event::Selection(Selection { photos, active: None, through, .. }),
        ] if photos.is_empty() && *through == taken),
        "{events:?}"
    );
    assert_eq!(driven.engine.applied, taken);
    remove(&path);
}

/// Commands still queued as the client drops its handle: the folder's close counts, so
/// `Unedited` with no collection closes the photo, and the change after it is not written.
#[test]
fn a_folder_closed_while_the_handle_drops_counts_for_the_selection() {
    let gpu = gpu();
    let folder = three_photos("selection-drop");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let open = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    let a = folder.join("A.ARW");
    open_live(&session, &wakes, &a);
    // Held at the gate, the engine takes what follows only after the handle is gone.
    let held = one_batch(&gpu, &session, 60);
    session.send(Command::Library(LibraryCommand::Close));
    session.send(select(SelectionCommand::Unedited));
    session.send(set("brightness", ControlValue::Number(0.2), true));
    drop(session);
    drop(held);
    assert!(
        !sidecar::path_for(&a).exists(),
        "the change after Unedited is not written"
    );
    remove(&a);
}

/// A listing of the folder again cancelled through its job, as the jobs window's Cancel
/// does, leaves the folder with no photo, as the Browser shows it: the selection keeps the
/// active photo alone, and the photos before are no longer the collection.
#[test]
fn a_listing_cancelled_leaves_the_folder_with_no_photo() {
    let gpu = gpu();
    let folder = three_photos("selection-cancel");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let open = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    open_live(&session, &wakes, &folder.join("A.ARW"));
    let add = session.send(select(SelectionCommand::Add(folder.join("B.ARW"))));
    told(
        &session,
        &wakes,
        &folder,
        &["A.ARW", "B.ARW"],
        Some("A.ARW"),
        add,
    );
    // Listed again, held before it reads, and cancelled.
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold));
    let again = session.send(open_folder(&folder));
    let added = |e: &Event| matches!(e, Event::Job { event: JobEvent::Added { .. }, command: Some(c) } if *c == again);
    let events = until(&session, &wakes, added);
    let job = events
        .iter()
        .find_map(|e| match e {
            Event::Job {
                event: JobEvent::Added { job, .. },
                command: Some(c),
            } if *c == again => Some(*job),
            _ => None,
        })
        .expect("its job");
    begun.recv_timeout(TEST_WAIT).expect("the reading begins");
    session.send(Command::CancelJob { job });
    // A refusal after it proves the cancel taken before the reading is let go.
    let elsewhere = scratch("selection-cancel-elsewhere");
    let refused = session.send(select(SelectionCommand::Add(elsewhere.clone())));
    let answered = |e: &Event| matches!(e, Event::Failed(SessionError::Refused { command, .. }) if *command == refused);
    let events = until(&session, &wakes, answered);
    drop(permit);
    // Its end may have come with the refusal: the held reading ends as the cancel reaches it.
    let ended = |e: &Event| matches!(e, Event::Job { event: JobEvent::Ended { job: j, .. }, .. } if *j == job);
    if !events.iter().any(ended) {
        until(&session, &wakes, ended);
    }
    // Inverted over no photo: none, and A, left out, closes; over the photos before, C.
    let invert = session.send(select(SelectionCommand::Invert));
    told_next(&session, &wakes, &folder, &[], None, invert);
    drop(session);
    remove(&folder.join("A.ARW"));
    remove(&elsewhere);
}

/// The job of the open numbered `open`, from the events that tell it added.
fn job_of(events: &[Event], open: u64) -> Option<JobId> {
    events.iter().find_map(|e| match e {
        Event::Job {
            event: JobEvent::Added { job, .. },
            command: Some(c),
        } if *c == open => Some(*job),
        _ => None,
    })
}

/// Waits for the answer to the selection command numbered `command`: its refusal, or a
/// selection told through it, which this returns.
fn answer(session: &Session, wakes: &Receiver<()>, command: u64) -> Result<Selection, ()> {
    let refused = |e: &Event| matches!(e, Event::Failed(SessionError::Refused { command: c, .. }) if *c == command);
    let told = |e: &Event| matches!(e, Event::Selection(s) if s.through >= command);
    let events = until(session, wakes, |e| refused(e) || told(e));
    match events.into_iter().find(|e| refused(e) || told(e)) {
        Some(Event::Selection(selection)) => Ok(selection),
        _ => Err(()),
    }
}

/// The folder's reading cancelled once it has sent its last chunk, which the Browser keeps,
/// but before that chunk reaches the engine thread: the cancel changes nothing, so the
/// photos stay the collection, as the Browser shows them.
#[test]
fn a_cancel_after_the_last_chunk_was_sent_keeps_the_listing() {
    let gpu = gpu();
    let folder = three_photos("selection-cancel-late");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    // The reading held with its last chunk claimed, before it sends it.
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold.holding_last_chunk()));
    let open = session.send(open_folder(&folder));
    let added = |e: &Event| matches!(e, Event::Job { event: JobEvent::Added { .. }, command: Some(c) } if *c == open);
    let events = until(&session, &wakes, added);
    let job = job_of(&events, open).expect("its job");
    begun
        .recv_timeout(TEST_WAIT)
        .expect("the last chunk is claimed");
    session.send(Command::CancelJob { job });
    // A refusal after it proves the cancel taken before the chunk is sent.
    let elsewhere = scratch("selection-cancel-late-elsewhere");
    let refused = session.send(select(SelectionCommand::Add(elsewhere.clone())));
    assert!(answer(&session, &wakes, refused).is_err());
    drop(permit);
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    let events = until(&session, &wakes, listed);
    let ended = |e: &Event| matches!(e, Event::Job { event: JobEvent::Ended { job: j, .. }, .. } if *j == job);
    if !events.iter().any(ended) {
        until(&session, &wakes, ended);
    }
    // A, listed, is one of the collection's.
    let set = session.send(select(SelectionCommand::Set(vec![folder.join("A.ARW")])));
    let selection = answer(&session, &wakes, set).expect("A is in the collection");
    assert_eq!(selection.photos, [folder.join("A.ARW")]);
    drop(session);
    remove(&folder.join("A.ARW"));
    remove(&elsewhere);
}

/// A selection command sent before a chunk of the listing and taken in one batch with it
/// is applied before it: B, not yet listed, is refused; sent after it, B is selected.
#[test]
fn a_command_and_a_chunk_in_one_batch_apply_in_the_order_sent() {
    let gpu = gpu();
    let folder = three_photos("selection-chunk-order");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    // A open, so the engine renders, and waits at the gate, as the test holds it.
    let a = folder.join("A.ARW");
    open_live(&session, &wakes, &a);
    let (hold, begun, permit) = export_job::testing::Hold::new();
    session.send(Command::HoldFolder(hold));
    let open = session.send(open_folder(&folder));
    begun.recv_timeout(TEST_WAIT).expect("the reading begins");
    // Held at the gate, the engine takes the command and the chunk after it in one batch.
    let held = one_batch(&gpu, &session, 60);
    let b = folder.join("B.ARW");
    let early = session.send(select(SelectionCommand::Add(b.clone())));
    drop(permit);
    // The chunk reaches the engine's inbox before the client.
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    drop(held);
    assert!(
        answer(&session, &wakes, early).is_err(),
        "B, not yet listed, is refused"
    );
    let late = session.send(select(SelectionCommand::Add(b.clone())));
    let selection = answer(&session, &wakes, late).expect("B, listed, is selected");
    assert_eq!(selection.photos, [a.clone(), b]);
    drop(session);
    remove(&a);
}

/// A photo's change whose writes fail, at its close too, leaves it read as its sidecar
/// holds it: `Unedited` selects it, as nothing was saved. On Unix, by a folder no write
/// lands in, unless the tests run with the rights to write there anyway.
#[cfg(unix)]
#[test]
fn a_photo_whose_write_failed_reads_as_listed() {
    use std::os::unix::fs::PermissionsExt;

    let gpu = gpu();
    let folder = three_photos("selection-unwritten");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let open = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    let a = folder.join("A.ARW");
    open_live(&session, &wakes, &a);
    // No write lands in the folder.
    let mode = |mode| std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(mode));
    mode(0o555).unwrap();
    let probe = folder.join("probe");
    if std::fs::write(&probe, b"").is_ok() {
        // Run with the rights to write anywhere: no write fails here.
        let _ = std::fs::remove_file(&probe);
        mode(0o755).unwrap();
        drop(session);
        remove(&a);
        return;
    }
    session.send(set("brightness", ControlValue::Number(0.2), true));
    let close = session.send(Command::Close);
    told(&session, &wakes, &folder, &["A.ARW"], None, close);
    let unedited = session.send(select(SelectionCommand::Unedited));
    let every = ["A.ARW", "B.ARW", "C.ARW"];
    told_next(&session, &wakes, &folder, &every, None, unedited);
    drop(session);
    mode(0o755).unwrap();
    assert!(!sidecar::path_for(&a).exists());
    remove(&a);
}

/// A turn that panics after a gesture never saved: the photo, closed unsaved, reads as
/// its listing found it, so `Unedited` selects it.
#[test]
fn a_photo_a_panic_closes_mid_gesture_reads_as_listed() {
    let path = scratch("selection-panic-gesture");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let folder = path.parent().unwrap();
    let photo = |name: &str| library::Photo {
        name: name.to_owned(),
        size: 0,
        modified: None,
        edit: library::EditState::None,
        rating: Default::default(),
        fields: Default::default(),
    };
    let selection = &mut driven.engine.selection;
    selection.folder_opened(1, folder);
    selection.listing(&LibraryEvent::Listed {
        open: 1,
        photos: vec![photo("other.ARW"), photo("synthetic.ARW")],
        done: true,
    });
    let events = driven.run([set("brightness", ControlValue::Number(0.5), false)]);
    assert!(values(&events).edited);
    driven.engine.selection_event(&events);
    let payload: Box<dyn std::any::Any + Send> = Box::new("a bug");
    let taken = driven.engine.applied;
    driven.engine.recover(payload.as_ref(), taken);
    // The writer classifies its sidecar after its last write, off the engine thread.
    driven.engine.take_classified(true);
    let unedited = driven
        .engine
        .selection
        .command(SelectionCommand::Unedited, None);
    assert!(matches!(unedited, Ok(Active::Stays)), "{unedited:?}");
    let told = driven.engine.selection.fact(None, taken).expect("told");
    assert_eq!(told.photos, [folder.join("other.ARW"), path.clone()]);
    remove(&path);
}

/// A change saved, then a turn that panics: the photo, closed, reads as its sidecar holds
/// it, edited, so `Unedited` leaves it out.
#[test]
fn a_photo_saved_then_closed_by_a_panic_reads_as_its_sidecar() {
    let path = scratch("selection-panic-saved");
    let mut driven = Driven::new(&[&Sample]);
    driven.open(&path);
    let folder = path.parent().unwrap();
    let photo = |name: &str| library::Photo {
        name: name.to_owned(),
        size: 0,
        modified: None,
        edit: library::EditState::None,
        rating: Default::default(),
        fields: Default::default(),
    };
    let selection = &mut driven.engine.selection;
    selection.folder_opened(1, folder);
    selection.listing(&LibraryEvent::Listed {
        open: 1,
        photos: vec![photo("other.ARW"), photo("synthetic.ARW")],
        done: true,
    });
    driven.run([set("brightness", ControlValue::Number(0.5), true)]);
    driven.writes();
    assert!(sidecar::path_for(&path).exists(), "saved");
    let payload: Box<dyn std::any::Any + Send> = Box::new("a bug");
    let taken = driven.engine.applied;
    driven.engine.recover(payload.as_ref(), taken);
    // The writer classifies its sidecar after its last write, off the engine thread.
    driven.engine.take_classified(true);
    let unedited = driven
        .engine
        .selection
        .command(SelectionCommand::Unedited, None);
    assert!(matches!(unedited, Ok(Active::Stays)), "{unedited:?}");
    let told = driven.engine.selection.fact(None, taken).expect("told");
    assert_eq!(told.photos, [folder.join("other.ARW")]);
    remove(&path);
}

/// The handle dropped with a `CancelJob` queued: the folder's reading, held before it
/// reads, cancelled, the folder holds no photo, so `Unedited` closes A and the change after
/// it is not written; a cancel of the reading that has ended changes nothing, so A stays
/// open and the change is written.
#[test]
fn a_cancel_queued_as_the_handle_drops_counts_only_when_taken() {
    for taken in [true, false] {
        let gpu = gpu();
        let name = if taken {
            "selection-drop-cancel"
        } else {
            "selection-drop-ended"
        };
        let folder = three_photos(name);
        let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
        session.send(Command::Resize {
            width: 64,
            height: 48,
        });
        let open = session.send(open_folder(&folder));
        let added = |open: u64| move |e: &Event| matches!(e, Event::Job { event: JobEvent::Added { .. }, command: Some(c) } if *c == open);
        let job_of = |events: &[Event], open: u64| {
            events.iter().find_map(|e| match e {
                Event::Job {
                    event: JobEvent::Added { job, .. },
                    command: Some(c),
                } if *c == open => Some(*job),
                _ => None,
            })
        };
        let mut events = until(&session, &wakes, added(open));
        let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
        if !events.iter().any(listed) {
            events.extend(until(&session, &wakes, listed));
        }
        let first = job_of(&events, open).expect("its job");
        let a = folder.join("A.ARW");
        events.extend(open_live(&session, &wakes, &a));
        let ended = |e: &Event| matches!(e, Event::Job { event: JobEvent::Ended { job: j, .. }, .. } if *j == first);
        if !events.iter().any(ended) {
            until(&session, &wakes, ended);
        }
        let mut permit = None;
        let job = if taken {
            let (hold, begun, go) = export_job::testing::Hold::new();
            session.send(Command::HoldFolder(hold));
            let again = session.send(open_folder(&folder));
            let events = until(&session, &wakes, added(again));
            begun.recv_timeout(TEST_WAIT).expect("the reading begins");
            permit = Some(go);
            job_of(&events, again).expect("its job")
        } else {
            first
        };
        // Held at the gate, the engine takes what follows only after the handle is gone.
        let held = one_batch(&gpu, &session, 60);
        session.send(Command::CancelJob { job });
        session.send(select(SelectionCommand::Unedited));
        session.send(set("brightness", ControlValue::Number(0.2), true));
        // The held reading lets go as the cancel, or the session's end, reaches it.
        drop(session);
        drop(held);
        drop(permit);
        assert_eq!(sidecar::path_for(&a).exists(), !taken, "taken: {taken}");
        remove(&a);
    }
}

/// A folder of its own holding the raws A, B, C and D, which [`remove`] takes with it.
fn four_photos(name: &str) -> PathBuf {
    let folder = three_photos(name);
    std::fs::write(folder.join("D.ARW"), [0u8; 4]).unwrap();
    folder
}

/// The first selection the session tells through at least the command numbered `through`,
/// which must be `photos`, in `folder`, with `active`, and `previous` whether there is an
/// earlier selection; the events up to it.
fn told_previous(
    session: &Session,
    wakes: &Receiver<()>,
    folder: &Path,
    (photos, active): (&[&str], Option<&str>),
    previous: bool,
    through: u64,
) -> Vec<Event> {
    let after = |e: &Event| matches!(e, Event::Selection(s) if s.through >= through);
    let events = until(session, wakes, after);
    let Some(Event::Selection(selection)) = events.iter().find(|e| after(e)) else {
        unreachable!("`until` stops at it");
    };
    let photos: Vec<PathBuf> = photos.iter().map(|photo| folder.join(photo)).collect();
    assert_eq!(selection.photos, photos, "{through}");
    assert_eq!(
        selection.active,
        active.map(|photo| folder.join(photo)),
        "{through}"
    );
    assert_eq!(selection.previous, previous, "{through}");
    events
}

/// Waits until the photo the command numbered `open` opened is open, numbered so, unless
/// `events`, drained already, say so; what it drains is added to them.
fn opened_by(
    session: &Session,
    wakes: &Receiver<()>,
    events: &mut Vec<Event>,
    path: &Path,
    open: u64,
) {
    let opened =
        |e: &Event| matches!(e, Event::Opened(photo) if photo.open == open && photo.path == path);
    if !events.iter().any(opened) {
        events.extend(until(session, wakes, opened));
    }
}

/// Whether the command numbered `command` was refused for having no earlier selection.
fn refused_as_nothing_earlier(session: &Session, wakes: &Receiver<()>, command: u64) -> bool {
    let refused = |e: &Event| {
        matches!(e, Event::Failed(SessionError::Refused { command: c, error })
            if *c == command && matches!(**error, SessionError::NoPreviousSelection))
    };
    let answered = |e: &Event| {
        refused(e)
            || matches!(e, Event::Failed(SessionError::Refused { command: c, .. }) if *c == command)
            || matches!(e, Event::Selection(s) if s.through >= command)
    };
    until(session, wakes, answered).iter().any(refused)
}

/// Previous Selection (decision 90): refused with nothing earlier, the first photo opened
/// included; each change of the photos selected from a selection, by a command or an open
/// selecting a photo alone, leaves the selection before it to go back to, with its active photo, which opens; a second goes forward again; the
/// active photo moving within the selection, or a command that changes nothing, leaves the
/// earlier one; `Selection::previous` says whether there is one.
#[test]
fn previous_puts_back_the_selection_before_its_last_change() {
    let gpu = gpu();
    let folder = four_photos("selection-previous");
    let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
    session.send(Command::Resize {
        width: 64,
        height: 48,
    });
    let open = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == open);
    until(&session, &wakes, listed);
    let previous = || select(SelectionCommand::Previous);

    // Nothing earlier: refused, and the selection is as it was.
    let refused = session.send(previous());
    assert!(refused_as_nothing_earlier(&session, &wakes, refused));
    // A, the first photo opened, is selected alone: from no photo, nothing to go back to.
    let a = session.send(Command::Open(folder.join("A.ARW")));
    let events = told_previous(
        &session,
        &wakes,
        &folder,
        (&["A.ARW"], Some("A.ARW")),
        false,
        a,
    );
    go_live(&session, &wakes, &events, a);
    let refused = session.send(previous());
    assert!(refused_as_nothing_earlier(&session, &wakes, refused));
    let abc = ["A.ARW", "B.ARW", "C.ARW"];
    let set = select(SelectionCommand::Set(abc.map(|p| folder.join(p)).to_vec()));
    let set = session.send(set);
    told_previous(&session, &wakes, &folder, (&abc, Some("A.ARW")), true, set);
    // C made active within the selection records nothing: Previous goes back to A alone,
    // as before the Set, not to the three with A active.
    let c = session.send(Command::Open(folder.join("C.ARW")));
    let events = told_previous(&session, &wakes, &folder, (&abc, Some("C.ARW")), true, c);
    go_live(&session, &wakes, &events, c);
    let back = session.send(previous());
    let mut events = told_previous(
        &session,
        &wakes,
        &folder,
        (&["A.ARW"], Some("A.ARW")),
        true,
        back,
    );
    opened_by(&session, &wakes, &mut events, &folder.join("A.ARW"), back);
    go_live(&session, &wakes, &events, back);
    // A second goes forward again, C opening with the three.
    let forward = session.send(previous());
    let mut events = told_previous(
        &session,
        &wakes,
        &folder,
        (&abc, Some("C.ARW")),
        true,
        forward,
    );
    opened_by(
        &session,
        &wakes,
        &mut events,
        &folder.join("C.ARW"),
        forward,
    );
    go_live(&session, &wakes, &events, forward);
    // B, selected already, added changes nothing and records nothing: Previous still goes
    // back to A alone.
    session.send(select(SelectionCommand::Add(folder.join("B.ARW"))));
    let back = session.send(previous());
    let events = told_previous(
        &session,
        &wakes,
        &folder,
        (&["A.ARW"], Some("A.ARW")),
        true,
        back,
    );
    go_live(&session, &wakes, &events, back);
    let forward = session.send(previous());
    let events = told_previous(
        &session,
        &wakes,
        &folder,
        (&abc, Some("C.ARW")),
        true,
        forward,
    );
    go_live(&session, &wakes, &events, forward);

    // Select None, then Previous: the three come back, C active and open again.
    let none = session.send(select(SelectionCommand::Clear));
    told_previous(&session, &wakes, &folder, (&[], None), true, none);
    let back = session.send(previous());
    let mut events = told_previous(&session, &wakes, &folder, (&abc, Some("C.ARW")), true, back);
    opened_by(&session, &wakes, &mut events, &folder.join("C.ARW"), back);
    go_live(&session, &wakes, &events, back);
    // A plain Set of one, then Previous: the three again.
    let one = select(SelectionCommand::Set(vec![folder.join("C.ARW")]));
    let one = session.send(one);
    told_previous(
        &session,
        &wakes,
        &folder,
        (&["C.ARW"], Some("C.ARW")),
        true,
        one,
    );
    let back = session.send(previous());
    told_previous(&session, &wakes, &folder, (&abc, Some("C.ARW")), true, back);

    // D opened, outside the selection, selects it alone; C removed from the folder, listed
    // again, is left out of what comes back, and B, the nearest to it, is active.
    let d = session.send(Command::Open(folder.join("D.ARW")));
    let events = told_previous(
        &session,
        &wakes,
        &folder,
        (&["D.ARW"], Some("D.ARW")),
        true,
        d,
    );
    go_live(&session, &wakes, &events, d);
    std::fs::remove_file(folder.join("C.ARW")).unwrap();
    let again = session.send(open_folder(&folder));
    let listed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Listed { open: o, done: true, .. }) if *o == again);
    until(&session, &wakes, listed);
    let back = session.send(previous());
    let ab = ["A.ARW", "B.ARW"];
    let mut events = told_previous(&session, &wakes, &folder, (&ab, Some("B.ARW")), true, back);
    opened_by(&session, &wakes, &mut events, &folder.join("B.ARW"), back);
    go_live(&session, &wakes, &events, back);

    // Another folder: no earlier selection, and Previous is refused.
    let other = three_photos("selection-previous-other");
    let other_open = session.send(open_folder(&other));
    told_previous(
        &session,
        &wakes,
        &folder,
        (&["B.ARW"], Some("B.ARW")),
        false,
        other_open,
    );
    let refused = session.send(previous());
    assert!(refused_as_nothing_earlier(&session, &wakes, refused));
    drop(session);
    remove(&folder.join("A.ARW"));
    remove(&other.join("A.ARW"));
}
