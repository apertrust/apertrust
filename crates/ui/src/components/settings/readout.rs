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

//! A value a settings row shows and cannot change, as Storage's folders: the value in
//! `text` at the column's end, a path in the mono face cut in its middle, other words cut
//! at their end, whole in its tooltip; and, where given, a button after it, a box high, as
//! a folder's Show ([`super::Button`]). To screen readers the value is a label, described
//! by what is said of it where that warns.

use egui::{Color32, FontId, Painter, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2};

use super::super::plate::undimmed_painter;
use super::super::truncated;
use super::{Button, body, dimmed, mono};
use crate::theme;

pub struct Readout<'a> {
    value: &'a str,
    path: bool,
    button: Option<Button<'a>>,
    description: Option<&'a str>,
}

impl<'a> Readout<'a> {
    /// `value` is what it shows, translated where it is words.
    pub fn new(value: &'a str) -> Self {
        Readout {
            value,
            path: false,
            button: None,
            description: None,
        }
    }

    /// A path: in the mono face, cut in its middle, whose start and end both say where it
    /// is.
    pub fn path(mut self, path: bool) -> Self {
        self.path = path;
        self
    }

    /// The button after the value, as a folder's Show.
    pub fn button(mut self, button: Button<'a>) -> Self {
        self.button = Some(button);
        self
    }

    /// What screen readers hear after the value, translated, as a warning said of it.
    pub fn description(mut self, text: &'a str) -> Self {
        self.description = Some(text);
        self
    }

    /// Draws it across `ui`'s width; returns whether its button was pressed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = theme(ui.ctx());
        let enabled = ui.is_enabled();
        let height = t.sizes.setting_control;
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let mut end = strip.right();
        let mut pressed = false;
        if let Some(button) = self.button {
            let mut place = ui.new_child(egui::UiBuilder::new().max_rect(strip));
            let response = button.show(&mut place);
            pressed = response.clicked();
            end = response.rect.left() - t.spacing.m;
        }
        let font = if self.path { mono(&t) } else { body(&t) };
        let painter = undimmed_painter(ui);
        let room = (end - strip.left()).max(0.0);
        let colour = dimmed(&t, enabled, t.colors.text);
        let (galley, cut) = if self.path {
            let shown = cut_middle(&painter, self.value, &font, room);
            let cut = shown != self.value;
            (painter.layout_no_wrap(shown, font, colour), cut)
        } else {
            let galley = painter.layout_job(truncated(self.value, font, room, colour));
            let cut = galley.elided;
            (galley, cut)
        };
        let at = pos2(
            end - galley.size().x,
            strip.center().y - galley.size().y / 2.0,
        );
        let rect = Rect::from_min_size(at, galley.size());
        painter.galley(at, galley, colour);
        let value = ui.interact(rect, ui.make_persistent_id("readout value"), Sense::hover());
        let whole = self.value;
        value.widget_info(|| WidgetInfo::labeled(Role::Label, enabled, whole));
        if let Some(said) = self.description {
            ui.ctx()
                .accesskit_node_builder(value.id, |node| node.set_description(said));
        }
        if cut {
            value.on_hover_text(whole);
        }
        pressed
    }
}

/// `text` whole where it fits in `width`, else with its middle cut to `…`.
fn cut_middle(painter: &Painter, text: &str, font: &FontId, width: f32) -> String {
    let fits = |text: &str| {
        painter
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER)
            .size()
            .x
            <= width
    };
    if fits(text) {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let kept = |kept: usize| {
        let head = kept.div_ceil(2);
        let start: String = chars[..head].iter().collect();
        let end: String = chars[chars.len() - (kept - head)..].iter().collect();
        format!("{start}…{end}")
    };
    // The most characters kept that still fit, found by halves: fewer kept is never wider.
    let (mut fit, mut over) = (0, chars.len());
    while over - fit > 1 {
        let middle = (fit + over) / 2;
        if fits(&kept(middle)) {
            fit = middle;
        } else {
            over = middle;
        }
    }
    kept(fit)
}
