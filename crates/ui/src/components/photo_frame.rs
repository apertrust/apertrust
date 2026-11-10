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

//! The photo frame (`docs/gui.md`, The photo frame): how the central panel shows a photo
//! and its caption, for looking and comparing. Nothing is drawn on the photo.
//!
//! **The slot and the block.** The central panel's area under the central bar is the
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
//! marks at the start, the centre for the reject, the rating and the colour labels (those
//! are not stored yet, so it is empty), the exposure values at the end. It drops parts as
//! the photo narrows ([`fit`]).

use egui::{
    FontFamily, FontId, Painter, Rect, Response, Role, Sense, Ui, Vec2, WidgetInfo,
    emath::GuiRounding as _, pos2, vec2,
};

use crate::theme::Theme;
use crate::{Icon, fonts, theme, tr};

/// The caption's height, in points.
pub const CAPTION_HEIGHT: f32 = 20.0;
/// The gap between the photo and its caption.
pub const CAPTION_GAP: f32 = 4.0;
/// The least stem of a file name, in characters, that the caption still shows cut.
const LEAST_STEM: usize = 5;

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
    /// The centre's width (reject, rating and labels); 0 while it is empty.
    pub centre: f32,
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

/// What a caption shows at a width: the parts that survived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptionFit {
    /// How many exposure values, from the first.
    pub exposure: usize,
    pub mark: CaptionMark,
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
        if self.centre > 0.0 {
            2.0 * start.max(end) + self.centre + 2.0 * self.apart
        } else if start > 0.0 && end > 0.0 {
            start + end + self.apart
        } else {
            start + end
        }
    }
}

/// What a caption of `available` points shows, dropping parts in the spec's order, one
/// step at a time: the exposure values from the last (ISO, then focal length, aperture and
/// shutter), the mark's word for its icon, then the name cut in its middle, then the name.
/// The rating and the labels, when the caption has them, are folded before the name is
/// cut and never go.
pub fn fit(widths: &Widths, available: f32) -> CaptionFit {
    let steps = (0..=widths.exposure.len())
        .rev()
        .map(|exposure| CaptionFit {
            exposure,
            mark: CaptionMark::Word,
            name: CaptionName::Whole,
        })
        .chain([
            CaptionFit {
                exposure: 0,
                mark: CaptionMark::Icon,
                name: CaptionName::Whole,
            },
            CaptionFit {
                exposure: 0,
                mark: CaptionMark::Icon,
                name: CaptionName::Cut,
            },
            CaptionFit {
                exposure: 0,
                mark: CaptionMark::Icon,
                name: CaptionName::Gone,
            },
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

/// A photo's caption: one line under it, as wide as the photo.
pub struct PhotoCaption<'a> {
    name: &'a str,
    copy: Option<u32>,
    view_only: bool,
    exposure: &'a [String],
    details: &'a str,
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
        }
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
            centre: 0.0,
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
    pub fn show(self, ui: &mut Ui, rect: Rect) -> Response {
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
        said.extend(self.exposure.iter().cloned());
        let said = said.join(", ");
        let response = ui.interact(rect, ui.make_persistent_id("photo caption"), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &said));
        if self.details.is_empty() {
            response.on_hover_text(said)
        } else {
            response.on_hover_text(format!("{said}\n{}", self.details))
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
            centre: 0.0,
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
            centre: 80.0,
            ..widths()
        };
        let all = CaptionFit {
            exposure: 4,
            mark: CaptionMark::Word,
            name: CaptionName::Whole,
        };
        assert_eq!(w.needed(all), 2.0 * 174.0 + 80.0 + 24.0);
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
