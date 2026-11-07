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

//! The app's jobs (`docs/gui.md`, The jobs window; decision 57): the mirror of what the
//! session reports of them, the window that shows it, and what the window's buttons do.
//!
//! - **The mirror** ([`Mirror`]) folds the session's `Event::Job` and `Event::Export` into
//!   the design system's [`Job`] rows, as the app's mirror (`mirror.rs`, decision 59's
//!   reducer) takes the events in. It starts empty and holds only the jobs
//!   this run's events have named. An export keeps the request that started it, so Try
//!   again can send the photos that did not get written, and what each photo became; the
//!   failure's cause stays the engine's code until a row is drawn, when Fluent turns it into
//!   words in the language then.
//! - **When the window opens itself** is [`Mirror::apply`]'s answer: only for a job the user
//!   started, one of darktable's user queues (`UserForeground`, `UserExport`, and
//!   `UserBackground` when a command of the user's started it). A system job turns the
//!   spinner and shows in the window, but never opens it.
//! - **The window** ([`Window`]) is the content of a window of the dialog host, modeless and
//!   beside the main window, whose rows `Jobs::window` draws; it scrolls when they are too
//!   many to show. Closing it stops no job.
//! - **What its buttons do** is the app's: Cancel sends `CancelJob`, Dismiss and Try again
//!   change the mirror (Try again also sends the export), and Show in Finder reveals the
//!   first file written, or the folder, through the platform ([`reveal`]).

use std::path::{Path, PathBuf};

use apertrust_engine::jobs::{Argument, JobEvent, JobId, Message, Queue, State};
use apertrust_engine::session::{Command, Event, ExportStatus, Failure, PhotoResult};
use apertrust_export::{ExportSettings, Places};
use apertrust_ui::components::{
    ActivityAction, ExportDetails, ExportFailure, FileManager, Job, JobMessage, JobPhase, Jobs,
    quiet_scroll,
};
use apertrust_ui::{Theme, Translator, theme, tr};
use eframe::egui;
use eframe::egui::os::OperatingSystem;

use crate::dialog_host::{AppDialogs, Asks, Dialog, Form, Keys, Look, Output};
use crate::settings::Position;

/// The exports' request: what an export job was sent, to send the photos it did not write
/// again with the same settings.
#[derive(Clone, Debug, PartialEq)]
struct Request {
    photos: Vec<PathBuf>,
    settings: ExportSettings,
    places: Places,
}

/// An export the app sent, until the session adds its job.
#[derive(Debug)]
struct Sent {
    /// The number [`Session::send`](apertrust_engine::session::Session::send) gave it.
    command: u64,
    request: Request,
    /// The job whose row this export replaces: the failed one it tries again.
    retry_of: Option<u64>,
}

/// What became of one photo of an export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Photo {
    /// Not reached yet, or never.
    Waiting,
    Written,
    /// Left out as On conflict says.
    Skipped,
    Failed,
}

/// What an export row keeps beyond any job's.
#[derive(Debug)]
struct ExportRow {
    request: Request,
    photos: Vec<Photo>,
    /// The photo under way.
    current: Option<usize>,
    /// The first file written or left, which Show in Finder reveals.
    first_file: Option<PathBuf>,
    /// The first photo's failure.
    failure: Option<(usize, Failure)>,
}

/// One job as the session reported it.
#[derive(Debug)]
struct Row {
    id: JobId,
    phase: JobPhase,
    message: Message,
    fraction: f32,
    cancellable: bool,
    /// Whether the user started it, as darktable's user queues have it: quitting asks first
    /// while one runs.
    user: bool,
    /// How it ended, if its body failed.
    failure: Option<Message>,
    export: Option<ExportRow>,
}

/// What a session event asks of the jobs window, the larger the more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Show {
    /// Nothing: the event is not a job's, or changes nothing a row shows.
    Nothing,
    /// The jobs changed: the window open shows them.
    Update,
    /// A job the user started: the window opens, if it is not open, without taking the
    /// focus.
    Open,
    /// The user asked for the window: it opens and comes to the front.
    Front,
}

/// The user's jobs that have not ended, as quitting asks about them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Running {
    /// Those a cancel stops.
    pub stoppable: usize,
    /// Those it cannot, or that were asked to stop and have not yet.
    pub waiting: usize,
}

impl Running {
    /// Whether none runs: quitting asks nothing.
    pub fn none(self) -> bool {
        self.stoppable == 0 && self.waiting == 0
    }
}

/// The jobs the session has reported, as the window shows them.
#[derive(Debug, Default)]
pub struct Mirror {
    rows: Vec<Row>,
    sent: Vec<Sent>,
    /// The person's home, which folders are shown relative to as `~`.
    home: Option<PathBuf>,
}

/// What a button of the jobs window asks the app to do.
#[derive(Debug, PartialEq)]
pub enum Do {
    /// Nothing: the job is gone, or there is nothing to do for it.
    Nothing,
    /// Send the session a command; `retry_of` names the failed export it tries again.
    Send {
        command: Command,
        retry_of: Option<u64>,
    },
    /// Show something in the platform's file manager.
    Reveal(Reveal),
    /// The mirror changed: the window shows it.
    Changed,
}

/// What Show in Finder reveals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reveal {
    /// A file written, selected in its folder.
    File(PathBuf),
    /// A folder, opened.
    Folder(PathBuf),
}

impl Mirror {
    pub fn new(home: Option<PathBuf>) -> Mirror {
        Mirror {
            rows: Vec::new(),
            sent: Vec::new(),
            home,
        }
    }

    /// Notes a command the app sent, numbered `number`: an export waits for its job to be
    /// added, so its row knows what to send again. `retry_of` is the failed job it tries
    /// again, whose row it takes the place of.
    pub fn sent(&mut self, number: u64, command: &Command, retry_of: Option<u64>) {
        if let Command::Export {
            photos,
            settings,
            places,
        } = command
        {
            self.sent.push(Sent {
                command: number,
                request: Request {
                    photos: photos.clone(),
                    settings: settings.clone(),
                    places: places.clone(),
                },
                retry_of,
            });
        }
    }

    /// Folds `event` in, and says what the window is to do about it.
    pub fn apply(&mut self, event: &Event) -> Show {
        match event {
            Event::Job { event, command } => self.job(event, *command),
            Event::Export { job, status } => self.export(*job, status),
            _ => Show::Nothing,
        }
    }

    /// The engine thread has ended, by a panic or by stopping: no job it ran will report
    /// again. The exports sent and not yet added are forgotten, and every row still queued
    /// or running ends as failed with the engine's stop as its cause, so that quitting waits
    /// for nothing.
    pub fn engine_stopped(&mut self) -> Show {
        let mut changed = !self.sent.is_empty();
        self.sent.clear();
        for row in self.rows.iter_mut().filter(|row| row.phase.active()) {
            changed = true;
            row.phase = JobPhase::Failed;
            row.cancellable = false;
            row.failure = Some(Message::new("job-export-failed").with("cause", "engine"));
            if let Some(export) = row.export.as_mut() {
                export.current = None;
            }
        }
        if changed { Show::Update } else { Show::Nothing }
    }

    fn row(&mut self, job: JobId) -> Option<&mut Row> {
        self.rows.iter_mut().find(|row| row.id == job)
    }

    fn job(&mut self, event: &JobEvent, command: Option<u64>) -> Show {
        match event {
            JobEvent::Added {
                job,
                queue,
                message,
                cancellable,
                ..
            } => {
                let sent = command.and_then(|number| {
                    let at = self.sent.iter().position(|sent| sent.command == number)?;
                    Some(self.sent.remove(at))
                });
                // A row under this id is the same job's, or a retry's that reuses it.
                self.rows.retain(|row| row.id != *job);
                if let Some(old) = sent.as_ref().and_then(|sent| sent.retry_of) {
                    self.rows.retain(|row| row.id.get() != old);
                }
                let export = sent.map(|sent| ExportRow {
                    photos: vec![Photo::Waiting; sent.request.photos.len()],
                    request: sent.request,
                    current: None,
                    first_file: None,
                    failure: None,
                });
                // A job the user started: darktable's user queues, and a background one a
                // command of the user's started, but not a folder's reading, which shows in
                // the Library tab and the grid panel (the spinner turns, the window stays
                // shut).
                let user = match queue {
                    Some(Queue::UserForeground | Queue::UserExport) => true,
                    Some(Queue::UserBackground) => command.is_some() && message.id != "job-folder",
                    _ => false,
                };
                self.rows.push(Row {
                    id: *job,
                    phase: JobPhase::Queued,
                    message: message.clone(),
                    fraction: 0.0,
                    cancellable: *cancellable,
                    user,
                    failure: None,
                    export,
                });
                if user { Show::Open } else { Show::Update }
            }
            JobEvent::Progress { job, fraction } => {
                self.update(*job, |row| row.fraction = *fraction as f32)
            }
            JobEvent::Message { job, message } => {
                self.update(*job, |row| row.message = message.clone())
            }
            JobEvent::State { job, state } => self.update(*job, |row| match state {
                State::Queued => row.phase = JobPhase::Queued,
                State::Running => row.phase = JobPhase::Running,
                // A running job asked to stop goes on until its next check: no second ask.
                State::Cancelled => row.cancellable = false,
                State::Initialized | State::Finished | State::Discarded => {}
            }),
            JobEvent::Ended {
                job,
                state,
                failure,
            } => {
                if *state == State::Discarded {
                    // Dropped unrun: there is nothing to show of it.
                    let before = self.rows.len();
                    self.rows.retain(|row| row.id != *job);
                    return if self.rows.len() == before {
                        Show::Nothing
                    } else {
                        Show::Update
                    };
                }
                self.update(*job, |row| {
                    row.cancellable = false;
                    row.failure = failure.clone();
                    let photo_failed = row
                        .export
                        .as_ref()
                        .is_some_and(|export| export.failure.is_some());
                    row.phase = match state {
                        State::Cancelled => JobPhase::Cancelled,
                        _ if failure.is_some() || photo_failed => JobPhase::Failed,
                        _ => {
                            row.fraction = 1.0;
                            JobPhase::Finished
                        }
                    };
                    if let Some(export) = row.export.as_mut() {
                        export.current = None;
                    }
                })
            }
        }
    }

    fn update(&mut self, job: JobId, change: impl FnOnce(&mut Row)) -> Show {
        match self.row(job) {
            Some(row) => {
                change(row);
                Show::Update
            }
            None => Show::Nothing,
        }
    }

    fn export(&mut self, job: JobId, status: &ExportStatus) -> Show {
        self.update(job, |row| {
            let Some(export) = row.export.as_mut() else {
                return;
            };
            match status {
                ExportStatus::Started { index, .. } => export.current = Some(*index),
                ExportStatus::Done { index, result } => {
                    export.current = None;
                    let photo = match result {
                        PhotoResult::Written { path, .. } => {
                            export.first_file.get_or_insert_with(|| path.clone());
                            Photo::Written
                        }
                        PhotoResult::Skipped { path, .. } => {
                            export.first_file.get_or_insert_with(|| path.clone());
                            Photo::Skipped
                        }
                        PhotoResult::Failed(failure) => {
                            export
                                .failure
                                .get_or_insert_with(|| (*index, failure.clone()));
                            Photo::Failed
                        }
                    };
                    if let Some(slot) = export.photos.get_mut(*index) {
                        *slot = photo;
                    }
                }
                ExportStatus::Finished(_) => export.current = None,
            }
        })
    }

    /// What a button of the window asks of the app: Cancel is the session's `CancelJob`;
    /// Show in Finder reveals what the export wrote; Try again is the export of the photos
    /// not written, which takes the failed row's place once its job is added; Dismiss takes
    /// an ended row away.
    pub fn act(&mut self, action: ActivityAction) -> Do {
        match action {
            ActivityAction::Cancel(job) => match self.rows.iter().find(|r| r.id.get() == job) {
                Some(row) => Do::Send {
                    command: Command::CancelJob { job: row.id },
                    retry_of: None,
                },
                None => Do::Nothing,
            },
            ActivityAction::ShowFolder(job) => self.reveal(job).map_or(Do::Nothing, Do::Reveal),
            ActivityAction::TryAgain(job) => {
                self.retry(job).map_or(Do::Nothing, |command| Do::Send {
                    command,
                    retry_of: Some(job),
                })
            }
            ActivityAction::Dismiss(job) => match self.dismiss(job) {
                Show::Nothing => Do::Nothing,
                _ => Do::Changed,
            },
        }
    }

    /// The jobs the user started that are queued or running, as quitting counts them. An
    /// export sent and not yet added has no id to cancel, so it is waited for.
    pub fn running(&self) -> Running {
        let mut running = Running {
            stoppable: 0,
            waiting: self.sent.len(),
        };
        for row in self
            .rows
            .iter()
            .filter(|row| row.user && row.phase.active())
        {
            if row.cancellable {
                running.stoppable += 1;
            } else {
                running.waiting += 1;
            }
        }
        running
    }

    /// The user's jobs that are queued or running and can be stopped: what a confirmed quit
    /// cancels.
    pub fn stoppable(&self) -> Vec<JobId> {
        self.rows
            .iter()
            .filter(|row| row.user && row.phase.active() && row.cancellable)
            .map(|row| row.id)
            .collect()
    }

    /// The jobs as the window and the spinner draw them, in the language of `words`.
    pub fn view(&self, words: &Translator) -> Vec<Job> {
        self.rows
            .iter()
            .map(|row| row.view(words, self.home.as_deref()))
            .collect()
    }

    /// Takes the job's row away, if it has ended: Dismiss.
    fn dismiss(&mut self, job: u64) -> Show {
        let before = self.rows.len();
        self.rows
            .retain(|row| row.id.get() != job || row.phase.active());
        if self.rows.len() == before {
            Show::Nothing
        } else {
            Show::Update
        }
    }

    /// The export that tries a failed one again: the photos that were not written, with the
    /// same settings, each photo's edit as it is when it is sent. None for a job that is
    /// not a failed export, or that wrote every photo.
    fn retry(&self, job: u64) -> Option<Command> {
        let row = self.rows.iter().find(|row| row.id.get() == job)?;
        let export = row.export.as_ref()?;
        if row.phase != JobPhase::Failed {
            return None;
        }
        let photos: Vec<PathBuf> = export
            .request
            .photos
            .iter()
            .zip(&export.photos)
            .filter(|(_, photo)| matches!(photo, Photo::Waiting | Photo::Failed))
            .map(|(path, _)| path.clone())
            .collect();
        (!photos.is_empty()).then(|| Command::Export {
            photos,
            settings: export.request.settings.clone(),
            places: export.request.places.clone(),
        })
    }

    /// What Show in Finder reveals for an export: the first file it wrote or left, or, with
    /// none yet, the folder its path names where that names one.
    fn reveal(&self, job: u64) -> Option<Reveal> {
        let export = self
            .rows
            .iter()
            .find(|row| row.id.get() == job)?
            .export
            .as_ref()?;
        if let Some(file) = &export.first_file {
            return Some(Reveal::File(file.clone()));
        }
        let folder = typed_folder(&export.request.settings.path)?;
        (!folder.contains("$("))
            .then(|| Reveal::Folder(expand_home(&folder, &export.request.places)))
    }
}

impl Row {
    fn view(&self, words: &Translator, home: Option<&Path>) -> Job {
        let job = |message| {
            Job::new(self.id.get(), self.phase, message)
                .fraction(self.fraction)
                .cancellable(self.cancellable && self.phase.active())
        };
        match &self.export {
            // An export's message is its counts, which change as photos are written.
            Some(export) => {
                let details = export.details(self, words, home);
                let message = JobMessage::new("job-export")
                    .number("done", details.done as f64)
                    .number("total", details.total as f64)
                    .text("target", words.text("job-target-disk"));
                job(message).export(details)
            }
            None => {
                let mut job = job(job_message(&self.message));
                if let Some(failure) = &self.failure {
                    job = job.error(failure_words(words, failure));
                }
                job
            }
        }
    }
}

impl ExportRow {
    fn details(&self, row: &Row, words: &Translator, home: Option<&Path>) -> ExportDetails {
        let count = |wanted: fn(&Photo) -> bool| self.photos.iter().filter(|p| wanted(p)).count();
        let folder = match &self.first_file {
            Some(file) => file
                .parent()
                .map(|folder| crate::export::shown(folder, home)),
            None => typed_folder(&self.request.settings.path),
        };
        let mut details = ExportDetails::new(
            self.request.photos.len(),
            folder.unwrap_or_else(|| self.request.settings.path.clone()),
        );
        details.done = count(|p| matches!(p, Photo::Written | Photo::Skipped));
        details.failed = count(|p| *p == Photo::Failed);
        details.current = self.current.map(|i| self.name(i));
        details.failure = self.failure_words(row, words);
        details
    }

    /// The raw's file name, `IMG_2047.ARW`.
    fn name(&self, index: usize) -> String {
        self.request
            .photos
            .get(index)
            .map(|path| file_name(path))
            .unwrap_or_default()
    }

    /// The failure to explain: the first photo's, else the job's own, such as a path that
    /// is not valid.
    fn failure_words(&self, row: &Row, words: &Translator) -> Option<ExportFailure> {
        if let Some((index, failure)) = &self.failure {
            return Some(ExportFailure {
                photo: self.name(*index),
                file: failure.file.as_deref().map(file_name).unwrap_or_default(),
                cause: cause_words(words, failure.cause.id(), Some(&failure.message)),
            });
        }
        let message = row.failure.as_ref()?;
        let name = |name: &str| match message.argument(name) {
            Some(Argument::Path(path)) => file_name(path),
            _ => String::new(),
        };
        Some(ExportFailure {
            photo: name("photo"),
            file: name("file"),
            cause: failure_words(words, message),
        })
    }
}

/// An engine message as the window's: the same id and arguments, a path as text.
fn job_message(message: &Message) -> JobMessage {
    let mut out = JobMessage::new(&message.id);
    for (name, value) in &message.arguments {
        out = match value {
            Argument::Number(n) => out.number(name, *n),
            Argument::Text(t) => out.text(name, t),
            Argument::Path(p) => out.text(name, file_name(p)),
        };
    }
    out
}

/// The words for a failed job's message: its cause in Fluent where the engine sent a code,
/// else the reason it gave, else the message itself.
fn failure_words(words: &Translator, failure: &Message) -> String {
    let text = |name: &str| match failure.argument(name) {
        Some(Argument::Text(text)) => Some(text.as_str()),
        _ => None,
    };
    match text("cause") {
        Some(cause) => cause_words(words, cause, text("reason")),
        None => text("reason").map_or_else(
            || job_message_words(words, failure),
            |reason| reason.trim_end_matches('.').to_owned(),
        ),
    }
}

fn job_message_words(words: &Translator, message: &Message) -> String {
    let mut args = apertrust_ui::i18n::Args::new();
    for (name, value) in &message.arguments {
        match value {
            Argument::Number(n) => args.set(name.as_str(), *n),
            Argument::Text(t) => args.set(name.as_str(), t.as_str()),
            Argument::Path(p) => args.set(name.as_str(), file_name(p)),
        }
    }
    words.text_with(&message.id, Some(&args))
}

/// A cause's words, `job-cause-<code>`, without a full stop; the engine's own `reason` for a
/// code this build has no words for.
fn cause_words(words: &Translator, code: &str, reason: Option<&str>) -> String {
    let id = format!("job-cause-{code}");
    let text = words.text(&id);
    if text != id {
        return text;
    }
    reason.map_or(text, |reason| reason.trim_end_matches('.').to_owned())
}

/// `path`'s last component, `IMG_2047.ARW`.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// The folder an export's path pattern names, as typed: the pattern without its last
/// component, which names the file.
fn typed_folder(pattern: &str) -> Option<String> {
    let folder = Path::new(pattern).parent()?.to_str()?;
    (!folder.is_empty()).then(|| folder.to_owned())
}

/// `folder` with a leading `~` the person's home, as darktable's paths have it.
fn expand_home(folder: &str, places: &Places) -> PathBuf {
    match (folder.strip_prefix('~'), &places.home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(folder),
    }
}

/// The program and arguments that show `what` in the platform's file manager.
pub fn reveal_command(what: &Reveal) -> (&'static str, Vec<std::ffi::OsString>) {
    let (file, path) = match what {
        Reveal::File(path) => (true, path),
        Reveal::Folder(path) => (false, path),
    };
    // Absolute, so a name that starts with `-` is never read as an option.
    let path = &std::path::absolute(path).unwrap_or_else(|_| path.clone());
    if cfg!(target_os = "macos") {
        // Finder opens the folder with the file selected, or the folder itself.
        let mut args = Vec::new();
        if file {
            args.push("-R".into());
        }
        args.push(path.into());
        ("open", args)
    } else if cfg!(target_os = "windows") {
        let mut arg = std::ffi::OsString::from(if file { "/select," } else { "" });
        arg.push(path);
        ("explorer", vec![arg])
    } else {
        // The desktop's file manager opens a folder; a file shows by its folder.
        let folder = if file {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        ("xdg-open", vec![folder.into()])
    }
}

/// Shows `what` in the platform's file manager, started and left to its own: the UI thread
/// never waits for it.
pub fn reveal(what: &Reveal) {
    let (program, args) = reveal_command(what);
    match std::process::Command::new(program).args(args).spawn() {
        Ok(mut child) => {
            // Reaped by a thread of its own, so it never lingers as a zombie; a failed
            // call is logged.
            std::thread::spawn(move || match child.wait() {
                Ok(status) if !status.success() => {
                    tracing::warn!(target: "pipe", "the file manager call failed: {status}");
                }
                Err(e) => tracing::warn!(target: "pipe", "the file manager call failed: {e}"),
                Ok(_) => {}
            });
        }
        Err(e) => tracing::warn!(target: "pipe", "cannot show the export in the file manager: {e}"),
    }
}

/// The platform's file manager, for the button's words.
pub const FILE_MANAGER: FileManager = if cfg!(target_os = "macos") {
    FileManager::Finder
} else if cfg!(target_os = "windows") {
    FileManager::Explorer
} else {
    FileManager::Files
};

/// What `show` asks of the jobs window, after the session's events are taken in: it opens,
/// if the event is a user job's, or comes forward; and, open, shows `jobs` as the mirror
/// holds them now, in the language of `ctx`.
pub fn refresh(
    jobs: &[Job],
    dialogs: &mut AppDialogs,
    show: Show,
    place: Option<Position>,
    ctx: &egui::Context,
) {
    if matches!(show, Show::Open | Show::Front) {
        let window = Window::new(jobs.to_vec());
        dialogs.open_jobs(window, place, ctx, show == Show::Front);
    }
    dialogs.set_jobs(jobs.to_vec());
}

/// The window's width, in points, inside its margin.
pub const WIDTH: f32 = 440.0;
/// The most it grows to before the rows scroll, in points.
pub const MAX_HEIGHT: f32 = 480.0;

/// The jobs window's content: the rows as the mirror last gave them.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    jobs: Vec<Job>,
}

/// The jobs window in the dialog host, the host's second slot: modeless on every platform,
/// kept where it was left, on the raised surface; no key of its own closes it, Close Window
/// doing so on the Mac while it has the focus.
impl Dialog for Window {
    fn id(&self) -> &'static str {
        "jobs"
    }

    fn title(&self, ctx: &egui::Context) -> String {
        tr(ctx, "jobs-name")
    }

    fn form(&self, _os: OperatingSystem) -> Form {
        Form::Modeless
    }

    fn look(&self, t: &Theme) -> Look {
        Look {
            background: t.colors.raised,
            margin: t.spacing.l,
        }
    }

    fn keys(&self, _os: OperatingSystem) -> Keys {
        Keys::default()
    }

    fn show(&mut self, ui: &mut egui::Ui) -> Asks {
        Asks {
            output: self.rows(ui).map(Output::Jobs),
            ..Asks::default()
        }
    }

    fn placed(&self, at: Position) -> Option<Output> {
        Some(Output::JobsPlaced(at))
    }
}

impl Window {
    pub fn new(jobs: Vec<Job>) -> Window {
        Window { jobs }
    }

    /// The rows it shows.
    #[cfg(test)]
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// The rows it shows, newest first as they are drawn. Returns whether they changed, for
    /// the window to be drawn again.
    pub fn set(&mut self, jobs: Vec<Job>) -> bool {
        let changed = self.jobs != jobs;
        self.jobs = jobs;
        changed
    }

    /// Draws the rows in `ui`, scrolling where they are more than fit. Returns what the
    /// user pressed.
    fn rows(&mut self, ui: &mut egui::Ui) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        ui.set_width(WIDTH);
        ui.spacing_mut().item_spacing = egui::vec2(t.spacing.m, t.spacing.m);
        let area = egui::ScrollArea::vertical()
            .max_height(MAX_HEIGHT)
            .auto_shrink([false, true]);
        let jobs = &self.jobs;
        quiet_scroll(ui, area, |ui| {
            Jobs::new(jobs).file_manager(FILE_MANAGER).window(ui)
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
