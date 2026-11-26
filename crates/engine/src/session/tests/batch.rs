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

/// 200 photos, the job's progress counting them, cancelled midway: the ten handed over keep
/// the batch, all in one, and the rest are untouched, the eleventh's route among them.
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
    // Ten photos routed, then the cancel while the job holds the eleventh's route, which
    // it then hands over too late.
    for _ in 0..10 {
        let route = next_route(&mut driven);
        driven.engine.routes.push(route);
        driven.engine.take_routes();
    }
    let eleventh = next_route(&mut driven);
    driven.run([Command::CancelJob { job }]);
    driven.engine.routes.push(eleventh);
    driven.engine.take_routes();
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
    assert_eq!(changed, &others[..10]);
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
            Event::Batch(BatchEvent::Ended { what, batch, .. }) => Some((what.clone(), *batch)),
            _ => None,
        })
        .collect();
    assert_eq!(styles[0].0, Batched::Style("Warm film".to_owned()));
    assert_eq!(styles[1].0, Batched::Style("Faded".to_owned()));
    driven.writes();
    assert_eq!(batches(&saved_log(&others[0])), [styles[0].1, styles[1].1]);
    remove(&active);
}

/// A batch's `Ended`: its action, the photos it changed and those it skipped.
type Ends = (BatchAction, Vec<PathBuf>, Vec<(PathBuf, Skip)>);

/// The `Ended`s among `events`, in order.
fn ends(events: &[Event]) -> Vec<Ends> {
    (events.iter())
        .filter_map(|event| match event {
            Event::Batch(BatchEvent::Ended {
                action,
                changed,
                skipped,
                ..
            }) => Some((*action, changed.clone(), skipped.clone())),
            _ => None,
        })
        .collect()
}

/// Pumps until the `n`th batch `Ended` is heard.
fn until_ends(driven: &mut Driven, n: usize) -> Vec<Event> {
    let mut heard = until_ended(driven);
    while ends(&heard).len() < n {
        heard.extend(until_ended(driven));
    }
    heard
}

/// Whether the log beside `photo`, as saved, holds Grain at its end.
fn grained(photo: &Path) -> bool {
    saved_log(photo).fold().tool("grain").is_some()
}

/// A style on three photos, one open: Undo there names the three photos, takes the batch
/// back there and, by an `undo-batch` job, on the two others; Redo puts it back on all.
#[test]
fn undo_takes_a_batch_back_on_every_photo_and_redo_puts_it_back() {
    let active = scratch("batch-undo");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let named = [vec![active.clone()], others.clone()].concat();
    let events = driven.run([warm(40.0, &named)]);
    let history = events.iter().rev().find_map(|event| match event {
        Event::History(history) => Some(history.clone()),
        _ => None,
    });
    let undo = history.and_then(|history| history.undo);
    assert!(
        matches!(undo, Some(Move::Steps { photos: 3, .. })),
        "{undo:?}"
    );
    until_ended(&mut driven);
    driven.writes();
    assert!(others.iter().all(|photo| grained(photo)));
    let undone = driven.run([Command::Undo]);
    assert!(failures(&undone).is_empty(), "{undone:?}");
    // Redo names the photos it would put the batch back on, its steps held outside the log.
    let redo = undone.iter().rev().find_map(|event| match event {
        Event::History(history) => Some(history.redo),
        _ => None,
    });
    assert!(
        matches!(redo, Some(Some(Move::Steps { photos: 3, .. }))),
        "{redo:?}"
    );
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed.len(), skipped.len()), (3, 0));
    let added = heard.iter().find_map(|event| match event {
        Event::Job {
            event: JobEvent::Added { kind, quiet, .. },
            ..
        } => Some((*kind, *quiet)),
        _ => None,
    });
    assert_eq!(added, Some(("undo-batch", true)));
    driven.writes();
    assert!(others.iter().all(|photo| !grained(photo)));
    let open = driven.engine.open.as_ref().unwrap();
    assert!(open.edit.tool("grain").is_none());
    // Redo puts it back on each.
    driven.run([Command::Redo]);
    let heard = until_ended(&mut driven);
    assert_eq!(ends(&heard)[0].0, BatchAction::PutBack);
    driven.writes();
    assert!(others.iter().all(|photo| grained(photo)));
    remove(&active);
}

/// A photo of the batch changed since keeps the batch under its own step, and is named;
/// the notice's Undo, `UndoBatch`, takes it back from a photo outside the batch; a batch
/// the session does not remember is refused.
#[test]
fn a_photo_changed_since_keeps_the_batch_and_undo_batch_works_from_any_photo() {
    let active = scratch("batch-undo-changed");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    driven.run([warm(40.0, &others)]);
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { batch, .. }) = ended(&heard) else {
        unreachable!()
    };
    let batch = *batch;
    // The first changed since: opened, Soften set, left again.
    driven.open(&others[0]);
    driven.run([Command::SetTool {
        tool: "soften".to_owned(),
        values: on("amount", 10.0),
    }]);
    driven.open(&active);
    driven.writes();
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!(changed, std::slice::from_ref(&others[1]));
    assert!(
        matches!(&skipped[..], [(photo, Skip::ChangedSince)] if *photo == others[0]),
        "{skipped:?}"
    );
    driven.writes();
    assert!(grained(&others[0]));
    assert!(!grained(&others[1]));
    let unknown = BatchId(7);
    let refused = driven.run([Command::UndoBatch { batch: unknown }]);
    assert!(
        matches!(&failures(&refused)[..], [SessionError::UnknownBatch { batch }] if *batch == unknown),
        "{refused:?}"
    );
    remove(&active);
}

/// `UndoBatch` from a photo of the batch open for viewing only, its next Undo the batch's:
/// that photo is skipped and named, its file untouched, and the batch taken back on the
/// two others.
#[test]
fn undo_batch_from_a_view_only_photo_skips_it_and_takes_the_others_back() {
    let active = scratch("batch-undo-view-only");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([warm(40.0, &named)]);
    until_let_go(&mut driven);
    let batch = driven.engine.remembered.back().unwrap().batch.id;
    // Grain at a version above its own: newer, the batch's step still read.
    driven.open(&others[0]);
    let sidecar = sidecar::path_for(&active);
    let xml = std::fs::read_to_string(&sidecar).unwrap().replace(
        r#"apertrust:toolVersion="2""#,
        r#"apertrust:toolVersion="3""#,
    );
    std::fs::write(&sidecar, &xml).unwrap();
    driven.open(&active);
    let open = driven.engine.open.as_ref().unwrap();
    assert!(open.view_only.is_some());
    assert!(open.history.moves_batch(batch, false));
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!(changed, &others);
    assert!(
        matches!(&skipped[..], [(photo, Skip::ViewOnly(_))] if *photo == active),
        "{skipped:?}"
    );
    driven.writes();
    assert!(others.iter().all(|photo| !grained(photo)));
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), xml);
    remove(&active);
}

/// `UndoBatch` on the active photo while a later batch still has it to come: the later one
/// applied there first, as before any command on the edit, so there, as on the others, the
/// batch is under it, kept and named as changed since.
#[test]
fn undo_batch_catches_a_later_batch_up_first() {
    let active = scratch("batch-undo-catch-up");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    driven.run([warm(40.0, &others)]);
    until_let_go(&mut driven);
    let batch = driven.engine.remembered.back().unwrap().batch.id;
    // Faded on both, its job holding the first's route; that photo opens.
    driven.run([Command::ApplyStyle {
        name: "Faded".to_owned(),
        file: "Faded.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("soften".to_owned(), on("amount", 20.0))],
        photos: others.clone(),
    }]);
    let faded = driven.engine.remembered.back().unwrap().batch.id;
    let route = next_route(&mut driven);
    assert_eq!(route.photo(), others[0].as_path());
    driven.open(&others[0]);
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    let heard = until_ends(&mut driven, 2);
    let taken = ends(&heard)
        .into_iter()
        .find(|(action, ..)| *action == BatchAction::TakenBack);
    let Some((_, changed, skipped)) = taken else {
        panic!("{heard:?}")
    };
    assert!(changed.is_empty(), "{changed:?}");
    let since: Vec<_> = (skipped.iter())
        .filter(|(_, skip)| *skip == Skip::ChangedSince)
        .map(|(photo, _)| photo.clone())
        .collect();
    assert_eq!(since, others);
    let open = driven.engine.open.as_ref().unwrap();
    assert_eq!(batches(open.history.log()), [batch, faded]);
    assert!(open.edit.tool("grain").is_some());
    remove(&active);
}

/// `UndoBatch` while the active photo, one of the batch's, opens, its history read and its
/// decode not done: not refused, that photo left to the job, which takes the batch back
/// there once it is live.
#[test]
fn undo_batch_while_the_active_photo_opens_takes_it_back_once_live() {
    let active = scratch("batch-undo-opening");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([warm(40.0, &named)]);
    until_let_go(&mut driven);
    driven.writes();
    let batch = driven.engine.remembered.back().unwrap().batch.id;
    let (hold, begun, permit) = export_job::testing::Hold::new();
    driven.run([Command::HoldDecode(hold)]);
    driven.run([Command::Open(others[0].clone())]);
    driven.pump(|_, events| events.iter().any(|e| matches!(e, Event::History(_))));
    begun.recv_timeout(TEST_WAIT).expect("the decode begins");
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    permit.send(()).unwrap();
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed, skipped.len()), (&named, 0), "{skipped:?}");
    let open = driven.engine.open.as_ref().unwrap();
    assert!(open.edit.tool("grain").is_none());
    driven.writes();
    assert!(named.iter().all(|photo| !grained(photo)));
    remove(&active);
}

/// A photo of the batch opened for viewing only while an Undo's job is under way, reached
/// live by it, is skipped and named as view-only, its file untouched.
#[test]
fn a_view_only_photo_reached_live_by_an_undo_is_named_view_only() {
    let active = scratch("batch-undo-live-view-only");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([warm(40.0, &named)]);
    until_let_go(&mut driven);
    driven.writes();
    driven.run([Command::Undo]);
    // The job holds the first other photo's route; that photo, made view-only, opens.
    let route = next_route(&mut driven);
    let sidecar = sidecar::path_for(&others[0]);
    let xml = std::fs::read_to_string(&sidecar).unwrap().replace(
        r#"apertrust:toolVersion="2""#,
        r#"apertrust:toolVersion="3""#,
    );
    std::fs::write(&sidecar, &xml).unwrap();
    driven.open(&others[0]);
    assert!(driven.engine.open.as_ref().unwrap().view_only.is_some());
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!(changed, &[active.clone(), others[1].clone()]);
    assert!(
        matches!(&skipped[..], [(photo, Skip::ViewOnly(_))] if *photo == others[0]),
        "{skipped:?}"
    );
    driven.writes();
    assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), xml);
    remove(&active);
}

/// An Undo's write over a photo not open that lands as the photo opens, before the job hears
/// of it, the photo edited at once: where the write left it, it is reached, not changed
/// since.
#[test]
fn an_undo_s_write_landing_as_its_photo_opens_is_reached() {
    let active = scratch("batch-undo-landed");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([warm(40.0, &named)]);
    until_let_go(&mut driven);
    driven.writes();
    driven.run([Command::Undo]);
    // The job's write lands, as the writer makes it; the photo opens and is edited.
    let route = next_route(&mut driven);
    std::fs::write(sidecar::path_for(&others[0]), route.xml()).unwrap();
    driven.open(&others[0]);
    let events = driven.run([Command::SetTool {
        tool: "soften".to_owned(),
        values: on("amount", 10.0),
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed, skipped.len()), (&named, 0), "{skipped:?}");
    let open = driven.engine.open.as_ref().unwrap();
    assert!(open.edit.tool("grain").is_none());
    assert!(open.edit.tool("soften").is_some());
    remove(&active);
}

/// Undo of a batch whose job still runs cancels the job first, then takes the batch back on
/// the photos it reached.
#[test]
fn undo_of_a_running_batch_cancels_its_job_first() {
    let active = scratch("batch-undo-running");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 20);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([warm(40.0, &named)]);
    // Three photos reached, then Undo on the active one.
    for _ in 0..3 {
        let route = next_route(&mut driven);
        driven.engine.routes.push(route);
        driven.engine.take_routes();
    }
    driven.run([Command::Undo]);
    let heard = until_ends(&mut driven, 2);
    let ended = ends(&heard);
    let (applied, taken) = (&ended[0], &ended[1]);
    assert_eq!(
        (applied.0, taken.0),
        (BatchAction::Applied, BatchAction::TakenBack)
    );
    let cancelled = heard.iter().any(|event| {
        matches!(
            event,
            Event::Batch(BatchEvent::Ended {
                action: BatchAction::Applied,
                cancelled: true,
                ..
            })
        )
    });
    assert!(cancelled);
    // Every photo the batch changed taken back, the active one among them.
    let mut reached = applied.1.clone();
    let mut back = taken.1.clone();
    reached.sort();
    back.sort();
    assert_eq!(reached, back);
    driven.writes();
    assert!(others.iter().all(|photo| !grained(photo)));
    remove(&active);
}

/// `UndoBatch` while the batch's write of a photo is handed over, landing after it: that
/// photo, opened and edited before the Undo's job starts, has the Undo caught up first, as
/// before any command on the edit, so the batch is taken back there before the edit, not
/// left under it as changed since.
#[test]
fn a_take_back_made_as_its_batch_writes_a_photo_lands_there_before_an_edit() {
    let active = scratch("batch-undo-in-flight");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    driven.run([warm(40.0, &others)]);
    let batch = driven.engine.remembered.back().unwrap().batch.id;
    // The first photo's write handed over and held, the Undo made meanwhile: the writer held
    // once the job has read the file, so its read never waits behind the hold.
    let route = next_route(&mut driven);
    let permit = driven.engine.writer.hold();
    driven.engine.routes.push(route);
    driven.engine.take_routes();
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    // The write lands after it, and the batch's job hears of it.
    drop(permit);
    let deadline = Instant::now() + TEST_WAIT;
    while !changed(&driven.saved.events()).contains(&others[0]) {
        assert!(Instant::now() < deadline, "the write lands");
        std::thread::sleep(Duration::from_millis(5));
    }
    // The photo opens and is edited, the engine thread hearing of the batch job's end only
    // after, so the Undo's job has not started.
    driven.run([
        Command::Open(others[0].clone()),
        Command::Resize {
            width: 40,
            height: 40,
        },
    ]);
    let mut held = false;
    while driven.engine.opening.is_some() {
        match (driven.inbox.recv_timeout(TEST_WAIT)).expect("the open goes on") {
            Inbox::Open(stage) => {
                driven.engine.stage(*stage);
            }
            Inbox::Task(waiting) => driven.engine.jobs.background.add(waiting),
            Inbox::BatchEnded => held = true,
            _ => {}
        }
    }
    let events = driven.run([Command::SetTool {
        tool: "soften".to_owned(),
        values: on("amount", 10.0),
    }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let mut heard = match held {
        true => driven.engine.start_batches(),
        false => Vec::new(),
    };
    let taken = |heard: &[Event]| {
        (ends(heard).into_iter()).find(|(action, ..)| *action == BatchAction::TakenBack)
    };
    while taken(&heard).is_none() {
        heard.extend(until_ended(&mut driven));
    }
    let (_, changed, skipped) = taken(&heard).unwrap();
    assert_eq!((changed, skipped), (vec![others[0].clone()], Vec::new()));
    let open = driven.engine.open.as_ref().unwrap();
    assert!(open.edit.tool("grain").is_none());
    assert!(open.edit.tool("soften").is_some());
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

/// The latest `History`'s Undo among `events`: how many photos it names.
fn undo_photos(events: &[Event]) -> Option<usize> {
    events.iter().rev().find_map(|event| match event {
        Event::History(History {
            undo: Some(Move::Steps { photos, .. }),
            ..
        }) => Some(*photos),
        _ => None,
    })
}

/// [`until_ended`], then until the engine thread let the batch go, telling the history
/// again.
fn until_let_go(driven: &mut Driven) -> Vec<Event> {
    let mut heard = until_ended(driven);
    let deadline = Instant::now() + TEST_WAIT;
    while !driven.engine.batches.is_empty() {
        assert!(Instant::now() < deadline, "the batch is let go");
        heard.extend(driven.pump_for(Duration::from_millis(10)));
    }
    heard
}

/// Undo names the photos a batch changed (decision 109): while it runs, those it changed and
/// those still to come; once it ends, told again, those it changed, a view-only one left out;
/// after a cancel, those it reached.
#[test]
fn undo_names_the_photos_a_batch_changed() {
    let active = scratch("batch-undo-count");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 11);
    let newer = sidecar::to_xml(&Edit::default(), OPERATIONS, TOOLS, "9.0.0")
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
    std::fs::write(sidecar::path_for(&others[0]), &newer).unwrap();
    let named = [vec![active.clone()], others.clone()].concat();
    let events = driven.run([warm(40.0, &named)]);
    // Twelve to change, or eleven where the job skipped the view-only one already.
    assert!(matches!(undo_photos(&events), Some(11 | 12)), "{events:?}");
    let heard = until_let_go(&mut driven);
    assert_eq!(undo_photos(&heard), Some(11), "{heard:?}");
    // Cancelled after three of twenty: the photos it reached.
    let more = photos(&active, "more", 20);
    let named = [vec![active.clone()], more].concat();
    driven.run([warm(60.0, &named)]);
    let deadline = Instant::now() + TEST_WAIT;
    let job = loop {
        let jobs: Vec<_> = driven.engine.batches.iter().map(|p| p.job()).collect();
        if let [Some(job)] = jobs[..] {
            break job;
        }
        assert!(Instant::now() < deadline, "the job starts: {jobs:?}");
        driven.pump_for(Duration::from_millis(10));
    };
    for _ in 0..3 {
        let route = next_route(&mut driven);
        driven.engine.routes.push(route);
        driven.engine.take_routes();
    }
    driven.run([Command::CancelJob { job }]);
    let heard = until_let_go(&mut driven);
    let Some(BatchEvent::Ended { changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    assert_eq!(undo_photos(&heard), Some(changed.len()));
    assert!((4..21).contains(&changed.len()), "{}", changed.len());
    remove(&active);
}

/// The 65th batch forgets the first, which the open photo's Undo names: the history is told
/// again, its Undo naming one photo, as after a restart.
#[test]
fn a_forgotten_batch_is_told_again_on_the_open_photo() {
    let active = scratch("batch-forgotten");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let named = [vec![active.clone()], photos(&active, "other", 1)].concat();
    driven.run([warm(40.0, &named)]);
    let heard = until_let_go(&mut driven);
    assert_eq!(undo_photos(&heard), Some(2), "{heard:?}");
    // 63 more on a photo not on the disk: 64 remembered, the open photo's untold.
    let missing = vec![active.with_file_name("missing.ARW")];
    let events = driven.run((0..63).map(|_| warm(50.0, &missing)));
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(undo_photos(&events), None, "{events:?}");
    // With a command that tells the values, the history comes once, after them.
    let events = driven.run([
        warm(50.0, &missing),
        Command::Hide {
            tool: "grain".to_owned(),
            hidden: true,
        },
    ]);
    assert_eq!(undo_photos(&events), Some(1), "{events:?}");
    let at = |fact: fn(&Event) -> bool| (events.iter()).position(fact);
    let values = at(|event| matches!(event, Event::Values(_)));
    let history = at(|event| matches!(event, Event::History(_)));
    assert!(values.is_some() && values < history, "{events:?}");
    let told = (events.iter()).filter(|event| matches!(event, Event::History(_)));
    assert_eq!(told.count(), 1, "{events:?}");
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
    let Some(BatchEvent::Ended { what, changed, .. }) = ended(&heard) else {
        unreachable!()
    };
    let faded = Batched::Style("Faded".to_owned());
    assert_eq!((what, changed), (&faded, &second));
    driven.writes();
    assert!(
        first
            .iter()
            .all(|photo| batches(&saved_log(photo)).is_empty())
    );
    remove(&active);
}

/// A batch taken back while it still waits for the one before never starts: it reaches no
/// photo, live or not.
#[test]
fn a_waiting_batch_taken_back_never_reaches_a_photo() {
    let active = scratch("batch-undo-waiting");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let first = photos(&active, "first", 1);
    let second = photos(&active, "second", 2);
    driven.run([warm(40.0, &first)]);
    let faded = Command::ApplyStyle {
        name: "Faded".to_owned(),
        file: "Faded.apstyle".to_owned(),
        mode: StyleMode::Append,
        tools: vec![("soften".to_owned(), on("amount", 20.0))],
        photos: second.clone(),
    };
    driven.run([faded]);
    let waiting = Arc::clone(&driven.engine.batches[1]);
    assert_eq!(waiting.job(), None);
    let batch = driven.engine.remembered.back().unwrap().batch.id;
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    // Its second photo opened meanwhile: nothing catches it up.
    driven.open(&second[1]);
    driven.run([Command::SetTool {
        tool: "grain".to_owned(),
        values: on("strength", 10.0),
    }]);
    let open = driven.engine.open.as_ref().unwrap();
    assert!(batches(open.history.log()).iter().all(|b| *b != batch));
    let heard = until_ends(&mut driven, 1);
    assert!(
        ends(&heard)
            .iter()
            .all(|(action, ..)| *action != BatchAction::PutBack)
    );
    let deadline = Instant::now() + TEST_WAIT;
    while !driven.engine.batches.is_empty() {
        assert!(Instant::now() < deadline, "the batches let go");
        driven.pump_for(Duration::from_millis(10));
    }
    driven.writes();
    for photo in &second {
        assert!(
            !batches(&saved_log(photo)).contains(&batch),
            "{}",
            photo.display()
        );
    }
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

/// Redo names the photos the Undo took the batch back on, not those that applied it: a
/// photo changed since, which the Undo skipped, is none Redo puts back.
#[test]
fn redo_names_the_photos_the_undo_took_back() {
    let active = scratch("batch-redo-count");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([warm(40.0, &named)]);
    until_let_go(&mut driven);
    // The second changed since: opened, Soften set, left again.
    driven.open(&others[1]);
    driven.run([Command::SetTool {
        tool: "soften".to_owned(),
        values: on("amount", 10.0),
    }]);
    driven.open(&active);
    driven.run([Command::Undo]);
    let heard = until_let_go(&mut driven);
    let [(_, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!((changed.len(), skipped.len()), (2, 1));
    let redo = heard.iter().rev().find_map(|event| match event {
        Event::History(history) => Some(history.redo),
        _ => None,
    });
    assert!(
        matches!(redo, Some(Some(Move::Steps { photos: 2, .. }))),
        "{redo:?}"
    );
    remove(&active);
}

/// `Rate` of three stars naming `photos` (decision 110).
fn three_stars(photos: &[PathBuf]) -> Command {
    Command::Rate {
        photos: photos.to_vec(),
        change: RatingChange::Stars(3),
    }
}

fn rated(stars: u8, rejected: bool, labels: u8) -> Rating {
    Rating {
        stars,
        rejected,
        labels,
    }
}

/// The rating the sidecar beside `photo` holds, as saved: none where there is no sidecar.
fn saved_rating(photo: &Path) -> Rating {
    let Ok(xml) = std::fs::read_to_string(sidecar::path_for(photo)) else {
        return Rating::default();
    };
    let file = sidecar::Edits::parse(&xml, 0).unwrap();
    Rating::of(file.copies.first().map_or(&[][..], |copy| &copy.fields))
}

/// The photos the `Rated` notices among `events` name, in order.
fn rated_photos(events: &[Event]) -> Vec<PathBuf> {
    (events.iter())
        .filter_map(|event| match event {
            Event::Rated { photo, .. } => Some(photo.clone()),
            _ => None,
        })
        .collect()
}

/// A rating on three photos, one open and one with only darktable's `.xmp` beside it: one
/// batch, the open photo at once and the others by a quiet `rate` job reading no raw, each
/// told by `Rated`, their logs left empty and darktable's label copied in. Undo on the open
/// photo names the three and takes the batch back on all, darktable's stars coming back;
/// Redo puts it back.
#[test]
fn a_rating_on_three_photos_is_one_batch_that_undo_takes_back_on_all() {
    let active = scratch("batch-rate");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF
        xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description
        xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:darktable="http://darktable.sf.net/"
        xmp:Rating="2"><darktable:colorlabels><rdf:Seq><rdf:li>0</rdf:li></rdf:Seq>
        </darktable:colorlabels></rdf:Description></rdf:RDF></x:xmpmeta>"#;
    std::fs::write(apertrust_library::darktable::path_for(&others[1]), xmp).unwrap();
    let named = [vec![active.clone()], others.clone()].concat();
    let events = driven.run([three_stars(&named)]);
    assert!(failures(&events).is_empty(), "{events:?}");
    assert_eq!(changed(&events), std::slice::from_ref(&active));
    let undo = events.iter().rev().find_map(|event| match event {
        Event::History(history) => Some(history.undo),
        _ => None,
    });
    assert!(
        matches!(undo, Some(Some(Move::Field { photos: 3, .. }))),
        "{undo:?}"
    );
    let heard = until_ended(&mut driven);
    let added = heard.iter().find_map(|event| match event {
        Event::Job {
            event: JobEvent::Added { kind, quiet, .. },
            ..
        } => Some((*kind, *quiet)),
        _ => None,
    });
    assert_eq!(added, Some(("rate", true)));
    assert_eq!(changed(&heard), others);
    assert_eq!(rated_photos(&heard), others);
    let Some(BatchEvent::Ended {
        what,
        changed,
        skipped,
        ..
    }) = ended(&heard)
    else {
        unreachable!()
    };
    assert_eq!(*what, Batched::Rating(RatingChange::Stars(3)));
    assert_eq!((changed.len(), skipped.len()), (3, 0));
    let Some(BatchEvent::Ended { batch, .. }) = ended(&heard) else {
        unreachable!()
    };
    let batch = *batch;
    driven.writes();
    assert_eq!(saved_rating(&others[0]), rated(3, false, 0));
    assert_eq!(saved_rating(&others[1]), rated(3, false, 0b1));
    assert!(
        others
            .iter()
            .all(|photo| saved_log(photo).steps().is_empty())
    );
    // Undo on the open photo, on all three.
    driven.run([Command::Undo]);
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed.len(), skipped.len()), (3, 0));
    driven.writes();
    assert_eq!(saved_rating(&others[0]), rated(0, false, 0));
    assert_eq!(saved_rating(&others[1]), rated(2, false, 0b1));
    assert_eq!(
        driven.engine.open.as_ref().unwrap().rating(),
        rated(0, false, 0)
    );
    // Redo puts it back on each.
    driven.run([Command::Redo]);
    let heard = until_ended(&mut driven);
    assert_eq!(ends(&heard)[0].0, BatchAction::PutBack);
    driven.writes();
    assert!(others.iter().all(|photo| saved_rating(photo).stars == 3));
    // From a photo outside it, after the open photo's own Undo and Redo moved it there,
    // the notice's Undo takes it back on all three, the photo left among them.
    let outside = photos(&active, "outside", 1).remove(0);
    driven.open(&outside);
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed.len(), skipped.len()), (3, 0), "{skipped:?}");
    driven.writes();
    assert_eq!(saved_rating(&active), rated(0, false, 0));
    assert_eq!(saved_rating(&others[1]), rated(2, false, 0b1));
    remove(&active);
}

/// A photo of a rating's batch open for viewing only, a newer Apertrust's, is skipped and
/// named, its file untouched; the others are rated, and the command is not refused.
#[test]
fn a_view_only_photo_is_skipped_and_named_in_a_rating_s_batch() {
    let active = scratch("batch-rate-view-only");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let newer = sidecar::to_xml(&Edit::default(), OPERATIONS, TOOLS, "9.0.0")
        .replace(r#"apertrust:version="1""#, r#"apertrust:version="3""#);
    let view_only = sidecar::path_for(&others[0]);
    std::fs::write(&view_only, &newer).unwrap();
    let events = driven.run([three_stars(&others)]);
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
    driven.writes();
    assert_eq!(std::fs::read_to_string(&view_only).unwrap(), newer);
    assert_eq!(saved_rating(&others[1]).stars, 3);
    remove(&active);
}

/// A style on a photo with only darktable's `.xmp` beside it writes its first `.apxmp` with
/// darktable's rating and label copied in, each with a clock (decision 51).
#[test]
fn a_style_s_batch_copies_darktable_s_rating_in() {
    let active = scratch("batch-style-darktable");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    let xmp = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF
        xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description
        xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:darktable="http://darktable.sf.net/"
        xmp:Rating="4"><darktable:colorlabels><rdf:Seq><rdf:li>2</rdf:li></rdf:Seq>
        </darktable:colorlabels></rdf:Description></rdf:RDF></x:xmpmeta>"#;
    std::fs::write(apertrust_library::darktable::path_for(&others[0]), xmp).unwrap();
    driven.run([warm(40.0, &others)]);
    until_ended(&mut driven);
    driven.writes();
    assert!(grained(&others[0]));
    assert_eq!(saved_rating(&others[0]), rated(4, false, 0b100));
    let xml = std::fs::read_to_string(sidecar::path_for(&others[0])).unwrap();
    let file = sidecar::Edits::parse(&xml, 0).unwrap();
    assert!(
        file.copies[0]
            .fields
            .iter()
            .all(|entry| entry.clock.is_some())
    );
    remove(&active);
}

/// A photo of a rating's batch rated again since keeps its later rating and is named; the
/// notice's Undo, `UndoBatch`, takes the batch back on the rest from a photo outside it.
#[test]
fn a_photo_rated_since_keeps_its_rating_and_undo_batch_takes_the_rest_back() {
    let active = scratch("batch-rate-since");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    driven.run([three_stars(&others)]);
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { batch, .. }) = ended(&heard) else {
        unreachable!()
    };
    let batch = *batch;
    // The first rated again: opened, five stars, left again.
    driven.open(&others[0]);
    driven.run([Command::Rate {
        photos: Vec::new(),
        change: RatingChange::Stars(5),
    }]);
    driven.open(&active);
    driven.writes();
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!(changed, std::slice::from_ref(&others[1]));
    assert!(
        matches!(&skipped[..], [(photo, Skip::ChangedSince)] if *photo == others[0]),
        "{skipped:?}"
    );
    driven.writes();
    assert_eq!(saved_rating(&others[0]).stars, 5);
    assert_eq!(saved_rating(&others[1]).stars, 0);
    remove(&active);
}

/// A rating on three photos with A active, then B, one of them, opened in a session of its
/// own, which holds no record of the batch: the notice's Undo takes it back on all three, B
/// live from the record the batch kept, as a photo not open is.
#[test]
fn undo_batch_takes_a_rating_back_on_a_photo_of_it_opened_since() {
    let active = scratch("batch-rate-reopened");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 2);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([three_stars(&named)]);
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { batch, .. }) = ended(&heard) else {
        unreachable!()
    };
    let batch = *batch;
    driven.writes();
    driven.open(&others[0]);
    assert_eq!(driven.engine.open.as_ref().unwrap().rating().stars, 3);
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed.len(), skipped.len()), (3, 0), "{skipped:?}");
    assert_eq!(
        driven.engine.open.as_ref().unwrap().rating(),
        rated(0, false, 0)
    );
    driven.writes();
    assert!(
        named
            .iter()
            .all(|photo| saved_rating(photo) == rated(0, false, 0))
    );
    remove(&active);
}

/// A rating on two photos with A active, then A's edit changed: the notice's Undo takes the
/// rating back on both, A's from the record the batch kept, though A's own next Undo is the
/// edit's change, which stays.
#[test]
fn undo_batch_takes_a_rating_back_under_a_later_edit() {
    let active = scratch("batch-rate-edited");
    let mut driven = Driven::new(TOOLS);
    driven.open(&active);
    let others = photos(&active, "other", 1);
    let named = [vec![active.clone()], others.clone()].concat();
    driven.run([three_stars(&named)]);
    let heard = until_ended(&mut driven);
    let Some(BatchEvent::Ended { batch, .. }) = ended(&heard) else {
        unreachable!()
    };
    let batch = *batch;
    driven.run([Command::SetTool {
        tool: "soften".to_owned(),
        values: on("amount", 10.0),
    }]);
    let events = driven.run([Command::UndoBatch { batch }]);
    assert!(failures(&events).is_empty(), "{events:?}");
    let heard = until_ended(&mut driven);
    let [(action, changed, skipped)] = &ends(&heard)[..] else {
        panic!("{heard:?}")
    };
    assert_eq!(*action, BatchAction::TakenBack);
    assert_eq!((changed.len(), skipped.len()), (2, 0), "{skipped:?}");
    let open = driven.engine.open.as_ref().unwrap();
    assert_eq!(open.rating(), rated(0, false, 0));
    assert!(open.edit.tool("soften").is_some());
    driven.writes();
    assert_eq!(saved_rating(&others[0]), rated(0, false, 0));
    remove(&active);
}
