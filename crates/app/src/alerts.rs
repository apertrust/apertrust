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

//! The app's confirmations (`docs/gui.md`, Dialogs and alerts; decision 41): the system's
//! alerts, asked through the dialog host and answered on a later frame. Five so far:
//!
//! - **Export's overwrite question** ([`Purpose::Overwrite`]): Export asks before it
//!   replaces files, as darktable does, from the Export dialog's own sheet.
//! - **The quit alert** ([`Purpose::Quit`]): Quit or Close Window while a job the user
//!   started runs, or after the edit's last write failed. The decision is [`Guard`], a
//!   small machine the tests drive: it waits for a write of the edit under way, asks once,
//!   lets the app leave when nothing runs and the edit is saved, and after a yes waits for
//!   the jobs that cannot be stopped.
//! - **Why the edit is not saved** ([`Purpose::Save`]): the top panel's save warning
//!   pressed, with Try Again.
//! - **Why a drop opens nothing** ([`Purpose::Refused`]): what was dropped onto the window
//!   is neither a folder nor a raw, or is a library's or a project's file, or an edit whose
//!   photo is not beside it.
//! - **Discard History** ([`Purpose::Discard`]): Edit › Discard History… and the History
//!   section's ⋯ ask before the edit's history is cleared, as darktable does
//!   (`ask_before_discard`).
//!
//! One alert is up at a time ([`Alerts`]); the words are Fluent messages handed over as
//! text.

use std::path::Path;

use apertrust_ui::alert::{Request, Role};
use apertrust_ui::i18n::Args;
use apertrust_ui::{tr, translator};
use eframe::egui;

use apertrust_engine::jobs::JobId;
use apertrust_engine::sidecar;

use crate::dialog_host::{Answer, AppDialogs, Ask};
use crate::dropped::Refusal;
use crate::jobs::{Mirror, Running};

/// What an alert is asked for, which says what its answer does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    /// Export would replace files already there: yes sends the export.
    Overwrite,
    /// A job the user started runs, or the edit's last write failed: yes stops the jobs
    /// that can be stopped and quits once the others have finished.
    Quit,
    /// The edit's last write failed: yes writes it again.
    Save,
    /// A drop opened nothing: the alert only says why.
    Refused,
    /// Discard History was asked for: yes sends it.
    Discard,
}

/// The open photo's edit as Quit finds it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Edit<'a> {
    /// On disk, or nothing to save.
    #[default]
    Saved,
    /// A write the session was handed is not yet confirmed: Quit waits for it.
    Writing,
    /// The last write failed, for this reason: Quit asks.
    Failed(&'a str),
}

/// The alert asked and not yet answered, if one.
#[derive(Debug, Default)]
pub struct Alerts {
    pending: Option<Purpose>,
}

impl Alerts {
    /// What the alert up is for, if one is.
    pub fn pending(&self) -> Option<Purpose> {
        self.pending
    }

    /// Asks `request` for `purpose` through `dialogs`, over the window of the main
    /// context `ctx`, and says what came of it: [`Ask::Busy`] while another alert is up,
    /// nothing asked then; [`Ask::Failed`] for a request that cannot be shown. Only
    /// [`Ask::Shown`] leaves an alert up.
    pub fn ask(
        &mut self,
        dialogs: &mut AppDialogs,
        purpose: Purpose,
        request: &Request,
        ctx: &egui::Context,
    ) -> Ask {
        if self.pending.is_some() {
            return Ask::Busy;
        }
        if let Err(why) = request.check() {
            tracing::error!(target: "gpu", "alert not shown: {why:?}");
            return Ask::Failed;
        }
        let asked = dialogs.ask(request, ctx);
        if asked == Ask::Shown {
            self.pending = Some(purpose);
        }
        asked
    }

    /// The alert's answer, with what it was asked for, once it has gone.
    pub fn answered(&mut self, dialogs: &mut AppDialogs) -> Option<(Purpose, Answer)> {
        let answer = dialogs.answer()?;
        Some((self.pending.take()?, answer))
    }
}

/// "Export in overwrite mode?": Overwrite, which destroys and has no key, or Cancel, the
/// default, which Return and Escape both press, and Don't ask again. The answers are
/// Overwrite's index 0 (the yes) and Cancel's 1. With the setting `held` by `--conf`
/// ([`Question::held`]) there is no box, since ticking it would do nothing.
pub fn overwrite_request(ctx: &egui::Context, held: bool) -> Request {
    let request = Request::new(
        tr(ctx, "alert-overwrite-title"),
        tr(ctx, "alert-overwrite-message"),
    )
    .button(tr(ctx, "alert-overwrite-button"), Role::Destructive)
    .button(tr(ctx, "alert-cancel"), Role::Cancel)
    .default_button(1);
    if held {
        request
    } else {
        request.suppression(tr(ctx, "alert-overwrite-suppress"))
    }
}

/// Whether a question with Don't ask again is asked: Export's and Export Again's overwrite
/// question, darktable's "ask before exporting in overwrite mode"
/// (`export.ask_before_overwrite`), and Discard History's, darktable's "ask before
/// discarding history" (`history.ask_before_discard`), each on at first. A yes with Don't
/// ask again ticked turns it off, for good, as it is kept in the settings, unless `--conf`
/// holds the setting for the run: then the store would refuse the write, so the run goes
/// on asking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Question {
    purpose: Purpose,
    ask: bool,
    held: bool,
}

impl Question {
    /// The overwrite question as the settings have it, `held` being whether `--conf` sets
    /// it for this run.
    pub fn overwrite(ask: bool, held: bool) -> Question {
        Question {
            purpose: Purpose::Overwrite,
            ask,
            held,
        }
    }

    /// Discard History's question, as [`Question::overwrite`].
    pub fn discard(ask: bool, held: bool) -> Question {
        Question {
            purpose: Purpose::Discard,
            ask,
            held,
        }
    }

    /// Whether `--conf` sets the setting for this run, so that ticking the box would do
    /// nothing and the alert shows none.
    pub fn held(self) -> bool {
        self.held
    }

    /// Whether a question the alert would ask is asked.
    pub fn asks(self) -> bool {
        self.ask
    }

    /// The answer to an alert asked for `purpose`: a yes to this question with its box
    /// ticked stops the asking. A no with the box ticked changes nothing: the person did
    /// not go ahead. Another question's answer changes nothing.
    pub fn answered(&mut self, purpose: Purpose, answer: Answer) {
        if purpose == self.purpose && answer.confirmed() && answer.ticked() && !self.held {
            self.ask = false;
        }
    }
}

/// The photo Discard History… acts on: the number of its open, and whether it is open for
/// viewing only because part of its edit's history cannot be read, which asks in words of
/// its own and starts the history afresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Discarding {
    pub open: u64,
    pub unreadable: bool,
}

/// What Discard History… does now ([`discard`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Discard {
    /// Ask first, answered on a later frame.
    Ask(Discarding),
    /// Send it at once: the question is off.
    Send(Discarding),
    /// Nothing: no photo is open, or the question would be asked where the platform shows
    /// no alert, which leaves the items disabled ([`discard_runs`]).
    Nothing,
}

/// Whether Discard History asks first on a photo: while `question` is on, and always on
/// an `unreadable` edit, since nothing takes back starting it afresh.
pub fn discard_asks(question: Question, unreadable: bool) -> bool {
    unreadable || question.asks()
}

/// Whether Discard History… can run where the history gives it something to do: where the
/// platform shows alerts (`alerts`), and where it shows none, Windows' until its task
/// dialog is built, only where nothing would be asked. Its items are drawn disabled
/// otherwise, saying why, as Export's On conflict › Overwrite is.
pub fn discard_runs(question: Question, unreadable: bool, alerts: bool) -> bool {
    alerts || !discard_asks(question, unreadable)
}

/// What Discard History… does on `photo`, the one open, if any, with `question` as the
/// settings have it and `alerts` whether the platform shows alerts.
pub fn discard(photo: Option<Discarding>, question: Question, alerts: bool) -> Discard {
    match photo {
        None => Discard::Nothing,
        Some(photo) if !discard_asks(question, photo.unreadable) => Discard::Send(photo),
        Some(photo) if alerts => Discard::Ask(photo),
        Some(_) => Discard::Nothing,
    }
}

/// What a yes to the question asked for `asked` does now that `photo` is open: send it to
/// the same photo, opened the same way; ask again, in the other words, where that photo's
/// history has become unreadable or readable since; and drop it where another photo, or
/// none, is open.
pub fn discard_confirmed(asked: Option<Discarding>, photo: Option<Discarding>) -> Discard {
    match (asked, photo) {
        (Some(asked), Some(photo)) if asked == photo => Discard::Send(photo),
        (Some(asked), Some(photo)) if asked.open == photo.open => Discard::Ask(photo),
        _ => Discard::Nothing,
    }
}

/// "Discard this photo’s history?": Discard History, which destroys and has no key, or
/// Cancel, the default, which Return and Escape both press. The answers are Discard
/// History's index 0 (the yes) and Cancel's 1. Undo takes it back while the photo is open,
/// as the message says, and Don't ask again turns the question off, unless `held` by
/// `--conf`. For an edit open for viewing only because part of its history cannot be read,
/// `unreadable`, the message says that what could not be read goes with the old history
/// and that nothing takes it back, so a copy of the edit's file, `file`, is to be kept
/// first; it always asks there, so there is no box.
pub fn discard_request(ctx: &egui::Context, held: bool, unreadable: bool, file: &str) -> Request {
    let message = if unreadable {
        let mut args = Args::new();
        args.set("file", file);
        translator(ctx).text_with("alert-discard-unreadable-message", Some(&args))
    } else {
        tr(ctx, "alert-discard-message")
    };
    let request = Request::new(tr(ctx, "alert-discard-title"), message)
        .button(tr(ctx, "alert-discard-button"), Role::Destructive)
        .button(tr(ctx, "alert-cancel"), Role::Cancel)
        .default_button(1);
    if held || unreadable {
        request
    } else {
        request.suppression(tr(ctx, "alert-overwrite-suppress"))
    }
}

/// "Quit while a job is running?": its message says what a yes does to the `running` jobs.
/// Where the edit's last write failed, "Quit without your last changes?", saying why, then
/// what a yes does to any jobs running, with Quit Anyway and Cancel, the default.
pub fn quit_request(ctx: &egui::Context, running: Running, edit: Edit<'_>) -> Request {
    let words = translator(ctx);
    let mut args = Args::new();
    args.set("stoppable", running.stoppable as f64);
    args.set("waiting", running.waiting as f64);
    let jobs = words.text_with("alert-quit-message", Some(&args));
    let Edit::Failed(why) = edit else {
        return Request::new(tr(ctx, "alert-quit-title"), jobs)
            .button(tr(ctx, "alert-quit-button"), Role::Destructive)
            .button(tr(ctx, "alert-cancel"), Role::Cancel);
    };
    let mut args = Args::new();
    args.set("error", why);
    let mut message = words.text_with("alert-quit-save-message", Some(&args));
    if !running.none() {
        message = format!("{message}\n\n{jobs}");
    }
    Request::new(tr(ctx, "alert-quit-save-title"), message)
        .button(tr(ctx, "alert-quit-anyway"), Role::Destructive)
        .button(tr(ctx, "alert-cancel"), Role::Cancel)
        .default_button(1)
}

/// "The edit isn't saved": why the write of the edit of the photo named `name` failed,
/// with Try Again, the default, and Cancel.
pub fn save_request(ctx: &egui::Context, name: &str, why: &str) -> Request {
    let mut args = Args::new();
    args.set("name", name);
    args.set("error", why);
    let message = translator(ctx).text_with("alert-save-message", Some(&args));
    Request::new(tr(ctx, "alert-save-title"), message)
        .button(tr(ctx, "alert-save-button"), Role::Default)
        .button(tr(ctx, "alert-cancel"), Role::Cancel)
}

/// "Can't open notes.txt": a drop that opens nothing, the first thing dropped named `name`,
/// saying why, with one button, which Return and Escape both press. An edit's names its
/// photo, `name` without `.apxmp`.
pub fn refused_request(ctx: &egui::Context, name: &str, why: Refusal) -> Request {
    let mut args = Args::new();
    args.set("name", name);
    let title = translator(ctx).text_with("alert-refused-title", Some(&args));
    let message = match why {
        Refusal::Library => tr(ctx, "alert-refused-library"),
        Refusal::NoPhoto => {
            let photo = sidecar::photo_for(Path::new(name));
            let photo = photo.as_deref().and_then(Path::to_str).unwrap_or(name);
            args.set("photo", photo);
            translator(ctx).text_with("alert-refused-no-photo", Some(&args))
        }
        Refusal::Unknown => tr(ctx, "alert-refused-unknown"),
    };
    Request::new(title, message).button(tr(ctx, "alert-refused-button"), Role::Cancel)
}

/// One Quit or Close Window deciding whether the app leaves: `run_action` asks
/// [`Leave::may`] once no dialog is open, and the app asks the alert afterwards if
/// [`Leave::ask`] says so.
pub struct Leave<'a> {
    guard: &'a mut Guard,
    running: Running,
    edit: Edit<'a>,
    /// The quit alert is to be asked.
    pub ask: bool,
}

impl<'a> Leave<'a> {
    pub fn new(guard: &'a mut Guard, running: Running, edit: Edit<'a>) -> Leave<'a> {
        Leave {
            guard,
            running,
            edit,
            ask: false,
        }
    }

    /// Whether the app leaves now ([`Guard::attempt`]).
    pub fn may(&mut self) -> bool {
        match self.guard.attempt(self.running, self.edit) {
            Attempt::Leave => true,
            Attempt::Hold => false,
            Attempt::Ask => {
                self.ask = true;
                false
            }
        }
    }
}

/// Asks the quit alert for the `running` jobs and the `edit`. Returns whether to quit again
/// at once:
/// - where the platform shows no alert, quitting asks nothing from now on, as it did before
///   the alerts;
/// - where showing it failed, this one Quit leaves, and the next asks again;
/// - while another alert is up, this Quit does nothing.
pub fn ask_quit(
    alerts: &mut Alerts,
    dialogs: &mut AppDialogs,
    guard: &mut Guard,
    running: Running,
    edit: Edit<'_>,
    ctx: &egui::Context,
) -> bool {
    let request = quit_request(ctx, running, edit);
    match alerts.ask(dialogs, Purpose::Quit, &request, ctx) {
        Ask::Shown => false,
        Ask::Busy => {
            guard.answered(false);
            false
        }
        Ask::NoAlerts => {
            guard.skip();
            true
        }
        Ask::Failed => {
            guard.leave_once();
            true
        }
    }
}

/// What the app does once an alert for `purpose` is answered.
#[derive(Debug, PartialEq, Eq)]
pub enum Then {
    /// Nothing: the question was answered no, or the system took the alert away.
    Nothing,
    /// Send the export, and close the dialog it was pressed in.
    Export,
    /// Stop these jobs, and go on quitting: the app leaves once none runs.
    Stop(Vec<JobId>),
    /// Write the edit again.
    RetrySave,
    /// Send Discard History.
    Discard,
    /// Quit again at once, to leave: the quit alert could not be shown, so this one Quit
    /// leaves and the next asks again, as when asking it fails ([`ask_quit`]).
    Leave,
    /// Nothing: the alert only said something, so no question waits on its answer.
    Told,
}

/// The decision an answer makes: only a yes (the first button, the default) does anything.
/// A yes to the quit alert is told to `guard`, which then waits for the jobs to end; any
/// other answer lets the next Quit ask again. A quit alert that could not be shown after
/// all ([`Answer::Failed`]) lets this Quit leave, as one that fails as it is asked does.
pub fn then(purpose: Purpose, answer: Answer, guard: &mut Guard, jobs: &Mirror) -> Then {
    let confirmed = answer.confirmed();
    match purpose {
        Purpose::Quit if answer == Answer::Failed => {
            guard.leave_once();
            Then::Leave
        }
        Purpose::Overwrite if confirmed => Then::Export,
        Purpose::Overwrite => Then::Nothing,
        Purpose::Quit => {
            guard.answered(confirmed);
            if confirmed {
                Then::Stop(jobs.stoppable())
            } else {
                Then::Nothing
            }
        }
        Purpose::Save if confirmed => Then::RetrySave,
        Purpose::Save => Then::Nothing,
        Purpose::Discard if confirmed => Then::Discard,
        Purpose::Discard => Then::Nothing,
        Purpose::Refused => Then::Told,
    }
}

/// Where quitting is: not asked, asking, or asked and said yes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    /// Quit waits for a write of the edit under way, to know whether it failed.
    Writing,
    /// The alert is up.
    Asking,
    /// The person said yes: the jobs that can be stopped are cancelled, and the app leaves
    /// once none runs.
    Confirmed,
    /// The platform shows no alert: quitting asks nothing, as it did before the alerts.
    Skipped,
    /// The alert could not be shown this time: the next attempt leaves, then it is Idle.
    LeaveOnce,
}

/// What Quit or Close Window does now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attempt {
    /// The app leaves.
    Leave,
    /// The alert is to be asked; the app stays meanwhile.
    Ask,
    /// The app stays: the alert is up, or the jobs have not ended.
    Hold,
}

/// The quit alert's decisions: when quitting asks, and what each answer does.
#[derive(Debug, Default)]
pub struct Guard {
    phase: Phase,
}

impl Guard {
    /// Quit, or Close Window on the main window, once no dialog is open: while a write of
    /// the edit is under way it waits for it; then with nothing running and the edit saved
    /// the app leaves; with a job running or the last write failed it asks, once; after a
    /// yes it waits for the jobs to end; while the alert is up it waits for the answer.
    pub fn attempt(&mut self, running: Running, edit: Edit<'_>) -> Attempt {
        match self.phase {
            Phase::Skipped => Attempt::Leave,
            Phase::LeaveOnce => {
                self.phase = Phase::Idle;
                Attempt::Leave
            }
            Phase::Asking => Attempt::Hold,
            Phase::Confirmed if running.none() => Attempt::Leave,
            Phase::Confirmed => Attempt::Hold,
            Phase::Idle | Phase::Writing if edit == Edit::Writing => {
                self.phase = Phase::Writing;
                Attempt::Hold
            }
            Phase::Idle | Phase::Writing if running.none() && edit == Edit::Saved => {
                self.phase = Phase::Idle;
                Attempt::Leave
            }
            Phase::Idle | Phase::Writing => {
                self.phase = Phase::Asking;
                Attempt::Ask
            }
        }
    }

    /// Whether Quit waits for a write of the edit: once it is confirmed or has failed,
    /// Quit is to run again.
    pub fn waits_for_write(&self) -> bool {
        self.phase == Phase::Writing
    }

    /// The alert's answer: yes keeps quitting, waiting for the jobs; anything else drops
    /// it, and the next Quit asks again.
    pub fn answered(&mut self, confirmed: bool) {
        self.phase = if confirmed {
            Phase::Confirmed
        } else {
            Phase::Idle
        };
    }

    /// The alert could not be shown: quitting asks nothing from now on.
    pub fn skip(&mut self) {
        self.phase = Phase::Skipped;
    }

    /// The alert could not be shown: the next attempt leaves, and the one after asks again.
    pub fn leave_once(&mut self) {
        self.phase = Phase::LeaveOnce;
    }

    /// Whether the person said yes and the app only waits for the jobs: when they have
    /// ended it is to leave without being asked again.
    pub fn confirmed(&self) -> bool {
        self.phase == Phase::Confirmed
    }

    /// Whether the alert is up or has been answered yes, so a quit is in progress: a job
    /// started now would not be among those the alert's yes counted.
    pub fn quitting(&self) -> bool {
        matches!(self.phase, Phase::Asking | Phase::Confirmed)
    }

    /// Whether the main window's close button is held, for Quit to decide: while a job
    /// runs or the edit is not on disk, unless the platform shows no alert.
    pub fn holds_close(&self, running: Running, edit: Edit<'_>) -> bool {
        !matches!(self.phase, Phase::Skipped | Phase::LeaveOnce)
            && (!running.none() || edit != Edit::Saved)
    }
}

#[cfg(test)]
mod tests {
    use apertrust_ui::{Locale, Theme};

    use super::*;

    const NONE: Running = Running {
        stoppable: 0,
        waiting: 0,
    };
    const EXPORT: Running = Running {
        stoppable: 1,
        waiting: 0,
    };
    const STUCK: Running = Running {
        stoppable: 0,
        waiting: 1,
    };
    const SAVED: Edit<'static> = Edit::Saved;

    #[test]
    fn quitting_with_nothing_running_asks_nothing() {
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(NONE, SAVED), Attempt::Leave);
        assert!(!guard.holds_close(NONE, SAVED));
    }

    #[test]
    fn quitting_with_a_job_running_asks_once_and_waits_for_the_answer() {
        let mut guard = Guard::default();
        assert!(guard.holds_close(EXPORT, SAVED));
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
        // Quit again, or the window's close, while the alert is up: no second alert.
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Hold);
        // Even if the job ends meanwhile, the alert is the person's to answer.
        assert_eq!(guard.attempt(NONE, SAVED), Attempt::Hold);
    }

    #[test]
    fn cancelling_the_alert_stays_and_the_next_quit_asks_again() {
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
        guard.answered(false);
        assert!(!guard.confirmed());
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
    }

    #[test]
    fn confirming_waits_for_the_jobs_that_cannot_stop_and_then_leaves() {
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
        guard.answered(true);
        assert!(guard.confirmed());
        // The export was cancelled and has not ended yet; a job that cannot stop runs.
        assert_eq!(guard.attempt(STUCK, SAVED), Attempt::Hold);
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Hold);
        assert_eq!(guard.attempt(NONE, SAVED), Attempt::Leave);
    }

    #[test]
    fn when_the_alert_cannot_be_shown_one_quit_leaves_and_the_next_asks_again() {
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
        guard.leave_once();
        assert!(!guard.holds_close(EXPORT, SAVED));
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Leave);
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
    }

    #[test]
    fn where_the_platform_shows_no_alert_quitting_leaves_as_before() {
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Ask);
        guard.skip();
        assert_eq!(guard.attempt(EXPORT, SAVED), Attempt::Leave);
        assert!(!guard.holds_close(EXPORT, SAVED));
    }

    fn context(locale: Locale) -> egui::Context {
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        apertrust_ui::set_locale(&ctx, locale);
        ctx
    }

    #[test]
    fn the_overwrite_question_is_a_destructive_yes_and_a_default_cancel_in_the_language() {
        let request = overwrite_request(&context(Locale::EnGb), false);
        assert_eq!(request.title, "Export in overwrite mode?");
        assert_eq!(request.check(), Ok(()));
        let buttons: Vec<_> = request
            .buttons
            .iter()
            .map(|button| (button.label.as_str(), button.role))
            .collect();
        assert_eq!(
            buttons,
            [("Overwrite", Role::Destructive), ("Cancel", Role::Cancel)]
        );
        // Cancel is the default, which Return presses, and Escape's as well; Overwrite is
        // reached by a click or by Tab alone.
        assert_eq!((request.default, request.cancel()), (1, Some(1)));
        // And carries Don't ask again, which the quit alert does not, and which the alert
        // leaves out while `--conf` holds the setting.
        assert_eq!(
            overwrite_request(&context(Locale::EnGb), true).suppression,
            None
        );
        let held = overwrite_request(&context(Locale::EnGb), true);
        assert_eq!((held.buttons.len(), held.default), (2, 1));
        assert_eq!(request.suppression.as_deref(), Some("Don’t ask again"));
        let quit = quit_request(&context(Locale::EnGb), EXPORT, SAVED);
        assert_eq!(quit.suppression, None);
        let suppression = |locale| overwrite_request(&context(locale), false).suppression;
        assert_eq!(suppression(Locale::Uk).as_deref(), Some("Більше не питати"));
        assert_eq!(
            suppression(Locale::Pl).as_deref(),
            Some("Nie pytaj ponownie")
        );
        let ukrainian = overwrite_request(&context(Locale::Uk), false);
        assert_eq!(ukrainian.buttons[0].label, "Перезаписати");
        assert_eq!(ukrainian.buttons[1].label, "Скасувати");
        let polish = overwrite_request(&context(Locale::Pl), false);
        assert_eq!(polish.buttons[1].label, "Anuluj");
    }

    #[test]
    fn only_a_yes_with_the_box_ticked_stops_the_overwrite_question() {
        let after = |purpose, answer| {
            let mut overwrite = Question::overwrite(true, false);
            overwrite.answered(purpose, answer);
            overwrite.asks()
        };
        assert!(!after(Purpose::Overwrite, Answer::Ticked(0)));
        // Not ticked, a no, taken away, or another alert's: the question stays.
        assert!(after(Purpose::Overwrite, Answer::Pressed(0)));
        assert!(after(Purpose::Overwrite, Answer::Ticked(1)));
        assert!(after(Purpose::Overwrite, Answer::Dismissed));
        assert!(after(Purpose::Quit, Answer::Ticked(0)));
        assert!(after(Purpose::Discard, Answer::Ticked(0)));
        // `--conf` holds it: it stays whatever the answer.
        let mut held = Question::overwrite(true, true);
        held.answered(Purpose::Overwrite, Answer::Ticked(0));
        assert!(held.asks());
        // Already off, it stays off.
        let mut off = Question::overwrite(false, false);
        off.answered(Purpose::Overwrite, Answer::Pressed(0));
        assert!(!off.asks());
        // Discard History's question, by its own answers alone.
        let mut discard = Question::discard(true, false);
        discard.answered(Purpose::Overwrite, Answer::Ticked(0));
        assert!(discard.asks());
        discard.answered(Purpose::Discard, Answer::Ticked(1));
        assert!(discard.asks());
        discard.answered(Purpose::Discard, Answer::Ticked(0));
        assert!(!discard.asks());
    }

    /// What Discard History… does: asked, sent at once with the question off, or nothing,
    /// by the photo, the setting and whether the platform shows alerts; and a yes sent only
    /// to the photo it was asked for, opened the same way.
    #[test]
    fn discard_history_asks_sends_or_does_nothing_by_the_photo_the_setting_and_the_platform() {
        let readable = Discarding {
            open: 3,
            unreadable: false,
        };
        let unreadable = Discarding {
            unreadable: true,
            ..readable
        };
        let (on, off) = (
            Question::discard(true, false),
            Question::discard(false, false),
        );
        assert_eq!(discard(Some(readable), on, true), Discard::Ask(readable));
        assert_eq!(discard(Some(readable), off, true), Discard::Send(readable));
        // An unreadable edit always asks.
        assert_eq!(
            discard(Some(unreadable), off, true),
            Discard::Ask(unreadable)
        );
        // No alert on the platform: only what asks nothing runs.
        assert_eq!(discard(Some(readable), on, false), Discard::Nothing);
        assert_eq!(discard(Some(readable), off, false), Discard::Send(readable));
        assert_eq!(discard(Some(unreadable), off, false), Discard::Nothing);
        assert_eq!(discard(None, off, true), Discard::Nothing);
        assert!(discard_runs(on, false, true) && discard_runs(off, false, false));
        assert!(!discard_runs(on, false, false) && !discard_runs(off, true, false));
        // The answer: the same photo opened the same way, sent; its history read otherwise
        // since, asked again in those words; another photo, or none, dropped.
        let yes = |asked, now| discard_confirmed(Some(asked), now);
        assert_eq!(yes(readable, Some(readable)), Discard::Send(readable));
        assert_eq!(yes(unreadable, Some(readable)), Discard::Ask(readable));
        assert_eq!(yes(readable, Some(unreadable)), Discard::Ask(unreadable));
        let other = Discarding {
            open: 4,
            ..readable
        };
        assert_eq!(yes(readable, Some(other)), Discard::Nothing);
        assert_eq!(yes(readable, None), Discard::Nothing);
        assert_eq!(discard_confirmed(None, Some(readable)), Discard::Nothing);
    }

    #[test]
    fn the_discard_question_is_a_destructive_yes_and_a_default_cancel_in_every_language() {
        let file = "IMG_2041.ARW.apxmp";
        let request = discard_request(&context(Locale::EnGb), false, false, file);
        assert_eq!(request.title, "Discard this photo’s history?");
        assert_eq!(
            request.message,
            "The photo goes back to how it first opened, and its history is cleared. Undo \
             brings it back until the photo closes."
        );
        assert_eq!(request.check(), Ok(()));
        let buttons: Vec<_> = (request.buttons.iter())
            .map(|button| (button.label.as_str(), button.role))
            .collect();
        assert_eq!(
            buttons,
            [
                ("Discard History", Role::Destructive),
                ("Cancel", Role::Cancel)
            ]
        );
        assert_eq!((request.default, request.cancel()), (1, Some(1)));
        assert_eq!(request.suppression.as_deref(), Some("Don’t ask again"));
        // Held by `--conf`: no box.
        let held = discard_request(&context(Locale::EnGb), true, false, file);
        assert_eq!(held.suppression, None);
        // An edit part of whose history cannot be read: what goes, and the file to copy
        // first; it always asks, so no box.
        let unreadable = discard_request(&context(Locale::EnGb), false, true, file);
        assert_eq!(unreadable.suppression, None);
        assert!(unreadable.message.contains(file), "{}", unreadable.message);
        assert_eq!(unreadable.check(), Ok(()));
        for locale in Locale::ALL {
            let words = apertrust_ui::Translator::new(locale);
            let request = discard_request(&context(locale), false, true, "a.apxmp");
            assert_eq!(request.title, words.text("alert-discard-title"));
            assert_eq!(request.buttons[0].label, words.text("alert-discard-button"));
            assert!(request.message.contains("a.apxmp"), "{locale:?}");
        }
        let (mirror, _) = exporting();
        let mut guard = Guard::default();
        let then = |answer, guard: &mut Guard| then(Purpose::Discard, answer, guard, &mirror);
        assert_eq!(then(Answer::Pressed(0), &mut guard), Then::Discard);
        assert_eq!(then(Answer::Ticked(0), &mut guard), Then::Discard);
        assert_eq!(then(Answer::Pressed(1), &mut guard), Then::Nothing);
        assert_eq!(then(Answer::Dismissed, &mut guard), Then::Nothing);
        assert_eq!(then(Answer::Failed, &mut guard), Then::Nothing);
        assert!(!guard.confirmed());
    }

    /// A mirror with an export of the user's running, and its job.
    fn exporting() -> (Mirror, JobId) {
        let job = crate::jobs::tests::ids(1)[0];
        let mut mirror = Mirror::new(None);
        let added = crate::jobs::tests::added(job, apertrust_engine::jobs::Queue::UserExport, None);
        mirror.apply(&added);
        (mirror, job)
    }

    /// Discard History's question as Linux draws it, and for an edit part of whose history
    /// cannot be read.
    #[test]
    fn the_discard_question_is_photographed() {
        use apertrust_ui::components::{AlertBody, ButtonOrder};
        let mut results = egui_kittest::SnapshotResults::new();
        for (name, unreadable) in [("alert-discard", false), ("alert-discard-unreadable", true)] {
            let ctx = context(Locale::EnGb);
            let request = discard_request(&ctx, false, unreadable, "IMG_2041.ARW.apxmp");
            let mut ticked = false;
            let mut harness = egui_kittest::Harness::builder()
                .with_size(egui::vec2(480.0, 240.0))
                .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
                .build_ui(move |ui| {
                    AlertBody::new(&request, &mut ticked)
                        .order(ButtonOrder::DefaultLast)
                        .show(ui);
                });
            harness.run();
            harness.snapshot(name);
            results.extend(harness.take_snapshot_results());
        }
    }

    #[test]
    fn only_a_yes_to_the_overwrite_question_exports() {
        let (mirror, _) = exporting();
        let mut guard = Guard::default();
        let then = |answer, guard: &mut Guard| then(Purpose::Overwrite, answer, guard, &mirror);
        assert_eq!(then(Answer::Pressed(0), &mut guard), Then::Export);
        assert_eq!(then(Answer::Pressed(1), &mut guard), Then::Nothing);
        assert_eq!(then(Answer::Dismissed, &mut guard), Then::Nothing);
        // The overwrite question says nothing of quitting.
        assert!(!guard.confirmed());
    }

    #[test]
    fn a_yes_to_the_quit_alert_stops_the_jobs_that_can_be_stopped_and_a_no_stops_none() {
        let (mirror, job) = exporting();
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(mirror.running(), SAVED), Attempt::Ask);
        let stop = then(Purpose::Quit, Answer::Pressed(0), &mut guard, &mirror);
        assert_eq!(stop, Then::Stop(vec![job]));
        assert!(guard.confirmed());

        let mut guard = Guard::default();
        assert_eq!(guard.attempt(mirror.running(), SAVED), Attempt::Ask);
        for answer in [Answer::Pressed(1), Answer::Dismissed] {
            let nothing = then(Purpose::Quit, answer, &mut guard, &mirror);
            assert_eq!(nothing, Then::Nothing);
            assert!(!guard.confirmed());
            // Back to asking: the next Quit asks again.
            assert_eq!(guard.attempt(mirror.running(), SAVED), Attempt::Ask);
        }
    }

    #[test]
    fn a_quit_alert_whose_window_could_not_be_made_lets_that_quit_leave_and_the_next_ask() {
        let (mirror, _) = exporting();
        let mut guard = Guard::default();
        assert_eq!(guard.attempt(mirror.running(), SAVED), Attempt::Ask);
        let leave = then(Purpose::Quit, Answer::Failed, &mut guard, &mirror);
        assert_eq!(leave, Then::Leave);
        assert!(!guard.confirmed());
        assert_eq!(guard.attempt(mirror.running(), SAVED), Attempt::Leave);
        assert_eq!(guard.attempt(mirror.running(), SAVED), Attempt::Ask);
        // Failed is no yes and ticks nothing: the other questions do nothing on it.
        assert!(!Answer::Failed.confirmed() && !Answer::Failed.ticked());
        for purpose in [Purpose::Overwrite, Purpose::Save, Purpose::Discard] {
            assert_eq!(
                then(purpose, Answer::Failed, &mut guard, &mirror),
                Then::Nothing
            );
        }
    }

    #[test]
    fn leaving_asks_through_the_alert_only_once_and_says_so() {
        let mut guard = Guard::default();
        let mut leave = Leave::new(&mut guard, EXPORT, SAVED);
        assert!(!leave.may());
        assert!(leave.ask);
        let mut leave = Leave::new(&mut guard, EXPORT, SAVED);
        assert!(!leave.may());
        assert!(!leave.ask, "the alert is up");
        let mut guard = Guard::default();
        let mut leave = Leave::new(&mut guard, NONE, SAVED);
        assert!(leave.may());
        assert!(!leave.ask);
    }

    #[test]
    fn the_quit_message_says_what_a_yes_does_to_the_jobs_running() {
        let ctx = context(Locale::EnGb);
        let message =
            |stoppable, waiting| quit_request(&ctx, Running { stoppable, waiting }, SAVED);
        let stops = message(2, 0);
        assert_eq!(stops.title, "Quit while a job is running?");
        assert!(
            stops.message.starts_with("The running jobs stop"),
            "{}",
            stops.message
        );
        assert_eq!(stops.check(), Ok(()));
        let mixed = message(1, 1);
        assert!(
            mixed.message.contains("once the others have finished"),
            "{}",
            mixed.message
        );
        let waits = message(0, 1);
        assert!(
            waits.message.starts_with("Apertrust quits once"),
            "{}",
            waits.message
        );
        assert_eq!(waits.buttons[0].label, "Quit");
        assert_eq!(waits.buttons[0].role, Role::Destructive);
        assert_eq!(waits.buttons[1].role, Role::Cancel);
    }

    #[test]
    fn quitting_waits_for_a_write_under_way_and_leaves_once_it_is_saved() {
        let mut guard = Guard::default();
        assert!(guard.holds_close(NONE, Edit::Writing));
        assert_eq!(guard.attempt(NONE, Edit::Writing), Attempt::Hold);
        assert!(guard.waits_for_write());
        assert_eq!(guard.attempt(NONE, Edit::Writing), Attempt::Hold);
        // Written: Quit runs again and leaves.
        assert_eq!(guard.attempt(NONE, SAVED), Attempt::Leave);
        assert!(!guard.waits_for_write());
    }

    #[test]
    fn quitting_after_a_failed_write_asks_and_a_yes_leaves() {
        let mut guard = Guard::default();
        let failed = Edit::Failed("disk full");
        assert!(guard.holds_close(NONE, failed));
        // The write under way fails: Quit asks.
        assert_eq!(guard.attempt(NONE, Edit::Writing), Attempt::Hold);
        assert_eq!(guard.attempt(NONE, failed), Attempt::Ask);
        assert_eq!(guard.attempt(NONE, failed), Attempt::Hold);
        // Cancel stays, and the next Quit asks again; Quit Anyway leaves.
        guard.answered(false);
        assert_eq!(guard.attempt(NONE, failed), Attempt::Ask);
        guard.answered(true);
        assert_eq!(guard.attempt(NONE, failed), Attempt::Leave);
    }

    #[test]
    fn the_quit_alert_says_why_the_last_changes_were_not_saved_cancel_the_default() {
        for locale in Locale::ALL {
            let ctx = context(locale);
            let words = apertrust_ui::Translator::new(locale);
            let request = quit_request(&ctx, NONE, Edit::Failed("disk full"));
            let mut args = Args::new();
            args.set("error", "disk full");
            let said = words.text_with("alert-quit-save-message", Some(&args));
            assert_eq!(request.title, words.text("alert-quit-save-title"));
            assert_eq!(request.message, said, "{locale:?}");
            assert_eq!(request.check(), Ok(()));
            let buttons: Vec<_> = request.buttons.iter().map(|b| b.role).collect();
            assert_eq!(buttons, [Role::Destructive, Role::Cancel]);
            assert_eq!(request.buttons[0].label, words.text("alert-quit-anyway"));
            // Cancel is the default, which Return presses, and Escape's too.
            assert_eq!((request.default, request.cancel()), (1, Some(1)));
            // With a job running too, what a yes does to it follows.
            let both = quit_request(&ctx, EXPORT, Edit::Failed("disk full"));
            assert!(both.message.starts_with(&said), "{locale:?}");
            assert!(both.message.len() > said.len() + 2, "{locale:?}");
        }
        let english = quit_request(&context(Locale::EnGb), NONE, Edit::Failed("disk full"));
        assert_eq!(english.title, "Quit without your last changes?");
        assert_eq!(
            english.message,
            "Your last changes couldn’t be saved: disk full"
        );
        assert_eq!(english.buttons[0].label, "Quit Anyway");
    }

    #[test]
    fn the_save_warning_s_alert_says_why_and_try_again_writes_again() {
        let request = save_request(&context(Locale::EnGb), "IMG_2041.ARW", "disk full");
        assert_eq!(request.title, "The edit isn’t saved");
        assert_eq!(
            request.message,
            "The edit of IMG_2041.ARW couldn’t be written beside it: disk full"
        );
        assert_eq!(request.check(), Ok(()));
        assert_eq!(request.buttons[0].label, "Try Again");
        assert_eq!((request.default, request.cancel()), (0, Some(1)));
        let (mirror, _) = exporting();
        let mut guard = Guard::default();
        let then = |answer, guard: &mut Guard| then(Purpose::Save, answer, guard, &mirror);
        assert_eq!(then(Answer::Pressed(0), &mut guard), Then::RetrySave);
        assert_eq!(then(Answer::Pressed(1), &mut guard), Then::Nothing);
        assert_eq!(then(Answer::Dismissed, &mut guard), Then::Nothing);
        assert!(!guard.confirmed());
    }

    #[test]
    fn a_refused_drop_s_alert_names_the_first_thing_dropped_and_says_why() {
        let ctx = context(Locale::EnGb);
        let request = refused_request(&ctx, "notes.txt", Refusal::Unknown);
        assert_eq!(request.title, "Can’t open notes.txt");
        assert_eq!(request.message, "Only folders and raw photos open here.");
        assert_eq!(request.check(), Ok(()));
        // One button, which Return and Escape both press.
        assert_eq!(request.buttons.len(), 1);
        assert_eq!(request.buttons[0].label, "OK");
        assert_eq!((request.default, request.cancel()), (0, Some(0)));
        let library = refused_request(&ctx, "Home.aplibrary", Refusal::Library);
        assert_eq!(
            library.message,
            "Libraries and projects come in a later version."
        );
        // An edit whose photo is not beside it names the photo.
        let edit = refused_request(&ctx, "IMG_2041.ARW.apxmp", Refusal::NoPhoto);
        assert_eq!(edit.title, "Can’t open IMG_2041.ARW.apxmp");
        assert_eq!(
            edit.message,
            "Its photo, IMG_2041.ARW, isn’t in the same folder."
        );
        assert_eq!(edit.check(), Ok(()));
        let (mirror, _) = exporting();
        let mut guard = Guard::default();
        let told = then(Purpose::Refused, Answer::Pressed(0), &mut guard, &mirror);
        assert_eq!(told, Then::Told);
    }
}
