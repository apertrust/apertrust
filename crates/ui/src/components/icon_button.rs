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

//! A button that is only an icon: a section's header buttons, the top panel's actions, a
//! slider's quad. Its name is its tooltip and what screen readers say; the keys that run
//! it, given by the app's actions registry, follow the name in the tooltip and are
//! reported to screen readers as its shortcut.

use egui::{Popup, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, vec2};

use super::{focus_ring, icon_colour, icon_lit, report_shortcut};
use crate::{Icon, popup, theme};

pub struct IconButton<'a> {
    icon: Icon,
    name: &'a str,
    toggled: Option<bool>,
    enabled: bool,
    shortcut: Option<&'a str>,
    size: Option<f32>,
    tip: Option<&'a str>,
}

impl<'a> IconButton<'a> {
    /// `name` is translated: it is the tooltip and the button's accessible name.
    pub fn new(icon: Icon, name: &'a str) -> Self {
        IconButton {
            icon,
            name,
            toggled: None,
            enabled: true,
            shortcut: None,
            size: None,
            tip: None,
        }
    }

    /// The tooltip, translated, in place of the name: a button whose tooltip says more,
    /// as the grid bar's Sort names the order ("Sort: capture time, ascending").
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// A toggle: while `on`, its icon filled where the icon has a filled form (a view
    /// toggle), else drawn pressed in a box (a quad's picker).
    pub fn toggled(mut self, on: bool) -> Self {
        self.toggled = Some(on);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The keys that do the same, in the platform's spelling (`Ctrl+,`), shown in the
    /// tooltip.
    pub fn shortcut(mut self, keys: &'a str) -> Self {
        self.shortcut = Some(keys);
        self
    }

    /// The button's side, in points; by default the icon's size plus a margin.
    pub fn size(mut self, side: f32) -> Self {
        self.size = Some(side);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let side = self
            .size
            .unwrap_or_else(|| (self.icon.size() + 8.0).max(t.sizes.control));
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(side, side), sense);
        let on = self.toggled == Some(true);
        // A toggle whose icon has a filled form shows it on by that alone; one without, a
        // quad's picker or a menu's button, is drawn pressed, in a box.
        let filled = self.icon.filled() != self.icon;
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if on && !filled {
                painter.rect(
                    rect,
                    t.shape.radius,
                    t.colors.raised,
                    Stroke::new(t.shape.hairline, t.colors.border),
                    StrokeKind::Inside,
                );
            } else if self.enabled && response.hovered() {
                painter.rect_filled(rect, t.shape.radius, t.colors.hover);
            }
            // Secondary text at rest, text while lit: under the pointer, pressed, focused
            // by a key or with its menu open. A toggle that is on shows it in text.
            let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response))
                || popup::is_open(ui.ctx(), popup::button_menu(response.id));
            let colour = icon_colour(
                &t,
                self.enabled,
                on || open || icon_lit(ui.ctx(), &response),
            );
            let size = self.icon.size();
            let icon = if on { self.icon.filled() } else { self.icon };
            icon.paint(
                painter,
                Rect::from_center_size(rect.center(), vec2(size, size)),
                colour,
            );
            if response.has_focus() {
                focus_ring(painter, rect, &t);
            }
        }
        let (name, enabled, toggled) = (self.name, self.enabled, self.toggled);
        response.widget_info(|| match toggled {
            Some(on) => WidgetInfo::selected(Role::Button, enabled, on, name),
            None => WidgetInfo::labeled(Role::Button, enabled, name),
        });
        let tip = self.tip.unwrap_or(name);
        let tip = match self.shortcut {
            Some(keys) => {
                report_shortcut(ui, response.id, keys);
                format!("{tip} ({keys})")
            }
            None => tip.to_owned(),
        };
        response.on_hover_text(tip)
    }
}

/// A button with a word, and an icon before it: Import…, Export…, Reset.
pub struct ToolButton<'a> {
    icon: Option<Icon>,
    text: &'a str,
    enabled: bool,
    tip: Option<&'a str>,
    shortcut: Option<&'a str>,
}

impl<'a> ToolButton<'a> {
    /// `text` is translated.
    pub fn new(text: &'a str) -> Self {
        ToolButton {
            icon: None,
            text,
            enabled: true,
            tip: None,
            shortcut: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// The keys that do the same, in the platform's spelling (`Ctrl+E`): the tooltip is the
    /// text or the tip, followed by them.
    pub fn shortcut(mut self, keys: &'a str) -> Self {
        self.shortcut = Some(keys);
        self
    }

    /// The width [`ToolButton::show`] takes: for a caller that fits its parts to its room.
    pub fn width(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        let galley = ui.painter().layout_no_wrap(
            self.text.to_owned(),
            egui::FontId::proportional(t.text.body),
            t.colors.text,
        );
        self.width_of(galley.size().x, &t)
    }

    /// The width with a text `text` points wide.
    fn width_of(&self, text: f32, t: &crate::Theme) -> f32 {
        let icon = self.icon.map_or(0.0, |i| i.size() + t.spacing.s);
        text + icon + 2.0 * t.spacing.m
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let colour = if self.enabled {
            t.colors.text
        } else {
            t.colors.text_disabled
        };
        let galley = ui.painter().layout_no_wrap(
            self.text.to_owned(),
            egui::FontId::proportional(t.text.body),
            colour,
        );
        let size = vec2(self.width_of(galley.size().x, &t), t.sizes.top_bar - 14.0);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if self.enabled && response.hovered() {
                painter.rect_filled(rect, t.shape.radius, t.colors.hover);
            }
            let mut x = rect.left() + t.spacing.m;
            if let Some(icon) = self.icon {
                let s = icon.size();
                // Only the icon is a step quieter; the word keeps the text colour.
                let lit = icon_lit(ui.ctx(), &response);
                icon.paint(
                    painter,
                    Rect::from_min_size(egui::pos2(x, rect.center().y - s / 2.0), vec2(s, s)),
                    icon_colour(&t, self.enabled, lit),
                );
                x += s + t.spacing.s;
            }
            painter.galley(
                egui::pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                colour,
            );
            if response.has_focus() {
                focus_ring(painter, rect, &t);
            }
        }
        let (text, enabled) = (self.text, self.enabled);
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, text));
        match (self.tip, self.shortcut) {
            (tip, Some(keys)) => {
                report_shortcut(ui, response.id, keys);
                response.on_hover_text(format!("{} ({keys})", tip.unwrap_or(text)))
            }
            (Some(tip), None) => response.on_hover_text(tip),
            (None, None) => response,
        }
    }
}
