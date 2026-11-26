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

//! The Info tool (`docs/gui.md`, The Info tool): the active photo at a glance, read-only,
//! a utility in no tab of the built-in workspaces.
//!
//! - **Its card** ([`Card`]) is the active photo's name and date, then in a well its
//!   camera with the white balance's tag and the flash, its lens, its size with the file's
//!   type, and a strip of its five exposure values, each worded by the Metadata tool's
//!   [`metadata::format`], never words of its own.
//! - **Its ⋯** holds Copy Information and Show in Metadata, its own items.
//! - **The card is one Tab stop,** Ctrl+C (Cmd+C) on it copying what Copy Information
//!   copies; to screen readers a region named Info, described in a sentence.

use apertrust_engine::session::Photo;
use apertrust_tools::Widget;
use apertrust_tools::messages::fact_words;
use apertrust_tools::utility::UtilityDefinition;
use apertrust_ui::components::{Tag, copy_asked, focus_ring_centred, truncated};
use apertrust_ui::fonts::MEDIUM;
use apertrust_ui::popup::{Entry, Item};
use apertrust_ui::{Icon, Theme, Translator, theme};
use eframe::egui::{self, FontFamily, FontId, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::metadata::{self, format};
use crate::mirror::Mirror;
use crate::utility::{Context, UtilityTool};

/// The tool's id.
pub const ID: &str = "info";
/// Its own ⋯ items, by their ids.
pub const COPY_INFORMATION: &str = "info-copy-information";
pub const SHOW_IN_METADATA: &str = "info-show-in-metadata";

/// The strip's five values, by the Metadata tool's facts, in order.
const STRIP: [&str; 5] = [
    "iso",
    "focal_length",
    "exposure_bias",
    "aperture",
    "exposure_time",
];
/// A line's height, in points, the wireframe's, shorter than a tool's rows.
const LINE: f32 = 20.0;

/// The Info tool's declaration, in `UTILITIES`.
pub fn definition() -> &'static UtilityDefinition {
    apertrust_tools::utility::utility(ID).expect("Info is built in")
}

/// A value as the card shows it: its words, none for "—", and its tooltip.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown {
    pub value: Option<String>,
    pub tip: String,
}

/// Whether the flash fired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flash {
    Fired,
    NotFired,
}

/// The card's words for one photo.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// The file's name, its path in the tooltip.
    pub name: Shown,
    /// The date taken, without its offset, which the tooltip gives with its sub-seconds.
    pub date: Shown,
    /// The camera's maker and model, the maker once.
    pub camera: String,
    /// The white balance's tag, "AWB" or "WB", and its tooltip; none where unrecorded.
    pub white_balance: Option<(String, String)>,
    /// The flash and its tooltip; none where unrecorded or the camera has no flash.
    pub flash: Option<(Flash, String)>,
    /// The lens, none for "No lens recorded".
    pub lens: Shown,
    /// The size, "6000 × 4000", its weight, "· 32 MB", and their tooltip.
    pub dimensions: String,
    pub weight: String,
    pub size_tip: String,
    /// The type's tag, "DNG", and the format's name.
    pub format: (String, String),
    /// ISO, focal length, exposure bias, aperture and exposure time.
    pub strip: [Shown; 5],
    /// What Copy Information copies, "Label: value" a line.
    pub copied: String,
    /// What screen readers hear of it.
    pub said: String,
}

impl Card {
    /// The card of `photo` in `words`.
    pub fn of(photo: &Photo, words: &Translator) -> Card {
        let source = format::Source {
            of: format::Of::photo(photo),
            history: None,
            home: None,
            words,
        };
        let fact = |id: &str| format::fact(id, &source);
        let row = |id: &str, also: Option<String>| row(id, photo, &source, also);
        let exif = &photo.exif;
        let name = fact("file_name");
        let name = Shown {
            tip: name.note.clone().unwrap_or_default(),
            value: name.value,
        };
        let taken = exif.taken.and_then(|at| format::naive(&at));
        let date = Shown {
            value: taken.map(|at| words.format_long_date_time(at, None)),
            tip: row("date_taken", None).tip,
        };
        let camera = camera(fact("make").value, fact("model").value);
        let white_balance = exif.white_balance.and_then(|code| {
            let tag = match code {
                0 => "info-white-balance-auto",
                1 => "info-white-balance-manual",
                _ => return None,
            };
            Some((words.text(tag), row("white_balance", None).tip))
        });
        // None where the camera has no flash function.
        let flash = exif.flash.filter(|bits| bits & 0x20 == 0).map(|bits| {
            let fired = if bits & 1 != 0 {
                Flash::Fired
            } else {
                Flash::NotFired
            };
            (fired, row("flash", None).tip)
        });
        let lens = row("lens", None);
        let lens = Shown {
            value: lens.value.filter(|lens| !lens.is_empty()),
            ..lens
        };
        let dimensions = source.say(
            "info-dimensions",
            &[
                ("width", photo.width.to_string()),
                ("height", photo.height.to_string()),
            ],
        );
        let weight = format::size(photo.file_size, words);
        let weight = source.say("info-weight", &[("size", weight)]);
        let size_tip = row("dimensions", None).tip;
        let id = format!("info-format-{}", photo.format.to_lowercase());
        let format_name = if words.has(&id) {
            words.text(&id)
        } else {
            photo.format.clone()
        };
        let strip = STRIP.map(|id| {
            // The 35 mm equivalent in the focal length's tooltip.
            let also = (id == "focal_length")
                .then(|| fact("focal_length_35mm").value)
                .flatten()
                .map(|mm| {
                    let label = words.text(&fact_words(metadata::ID, "focal_length_35mm")[0]);
                    format!("{label}: {mm}")
                });
            row(id, also)
        });
        let mut card = Card {
            name,
            date,
            camera,
            white_balance,
            flash,
            lens,
            dimensions,
            weight,
            size_tip,
            format: (photo.format.clone(), format_name),
            strip,
            copied: String::new(),
            said: String::new(),
        };
        card.copied = card.information(&source);
        card.said = said(&card, photo, &source);
        card
    }

    /// Its lines as shown, "Label: value" a line, "—" for none, as the Metadata tool's Copy
    /// Information copies its rows, the tags and the flash in the Metadata tool's words.
    fn information(&self, source: &format::Source<'_>) -> String {
        let words = source.words;
        let label = |id: &str| words.text(&fact_words(metadata::ID, id)[0]);
        let mut lines = vec![
            (label("file_name"), self.name.value.clone()),
            (label("date_taken"), self.date.value.clone()),
            (words.text("info-camera"), Some(self.camera.clone())),
        ];
        let marks = self.marks(words).into_iter();
        lines.extend(marks.map(|(label, value)| (label, Some(value))));
        lines.push((label("lens"), self.lens.value.clone()));
        let size = format!("{} {}", self.dimensions, self.weight);
        lines.push((words.text("info-size"), Some(size)));
        lines.push((label("type"), Some(self.format.0.clone())));
        for (id, shown) in STRIP.iter().zip(&self.strip) {
            lines.push((label(id), shown.value.clone()));
        }
        let lines = lines.into_iter().map(|(label, value)| {
            let value = value.filter(|value| !value.is_empty());
            format!("{label}: {}", value.as_deref().unwrap_or("—"))
        });
        lines.collect::<Vec<_>>().join("\n")
    }

    /// What the marks at the camera line's end stand for, each a label and the Metadata
    /// tool's words, its tooltip's first line: the white balance's, then the flash's, where
    /// the card shows them.
    fn marks(&self, words: &Translator) -> Vec<(String, String)> {
        let label = |id: &str| words.text(&fact_words(metadata::ID, id)[0]);
        let first = |tip: &str| tip.lines().next().unwrap_or_default().to_owned();
        let balance =
            (self.white_balance.as_ref()).map(|(_, tip)| (label("white_balance"), first(tip)));
        let flash = (self.flash.as_ref()).map(|(_, tip)| (label("flash"), first(tip)));
        balance.into_iter().chain(flash).collect()
    }
}

/// The Metadata tool's fact `id` of `photo` as its row shows it ([`metadata::one_fact`]),
/// its tooltip then saying `also` and the fact's tag.
fn row(id: &str, photo: &Photo, source: &format::Source<'_>, also: Option<String>) -> Shown {
    let (value, tip) = metadata::one_fact(id, photo, source);
    let tag = source.words.text(&fact_words(metadata::ID, id)[1]);
    let lines = [Some(tip), also, Some(tag)];
    Shown {
        value,
        tip: lines.into_iter().flatten().collect::<Vec<_>>().join("\n"),
    }
}

/// The camera as its maker and model name it, the maker's first word once: "RICOH GR IV"
/// of RICOH IMAGING COMPANY, LTD. and RICOH GR IV, "SONY ILCE-7RM5" of SONY and ILCE-7RM5.
fn camera(make: Option<String>, model: Option<String>) -> String {
    let make = make.unwrap_or_default();
    let model = model.unwrap_or_default();
    let maker = make.split_whitespace().next().unwrap_or_default();
    let named = model
        .split_whitespace()
        .next()
        .is_some_and(|first| first.eq_ignore_ascii_case(maker));
    if maker.is_empty() || named {
        model
    } else if model.is_empty() {
        make
    } else {
        format!("{maker} {model}")
    }
}

/// What screen readers hear of the card, every value it has in the card's order, its
/// marks as Copy Information words them: "R0001225.DNG, Saturday, 4 July 2026, 08:10, RICOH
/// GR IV, White balance: Auto, Flash: Off, did not fire, GR Lens 18.3mm F2.8, 6000 × 4000,
/// 32 MB, ISO 200, 18.3 millimetres, −0.33 EV, f 5.6, 1/500 second".
fn said(card: &Card, photo: &Photo, source: &format::Source<'_>) -> String {
    let (exif, words) = (&photo.exif, source.words);
    let taken = exif.taken.and_then(|at| format::naive(&at));
    let date = taken.map(|at| {
        let day = words.format_spoken_date(at.date());
        format!("{day}, {}", words.format_time(at))
    });
    let iso = card.strip[0].value.clone();
    let focal = format::recorded(exif.focal_length).map(|mm| {
        let value = source.trimmed(f64::from(mm), 1, false);
        source.say("info-spoken-focal", &[("value", value)])
    });
    let aperture = format::recorded(exif.aperture).map(|f| {
        let value = source.number(f64::from(f), 1);
        source.say("info-spoken-aperture", &[("value", value)])
    });
    let shutter = format::recorded(exif.exposure_time).map(|seconds| {
        let value = crate::exposure::shutter(seconds, words);
        let mut args = apertrust_ui::i18n::Args::new();
        args.set("value", value);
        // A fraction of a second is said as one.
        args.set("count", f64::from(seconds.max(1.0)));
        words.text_with("info-spoken-shutter", Some(&args))
    });
    let camera = Some(card.camera.clone()).filter(|camera| !camera.is_empty());
    let weight = format::size(photo.file_size, words);
    let marks = (card.marks(words).into_iter()).map(|(label, value)| format!("{label}: {value}"));
    let mut parts = vec![card.name.value.clone(), date, camera];
    parts.extend(marks.map(Some));
    parts.extend([
        card.lens.value.clone(),
        Some(card.dimensions.clone()),
        Some(weight),
        iso,
        focal,
        card.strip[2].value.clone(),
        aperture,
        shutter,
    ]);
    parts.into_iter().flatten().collect::<Vec<_>>().join(", ")
}

/// ⋯'s own items in `words`: Copy Information, while there is a photo, and Show in
/// Metadata.
pub fn options(words: &Translator, mirror: &Mirror) -> Vec<Entry> {
    let copy = Item::new(COPY_INFORMATION, words.text(COPY_INFORMATION));
    vec![
        Entry::Item(copy.enabled(metadata::active_photo(mirror).is_some())),
        Entry::Item(Item::new(SHOW_IN_METADATA, words.text(SHOW_IN_METADATA))),
    ]
}

/// The Info tool, which keeps nothing: what it draws is the mirror's.
#[derive(Debug, Default)]
pub struct Info;

impl UtilityTool for Info {
    type Action = ();

    fn definition(&self) -> &'static UtilityDefinition {
        definition()
    }

    fn body(&mut self, widget: Widget, ui: &mut Ui, cx: &mut Context<'_>) -> Vec<()> {
        if widget == Widget::Info {
            let card = metadata::active_photo(cx.mirror).map(|photo| Card::of(photo, cx.words));
            show(ui, card.as_ref(), cx.words);
        }
        Vec::new()
    }
}

/// Draws `card`, or "No photo" where there is none.
fn show(ui: &mut Ui, card: Option<&Card>, words: &Translator) {
    let t = theme(ui.ctx());
    let Some(card) = card else {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), LINE), Sense::hover());
        let said = words.text("info-no-photo");
        let colour = t.colors.text_disabled;
        let job = truncated(&said, small(&t), rect.width(), colour);
        let galley = ui.painter().layout_job(job);
        let at = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, colour);
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &said));
        return;
    };
    let gap = t.spacing.m;
    let well_top = t.spacing.s;
    let height = 2.0 * LINE + gap + well_top + 3.0 * LINE + t.spacing.s + t.sizes.panel_line;
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let title = crate::tools::title(words, ID);
    response.widget_info(|| WidgetInfo::labeled(Role::Region, true, &title));
    let said = card.said.clone();
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_description(said));
    if response.clicked() {
        response.request_focus();
    }
    if response.has_focus() && copy_asked(ui) {
        ui.ctx().copy_text(card.copied.clone());
    }
    let mut lines = Lines {
        ui,
        id: response.id,
        t: &t,
        at: rect.top(),
        left: rect.left(),
        right: rect.right(),
        tips: 0,
    };
    let strong = FontId::new(t.text.body, FontFamily::Name(MEDIUM.into()));
    lines.text(&card.name, strong.clone(), t.colors.text_strong);
    lines.text(&card.date, small(&t), t.colors.text_secondary);
    // The well, as a field's.
    let well = Rect::from_min_max(pos2(rect.left(), lines.at + gap), rect.max);
    lines
        .ui
        .painter()
        .rect_filled(well, t.shape.radius, t.colors.sunken);
    let inset = t.spacing.m + t.spacing.xxs;
    lines.at = well.top() + well_top;
    lines.left = well.left() + inset;
    lines.right = well.right() - inset;
    lines.camera(card, strong);
    lines.text(&card.lens_line(words), small(&t), lens_colour(card, &t));
    lines.size(card);
    lines.strip(card, well);
    if response.has_focus() {
        focus_ring_centred(ui.painter(), rect, &t);
    }
}

impl Card {
    /// The lens line: the lens, or "No lens recorded".
    fn lens_line(&self, words: &Translator) -> Shown {
        Shown {
            value: Some(
                self.lens
                    .value
                    .clone()
                    .unwrap_or_else(|| words.text("info-no-lens")),
            ),
            tip: self.lens.tip.clone(),
        }
    }
}

fn lens_colour(card: &Card, t: &Theme) -> egui::Color32 {
    if card.lens.value.is_some() {
        t.colors.text
    } else {
        t.colors.text_disabled
    }
}

fn small(t: &Theme) -> FontId {
    FontId::proportional(t.text.small)
}

/// The card's lines as they are drawn, top to bottom, between `left` and `right`.
struct Lines<'u, 't> {
    ui: &'u mut Ui,
    /// The card's, each part with a tooltip a hover area of its own under it.
    id: egui::Id,
    t: &'t Theme,
    /// The next line's top.
    at: f32,
    left: f32,
    right: f32,
    tips: usize,
}

impl Lines<'_, '_> {
    /// The next line's rect, `height` high.
    fn next(&mut self, height: f32) -> Rect {
        let rect = Rect::from_min_max(pos2(self.left, self.at), pos2(self.right, self.at + height));
        self.at += height;
        rect
    }

    /// `text` in `colour` from `left`, at most `width`, cut at its end, centred on `line`;
    /// its right.
    fn paint(&self, text: &str, font: FontId, colour: egui::Color32, line: Rect, left: f32) -> f32 {
        let room = (line.right() - left).max(0.0);
        let galley = self
            .ui
            .painter()
            .layout_job(truncated(text, font, room, colour));
        let size = galley.size();
        self.ui
            .painter()
            .galley(pos2(left, line.center().y - size.y / 2.0), galley, colour);
        left + size.x
    }

    /// `tip` on hover over `rect`.
    fn tip(&mut self, rect: Rect, tip: &str) {
        self.tips += 1;
        if tip.is_empty() {
            return;
        }
        let id = self.id.with(("tip", self.tips));
        self.ui
            .interact(rect, id, Sense::hover())
            .on_hover_text(tip);
    }

    /// A line of `shown`, "—" in `text_disabled` for none.
    fn text(&mut self, shown: &Shown, font: FontId, colour: egui::Color32) {
        let line = self.next(LINE);
        let (text, colour) = match &shown.value {
            Some(value) => (value.as_str(), colour),
            None => ("—", self.t.colors.text_disabled),
        };
        let right = self.paint(text, font, colour, line, line.left());
        let over = Rect::from_min_max(line.min, pos2(right, line.max.y));
        self.tip(over, &shown.tip);
    }

    /// The camera, then at the line's end the white balance's tag and the flash.
    fn camera(&mut self, card: &Card, font: FontId) {
        let line = self.next(LINE);
        let gap = self.t.spacing.s;
        let mut end = line.right();
        if let Some((fired, tip)) = &card.flash {
            let (icon, colour) = match fired {
                Flash::Fired => (Icon::Flash, self.t.colors.text),
                Flash::NotFired => (Icon::FlashOff, self.t.colors.text_disabled),
            };
            let side = icon.size();
            let rect = Rect::from_min_size(
                pos2(end - side, line.center().y - side / 2.0),
                vec2(side, side),
            );
            icon.paint(self.ui.painter(), rect, colour);
            self.tip(rect, tip);
            end = rect.left() - gap;
        }
        if let Some((tag, tip)) = &card.white_balance {
            end = self.tag(tag, tip, line, end) - gap;
        }
        let ends = Rect::from_min_max(line.min, pos2(end, line.max.y));
        let right = self.paint(
            &card.camera,
            font,
            self.t.colors.text_strong,
            ends,
            line.left(),
        );
        self.tip(
            Rect::from_min_max(line.min, pos2(right, line.max.y)),
            &card.camera,
        );
    }

    /// The size and its weight, then the type's tag at the line's end.
    fn size(&mut self, card: &Card) {
        let line = self.next(LINE);
        let (tag, name) = &card.format;
        let end = self.tag(tag, name, line, line.right()) - self.t.spacing.s;
        let ends = Rect::from_min_max(line.min, pos2(end, line.max.y));
        let font = small(self.t);
        let right = self.paint(
            &card.dimensions,
            font.clone(),
            self.t.colors.text,
            ends,
            line.left(),
        );
        let space = self.t.spacing.xs;
        let right = self.paint(
            &card.weight,
            font,
            self.t.colors.text_secondary,
            ends,
            right + space,
        );
        let over = Rect::from_min_max(line.min, pos2(right.min(end), line.max.y));
        self.tip(over, &card.size_tip);
    }

    /// A tag ending at `end` on `line`; its left.
    fn tag(&mut self, text: &str, tip: &str, line: Rect, end: f32) -> f32 {
        let width = Tag::width(self.ui, text);
        let rect = Rect::from_min_max(pos2(end - width, line.top()), pos2(end, line.bottom()));
        self.ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                Tag::new(text).tip(tip).show(ui);
            },
        );
        rect.left()
    }

    /// The strip's five equal cells under a separator, each split from the next by it.
    fn strip(&mut self, card: &Card, well: Rect) {
        self.at += self.t.spacing.s;
        let top = self.at;
        let strip = Rect::from_min_max(
            pos2(well.left(), top),
            pos2(well.right(), top + self.t.sizes.panel_line),
        );
        let stroke = egui::Stroke::new(self.t.shape.hairline, self.t.colors.separator);
        let painter = self.ui.painter().clone();
        painter.hline(strip.x_range(), top, stroke);
        let width = strip.width() / STRIP.len() as f32;
        let font = small(self.t);
        let inset = self.t.spacing.xxs;
        for (at, shown) in card.strip.iter().enumerate() {
            let left = strip.left() + width * at as f32;
            let cell = Rect::from_min_max(pos2(left, top), pos2(left + width, strip.bottom()));
            if at > 0 {
                painter.vline(left, cell.y_range(), stroke);
            }
            let (text, colour) = match &shown.value {
                Some(value) => (value.as_str(), self.t.colors.text_strong),
                None => ("—", self.t.colors.text_disabled),
            };
            let room = (cell.width() - 2.0 * inset).max(0.0);
            let galley = painter.layout_job(truncated(text, font.clone(), room, colour));
            let size = galley.size();
            painter.galley(cell.center() - size / 2.0, galley, colour);
            self.tip(cell, &shown.tip);
        }
        self.at = strip.bottom();
    }
}

#[cfg(test)]
mod tests;
