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

//! The grid cell: one photo of the grid panel (`docs/gui.md`, The grid panel, The cell):
//! its picture in a square and, while the grid shows labels, a caption under it. It is
//! given a file's name, what the library knows of its edit, its rating and labels where
//! the library has them, and, once it has come, the photo's thumbnail, a texture the
//! caller made ([`CellPicture`]).
//!
//! - **The picture alone,** with no box or margin round it: fitted in the square, its
//!   shape kept, at the square's bottom centre, right above its caption; with square
//!   thumbnails it fills the square, cut at its longer sides. Until it comes, and for a
//!   photo with none, the file name stands in a box of the square's size.
//! - **The rings** hug the picture, 2 points out: a thin `frame_ring` for selected and a
//!   thicker one for the active photo, the icons' grey and never the accent. The focus
//!   ring is the usual one, outside them.
//! - **The corner marks,** always, labels on or off, each on a small scrim at the
//!   picture's bottom right: the pencil of an edited photo, and a lock for an edit a newer
//!   Apertrust wrote (view only) or a warning, in `warning_on_thumb_scrim`, for a sidecar
//!   that could not be read.
//! - **The caption** under the picture, while labels show: at Medium and larger 40
//!   points, 10 of them above its first line so the active ring keeps clear of it, its
//!   first line the reject mark, the rating's five stars and the colour labels' circle,
//!   and its second the file name, cut in its middle when too long, the active photo's in
//!   `text_strong`; at Small 20 points, the name alone. Without a rating ([`CellRating`],
//!   which the library has yet to keep), the first line is not drawn and the name takes its
//!   place.
//! - **The first line's clicks**, reported as a [`CellAction`]: a star rates the photo, the
//!   reject mark turns its reject on or off, and the circle opens the list of the five
//!   labels through the popup service (AppKit's menu on the Mac), each checked while on,
//!   with darktable's keys, and Clear labels.
//! - **To AccessKit** a list item named by the file name, its marks, rating, reject and
//!   labels in its description whatever the caption shows, selected when it is selected or
//!   active; the first line's stars, reject and circle are buttons inside it.

use egui::accesskit::HasPopup;
use egui::{
    Align, Color32, FontId, Id, Pos2, Rect, Response, Role, Sense, Shape, Stroke, StrokeKind, Ui,
    UiBuilder, WidgetInfo, pos2, text::LayoutJob, vec2,
};

use super::photo_frame::middle_cut;
use super::{focus_ring, pull_down};
use crate::i18n::Args;
use crate::popup::{Entry, Item};
use crate::{Icon, Theme, theme, tr, translator};

/// What a cell's status mark says: the photo opens, but not as an edit can change it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellStatus<'a> {
    /// A newer Apertrust wrote its edit: the photo opens for viewing only (decision 43).
    ViewOnly,
    /// The edit's sidecar could not be read or is no edit of Apertrust's, and why, in the
    /// library's words.
    Unreadable(&'a str),
}

/// A cell's picture: a texture the caller made of the photo's thumbnail, and its size in
/// pixels, whose shape the cell keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellPicture {
    pub texture: egui::TextureId,
    pub size: egui::Vec2,
}

/// One of darktable's colour labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColourLabel {
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl ColourLabel {
    /// In darktable's order, which the circle's segments and its list follow.
    pub const ALL: [ColourLabel; 5] = [
        ColourLabel::Red,
        ColourLabel::Yellow,
        ColourLabel::Green,
        ColourLabel::Blue,
        ColourLabel::Purple,
    ];

    /// Its id: its words are `colour-label-<id>`, its item in the list `label-<id>`.
    pub fn id(self) -> &'static str {
        match self {
            ColourLabel::Red => "red",
            ColourLabel::Yellow => "yellow",
            ColourLabel::Green => "green",
            ColourLabel::Blue => "blue",
            ColourLabel::Purple => "purple",
        }
    }

    /// Its colour in the theme.
    fn colour(self, t: &Theme) -> Color32 {
        let c = &t.colors;
        match self {
            ColourLabel::Red => c.label_red,
            ColourLabel::Yellow => c.label_yellow,
            ColourLabel::Green => c.label_green,
            ColourLabel::Blue => c.label_blue,
            ColourLabel::Purple => c.label_purple,
        }
    }

    /// darktable's key for it, F1 to F5, which its list shows.
    fn keys(self) -> &'static str {
        match self {
            ColourLabel::Red => "F1",
            ColourLabel::Yellow => "F2",
            ColourLabel::Green => "F3",
            ColourLabel::Blue => "F4",
            ColourLabel::Purple => "F5",
        }
    }

    fn index(self) -> usize {
        ColourLabel::ALL
            .iter()
            .position(|label| *label == self)
            .unwrap_or_default()
    }
}

/// The colour labels a photo has, any of the five.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ColourLabels(u8);

impl ColourLabels {
    /// The labels in `labels`.
    pub fn of(labels: impl IntoIterator<Item = ColourLabel>) -> Self {
        labels
            .into_iter()
            .fold(ColourLabels::default(), |set, label| set.with(label, true))
    }

    pub fn contains(self, label: ColourLabel) -> bool {
        self.0 & (1 << label.index()) != 0
    }

    /// The set with `label` on or off.
    pub fn with(self, label: ColourLabel, on: bool) -> Self {
        let bit = 1 << label.index();
        ColourLabels(if on { self.0 | bit } else { self.0 & !bit })
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The labels set, in darktable's order.
    pub fn iter(self) -> impl Iterator<Item = ColourLabel> {
        ColourLabel::ALL
            .into_iter()
            .filter(move |label| self.contains(*label))
    }
}

/// A photo's rating, its reject mark and its colour labels, as the caption's first line
/// shows them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellRating {
    /// Its stars, 0 to 5.
    pub stars: u8,
    pub rejected: bool,
    pub labels: ColourLabels,
}

/// What the user did on a cell's first line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellAction {
    /// A star was clicked: rate the photo so many stars, 1 to 5.
    Rate(u8),
    /// The reject mark was clicked: reject the photo, or take its reject back.
    ToggleReject,
    /// A label was chosen in the circle's list: turn it on, or off.
    ToggleLabel(ColourLabel),
    /// Clear labels was chosen in the circle's list.
    ClearLabels,
}

/// What a cell reported: its response, for its clicks and focus, and what the user did on
/// its first line.
pub struct GridCellOutput {
    pub response: Response,
    pub action: Option<CellAction>,
}

pub struct GridCell<'a> {
    name: &'a str,
    picture: Option<CellPicture>,
    side: f32,
    labels: bool,
    square: bool,
    rating: Option<CellRating>,
    edited: bool,
    status: Option<CellStatus<'a>>,
    selected: bool,
    active: bool,
}

/// The caption's height at Medium and larger, and at Small.
const CAPTION: f32 = 40.0;
const CAPTION_SMALL: f32 = 20.0;
/// Above the caption's first line, which the active ring, 5 points out, keeps clear of.
const CLEAR: f32 = 10.0;
/// The first line's marks.
const MARK: f32 = 12.0;
/// Between the first line and the name, and between the stars.
const STEP: f32 = 2.0;
/// From the picture to its rings.
const RING_GAP: f32 = 2.0;
/// A corner mark's scrim, and its inset from the picture's corner.
const CHIP: f32 = 16.0;

impl GridCell<'_> {
    /// The Medium size of the design, in points.
    pub const MEDIUM: f32 = 120.0;

    /// The caption's height under a square of `side` points while `labels` show: 40 points
    /// at Medium and larger, 20 at Small, which shows the name alone.
    pub fn caption_height(side: f32, labels: bool) -> f32 {
        if !labels {
            0.0
        } else if side < Self::MEDIUM {
            CAPTION_SMALL
        } else {
            CAPTION
        }
    }
}

impl<'a> GridCell<'a> {
    /// A cell for the photo whose file is `name`, with its extension.
    pub fn new(name: &'a str) -> Self {
        GridCell {
            name,
            picture: None,
            side: Self::MEDIUM,
            labels: true,
            square: false,
            rating: None,
            edited: false,
            status: None,
            selected: false,
            active: false,
        }
    }

    /// The photo's thumbnail, once it has come; the file name stands in for it until then.
    pub fn picture(mut self, picture: Option<CellPicture>) -> Self {
        self.picture = picture;
        self
    }

    /// The side of the square, in points.
    pub fn side(mut self, side: f32) -> Self {
        self.side = side;
        self
    }

    /// Whether the caption shows under the square; on by default.
    pub fn labels(mut self, labels: bool) -> Self {
        self.labels = labels;
        self
    }

    /// Whether the picture fills its square, cut at its longer sides, rather than fitting
    /// in it; off by default.
    pub fn square(mut self, square: bool) -> Self {
        self.square = square;
        self
    }

    /// The photo's rating, reject and colour labels, where the library has them: the
    /// caption's first line is drawn only with them.
    pub fn rating(mut self, rating: Option<CellRating>) -> Self {
        self.rating = rating;
        self
    }

    /// Whether the photo has an edit.
    pub fn edited(mut self, edited: bool) -> Self {
        self.edited = edited;
        self
    }

    /// The status mark, if the photo has one.
    pub fn status(mut self, status: Option<CellStatus<'a>>) -> Self {
        self.status = status;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The photo being edited: a thicker ring, and its name brighter.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The cell's size: its square and the caption under it.
    pub fn size(&self) -> egui::Vec2 {
        vec2(
            self.side,
            self.side + Self::caption_height(self.side, self.labels),
        )
    }

    /// Draws the cell in the next free space of `ui`.
    pub fn show(self, ui: &mut Ui) -> GridCellOutput {
        let (rect, _) = ui.allocate_exact_size(self.size(), Sense::hover());
        let id = ui.make_persistent_id(("grid cell", self.name));
        self.show_at(ui, rect, id)
    }

    /// Draws the cell in `rect`, of its [`GridCell::size`], as the widget `id`.
    pub fn show_at(self, ui: &mut Ui, rect: Rect, id: Id) -> GridCellOutput {
        let t = theme(ui.ctx());
        let response = ui.interact(rect, id, Sense::click());
        // The node exists before the first line's buttons, so they hang under it.
        let selected = self.selected || self.active;
        response.widget_info(|| WidgetInfo::selected(Role::ListItem, true, selected, self.name));
        let painter = ui.painter_at(rect.expand(8.0));
        let square = Rect::from_min_size(rect.min, vec2(self.side, self.side));

        // The picture, or the file name in a box of the square's size.
        let picture = match self.picture.filter(|p| p.size.x > 0.0 && p.size.y > 0.0) {
            Some(picture) => {
                let (place, uv) = placed(square, picture.size, self.square);
                painter.image(picture.texture, place, uv, Color32::WHITE);
                place
            }
            None => {
                let ground = if response.hovered() {
                    t.colors.hover
                } else {
                    t.colors.sunken
                };
                painter.rect_filled(square, t.shape.radius_small, ground);
                let mut job = LayoutJob::simple(
                    self.name.to_owned(),
                    FontId::proportional(t.text.caption),
                    t.colors.text_secondary,
                    square.width() - 2.0 * t.spacing.xs,
                );
                job.wrap.max_rows = 3;
                job.wrap.break_anywhere = true;
                job.halign = Align::Center;
                let name = painter.layout_job(job);
                // Centred text is laid out about its position's x.
                painter.galley(
                    pos2(square.center().x, square.center().y - name.size().y / 2.0),
                    name,
                    t.colors.text_secondary,
                );
                square
            }
        };

        // The corner marks, from the picture's bottom right: the status, then the pencil.
        let marks = [
            self.status.map(|status| match status {
                CellStatus::ViewOnly => (Icon::Lock, t.colors.text_on_thumb_scrim),
                CellStatus::Unreadable(_) => (Icon::Warning, t.colors.warning_on_thumb_scrim),
            }),
            self.edited
                .then_some((Icon::Edited, t.colors.text_on_thumb_scrim)),
        ];
        let mut right = picture.right() - t.spacing.xs;
        for (icon, colour) in marks.into_iter().flatten() {
            let chip = Rect::from_min_max(
                pos2(right - CHIP, picture.bottom() - t.spacing.xs - CHIP),
                pos2(right, picture.bottom() - t.spacing.xs),
            );
            painter.rect_filled(chip, t.shape.radius_small, t.colors.thumb_scrim);
            icon.paint(&painter, chip.shrink(2.0), colour);
            right -= CHIP + STEP;
        }

        // The rings round the picture: a thin one for selected, a thicker for the active.
        let ring = if self.active {
            3.0
        } else if self.selected {
            1.0
        } else {
            0.0
        };
        if ring > 0.0 {
            painter.rect_stroke(
                picture.expand(RING_GAP),
                t.shape.radius_small,
                Stroke::new(ring, t.colors.frame_ring),
                StrokeKind::Outside,
            );
        }
        if response.has_focus() {
            focus_ring(&painter, picture.expand(RING_GAP + ring + 2.0), &t);
        }

        // The caption: the first line, where the cell has a rating, and the name.
        let caption = Self::caption_height(self.side, self.labels);
        let strip = Rect::from_min_size(square.left_bottom(), vec2(self.side, caption));
        let mut action = None;
        if caption == CAPTION_SMALL {
            self.paint_name(&painter, &t, strip, None);
        } else if caption > 0.0 {
            let line = Rect::from_min_size(
                pos2(strip.left() + t.spacing.xs, strip.top() + CLEAR),
                vec2(self.side - 2.0 * t.spacing.xs, MARK),
            );
            match self.rating {
                Some(rating) => {
                    let build = UiBuilder::new().max_rect(strip).accessibility_parent(id);
                    let mut child = ui.new_child(build);
                    action = first_line(&mut child, &painter, &t, id, line, rating);
                    self.paint_name(&painter, &t, strip, Some(line.bottom() + STEP));
                }
                None => self.paint_name(&painter, &t, strip, Some(line.top())),
            }
        }

        // What screen readers and the tooltip say of the marks.
        let notes = self.notes(ui);
        ui.ctx().accesskit_node_builder(response.id, |node| {
            if !notes.is_empty() {
                node.set_description(notes.clone());
            }
        });
        let tip = if notes.is_empty() {
            self.name.to_owned()
        } else {
            format!("{}\n{notes}", self.name)
        };
        GridCellOutput {
            response: response.on_hover_text(tip),
            action,
        }
    }

    /// The file name under the picture, cut in its middle to the caption's width: at `top`
    /// in a caption of two lines, or at the bottom of a Small one's.
    fn paint_name(&self, painter: &egui::Painter, t: &Theme, strip: Rect, top: Option<f32>) {
        let font = match top {
            Some(_) => FontId::proportional(t.text.small),
            None => FontId::proportional(t.text.caption),
        };
        let colour = if self.active {
            t.colors.text_strong
        } else {
            t.colors.text
        };
        let text = fitted(painter, self.name, &font, strip.width());
        let galley = painter.layout_no_wrap(text, font, colour);
        let y = top.unwrap_or(strip.bottom() - galley.size().y);
        let at = pos2(strip.center().x - galley.size().x / 2.0, y);
        painter.galley(at, galley, colour);
    }

    /// The marks, the rating, the reject and the labels, in words.
    fn notes(&self, ui: &Ui) -> String {
        let tr = translator(ui.ctx());
        let mut notes = Vec::new();
        if self.edited {
            notes.push(tr.text("grid-cell-edited"));
        }
        match self.status {
            Some(CellStatus::ViewOnly) => notes.push(tr.text("grid-cell-view-only")),
            Some(CellStatus::Unreadable(reason)) => {
                let mut args = Args::new();
                args.set("reason", reason);
                notes.push(tr.text_with("grid-cell-unreadable", Some(&args)));
            }
            None => {}
        }
        if let Some(rating) = self.rating {
            if rating.rejected {
                notes.push(tr.text("grid-cell-rejected"));
            } else {
                let mut args = Args::new();
                args.set("count", rating.stars);
                notes.push(tr.text_with("grid-cell-rating", Some(&args)));
            }
            if !rating.labels.is_empty() {
                let mut args = Args::new();
                args.set("labels", label_names(ui.ctx(), rating.labels));
                notes.push(tr.text_with("grid-cell-labels", Some(&args)));
            }
        }
        notes.join(". ")
    }
}

/// Where a picture of `size` pixels lies in `square`, and the part of it drawn: fitted,
/// its shape kept, at the bottom centre; or, `filling` it, all of the square, cut evenly
/// at its longer sides.
fn placed(square: Rect, size: egui::Vec2, filling: bool) -> (Rect, Rect) {
    let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    let aspect = size.x / size.y;
    if filling {
        let uv = if aspect > 1.0 {
            let cut = (1.0 - 1.0 / aspect) / 2.0;
            Rect::from_min_max(pos2(cut, 0.0), pos2(1.0 - cut, 1.0))
        } else {
            let cut = (1.0 - aspect) / 2.0;
            Rect::from_min_max(pos2(0.0, cut), pos2(1.0, 1.0 - cut))
        };
        return (square, uv);
    }
    let scale = (square.width() / size.x).min(square.height() / size.y);
    let fitted = size * scale;
    let min = pos2(
        square.center().x - fitted.x / 2.0,
        square.bottom() - fitted.y,
    );
    (Rect::from_min_size(min, fitted), whole)
}

/// `name` whole where it fits in `width`, else cut in its middle, its extension kept.
fn fitted(painter: &egui::Painter, name: &str, font: &FontId, width: f32) -> String {
    let (stem, tail) = match name.rfind('.') {
        Some(dot) if dot > 0 => name.split_at(dot),
        _ => (name, ""),
    };
    middle_cut(painter, stem, tail, font, width)
}

/// The labels' names, translated, in darktable's order.
fn label_names(ctx: &egui::Context, labels: ColourLabels) -> String {
    labels
        .iter()
        .map(|label| tr(ctx, &format!("colour-label-{}", label.id())))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The caption's first line in `line`: the reject mark at its start, the five stars in its
/// middle and the labels' circle at its end, each a button of the cell `cell`. Returns
/// what was clicked or chosen.
fn first_line(
    ui: &mut Ui,
    painter: &egui::Painter,
    t: &Theme,
    cell: Id,
    line: Rect,
    rating: CellRating,
) -> Option<CellAction> {
    let ctx = ui.ctx().clone();
    let words = translator(&ctx);
    let mut action = None;
    // Buttons for the pointer and screen readers, but no Tab stops: the keys rate.
    let button = |ui: &mut Ui, rect: Rect, id: Id| ui.interact(rect, id, Sense::CLICK);

    // Reject: outlined at rest, in the error colour while rejected.
    let reject = Rect::from_min_size(line.min, vec2(MARK, MARK));
    let pressed = button(ui, reject, cell.with("reject"));
    let colour = if rating.rejected {
        t.colors.error
    } else if pressed.hovered() {
        t.colors.text
    } else {
        t.colors.text_disabled
    };
    Icon::Reject.paint(painter, reject, colour);
    let name = tr(&ctx, "grid-cell-reject");
    pressed.widget_info(|| WidgetInfo::selected(Role::Button, true, rating.rejected, &name));
    if pressed.clicked() {
        action = Some(CellAction::ToggleReject);
    }

    // The stars: filled for each point, those up to one under the pointer lit, all dimmed
    // while the photo is rejected.
    let stars = 5.0 * MARK + 4.0 * STEP;
    let start = line.center().x - stars / 2.0;
    let rects: Vec<Rect> = (0..5)
        .map(|i| {
            Rect::from_min_size(
                pos2(start + i as f32 * (MARK + STEP), line.top()),
                vec2(MARK, MARK),
            )
        })
        .collect();
    let responses: Vec<Response> = rects
        .iter()
        .enumerate()
        .map(|(i, rect)| button(ui, *rect, cell.with(("star", i))))
        .collect();
    let hovered = responses.iter().position(Response::hovered);
    for (i, (rect, star)) in rects.iter().zip(&responses).enumerate() {
        let points = i as u8 + 1;
        let lit = hovered.is_some_and(|h| i <= h);
        let filled = lit || (hovered.is_none() && points <= rating.stars);
        let colour = if rating.rejected {
            t.colors.text_disabled
        } else if lit {
            t.colors.text
        } else {
            t.colors.text_secondary
        };
        let icon = if filled { Icon::StarFilled } else { Icon::Star };
        icon.paint(painter, *rect, colour);
        let mut args = Args::new();
        args.set("count", points);
        let name = words.text_with("grid-cell-rate", Some(&args));
        let on = points <= rating.stars;
        star.widget_info(|| WidgetInfo::selected(Role::Button, true, on, &name));
        if star.clicked() {
            action = Some(CellAction::Rate(points));
        }
    }

    // The colour labels' circle, which opens their list.
    let circle = Rect::from_min_size(pos2(line.right() - MARK, line.top()), vec2(MARK, MARK));
    let opener = button(ui, circle, cell.with("labels"));
    let name = tr(&ctx, "grid-cell-colour-labels");
    // Outlined on hover only: while its list is open, the list itself shows what is open.
    let open = crate::popup::is_open(&ctx, crate::popup::button_menu(opener.id));
    paint_labels(painter, t, circle, rating.labels);
    if opener.hovered() && !open {
        painter.circle_stroke(
            circle.center(),
            MARK / 2.0 + 2.0,
            Stroke::new(1.5, t.colors.frame_ring),
        );
    }
    let value = if rating.labels.is_empty() {
        tr(&ctx, "grid-cell-no-labels")
    } else {
        label_names(&ctx, rating.labels)
    };
    opener.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
    ctx.accesskit_node_builder(opener.id, |node| {
        node.set_value(value.clone());
        node.set_has_popup(HasPopup::Menu);
    });
    let chosen = pull_down(ui, &opener, &name, || {
        let mut entries: Vec<Entry> = ColourLabel::ALL
            .into_iter()
            .map(|label| {
                let words = tr(&ctx, &format!("colour-label-{}", label.id()));
                Item::new(format!("label-{}", label.id()), words)
                    .checked(rating.labels.contains(label))
                    .shortcut(label.keys())
                    .into()
            })
            .collect();
        entries.push(Entry::Separator);
        entries.push(Item::new("clear-labels", tr(&ctx, "grid-cell-clear-labels")).into());
        entries
    });
    match chosen.as_deref() {
        Some("clear-labels") => action = Some(CellAction::ClearLabels),
        Some(id) => {
            if let Some(label) = ColourLabel::ALL
                .into_iter()
                .find(|label| id.strip_prefix("label-") == Some(label.id()))
            {
                action = Some(CellAction::ToggleLabel(label));
            }
        }
        None => {}
    }
    action
}

/// The colour labels as one circle in `rect`: an empty ring for none, filled by one, split
/// into equal segments for several, in darktable's order clockwise from the top.
fn paint_labels(painter: &egui::Painter, t: &Theme, rect: Rect, labels: ColourLabels) {
    let centre = rect.center();
    let radius = rect.width() / 2.0;
    let set: Vec<ColourLabel> = labels.iter().collect();
    match set.as_slice() {
        [] => {
            painter.circle_stroke(
                centre,
                radius - 0.5,
                Stroke::new(1.0, t.colors.text_disabled),
            );
        }
        [label] => {
            painter.circle_filled(centre, radius, label.colour(t));
        }
        several => {
            let share = std::f32::consts::TAU / several.len() as f32;
            for (i, label) in several.iter().enumerate() {
                let from = i as f32 * share;
                let mut points = vec![centre];
                // Enough points for the arc to look round at any scale.
                let steps = 24 / several.len();
                for step in 0..=steps {
                    let angle = from + share * step as f32 / steps as f32;
                    points.push(at(centre, radius, angle));
                }
                painter.add(Shape::convex_polygon(points, label.colour(t), Stroke::NONE));
            }
        }
    }
}

/// The point `radius` from `centre` at `angle` clockwise from the top.
fn at(centre: Pos2, radius: f32, angle: f32) -> Pos2 {
    centre + radius * vec2(angle.sin(), -angle.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fitted_picture_sits_at_the_square_s_bottom_centre_and_a_filling_one_is_cut_evenly() {
        let square = Rect::from_min_size(pos2(10.0, 20.0), vec2(120.0, 120.0));
        // A landscape picture is as wide as the square, on its bottom edge.
        let (place, uv) = placed(square, vec2(300.0, 200.0), false);
        assert_eq!(
            place,
            Rect::from_min_max(pos2(10.0, 60.0), pos2(130.0, 140.0))
        );
        assert_eq!(uv, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)));
        // A portrait one is as tall as the square, centred across it.
        let (place, _) = placed(square, vec2(200.0, 300.0), false);
        assert_eq!(
            place,
            Rect::from_min_max(pos2(30.0, 20.0), pos2(110.0, 140.0))
        );
        // Filling the square, a 3:2 picture loses a sixth at each end.
        let (place, uv) = placed(square, vec2(300.0, 200.0), true);
        assert_eq!(place, square);
        assert!((uv.min.x - 1.0 / 6.0).abs() < 1e-6 && (uv.max.x - 5.0 / 6.0).abs() < 1e-6);
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
        let (_, uv) = placed(square, vec2(200.0, 300.0), true);
        assert!((uv.min.y - 1.0 / 6.0).abs() < 1e-6, "{uv:?}");
    }

    #[test]
    fn the_caption_is_40_points_from_medium_up_20_at_small_and_none_without_labels() {
        assert_eq!(GridCell::caption_height(84.0, true), 20.0);
        assert_eq!(GridCell::caption_height(120.0, true), 40.0);
        assert_eq!(GridCell::caption_height(256.0, true), 40.0);
        assert_eq!(GridCell::caption_height(120.0, false), 0.0);
    }

    #[test]
    fn colour_labels_keep_darktable_s_order() {
        use ColourLabel::*;
        let set = ColourLabels::of([Purple, Red, Green]);
        assert_eq!(set.iter().collect::<Vec<_>>(), [Red, Green, Purple]);
        assert!(set.contains(Green) && !set.contains(Blue));
        assert_eq!(set.with(Green, false), ColourLabels::of([Red, Purple]));
        assert!(ColourLabels::default().is_empty());
    }
}
