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

//! The log window (`docs/gui.md`, The log window): Help › Show Log, the lines of this
//! session's log that the app keeps in memory while Settings' Diagnostics says so
//! (`apertrust_base::log::Kept`), in a window of the dialog host, modeless as the jobs
//! window is, its place kept.
//!
//! - **Logging off**, it says no log is kept and offers Settings' Diagnostics: the menu
//!   item is never disabled without a reason.
//! - **Logging on**, it shows the lines as the terminal writes them, in IBM Plex Mono and
//!   in columns: the seconds right-aligned, the domain, the line; warnings in `warning`
//!   and errors in `error`, on a faint band of their colour. Its checkboxes filter what is
//!   shown among what is kept and change no setting; a domain not kept is drawn disabled,
//!   its tooltip saying where to turn it on. The lines are drawn row by row, only those in
//!   view; the view follows the end while it is there, and scrolled up it stays put, a pill
//!   offering the newer lines. Copy to clipboard and Save to file… take the lines shown as
//!   plain text, as the terminal writes them.
//! - It reads the lines kept since its last frame into a copy of its own, so it never holds
//!   the log while it draws, and a line logged meanwhile waits for nothing. A line kept
//!   wakes its window ([`Dialog::wake_with`]); nothing else draws it again.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::task::Waker;

use apertrust_base::log::{CAPACITY, Cursor, Domain, Kept, Line, Severity, Switches};
use apertrust_base::settings::Choice;
use apertrust_export::variables::DateTime;
use apertrust_ui::components::{Button, ToggleRow, quiet_scroll_rows};
use apertrust_ui::{Theme, fonts, theme, tr, translator};
use eframe::egui::os::OperatingSystem;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{
    self, Align, Align2, FontFamily, FontId, Layout, Rect, Role, ScrollArea, Sense, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use crate::dialog_host::{Asks, Dialog, Form, Keys, Look, Output, Resize};
use crate::settings::Position;

/// The window's size until the person resizes it, in points, as its design has it: its
/// filters on one line, 380 points of lines and the footer.
pub const SIZE: egui::Vec2 = egui::vec2(960.0, 466.0);
/// The least it is resized to: room for the filters on two lines, a few lines, and the
/// footer's buttons.
pub const LEAST: egui::Vec2 = egui::vec2(560.0, 320.0);
/// The width of the text with logging off, centred in the window.
const OFF_WIDTH: f32 = 416.0;
/// The fewest rows of lines shown, however small the window.
const FEWEST: f32 = 3.0;
/// A line's row.
const ROW: f32 = 20.0;
/// The seconds' column in characters of the lines' font, right-aligned, as wide as the
/// terminal writes them (`{:10.4}`); the domain's after it, as wide as the longest domain
/// in its brackets, `[settings]`.
const SECONDS: usize = 10;
const DOMAIN: usize = 10;
/// `log.domains`' options, each a filter: the domains, then `verbose`.
const OPTIONS: usize = <Domain as Choice>::OPTIONS.len();

/// What the window asked of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// Open Diagnostics settings…: Settings, on General.
    Settings,
    /// Save to file…: the lines shown, as the terminal writes them, for the save dialog.
    Save(String),
}

/// Which of the lines kept are shown: warnings and errors, and each of `log.domains`'
/// options, in their order, `verbose` last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Filters {
    warnings: bool,
    domains: [bool; OPTIONS],
}

/// The log window's content.
#[derive(Debug)]
pub struct Window {
    kept: Kept,
    cursor: Cursor,
    /// The lines read, as many as the log keeps at most.
    lines: VecDeque<Line>,
    /// The line saying how often the last one has repeated so far.
    pending: Option<Line>,
    /// How many of the session's oldest lines the log no longer keeps.
    dropped: u64,
    /// What is kept, as the log said this frame; None while nothing is.
    keeps: Option<Switches>,
    filters: Filters,
    /// The lines shown, by their place in `lines`, then `pending` where it is shown.
    shown: Vec<usize>,
    pending_shown: bool,
    /// Whether the view was at its end as the last frame ended, which it then follows.
    at_end: bool,
    /// The lines shown that came while the view was scrolled up, which the pill offers.
    newer: usize,
    /// The pill was pressed: the view goes to its end.
    to_end: bool,
    /// Whether the log wakes this window, which it stops doing as the window goes.
    woken: bool,
    /// The size it opens at, as the person last left it; None for [`SIZE`].
    size: Option<egui::Vec2>,
}

impl Window {
    /// The window over `kept`, every line kept shown, opening at `size`, where the person
    /// last left it, or at [`SIZE`].
    pub fn new(kept: Kept, size: Option<egui::Vec2>) -> Window {
        let mut window = Window {
            kept,
            cursor: Cursor::default(),
            lines: VecDeque::new(),
            pending: None,
            dropped: 0,
            keeps: None,
            filters: Filters {
                warnings: true,
                domains: [true; OPTIONS],
            },
            shown: Vec::new(),
            pending_shown: false,
            at_end: true,
            newer: 0,
            to_end: false,
            woken: false,
            size,
        };
        window.update();
        window
    }

    /// Takes in the lines kept since the last frame. Returns how many more are shown.
    fn update(&mut self) -> usize {
        let read = self.kept.read(&mut self.cursor);
        let keeps = self.kept.switches();
        let before = self.shown.len();
        let restarted = read.restarted || keeps.is_none();
        if restarted {
            self.lines.clear();
        }
        let changed = restarted
            || !read.lines.is_empty()
            || read.pending != self.pending
            || keeps != self.keeps;
        self.keeps = keeps;
        self.lines.extend(read.lines);
        let over = self.lines.len().saturating_sub(CAPACITY);
        self.lines.drain(..over);
        self.dropped = read.dropped;
        self.pending = read.pending;
        if !changed {
            return 0;
        }
        self.filter();
        if restarted {
            self.shown.len()
        } else {
            self.shown.len().saturating_sub(before)
        }
    }

    /// The lines shown, again, as the lines or the filters changed: a filter whose lines
    /// are not kept shows none, as its box is drawn.
    fn filter(&mut self) {
        let mut filters = self.filters;
        for (i, (domain, _)) in <Domain as Choice>::OPTIONS.iter().enumerate() {
            filters.domains[i] &= kept(self.keeps.as_ref(), domain);
        }
        self.shown = (0..self.lines.len())
            .filter(|i| shows(&filters, &self.lines[*i]))
            .collect();
        self.pending_shown = self.pending.as_ref().is_some_and(|l| shows(&filters, l));
    }

    /// The lines shown, oldest first.
    fn shown_lines(&self) -> impl Iterator<Item = &Line> {
        let pending = self.pending.as_ref().filter(|_| self.pending_shown);
        self.shown.iter().map(|i| &self.lines[*i]).chain(pending)
    }

    /// The lines shown as plain text, each as the terminal writes it, with its newline:
    /// what Copy to clipboard copies and Save to file… writes.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for line in self.shown_lines() {
            text.push_str(line.text());
            text.push('\n');
        }
        text
    }

    /// Draws logging off: why the window is empty, and where to turn logging on.
    fn off(&mut self, ui: &mut Ui, t: &Theme) -> Option<Asked> {
        let ctx = ui.ctx().clone();
        let mut asked = None;
        // In the middle of the window, whatever its size: as high above as the last pass
        // measured the words to need.
        let size = ui.available_size();
        let id = ui.make_persistent_id("log off height");
        let height: Option<f32> = ui.data(|d| d.get_temp(id));
        ui.allocate_ui_with_layout(size, Layout::top_down(Align::Center), |ui| {
            ui.set_min_size(size);
            ui.add_space(((size.y - height.unwrap_or(0.0)) / 2.0).max(0.0));
            let width = OFF_WIDTH.min(size.x - 4.0 * t.spacing.xl).max(0.0);
            let words = Layout::top_down(Align::Center);
            let drawn = ui.allocate_ui_with_layout(vec2(width, 0.0), words, |ui| {
                ui.spacing_mut().item_spacing.y = t.spacing.m;
                let title = egui::RichText::new(tr(&ctx, "log-off-title"))
                    .size(t.text.body)
                    .color(t.colors.text_strong);
                ui.label(title);
                let text = egui::RichText::new(tr(&ctx, "log-off-text"))
                    .size(t.text.small)
                    .color(t.colors.text_secondary);
                ui.add(egui::Label::new(text).wrap());
                ui.add_space(t.spacing.xxs);
                if Button::new(&tr(&ctx, "log-open-settings"))
                    .show(ui)
                    .clicked()
                {
                    asked = Some(Asked::Settings);
                }
            });
            let measured = drawn.response.rect.height();
            if height != Some(measured) {
                ui.data_mut(|d| d.insert_temp(id, measured));
                ui.ctx().request_discard("the log window's words measured");
            }
        });
        asked
    }

    /// Draws logging on across the window, whatever its size: the filters, wrapping where
    /// they need to, the lines in all the height left, and the footer on one line.
    fn on(&mut self, ui: &mut Ui, t: &Theme, came: usize) -> Option<Asked> {
        let ctx = ui.ctx().clone();
        let size = ui.available_size();
        let inside = (size.x - 2.0 * t.spacing.xl).max(0.0);
        ui.set_width(size.x);
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let top = ui.cursor().min.y;
        let bar = egui::Margin::symmetric(t.spacing.xl as i8, t.spacing.m as i8);
        egui::Frame::NONE.inner_margin(bar).show(ui, |ui| {
            ui.set_width(inside);
            self.filters(ui, t);
        });
        rule(ui, t);
        // Following the end, egui moves the view to the lines that came as this frame ends,
        // so the next shows them.
        if !self.at_end {
            self.newer += came;
        } else if came > 0 {
            ctx.request_repaint();
        }
        // The lines take what the footer, its margins and its rule leave.
        let footer = t.sizes.row + 2.0 * t.spacing.m + t.shape.separator_width;
        let used = ui.cursor().min.y - top;
        let lines = (size.y - used - footer).max(FEWEST * ROW);
        self.lines_area(ui, t, vec2(size.x, lines));
        rule(ui, t);
        let mut asked = None;
        egui::Frame::NONE.inner_margin(bar).show(ui, |ui| {
            ui.set_width(inside);
            match self.footer(ui, t) {
                Some(0) => asked = Some(Asked::Save(self.text())),
                Some(_) => ctx.copy_text(self.text()),
                None => {}
            }
        });
        asked
    }

    /// The filters' checkboxes, wrapping: warnings and errors, then each option of
    /// `log.domains`; one whose lines are not kept disabled, saying where to keep them.
    fn filters(&mut self, ui: &mut Ui, t: &Theme) {
        let words = translator(ui.ctx());
        let not_kept = words.text("log-filter-not-kept");
        let mut changed = false;
        let response = ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(t.spacing.xl, 0.0);
            // A checkbox row's width is known only once it is drawn, so egui cannot wrap
            // it: a box that would not fit goes on the next line before it is drawn. The
            // edges are the bar's, which a row past them would widen.
            let (left, right) = (ui.max_rect().left(), ui.max_rect().right());
            let wrap = |ui: &mut Ui, name: &str| {
                let font = egui::TextStyle::Body.resolve(ui.style());
                let galley = ui
                    .painter()
                    .layout_no_wrap(name.to_owned(), font, t.colors.text);
                let spacing = ui.spacing();
                let width = spacing.icon_width + spacing.icon_spacing + galley.size().x;
                let at = ui.cursor().min.x;
                if at + width > right && at > left {
                    ui.end_row();
                }
            };
            let warnings = words.text("log-filter-warnings");
            wrap(ui, &warnings);
            if let Some(on) = ToggleRow::new(&warnings, self.filters.warnings).show(ui) {
                self.filters.warnings = on;
                changed = true;
            }
            for (i, (domain, id)) in <Domain as Choice>::OPTIONS.iter().enumerate() {
                let kept = kept(self.keeps.as_ref(), domain);
                let name = words.text(&format!("setting-log-domains-option-{id}"));
                wrap(ui, &name);
                let mut toggle = ToggleRow::new(&name, kept && self.filters.domains[i]);
                if !kept {
                    toggle = toggle.enabled(false).promise(&not_kept);
                }
                if let Some(on) = toggle.show(ui) {
                    self.filters.domains[i] = on;
                    changed = true;
                }
            }
        });
        let name = words.text("log-filters");
        response
            .response
            .widget_info(|| WidgetInfo::labeled(Role::Toolbar, true, &name));
        if changed {
            self.filter();
            self.newer = 0;
        }
    }

    /// The lines in view across `size`, row by row, following the end while the view is
    /// there, and the pill over them while it is not.
    fn lines_area(&mut self, ui: &mut Ui, t: &Theme, size: egui::Vec2) {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let dropped = self.dropped > 0;
        let rows = usize::from(dropped) + self.shown.len() + usize::from(self.pending_shown);
        let mut area = ScrollArea::vertical()
            .id_salt("log lines")
            .min_scrolled_height(size.y)
            .max_height(size.y)
            .auto_shrink([false, false])
            .stick_to_bottom(true);
        if std::mem::take(&mut self.to_end) {
            area = area.vertical_scroll_offset((rows as f32 * ROW - size.y).max(0.0));
        }
        let scope = ui.scope_builder(UiBuilder::new(), |ui| {
            ui.set_min_size(size);
            let output = quiet_scroll_rows(ui, area, ROW, rows, |ui, range| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for row in range {
                    let row_size = vec2(ui.available_width(), ROW);
                    let (rect, response) = ui.allocate_exact_size(row_size, Sense::hover());
                    let line = match (dropped, row) {
                        (true, 0) => {
                            let mut args = apertrust_ui::i18n::Args::new();
                            args.set("count", self.dropped as f64);
                            let said = words.text_with("log-dropped", Some(&args));
                            draw_note(ui, t, rect, &said);
                            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &said));
                            continue;
                        }
                        (_, row) => row - usize::from(dropped),
                    };
                    let line = match self.shown.get(line) {
                        Some(i) => &self.lines[*i],
                        None => match &self.pending {
                            Some(pending) => pending,
                            None => continue,
                        },
                    };
                    draw_line(ui, t, rect, line);
                    let text = line.text();
                    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
                }
            });
            let end = output.content_size.y - output.inner_rect.height();
            self.at_end = output.state.clamped_offset().y >= end - ROW / 2.0;
            if self.at_end {
                self.newer = 0;
            }
            output.inner_rect
        });
        let name = words.text("log-lines");
        scope
            .response
            .widget_info(|| WidgetInfo::labeled(Role::Log, true, &name));
        if self.newer > 0 {
            let mut args = apertrust_ui::i18n::Args::new();
            args.set("count", self.newer as f64);
            let said = words.text_with("log-newer", Some(&args));
            // A text button, as wide as its words and their padding.
            let font = FontId::proportional(t.text.body);
            let width = ui
                .painter()
                .layout_no_wrap(said.clone(), font, t.colors.text)
                .size()
                .x;
            let size = vec2(width + 2.0 * t.spacing.m, t.sizes.control);
            let area = scope.inner;
            let at = area.max - size - vec2(t.spacing.xxl, t.spacing.l);
            // Over the lines, in a child that moves nothing after it.
            let mut over = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(at, size)));
            if Button::new(&said).show(&mut over).clicked() {
                self.to_end = true;
                self.newer = 0;
                ctx.request_repaint();
            }
        }
    }

    /// The footer, on one line: the counts, then the note and the buttons at its end. Where
    /// the line is too short, the note gives way first, then the counts are cut with "…";
    /// the buttons never are. Returns the button pressed: 0 for Save to file…, 1 for Copy to
    /// clipboard.
    fn footer(&self, ui: &mut Ui, t: &Theme) -> Option<usize> {
        let words = translator(ui.ctx());
        let shown = self.shown.len() + usize::from(self.pending_shown);
        let counts = counts(&words, self.lines.len(), shown);
        let note = words.text("log-names-files");
        let quiet = |text: &str| {
            egui::RichText::new(text)
                .size(t.text.small)
                .color(t.colors.text_secondary)
        };
        let font = FontId::proportional(t.text.small);
        let width_of = |text: &str| {
            let galley = ui
                .painter()
                .layout_no_wrap(text.to_owned(), font.clone(), t.colors.text);
            galley.size().x.ceil()
        };
        let (counts_width, note_width) = (width_of(&counts), width_of(&note));
        // The buttons' width as the last pass measured it, as a dialog's buttons are.
        let id = ui.make_persistent_id("log footer buttons");
        let buttons: Option<f32> = ui.data(|d| d.get_temp(id));
        let gap = t.spacing.l;
        let (line, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), t.sizes.row), Sense::hover());
        let end = buttons.unwrap_or(0.0);
        let room = (line.width() - end - gap).max(0.0);
        let with_note = counts_width + gap + note_width <= room;
        let mut part = |x: f32, width: f32| {
            let rect =
                Rect::from_min_size(pos2(x, line.top()), vec2(width.max(0.0), line.height()));
            ui.new_child(
                UiBuilder::new()
                    .max_rect(rect)
                    .layout(Layout::left_to_right(Align::Center)),
            )
        };
        // In reading order, for screen readers and Tab: the counts, the note, the buttons.
        let mut left = part(line.left(), if with_note { counts_width } else { room });
        left.add(egui::Label::new(quiet(&counts)).truncate());
        if with_note {
            let mut middle = part(line.right() - end - gap - note_width, note_width);
            middle.label(quiet(&note));
        }
        let mut right = part(line.right() - end, end.max(1.0));
        right.spacing_mut().item_spacing.x = gap;
        let mut pressed = None;
        for (i, words) in [words.text("log-save"), words.text("log-copy")]
            .into_iter()
            .enumerate()
        {
            if Button::new(&words).show(&mut right).clicked() {
                pressed = Some(i);
            }
        }
        let measured = right.min_rect().width().ceil();
        if buttons != Some(measured) {
            ui.data_mut(|d| d.insert_temp(id, measured));
            ui.ctx()
                .request_discard("the log footer's buttons measured");
        }
        pressed
    }
}

/// The footer's counts, `kept` lines and `shown` of them, in the language of `words`, the
/// numbers in its format, their digits grouped: "1,284 lines kept · 1,107 shown · …".
fn counts(words: &apertrust_ui::Translator, kept: usize, shown: usize) -> String {
    let mut args = apertrust_ui::i18n::Args::new();
    args.set("kept", kept as f64);
    args.set("shown", shown as f64);
    words.text_with("log-counts", Some(&args))
}

/// Whether `keeps` keeps `domain`'s lines, or `verbose`'s detail for [`Domain::VERBOSE`].
fn kept(keeps: Option<&Switches>, domain: &Domain) -> bool {
    keeps.is_some_and(|keeps| {
        if *domain == Domain::VERBOSE {
            keeps.verbose
        } else {
            keeps.domains.contains(&domain.name)
        }
    })
}

/// Whether `filters` show `line`: a warning or an error as warnings are; a domain's line
/// as its domain is, its detail only with `verbose` too; the banner's always.
fn shows(filters: &Filters, line: &Line) -> bool {
    if matches!(line.severity(), Severity::Error | Severity::Warning) {
        return filters.warnings;
    }
    if line.domain().is_empty() {
        return true;
    }
    let verbose = filters.domains[OPTIONS - 1];
    let detail = line.severity() == Severity::Detail;
    let options = <Domain as Choice>::OPTIONS.iter();
    match options
        .take(OPTIONS - 1)
        .position(|(d, _)| d.name == line.domain())
    {
        Some(i) => filters.domains[i] && (!detail || verbose),
        None => !detail || verbose,
    }
}

/// A line across `rect`: the seconds right-aligned in their column, the domain, the line
/// cut at the end with "…"; a warning or an error in its colour on a faint band of it.
fn draw_line(ui: &Ui, t: &Theme, rect: Rect, line: &Line) {
    let painter = ui.painter_at(rect);
    let marked = match line.severity() {
        Severity::Error => Some(t.colors.error),
        Severity::Warning => Some(t.colors.warning),
        _ => None,
    };
    if let Some(colour) = marked {
        painter.rect_filled(rect, 0.0, colour.gamma_multiply(0.09));
    }
    let (font, seconds, domain) = line_font(ui, t);
    let quiet = marked.unwrap_or(t.colors.text_secondary);
    let y = rect.center().y;
    let seconds_end = rect.left() + t.spacing.xl + seconds;
    painter.text(
        pos2(seconds_end, y),
        Align2::RIGHT_CENTER,
        line.seconds(),
        font.clone(),
        t.colors.text_secondary,
    );
    let domain_at = seconds_end + t.spacing.l;
    painter.text(
        pos2(domain_at, y),
        Align2::LEFT_CENTER,
        line.bracket(),
        font.clone(),
        quiet,
    );
    let message_at = domain_at + domain + t.spacing.l;
    let width = rect.right() - t.spacing.xl - message_at;
    let colour = marked.unwrap_or(t.colors.text);
    let mut job =
        LayoutJob::single_section(line.message().to_owned(), TextFormat::simple(font, colour));
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    let galley = painter.layout_job(job);
    let at = pos2(message_at, y - galley.size().y / 2.0);
    painter.galley(at, galley, colour);
}

/// The note in the first row once the oldest lines have gone.
fn draw_note(ui: &Ui, t: &Theme, rect: Rect, said: &str) {
    let painter = ui.painter_at(rect);
    let (_, seconds, domain) = line_font(ui, t);
    let at = rect.left() + t.spacing.xl + seconds + t.spacing.l + domain + t.spacing.l;
    painter.text(
        pos2(at, rect.center().y),
        Align2::LEFT_CENTER,
        said,
        FontId::proportional(t.text.small),
        t.colors.text_secondary,
    );
}

/// The lines' font, IBM Plex Mono at `text.small`, and the widths of the seconds' and the
/// domain's columns in it.
fn line_font(ui: &Ui, t: &Theme) -> (FontId, f32, f32) {
    let font = FontId::new(t.text.small, FontFamily::Name(fonts::MONO.into()));
    let advance = ui.ctx().fonts_mut(|f| f.glyph_width(&font, '0'));
    (font, SECONDS as f32 * advance, DOMAIN as f32 * advance)
}

/// A rule between the window's parts, across it.
fn rule(ui: &mut Ui, t: &Theme) {
    let width = ui.max_rect().width();
    let (line, _) = ui.allocate_exact_size(vec2(width, t.shape.separator_width), Sense::hover());
    ui.painter().rect_filled(line, 0.0, t.colors.divider);
}

/// The log window in the dialog host, a slot of its own: modeless on every platform, kept
/// where and at the size it was left, the one dialog the person resizes, on the panel's
/// surface; no key of its own closes it, Close Window doing so on the Mac while it has the
/// focus.
impl Dialog for Window {
    fn id(&self) -> &'static str {
        "log"
    }

    fn title(&self, ctx: &egui::Context) -> String {
        tr(ctx, "log-name")
    }

    fn form(&self, _os: OperatingSystem) -> Form {
        Form::Modeless
    }

    /// The panel's colour to its edges: the window draws its own margins.
    fn look(&self, t: &Theme) -> Look {
        Look {
            background: t.colors.panel,
            margin: 0.0,
        }
    }

    fn keys(&self, _os: OperatingSystem) -> Keys {
        Keys::default()
    }

    fn show(&mut self, ui: &mut Ui) -> Asks {
        let t = theme(ui.ctx());
        let came = self.update();
        let asked = ui
            .with_layout(Layout::top_down(Align::Min), |ui| {
                if self.keeps.is_some() {
                    self.on(ui, &t, came)
                } else {
                    self.off(ui, &t)
                }
            })
            .inner;
        Asks {
            outputs: asked.map(Output::Log).into_iter().collect(),
            ..Asks::default()
        }
    }

    fn placed(&self, at: Position) -> Option<Output> {
        Some(Output::LogPlaced(at))
    }

    fn wake_with(&mut self, waker: Waker) {
        self.kept.wake_with(Some(waker));
        self.woken = true;
    }

    /// The person resizes it, from [`LEAST`] up, and it opens as they left it.
    fn resize(&self) -> Option<Resize> {
        let size = self.size.unwrap_or(SIZE).max(LEAST);
        Some(Resize { least: LEAST, size })
    }

    fn sized(&self, size: egui::Vec2) -> Option<Output> {
        Some(Output::LogSized(size))
    }
}

/// As the window goes, the log stops waking it.
impl Drop for Window {
    fn drop(&mut self) {
        if self.woken {
            self.kept.wake_with(None);
        }
    }
}

/// The name Save to file… offers: `apertrust-log-` and the session's start, `started`, to
/// the minute, `apertrust-log-2026-11-07-19-15.txt`.
pub fn file_name(started: &DateTime) -> String {
    format!(
        "apertrust-log-{:04}-{:02}-{:02}-{:02}-{:02}.txt",
        started.year, started.month, started.day, started.hour, started.minute
    )
}

/// The folder Save to file…'s dialog opens in: the one the last save went to, else
/// Downloads. It is never looked at here, on the UI thread: a folder that has gone is the
/// platform's dialog's to pass over.
pub fn save_folder(last: Option<&Path>, downloads: Option<&Path>) -> Option<PathBuf> {
    last.or(downloads).map(Path::to_owned)
}

#[cfg(test)]
mod tests;
