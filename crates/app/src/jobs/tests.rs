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

//! The jobs mirror and the jobs window: the session's events in, the rows out, when the
//! window opens itself, what the buttons ask, and the window's content in the dialog
//! host's harness.

use std::time::Duration;

use apertrust_base::jobs::{Control, Job as ControlJob};
use apertrust_engine::session::Cause;
use apertrust_ui::components::JobPhase;
use apertrust_ui::{Builtin, Locale, Theme};
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::*;
use crate::dialog_host;

/// `count` job numbers, real ones from a control, which gives the only way to make them.
pub(crate) fn ids(count: usize) -> Vec<JobId> {
    let control = Control::start(|_| {});
    (0..count)
        .map(|_| {
            let job = ControlJob::new("test", "a job to number", |_| Ok(()));
            control.add(Queue::SystemBackground, job)
        })
        .collect()
}

/// A job added to `queue`, by the command numbered `command` where it names one: an export
/// with nothing but its title.
pub(crate) fn added(job: JobId, queue: Queue, command: Option<u64>) -> Event {
    Event::Job {
        event: JobEvent::Added {
            job,
            kind: "export",
            queue: Some(queue),
            title: Message::new("job-export-photo"),
            cancellable: true,
            destination: None,
            detail: None,
            progress: Progress::default(),
            retries: None,
        },
        command,
    }
}

fn job(event: JobEvent) -> Event {
    Event::Job {
        event,
        command: None,
    }
}

pub(crate) fn state(job_id: JobId, state: State) -> Event {
    job(JobEvent::State { job: job_id, state })
}

pub(crate) fn ended(job_id: JobId, state: State, failure: Option<Failure>) -> Event {
    job(JobEvent::Ended {
        job: job_id,
        state,
        outcome: None,
        failure,
        at: SystemTime::UNIX_EPOCH,
        offers: Vec::new(),
    })
}

fn english() -> Translator {
    Translator::new(Locale::EnGb)
}

/// The only row, as the window draws it.
fn only(mirror: &Mirror) -> Job {
    let mut view = mirror.view(&english());
    assert_eq!(view.len(), 1, "{view:?}");
    view.remove(0)
}

const HOME: &str = "/Users/anna";
const OUT: &str = "/Users/anna/out";

/// Photo `number` of an export, from 1.
fn raw(number: usize) -> PathBuf {
    PathBuf::from(format!("/raws/IMG_{number:04}.ARW"))
}

fn file(number: usize) -> PathBuf {
    Path::new(OUT).join(format!("IMG_{number:04}.jpg"))
}

/// The export's title as it exports photo `number` of `total`, darktable's message.
fn title(number: usize, total: usize) -> Message {
    Message::new("job-export-photo")
        .with("number", number)
        .with("total", total)
        .with("storage", "disk")
}

/// The same title as the window has it.
fn title_row(number: usize, total: usize) -> JobMessage {
    JobMessage::new("job-export-photo")
        .number("number", number as f64)
        .number("total", total as f64)
        .text("storage", "disk")
}

/// An export of `total` photos to `~/out` as the session adds it, sent by this app as its
/// command 7.
fn export_added(job: JobId, total: usize) -> Event {
    let queued = Message::new("job-export-queued")
        .with("count", total)
        .with("folder", "~/out");
    Event::Job {
        event: JobEvent::Added {
            job,
            kind: "export",
            queue: Some(Queue::UserExport),
            title: title(0, total),
            cancellable: true,
            destination: Some(Destination::Folder(PathBuf::from(OUT))),
            detail: Some(queued),
            progress: Progress {
                done: 0,
                total: Some(total as u64),
            },
            retries: None,
        },
        command: Some(7),
    }
}

/// What the export reports as photo `number` of `total` begins.
fn photo_starts(job_id: JobId, number: usize, total: usize) -> Vec<Event> {
    let now = Message::new("job-export-now")
        .with("done", number - 1)
        .with("total", total)
        .with("photo", raw(number));
    vec![
        job(JobEvent::Title {
            job: job_id,
            title: title(number, total).with("photo", raw(number)),
        }),
        job(JobEvent::Detail {
            job: job_id,
            detail: Some(now),
        }),
    ]
}

/// What the export reports as photo `number` of `total` is done, its file the first
/// written where `first`.
fn photo_done(job_id: JobId, number: usize, total: usize, first: bool) -> Vec<Event> {
    let between = Message::new("job-export-between")
        .with("done", number)
        .with("total", total);
    let mut events = Vec::new();
    if first {
        events.push(job(JobEvent::Destination {
            job: job_id,
            destination: Destination::File(file(number)),
        }));
    }
    events.push(job(JobEvent::Progress {
        job: job_id,
        done: number as u64,
        total: Some(total as u64),
    }));
    events.push(job(JobEvent::Detail {
        job: job_id,
        detail: Some(between),
    }));
    events
}

/// `seconds` after the epoch, an end's time.
fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

/// The events in, in order: what the window is asked, the most it was asked.
fn fold(mirror: &mut Mirror, events: impl IntoIterator<Item = Event>) -> Show {
    events
        .into_iter()
        .map(|event| mirror.apply(&event))
        .max()
        .unwrap_or(Show::Nothing)
}

#[test]
fn an_exports_events_fold_into_its_row() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(Some(PathBuf::from(HOME)));
    // Added: queued, its title at photo 0, the detail that it waits, its folder.
    assert_eq!(mirror.apply(&export_added(id, 3)), Show::Open);
    let queued = JobMessage::new("job-export-queued")
        .number("count", 3.0)
        .text("folder", "~/out");
    let expected = Job::new(id.get(), JobPhase::Queued, title_row(0, 3))
        .kind("export")
        .progress(0, Some(3))
        .cancellable(true)
        .detail(queued)
        .destination("~/out");
    assert_eq!(only(&mirror), expected);

    // Running, the first photo under way: its title and the detail naming it.
    mirror.apply(&state(id, State::Running));
    fold(&mut mirror, photo_starts(id, 1, 3));
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Running);
    assert_eq!(
        row.title,
        title_row(1, 3).text("photo", "IMG_0001.ARW"),
        "a path by its name"
    );
    assert_eq!(
        row.detail,
        Some(
            JobMessage::new("job-export-now")
                .number("done", 0.0)
                .number("total", 3.0)
                .text("photo", "IMG_0001.ARW")
        )
    );

    // Its first file: the destination is that file's folder, the bar a third full.
    fold(&mut mirror, photo_done(id, 1, 3, true));
    let row = only(&mirror);
    assert_eq!(row.destination.as_deref(), Some("~/out"));
    assert_eq!((row.done, row.total), (1, Some(3)));
    assert_eq!(
        row.detail,
        Some(
            JobMessage::new("job-export-between")
                .number("done", 1.0)
                .number("total", 3.0)
        )
    );
    for number in 2..=3 {
        fold(&mut mirror, photo_starts(id, number, 3));
        fold(&mut mirror, photo_done(id, number, 3, false));
    }
    assert_eq!(only(&mirror).fraction(), Some(1.0));

    // Ended: finished, its outcome and time, no detail line, nothing to cancel.
    let outcome = Message::new("job-export-finished")
        .with("written", 3)
        .with("skipped", 0)
        .with("failed", 0)
        .with("total", 3);
    let end = JobEvent::Ended {
        job: id,
        state: State::Finished,
        outcome: Some(outcome),
        failure: None,
        at: at(60),
        offers: Vec::new(),
    };
    assert_eq!(mirror.apply(&job(end)), Show::Update);
    let outcome = JobMessage::new("job-export-finished")
        .number("written", 3.0)
        .number("skipped", 0.0)
        .number("failed", 0.0)
        .number("total", 3.0);
    let expected = Job::new(
        id.get(),
        JobPhase::Finished,
        title_row(3, 3).text("photo", "IMG_0003.ARW"),
    )
    .kind("export")
    .progress(3, Some(3))
    .destination("~/out")
    .outcome(outcome)
    .ended_at(at(60));
    assert_eq!(only(&mirror), expected);
    // Show in Finder selects its first file.
    assert_eq!(
        mirror.act(ActivityAction::ShowFolder(id.get())),
        Do::Reveal(Reveal::File(file(1)))
    );
}

#[test]
fn a_folders_reading_folds_through_the_same_code_and_never_opens_the_window() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(Some(PathBuf::from(HOME)));
    let folder = PathBuf::from("/photos/2026-10 Lisbon");
    let added = Event::Job {
        event: JobEvent::Added {
            job: id,
            kind: "folder",
            queue: Some(Queue::UserBackground),
            title: Message::new("job-folder").with("folder", folder),
            cancellable: true,
            destination: None,
            detail: None,
            progress: Progress::default(),
            retries: None,
        },
        command: Some(4),
    };
    // Sent by this app, but not on a user queue: it shows, and opens nothing.
    assert_eq!(mirror.apply(&added), Show::Update);
    let reading = JobMessage::new("job-folder").text("folder", "2026-10 Lisbon");
    let expected = Job::new(id.get(), JobPhase::Running, reading.clone())
        .kind("folder")
        .progress(120, None)
        .cancellable(true);
    // With no total its bar has no fraction, however much is done: it is indeterminate.
    let events = [
        state(id, State::Running),
        job(JobEvent::Progress {
            job: id,
            done: 120,
            total: None,
        }),
    ];
    assert_eq!(fold(&mut mirror, events), Show::Update);
    assert_eq!(only(&mirror), expected);

    mirror.apply(&job(JobEvent::Ended {
        job: id,
        state: State::Finished,
        outcome: None,
        failure: None,
        at: at(5),
        offers: Vec::new(),
    }));
    let expected = Job::new(id.get(), JobPhase::Finished, reading)
        .kind("folder")
        .progress(120, None)
        .ended_at(at(5));
    assert_eq!(only(&mirror), expected);
    // No destination: Show in Finder has nothing to reveal.
    assert_eq!(
        mirror.act(ActivityAction::ShowFolder(id.get())),
        Do::Nothing
    );
}

#[test]
fn a_failed_export_says_which_photo_and_why_in_words_and_offers_a_retry() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(Some(PathBuf::from(HOME)));
    mirror.apply(&export_added(id, 3));
    mirror.apply(&state(id, State::Running));
    fold(&mut mirror, photo_starts(id, 1, 3));
    fold(&mut mirror, photo_done(id, 1, 3, true));
    fold(&mut mirror, photo_starts(id, 2, 3));
    // The disk fills up: that photo failed, and the job ends, the last not reached.
    let failure = Failure::new("disk-full", "an error in the engine's words")
        .item(raw(2))
        .file(file(2));
    let outcome = Message::new("job-export-failed-outcome")
        .with("written", 1)
        .with("total", 3);
    mirror.apply(&job(JobEvent::Ended {
        job: id,
        state: State::Finished,
        outcome: Some(outcome),
        failure: Some(failure),
        at: at(9),
        offers: vec![Offer::Retry { items: 2 }],
    }));

    let row = only(&mirror);
    assert_eq!((row.phase, row.cancellable), (JobPhase::Failed, false));
    assert_eq!(row.detail, None);
    assert!(row.retry);
    assert_eq!(
        row.failure,
        Some(JobFailure {
            item: "IMG_0002.ARW".to_owned(),
            file: "IMG_0002.jpg".to_owned(),
            cause: "the disk is full".to_owned(),
        })
    );
    // In Ukrainian the cause is in Ukrainian.
    let uk = mirror.view(&Translator::new(Locale::Uk));
    let cause = uk[0].failure.as_ref().map(|f| f.cause.as_str());
    assert_eq!(cause, Some("диск заповнений"));
}

#[test]
fn an_export_refused_before_any_photo_names_no_photo_and_no_file() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(None);
    mirror.apply(&export_added(id, 2));
    mirror.apply(&state(id, State::Running));
    let end = Failure::new("pattern", "the path holds an unknown variable");
    mirror.apply(&ended(id, State::Finished, Some(end)));
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Failed);
    assert!(!row.retry, "no offer, no Try again");
    assert_eq!(
        row.failure,
        Some(JobFailure {
            item: String::new(),
            file: String::new(),
            cause: "the file name is not valid".to_owned(),
        })
    );
}

#[test]
fn a_cancel_takes_the_cancel_away_and_ends_cancelled_or_failed_with_a_failure() {
    let ids = ids(2);
    let mut mirror = Mirror::new(None);
    for id in &ids {
        mirror.apply(&export_added(*id, 3));
        mirror.apply(&state(*id, State::Running));
        // Asked to stop, it goes on until its next check: no second Cancel meanwhile.
        mirror.apply(&state(*id, State::Cancelled));
    }
    let view = mirror.view(&english());
    assert!(
        view.iter()
            .all(|row| row.phase == JobPhase::Running && !row.cancellable)
    );
    // Cancelled with nothing failed: a cancelled row.
    mirror.apply(&ended(ids[0], State::Cancelled, None));
    // Cancelled after a photo failed: the failure makes it a failed row (decision 72).
    let failure = Failure::new("unreadable", "cannot decode").item(raw(1));
    mirror.apply(&ended(ids[1], State::Cancelled, Some(failure)));
    let view = mirror.view(&english());
    let phase = |id: JobId| view.iter().find(|row| row.id == id.get()).unwrap().phase;
    assert_eq!(phase(ids[0]), JobPhase::Cancelled);
    assert_eq!(phase(ids[1]), JobPhase::Failed);
}

#[test]
fn a_job_dropped_unrun_leaves_no_row_and_events_of_unknown_jobs_change_nothing() {
    let ids = ids(2);
    let (one, other) = (ids[0], ids[1]);
    let mut mirror = Mirror::new(None);
    // The window starts with nothing: it is seeded with no old jobs.
    assert!(mirror.view(&english()).is_empty());
    // Events of a job never added (from before this run) are not taken in.
    let progress = JobEvent::Progress {
        job: other,
        done: 1,
        total: Some(2),
    };
    assert_eq!(mirror.apply(&job(progress)), Show::Nothing);
    assert_eq!(fold(&mut mirror, photo_starts(other, 1, 2)), Show::Nothing);
    assert!(mirror.view(&english()).is_empty());
    // Nor is the export's own record, which is for scripts and the CLI.
    let status = apertrust_engine::session::ExportStatus::Started {
        index: 0,
        source: raw(1),
    };
    let record = Event::Export { job: one, status };
    assert_eq!(mirror.apply(&record), Show::Nothing);

    mirror.apply(&added(one, Queue::SystemForeground, None));
    assert_eq!(mirror.view(&english()).len(), 1);
    let dropped = ended(one, State::Discarded, None);
    assert_eq!(mirror.apply(&dropped), Show::Update);
    assert!(mirror.view(&english()).is_empty());
}

#[test]
fn only_a_job_on_a_user_queue_that_this_app_started_opens_the_window() {
    let id = ids(1)[0];
    let opens = |queue, command| {
        // Each in a mirror of its own, so a row never replaces another.
        let mut mirror = Mirror::new(None);
        mirror.apply(&added(id, queue, command))
    };
    // darktable's user queues, by a command of this app's.
    assert_eq!(opens(Queue::UserForeground, Some(2)), Show::Open);
    assert_eq!(opens(Queue::UserExport, Some(3)), Show::Open);
    // Another client's export shows, but opens nothing.
    assert_eq!(opens(Queue::UserExport, None), Show::Update);
    assert_eq!(opens(Queue::UserForeground, None), Show::Update);
    // A background job, a folder's reading, even one this app asked for, and the system's.
    assert_eq!(opens(Queue::UserBackground, Some(4)), Show::Update);
    assert_eq!(opens(Queue::UserBackground, None), Show::Update);
    assert_eq!(opens(Queue::SystemForeground, None), Show::Update);
    assert_eq!(opens(Queue::SystemBackground, Some(5)), Show::Update);
}

#[test]
fn a_job_with_no_outcome_says_why_it_failed() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(None);
    let message = Message::new("job-import").with("done", 2).with("total", 9);
    let event = JobEvent::Added {
        job: id,
        kind: "import",
        queue: Some(Queue::UserForeground),
        title: message,
        cancellable: true,
        destination: None,
        detail: None,
        progress: Progress::default(),
        retries: None,
    };
    mirror.apply(&job(event));
    mirror.apply(&state(id, State::Running));
    let row = only(&mirror);
    assert_eq!(
        row.title,
        JobMessage::new("job-import")
            .number("done", 2.0)
            .number("total", 9.0)
    );
    let failure = Failure::new("import", "the card was removed.");
    mirror.apply(&ended(id, State::Finished, Some(failure)));
    let row = only(&mirror);
    assert_eq!((row.phase, row.outcome), (JobPhase::Failed, None));
    assert_eq!(
        row.failure.map(|f| f.cause),
        Some("the card was removed".to_owned())
    );
}

#[test]
fn the_buttons_ask_for_cancel_the_reveal_and_the_dismissal() {
    let id = ids(1)[0];
    let number = id.get();
    let mut mirror = Mirror::new(None);
    mirror.apply(&export_added(id, 3));
    mirror.apply(&state(id, State::Running));

    // Cancel is the session's CancelJob for the job.
    assert_eq!(
        mirror.act(ActivityAction::Cancel(number)),
        Do::Send(Command::CancelJob { job: id })
    );
    // Before any file, Show in Finder opens the folder it was added with.
    assert_eq!(
        mirror.act(ActivityAction::ShowFolder(number)),
        Do::Reveal(Reveal::Folder(PathBuf::from(OUT)))
    );
    // A running job is not dismissed, and Try again waits for the session's RetryJob.
    assert_eq!(mirror.act(ActivityAction::Dismiss(number)), Do::Nothing);
    assert_eq!(mirror.act(ActivityAction::TryAgain(number)), Do::Nothing);
    // A job the mirror does not hold asks nothing.
    assert_eq!(
        mirror.act(ActivityAction::Cancel(number + 100)),
        Do::Nothing
    );
}

#[test]
fn a_retry_takes_the_place_of_the_job_it_tries_again() {
    let ids = ids(2);
    let (first, second) = (ids[0], ids[1]);
    let mut mirror = Mirror::new(None);
    mirror.apply(&export_added(first, 2));
    let failure = Failure::new("unreadable", "cannot decode").item(raw(1));
    mirror.apply(&ended(first, State::Finished, Some(failure)));
    assert_eq!(only(&mirror).phase, JobPhase::Failed);

    let retry = Event::Job {
        event: JobEvent::Added {
            job: second,
            kind: "export",
            queue: Some(Queue::UserExport),
            title: title(0, 1),
            cancellable: true,
            destination: None,
            detail: None,
            progress: Progress::default(),
            retries: Some(first),
        },
        command: Some(9),
    };
    assert_eq!(mirror.apply(&retry), Show::Open);
    let row = only(&mirror);
    assert_eq!((row.id, row.phase), (second.get(), JobPhase::Queued));

    // A job that comes back under the number it had, as a retry that reuses it may, is one
    // row, queued again.
    mirror.apply(&state(second, State::Running));
    mirror.apply(&ended(second, State::Finished, None));
    mirror.apply(&added(second, Queue::UserExport, None));
    assert_eq!(only(&mirror).phase, JobPhase::Queued);
}

#[test]
fn dismissing_an_ended_row_takes_it_away() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(None);
    mirror.apply(&export_added(id, 1));
    mirror.apply(&state(id, State::Running));
    mirror.apply(&ended(id, State::Cancelled, None));
    assert_eq!(mirror.act(ActivityAction::Dismiss(id.get())), Do::Changed);
    assert!(mirror.view(&english()).is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn finder_selects_a_file_and_opens_a_folder() {
    let file = Reveal::File(PathBuf::from("/out/a b.jpg"));
    let (program, args) = reveal_command(&file);
    assert_eq!(
        (program, args),
        ("open", vec!["-R".into(), "/out/a b.jpg".into()])
    );
    let folder = Reveal::Folder(PathBuf::from("/out"));
    let (program, args) = reveal_command(&folder);
    assert_eq!((program, args), ("open", vec!["/out".into()]));
    // A relative name starting with a dash is made absolute, never read as an option.
    let dashed = Reveal::Folder(PathBuf::from("-p"));
    let (_, args) = reveal_command(&dashed);
    assert!(Path::new(&args[0]).is_absolute(), "{args:?}");
}

#[test]
fn every_cause_the_engine_names_has_words_in_every_language() {
    let causes = [
        Cause::Unreadable,
        Cause::Edit,
        Cause::Binding,
        Cause::Size,
        Cause::Pattern,
        Cause::Name,
        Cause::Render,
        Cause::Encode,
        Cause::DiskFull,
        Cause::ReadOnly,
        Cause::Gone,
        Cause::Folder,
        Cause::Write,
        Cause::Engine,
    ];
    for locale in [Locale::EnGb, Locale::Uk, Locale::Pl] {
        let words = Translator::new(locale);
        for cause in causes {
            let id = format!("job-cause-{}", cause.id());
            assert!(words.has(&id), "{locale:?}: {id}");
            let text = words.text(&id);
            assert!(!text.ends_with('.'), "{id}: no full stop: {text}");
        }
    }
    // A code this build has no words for falls back to the job's own.
    assert_eq!(
        cause_words(&english(), "from-the-future", "it broke."),
        "it broke"
    );
}

/// The window as the host draws it in its own window, and what each frame asked.
struct Bench {
    content: Window,
    asked: Vec<dialog_host::Drawn>,
}

fn window(jobs: Vec<Job>) -> Harness<'static, Bench> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 900.0))
        .with_os(egui::os::OperatingSystem::Mac)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let drawn = dialog_host::draw(ui, &mut bench.content);
                bench.asked.push(drawn);
            },
            Bench {
                content: Window::new(jobs),
                asked: Vec::new(),
            },
        );
    harness.run();
    harness
}

fn many(count: usize) -> Vec<Job> {
    (1..=count as u64)
        .map(|id| Job::new(id, JobPhase::Running, JobMessage::new("job-thumbnails")))
        .collect()
}

#[test]
fn the_window_is_named_jobs_and_grows_with_its_rows_up_to_five_then_they_scroll() {
    let size = |count| {
        let harness = window(many(count));
        harness.get_by_role_and_label(Role::Dialog, "Jobs");
        harness.state().asked.last().expect("drawn").size
    };
    let (one, five, six, lots) = (size(1), size(5), size(6), size(40));
    assert!(one.y < five.y, "{one:?} {five:?}");
    // Capped at five rows: the rest scroll, the footer below them.
    assert_eq!(six, five);
    assert_eq!(lots, five);
    assert_eq!(one.x, five.x);
    assert!(five.x >= WIDTH, "{five:?}");
}

#[test]
fn what_is_pressed_in_the_window_reaches_the_app_as_an_output() {
    let id = 5;
    let job = Job::new(id, JobPhase::Running, JobMessage::new("job-thumbnails")).cancellable(true);
    let mut harness = window(vec![job]);
    harness
        .get_by_role_and_label(Role::Button, "Cancel")
        .click();
    harness.run();
    let outputs: Vec<_> = harness
        .state()
        .asked
        .iter()
        .filter_map(|drawn| drawn.output.clone())
        .collect();
    assert_eq!(
        outputs,
        [dialog_host::Output::Jobs(ActivityAction::Cancel(id))]
    );
    // Closing is the window's own, and never asks for anything of the job.
    assert!(harness.state().asked.iter().all(|drawn| !drawn.close));
}

#[test]
fn the_window_draws_no_try_again_until_the_session_takes_retry_job() {
    let failed = Job::new(5, JobPhase::Failed, JobMessage::new("job-thumbnails"))
        .failure(JobFailure {
            item: String::new(),
            file: String::new(),
            cause: "the disk is full".to_owned(),
        })
        .retry(true);
    let harness = window(vec![failed]);
    assert!(harness.query_by_label("Try again").is_none());
    assert!(harness.query_by_label("Dismiss").is_some());
}

// When the window closes itself.

/// A job of the window's, numbered `id`, in `phase`.
fn job_in(id: u64, phase: JobPhase) -> Job {
    Job::new(id, phase, JobMessage::new("job-thumbnails"))
}

/// The window as an export the person started opens it, for jobs 1 and 2, drawn every half
/// second, on the Mac without the focus as the host shows it there.
fn opened(jobs: Vec<Job>) -> Harness<'static, Bench> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 900.0))
        .with_os(egui::os::OperatingSystem::Mac)
        .with_step_dt(0.5)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
        })
        .build_ui_state(
            |ui, bench: &mut Bench| {
                let drawn = dialog_host::draw(ui, &mut bench.content);
                bench.asked.push(drawn);
            },
            Bench {
                content: Window::new(Vec::new()),
                asked: Vec::new(),
            },
        );
    // Opened once the harness's own first frames, which have the focus, are drawn.
    harness.input_mut().focused = false;
    harness.state_mut().content = Window::new(jobs).opened_for(vec![1, 2]);
    harness.run();
    harness
}

/// Whether any frame drawn so far asked to close.
fn closed(harness: &Harness<'_, Bench>) -> bool {
    harness.state().asked.iter().any(|drawn| drawn.close)
}

/// The jobs given to the window, as the app gives them as they change.
fn give(harness: &mut Harness<'_, Bench>, jobs: Vec<Job>) {
    harness.state_mut().content.set(jobs);
}

#[test]
fn a_window_that_opened_itself_closes_about_two_seconds_after_its_jobs_end_well() {
    let running = vec![
        job_in(1, JobPhase::Running),
        job_in(2, JobPhase::Queued),
        job_in(3, JobPhase::Running),
    ];
    let mut harness = opened(running);
    harness.run_steps(8);
    assert!(!closed(&harness), "closed while its jobs run");
    // One of its jobs ends; the other still waits.
    give(
        &mut harness,
        vec![
            job_in(1, JobPhase::Finished),
            job_in(2, JobPhase::Running),
            job_in(3, JobPhase::Running),
        ],
    );
    harness.run_steps(8);
    assert!(!closed(&harness), "closed before all its jobs ended");
    // Both end well; a job it did not open for, a folder's reading, goes on, and does not
    // hold it.
    give(
        &mut harness,
        vec![
            job_in(1, JobPhase::Finished),
            job_in(2, JobPhase::Finished),
            job_in(3, JobPhase::Running),
        ],
    );
    harness.run_steps(3);
    assert!(!closed(&harness), "closed before its outcome could be read");
    harness.run_steps(2);
    assert!(
        closed(&harness),
        "still open two seconds after its jobs ended"
    );
}

#[test]
fn a_failure_a_cancel_a_click_a_key_or_the_focus_keeps_it_open() {
    let ended = |second| vec![job_in(1, JobPhase::Finished), job_in(2, second)];
    // One of its jobs failed or was cancelled: it stays, for the person to see.
    for phase in [JobPhase::Failed, JobPhase::Cancelled] {
        let mut harness = opened(ended(phase));
        harness.run_steps(10);
        assert!(!closed(&harness), "{phase:?}");
    }
    // Touched: a click in it, a key typed in it, or on the Mac the focus, which a click, a
    // move by its title bar or bringing it to the front gives it.
    let touches: [fn(&mut Harness<'_, Bench>); 3] = [
        |harness| {
            // A click on an empty part of it, the pointer then leaving: it stays all the same.
            let pos = egui::pos2(40.0, 40.0);
            let events = &mut harness.input_mut().events;
            for pressed in [true, false] {
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
        },
        |harness| harness.key_press(egui::Key::Tab),
        |harness| harness.input_mut().focused = true,
    ];
    for (i, touch) in touches.into_iter().enumerate() {
        let mut harness = opened(vec![job_in(1, JobPhase::Running)]);
        touch(&mut harness);
        harness.step();
        harness.input_mut().events.push(egui::Event::PointerGone);
        give(&mut harness, ended(JobPhase::Finished));
        harness.run_steps(10);
        assert!(!closed(&harness), "touch {i}");
    }
    // Untouched, the same window closes.
    let mut harness = opened(vec![job_in(1, JobPhase::Running)]);
    harness.step();
    harness.input_mut().events.push(egui::Event::PointerGone);
    give(&mut harness, ended(JobPhase::Finished));
    harness.run_steps(10);
    assert!(closed(&harness), "untouched");
}

#[test]
fn the_pointer_over_it_holds_the_close_until_it_leaves() {
    let mut harness = opened(vec![job_in(1, JobPhase::Finished)]);
    harness.hover_at(egui::pos2(40.0, 40.0));
    harness.run_steps(10);
    assert!(!closed(&harness), "closed under the pointer");
    harness.input_mut().events.push(egui::Event::PointerGone);
    harness.run_steps(3);
    assert!(!closed(&harness), "closed at once as the pointer left");
    harness.run_steps(3);
    assert!(closed(&harness), "never closed after the pointer left");
}

#[test]
fn opened_from_the_spinner_or_brought_to_the_front_it_stays_and_a_new_job_is_waited_for() {
    let finished = || vec![job_in(1, JobPhase::Finished), job_in(2, JobPhase::Finished)];
    // Opened by the person: it never closes itself.
    let mut window = Window::new(finished());
    assert_eq!(window.close(0.0, false, false), Close::Stay);
    assert_eq!(window.close(10.0, false, false), Close::Stay);
    // Opened by itself, then brought to the front by the spinner: it stays.
    let mut window = Window::new(finished()).opened_for(vec![1]);
    window.again(Window::new(finished()), true);
    assert_eq!(window.close(10.0, false, false), Close::Stay);
    // Opened by itself, then another export starts: it waits for that one too.
    let mut window = Window::new(finished()).opened_for(vec![1]);
    assert_eq!(window.close(0.0, false, false), Close::After(CLOSE_AFTER));
    let later = vec![job_in(1, JobPhase::Finished), job_in(2, JobPhase::Running)];
    window.again(Window::new(later).opened_for(vec![2]), false);
    assert_eq!(window.close(3.0, false, false), Close::Stay);
    window.set(finished());
    assert_eq!(window.close(4.0, false, false), Close::After(CLOSE_AFTER));
    assert_eq!(window.close(6.0, false, false), Close::Now);
    // A window the person opened is not made to close by an automatic open after it.
    let mut window = Window::new(finished());
    window.again(Window::new(finished()).opened_for(vec![2]), false);
    assert_eq!(window.close(10.0, false, false), Close::Stay);
}

#[test]
fn the_jobs_that_open_the_window_are_kept_for_it_to_wait_for() {
    let ids = ids(3);
    let mut mirror = Mirror::new(None);
    mirror.apply(&added(ids[0], Queue::UserExport, Some(1)));
    mirror.apply(&added(ids[1], Queue::SystemBackground, None));
    mirror.apply(&added(ids[2], Queue::UserForeground, Some(2)));
    assert_eq!(mirror.opened(), [ids[0].get(), ids[2].get()]);
    // Taken once.
    assert!(mirror.opened().is_empty());
}

#[test]
fn clear_finished_takes_every_finished_and_cancelled_row_and_leaves_a_failed_or_running_one() {
    let ids = ids(4);
    let mut mirror = Mirror::new(None);
    for id in &ids {
        mirror.apply(&added(*id, Queue::UserExport, None));
    }
    mirror.apply(&ended(ids[0], State::Finished, None));
    mirror.apply(&ended(ids[1], State::Cancelled, None));
    let failure = Failure::new("write", "cannot write");
    mirror.apply(&ended(ids[2], State::Finished, Some(failure)));
    assert_eq!(mirror.act(ActivityAction::ClearFinished), Do::Changed);
    let left: Vec<u64> = mirror.view(&english()).iter().map(|job| job.id).collect();
    assert_eq!(left, [ids[2].get(), ids[3].get()]);
    // Nothing more to clear.
    assert_eq!(mirror.act(ActivityAction::ClearFinished), Do::Nothing);
}

// What quitting asks about.

/// A job added to `queue` that `cancellable` says can or cannot be stopped.
pub(crate) fn added_stoppable(
    job: JobId,
    queue: Queue,
    command: Option<u64>,
    cancellable: bool,
) -> Event {
    Event::Job {
        event: JobEvent::Added {
            job,
            kind: "delete",
            queue: Some(queue),
            title: Message::new("job-delete"),
            cancellable,
            destination: None,
            detail: None,
            progress: Progress::default(),
            retries: None,
        },
        command,
    }
}

#[test]
fn quitting_counts_the_jobs_on_the_user_queues_that_have_not_ended() {
    let jobs = ids(5);
    let mut mirror = Mirror::new(None);
    assert!(mirror.running().none());
    // An export (stoppable), whichever client sent it, and a delete that cannot be stopped.
    mirror.apply(&added(jobs[0], Queue::UserExport, None));
    mirror.apply(&added_stoppable(
        jobs[1],
        Queue::UserForeground,
        None,
        false,
    ));
    mirror.apply(&added(jobs[2], Queue::UserExport, Some(3)));
    // A system job and a background one, a folder's reading, never count.
    mirror.apply(&added(jobs[3], Queue::SystemBackground, None));
    mirror.apply(&added(jobs[4], Queue::UserBackground, Some(4)));
    assert_eq!(
        mirror.running(),
        Running {
            stoppable: 2,
            waiting: 1
        }
    );
    assert_eq!(mirror.stoppable(), [jobs[0], jobs[2]]);
}

#[test]
fn an_export_sent_and_not_yet_added_is_waited_for_until_its_job_is() {
    let job = ids(1)[0];
    let mut mirror = Mirror::new(None);
    mirror.starting(7);
    // It has no number to cancel yet.
    assert_eq!(
        mirror.running(),
        Running {
            stoppable: 0,
            waiting: 1
        }
    );
    assert!(mirror.stoppable().is_empty());
    mirror.apply(&added(job, Queue::UserExport, Some(7)));
    assert_eq!(
        mirror.running(),
        Running {
            stoppable: 1,
            waiting: 0
        }
    );
}

#[test]
fn a_job_that_ended_no_longer_counts_and_one_asked_to_stop_waits_until_it_has() {
    let job_id = ids(1)[0];
    let mut mirror = Mirror::new(None);
    mirror.apply(&added(job_id, Queue::UserExport, None));
    mirror.apply(&state(job_id, State::Running));
    assert_eq!(mirror.running().stoppable, 1);
    // Asked to stop, it goes on until its next check: it is waited for, and asked no more.
    mirror.apply(&state(job_id, State::Cancelled));
    assert_eq!(
        mirror.running(),
        Running {
            stoppable: 0,
            waiting: 1
        }
    );
    assert!(mirror.stoppable().is_empty());
    mirror.apply(&ended(job_id, State::Cancelled, None));
    assert!(mirror.running().none());
    // A finished one and a failed one never count either.
    let more = ids(2);
    for (job_id, outcome) in [
        (more[0], None),
        (more[1], Some(Failure::new("write", "cannot write"))),
    ] {
        mirror.apply(&added(job_id, Queue::UserExport, None));
        mirror.apply(&ended(job_id, State::Finished, outcome));
    }
    assert!(mirror.running().none());
}

#[test]
fn the_engine_stopping_fails_the_running_rows_and_forgets_the_exports_sent() {
    let ids = ids(2);
    let (running, done) = (ids[0], ids[1]);
    let mut mirror = Mirror::new(None);
    mirror.apply(&export_added(running, 3));
    mirror.apply(&state(running, State::Running));
    fold(&mut mirror, photo_starts(running, 1, 3));
    // One that ended before keeps how it ended; one sent has no job yet.
    mirror.apply(&added(done, Queue::UserExport, None));
    mirror.apply(&ended(done, State::Finished, None));
    mirror.starting(9);
    assert!(!mirror.running().none());

    assert_eq!(mirror.engine_stopped(), Show::Update);

    assert!(mirror.running().none());
    assert!(mirror.stoppable().is_empty());
    let view = mirror.view(&english());
    let row = view.iter().find(|job| job.id == running.get()).unwrap();
    assert_eq!((row.phase, row.cancellable), (JobPhase::Failed, false));
    assert_eq!(row.detail, None);
    assert_eq!(
        row.failure.as_ref().map(|f| f.cause.as_str()),
        Some("the engine stopped unexpectedly")
    );
    let row = view.iter().find(|job| job.id == done.get()).unwrap();
    assert_eq!(row.phase, JobPhase::Finished);
    // In Ukrainian the cause is in Ukrainian, and a second stop changes nothing.
    let uk = mirror.view(&Translator::new(Locale::Uk));
    let cause = uk.iter().find_map(|job| job.failure.as_ref());
    assert!(cause.is_some_and(|f| f.cause != "job-cause-engine"));
    assert_eq!(mirror.engine_stopped(), Show::Nothing);
}
