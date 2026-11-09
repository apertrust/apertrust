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
//! (`docs/gui.md`, The jobs window; decisions 57 and 72).
//!
//! - **The spinner** ([`Jobs::spinner`]) is one icon button beside the save state, never
//!   one per job and with no words, so the top panel never folds it. It turns while a job
//!   is queued or running; with none, it shows the warning sign while a failed job is
//!   left, and nothing otherwise: the finished rows wait in the window. It is named
//!   "Jobs", with the count in its value ("2 running"), so its name stays the same as jobs
//!   come and go; a click or Enter asks for the jobs window.
//! - **The window's content** ([`Jobs::window`]) is a function of a `Ui`, which the app's
//!   dialog host draws in a window of its own: one row per job, newest first, stacked on a
//!   sunken list, and a footer with Clear Finished. Every job's row is drawn alike from
//!   the parts it reported of itself (decision 72): its kind's icon on a tile (the warning
//!   sign once it failed), its title (once ended, its outcome), a bar while it is queued or
//!   running with the ✕ that cancels it, the detail under the bar, "To" its destination
//!   with the time it ended, or a failure's item, file and cause; then its buttons, Show in
//!   Finder where it has a destination, and Try again and Dismiss on a failed row. The list
//!   is as tall as its first five rows, and scrolls past them.
//! - **Plain data in, actions out.** The caller passes the jobs as the app mirrors them from
//!   the session's events and applies the [`ActivityAction`] it gets back; the component
//!   knows no engine type, and keeps only what it last announced and its rows' heights, in
//!   egui's memory. The caller names the platform's file manager.
//! - **Screen readers** find the spinner in the Tab order, and a polite live region says
//!   when a job starts and when it ends, never as it progresses. The region is in the tree
//!   whenever the top panel has jobs, empty while there are none.
//! - **Reduced motion:** egui has no setting for it; the theme's motion tokens set its
//!   `animation_time` to zero then, and the spinner and an indeterminate bar's segment hold
//!   still when it is zero.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::time::{Duration, SystemTime};

use egui::accesskit::Live;
use egui::epaint::text::LayoutJob;
use egui::{
    Align, FontFamily, FontId, Frame, Id, Layout, Margin, Pos2, Rect, Response, Role, ScrollArea,
    Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::{IconButton, focus_ring, quiet_scroll};
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
    /// Ended with a failure, cancelled or not.
    Failed,
    Cancelled,
}

impl JobPhase {
    /// The word an ended job's title carries without an outcome (`jobs-<word>`); a job
    /// that has not ended has none of its own.
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

    /// Whether it ended without a failure, finished or cancelled: what Clear Finished
    /// takes away.
    pub fn cleared(self) -> bool {
        matches!(self, JobPhase::Finished | JobPhase::Cancelled)
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

/// Why a job failed, with the words the app made of its cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobFailure {
    /// The item it failed on, by its file name: `IMG_2047.ARW`. Empty for a failure of the
    /// whole job, such as an export's path that is not valid.
    pub item: String,
    /// The file that could not be written: `IMG_2047.jpg`. Empty where none was named, a
    /// raw that cannot be read.
    pub file: String,
    /// The cause, translated and without a full stop: "the disk is full".
    pub cause: String,
}

/// One job's row, as the window shows it: the parts the job reported of itself, the same
/// for every kind of job.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub id: JobId,
    /// What it does, as the session names it: `export`, `folder`. Its row's icon is its
    /// kind's ([`kind_icon`]).
    pub kind: String,
    pub phase: JobPhase,
    /// Its title, darktable's message: "Exporting 3 / 12 to file on disk".
    pub title: JobMessage,
    /// How much is done, of [`Job::total`].
    pub done: u64,
    /// How much there is to do, where the job knows it: the bar fills at `done / total`,
    /// and with none it is indeterminate, a moving segment.
    pub total: Option<u64>,
    /// Whether the job can be stopped; only a queued or running one shows the ✕.
    pub cancellable: bool,
    /// The line under its bar while it is queued or running: "3 of 12 done · now
    /// IMG_2044.ARW".
    pub detail: Option<JobMessage>,
    /// Where its results go, as the app wants it read: `~/Pictures/apertrust_exported`.
    /// Show in Finder reveals it.
    pub destination: Option<String>,
    /// How it ended, once it has, its title then: "11 of 12 photos exported".
    pub outcome: Option<JobMessage>,
    /// Why it failed, on a job that failed.
    pub failure: Option<JobFailure>,
    /// Whether it offers to be tried again, once it has ended.
    pub retry: bool,
    /// When it ended, by the session's clock: the time after its destination once it
    /// finished or was cancelled.
    pub ended_at: Option<SystemTime>,
}

impl Job {
    pub fn new(id: JobId, phase: JobPhase, title: JobMessage) -> Self {
        Job {
            id,
            kind: String::new(),
            phase,
            title,
            done: 0,
            total: None,
            cancellable: false,
            detail: None,
            destination: None,
            outcome: None,
            failure: None,
            retry: false,
            ended_at: None,
        }
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }

    /// How much is done of how much, `total` none where the job does not know it.
    pub fn progress(mut self, done: u64, total: Option<u64>) -> Self {
        self.done = done;
        self.total = total;
        self
    }

    /// The part done, from 0 to 1, `done` clamped to `total`: none with no total, where
    /// the bar is indeterminate; an empty bar for a total of 0.
    pub fn fraction(&self) -> Option<f32> {
        let total = self.total?;
        Some(self.done.min(total) as f32 / total.max(1) as f32)
    }

    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    pub fn detail(mut self, detail: JobMessage) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn destination(mut self, destination: impl Into<String>) -> Self {
        self.destination = Some(destination.into());
        self
    }

    pub fn outcome(mut self, outcome: JobMessage) -> Self {
        self.outcome = Some(outcome);
        self
    }

    pub fn failure(mut self, failure: JobFailure) -> Self {
        self.failure = Some(failure);
        self
    }

    pub fn retry(mut self, retry: bool) -> Self {
        self.retry = retry;
        self
    }

    pub fn ended_at(mut self, at: SystemTime) -> Self {
        self.ended_at = Some(at);
        self
    }
}

/// Each kind of job's icon, as its row shows it on a tile at its start (decision 72). A
/// new kind takes a line here; until it has one it shows [`GENERIC`], so it shows with no
/// client code.
const KIND_ICONS: [(&str, Icon); 3] = [
    ("export", Icon::Export),
    ("import", Icon::Import),
    ("folder", Icon::Library),
];

/// The icon of a kind [`KIND_ICONS`] does not list.
const GENERIC: Icon = Icon::Settings;

/// The icon of a job of `kind`, by the design system's table, or a generic one for a kind
/// it does not list.
pub fn kind_icon(kind: &str) -> Icon {
    KIND_ICONS
        .iter()
        .find(|(listed, _)| *listed == kind)
        .map_or(GENERIC, |(_, icon)| *icon)
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

/// What the user asked of a job, or of the window's rows. The app does it: reveals the
/// destination through the platform, asks the session to stop the job or to try it again,
/// and forgets a failed job that is dismissed, or every finished and cancelled one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityAction {
    ShowFolder(JobId),
    Cancel(JobId),
    TryAgain(JobId),
    Dismiss(JobId),
    /// Clear Finished, in the footer: every finished and cancelled row, never a failed one.
    ClearFinished,
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
/// The tile a row's icon sits on, at its start.
const TILE: f32 = 28.0;
/// A row's bar: its height, fully rounded.
const BAR: f32 = 8.0;
/// An indeterminate bar's segment, as a part of the bar, and how fast it crosses it
/// (crossings a second).
const SEGMENT: f32 = 0.3;
const CROSSINGS: f64 = 0.6;
/// How many rows the list shows before it scrolls.
const ROWS: usize = 5;

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

    /// Names the file manager on the button that shows a job's destination.
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

    /// The spinner's value, for screen readers and its tooltip, in the current language:
    /// "2 running", "1 waiting", "1 running, 1 waiting", or with none of those "1 failed".
    pub fn spinner_value(&self, ctx: &egui::Context) -> String {
        let (running, waiting, failed) = (
            self.count(JobPhase::Running),
            self.count(JobPhase::Queued),
            self.count(JobPhase::Failed),
        );
        let id = match (running, waiting) {
            (0, 0) => "jobs-value-failed",
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
    /// window; None when nothing is queued or running and nothing failed, though the live
    /// region is drawn all the same.
    pub fn spinner(&self, ui: &mut Ui) -> Option<Response> {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let live = self.announce(ui, &words);
        let turning = self.jobs.iter().any(|j| j.phase.active());
        if !turning && self.count(JobPhase::Failed) == 0 {
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
            if turning {
                let turning = ui.style().animation_time > 0.0;
                let time = turning.then(|| ui.input(|i| i.time));
                if turning {
                    ctx.request_repaint_after(Duration::from_millis(16));
                }
                paint_spinner(painter, rect.center(), time, &t);
            } else {
                let icon = Icon::Warning;
                let size = icon.size();
                icon.paint(
                    painter,
                    Rect::from_center_size(rect.center(), Vec2::splat(size)),
                    t.colors.warning,
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
                    Some(row_title(words, job))
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

    /// The window's content, drawn into `ui`: the rows on their list, the newest first,
    /// then the footer, in view however the list scrolls. Returns what the user pressed.
    pub fn window(&self, ui: &mut Ui) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        let words = translator(ui.ctx());
        let mut action = None;
        ui.scope_builder(
            UiBuilder::new()
                .id_salt("jobs window")
                .accessibility_role(Role::Group)
                .accessibility_label(words.text("jobs-name")),
            |ui| {
                ui.spacing_mut().item_spacing = vec2(t.spacing.m, t.spacing.m);
                action = self.list(ui, &words);
                // Clear Finished, at the footer's end, while there is a row it takes.
                let any = self.jobs.iter().any(|job| job.phase.cleared());
                let footer = vec2(ui.available_width(), t.sizes.control);
                ui.allocate_ui_with_layout(footer, Layout::right_to_left(Align::Center), |ui| {
                    let clear = egui::Button::new(words.text("jobs-clear-finished"));
                    if ui.add_enabled(any, clear).clicked() {
                        action = Some(ActivityAction::ClearFinished);
                    }
                });
            },
        );
        action
    }

    /// The rows on a sunken list, parted by a faint line: as tall as they are up to
    /// [`ROWS`] of them, and with more, scrolling at the height of the first five, the
    /// heights measured on the frame before.
    fn list(&self, ui: &mut Ui, words: &Translator) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        let rows = self.newest_first();
        let heights_id = ui.scope_id().with("jobs row heights");
        let heights: HashMap<JobId, f32> = ui.data(|d| d.get_temp(heights_id)).unwrap_or_default();
        let shown = (rows.len() > ROWS).then(|| {
            rows[..ROWS]
                .iter()
                .map(|job| heights.get(&job.id).copied())
                .sum::<Option<f32>>()
        });
        let area = ScrollArea::vertical()
            .id_salt("jobs list")
            .auto_shrink([false, true])
            .max_height(shown.flatten().unwrap_or(f32::INFINITY));
        let mut action = None;
        let mut measured = HashMap::new();
        Frame::new()
            .fill(t.colors.sunken)
            .corner_radius(t.shape.radius_large)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                quiet_scroll(ui, area, |ui| {
                    ui.spacing_mut().item_spacing = vec2(t.spacing.m, 0.0);
                    if rows.is_empty() {
                        Frame::new()
                            .inner_margin(Margin::same(t.spacing.l as i8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let empty = words.text("jobs-empty");
                                let font = FontId::proportional(t.text.body);
                                line(ui, &empty, font, t.colors.text_secondary, false);
                            });
                    }
                    for (i, job) in rows.iter().enumerate() {
                        let drawn = ui.push_id(job.id, |ui| self.row(ui, job, words));
                        let rect = drawn.response.rect;
                        if i > 0 {
                            let line = Stroke::new(t.shape.hairline, t.colors.divider);
                            ui.painter().hline(rect.x_range(), rect.top(), line);
                        }
                        measured.insert(job.id, rect.height());
                        action = action.or(drawn.inner);
                    }
                });
            });
        // A row not measured yet leaves the list unbounded: drawn again at once, it is.
        if shown.is_some_and(|height| height.is_none()) {
            ui.ctx().request_discard("the jobs window's rows measured");
        }
        ui.data_mut(|d| d.insert_temp(heights_id, measured));
        action
    }

    /// One job's row: a group named by its title, its kind's tile at its start, then its
    /// title, its bar, its lines and its buttons.
    fn row(&self, ui: &mut Ui, job: &Job, words: &Translator) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        let title = row_title(words, job);
        let mut action = None;
        ui.scope_builder(
            UiBuilder::new()
                .id_salt("job row")
                .accessibility_role(Role::Group)
                .accessibility_label(title.clone()),
            |ui| {
                Frame::new()
                    .inner_margin(Margin::same(t.spacing.l as i8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            kind_tile(ui, job);
                            let width = ui.available_width();
                            ui.vertical(|ui| {
                                ui.set_width(width);
                                action = self.content(ui, job, &title, words);
                            });
                        });
                    });
            },
        );
        action
    }

    /// A row's content beside its tile: the title in Medium; while it is queued or running,
    /// the bar and the ✕; its lines; its buttons.
    fn content(
        &self,
        ui: &mut Ui,
        job: &Job,
        title: &str,
        words: &Translator,
    ) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        ui.spacing_mut().item_spacing.y = 0.0;
        let medium = FontId::new(t.text.body, FontFamily::Name(fonts::MEDIUM.into()));
        line(ui, title, medium, t.colors.text_strong, false);
        let mut action = None;
        if job.phase.active() {
            if bar_line(ui, job, title, words) {
                action = Some(ActivityAction::Cancel(job.id));
            }
        } else {
            ui.add_space(t.spacing.xxs);
        }
        details(ui, job, words);
        self.buttons(ui, job, words).or(action)
    }

    /// The buttons the job's state allows, under its lines, in the order they are heard:
    /// Show in Finder where it has a destination and is not waiting; on a failed row, Try
    /// again where it offers it before, and Dismiss after.
    fn buttons(&self, ui: &mut Ui, job: &Job, words: &Translator) -> Option<ActivityAction> {
        let t = theme(ui.ctx());
        let show = (job.destination.is_some() && job.phase != JobPhase::Queued).then(|| {
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
        let failed = job.phase == JobPhase::Failed;
        let again = (failed && job.retry).then(|| {
            (
                words.text("activity-try-again"),
                Some(words.text("activity-try-again-tip")),
                ActivityAction::TryAgain(job.id),
            )
        });
        let dismiss = failed.then(|| {
            (
                words.text("activity-dismiss"),
                None,
                ActivityAction::Dismiss(job.id),
            )
        });
        let buttons: Vec<_> = [again, show, dismiss].into_iter().flatten().collect();
        if buttons.is_empty() {
            return None;
        }
        ui.add_space(t.spacing.s);
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(t.spacing.m, t.spacing.xs);
            // At rest on the window's own surface, so they stand out of the sunken list;
            // under the pointer and pressed as every button is.
            let rest = &mut ui.visuals_mut().widgets.inactive;
            (rest.bg_fill, rest.weak_bg_fill) = (t.colors.raised, t.colors.raised);
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
    let mut args = Args::new();
    args.set("message", job.title.translate(words));
    words.text_with("jobs-live-started", Some(&args))
}

/// A row's title, which the live region also says as the job ends: its title while it is
/// queued or running; once ended its outcome, which says how it went, or without one its
/// title with the state's word ("Finished: …").
fn row_title(words: &Translator, job: &Job) -> String {
    if job.phase.active() {
        return job.title.translate(words);
    }
    if let Some(outcome) = &job.outcome {
        return outcome.translate(words);
    }
    let mut args = Args::new();
    args.set("message", job.title.translate(words));
    words.text_with(&format!("jobs-{}", job.phase.word()), Some(&args))
}

/// The tile at a row's start: its kind's icon, or once it failed the warning sign in
/// `warning`.
fn kind_tile(ui: &mut Ui, job: &Job) {
    let t = theme(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(TILE), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, t.shape.radius_large, t.colors.surface);
    let (icon, colour) = match job.phase {
        JobPhase::Failed => (Icon::Warning, t.colors.warning),
        _ => (kind_icon(&job.kind), t.colors.text),
    };
    let side = Vec2::splat(t.sizes.icon_large);
    icon.paint(painter, Rect::from_center_size(rect.center(), side), colour);
}

/// A row's lines under its bar, small: what the job is doing, or that it waits; then
/// where its results go, with the time it ended once it finished or was cancelled; or,
/// in their place, why it failed, in `error`.
fn details(ui: &mut Ui, job: &Job, words: &Translator) {
    let t = theme(ui.ctx());
    let small = FontId::proportional(t.text.small);
    let quiet = t.colors.text_secondary;
    match job.phase {
        JobPhase::Queued => {
            // The detail names the place, so no "To" line.
            match &job.detail {
                Some(detail) => line(ui, &detail.translate(words), small, quiet, true),
                None => line(ui, &words.text("jobs-queued"), small, quiet, false),
            }
            return;
        }
        JobPhase::Running => {
            if let Some(detail) = &job.detail {
                line(ui, &detail.translate(words), small.clone(), quiet, false);
            }
        }
        JobPhase::Failed => {
            if let Some(failure) = &job.failure {
                let mut args = Args::new();
                args.set("photo", failure.item.as_str());
                args.set("file", failure.file.as_str());
                args.set("cause", failure.cause.as_str());
                let id = match (failure.item.is_empty(), failure.file.is_empty()) {
                    (false, false) => "activity-failure",
                    (false, true) => "activity-failure-photo",
                    (true, _) => "activity-failure-cause",
                };
                let reason = words.text_with(id, Some(&args));
                line(ui, &reason, small, t.colors.error, false);
            }
            return;
        }
        JobPhase::Finished | JobPhase::Cancelled => {}
    }
    if let Some(destination) = &job.destination {
        let mut args = Args::new();
        args.set("folder", destination.as_str());
        let to = match job.ended_at.filter(|_| job.phase.cleared()) {
            Some(at) => {
                args.set("time", clock(at));
                words.text_with("activity-folder-at", Some(&args))
            }
            None => words.text_with("activity-folder", Some(&args)),
        };
        line(ui, &to, small, quiet, true);
    }
}

/// `at` on the local clock, hours and minutes: "14:02".
fn clock(at: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format("%H:%M")
        .to_string()
}

/// A line of text in `font`, wrapped at the row's width, `anywhere` for a path with no
/// spaces, which would run past it.
fn line(ui: &mut Ui, text: &str, font: FontId, colour: egui::Color32, anywhere: bool) {
    let mut job = LayoutJob::simple(text.to_owned(), font, colour, ui.available_width());
    job.wrap.break_anywhere = anywhere;
    job.halign = Align::LEFT;
    ui.add(egui::Label::new(job));
}

/// The bar of a queued or running job, a control high, with the ✕ that cancels it centred
/// on it at its end, a gap after it, the bar shortened for it, where the job can be
/// cancelled. Returns whether the ✕ was pressed.
fn bar_line(ui: &mut Ui, job: &Job, title: &str, words: &Translator) -> bool {
    let t = theme(ui.ctx());
    let side = t.sizes.control;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), side), Sense::hover());
    let end = if job.cancellable {
        rect.right() - side - t.spacing.m
    } else {
        rect.right()
    };
    let bar = Rect::from_min_max(
        pos2(rect.left(), rect.center().y - BAR / 2.0),
        pos2(end, rect.center().y + BAR / 2.0),
    );
    progress_bar(ui, bar, job, title);
    if !job.cancellable {
        return false;
    }
    let (name, tip) = (words.text("jobs-cancel"), words.text("jobs-cancel-tip"));
    let at = Rect::from_min_size(pos2(rect.right() - side, rect.top()), Vec2::splat(side));
    ui.scope_builder(UiBuilder::new().max_rect(at), |ui| {
        IconButton::new(Icon::Close, &name)
            .tip(&tip)
            .size(side)
            .show(ui)
            .clicked()
    })
    .inner
}

/// The bar in `rect`: `accent` on a track darker than the list, with a faint edge, filled
/// at the part done, empty while the job waits, and a moving segment where the job gives
/// no total. To screen readers a progress indicator named by the row's title, with the
/// percentage done where there is one.
fn progress_bar(ui: &mut Ui, rect: Rect, job: &Job, title: &str) {
    let t = theme(ui.ctx());
    let response = ui.interact(rect, ui.scope_id().with("job bar"), Sense::hover());
    let fraction = job.fraction();
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let round = BAR / 2.0;
        painter.rect_filled(rect, round, t.colors.separator);
        let part = match fraction {
            _ if job.phase == JobPhase::Queued => None,
            Some(fraction) => Some((rect.left(), rect.left() + rect.width() * fraction)),
            None => {
                let width = rect.width() * SEGMENT;
                let start = if ui.style().animation_time > 0.0 {
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                    let crossed = (ui.input(|i| i.time) * CROSSINGS).fract() as f32;
                    rect.left() - width + crossed * (rect.width() + width)
                } else {
                    rect.center().x - width / 2.0
                };
                Some((start.max(rect.left()), (start + width).min(rect.right())))
            }
        };
        if let Some((left, right)) = part.filter(|(left, right)| right > left) {
            let part = Rect::from_x_y_ranges(left..=right, rect.y_range());
            painter.rect_filled(part, round, t.colors.accent);
        }
        let edge = Stroke::new(t.shape.hairline, t.colors.divider);
        painter.rect_stroke(rect, round, edge, StrokeKind::Inside);
    }
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::ProgressIndicator, true, title);
        info.value = fraction.map(|f| (f64::from(f) * 100.0).floor());
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
