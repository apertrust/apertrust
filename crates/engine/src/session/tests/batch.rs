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

//! A style on several photos (decision 109): one `ApplyStyle` naming them, one batch on
//! every photo it changes, the active one at once and the others by the `apply-style` job,
//! each routed by the engine thread, which the tests stand in for by pumping its inbox.

use apertrust_tools::grain::Grain;
use apertrust_tools::soften::Soften;
use apertrust_tools::vignetting::Vignetting;

use super::*;
use crate::history::{BatchId, Log, StepSource};

/// The tools of these tests, each switched, so a style turns it on.
const TOOLS: &[&dyn ProcessingTool] = &[&Grain, &Soften, &Vignetting];

fn on(control: &str, value: f32) -> ToolValues {
    let mut values = ToolValues::new();
    values.set(control, ControlValue::Number(value));
    values.set_enabled(true);
    values
}

/// `ApplyStyle` of Warm film, Grain at `strength`, on `photos`.
fn warm(strength: f32, photos: &[PathBuf]) -> Command {
    Command::ApplyStyle {
        name: "Warm film".to_owned(),
        file: "Warm film.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("grain".to_owned(), on("strength", strength))],
        photos: photos.to_vec(),
    }
}

/// Cool film, Soften at `amount`, on the active photo alone.
fn cool(amount: f32) -> Command {
    Command::ApplyStyle {
        name: "Cool film".to_owned(),
        file: "Cool film.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("soften".to_owned(), on("amount", amount))],
        photos: Vec::new(),
    }
}

/// `count` raws beside `active`, each a file on the disk, as `<name><n>.ARW`.
fn photos(active: &Path, name: &str, count: usize) -> Vec<PathBuf> {
    (0..count)
        .map(|n| {
            let photo = active.with_file_name(format!("{name}{n}.ARW"));
            std::fs::write(&photo, b"a raw").unwrap();
            photo
        })
        .collect()
}

/// The log the sidecar beside `photo` holds, none where there is no sidecar.
fn saved_log(photo: &Path) -> Log {
    let path = sidecar::path_for(photo);
    match path.exists() {
        true => sidecar::read(&path, OPERATIONS, TOOLS, &[]).unwrap().log,
        false => Log::default(),
    }
}

/// The batches a log's steps of source Style are in, in order, each once.
fn batches(log: &Log) -> Vec<BatchId> {
    let mut batches = Vec::new();
    for step in log.steps() {
        if let (StepSource::Style, Some(batch)) = (step.source, step.batch)
            && batches.last() != Some(&batch)
        {
            batches.push(batch);
        }
    }
    batches
}

/// The engine thread's inbox pumped, the batches' photos routed as it routes them, until
/// the job's `Ended` comes; every event heard, the outbox's among them.
fn until_ended(driven: &mut Driven) -> Vec<Event> {
    let mut heard = Vec::new();
    let deadline = Instant::now() + TEST_WAIT;
    loop {
        heard.extend(driven.pump_for(Duration::from_millis(10)));
        heard.extend(driven.saved.events());
        if ended(&heard).is_some() {
            return heard;
        }
        assert!(Instant::now() < deadline, "the batch ends: {heard:?}");
    }
}

/// The next photo a batch's job hands the engine thread, not yet routed; what else arrives
/// meanwhile taken in.
fn next_route(driven: &mut Driven) -> crate::session::batch::Route {
    loop {
        match driven
            .inbox
            .recv_timeout(TEST_WAIT)
            .expect("a photo handed over")
        {
            Inbox::Route(route) => return *route,
            Inbox::Open(stage) => {
                driven.engine.stage(*stage);
            }
            Inbox::Task(waiting) => driven.engine.jobs.background.add(waiting),
            _ => {}
        }
    }
}

/// The batch's `Ended` among `events`.
fn ended(events: &[Event]) -> Option<&BatchEvent> {
    events.iter().find_map(|event| match event {
        Event::Batch(ended @ BatchEvent::Ended { .. }) => Some(ended),
        _ => None,
    })
}

/// The photos the batch's `Changed` events name, in order.
fn changed(events: &[Event]) -> Vec<PathBuf> {
    (events.iter())
        .filter_map(|event| match event {
            Event::Batch(BatchEvent::Changed { photo, .. }) => Some(photo.clone()),
            _ => None,
        })
        .collect()
}

/// A style on three photos, one of them open: the open one changed in the command's turn,
/// the two others by the job, all three sidecars holding the style's steps in one batch,
/// the job quiet, and the batch's `Ended` naming all three.
#[test]
fn a_style_on_three_photos_one_open_is_one_batch_in_three_sidecars() {
    let active = scratch("batch-three");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let named = [vec![active.clone()], others.clone()].concat();
    let events = driven.run([warm(40.0, &named)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    // The active photo at once: its change in the turn, its batch on its log.
    assert_eq!(changed(&events), std::slice::from_ref(&active));
    let open = driven.engine.open.as_ref().unwrap();
    let ours = batches(open.history.log());
    assert_eq!(ours.len(), 1);
    let heard = until_ended(&mut driven);
    let added = heard.iter().find_map(|event| match event {
        Event::Job {
            event: JobEvent::Added { kind, quiet, .. },
            ..
        } => Some((*kind, *quiet)),
        _ => None,
    });
    assert_eq!(added, Some(("apply-style", true)));
    assert_eq!(changed(&heard), others);
    driven.writes();
    for photo in &named {
        assert_eq!(batches(&saved_log(photo)), ours, "{}", photo.display());
    }
    let Some(BatchEvent::Ended {
        batch,
        changed,
        skipped,
        cancelled,
        action,
        ..
    }) = ended(&heard)
    else {
        unreachable!()
    };
    assert_eq!(*batch, ours[0]);
    assert_eq!((changed.len(), skipped.len()), (3, 0));
    assert_eq!((*cancelled, *action), (false, BatchAction::Applied));
    remove(&active);
}

/// A photo whose edit opens for viewing only, a newer Apertrust's, is skipped and named,
/// its file untouched; the others change, and the command is not refused.
#[test]
fn a_view_only_photo_is_skipped_and_named() {
    let active = scratch("batch-view-only");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let newer = sidecar::to_xml(&Edit::default(), OPERATIONS, TOOLS, "9.0.0")
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
    let view_only = sidecar::path_for(&others[0]);
    std::fs::write(&view_only, &newer).unwrap();
    let events = driven.run([warm(40.0, &others)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended {
        changed, skipped, ..
    }) = ended(&heard)
    else {
        unreachable!()
    };
    assert_eq!(changed, &others[1..]);
    assert!(
        matches!(&skipped[..], [(photo, Skip::ViewOnly(_))] if *photo == others[0]),
        "{skipped:?}"
    );
    assert_eq!(std::fs::read_to_string(&view_only).unwrap(), newer);
    remove(&active);
}

/// 200 photos, the job's progress counting them, cancelled midway: the photos done keep
/// the batch, all in one, and the rest are untouched.
#[test]
fn a_batch_cancelled_midway_keeps_the_photos_done_in_one_batch() {
    let active = scratch("batch-cancel");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 200);
    let events = driven.run([warm(40.0, &others)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let deadline = Instant::now() + TEST_WAIT;
    let job = loop {
        assert!(Instant::now() < deadline, "the job is added");
        let heard = driven.saved.events();
        let added = heard.iter().find_map(|event| match event {
            Event::Job {
                event: JobEvent::Added { job, progress, .. },
                ..
            } => Some((*job, progress.total)),
            _ => None,
        });
        if let Some((job, total)) = added {
            assert_eq!(total, Some(200));
            break job;
        }
    };
    // Ten photos routed, then the cancel.
    for _ in 0..10 {
        let route = next_route(&mut driven);
        driven.engine.routes.push(route);
        driven.engine.take_routes();
    }
    driven.run([Command::CancelJob { job }]);
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended {
        batch,
        changed,
        cancelled,
        ..
    }) = ended(&heard)
    else {
        unreachable!()
    };
    assert!(*cancelled);
    assert!((10..200).contains(&changed.len()), "{}", changed.len());
    driven.writes();
    for photo in &others {
        let found = batches(&saved_log(photo));
        match changed.contains(photo) {
            true => assert_eq!(found, [*batch], "{}", photo.display()),
            false => assert!(found.is_empty(), "{}", photo.display()),
        }
    }
    remove(&active);
}

/// A photo opened while the job runs, before it is reached, is changed live, as the active
/// photo is, once; and a style applied there before the batch reached it lands after the
/// batch's, as on every other photo where both land.
#[test]
fn a_photo_opened_during_the_job_takes_the_batch_before_a_later_style() {
    let active = scratch("batch-opened");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    driven.run([warm(40.0, &others)]);
    // The job holds its first photo's route; the second, not reached, opens.
    let first = next_route(&mut driven);
    driven.open(&others[1]);
    let events = driven.run([cool(30.0)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(changed(&events), std::slice::from_ref(&others[1]));
    let log = driven.engine.open.as_ref().unwrap().history.log().clone();
    let notes: Vec<_> = (log.steps().iter())
        .filter(|step| step.source == StepSource::Style)
        .map(|step| step.note.clone())
        .collect();
    assert_eq!(
        notes,
        [
            Some(crate::history::Note::style("Warm film")),
            Some(crate::history::Note::style("Cool film"))
        ]
    );
    driven.engine.routes.push(first);
    driven.engine.take_routes();
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    // The open one by the engine thread first, then the first by the job.
    assert_eq!(changed, &[others[1].clone(), others[0].clone()]);
    // Reached live already: the job left it, the batch on it once.
    let log = driven.engine.open.as_ref().unwrap().history.log().clone();
    assert_eq!(batches(&log).len(), 2);
    remove(&active);
}

/// A file another program changes between the job's read and its write is not written
/// over: the job reads it again and lays the batch on what it holds.
#[test]
fn a_file_changed_between_its_read_and_its_write_is_read_again() {
    let active = scratch("batch-changed");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    driven.run([warm(40.0, &others)]);
    let route = next_route(&mut driven);
    // Another program's edit, Vignetting on, written after the job read the file.
    let mut edit = Edit::default();
    let image = testing::image();
    (edit.set_tool(&Vignetting, &on("scale", 50.0), &image)).unwrap();
    let xml = sidecar::to_xml(&edit, OPERATIONS, TOOLS, BUILD);
    std::fs::write(sidecar::path_for(&others[0]), xml).unwrap();
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended {
        changed, skipped, ..
    }) = ended(&heard)
    else {
        unreachable!()
    };
    assert_eq!((changed, skipped.len()), (&others, 0));
    let log = saved_log(&others[0]);
    let fold = log.fold();
    assert!(fold.tool("vignette").is_some(), "{log:?}");
    assert!(fold.tool("grain").is_some(), "{log:?}");
    assert_eq!(batches(&log).len(), 1);
    remove(&active);
}

/// With only the active photo named there is no job: its `Ended` comes in the command's
/// turn; a value it refuses skips it, named, and the command is not refused.
#[test]
fn the_active_photo_alone_ends_in_the_turn_and_a_refusal_skips_it() {
    let active = scratch("batch-alone");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let events = driven.run([warm(40.0, std::slice::from_ref(&active))]);
    let Some(BatchEvent::Ended { changed, .. }) = ended(&events) else {
        panic!("{events:?}")
    };
    assert_eq!(changed, std::slice::from_ref(&active));
    let refused = Command::ApplyStyle {
        name: "Odd".to_owned(),
        file: "Odd.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("nothing".to_owned(), on("amount", 1.0))],
        photos: vec![active.clone()],
    };
    let events = driven.run([refused]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let Some(BatchEvent::Ended { skipped, .. }) = ended(&events) else {
        panic!("{events:?}")
    };
    assert!(
        matches!(&skipped[..], [(photo, Skip::Refused(_))] if *photo == active),
        "{skipped:?}"
    );
    remove(&active);
}

/// One batch job at a time, in the order sent: a second batch on the same photo waits for
/// the first, so its steps land after the first's.
#[test]
fn batch_jobs_run_one_at_a_time_in_the_order_sent() {
    let active = scratch("batch-lane");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    driven.run([warm(40.0, &others)]);
    let faded = Command::ApplyStyle {
        name: "Faded".to_owned(),
        file: "Faded.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("soften".to_owned(), on("amount", 20.0))],
        photos: others.clone(),
    };
    driven.run([faded]);
    let mut heard = until_ended(&mut driven);
    while (heard.iter())
        .filter(|event| matches!(event, Event::Batch(BatchEvent::Ended { .. })))
        .count()
        < 2
    {
        heard.extend(until_ended(&mut driven));
    }
    let styles: Vec<_> = (heard.iter())
        .filter_map(|event| match event {
            Event::Batch(BatchEvent::Ended { style, batch, .. }) => Some((style.clone(), *batch)),
            _ => None,
        })
        .collect();
    assert_eq!(styles[0].0, "Warm film");
    assert_eq!(styles[1].0, "Faded");
    driven.writes();
    assert_eq!(batches(&saved_log(&others[0])), [styles[0].1, styles[1].1]);
    remove(&active);
}

/// A photo that opens while its file is handed over waits, its job reading it again once it
/// is live, where the engine thread applies the batch to it there, once.
#[test]
fn a_photo_opening_as_it_is_handed_over_takes_the_batch_once_live() {
    let active = scratch("batch-routed-opening");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    driven.run([warm(40.0, &others)]);
    let route = next_route(&mut driven);
    // It opens, not yet live, as the route reaches the engine thread.
    driven.run([Command::Open(others[0].clone())]);
    assert!(driven.engine.opening.is_some());
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    driven.settle();
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    assert_eq!(changed, &others);
    let log = driven.engine.open.as_ref().unwrap().history.log().clone();
    assert_eq!(batches(&log).len(), 1, "{log:?}");
    remove(&active);
}

/// A write over the file as read that the writer drops, as its photo's generation rose
/// meanwhile, is routed again and lands.
#[test]
fn a_dropped_write_is_routed_again() {
    let active = scratch("batch-routed-dropped");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    driven.run([warm(40.0, &others)]);
    let route = next_route(&mut driven);
    let permit = driven.engine.writer.hold();
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    // Its photo watched meanwhile, as an open does, then left: the write queued is dropped.
    driven
        .engine
        .writer
        .files()
        .raise(&sidecar::path_for(&others[0]));
    drop(permit);
    let again = next_route(&mut driven);
    assert_eq!(again.photo(), others[0].as_path());
    driven.engine.routes.push(again);
    driven.engine.take_routes();
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    assert_eq!(changed, &others);
    driven.writes();
    assert_eq!(batches(&saved_log(&others[0])).len(), 1);
    remove(&active);
}

/// A batch waiting for the one before holds no job's worker: it has no job until the engine
/// thread starts it, once the one before ends, cancelled or not, so its steps land after
/// that one's.
#[test]
fn a_waiting_batch_starts_after_the_one_before_is_cancelled() {
    let active = scratch("batch-lane-cancel");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 3);
    let events = driven.run([warm(40.0, &others)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let faded = Command::ApplyStyle {
        name: "Faded".to_owned(),
        file: "Faded.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("soften".to_owned(), on("amount", 20.0))],
        photos: others.clone(),
    };
    driven.run([faded]);
    // One job runs, the first's.
    let jobs: Vec<_> = (driven.engine.batches.iter())
        .map(|pending| pending.job())
        .collect();
    let [Some(first), None] = jobs[..] else {
        panic!("{jobs:?}")
    };
    let route = next_route(&mut driven);
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    driven.run([Command::CancelJob { job: first }]);
    let mut heard = until_ended(&mut driven);
    let count = |heard: &[Event]| {
        let ended = |event: &&Event| matches!(event, Event::Batch(BatchEvent::Ended { .. }));
        heard.iter().filter(ended).count()
    };
    while count(&heard) < 2 {
        heard.extend(until_ended(&mut driven));
    }
    let Some(BatchEvent::Ended { changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    driven.writes();
    // Where both landed, Faded after Warm film.
    for photo in changed {
        let log = saved_log(photo);
        let notes: Vec<_> = (log.steps().iter())
            .filter(|step| step.source == StepSource::Style)
            .map(|step| step.note.clone())
            .collect();
        assert_eq!(
            notes,
            [
                Some(crate::history::Note::style("Warm film")),
                Some(crate::history::Note::style("Faded"))
            ]
        );
    }
    remove(&active);
}

/// A batch job cancelled while still queued, every worker busy, ends without its body: the
/// engine thread ends it, so the batch after it runs, and the cancelled one lands nowhere.
#[test]
fn a_batch_cancelled_while_queued_lets_the_next_run() {
    let active = scratch("batch-cancel-queued");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let first = photos(&active, "first", 2);
    let second = photos(&active, "second", 1);
    // Every worker held.
    let (told, _heard) = Heard::start();
    let mut gos = Vec::new();
    for n in 0..apertrust_base::jobs::workers(64) {
        let label: &'static str = Box::leak(format!("busy {n}").into_boxed_str());
        let (job, go) = Synthetic::new(label, Queue::UserForeground, 1, &told).held();
        driven.run([Command::Synthetic(job)]);
        gos.push(go);
    }
    driven.run([warm(40.0, &first)]);
    let faded = Command::ApplyStyle {
        name: "Faded".to_owned(),
        file: "Faded.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("soften".to_owned(), on("amount", 20.0))],
        photos: second.clone(),
    };
    driven.run([faded]);
    let job = driven.engine.batches[0].job().expect("its job queued");
    driven.run([Command::CancelJob { job }]);
    // The turn after the cancel ends it and starts the next.
    let events = driven.engine.start_batches();
    let Some(BatchEvent::Ended {
        cancelled, changed, ..
    }) = ended(&events)
    else {
        panic!("{events:?}")
    };
    assert!(*cancelled);
    assert!(changed.is_empty());
    assert!(driven.engine.batches[0].job().is_some());
    drop(gos);
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { style, changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    assert_eq!((style.as_str(), changed), ("Faded", &second));
    driven.writes();
    assert!(
        first
            .iter()
            .all(|photo| batches(&saved_log(photo)).is_empty())
    );
    remove(&active);
}

/// A batch cancelled reaches no more photos: one it had not reached, opened and edited
/// after the cancel, never takes it, though its job is still finishing.
#[test]
fn a_cancelled_batch_catches_up_no_photo() {
    let active = scratch("batch-cancel-catch-up");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 3);
    driven.run([warm(40.0, &others)]);
    let job = driven.engine.batches[0].job().expect("its job");
    // The job holds its first photo's route, so it has not ended.
    let route = next_route(&mut driven);
    driven.run([Command::CancelJob { job }]);
    // The last, not reached, opened and edited while the job still runs.
    driven.open(&others[2]);
    driven.run([Command::SetTool {
        tool: "soften".to_owned(),
        values: on("amount", 10.0),
    }]);
    let open = driven.engine.open.as_ref().unwrap();
    assert!(batches(open.history.log()).is_empty());
    // Then the route held, and the job's end.
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended {
        changed, cancelled, ..
    }) = ended(&heard)
    else {
        unreachable!()
    };
    assert!(*cancelled);
    assert!(!changed.contains(&others[2]));
    remove(&active);
}
