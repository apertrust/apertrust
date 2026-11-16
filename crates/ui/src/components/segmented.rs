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

//! A few options side by side, one of them chosen: Linear | Log, a scope's colour space.
//! In a bar each option is as wide as the widest; in a tool the options share the full
//! width of one row, with no label (`docs/gui.md`, How the rows are laid out): a segmented
//! choice only ever switches a mode, which its options say, and screen readers hear its
//! name.
//!
//! An option can be drawn disabled with its reason as its tooltip, as Exposure's Automatic
//! on an image that is not a raw (decision 88): dimmed, it takes no click and no focus,
//! and screen readers hear the reason as its description. A disabled option may still be
//! the chosen one, as an edit from another photo holds it. With nothing else to choose,
//! every other option disabled, the chosen one is drawn as it is anywhere, not dimmed, and
//! only the others show disabled with their reasons.
//!
//! The chosen option is filled `raised` with `text_strong` and no border, on the `sunken`
//! track.

use egui::{Color32, FontId, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2};

use super::focus_ring;
use crate::theme;

pub struct Segmented<'a> {
    name: &'a str,
    options: &'a [&'a str],
    selected: usize,
    full_width: bool,
    promise: Option<&'a str>,
    enabled: bool,
    /// Options that cannot be chosen, by index, each with its reason.
    disabled: &'a [(usize, &'a str)],
}

impl<'a> Segmented<'a> {
    /// `name` says what the options choose, for screen readers; the options are
    /// translated.
    pub fn new(name: &'a str, options: &'a [&'a str], selected: usize) -> Self {
        Segmented {
            name,
            options,
            selected,
            full_width: false,
            promise: None,
            enabled: true,
            disabled: &[],
        }
    }

    /// In a tool: the options across the full width of one row, with no label.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// The control's promise, translated: the tooltip of each option with no reason of
    /// its own.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// A disabled switch, every option of it, the chosen one too, is dimmed and ignores
    /// input: a control that can change nothing on the photo.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Options that cannot be chosen, by index, each with its reason, translated, as its
    /// tooltip and its description to screen readers.
    pub fn disabled_options(mut self, options: &'a [(usize, &'a str)]) -> Self {
        self.disabled = options;
        self
    }

    /// Why `option` cannot be chosen, if it cannot.
    fn reason(&self, option: usize) -> Option<&'a str> {
        self.disabled
            .iter()
            .find(|(at, _)| *at == option)
            .map(|(_, why)| *why)
    }

    /// Returns the option the user chose this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        let font = FontId::proportional(t.text.small);
        let pad = t.spacing.xxs;
        let live = self.enabled && ui.is_enabled();
        // Laid out in the placeholder colour, so each is painted in its state's.
        let galleys: Vec<_> = self
            .options
            .iter()
            .map(|o| {
                ui.painter()
                    .layout_no_wrap((*o).to_owned(), font.clone(), Color32::PLACEHOLDER)
            })
            .collect();
        let count = self.options.len() as f32;
        let height = t.sizes.control - 2.0;
        let (outer, group) = if self.full_width {
            let (row, group) =
                ui.allocate_exact_size(vec2(ui.available_width(), t.sizes.row), Sense::hover());
            let outer = Rect::from_center_size(row.center(), vec2(row.width(), height + 2.0 * pad));
            (outer, group)
        } else {
            let segment =
                galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + 2.0 * t.spacing.m;
            let size = vec2(segment * count + pad * (count + 1.0), height + 2.0 * pad);
            ui.allocate_exact_size(size, Sense::hover())
        };
        let segment = (outer.width() - pad * (count + 1.0)) / count;
        group.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, live, self.name));
        let group_id = group.id;
        ui.painter()
            .rect_filled(outer, t.shape.radius_large, t.colors.sunken);
        let mut chosen = None;
        for (i, galley) in galleys.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                pos2(
                    outer.left() + pad + i as f32 * (segment + pad),
                    outer.top() + pad,
                ),
                vec2(segment, height),
            );
            let reason = self.reason(i);
            let enabled = live && reason.is_none();
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = ui.interact(rect, group_id.with(i), sense);
            let on = i == self.selected;
            if on {
                ui.painter()
                    .rect_filled(rect, t.shape.radius, t.colors.raised);
            } else if enabled && response.hovered() {
                ui.painter()
                    .rect_filled(rect, t.shape.radius, t.colors.hover);
            }
            let colour = if !enabled {
                t.colors.text_disabled
            } else if on {
                t.colors.text_strong
            } else {
                t.colors.text_secondary
            };
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, colour);
            if response.has_focus() {
                focus_ring(ui.painter(), rect, &t);
            }
            let option = self.options[i];
            response.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, on, option));
            if let Some(reason) = reason {
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_description(reason));
            }
            let clicked = response.clicked();
            if let Some(tip) = reason.or(self.promise) {
                response.on_hover_text(tip).on_disabled_hover_text(tip);
            }
            if clicked && enabled && !on {
                chosen = Some(i);
            }
        }
        chosen
    }
}
