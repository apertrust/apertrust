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

//! The Export tool's Summary and its end (`docs/gui.md`, The tool's end): the Summary, a
//! collapsible group like Global options, a labelled list of what an export would write;
//! below it the warnings, the Export button in each of its states and a failed export's
//! line. They scroll with the tool and are never pinned.
//!
//! What they need of the app besides the settings comes in each frame as [`Shown`]: whether
//! the groups are open, the photo the tool acts on and whether that photo can be exported,
//! and the last export's failure ([`Jobs`]).

use apertrust_ui::components::{ToolButton, middle_cut};
use apertrust_ui::i18n::{Args, Translator, decimals};
use eframe::egui::{
    self, Color32, FontId, Label, Rect, RichText, Role, Sense, Ui, WidgetInfo, pos2, vec2,
};

use super::*;

/// How soon after Export a second press is ignored, in seconds.
const GUARD: f64 = 0.5;

/// Why a photo cannot be exported though its edit is there to see: it is open for viewing
/// only.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
    /// A newer Apertrust wrote its edit.
    Newer,
    /// Part of its history cannot be read; `file` is the edit's file.
    Unreadable { file: String },
}

/// The photo the tool acts on, as the app tells it each frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Photo {
    /// None is open: nothing to export.
    #[default]
    None,
    /// `name` is open and can be exported.
    Ready { name: String },
    /// `name` is open for viewing only, so it is left out of the export.
    ViewOnly { name: String, why: Why },
}

/// How the user's exports stand, read from the jobs mirror: only a failure is the tool's to
/// say, progress being the jobs window's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Jobs {
    /// The last export's photos exported and its total, while it ended failed.
    pub failed: Option<(u64, u64)>,
}

/// The selected photos besides the one open, while several are selected (decision 90):
/// how many the export takes, and how many it leaves out, open for viewing only. None
/// with one photo. `pending` while a selection command the session has not told yet
/// leaves the photos to export unknown: Export waits, drawn disabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Others {
    pub taken: usize,
    pub left_out: usize,
    pub pending: bool,
}

/// What the app tells the tool each frame besides its header, which is the shell's
/// (`tools::Head`), and its page's groups, which the settings keep.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    /// Whether the summary is open.
    pub summary: bool,
    pub photo: Photo,
    pub others: Others,
    pub jobs: Jobs,
}

/// What the end reports for the frame.
#[derive(Default)]
pub(super) struct End {
    pub pressed: bool,
    pub show_jobs: bool,
}

/// A row of the Summary's list: its label, its value, and whether the value is a path, cut
/// in its middle where it is too long, else at its end.
struct Row {
    label: String,
    value: String,
    path: bool,
}

/// The height of a Summary row, in points.
const ROW: f32 = 22.0;

/// How wide `text` is in `font`.
fn width_of(painter: &egui::Painter, font: &FontId, text: &str) -> f32 {
    painter
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x
}

/// `text` cut at its end with … to fit `room` in `font`; whole where it fits.
fn cut_end(painter: &egui::Painter, font: &FontId, text: &str, room: f32) -> String {
    if width_of(painter, font, text) <= room {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    // The most characters that fit with the ellipsis after them.
    while low < high {
        let mid = (low + high).div_ceil(2);
        let cut: String = chars[..mid].iter().collect::<String>() + "…";
        if width_of(painter, font, &cut) <= room {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    chars[..low].iter().collect::<String>() + "…"
}

impl Export {
    /// Notes that Export was pressed to some effect at `now`, for the guard against a second
    /// press.
    pub fn note_started(&mut self, now: f64) {
        self.pressed_at = Some(now);
    }

    /// Whether a second press at `now` is too soon after the last.
    pub fn too_soon(&self, now: f64) -> bool {
        self.pressed_at.is_some_and(|at| now - at < GUARD)
    }

    /// The end: the warnings, the button and a failed export's line. Returns what was
    /// pressed.
    pub(super) fn end(&mut self, ui: &mut Ui, t: &Theme) -> End {
        ui.add_space(t.spacing.m);
        // With no photo there is nothing to warn of: only the button, disabled.
        let others = self.others.taken + self.others.left_out > 0;
        if others || matches!(self.photo, Photo::Ready { .. } | Photo::ViewOnly { .. }) {
            self.warnings(ui, t);
            ui.add_space(t.spacing.m);
        }
        let pressed = self.button(ui);
        ui.add_space(t.spacing.xs);
        let show_jobs = self.job_line(ui, t);
        ui.add_space(t.spacing.m);
        End { pressed, show_jobs }
    }

    /// The folder the files go to, shown as the person's paths are: the first photo's
    /// from the session's answer, else what the pattern says before the file name.
    fn folder(&self) -> String {
        let named = self.preview.as_ref().and_then(|(_, preview)| {
            preview.names.iter().find_map(|name| match &name.fate {
                Fate::File { file, .. } => file.parent().map(Path::to_owned),
                Fate::NoName => None,
            })
        });
        let folder = named.or_else(|| Path::new(&self.values.path).parent().map(Path::to_owned));
        folder.map_or_else(String::new, |f| shown(&f, self.home.as_deref()))
    }

    /// The file's name as the export will write it: the session's, "IMG_2041.jpg", with the
    /// extension of the format now chosen; where it is a guess, or there is no answer yet, the pattern's own file name as written, with
    /// the extension, as the path field's tooltip has it.
    fn file_name_text(&self) -> String {
        let extension = match self.values.format {
            FileFormat::Jpeg => "jpg",
            FileFormat::Png => "png",
        };
        let pattern = || {
            let name = Path::new(&self.values.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("{name}.{extension}")
        };
        match self.preview.as_ref().and_then(|(_, p)| p.names.first()) {
            Some(first) => match &first.fate {
                // The session's name, with the format's extension as it is now: the answer
                // for the format before a change comes until the new one does.
                Fate::File { file, .. } if first.exact => file_of(&file.with_extension(extension)),
                Fate::File { .. } => pattern(),
                Fate::NoName => "—".to_owned(),
            },
            None => pattern(),
        }
    }

    /// The size the export will write, as the session's answer gives it: "2048 × 1365 px",
    /// and in the print modes the physical size as well, "20.00 × 13.33 cm (6000 × 4000 px)".
    /// "—" until there is one, or where the size is a problem.
    fn size_text(&self, words: &Translator) -> String {
        let Some(Ok(output)) = self
            .preview
            .as_ref()
            .and_then(|(_, p)| p.size.as_ref())
            .map(|active| &active.size)
        else {
            return "—".to_owned();
        };
        let v = &self.values;
        let mut args = Args::new();
        args.set("width", output.width.to_string());
        args.set("height", output.height.to_string());
        if !matches!(v.size_mode, SizeMode::Cm | SizeMode::Inch) {
            return words.text_with("export-summary-size", Some(&args));
        }
        let cm = v.size_mode == SizeMode::Cm;
        let dpi = v.dpi.clamp(72, 9600);
        let length = |pixels: u32| {
            f64::from(if cm {
                size::pixels_to_cm(pixels, dpi)
            } else {
                size::pixels_to_inches(pixels, dpi)
            })
        };
        // The lengths rounded to two decimals, in the language's numbers.
        let two = |pixels: u32| decimals((length(pixels) * 100.0).round() / 100.0, 2);
        let mut print = Args::new();
        print.set("width", two(output.width));
        print.set("height", two(output.height));
        print.set(
            "unit",
            words.text(if cm {
                "export-unit-cm"
            } else {
                "export-unit-inch"
            }),
        );
        print.set("pixels_width", output.width.to_string());
        print.set("pixels_height", output.height.to_string());
        words.text_with("export-summary-size-print", Some(&print))
    }

    /// The Summary's rows, in their order: Preset, File name, Size, Scale where Set size is
    /// Scale, Profile, Format and Destination.
    fn summary_rows(&self, ctx: &egui::Context) -> Vec<Row> {
        let words = translator(ctx);
        let v = &self.values;
        let plain = |label: &str, value: String| Row {
            label: tr(ctx, label),
            value,
            path: false,
        };
        let preset = if *v == Values::default() {
            tr(ctx, "export-preset-default")
        } else {
            tr(ctx, "preset-custom")
        };
        let mut rows = vec![
            plain("export-summary-label-preset", preset),
            plain("export-summary-label-file-name", self.file_name_text()),
            plain("export-summary-label-size", self.size_text(&words)),
        ];
        if v.size_mode == SizeMode::Scale {
            let mut args = Args::new();
            args.set("percent", size::scale_factor(&v.scale) * 100.0);
            let scale = words.text_with("export-summary-scale", Some(&args));
            rows.push(plain("export-summary-label-scale", scale));
        }
        let profile = match v.profile {
            Profile::ImageSettings => tr(ctx, "export-profile-image"),
            Profile::Srgb => tr(ctx, PROFILES[0]),
        };
        rows.push(plain("export-summary-label-profile", profile));
        let format = match v.format {
            FileFormat::Jpeg => {
                let mut args = Args::new();
                args.set("quality", f64::from(v.jpeg_quality));
                words.text_with("export-summary-jpeg", Some(&args))
            }
            FileFormat::Png => {
                let mut args = Args::new();
                let bits = match v.png_bit_depth {
                    BitDepth::Eight => 8.0,
                    BitDepth::Sixteen => 16.0,
                };
                args.set("bits", bits);
                words.text_with("export-summary-png", Some(&args))
            }
        };
        rows.push(plain("export-summary-label-format", format));
        rows.push(Row {
            label: tr(ctx, "export-summary-label-destination"),
            value: self.folder(),
            path: true,
        });
        rows
    }

    /// The Summary's list: labels right-aligned in a column as wide as the longest, values
    /// left-aligned after it, a row 22 points high, each value cut at its end with … (a path
    /// in its middle) and whole in its tooltip. To screen readers each row is a label with
    /// its full value.
    pub(super) fn summary_list(&self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let rows = self.summary_rows(&ctx);
        let body = FontId::proportional(t.text.body);
        let column = rows
            .iter()
            .map(|row| width_of(ui.painter(), &body, &row.label))
            .fold(0.0, f32::max);
        let width = ui.available_width();
        for row in &rows {
            let (rect, _) = ui.allocate_exact_size(vec2(width, ROW), Sense::hover());
            let id = ui.make_persistent_id(("export summary", &row.label));
            let reading = ui.interact(rect, id, Sense::hover());
            let spoken = format!("{} {}", row.label, row.value);
            reading.widget_info(|| WidgetInfo::labeled(Role::Label, true, &spoken));
            let painter = ui.painter();
            let middle = rect.center().y;
            painter.text(
                pos2(rect.left() + column, middle),
                egui::Align2::RIGHT_CENTER,
                &row.label,
                body.clone(),
                t.colors.text_secondary,
            );
            let left = rect.left() + column + t.spacing.m;
            let room = (rect.right() - left).max(0.0);
            let shown = if row.path {
                middle_cut(painter, &row.value, "", &body, room)
            } else {
                cut_end(painter, &body, &row.value, room)
            };
            painter.text(
                pos2(left, middle),
                egui::Align2::LEFT_CENTER,
                &shown,
                body.clone(),
                t.colors.text,
            );
            if shown != row.value {
                drop(reading.on_hover_text(&row.value));
            }
        }
    }

    /// The warnings, in the warning colour with the sign: view-only photos left out, and
    /// the files an export would replace.
    fn warnings(&self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let left_out =
            usize::from(matches!(self.photo, Photo::ViewOnly { .. })) + self.others.left_out;
        if left_out > 0 {
            let mut args = Args::new();
            args.set("count", left_out as f64);
            warn(
                ui,
                t,
                words.text_with("export-view-only-left-out", Some(&args)),
            );
        }
        if !matches!(self.photo, Photo::Ready { .. }) || self.path_problem(&words).is_some() {
            return;
        }
        let Some((_, preview)) = &self.preview else {
            return;
        };
        let replaced = preview.counts().overwritten;
        if replaced > 0 {
            // Where some names are known only as the photos are exported, the count is a
            // lower bound.
            let guessed = preview.counts().guessed > 0;
            let id = if guessed {
                "export-warn-replaces-at-least"
            } else {
                "export-warn-replaces"
            };
            let mut args = Args::new();
            args.set("count", replaced as f64);
            warn(ui, t, words.text_with(id, Some(&args)));
        }
    }

    /// Why Export cannot run, in words, if it cannot: nothing to export, or only a photo
    /// open for viewing only, or the session has found a problem with the path, its folder
    /// or the size (D8). Until the session answers there is none to find; it checks again as
    /// the job starts. Other selected photos it takes make the photo open no reason.
    pub(super) fn cannot_export(&self, ctx: &egui::Context) -> Option<String> {
        let say = |id: &str, problem: &str| {
            let mut args = Args::new();
            args.set("problem", problem);
            translator(ctx).text_with(id, Some(&args))
        };
        match &self.photo {
            _ if self.others.taken > 0 => {}
            // Nothing open: the button is disabled, and says nothing more.
            Photo::None => return None,
            Photo::ViewOnly {
                why: Why::Newer, ..
            } => return Some(tr(ctx, "export-why-view-only-newer")),
            Photo::ViewOnly {
                why: Why::Unreadable { file },
                ..
            } => {
                let mut args = Args::new();
                args.set("file", file.as_str());
                return Some(
                    translator(ctx).text_with("export-why-view-only-unreadable", Some(&args)),
                );
            }
            Photo::Ready { .. } => {}
        }
        // The path's own problems are every photo's; its folder's checks are the photo
        // open's folder's, no reason where the export leaves that photo out.
        let problem = match self.photo {
            Photo::Ready { .. } => self.path_problem(&translator(ctx)),
            _ => self.pattern_problem(&translator(ctx)),
        };
        if let Some(problem) = problem {
            return Some(say("export-why-path", &problem.text));
        }
        // Overwrite kept from a platform that asks: no question here, so no export.
        if !self.alerts && asks_before(self.values.on_conflict) {
            return Some(tr(ctx, "export-conflict-overwrite-no-alert"));
        }
        // The preview's names and size are the photo open's: no reason where the export
        // leaves it out.
        if !matches!(self.photo, Photo::Ready { .. }) {
            return None;
        }
        let (_, preview) = self.preview.as_ref()?;
        // A path that expands to nothing for every photo, such as one made of variables with
        // no value here, would fail each: no file name, no export.
        if !preview.names.is_empty()
            && preview
                .names
                .iter()
                .all(|name| matches!(name.fate, Fate::NoName))
        {
            return Some(tr(ctx, "export-no-name"));
        }
        match &preview.size.as_ref()?.size {
            Ok(_) => None,
            Err(problem) => Some(say(
                "export-why-size",
                &size_problem(&translator(ctx), problem),
            )),
        }
    }

    /// The Export button, primary and the section's full width, its words centred. Where
    /// Export cannot run it is held off: drawn disabled, still a stop for the keyboard, the
    /// reason its tooltip and description. Returns whether it was pressed to some effect.
    fn button(&mut self, ui: &mut Ui) -> bool {
        let ctx = ui.ctx().clone();
        let reason = self.cannot_export(&ctx);
        let none = matches!(self.photo, Photo::None) && self.others.taken == 0;
        let idle = none || self.others.pending;
        // The photos it exports; with none, the words as for one.
        let count = usize::from(matches!(self.photo, Photo::Ready { .. })) + self.others.taken;
        let mut args = Args::new();
        args.set("count", count.max(1) as f64);
        let words = translator(&ctx).text_with("export-button", Some(&args));
        let mut button = Button::new(&words)
            .primary(true)
            .centred(true)
            .enabled(!idle)
            .held_off(reason.is_some())
            .height(theme(&ctx).sizes.row)
            .width(ui.available_width());
        if let Some(why) = &reason {
            button = button.tip(why);
        }
        button.show_pressed(ui)
    }

    /// The line under the button: the last export's failure, in `error`, with Show jobs,
    /// until the next export is added or its row is dismissed. Nothing under it while
    /// exports run: progress is the jobs window's. Returns whether Show jobs was pressed.
    fn job_line(&self, ui: &mut Ui, t: &Theme) -> bool {
        let ctx = ui.ctx().clone();
        let Some((done, total)) = self.jobs.failed else {
            return false;
        };
        let mut args = Args::new();
        args.set("done", done as f64);
        args.set("total", total as f64);
        let line = translator(&ctx).text_with("export-failed", Some(&args));
        let show = tr(&ctx, "export-show-jobs");
        let button = ToolButton::new(&show);
        let room = (ui.available_width() - button.width(ui) - t.spacing.s).max(0.0);
        let mut pressed = false;
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            ui.allocate_ui_with_layout(
                vec2(room, t.sizes.panel_line),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add(
                        Label::new(RichText::new(line).size(t.text.small).color(t.colors.error))
                            .selectable(false)
                            .wrap(),
                    );
                },
            );
            ui.add_space((ui.available_width() - button.width(ui)).max(0.0));
            pressed = button.show(ui).clicked();
        });
        pressed
    }
}

/// The name of the file at `path`, as shown.
fn file_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A warning: the sign, then the words in the warning colour, wrapping.
fn warn(ui: &mut Ui, t: &Theme, text: String) {
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(12.0, 16.0), Sense::hover());
        Icon::Warning.paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(12.0, 12.0)),
            t.colors.warning,
        );
        ui.add(
            Label::new(
                RichText::new(text)
                    .size(t.text.small)
                    .color(t.colors.warning),
            )
            .wrap(),
        );
    });
}
