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
//!   beside the main window, whose rows `Jobs::window` draws, scrolling past five. Closing
//!   it stops no job. Opened by itself, it closes itself about two seconds after the jobs
//!   it opened for have ended well, unless one failed or was cancelled, or the person
//!   touched it ([`Window::opened_for`]).
//! - **What its buttons do** is the app's: the ✕ sends `CancelJob`, Try again on a failed
//!   row that offers it sends `RetryJob`, its job's row taking the failed one's place,
//!   Dismiss and Clear Finished change the mirror, and Show in Finder reveals the job's
//!   destination through the platform ([`reveal`]), a file selected in its folder.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use apertrust_engine::jobs::{
    Argument, Destination, Failure, JobEvent, JobId, Message, Offer, Progress, Queue, State,
};
use apertrust_engine::session::{Command, Event};
use apertrust_ui::components::{
    ActivityAction, FileManager, Job, JobFailure, JobMessage, JobPhase, Jobs,
};
use apertrust_ui::{Theme, Translator, tr};
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
    /// The jobs that opened the window ([`Show::Open`]) since the app last took them
    /// ([`Mirror::opened`]), for it to wait for before it closes itself.
    opened: Vec<u64>,
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
    /// Send the session a command that starts a job of the user's, which quitting waits
    /// for until its job is added; the mirror changed with it.
    Start(Command),
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
            opened: Vec::new(),
            home,
        }
    }

    /// Notes a command the app sent, numbered `number`, that starts a job of the user's,
    /// such as an export: until its job is added it has no number to cancel, so quitting
    /// waits for it.
    pub fn starting(&mut self, number: u64) {
        self.starting.push(number);
    }

    /// The session refused the command numbered `number`: it starts no job, so quitting
    /// no longer waits for it.
    pub fn refused(&mut self, number: u64) {
        self.starting.retain(|starting| *starting != number);
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
                    self.opened.push(job.get());
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

    /// The jobs that have opened the window since this was last asked, for the window to
    /// wait for before it closes itself ([`Window::opened_for`]).
    pub fn opened(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.opened)
    }

    /// What a button of the window asks of the app: the ✕ is the session's `CancelJob`;
    /// Try again, on a failed row that offers it, the session's `RetryJob`, the offer taken
    /// from the row as it is sent, so it is asked once, and the row stays failed should the
    /// session refuse it; Show in Finder reveals the job's destination; Dismiss takes an
    /// ended row away, and Clear Finished every finished and cancelled one.
    pub fn act(&mut self, action: ActivityAction) -> Do {
        match action {
            ActivityAction::Cancel(job) => match self.rows.iter().find(|r| r.id.get() == job) {
                Some(row) => Do::Send(Command::CancelJob { job: row.id }),
                None => Do::Nothing,
            },
            ActivityAction::ShowFolder(job) => self.reveal(job).map_or(Do::Nothing, Do::Reveal),
            ActivityAction::TryAgain(job) => self.try_again(job),
            ActivityAction::Dismiss(job) => self.remove(|row| row.id.get() == job),
            ActivityAction::ClearFinished => self.remove(|row| row.phase.cleared()),
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

    /// Try again on the row of job `job`: its retry offer taken, and the session asked for
    /// the retry; nothing for a row that is not a failed one offering it.
    fn try_again(&mut self, job: u64) -> Do {
        let offers = |row: &Row| row.offers.iter().any(|o| matches!(o, Offer::Retry { .. }));
        let Some(row) = self
            .rows
            .iter_mut()
            .find(|row| row.id.get() == job && row.phase == JobPhase::Failed && offers(row))
        else {
            return Do::Nothing;
        };
        row.offers.retain(|o| !matches!(o, Offer::Retry { .. }));
        Do::Start(Command::RetryJob { job: row.id })
    }

    /// Takes away the ended rows `which` names: Dismiss's, Clear Finished's.
    fn remove(&mut self, which: impl Fn(&Row) -> bool) -> Do {
        let before = self.rows.len();
        self.rows.retain(|row| row.phase.active() || !which(row));
        if self.rows.len() == before {
            Do::Nothing
        } else {
            Do::Changed
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
        let mut job = Job::new(self.id.get(), self.phase, job_message(&self.title))
            .kind(self.kind)
            .progress(self.progress.done, self.progress.total)
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
/// if the event is a user job's, waiting for `opened`, the jobs that opened it, to close
/// itself, or comes forward; and, open, shows `jobs` as the mirror holds them now, in the
/// language of `ctx`.
pub fn refresh(
    jobs: &[Job],
    opened: Vec<u64>,
    dialogs: &mut AppDialogs,
    show: Show,
    place: Option<Position>,
    ctx: &egui::Context,
) {
    match show {
        Show::Open => {
            let window = Window::new(jobs.to_vec()).opened_for(opened);
            dialogs.open_jobs(window, place, ctx, false);
        }
        Show::Front => dialogs.open_jobs(Window::new(jobs.to_vec()), place, ctx, true),
        Show::Nothing | Show::Update => {}
    }
    dialogs.set_jobs(jobs.to_vec());
}

/// The window's width, in points, inside its margin.
pub const WIDTH: f32 = 440.0;
/// How long a window that opened itself waits, once the jobs it opened for have ended
/// well, before it closes itself: long enough for their outcome to be seen.
pub const CLOSE_AFTER: Duration = Duration::from_secs(2);

/// The jobs window's content: the rows as the mirror last gave them.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    jobs: Vec<Job>,
    /// What it waits for to close itself, where it opened itself; none where the person
    /// opened it or touched it, or a job it opened for failed or was cancelled.
    closing: Option<Closing>,
}

/// What a window that opened itself waits for before it closes itself.
#[derive(Clone, Debug, PartialEq)]
struct Closing {
    /// The jobs it opened for.
    jobs: Vec<u64>,
    /// When it saw them all ended well, by its clock, in seconds; none while one is queued
    /// or running or the pointer is over the window.
    since: Option<f64>,
}

/// Whether the window closes itself, as a frame of it asks.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Close {
    /// Not now; looked at again as it is next drawn.
    Stay,
    /// Once this long has gone by, unless anything changes first.
    After(Duration),
    Now,
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
            margin: t.spacing.m,
        }
    }

    fn keys(&self, _os: OperatingSystem) -> Keys {
        Keys::default()
    }

    fn show(&mut self, ui: &mut egui::Ui) -> Asks {
        let output = self.rows(ui).map(Output::Jobs);
        let ctx = ui.ctx().clone();
        let os = ctx.os();
        let (now, touched, over) = ctx.input(|i| (i.time, touched(i, os), i.pointer.has_pointer()));
        let close = match self.close(now, touched, over) {
            Close::Now => true,
            Close::After(left) => {
                ctx.request_repaint_after(left);
                false
            }
            Close::Stay => false,
        };
        Asks {
            output,
            close,
            ..Asks::default()
        }
    }

    fn placed(&self, at: Position) -> Option<Output> {
        Some(Output::JobsPlaced(at))
    }
}

/// Whether the person touched the window in this frame: pressed in it or typed in it, or,
/// on the Mac, where a window that opens itself does so without the focus, gave it the
/// focus, by a click, a move by its title bar or bringing it to the front. Elsewhere it
/// takes the focus as it opens, so having it says nothing.
fn touched(input: &egui::InputState, os: OperatingSystem) -> bool {
    let typed = input
        .events
        .iter()
        .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }));
    input.pointer.any_pressed() || typed || (os == OperatingSystem::Mac && input.focused)
}

impl Window {
    pub fn new(jobs: Vec<Job>) -> Window {
        Window {
            jobs,
            closing: None,
        }
    }

    /// The window as a job the person started opens it: it closes itself about
    /// [`CLOSE_AFTER`] after `opened`, the jobs it opened for, have all ended well, unless
    /// one failed or was cancelled, or the person touched it.
    pub fn opened_for(mut self, opened: Vec<u64>) -> Window {
        self.closing = Some(Closing {
            jobs: opened,
            since: None,
        });
        self
    }

    /// Asked to open again while it is open, or asked for and not yet made, as `asked`: a
    /// click on the spinner (`focus`) brings it to the front, a touch, so it stays open;
    /// another automatic open adds the jobs it opened for to those it waits for. A window
    /// the person opened stays open. Its rows are the newer.
    pub fn again(&mut self, asked: Window, focus: bool) {
        self.jobs = asked.jobs;
        if focus {
            self.closing = None;
        } else if let (Some(closing), Some(more)) = (&mut self.closing, asked.closing) {
            closing.jobs.extend(more.jobs);
            closing.since = None;
        }
    }

    /// Whether a window that opened itself closes now, at `now` seconds by its clock: once
    /// a job it opened for failed or was cancelled, or the person `touched` it, it stays
    /// open for good; while one is queued or running, or the pointer is `over` it, it
    /// waits; otherwise it closes [`CLOSE_AFTER`] after it first saw them all ended.
    fn close(&mut self, now: f64, touched: bool, over: bool) -> Close {
        let Some(closing) = &mut self.closing else {
            return Close::Stay;
        };
        let phases: Vec<JobPhase> = self
            .jobs
            .iter()
            .filter(|job| closing.jobs.contains(&job.id))
            .map(|job| job.phase)
            .collect();
        let ended_badly = phases
            .iter()
            .any(|phase| matches!(phase, JobPhase::Failed | JobPhase::Cancelled));
        if ended_badly || touched {
            self.closing = None;
            return Close::Stay;
        }
        if over || phases.iter().any(|phase| phase.active()) {
            closing.since = None;
            return Close::Stay;
        }
        let since = *closing.since.get_or_insert(now);
        let left = CLOSE_AFTER.as_secs_f64() - (now - since);
        if left > 0.0 {
            Close::After(Duration::from_secs_f64(left))
        } else {
            Close::Now
        }
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

    /// Draws the rows in `ui`, which scroll past five, and the footer. Returns what the
    /// user pressed.
    fn rows(&mut self, ui: &mut egui::Ui) -> Option<ActivityAction> {
        ui.set_width(WIDTH);
        Jobs::new(&self.jobs).file_manager(FILE_MANAGER).window(ui)
    }
}

#[cfg(test)]
pub(crate) mod tests;
