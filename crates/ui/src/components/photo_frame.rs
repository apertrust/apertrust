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

//! The photo frame (`docs/gui.md`, The photo frame): how the Viewer shows a photo
//! and its caption, for looking and comparing. Nothing is drawn on the photo.
//!
//! **The slot and the block.** The Viewer's area under the viewer bar is the
//! photo's slot. The photo and its caption are one block in it, laid out by [`PhotoFrame`]:
//!
//! 1. the slot loses the margin on every side;
//! 2. the caption's height, [`CAPTION_HEIGHT`] and the [`CAPTION_GAP`] above it, counts
//!    inside what is left;
//! 3. the photo is fitted at its own aspect in the rest, by whoever renders it: the room
//!    is [`PhotoFrame::bounds`];
//! 4. the caption is exactly as wide as the photo;
//! 5. the block is centred in the slot.
//!
//! **The margin** is [`Margin::Points`], the default, or [`Margin::Proof`] while Proof
//! margin is on: a fraction of the slot's smaller side, darktable's colour assessment
//! border as `dt_dev_configure` sizes it, all of it the surround. darktable paints its
//! inner part white; Apertrust paints no white, so Proof margin only gives the photo more
//! room round it.
//!
//! **The caption** ([`PhotoCaption`]) is one line, never wrapped: the file name with its
//! marks at the start, the centre for the reject, the rating and the colour labels, which
//! the person sets there ([`PhotoCaption::rating`], reported as a [`CellAction`]), the
//! exposure values at the end. It drops parts as the photo narrows ([`fit`]), folding the
//! labels into dots and the stars into one star and the number before the name is cut.

use egui::{
    Color32, FontFamily, FontId, Key, Painter, Pos2, Rect, Response, Role, Sense, Stroke, Ui, Vec2,
    WidgetInfo, emath::GuiRounding as _, pos2, vec2,
};

use super::focus_ring;
use super::grid_cell::{CellAction, CellRating, ColourLabel};
use crate::i18n::Args;
use crate::theme::{Theme, contrast};
use crate::{Icon, fonts, theme, tr, translator};

/// The caption's height, in points.
pub const CAPTION_HEIGHT: f32 = 20.0;
/// The gap between the photo and its caption.
pub const CAPTION_GAP: f32 = 4.0;
/// The least stem of a file name, in characters, that the caption still shows cut.
const LEAST_STEM: usize = 5;
/// The hit target of Reject and of each star, in points, with no gap between them.
const TARGET: f32 = 20.0;
/// The hit target of each colour label.
const LABEL_TARGET: f32 = 18.0;
/// A label's circle's radius, and the stroke of its ring while it is off.
const LABEL_RADIUS: f32 = 5.5;
/// A folded label's dot's diameter, and the space between two.
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 3.0;
/// The least contrast a label's colour, or a control's outline, needs on the surround.
const CONTROL: f32 = 3.0;

/// The margin round a block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Margin {
    /// The default margin, in points.
    Points(f32),
    /// Proof margin's: a fraction of the slot's smaller side.
    Proof(f32),
}

/// Where a photo and its caption go in a slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhotoFrame {
    slot: Rect,
    /// The margin round the block, in points.
    margin: f32,
}

impl PhotoFrame {
    /// The frame of `slot` under `margin`.
    pub fn new(slot: Rect, margin: Margin) -> Self {
        let margin = match margin {
            Margin::Points(points) => points.max(0.0),
            Margin::Proof(fraction) => {
                let side = slot.width().min(slot.height()).max(0.0);
                (fraction.max(0.0) * side).round()
            }
        };
        PhotoFrame { slot, margin }
    }

    /// The margin round the block, in points: Proof margin's worked out for the slot.
    pub fn margin(&self) -> f32 {
        self.margin
    }

    /// The largest box the photo may take, for whoever fits it at its aspect: the slot
    /// less the margin on every side and the caption.
    pub fn bounds(&self) -> Vec2 {
        let around = 2.0 * self.margin;
        vec2(
            (self.slot.width() - around).max(0.0),
            (self.slot.height() - around - CAPTION_GAP - CAPTION_HEIGHT).max(0.0),
        )
    }

    /// The block for a photo drawn at `photo` (which fits in [`bounds`](Self::bounds)),
    /// centred in the slot with the photo's corner on the pixel grid at `ppp`.
    pub fn place(&self, photo: Vec2, ppp: f32) -> Placed {
        let block = vec2(photo.x, photo.y + CAPTION_GAP + CAPTION_HEIGHT);
        let corner = Rect::from_center_size(self.slot.center(), block).min;
        let photo = Rect::from_min_size(corner.round_to_pixels(ppp), photo);
        let caption = Rect::from_min_size(
            pos2(photo.left(), photo.bottom() + CAPTION_GAP),
            vec2(photo.width(), CAPTION_HEIGHT),
        );
        Placed { photo, caption }
    }
}

/// A placed block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// Where the photo is drawn.
    pub photo: Rect,
    /// The caption's line, as wide as the photo.
    pub caption: Rect,
}

/// The measured widths of a caption's parts, in points, which [`fit`] chooses by.
#[derive(Clone, Debug, PartialEq)]
pub struct Widths {
    /// The file name whole, with its copy.
    pub name: f32,
    /// The file name cut as far as the caption cuts it.
    pub name_least: f32,
    /// The view-only mark with its word, and as its icon alone; 0 where there is no mark.
    pub mark_word: f32,
    pub mark_icon: f32,
    /// Each exposure value, in the order they drop from the last.
    pub exposure: Vec<f32>,
    /// The centre's width (reject, rating and labels) whole, with the labels folded into
    /// dots, and with the stars folded too ([`CaptionCentre`]); 0 while it is empty.
    pub centre: [f32; 3],
    /// The gap between the exposure values and between a name and its mark.
    pub gap: f32,
    /// The least gap between the start, the centre and the end.
    pub apart: f32,
}

/// How a name is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionName {
    Whole,
    /// Cut in its middle, keeping its extension and copy.
    Cut,
    Gone,
}

/// How the view-only mark is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionMark {
    Word,
    Icon,
}

/// How the centre is shown (`docs/gui.md`, The caption under the photo, Narrowing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionCentre {
    /// Reject, the five stars and the five labels, each a control.
    Whole,
    /// The labels folded into the dots of those on, read only.
    Labels,
    /// The stars folded too, into one star and the number, ⊗ before it while rejected,
    /// read only.
    Folded,
}

impl CaptionCentre {
    fn index(self) -> usize {
        match self {
            CaptionCentre::Whole => 0,
            CaptionCentre::Labels => 1,
            CaptionCentre::Folded => 2,
        }
    }
}

/// What a caption shows at a width: the parts that survived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptionFit {
    /// How many exposure values, from the first.
    pub exposure: usize,
    pub mark: CaptionMark,
    pub centre: CaptionCentre,
    pub name: CaptionName,
}

impl Widths {
    fn exposure_width(&self, shown: usize) -> f32 {
        match shown {
            0 => 0.0,
            n => self.exposure[..n].iter().sum::<f32>() + self.gap * (n - 1) as f32,
        }
    }

    fn start_width(&self, step: CaptionFit) -> f32 {
        let name = match step.name {
            CaptionName::Whole => self.name,
            CaptionName::Cut => self.name_least,
            CaptionName::Gone => 0.0,
        };
        let mark = match step.mark {
            CaptionMark::Word => self.mark_word,
            CaptionMark::Icon => self.mark_icon,
        };
        match (name > 0.0, mark > 0.0) {
            (true, true) => name + self.gap + mark,
            _ => name + mark,
        }
    }

    /// The width a caption needs for `step`. The centre stays centred, so each side gets
    /// half of what it leaves and the wider counts twice; with the centre empty the start
    /// and the end share the line.
    fn needed(&self, step: CaptionFit) -> f32 {
        let (start, end) = (self.start_width(step), self.exposure_width(step.exposure));
        let centre = self.centre[step.centre.index()];
        if centre > 0.0 {
            2.0 * start.max(end) + centre + 2.0 * self.apart
        } else if start > 0.0 && end > 0.0 {
            start + end + self.apart
        } else {
            start + end
        }
    }
}

/// What a caption of `available` points shows, dropping parts in the spec's order, one
/// step at a time: the exposure values from the last (ISO, then focal length, aperture and
/// shutter), the mark's word for its icon, the labels folded into dots, the stars into one
/// star and the number, then the name cut in its middle, then the name. The folded rating
/// and labels never go: they are what culling looks at.
pub fn fit(widths: &Widths, available: f32) -> CaptionFit {
    let step = |mark, centre, name| CaptionFit {
        exposure: 0,
        mark,
        centre,
        name,
    };
    let steps = (0..=widths.exposure.len())
        .rev()
        .map(|exposure| CaptionFit {
            exposure,
            mark: CaptionMark::Word,
            centre: CaptionCentre::Whole,
            name: CaptionName::Whole,
        })
        .chain([
            step(CaptionMark::Icon, CaptionCentre::Whole, CaptionName::Whole),
            step(CaptionMark::Icon, CaptionCentre::Labels, CaptionName::Whole),
            step(CaptionMark::Icon, CaptionCentre::Folded, CaptionName::Whole),
            step(CaptionMark::Icon, CaptionCentre::Folded, CaptionName::Cut),
            step(CaptionMark::Icon, CaptionCentre::Folded, CaptionName::Gone),
        ]);
    let mut last = None;
    for step in steps {
        if widths.needed(step) <= available {
            return step;
        }
        last = Some(step);
    }
    last.expect("the steps are never empty")
}

/// `stem` and `tail` together where they fit in `width` in `font`, else `stem` cut in its
/// middle with … and `tail` kept whole after it (`docs/gui.md`, Names), as a file name
/// keeps its extension. At worst one character of `stem` is left.
pub fn middle_cut(painter: &Painter, stem: &str, tail: &str, font: &FontId, width: f32) -> String {
    let fits = |text: &str| {
        painter
            .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
            .size()
            .x
            <= width
    };
    let whole = format!("{stem}{tail}");
    if fits(&whole) {
        return whole;
    }
    (1..stem.chars().count())
        .rev()
        .map(|chars| cut(stem, tail, chars))
        .find(|text| fits(text))
        .unwrap_or_else(|| cut(stem, tail, 1))
}

/// `stem` cut in its middle to `chars` characters, an ellipsis among them, with `tail`
/// (the extension and the copy) kept after it; `stem` as it is when it has no more.
pub(super) fn cut(stem: &str, tail: &str, chars: usize) -> String {
    let count = stem.chars().count();
    if count <= chars {
        return format!("{stem}{tail}");
    }
    let kept = chars.saturating_sub(1);
    let head = kept.div_ceil(2);
    let end = kept - head;
    let first: String = stem.chars().take(head).collect();
    let last: String = stem.chars().skip(count - end).collect();
    format!("{first}…{last}{tail}")
}

/// The tooltips of the caption's controls, which name what a click does and its key
/// (`docs/gui.md`, The caption under the photo): the caller's, who knows the keys and how
/// many photos the action takes; [`CaptionTips::plain`] has the words alone.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaptionTips {
    /// Reject's while the photo is not rejected, and while it is.
    pub reject: String,
    pub unreject: String,
    /// The nth star's, which rates n.
    pub stars: [String; 5],
    /// The current rating's star's, which clears it.
    pub clear: String,
    /// Each label's, in darktable's order.
    pub labels: [String; 5],
}

impl CaptionTips {
    /// The words alone, with no key.
    pub fn plain(ctx: &egui::Context) -> Self {
        Self::plain_with(&translator(ctx))
    }

    /// The words alone in `words`' language.
    pub fn plain_with(words: &crate::Translator) -> Self {
        let stars = std::array::from_fn(|i| {
            let mut args = Args::new();
            args.set("count", i + 1);
            words.text_with("grid-cell-rate", Some(&args))
        });
        let labels =
            ColourLabel::ALL.map(|label| words.text(&format!("caption-label-{}", label.id())));
        CaptionTips {
            reject: words.text("caption-reject"),
            unreject: words.text("caption-unreject"),
            stars,
            clear: words.text("caption-clear-rating"),
            labels,
        }
    }
}

/// What a caption reported: its response, the whole line's, and what the user did on its
/// centre's controls.
pub struct CaptionOutput {
    pub response: Response,
    pub action: Option<CellAction>,
}

/// A photo's caption: one line under it, as wide as the photo.
pub struct PhotoCaption<'a> {
    name: &'a str,
    copy: Option<u32>,
    view_only: bool,
    exposure: &'a [String],
    details: &'a str,
    rating: Option<CellRating>,
    tips: Option<&'a CaptionTips>,
}

impl<'a> PhotoCaption<'a> {
    /// The caption of the photo whose file is `name`.
    pub fn new(name: &'a str) -> Self {
        PhotoCaption {
            name,
            copy: None,
            view_only: false,
            exposure: &[],
            details: "",
            rating: None,
            tips: None,
        }
    }

    /// The photo's rating, reject and labels, which its centre shows and sets; `None`
    /// leaves the centre empty.
    pub fn rating(mut self, rating: Option<CellRating>) -> Self {
        self.rating = rating;
        self
    }

    /// Its controls' tooltips; [`CaptionTips::plain`] where none are given.
    pub fn tips(mut self, tips: &'a CaptionTips) -> Self {
        self.tips = Some(tips);
        self
    }

    /// The virtual copy's number, where the photo is one of several.
    pub fn copy(mut self, copy: Option<u32>) -> Self {
        self.copy = copy;
        self
    }

    /// Whether the photo is open for viewing only: the mark after the name.
    pub fn view_only(mut self, view_only: bool) -> Self {
        self.view_only = view_only;
        self
    }

    /// The exposure values, already written in the language, in the order they drop from
    /// the last: shutter, aperture, focal length, ISO.
    pub fn exposure(mut self, exposure: &'a [String]) -> Self {
        self.exposure = exposure;
        self
    }

    /// More that the tooltip and screen readers say, such as the camera and why the
    /// photo is open for viewing only.
    pub fn details(mut self, details: &'a str) -> Self {
        self.details = details;
        self
    }

    /// The name with its copy, whole: what screen readers and the tooltip hold.
    fn whole(&self) -> String {
        let (stem, tail) = self.split();
        format!("{stem}{tail}")
    }

    /// The name's stem, and its extension and copy.
    fn split(&self) -> (&str, String) {
        let (stem, extension) = match self.name.rfind('.') {
            Some(dot) if dot > 0 => self.name.split_at(dot),
            _ => (self.name, ""),
        };
        let copy = self.copy.map(|n| format!(" [{n}]")).unwrap_or_default();
        (stem, format!("{extension}{copy}"))
    }

    /// The caption's fonts: the name's in SemiBold, as the active photo's, and the values'.
    fn fonts(t: &Theme) -> (FontId, FontId) {
        (
            FontId::new(t.text.small, FontFamily::Name(fonts::SEMIBOLD.into())),
            FontId::proportional(t.text.caption),
        )
    }

    /// The widths of the caption's parts as `painter` draws them.
    fn measure(&self, painter: &Painter, t: &Theme, word: &str) -> Widths {
        let (name_font, value_font) = Self::fonts(t);
        let width_of = |text: &str, font: &FontId| {
            painter
                .layout_no_wrap(text.to_owned(), font.clone(), t.colors.text_on_surround())
                .size()
                .x
        };
        let (stem, tail) = self.split();
        let icon = Icon::Lock.size();
        let (mark_word, mark_icon) = if self.view_only {
            (icon + t.spacing.xs + width_of(word, &name_font), icon)
        } else {
            (0.0, 0.0)
        };
        Widths {
            name: width_of(&self.whole(), &name_font),
            name_least: width_of(&cut(stem, &tail, LEAST_STEM), &name_font),
            mark_word,
            mark_icon,
            exposure: self
                .exposure
                .iter()
                .map(|text| width_of(text, &value_font))
                .collect(),
            centre: self.rating.map_or([0.0; 3], |rating| {
                let number = width_of(&rating.stars.to_string(), &value_font);
                centre_widths(t, rating, number)
            }),
            gap: t.spacing.m,
            apart: t.spacing.l,
        }
    }

    /// What the caption shows in a line `width` points wide.
    pub fn fitted(&self, ui: &Ui, width: f32) -> CaptionFit {
        let t = theme(ui.ctx());
        let word = tr(ui.ctx(), "status-view-only");
        fit(&self.measure(ui.painter(), &t, &word), width)
    }

    /// Draws the caption in `rect`, a line [`PhotoFrame::place`] gave.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> CaptionOutput {
        let t = theme(ui.ctx());
        let color = t.colors.text_on_surround();
        let (name_font, value_font) = Self::fonts(&t);
        let width_of = |painter: &Painter, text: &str, font: &FontId| {
            painter
                .layout_no_wrap(text.to_owned(), font.clone(), color)
                .size()
                .x
        };
        let painter = ui.painter_at(rect);
        let whole = self.whole();
        let (stem, tail) = self.split();
        let word = tr(ui.ctx(), "status-view-only");
        let icon = Icon::Lock.size();
        let widths = self.measure(&painter, &t, &word);
        let shown = fit(&widths, rect.width());

        // The end: the exposure values, right-aligned, shutter first.
        let mut x = rect.right() - widths.exposure_width(shown.exposure);
        let end = x;
        for text in &self.exposure[..shown.exposure] {
            let galley = painter.layout_no_wrap(text.clone(), value_font.clone(), color);
            let width = galley.size().x;
            painter.galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                color,
            );
            x += width + widths.gap;
        }

        // The start: the name, then the view-only mark.
        let mut x = rect.left();
        let room = (end
            - rect.left()
            - if shown.exposure > 0 {
                widths.apart
            } else {
                0.0
            })
        .max(0.0);
        let mark_width = match shown.mark {
            CaptionMark::Word => widths.mark_word,
            CaptionMark::Icon => widths.mark_icon,
        };
        let name = match shown.name {
            CaptionName::Whole => Some(whole.clone()),
            CaptionName::Cut => {
                let room = (room - mark_width - widths.gap).max(0.0);
                let chars = (LEAST_STEM..=stem.chars().count())
                    .rev()
                    .find(|chars| width_of(&painter, &cut(stem, &tail, *chars), &name_font) <= room)
                    .unwrap_or(LEAST_STEM);
                Some(cut(stem, &tail, chars))
            }
            CaptionName::Gone => None,
        };
        if let Some(name) = name {
            let galley = painter.layout_no_wrap(name, name_font.clone(), color);
            let width = galley.size().x;
            painter.galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                color,
            );
            x += width + widths.gap;
        }
        if self.view_only {
            let square =
                Rect::from_min_size(pos2(x, rect.center().y - icon / 2.0), vec2(icon, icon));
            Icon::Lock.paint(&painter, square, color);
            if shown.mark == CaptionMark::Word {
                let galley = painter.layout_no_wrap(word.clone(), name_font, color);
                let at = pos2(
                    x + icon + t.spacing.xs,
                    rect.center().y - galley.size().y / 2.0,
                );
                painter.galley(at, galley, color);
            }
        }

        // Screen readers and the tooltip hear everything, whatever the width dropped.
        let mut said = vec![whole];
        if self.view_only {
            said.push(word);
        }
        if let Some(rating) = self.rating {
            said.extend(rating_words(ui.ctx(), rating));
        }
        said.extend(self.exposure.iter().cloned());
        let said = said.join(", ");
        let id = ui.make_persistent_id("photo caption");
        let response = ui.interact(rect, id, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &said));
        if !self.details.is_empty() {
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_description(self.details));
        }
        let response = if self.details.is_empty() {
            response.on_hover_text(said)
        } else {
            response.on_hover_text(format!("{said}\n{}", self.details))
        };
        // The centre, over the line, so its controls take the pointer.
        let plain;
        let tips = match self.tips {
            Some(tips) => tips,
            None => {
                plain = CaptionTips::plain(ui.ctx());
                &plain
            }
        };
        let width = widths.centre[shown.centre.index()];
        let action = self.rating.and_then(|rating| {
            let line = Rect::from_center_size(rect.center(), vec2(width, rect.height()));
            let centre = Centre {
                id,
                line,
                rating,
                tips,
            };
            match shown.centre {
                CaptionCentre::Folded => {
                    centre.folded(&painter, &t, &value_font);
                    None
                }
                folding => centre.controls(ui, &painter, &t, folding == CaptionCentre::Labels),
            }
        });
        CaptionOutput { response, action }
    }
}

/// What screen readers and the tooltip say of `rating`: its stars, whether it is rejected
/// and its labels.
fn rating_words(ctx: &egui::Context, rating: CellRating) -> Vec<String> {
    let words = translator(ctx);
    let mut args = Args::new();
    args.set("count", rating.stars);
    let mut said = vec![words.text_with("grid-cell-rating", Some(&args))];
    if rating.rejected {
        said.push(words.text("grid-cell-rejected"));
    }
    if !rating.labels.is_empty() {
        let names: Vec<String> = (rating.labels.iter())
            .map(|label| words.text(&format!("colour-label-{}", label.id())))
            .collect();
        let mut args = Args::new();
        args.set("labels", names.join(", "));
        said.push(words.text_with("grid-cell-labels", Some(&args)));
    }
    said
}

/// The centre's widths: whole, its labels folded, and its stars folded too, `number` the
/// stars' number's width.
fn centre_widths(t: &Theme, rating: CellRating, number: f32) -> [f32; 3] {
    let on = rating.labels.iter().count() as f32;
    let dots = if on > 0.0 {
        t.spacing.m + on * DOT + (on - 1.0) * DOT_GAP
    } else {
        0.0
    };
    let icon = Icon::Star.size();
    let reject = if rating.rejected {
        icon + t.spacing.xs
    } else {
        0.0
    };
    [
        6.0 * TARGET + t.spacing.m + 5.0 * LABEL_TARGET,
        6.0 * TARGET + dots,
        reject + icon + t.spacing.xs + number + dots,
    ]
}

/// The colour a set Reject is drawn in on the surround: `error`, or where that reads under
/// 3:1 there, as on the light theme's mid grey, the surround's text.
fn reject_colour(t: &Theme) -> Color32 {
    let c = &t.colors;
    if contrast(c.error, c.photo_surround) >= CONTROL {
        c.error
    } else {
        c.text_on_surround()
    }
}

/// A set Reject: a disc with its × cut out in the surround.
fn paint_rejected(painter: &Painter, t: &Theme, centre: Pos2, radius: f32) {
    painter.circle_filled(centre, radius, reject_colour(t));
    let arm = radius * 0.45;
    let stroke = Stroke::new(1.5, t.colors.photo_surround);
    for (from, to) in [
        (vec2(-arm, -arm), vec2(arm, arm)),
        (vec2(arm, -arm), vec2(-arm, arm)),
    ] {
        painter.line_segment([centre + from, centre + to], stroke);
    }
}

/// A label's circle at `centre`: a ring in its colour while off, filled while on, half
/// filled under the pointer as a preview of the change; with a thin neutral ring round it
/// where its colour reads under 3:1 on the surround.
fn paint_label(
    painter: &Painter,
    t: &Theme,
    centre: Pos2,
    colour: Color32,
    on: bool,
    hovered: bool,
    radius: f32,
) {
    if contrast(colour, t.colors.photo_surround) < CONTROL {
        painter.circle_stroke(
            centre,
            radius + 1.5,
            Stroke::new(1.0, t.colors.surround_text_secondary),
        );
    }
    if hovered {
        painter.circle_filled(centre, radius, colour.gamma_multiply(0.5));
    } else if on {
        painter.circle_filled(centre, radius, colour);
    }
    if !on || hovered {
        painter.circle_stroke(centre, radius - 0.75, Stroke::new(1.5, colour));
    }
}

/// The centre being drawn: the caption's id, its line, the rating and the tooltips.
struct Centre<'a> {
    id: egui::Id,
    line: Rect,
    rating: CellRating,
    tips: &'a CaptionTips,
}

impl Centre<'_> {
    /// Reject, the five stars and the labels, each a control, or with `folded` the labels'
    /// dots in place of their controls: what was clicked or set.
    fn controls(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        t: &Theme,
        folded: bool,
    ) -> Option<CellAction> {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let (rating, line) = (self.rating, self.line);
        let target = |x: f32, side: f32| {
            Rect::from_center_size(pos2(x + side / 2.0, line.center().y), vec2(side, side))
        };
        let mut action = None;
        let secondary = t.colors.surround_text_secondary;

        // Reject: outlined at rest, in the reject's colour under the pointer; set, a disc.
        let reject = target(line.left(), TARGET);
        let pressed = ui.interact(reject, self.id.with("reject"), Sense::click());
        if rating.rejected {
            paint_rejected(painter, t, reject.center(), 6.5);
        } else {
            let colour = if pressed.hovered() {
                reject_colour(t)
            } else {
                secondary
            };
            let icon = Icon::Reject.size();
            Icon::Reject.paint(
                painter,
                Rect::from_center_size(reject.center(), vec2(icon, icon)),
                colour,
            );
        }
        let name = words.text("caption-reject");
        pressed.widget_info(|| WidgetInfo::selected(Role::Button, true, rating.rejected, &name));
        if pressed.has_focus() {
            focus_ring(painter, reject, t);
        }
        let tip = if rating.rejected {
            &self.tips.unreject
        } else {
            &self.tips.reject
        };
        if pressed.clicked() {
            action = Some(CellAction::ToggleReject);
        }
        pressed.on_hover_text(tip);

        // The stars: one Tab stop, a radio group; ← and → move and set, Space or Enter
        // on the checked one clears it, as a click does.
        let stars = Rect::from_min_size(
            pos2(reject.right(), reject.top()),
            vec2(5.0 * TARGET, TARGET),
        );
        let group = ui.interact(stars, self.id.with("stars"), Sense::FOCUSABLE);
        let rating_name = words.text("caption-rating");
        group.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, true, &rating_name));
        if group.has_focus() {
            focus_ring(painter, stars, t);
            let (right, left, press) = ui.input(|input| {
                (
                    input.key_pressed(Key::ArrowRight),
                    input.key_pressed(Key::ArrowLeft),
                    input.key_pressed(Key::Space) || input.key_pressed(Key::Enter),
                )
            });
            // An arrow only where it moves: the current star again would clear the rating.
            let moved = if right && rating.stars < 5 {
                Some(rating.stars + 1)
            } else if left && rating.stars > 1 {
                Some(rating.stars - 1)
            } else if press && rating.stars > 0 {
                Some(rating.stars)
            } else {
                None
            };
            action = moved.map(CellAction::Rate).or(action);
        }
        let rects: Vec<Rect> = (0..5)
            .map(|i| target(stars.left() + i as f32 * TARGET, TARGET))
            .collect();
        let responses: Vec<Response> = (rects.iter().enumerate())
            .map(|(i, rect)| ui.interact(*rect, self.id.with(("star", i)), Sense::CLICK))
            .collect();
        let hovered = responses.iter().position(Response::hovered);
        // Over the current rating's star the stars preview empty, as its click clears it.
        let preview = hovered.map(|h| {
            if h as u8 + 1 == rating.stars {
                0
            } else {
                h as u8 + 1
            }
        });
        let icon = Icon::Star.size();
        for (i, (rect, star)) in rects.iter().zip(responses).enumerate() {
            let points = i as u8 + 1;
            let (filled, colour) = match preview {
                Some(lit) => (points <= lit, secondary),
                None => (points <= rating.stars, t.colors.text_on_surround()),
            };
            // A reject keeps the stars, dimmed beneath it.
            let colour = if rating.rejected && preview.is_none() {
                colour.gamma_multiply(0.5)
            } else if filled {
                colour
            } else {
                secondary
            };
            let glyph = if filled { Icon::StarFilled } else { Icon::Star };
            glyph.paint(
                painter,
                Rect::from_center_size(rect.center(), vec2(icon, icon)),
                colour,
            );
            let mut args = Args::new();
            args.set("count", points);
            let name = words.text_with("grid-cell-rating", Some(&args));
            let checked = points == rating.stars;
            star.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, checked, &name));
            if star.clicked() {
                action = Some(CellAction::Rate(points));
            }
            let tip = if checked {
                &self.tips.clear
            } else {
                &self.tips.stars[i]
            };
            star.on_hover_text(tip);
        }

        // The labels: each a toggle, or folded, the dots of those on.
        let start = stars.right() + t.spacing.m;
        if folded {
            self.dots(painter, t, start);
            return action;
        }
        for (i, label) in ColourLabel::ALL.into_iter().enumerate() {
            let rect = target(start + i as f32 * LABEL_TARGET, LABEL_TARGET);
            let button = ui.interact(rect, self.id.with(("label", i)), Sense::click());
            let on = rating.labels.contains(label);
            paint_label(
                painter,
                t,
                rect.center(),
                label.colour(t),
                on,
                button.hovered(),
                LABEL_RADIUS,
            );
            let name = words.text(&format!("caption-label-{}", label.id()));
            button.widget_info(|| WidgetInfo::selected(Role::Button, true, on, &name));
            if button.has_focus() {
                focus_ring(painter, rect, t);
            }
            if button.clicked() {
                action = Some(CellAction::ToggleLabel(label));
            }
            button.on_hover_text(&self.tips.labels[i]);
        }
        action
    }

    /// The labels on as dots from `x`, read only.
    fn dots(&self, painter: &Painter, t: &Theme, x: f32) {
        let y = self.line.center().y;
        for (i, label) in self.rating.labels.iter().enumerate() {
            let centre = pos2(x + i as f32 * (DOT + DOT_GAP) + DOT / 2.0, y);
            paint_label(painter, t, centre, label.colour(t), true, false, DOT / 2.0);
        }
    }

    /// The rating folded, read only: ⊗ while rejected, one star and the number, then the
    /// labels' dots.
    fn folded(&self, painter: &Painter, t: &Theme, font: &FontId) {
        let (rating, line) = (self.rating, self.line);
        let colour = t.colors.text_on_surround();
        let icon = Icon::Star.size();
        let y = line.center().y;
        let mut x = line.left();
        if rating.rejected {
            paint_rejected(painter, t, pos2(x + icon / 2.0, y), icon / 2.0);
            x += icon + t.spacing.xs;
        }
        let glyph = if rating.stars > 0 {
            Icon::StarFilled
        } else {
            Icon::Star
        };
        let dimmed = if rating.rejected {
            colour.gamma_multiply(0.5)
        } else {
            colour
        };
        glyph.paint(
            painter,
            Rect::from_center_size(pos2(x + icon / 2.0, y), vec2(icon, icon)),
            dimmed,
        );
        x += icon + t.spacing.xs;
        let galley = painter.layout_no_wrap(rating.stars.to_string(), font.clone(), dimmed);
        let width = galley.size().x;
        painter.galley(pos2(x, y - galley.size().y / 2.0), galley, dimmed);
        x += width;
        if !rating.labels.is_empty() {
            self.dots(painter, t, x + t.spacing.m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(100.0, 50.0), vec2(w, h))
    }

    /// A photo of `aspect` fitted in `room`, as the engine fits one.
    fn fitted(room: Vec2, aspect: f32) -> Vec2 {
        let width = room.x.min(room.y * aspect);
        vec2(width, width / aspect)
    }

    #[test]
    fn the_slot_loses_the_margin_and_the_caption_and_the_block_is_centred() {
        let frame = PhotoFrame::new(slot(800.0, 600.0), Margin::Points(10.0));
        assert_eq!(frame.bounds(), vec2(780.0, 600.0 - 20.0 - 24.0));
        // Landscape: the width limits, so the block is centred up and down.
        let photo = fitted(frame.bounds(), 1.5);
        let placed = frame.place(photo, 1.0);
        assert_eq!(placed.photo.width(), 780.0);
        assert_eq!(placed.photo.height(), 520.0);
        assert_eq!(placed.photo.left(), 110.0);
        let block_height = placed.photo.height() + CAPTION_GAP + CAPTION_HEIGHT;
        assert_eq!(placed.photo.top(), 50.0 + (600.0 - block_height) / 2.0);
        // The caption is as wide as the photo, under it, the gap between.
        assert_eq!(placed.caption.x_range(), placed.photo.x_range());
        assert_eq!(placed.caption.top(), placed.photo.bottom() + CAPTION_GAP);
        assert_eq!(placed.caption.height(), CAPTION_HEIGHT);
    }

    #[test]
    fn a_portrait_photo_is_limited_by_the_height_and_a_square_one_by_the_smaller_side() {
        let frame = PhotoFrame::new(slot(800.0, 600.0), Margin::Points(10.0));
        let portrait = frame.place(fitted(frame.bounds(), 2.0 / 3.0), 1.0);
        assert_eq!(portrait.photo.height(), 556.0);
        assert!((portrait.photo.width() - 556.0 * 2.0 / 3.0).abs() < 0.01);
        // Centred sideways; the block as high as the slot less its margins.
        let centre = 100.0 + 400.0;
        assert!((portrait.photo.center().x - centre).abs() < 0.5);
        assert_eq!(portrait.photo.top(), 60.0);
        assert_eq!(portrait.caption.bottom(), 50.0 + 600.0 - 10.0);
        let square = frame.place(fitted(frame.bounds(), 1.0), 1.0);
        assert_eq!(
            (square.photo.width(), square.photo.height()),
            (556.0, 556.0)
        );
    }

    #[test]
    fn proof_margin_is_a_fraction_of_the_smaller_side_and_draws_nothing_white() {
        let frame = PhotoFrame::new(slot(800.0, 600.0), Margin::Proof(0.2));
        // 0.2 of 600 is 120, all of it the surround.
        assert_eq!(frame.margin, 120.0);
        assert_eq!(
            frame.bounds(),
            vec2(800.0 - 2.0 * 120.0, 600.0 - 2.0 * 120.0 - 24.0)
        );
        let placed = frame.place(fitted(frame.bounds(), 1.5), 1.0);
        // The caption right under the photo; the block centred.
        assert_eq!(placed.caption.top(), placed.photo.bottom() + 4.0);
        assert_eq!(placed.caption.x_range(), placed.photo.x_range());
        let block = placed.photo.union(placed.caption);
        assert!((block.center().x - 500.0).abs() < 0.5);
        assert!((block.center().y - 350.0).abs() < 0.5);
        // The smaller side counts, and the photo keeps the whole margin from the slot's
        // edge: 60 on a 300 wide slot.
        let tall = PhotoFrame::new(slot(300.0, 800.0), Margin::Proof(0.2));
        assert_eq!(tall.margin, 60.0);
        let placed = tall.place(fitted(tall.bounds(), 1.5), 1.0);
        assert_eq!(placed.photo.left(), 100.0 + 60.0);
        assert_eq!(placed.photo.right(), 100.0 + 300.0 - 60.0);
    }

    #[test]
    fn a_narrow_panel_leaves_the_photo_what_remains_and_never_less_than_nothing() {
        let narrow = PhotoFrame::new(slot(100.0, 400.0), Margin::Points(10.0));
        assert_eq!(narrow.bounds(), vec2(80.0, 356.0));
        let placed = narrow.place(fitted(narrow.bounds(), 1.5), 1.0);
        assert_eq!(placed.photo.width(), 80.0);
        assert!(placed.photo.left() >= 100.0 + 10.0 - 0.5);
        // A margin larger than the slot leaves nothing, not a negative size.
        let tiny = PhotoFrame::new(slot(30.0, 30.0), Margin::Points(100.0));
        assert_eq!(tiny.bounds(), Vec2::ZERO);
        let proof = PhotoFrame::new(slot(30.0, 30.0), Margin::Proof(0.4));
        assert_eq!(proof.bounds().y, 0.0);
    }

    #[test]
    fn the_photo_stays_on_the_physical_pixel_grid() {
        let frame = PhotoFrame::new(slot(801.0, 601.0), Margin::Points(10.0));
        let placed = frame.place(vec2(777.3, 518.2), 2.0);
        let min = placed.photo.min;
        assert_eq!((min.x * 2.0).fract(), 0.0);
        assert_eq!((min.y * 2.0).fract(), 0.0);
    }

    fn widths() -> Widths {
        Widths {
            name: 100.0,
            name_least: 50.0,
            mark_word: 60.0,
            mark_icon: 12.0,
            exposure: vec![40.0, 30.0, 40.0, 40.0],
            centre: [0.0; 3],
            gap: 8.0,
            apart: 12.0,
        }
    }

    #[test]
    fn a_caption_drops_its_parts_in_order_as_the_photo_narrows() {
        let w = widths();
        let at = |available| fit(&w, available);
        // Everything: 100 + 8 + 60 at the start, 40 + 30 + 40 + 40 + 24 at the end, 12 apart.
        assert_eq!(
            at(354.0),
            CaptionFit {
                exposure: 4,
                mark: CaptionMark::Word,
                centre: CaptionCentre::Whole,
                name: CaptionName::Whole
            }
        );
        // The exposure goes from the last: ISO, then focal length, aperture, shutter.
        assert_eq!(at(353.0).exposure, 3);
        assert_eq!(at(306.0).exposure, 3);
        assert_eq!(at(305.0).exposure, 2);
        assert_eq!(at(258.0).exposure, 2);
        assert_eq!(at(257.0).exposure, 1);
        assert_eq!(at(220.0).exposure, 1);
        assert_eq!(at(219.0).exposure, 0);
        // Then the mark's word becomes its icon, then the name is cut, then it goes.
        let bare = at(168.0);
        assert_eq!(
            (bare.exposure, bare.mark, bare.name),
            (0, CaptionMark::Word, CaptionName::Whole)
        );
        let icon = at(167.0);
        assert_eq!(
            (icon.mark, icon.name),
            (CaptionMark::Icon, CaptionName::Whole)
        );
        assert_eq!(at(120.0).name, CaptionName::Whole);
        assert_eq!(
            (at(119.0).mark, at(119.0).name),
            (CaptionMark::Icon, CaptionName::Cut)
        );
        assert_eq!(at(70.0).name, CaptionName::Cut);
        assert_eq!(at(69.0).name, CaptionName::Gone);
        assert_eq!(at(0.0).name, CaptionName::Gone);
    }

    #[test]
    fn a_photo_with_no_mark_or_no_exposure_drops_only_what_it_has() {
        let none = Widths {
            mark_word: 0.0,
            mark_icon: 0.0,
            exposure: Vec::new(),
            ..widths()
        };
        assert_eq!(fit(&none, 100.0).name, CaptionName::Whole);
        assert_eq!(fit(&none, 99.0).name, CaptionName::Cut);
        assert_eq!(fit(&none, 49.0).name, CaptionName::Gone);
    }

    #[test]
    fn a_centre_stays_centred_so_the_wider_side_counts_twice() {
        let w = Widths {
            centre: [80.0, 60.0, 30.0],
            ..widths()
        };
        let all = CaptionFit {
            exposure: 4,
            mark: CaptionMark::Word,
            centre: CaptionCentre::Whole,
            name: CaptionName::Whole,
        };
        assert_eq!(w.needed(all), 2.0 * 174.0 + 80.0 + 24.0);
    }

    /// With a rating, once the exposure has gone and the mark is its icon, the labels fold,
    /// then the stars, and only then is the name cut; the folded centre never goes.
    #[test]
    fn a_rated_caption_folds_its_labels_then_its_stars_before_the_name() {
        let w = Widths {
            centre: [218.0, 140.0, 40.0],
            ..widths()
        };
        let mut order = Vec::new();
        for available in (0..800).rev() {
            let step = fit(&w, available as f32);
            if order.last() != Some(&(step.centre, step.name)) && step.exposure == 0 {
                order.push((step.centre, step.name));
            }
        }
        assert_eq!(
            order,
            [
                (CaptionCentre::Whole, CaptionName::Whole),
                (CaptionCentre::Labels, CaptionName::Whole),
                (CaptionCentre::Folded, CaptionName::Whole),
                (CaptionCentre::Folded, CaptionName::Cut),
                (CaptionCentre::Folded, CaptionName::Gone),
            ]
        );
        // Whole with the exposure: twice the wider side, the centre and the gaps.
        assert_eq!(fit(&w, 2.0 * 174.0 + 218.0 + 24.0).exposure, 4);
    }

    /// The cut measured in a font: whole where it fits, else cut in its middle to fit, the
    /// tail kept whole.
    #[test]
    fn a_name_is_cut_in_its_middle_to_fit_a_width_keeping_its_tail() {
        let ctx = egui::Context::default();
        crate::install(&ctx, Theme::default());
        // Fonts take effect from the next pass; no renderer takes the atlas's updates here.
        for pass in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                if pass == 0 {
                    return;
                }
                let painter = ui.painter().clone();
                let font = FontId::proportional(13.0);
                let width = |text: &str| {
                    painter
                        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x
                };
                let (stem, tail) = ("2026-10 Lisbon and Sintra", " · 212 photos");
                let whole = format!("{stem}{tail}");
                assert_eq!(
                    middle_cut(&painter, stem, tail, &font, width(&whole)),
                    whole
                );
                let room = width(&whole) * 0.7;
                let cut = middle_cut(&painter, stem, tail, &font, room);
                assert!(
                    cut.ends_with(tail) && cut.starts_with("2026") && cut.contains('…'),
                    "{cut}"
                );
                assert!(width(&cut) <= room, "{cut}");
                // At worst one character of the stem is left.
                assert_eq!(
                    middle_cut(&painter, stem, tail, &font, 1.0),
                    format!("…{tail}")
                );
            });
            output.textures_delta.clear();
        }
    }

    #[test]
    fn a_name_is_cut_in_its_middle_keeping_its_extension_and_copy() {
        assert_eq!(cut("R0001176", ".DNG [2]", 5), "R0…76.DNG [2]");
        assert_eq!(cut("R0001176", ".DNG", 8), "R0001176.DNG");
        assert_eq!(cut("R0001176", ".DNG", 6), "R00…76.DNG");
        assert_eq!(cut("IMG_2041", ".ARW", 20), "IMG_2041.ARW");
        // No extension, a name of one character: nothing to cut.
        assert_eq!(cut("x", "", 5), "x");
    }
}
