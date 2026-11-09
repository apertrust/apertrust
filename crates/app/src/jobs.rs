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

//! The app's jobs (`docs/gui.md`, The jobs window; decisions 57 and 72): the mirror of what
//! the session reports of them, the window that shows it, and what the window's buttons do.
//!
//! - **The mirror** ([`Mirror`]) folds the session's `Event::Job` into the design system's
//!   [`Job`] rows, as the app's mirror (`mirror.rs`, decision 59's reducer) takes the events
//!   in: each event sets its part of its job's row, with no code for any kind of job
//!   (decision 72). It starts empty and holds only the jobs this run's events have named.
//!   A failure's cause stays the job's code until a row is drawn, when Fluent turns it
//!   into words in the language then.
//! - **When the window opens itself** is [`Mirror::apply`]'s answer: only for a job added
//!   to one of darktable's user queues, `UserForeground` or `UserExport`, whose `Added`
//!   names a command this app sent. Every other job turns the spinner and shows in the
//!   window, but never opens it.
//! - **The window** ([`Window`]) is the content of a window of the dialog host, modeless and
//!   beside the main window, whose rows `Jobs::window` draws; it scrolls when they are too
//!   many to show. Closing it stops no job.
//! - **What its buttons do** is the app's: Cancel sends `CancelJob`, Dismiss changes the
//!   mirror, and Show in Finder reveals the job's destination through the platform
//!   ([`reveal`]), a file selected in its folder. The rows keep a job's retry offer, but
//!   the window draws no Try again until the session takes `Command::RetryJob` (decision
//!   72's step 6).

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use apertrust_engine::jobs::{
    Argument, Destination, Failure, JobEvent, JobId, Message, Offer, Progress, Queue, State,
};
use apertrust_engine::session::{Command, Event};
use apertrust_ui::components::{
    ActivityAction, FileManager, Job, JobFailure, JobMessage, JobPhase, Jobs, quiet_scroll,
};
use apertrust_ui::{Theme, Translator, theme, tr};
use eframe::egui;
use eframe::egui::os::OperatingSystem;

use crate::dialog_host::{AppDialogs, Asks, Dialog, Form, Keys, Look, Output};
use crate::settings::Position;

/// One job's row, each part as the job's events last set it.
#[derive(Debug)]
struct Row {
    id: JobId,
    kind: &'static str,
    /// Whether it is on one of darktable's user queues, `UserForeground` or `UserExport`:
    /// quitting asks first while one runs.
    user: bool,
    phase: JobPhase,
    title: Message,
    progress: Progress,
    cancellable: bool,
    /// The line under its bar, none once it has ended.
    detail: Option<Message>,
    destination: Option<Destination>,
    outcome: Option<Message>,
    failure: Option<Failure>,
    ended_at: Option<SystemTime>,
    offers: Vec<Offer>,
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
    /// The commands the app sent that start a job of the user's, by their numbers, until
    /// the job's `Added` names them: quitting waits for them.
    starting: Vec<u64>,
    /// The person's home, which folders are shown relative to as `~`.
    home: Option<PathBuf>,
}

/// What a button of the jobs window asks the app to do.
#[derive(Debug, PartialEq)]
pub enum Do {
    /// Nothing: the job is gone, or there is nothing to do for it.
    Nothing,
    /// Send the session a command.
    Send(Command),
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
            starting: Vec::new(),
            home,
        }
    }

    /// Notes a command the app sent, numbered `number`, that starts a job of the user's,
    /// such as an export: until its job is added it has no number to cancel, so quitting
    /// waits for it.
    pub fn starting(&mut self, number: u64) {
        self.starting.push(number);
    }

    /// Folds `event` in, and says what the window is to do about it.
    pub fn apply(&mut self, event: &Event) -> Show {
        match event {
            Event::Job { event, command } => self.job(event, *command),
            _ => Show::Nothing,
        }
    }

    /// The engine thread has ended, by a panic or by stopping: no job it ran will report
    /// again. The commands sent and not yet added are forgotten, and every row still queued
    /// or running ends as failed with the engine's stop as its cause, so that quitting
    /// waits for nothing.
    pub fn engine_stopped(&mut self) -> Show {
        let mut changed = !self.starting.is_empty();
        self.starting.clear();
        for row in self.rows.iter_mut().filter(|row| row.phase.active()) {
            changed = true;
            row.phase = JobPhase::Failed;
            row.cancellable = false;
            row.detail = None;
            row.failure = Some(Failure::new("engine", "the engine stopped"));
        }
        if changed { Show::Update } else { Show::Nothing }
    }

    fn row(&mut self, job: JobId) -> Option<&mut Row> {
        self.rows.iter_mut().find(|row| row.id == job)
    }

    /// Sets the part of its job's row that `event` carries.
    fn job(&mut self, event: &JobEvent, command: Option<u64>) -> Show {
        match event {
            JobEvent::Added {
                job,
                kind,
                queue,
                title,
                cancellable,
                destination,
                detail,
                progress,
                retries,
            } => {
                if let Some(number) = command {
                    self.starting.retain(|starting| *starting != number);
                }
                // A row under this id is the same job's, or a retry's that reuses it; the
                // job it tries again gives it its place.
                self.rows
                    .retain(|row| row.id != *job && Some(row.id) != *retries);
                let user = matches!(queue, Some(Queue::UserForeground | Queue::UserExport));
                self.rows.push(Row {
                    id: *job,
                    kind,
                    user,
                    phase: JobPhase::Queued,
                    title: title.clone(),
                    progress: *progress,
                    cancellable: *cancellable,
                    detail: detail.clone(),
                    destination: destination.clone(),
                    outcome: None,
                    failure: None,
                    ended_at: None,
                    offers: Vec::new(),
                });
                // Only this app's commands open the window: another client's export shows
                // in it, but never opens it.
                if user && command.is_some() {
                    Show::Open
                } else {
                    Show::Update
                }
            }
            JobEvent::Progress { job, done, total } => self.update(*job, |row| {
                row.progress = Progress {
                    done: *done,
                    total: *total,
                };
            }),
            JobEvent::Title { job, title } => self.update(*job, |row| row.title = title.clone()),
            JobEvent::Detail { job, detail } => {
                self.update(*job, |row| row.detail = detail.clone())
            }
            JobEvent::Destination { job, destination } => {
                self.update(*job, |row| row.destination = Some(destination.clone()))
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
                outcome,
                failure,
                at,
                offers,
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
                    // A job ends failed exactly when it carries a failure, cancelled or not
                    // (decision 72).
                    row.phase = match state {
                        _ if failure.is_some() => JobPhase::Failed,
                        State::Cancelled => JobPhase::Cancelled,
                        _ => JobPhase::Finished,
                    };
                    row.cancellable = false;
                    // An ended row has no detail line: its outcome says how it went.
                    row.detail = None;
                    row.outcome = outcome.clone();
                    row.failure = failure.clone();
                    row.ended_at = Some(*at);
                    row.offers = offers.clone();
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

    /// What a button of the window asks of the app: Cancel is the session's `CancelJob`;
    /// Show in Finder reveals the job's destination; Dismiss takes an ended row away. Try
    /// again asks nothing until the session takes `RetryJob` (decision 72's step 6).
    pub fn act(&mut self, action: ActivityAction) -> Do {
        match action {
            ActivityAction::Cancel(job) => match self.rows.iter().find(|r| r.id.get() == job) {
                Some(row) => Do::Send(Command::CancelJob { job: row.id }),
                None => Do::Nothing,
            },
            ActivityAction::ShowFolder(job) => self.reveal(job).map_or(Do::Nothing, Do::Reveal),
            ActivityAction::TryAgain(_) => Do::Nothing,
            ActivityAction::Dismiss(job) => match self.dismiss(job) {
                Show::Nothing => Do::Nothing,
                _ => Do::Changed,
            },
        }
    }

    /// The jobs the user started that are queued or running, as quitting counts them. A
    /// command sent and not yet added has no id to cancel, so it is waited for.
    pub fn running(&self) -> Running {
        let mut running = Running {
            stoppable: 0,
            waiting: self.starting.len(),
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

    /// What Show in Finder reveals: the job's destination, a file selected in its folder.
    fn reveal(&self, job: u64) -> Option<Reveal> {
        let row = self.rows.iter().find(|row| row.id.get() == job)?;
        Some(match row.destination.as_ref()? {
            Destination::Folder(folder) => Reveal::Folder(folder.clone()),
            Destination::File(file) => Reveal::File(file.clone()),
        })
    }
}

impl Row {
    fn view(&self, words: &Translator, home: Option<&Path>) -> Job {
        let fraction = self.progress.fraction().unwrap_or(0.0) as f32;
        let mut job = Job::new(self.id.get(), self.phase, job_message(&self.title))
            .kind(self.kind)
            .fraction(fraction)
            .cancellable(self.cancellable && self.phase.active())
            .retry(self.offers.iter().any(|o| matches!(o, Offer::Retry { .. })));
        job.detail = self.detail.as_ref().map(job_message);
        job.destination = self.destination.as_ref().and_then(|destination| {
            let folder = match destination {
                Destination::Folder(folder) => folder.as_path(),
                Destination::File(file) => file.parent()?,
            };
            Some(crate::export::shown(folder, home))
        });
        job.outcome = self.outcome.as_ref().map(job_message);
        job.failure = self
            .failure
            .as_ref()
            .map(|failure| failure_words(words, failure));
        job.ended_at = self.ended_at;
        job
    }
}

/// A job's message as the window's: the same id and arguments, a path as its file name.
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

/// A failure as the window words it: the item and file by their names, the cause in
/// Fluent, else the reason the job gave.
fn failure_words(words: &Translator, failure: &Failure) -> JobFailure {
    let name = |path: &Option<PathBuf>| path.as_deref().map(file_name).unwrap_or_default();
    JobFailure {
        item: name(&failure.item),
        file: name(&failure.file),
        cause: cause_words(words, &failure.cause, &failure.reason),
    }
}

/// A cause's words, `job-cause-<code>`, without a full stop; the job's own `reason` for a
/// code this build has no words for.
fn cause_words(words: &Translator, code: &str, reason: &str) -> String {
    let id = format!("job-cause-{code}");
    let text = words.text(&id);
    if text != id {
        return text;
    }
    reason.trim_end_matches('.').to_owned()
}

/// `path`'s last component, `IMG_2047.ARW`.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
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
        // Try again is not drawn until the session takes `RetryJob` (decision 72's step 6):
        // the rows keep the offer, but the button would ask nothing.
        let jobs: Vec<Job> = self
            .jobs
            .iter()
            .cloned()
            .map(|job| job.retry(false))
            .collect();
        quiet_scroll(ui, area, |ui| {
            Jobs::new(&jobs).file_manager(FILE_MANAGER).window(ui)
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
