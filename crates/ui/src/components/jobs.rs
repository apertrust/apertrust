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

//! The jobs: the spinner at the top panel's end and the content of the jobs window
//! (`docs/gui.md`, The jobs window; decision 57, which replaces the export activity's chip
//! and popover).
//!
//! - **The spinner** ([`Jobs::spinner`]) is one icon button beside Saved while there is any
//!   job to show, never one per job and with no words, so the top panel never folds it.
//!   While a job is queued or running it turns; with only ended jobs left it shows the
//!   tick, or the warning sign where one failed. It is named "Jobs", with the count in its
//!   value ("2 running"), so its name stays the same as jobs come and go; a click or Enter
//!   asks for the jobs window.
//! - **The window's content** ([`Jobs::window`]) is a function of a `Ui`, which the app's
//!   dialog host draws in a window of its own: one row per job, newest first, each with its
//!   message, a bar, its details and the buttons its state allows. An export's row keeps
//!   the activity's details ("3 of 12 done · now IMG_2044.ARW", the folder, a failure's
//!   photo, file and cause) and buttons (Show in Finder, Cancel export, Try again,
//!   Dismiss); a queued job says so; a job that cannot be cancelled has no Cancel.
//! - **Plain data in, actions out.** The caller passes the jobs as the app mirrors them from
//!   the session's events and applies the [`ActivityAction`] it gets back; the component
//!   knows no engine type, and keeps only what it last announced, in egui's memory. The
//!   caller names the platform's file manager.
//! - **Screen readers** find the spinner in the Tab order, and a polite live region says
//!   when a job starts and when it ends, never as it progresses. The region is in the tree
//!   whenever the top panel has jobs, empty while there are none.
//! - **Reduced motion:** egui has no setting for it; the theme's motion tokens set its
//!   `animation_time` to zero then, and the spinner holds still when it is zero.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::time::Duration;

use egui::accesskit::Live;
use egui::epaint::text::LayoutJob;
use egui::{
    Align, Color32, FontFamily, FontId, Frame, Id, Margin, Pos2, Rect, Response, Role, Sense,
    Shape, Stroke, TextStyle, Ui, UiBuilder, Vec2, WidgetInfo, vec2,
};

use super::{focus_ring, icon_colour, icon_lit};
use crate::i18n::{Args, Translator};
use crate::{Icon, fonts, theme, translator};

/// A job, as the session names it.
pub type JobId = u64;

/// Where a job is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobPhase {
    /// Behind other jobs, started when they allow.
    Queued,
    Running,
    Finished,
    /// Ended with an error, or an export with at least one photo not written.
    Failed,
    Cancelled,
}

impl JobPhase {
    /// The word its messages carry (`jobs-live-<word>`, `activity-name-<word>`); a waiting
    /// job has none of its own.
    fn word(self) -> &'static str {
        match self {
            JobPhase::Queued | JobPhase::Running => "running",
            JobPhase::Finished => "finished",
            JobPhase::Failed => "failed",
            JobPhase::Cancelled => "cancelled",
        }
    }

    /// Whether the job is queued or running.
    pub fn active(self) -> bool {
        matches!(self, JobPhase::Queued | JobPhase::Running)
    }
}

/// A value in a job's message.
#[derive(Clone, Debug, PartialEq)]
pub enum JobArg {
    Number(f64),
    Text(String),
}

/// A job's message as the session sends it: an id (`job-…`) and named arguments, which the
/// component translates with Fluent.
#[derive(Clone, Debug, PartialEq)]
pub struct JobMessage {
    pub id: String,
    pub args: Vec<(String, JobArg)>,
}

impl JobMessage {
    pub fn new(id: impl Into<String>) -> Self {
        JobMessage {
            id: id.into(),
            args: Vec::new(),
        }
    }

    pub fn number(mut self, name: &str, value: f64) -> Self {
        self.args.push((name.to_owned(), JobArg::Number(value)));
        self
    }

    pub fn text(mut self, name: &str, value: impl Into<String>) -> Self {
        self.args
            .push((name.to_owned(), JobArg::Text(value.into())));
        self
    }

    fn translate(&self, words: &Translator) -> String {
        let mut args = Args::new();
        for (name, value) in &self.args {
            match value {
                JobArg::Number(n) => args.set(name.as_str(), *n),
                JobArg::Text(t) => args.set(name.as_str(), t.as_str()),
            }
        }
        words.text_with(&self.id, Some(&args))
    }
}

/// Why a photo was not written, with the words the app made of the engine's error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportFailure {
    /// The photo, by its file name: `IMG_2047.ARW`. Empty for a failure of the whole
    /// export, such as a path that is not valid.
    pub photo: String,
    /// The file that could not be written: `IMG_2047.jpg`. Empty where none was named, a
    /// raw that cannot be read.
    pub file: String,
    /// The reason, translated and without a full stop: "the disk is full".
    pub cause: String,
}

/// What an export's row adds to any job's: the photos, the folder and the failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportDetails {
    /// The photos asked for.
    pub total: usize,
    /// The photos written.
    pub done: usize,
    /// The photos not written, once the job has ended.
    pub failed: usize,
    /// The photo being exported now, while it runs.
    pub current: Option<String>,
    /// Where the files go, as the app wants it read: `~/Pictures/apertrust_exported`.
    pub folder: String,
    /// The failure to explain, on a job that failed.
    pub failure: Option<ExportFailure>,
}

impl ExportDetails {
    /// An export with nothing written yet.
    pub fn new(total: usize, folder: impl Into<String>) -> Self {
        ExportDetails {
            total,
            done: 0,
            failed: 0,
            current: None,
            folder: folder.into(),
            failure: None,
        }
    }

    fn args(&self) -> Args<'_> {
        let mut args = Args::new();
        args.set("done", self.done);
        args.set("total", self.total);
        args.set("failed", self.failed);
        args.set("folder", self.folder.as_str());
        args.set("file", self.current.as_deref().unwrap_or_default());
        args
    }
}

/// One job, as the window shows it: any job the session lists, an export among them.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub id: JobId,
    pub phase: JobPhase,
    pub message: JobMessage,
    /// How much is done, from 0 to 1.
    pub fraction: f32,
    /// Whether the job can be stopped; only a queued or running one shows Cancel.
    pub cancellable: bool,
    /// Why a job failed, translated and without a full stop, for a job that is not an
    /// export.
    pub error: Option<String>,
    /// Set for an export, whose row has the activity's details and buttons.
    pub export: Option<ExportDetails>,
}

impl Job {
    pub fn new(id: JobId, phase: JobPhase, message: JobMessage) -> Self {
        Job {
            id,
            phase,
            message,
            fraction: 0.0,
            cancellable: false,
            error: None,
            export: None,
        }
    }

    pub fn fraction(mut self, fraction: f32) -> Self {
        self.fraction = fraction;
        self
    }

    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn export(mut self, export: ExportDetails) -> Self {
        self.export = Some(export);
        self
    }
}

/// The platform's name for the app that shows a folder, for the button's words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileManager {
    Finder,
    Explorer,
    /// Linux's, and anything else.
    #[default]
    Files,
}

/// What the user asked of a job. The app does it: reveals the folder through the
/// platform, asks the session to stop or to run the failed photos again, and forgets a
/// job that is dismissed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityAction {
    ShowFolder(JobId),
    Cancel(JobId),
    TryAgain(JobId),
    Dismiss(JobId),
}

/// The jobs: the spinner and the window's content.
#[derive(Clone, Copy)]
pub struct Jobs<'a> {
    jobs: &'a [Job],
    file_manager: FileManager,
}

/// The spinner's circle.
const RADIUS: f32 = 7.0;
const STROKE: f32 = 1.5;
/// How far round the circle the turning arc reaches, in radians, and how fast it turns
/// (turns a second).
const SWEEP: f32 = 1.6;
const TURNS: f64 = 0.9;
/// The spinner's side, as the top panel's other buttons.
const BUTTON: f32 = 30.0;

/// The spinner's id, of the `ui` it is drawn in, so a gallery can show several.
fn spinner_id(ui: &Ui) -> Id {
    ui.scope_id().with("jobs spinner")
}

fn live_id(ui: &Ui) -> Id {
    ui.scope_id().with("jobs live region")
}

/// What the live region last heard of, kept in egui's memory.
#[derive(Clone, Default)]
struct Heard {
    phases: HashMap<JobId, JobPhase>,
    said: String,
}

impl<'a> Jobs<'a> {
    pub fn new(jobs: &'a [Job]) -> Self {
        Jobs {
            jobs,
            file_manager: FileManager::default(),
        }
    }

    /// Names the file manager on an export's button that shows the folder.
    pub fn file_manager(mut self, file_manager: FileManager) -> Self {
        self.file_manager = file_manager;
        self
    }

    /// The jobs, the newest first: ids only grow.
    fn newest_first(&self) -> Vec<&'a Job> {
        let mut jobs: Vec<&Job> = self.jobs.iter().collect();
        jobs.sort_by_key(|job| std::cmp::Reverse(job.id));
        jobs
    }

    fn count(&self, phase: JobPhase) -> usize {
        self.jobs.iter().filter(|j| j.phase == phase).count()
    }

    /// The spinner's width, for the top panel to place it by: zero when there is no job
    /// to show. Constant, whatever the jobs are doing, so nothing moves as they change.
    pub(crate) fn spinner_width(&self) -> f32 {
        if self.jobs.is_empty() { 0.0 } else { BUTTON }
    }

    /// The spinner's value, for screen readers and its tooltip, in the current language:
    /// "2 running", "1 waiting", "Finished", "1 failed".
    pub fn spinner_value(&self, ctx: &egui::Context) -> String {
        let (running, waiting, failed) = (
            self.count(JobPhase::Running),
            self.count(JobPhase::Queued),
            self.count(JobPhase::Failed),
        );
        let finished = self.count(JobPhase::Finished);
        let id = match (running, waiting) {
            (0, 0) if failed > 0 => "jobs-value-failed",
            (0, 0) if finished > 0 => "jobs-value-finished",
            (0, 0) => "jobs-value-cancelled",
            (0, _) => "jobs-value-waiting",
            (_, 0) => "jobs-value-running",
            _ => "jobs-value-running-waiting",
        };
        let mut args = Args::new();
        args.set("running", running);
        args.set("waiting", waiting);
        args.set("failed", failed);
        translator(ctx).text_with(id, Some(&args))
    }

    /// Draws the spinner at the layout's next place, and keeps the live region in the tree.
    /// Returns its response, for the caller's own bookkeeping (the top panel keeps it out
    /// of the title bar's empty areas), which is `clicked` when the user asks for the
    /// window; None when there is no job to show, though the live region is drawn all the
    /// same, empty.
    pub fn spinner(&self, ui: &mut Ui) -> Option<Response> {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let live = self.announce(ui, &words);
        if self.jobs.is_empty() {
            live_region(ui, ui.next_widget_position(), &live);
            return None;
        }
        let t = theme(&ctx);
        let (rect, _) = ui.allocate_exact_size(vec2(BUTTON, BUTTON), Sense::hover());
        let response = ui.interact(rect, spinner_id(ui), Sense::click());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if response.hovered() {
                painter.rect_filled(rect, t.shape.radius, t.colors.hover);
            }
            if self.jobs.iter().any(|j| j.phase.active()) {
                let turning = ui.style().animation_time > 0.0;
                let time = turning.then(|| ui.input(|i| i.time));
                if turning {
                    ctx.request_repaint_after(Duration::from_millis(16));
                }
                paint_spinner(painter, rect.center(), time, &t);
            } else {
                let (icon, colour) = if self.count(JobPhase::Failed) > 0 {
                    (Icon::Warning, t.colors.warning)
                } else {
                    (
                        Icon::Check,
                        icon_colour(&t, true, icon_lit(&ctx, &response)),
                    )
                };
                let size = icon.size();
                icon.paint(
                    painter,
                    Rect::from_center_size(rect.center(), Vec2::splat(size)),
                    colour,
                );
            }
            if response.has_focus() {
                focus_ring(painter, rect, &t);
            }
        }
        let name = words.text("jobs-name");
        let value = self.spinner_value(&ctx);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
        ctx.accesskit_node_builder(response.id, |node| node.set_value(value.as_str()));
        let mut args = Args::new();
        args.set("value", value.as_str());
        let response = response.on_hover_text(words.text_with("jobs-tip", Some(&args)));
        live_region(ui, rect.left_top(), &live);
        Some(response)
    }

    /// What the live region says now: the words for the last job to start or to end, kept
    /// until another does, so a screen reader hears each once and nothing as a job
    /// progresses.
    fn announce(&self, ui: &Ui, words: &Translator) -> String {
        let id = live_id(ui);
        let mut heard: Heard = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        let mut now = HashMap::new();
        let mut said = Vec::new();
        for job in self.newest_first().into_iter().rev() {
            let before = heard.phases.get(&job.id).copied();
            now.insert(job.id, job.phase);
            // A job that starts again under an id it ended with, as a retry that reuses
            // it, is heard to start like any other.
            let event = match (before, job.phase.active()) {
                (None, true) => Some(started(words, job)),
                (Some(JobPhase::Finished | JobPhase::Failed | JobPhase::Cancelled), true) => {
                    Some(started(words, job))
                }
                (None, false) | (Some(JobPhase::Queued | JobPhase::Running), false) => {
                    Some(ended(words, job))
                }
                _ => None,
            };
            said.extend(event);
        }
        heard.phases = now;
        if self.jobs.is_empty() {
            heard.said.clear();
        } else if !said.is_empty() {
            heard.said = said.join(" ");
        }
        let text = heard.said.clone();
        ui.data_mut(|d| d.insert_temp(id, heard));
        text
    }

    /// The window's content, drawn into `ui`: one row per job, the newest first. Returns
    /// what the user pressed.
    pub fn window(&self, ui: &mut Ui) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        let words = translator(ui.ctx());
        let mut action = None;
        ui.spacing_mut().item_spacing = vec2(t.spacing.m, t.spacing.m);
        ui.scope_builder(
            UiBuilder::new()
                .id_salt("jobs window")
                .accessibility_role(Role::Group)
                .accessibility_label(words.text("jobs-name")),
            |ui| {
                if self.jobs.is_empty() {
                    line(
                        ui,
                        &words.text("jobs-empty"),
                        t.text.body,
                        t.colors.text_secondary,
                        false,
                    );
                }
                for job in self.newest_first() {
                    ui.push_id(job.id, |ui| {
                        if let Some(asked) = self.row(ui, job, &words) {
                            action = Some(asked);
                        }
                    });
                }
            },
        );
        action
    }

    /// One job's row: a group named by its message.
    fn row(&self, ui: &mut Ui, job: &Job, words: &Translator) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        let title = job.message.translate(words);
        let mut action = None;
        Frame::new()
            .fill(t.colors.raised)
            .stroke(Stroke::new(t.shape.hairline, t.colors.border))
            .corner_radius(t.shape.radius)
            .inner_margin(Margin::same(t.spacing.l as i8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = vec2(t.spacing.m, t.spacing.m);
                ui.scope_builder(
                    UiBuilder::new()
                        .id_salt("job row")
                        .accessibility_role(Role::Group)
                        .accessibility_label(title.clone()),
                    |ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new(&title)
                                .font(FontId::new(
                                    t.text.body,
                                    FontFamily::Name(fonts::MEDIUM.into()),
                                ))
                                .color(t.colors.text),
                        ));
                        if job.phase == JobPhase::Running {
                            progress_bar(ui, job, &title);
                        }
                        details(ui, job, words);
                        action = self.buttons(ui, job, words);
                    },
                );
            });
        action
    }

    /// The buttons the job's state allows, at the row's end, in the order they are heard.
    fn buttons(&self, ui: &mut Ui, job: &Job, words: &Translator) -> Option<ActivityAction> {
        let export = job.export.is_some();
        let show = export.then(|| {
            (
                words.text(match self.file_manager {
                    FileManager::Finder => "activity-show-finder",
                    FileManager::Explorer => "activity-show-explorer",
                    FileManager::Files => "activity-show-files",
                }),
                Some(words.text("activity-show-tip")),
                ActivityAction::ShowFolder(job.id),
            )
        });
        let cancel = job.cancellable.then(|| {
            let (label, tip) = if export {
                ("activity-cancel", "activity-cancel-tip")
            } else {
                ("jobs-cancel", "jobs-cancel-tip")
            };
            (
                words.text(label),
                Some(words.text(tip)),
                ActivityAction::Cancel(job.id),
            )
        });
        let again = export.then(|| {
            (
                words.text("activity-try-again"),
                Some(words.text("activity-try-again-tip")),
                ActivityAction::TryAgain(job.id),
            )
        });
        let dismiss = (
            words.text("activity-dismiss"),
            None,
            ActivityAction::Dismiss(job.id),
        );
        let buttons: Vec<_> = match job.phase {
            JobPhase::Running => vec![show, cancel],
            JobPhase::Queued => vec![cancel],
            JobPhase::Finished | JobPhase::Cancelled => vec![show, Some(dismiss)],
            JobPhase::Failed => vec![again, show, Some(dismiss)],
        }
        .into_iter()
        .flatten()
        .collect();
        if buttons.is_empty() {
            return None;
        }
        // At the row's end when they fit on one line, from its start when they wrap.
        let style = TextStyle::Button.resolve(ui.style());
        let padding = 2.0 * ui.spacing().button_padding.x;
        let gap = ui.spacing().item_spacing.x;
        let wide: f32 = buttons
            .iter()
            .map(|(label, ..)| {
                let width = ui
                    .painter()
                    .layout_no_wrap(label.clone(), style.clone(), Color32::WHITE)
                    .size()
                    .x;
                (width + padding).max(ui.spacing().interact_size.x)
            })
            .sum::<f32>()
            + gap * (buttons.len() as f32 - 1.0);
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            if wide <= ui.available_width() {
                ui.add_space(ui.available_width() - wide);
            }
            for (label, tip, done) in buttons {
                let mut response = ui.add(egui::Button::new(label));
                if let Some(tip) = tip {
                    response = response.on_hover_text(tip);
                }
                if response.clicked() {
                    action = Some(done);
                }
            }
        });
        action
    }
}

/// The words for a job that has just started, for the live region.
fn started(words: &Translator, job: &Job) -> String {
    match &job.export {
        Some(export) => words.text_with("activity-title-running", Some(&export.args())),
        None => {
            let mut args = Args::new();
            args.set("message", job.message.translate(words));
            words.text_with("jobs-live-started", Some(&args))
        }
    }
}

/// The words for a job that has just ended, for the live region.
fn ended(words: &Translator, job: &Job) -> String {
    match &job.export {
        Some(export) => words.text_with(
            &format!("activity-name-{}", job.phase.word()),
            Some(&export.args()),
        ),
        None => {
            let mut args = Args::new();
            args.set("message", job.message.translate(words));
            words.text_with(&format!("jobs-live-{}", job.phase.word()), Some(&args))
        }
    }
}

/// A row's lines under its bar: what the job is doing or how it ended, and for an export
/// the folder.
fn details(ui: &mut Ui, job: &Job, words: &Translator) {
    let t = theme(ui.ctx());
    let small = t.text.small;
    let Some(export) = &job.export else {
        let outcome = match job.phase {
            JobPhase::Queued => Some(("jobs-queued", t.colors.text_secondary)),
            JobPhase::Running => None,
            JobPhase::Finished => Some(("jobs-finished", t.colors.text)),
            JobPhase::Failed => Some(("jobs-failed", t.colors.text)),
            JobPhase::Cancelled => Some(("jobs-cancelled", t.colors.text)),
        };
        if let Some((id, colour)) = outcome {
            line(ui, &words.text(id), small, colour, false);
        }
        if let Some(error) = &job.error {
            line(ui, &format!("{error}."), small, t.colors.error, false);
        }
        return;
    };
    let args = export.args();
    match job.phase {
        JobPhase::Queued => {
            let mut args = Args::new();
            args.set("count", export.total);
            args.set("folder", export.folder.as_str());
            let queued = words.text_with("activity-queued", Some(&args));
            line(ui, &queued, small, t.colors.text_secondary, true);
            return;
        }
        JobPhase::Running => {
            let id = match (&export.current, export.total) {
                (Some(_), _) => Some("activity-now"),
                (None, total) if total > 1 => Some("activity-between"),
                _ => None,
            };
            if let Some(id) = id {
                let now = words.text_with(id, Some(&args));
                line(ui, &now, small, t.colors.text, false);
            }
        }
        JobPhase::Finished | JobPhase::Failed | JobPhase::Cancelled => {
            let id = format!("activity-title-{}", job.phase.word());
            line(
                ui,
                &words.text_with(&id, Some(&args)),
                small,
                t.colors.text,
                false,
            );
            if let Some(failure) = &export.failure {
                let mut args = Args::new();
                args.set("photo", failure.photo.as_str());
                args.set("file", failure.file.as_str());
                args.set("cause", failure.cause.as_str());
                let id = match (failure.photo.is_empty(), failure.file.is_empty()) {
                    (false, false) => "activity-failure",
                    (false, true) => "activity-failure-photo",
                    (true, _) => "activity-failure-cause",
                };
                let reason = words.text_with(id, Some(&args));
                line(ui, &reason, small, t.colors.error, false);
            }
            if job.phase == JobPhase::Cancelled {
                let detail = words.text_with("activity-cancelled-detail", Some(&args));
                line(ui, &detail, small, t.colors.text, false);
            }
        }
    }
    let folder = words.text_with("activity-folder", Some(&args));
    line(ui, &folder, small, t.colors.text_secondary, true);
}

/// A line of text, wrapped at the row's width, `anywhere` for a path with no spaces, which
/// would run past it.
fn line(ui: &mut Ui, text: &str, size: f32, colour: Color32, anywhere: bool) {
    let mut job = LayoutJob::simple(
        text.to_owned(),
        FontId::proportional(size),
        colour,
        ui.available_width(),
    );
    job.wrap.break_anywhere = anywhere;
    job.halign = Align::LEFT;
    ui.add(egui::Label::new(job));
}

/// The row's bar: the track and the part done, to screen readers a progress indicator
/// named by the row's message, with the percentage done.
fn progress_bar(ui: &mut Ui, job: &Job, title: &str) {
    let t = theme(ui.ctx());
    let fraction = job.fraction.clamp(0.0, 1.0);
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), t.shape.track), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, t.shape.track / 2.0, t.colors.slider_track);
    let fill = rect.with_max_x(rect.left() + rect.width() * fraction);
    painter.rect_filled(fill, t.shape.track / 2.0, t.colors.text);
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::ProgressIndicator, true, title);
        info.value = Some((f64::from(fraction) * 100.0).floor());
        info
    });
}

/// The spinner's ring with an arc on it turning at `time` seconds, or still where there is
/// none.
fn paint_spinner(painter: &egui::Painter, centre: Pos2, time: Option<f64>, t: &crate::Theme) {
    painter.circle_stroke(centre, RADIUS, Stroke::new(STROKE, t.colors.border));
    let start = -FRAC_PI_2 + time.map_or(0.0, |s| (s * TURNS).fract() as f32 * TAU);
    let points: Vec<Pos2> = (0..=12)
        .map(|i| {
            let a = start + SWEEP * i as f32 / 12.0;
            centre + RADIUS * vec2(a.cos(), a.sin())
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(STROKE, t.colors.text)));
}

/// A polite live region at `at`, saying `message`: a status that takes no room. Its words
/// change only when a job starts or ends.
fn live_region(ui: &mut Ui, at: Pos2, message: &str) {
    let response = ui.interact(
        Rect::from_min_size(at, Vec2::ZERO),
        live_id(ui),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::Status, true, message));
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_live(Live::Polite);
    });
}
