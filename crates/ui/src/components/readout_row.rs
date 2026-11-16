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

//! The readout row, one line (`docs/gui.md`, How the rows are laid out): a number the
//! photo gives and nobody edits, as darktable's "computed EC" under Exposure's Target
//! level. Its label at the start in `text_secondary`, at the body size of every row's label,
//! its value at the end in `text`,
//! where a slider's value ends, written as a slider writes its value at rest, in the
//! language's format with its unit; with no value, nothing at the end. It takes no focus;
//! screen readers hear the label and the value as one label, and its promise is the
//! tooltip.
//!
//! A readout of words of its own ([`ReadoutRow::text`]) shows them in place of a number.
//! Where those words hold [`ReadoutRow::PATCH`], each is drawn as a patch, a colour of
//! [`ReadoutRow::patches`] in turn, 16 points by 12, as Exposure's Last pick,
//! "■ L 41 % → ■ L 50 % · +0.55 EV", shows the area before and after the pick, a patch
//! before each value (decision 88). Screen readers hear the words without the patches.

use egui::accesskit::Role;
use egui::{Color32, FontId, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::{theme, translator};

/// Where in a readout's words ([`ReadoutRow::text`]) a patch is drawn, as a message writes
/// it: "■ L 41 % → ■ L 50 %".
const PATCH: char = '■';

/// A patch's width and height, in points.
const PATCH_SIZE: [f32; 2] = [16.0, 12.0];

pub struct ReadoutRow<'a> {
    label: &'a str,
    promise: Option<&'a str>,
    value: Option<f64>,
    decimals: usize,
    signed: bool,
    unit: Option<&'a str>,
    /// Words of its own in place of the number.
    text: Option<&'a str>,
    /// The colours of the patches in the words, in turn.
    patches: Vec<Color32>,
}

impl<'a> ReadoutRow<'a> {
    /// The mark of a patch in a readout's words ([`ReadoutRow::text`]), as a message writes
    /// it: "■ L 41 % → ■ L 50 %".
    pub const PATCH: char = PATCH;

    /// `words` as screen readers hear them, without the patches [`ReadoutRow::text`] draws
    /// at each [`Self::PATCH`] and the space after it.
    pub fn spoken(words: &str) -> String {
        words.replace(&format!("{PATCH} "), "").replace(PATCH, "")
    }

    /// `label` is the short, translated label; `value` the number, `None` while there is
    /// none to show.
    pub fn new(label: &'a str, value: Option<f64>) -> Self {
        ReadoutRow {
            label,
            promise: None,
            value,
            decimals: 2,
            signed: false,
            unit: None,
            text: None,
            patches: Vec::new(),
        }
    }

    /// Words of its own, translated, shown at the end in place of a number: Last pick's
    /// "■ L 41 % → ■ L 50 % · +0.55 EV", each [`ReadoutRow::PATCH`] drawn as a patch.
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }

    /// The colours of the patches the words draw at their [`ReadoutRow::PATCH`]s, in turn: the area
    /// Last pick read, as it rendered before the pick and as it renders after.
    pub fn patches(mut self, colours: &[Color32]) -> Self {
        self.patches = colours.to_vec();
        self
    }

    /// The readout's promise, in the tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// The decimals the value shows.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Whether the value shows `+` before zero and above, as a slider's over a range that
    /// spans zero.
    pub fn signed(mut self, signed: bool) -> Self {
        self.signed = signed;
        self
    }

    /// The value's unit, as the id of the message that writes a value with it, such as
    /// `unit-ev-value`: "+0.83 EV".
    pub fn unit(mut self, message: &'a str) -> Self {
        self.unit = Some(message);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        let number = self.value.map(|value| {
            let number = tr.format_number(value, self.decimals, self.signed);
            match self.unit {
                Some(unit) => {
                    let mut args = crate::i18n::Args::new();
                    args.set("value", number);
                    tr.text_with(unit, Some(&args))
                }
                None => number,
            }
        });
        let value = self.text.map(str::to_owned).or(number);
        let font = FontId::proportional(t.text.small);
        // The label is every row's, at the body size.
        let body = FontId::proportional(t.text.body);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            // The value first, never cut, in pieces between its patches; the label cut
            // with … to what it leaves, a choice row's 12-point gap between them. Screen
            // readers hear both whole.
            let end = rect.right() - super::field_inset(&t);
            // Between its patches the words are laid out trimmed, a space against a patch a
            // gap of `spacing.xs`, which a galley's own trailing space would not give.
            let split: Vec<&str> = value
                .as_deref()
                .map_or_else(Vec::new, |value| value.split(PATCH).collect());
            let patched = split.len() > 1;
            let gap = |space: bool| if patched && space { t.spacing.xs } else { 0.0 };
            let pieces: Vec<_> = split
                .iter()
                .enumerate()
                .map(|(at, piece)| {
                    let text = if patched { piece.trim() } else { piece };
                    let before = gap(at > 0 && piece.starts_with(' '));
                    let after = gap(at + 1 < split.len() && piece.ends_with(' '));
                    let galley =
                        painter.layout_no_wrap(text.to_owned(), font.clone(), t.colors.text);
                    (galley, before, after)
                })
                .collect();
            let [patch_width, patch_height] = PATCH_SIZE;
            let patches = pieces.len().saturating_sub(1) as f32;
            let width: f32 = pieces
                .iter()
                .map(|(galley, before, after)| galley.size().x + before + after)
                .sum::<f32>()
                + patches * patch_width;
            let room = if pieces.is_empty() {
                rect.width()
            } else {
                end - width - t.spacing.l - rect.left()
            };
            let label = painter.layout_job(super::truncated(
                self.label,
                body,
                room.max(0.0),
                t.colors.text_secondary,
            ));
            let centred = |height: f32| rect.center().y - height / 2.0;
            painter.galley(
                pos2(rect.left(), centred(label.size().y)),
                label,
                t.colors.text_secondary,
            );
            // Where a slider's value ends at rest, inside its field's inset.
            let mut x = end - width;
            for (at, (galley, before, after)) in pieces.into_iter().enumerate() {
                if at > 0 {
                    let colour = self.patches.get(at - 1).copied().unwrap_or_default();
                    let patch = Rect::from_min_size(
                        pos2(x, rect.center().y - patch_height / 2.0),
                        vec2(patch_width, patch_height),
                    );
                    painter.rect_filled(patch, t.shape.radius_small, colour);
                    painter.rect_stroke(
                        patch,
                        t.shape.radius_small,
                        egui::Stroke::new(1.0, t.colors.border),
                        egui::StrokeKind::Inside,
                    );
                    x += patch_width;
                }
                x += before;
                let size = galley.size();
                painter.galley(pos2(x, centred(size.y)), galley, t.colors.text);
                x += size.x + after;
            }
        }
        let heard = match &value {
            Some(value) => format!("{} {}", self.label, Self::spoken(value)),
            None => self.label.to_owned(),
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &heard));
        match self.promise {
            Some(promise) => response.on_hover_text(promise),
            None => response,
        }
    }
}
