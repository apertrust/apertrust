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

//! A choice in a settings row, for five options or more, or long ones: a box the column
//! wide and `sizes.setting_control` high, `raised` with no frame and
//! `shape.radius` corners, the option in `text_strong` at the body size and ⌄ in
//! `text_secondary` at its end. An option too long for it is cut with …, whole in its
//! tooltip.
//!
//! It opens the choice's list, the popup service's dropdown a [`Choice`] opens, with the
//! chosen option checked, options that cannot be chosen disabled with their reasons, an
//! option the list no longer offers last and disabled, and a separator where one is given
//! ([`Choice::list`]). A click, Space, Enter or ↓ opens it, and a click again closes it. To
//! screen readers it is a combo box named by the row's label, the option its value.

use egui::{Color32, Key, Rect, Role, Ui, WidgetInfo, pos2, vec2};

use super::super::plate::{allocate, undimmed_painter};
use super::super::{Choice, field_inset, truncated};
use super::{body, dimmed, paint_box, ring};
use crate::popup::{self, Answer};
use crate::{Icon, theme};

pub struct Combo<'a> {
    /// What builds the list, a choice's own.
    choice: Choice<'a>,
    name: &'a str,
    options: &'a [&'a str],
    selected: Option<usize>,
    description: Option<&'a str>,
    enabled: bool,
}

impl<'a> Combo<'a> {
    /// `name` is the row's label and `options` the options' words, translated; `selected`
    /// the chosen one's index, if any.
    pub fn new(name: &'a str, options: &'a [&'a str], selected: Option<usize>) -> Self {
        Combo {
            choice: Choice::new("", options, selected).name(name),
            name,
            options,
            selected,
            description: None,
            enabled: true,
        }
    }

    /// Options that cannot be chosen, by index, and their tooltip, as [`Choice`]'s.
    pub fn disabled_options(mut self, options: &'a [usize], tip: &'a str) -> Self {
        self.choice = self.choice.disabled_options(options, tip);
        self
    }

    /// An option chosen that the list no longer offers, listed last and disabled, as
    /// [`Choice`]'s.
    pub fn unlisted(mut self, option: usize, tip: &'a str) -> Self {
        self.choice = self.choice.unlisted(option, tip);
        self
    }

    /// A separator in the list before `option`.
    pub fn separator_before(mut self, option: usize) -> Self {
        self.choice = self.choice.separator_before(option);
        self
    }

    /// What screen readers hear after its value, translated: what the row says under it.
    pub fn description(mut self, text: &'a str) -> Self {
        self.description = Some(text);
        self
    }

    /// A disabled combo keeps its option and opens nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws it across `ui`'s width; returns the option chosen this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        let enabled = self.enabled && ui.is_enabled();
        let size = vec2(ui.available_width(), t.sizes.setting_control);
        // A disabled one is allocated disabled, so it takes no focus either.
        let (rect, response) = allocate(ui, size, enabled);
        let id = response.id;
        let chevron = Icon::ChevronDown.size();
        let inset = field_inset(&t);
        let current = self
            .selected
            .and_then(|i| self.options.get(i))
            .copied()
            .unwrap_or("");
        let room = rect.width() - 2.0 * inset - chevron - t.spacing.xs;
        let shown = ui.painter().layout_job(truncated(
            current,
            body(&t),
            room.max(0.0),
            Color32::PLACEHOLDER,
        ));
        let cut = shown.elided;
        if ui.is_rect_visible(rect) {
            let painter = undimmed_painter(ui);
            paint_box(&painter, rect, &t);
            let middle = rect.center().y;
            let text = dimmed(&t, enabled, t.colors.text_strong);
            painter.galley(
                pos2(rect.left() + inset, middle - shown.size().y / 2.0),
                shown,
                text,
            );
            Icon::ChevronDown.paint(
                &painter,
                Rect::from_center_size(
                    pos2(rect.right() - inset - chevron / 2.0, middle),
                    vec2(chevron, chevron),
                ),
                dimmed(&t, enabled, t.colors.text_secondary),
            );
            if response.has_focus() {
                ring(&painter, rect, &t);
            }
        }
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::ComboBox, enabled, self.name);
            info.current_text_value = Some(current.to_owned());
            info
        });
        if let Some(said) = self.description {
            ui.ctx()
                .accesskit_node_builder(id, |node| node.set_description(said));
        }
        // The list: a click, Space, Enter or ↓ opens it, and a click closes it again. ↓ is
        // consumed, so egui's focus does not move down too.
        let list = id.with("list");
        let open = popup::is_open(ui.ctx(), list);
        let down = enabled
            && response.has_focus()
            && !open
            && ui.input_mut(|i| super::super::steps::plain(i, Key::ArrowDown));
        if enabled && (response.clicked() || down) {
            if open {
                popup::close(ui.ctx(), list);
            } else {
                let request = self.choice.list(rect).opened_by(id);
                popup::open(ui.ctx(), list, request);
            }
        }
        // A combo disabled while its list is open closes it: nothing in it can be chosen.
        if !enabled && popup::is_open(ui.ctx(), list) {
            popup::close(ui.ctx(), list);
        }
        if popup::is_open(ui.ctx(), list) {
            popup::show(ui.ctx());
        }
        let mut chosen = None;
        if let Some(Answer::Chosen(item)) = popup::answer(ui.ctx(), list)
            && let Ok(i) = item.parse::<usize>()
            && self.selected != Some(i)
            && enabled
            && !self.choice.disabled_now(i)
        {
            chosen = Some(i);
        }
        if cut {
            response
                .on_hover_text(current)
                .on_disabled_hover_text(current);
        }
        chosen
    }
}
