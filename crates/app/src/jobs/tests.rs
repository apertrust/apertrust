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

//! The jobs mirror and the jobs window: the session's events folded into rows, when the
//! window opens itself, what the buttons ask, and the window's content in the dialog
//! host's harness.

use apertrust_base::jobs::{Control, Job as ControlJob};
use apertrust_engine::session::{Cause, Summary};
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

pub(crate) fn photos(count: usize) -> Vec<PathBuf> {
    (1..=count)
        .map(|i| PathBuf::from(format!("/raws/IMG_{i:04}.ARW")))
        .collect()
}

/// An export of `count` photos, as the Export dialog sends it.
pub(crate) fn export(count: usize) -> Command {
    Command::Export {
        photos: photos(count),
        settings: crate::export::Values::default().settings(),
        places: Places::default(),
    }
}

pub(crate) fn added(job: JobId, queue: Queue, command: Option<u64>) -> Event {
    let message = Message::new("job-export");
    Event::Job {
        event: JobEvent::Added {
            job,
            kind: "export",
            queue: Some(queue),
            title: message,
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

pub(crate) fn ended(job_id: JobId, state: State, failure: Option<JobFailure>) -> Event {
    job(JobEvent::Ended {
        job: job_id,
        state,
        outcome: None,
        failure,
        at: std::time::SystemTime::UNIX_EPOCH,
        offers: Vec::new(),
    })
}

fn status(job: JobId, status: ExportStatus) -> Event {
    Event::Export { job, status }
}

pub(crate) fn started(job: JobId, index: usize) -> Event {
    let source = photos(index + 1).remove(index);
    status(job, ExportStatus::Started { index, source })
}

pub(crate) fn written(job: JobId, index: usize) -> Event {
    let result = PhotoResult::Written {
        path: PathBuf::from(format!("/out/IMG_{:04}.jpg", index + 1)),
        width: 6000,
        height: 4000,
        tiles: 1,
        newer: false,
    };
    status(job, ExportStatus::Done { index, result })
}

fn failed(job: JobId, index: usize, cause: Cause) -> Event {
    let failure = Failure {
        cause,
        file: Some(PathBuf::from(format!("/out/IMG_{:04}.jpg", index + 1))),
        message: "an error in the engine's words".to_owned(),
    };
    let result = PhotoResult::Failed(failure);
    status(job, ExportStatus::Done { index, result })
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

/// A mirror that has been sent an export of `count` photos and told its job was added.
fn exporting(count: usize) -> (Mirror, JobId) {
    let job_id = ids(1)[0];
    let mut mirror = Mirror::new(Some(PathBuf::from("/Users/anna")));
    mirror.sent(7, &export(count), None);
    mirror.apply(&added(job_id, Queue::UserExport, Some(7)));
    (mirror, job_id)
}

fn export_message(done: f64, total: f64) -> JobMessage {
    JobMessage::new("job-export")
        .number("done", done)
        .number("total", total)
        .text("target", "file on disk")
}

#[test]
fn an_exports_start_progress_and_end_fold_into_its_row() {
    let (mut mirror, id) = exporting(3);
    // Added: queued, with the counts at nothing and the folder its path names.
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Queued);
    assert!(row.cancellable);
    let details = row.export.expect("an export");
    assert_eq!((details.total, details.done, details.failed), (3, 0, 0));
    assert_eq!(details.folder, "$(FILE_FOLDER)/apertrust_exported");
    assert_eq!(row.message, export_message(0.0, 3.0));

    // Running, the first photo under way.
    mirror.apply(&state(id, State::Running));
    mirror.apply(&started(id, 0));
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Running);
    let details = row.export.expect("an export");
    assert_eq!(details.current.as_deref(), Some("IMG_0001.ARW"));

    // Photo after photo: counted, the folder the files really go to, the fraction as sent.
    mirror.apply(&written(id, 0));
    mirror.apply(&job(JobEvent::Progress {
        job: id,
        done: 1,
        total: Some(3),
    }));
    mirror.apply(&started(id, 1));
    mirror.apply(&written(id, 1));
    let row = only(&mirror);
    let details = row.export.clone().expect("an export");
    assert_eq!((details.done, details.current), (2, None));
    assert_eq!(details.folder, "/out");
    assert!((row.fraction - 1.0 / 3.0).abs() < 1e-6);
    assert_eq!(row.message, export_message(2.0, 3.0));

    // Done: finished, whole, with nothing to cancel.
    mirror.apply(&started(id, 2));
    mirror.apply(&written(id, 2));
    mirror.apply(&status(id, ExportStatus::Finished(Summary::default())));
    mirror.apply(&ended(id, State::Finished, None));
    let row = only(&mirror);
    assert_eq!(
        (row.phase, row.fraction, row.cancellable),
        (JobPhase::Finished, 1.0, false)
    );
    let details = row.export.expect("an export");
    assert_eq!(
        (details.done, details.failed, details.failure),
        (3, 0, None)
    );
}

#[test]
fn a_failed_export_says_which_photo_and_why_in_words() {
    let (mut mirror, id) = exporting(3);
    mirror.apply(&state(id, State::Running));
    mirror.apply(&started(id, 0));
    mirror.apply(&written(id, 0));
    mirror.apply(&started(id, 1));
    // The disk fills up: that photo failed, and the job ends, the last not reached.
    mirror.apply(&failed(id, 1, Cause::DiskFull));
    let end = JobFailure::new("disk-full", "an error in the engine's words")
        .item(photos(2).remove(1))
        .file("/out/IMG_0002.jpg");
    mirror.apply(&ended(id, State::Finished, Some(end)));

    let row = only(&mirror);
    assert_eq!((row.phase, row.cancellable), (JobPhase::Failed, false));
    let details = row.export.expect("an export");
    assert_eq!((details.done, details.failed), (1, 1));
    assert_eq!(
        details.failure,
        Some(ExportFailure {
            photo: "IMG_0002.ARW".to_owned(),
            file: "IMG_0002.jpg".to_owned(),
            cause: "the disk is full".to_owned(),
        })
    );
    // In Ukrainian the cause is in Ukrainian.
    let uk = mirror.view(&Translator::new(Locale::Uk));
    let cause = uk[0].export.as_ref().and_then(|d| d.failure.as_ref());
    assert_eq!(cause.map(|f| f.cause.as_str()), Some("диск заповнений"));
}

#[test]
fn an_export_refused_before_any_photo_names_no_photo_and_no_file() {
    let (mut mirror, id) = exporting(2);
    mirror.apply(&state(id, State::Running));
    let end = JobFailure::new("pattern", "the path holds an unknown variable");
    mirror.apply(&ended(id, State::Finished, Some(end)));
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Failed);
    let failure = row.export.and_then(|d| d.failure).expect("a failure");
    assert_eq!(
        (
            failure.photo.as_str(),
            failure.file.as_str(),
            failure.cause.as_str()
        ),
        ("", "", "the file name is not valid")
    );
}

#[test]
fn a_cancelled_export_keeps_what_it_wrote_and_offers_no_second_cancel() {
    let (mut mirror, id) = exporting(3);
    mirror.apply(&state(id, State::Running));
    mirror.apply(&started(id, 0));
    mirror.apply(&written(id, 0));
    mirror.apply(&started(id, 1));
    // Asked to stop, it goes on until its next check: no second Cancel meanwhile.
    mirror.apply(&state(id, State::Cancelled));
    let row = only(&mirror);
    assert_eq!((row.phase, row.cancellable), (JobPhase::Running, false));
    // Then it ends cancelled, the file it wrote staying counted.
    let summary = Summary {
        cancelled: true,
        ..Summary::default()
    };
    mirror.apply(&status(id, ExportStatus::Finished(summary)));
    mirror.apply(&ended(id, State::Cancelled, None));
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Cancelled);
    let details = row.export.expect("an export");
    assert_eq!((details.done, details.current), (1, None));
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
    assert_eq!(mirror.apply(&written(other, 0)), Show::Nothing);
    assert!(mirror.view(&english()).is_empty());

    mirror.apply(&added(one, Queue::SystemForeground, None));
    assert_eq!(mirror.view(&english()).len(), 1);
    let dropped = ended(one, State::Discarded, None);
    assert_eq!(mirror.apply(&dropped), Show::Update);
    assert!(mirror.view(&english()).is_empty());
}

#[test]
fn only_a_job_the_user_started_opens_the_window() {
    let id = ids(1)[0];
    let opens = |queue, command| {
        // Each in a mirror of its own, so a row never replaces another.
        let mut mirror = Mirror::new(None);
        mirror.apply(&added(id, queue, command))
    };
    // darktable's user queues.
    assert_eq!(opens(Queue::UserForeground, None), Show::Open);
    assert_eq!(opens(Queue::UserExport, Some(3)), Show::Open);
    // A background one when a command of the user's started it.
    assert_eq!(opens(Queue::UserBackground, Some(4)), Show::Open);
    // The system's, and a background job the system started, show but never open it.
    assert_eq!(opens(Queue::UserBackground, None), Show::Update);
    assert_eq!(opens(Queue::SystemForeground, None), Show::Update);
    assert_eq!(opens(Queue::SystemBackground, None), Show::Update);
    assert_eq!(opens(Queue::SystemBackground, Some(5)), Show::Update);
}

#[test]
fn a_job_that_is_not_an_export_says_how_it_ended() {
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
        row.message,
        JobMessage::new("job-import")
            .number("done", 2.0)
            .number("total", 9.0)
    );
    assert!(row.export.is_none());
    let failure = JobFailure::new("import", "the card was removed.");
    mirror.apply(&ended(id, State::Finished, Some(failure)));
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Failed);
    assert_eq!(row.error.as_deref(), Some("the card was removed"));
}

#[test]
fn the_buttons_ask_for_cancel_the_reveal_the_retry_and_the_dismissal() {
    let (mut mirror, id) = exporting(3);
    let number = id.get();
    mirror.apply(&state(id, State::Running));
    mirror.apply(&started(id, 0));
    mirror.apply(&written(id, 0));

    // Cancel is the session's CancelJob for the job.
    assert_eq!(
        mirror.act(ActivityAction::Cancel(number)),
        Do::Send {
            command: Command::CancelJob { job: id },
            retry_of: None,
        }
    );
    // Show in Finder reveals the first file it wrote.
    assert_eq!(
        mirror.act(ActivityAction::ShowFolder(number)),
        Do::Reveal(Reveal::File(PathBuf::from("/out/IMG_0001.jpg")))
    );
    // A running job is not dismissed, a retry needs a failed export.
    assert_eq!(mirror.act(ActivityAction::Dismiss(number)), Do::Nothing);
    assert_eq!(mirror.act(ActivityAction::TryAgain(number)), Do::Nothing);

    // The second photo fails, the disk is full, the third is not reached.
    mirror.apply(&started(id, 1));
    mirror.apply(&failed(id, 1, Cause::DiskFull));
    let end = JobFailure::new("disk-full", "no space left on device");
    mirror.apply(&ended(id, State::Finished, Some(end)));

    // Try again sends the export again: the photos not written, the same settings.
    let again = mirror.act(ActivityAction::TryAgain(number));
    let Do::Send {
        command:
            Command::Export {
                photos: again_photos,
                settings,
                ..
            },
        retry_of,
    } = &again
    else {
        panic!("Try again sends an export: {again:?}");
    };
    assert_eq!(*again_photos, photos(3)[1..].to_vec());
    assert_eq!(*settings, crate::export::Values::default().settings());
    assert_eq!(*retry_of, Some(number));
    assert_eq!(
        mirror.act(ActivityAction::Cancel(number + 100)),
        Do::Nothing
    );
}

#[test]
fn try_again_takes_the_failed_rows_place_whatever_number_its_job_has() {
    let ids = ids(2);
    let (first, second) = (ids[0], ids[1]);
    let mut mirror = Mirror::new(None);
    mirror.sent(1, &export(2), None);
    mirror.apply(&added(first, Queue::UserExport, Some(1)));
    mirror.apply(&state(first, State::Running));
    mirror.apply(&started(first, 0));
    mirror.apply(&failed(first, 0, Cause::Unreadable));
    mirror.apply(&started(first, 1));
    mirror.apply(&written(first, 1));
    mirror.apply(&ended(first, State::Finished, None));
    assert_eq!(only(&mirror).phase, JobPhase::Failed);

    let Do::Send { command, retry_of } = mirror.act(ActivityAction::TryAgain(first.get())) else {
        panic!("Try again sends the export");
    };
    // Only the photo that failed goes again.
    assert!(matches!(&command, Command::Export { photos, .. } if photos.len() == 1));
    mirror.sent(2, &command, retry_of);
    // The new job is a user job: it opens the window, and takes the failed row's place.
    assert_eq!(
        mirror.apply(&added(second, Queue::UserExport, Some(2))),
        Show::Open
    );
    let row = only(&mirror);
    assert_eq!((row.id, row.phase), (second.get(), JobPhase::Queued));
    assert_eq!(row.export.map(|d| d.total), Some(1));

    // A job that comes back under the number it had, as a retry that reuses it may, is one
    // row, queued again.
    mirror.apply(&state(second, State::Running));
    mirror.apply(&ended(second, State::Finished, None));
    mirror.apply(&added(second, Queue::UserExport, None));
    assert_eq!(only(&mirror).phase, JobPhase::Queued);
}

#[test]
fn dismissing_an_ended_row_takes_it_away() {
    let (mut mirror, id) = exporting(1);
    mirror.apply(&state(id, State::Running));
    mirror.apply(&ended(id, State::Cancelled, None));
    assert_eq!(mirror.act(ActivityAction::Dismiss(id.get())), Do::Changed);
    assert!(mirror.view(&english()).is_empty());
}

#[test]
fn show_in_finder_reveals_a_folder_before_anything_is_written_if_the_path_names_one() {
    let id = ids(1)[0];
    let mut mirror = Mirror::new(None);
    let values = crate::export::Values {
        path: "~/proofs/$(FILE_NAME)".to_owned(),
        ..crate::export::Values::default()
    };
    let command = Command::Export {
        photos: photos(1),
        settings: values.settings(),
        places: Places {
            home: Some(PathBuf::from("/Users/anna")),
            ..Places::default()
        },
    };
    mirror.sent(1, &command, None);
    mirror.apply(&added(id, Queue::UserExport, Some(1)));
    assert_eq!(
        mirror.act(ActivityAction::ShowFolder(id.get())),
        Do::Reveal(Reveal::Folder(PathBuf::from("/Users/anna/proofs")))
    );
    // A folder that depends on the photo cannot be shown before one is written.
    let id = ids(1)[0];
    mirror.sent(2, &export(1), None);
    mirror.apply(&added(id, Queue::UserExport, Some(2)));
    assert_eq!(
        mirror.act(ActivityAction::ShowFolder(id.get())),
        Do::Nothing
    );
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
        assert!(words.has("job-target-disk"), "{locale:?}");
    }
    // A code this build has no words for falls back to the engine's own.
    assert_eq!(
        cause_words(&english(), "from-the-future", Some("it broke.")),
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
fn the_window_is_named_jobs_and_grows_with_its_rows_until_they_scroll() {
    let one = window(many(1));
    one.get_by_role_and_label(Role::Dialog, "Jobs");
    let small = one.state().asked.last().expect("drawn").size;
    assert!(small.y < MAX_HEIGHT, "{small:?}");

    let lots = window(many(40));
    let tall = lots.state().asked.last().expect("drawn").size;
    // Capped: the rows past it scroll, within the window's margin.
    let margin = theme(&lots.ctx).spacing.l;
    assert!(tall.y <= MAX_HEIGHT + 2.0 * margin + 1.0, "{tall:?}");
    assert!(tall.y > small.y);
    assert_eq!(tall.x, small.x);
    assert!(tall.x >= WIDTH, "{tall:?}");
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
fn quitting_counts_the_jobs_the_user_started_that_have_not_ended() {
    let jobs = ids(5);
    let mut mirror = Mirror::new(None);
    assert!(mirror.running().none());
    // An export (stoppable), a delete that cannot be stopped, and a background job a
    // command of the user's started.
    mirror.apply(&added(jobs[0], Queue::UserExport, None));
    mirror.apply(&added_stoppable(
        jobs[1],
        Queue::UserForeground,
        None,
        false,
    ));
    mirror.apply(&added(jobs[2], Queue::UserBackground, Some(3)));
    // A system job and a background one no command of the user's started never count.
    mirror.apply(&added(jobs[3], Queue::SystemBackground, None));
    mirror.apply(&added(jobs[4], Queue::UserBackground, None));
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
    mirror.sent(7, &export(2), None);
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
        (more[1], Some(JobFailure::new("write", "cannot write"))),
    ] {
        mirror.apply(&added(job_id, Queue::UserExport, None));
        mirror.apply(&ended(job_id, State::Finished, outcome));
    }
    assert!(mirror.running().none());
}

#[test]
fn the_engine_stopping_fails_the_running_rows_and_forgets_the_exports_sent() {
    let (mut mirror, running) = exporting(3);
    mirror.apply(&state(running, State::Running));
    mirror.apply(&started(running, 0));
    // One that ended before keeps how it ended; one sent has no job yet.
    let done = ids(2)[1];
    mirror.apply(&added(done, Queue::UserExport, None));
    mirror.apply(&ended(done, State::Finished, None));
    mirror.sent(9, &export(1), None);
    assert!(!mirror.running().none());

    assert_eq!(mirror.engine_stopped(), Show::Update);

    assert!(mirror.running().none());
    assert!(mirror.stoppable().is_empty());
    let view = mirror.view(&english());
    let row = view.iter().find(|job| job.id == running.get()).unwrap();
    assert_eq!((row.phase, row.cancellable), (JobPhase::Failed, false));
    let details = row.export.as_ref().expect("an export");
    assert_eq!(details.current, None);
    assert_eq!(
        details.failure.as_ref().map(|f| f.cause.as_str()),
        Some("the engine stopped unexpectedly")
    );
    let row = view.iter().find(|job| job.id == done.get()).unwrap();
    assert_eq!(row.phase, JobPhase::Finished);
    // In Ukrainian the cause is in Ukrainian, and a second stop changes nothing.
    let uk = mirror.view(&Translator::new(Locale::Uk));
    let cause = uk
        .iter()
        .find_map(|job| job.export.as_ref()?.failure.as_ref());
    assert!(cause.is_some_and(|f| f.cause != "job-cause-engine"));
    assert_eq!(mirror.engine_stopped(), Show::Nothing);
}

#[test]
fn a_job_that_is_not_an_export_fails_with_the_engine_s_stop_as_its_cause() {
    let job_id = ids(1)[0];
    let mut mirror = Mirror::new(None);
    mirror.apply(&added(job_id, Queue::UserForeground, None));
    mirror.apply(&state(job_id, State::Running));
    mirror.engine_stopped();
    let row = only(&mirror);
    assert_eq!(row.phase, JobPhase::Failed);
    assert_eq!(
        row.error.as_deref(),
        Some("the engine stopped unexpectedly")
    );
}
