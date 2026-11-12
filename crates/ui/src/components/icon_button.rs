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
//!
//! An icon button draws no plate in any state (decision 81; `docs/gui.md`, Checkboxes,
//! text buttons and icon buttons): its icon in `text_secondary` at rest, `text_strong`
//! under the pointer, `accent_text` while held down, and its filled form in `text_strong`
//! while on, a toggle that is on or a button whose menu is open; the focus ring after a
//! key; high contrast's 1-point `button_edge` round it under the pointer. A word on a bar,
//! [`ToolButton`], keeps the quiet plate: none at rest, `button` under the pointer,
//! `button_pressed` while pressed.

use egui::{Color32, Popup, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, WidgetInfo, vec2};

use super::plate::{ButtonState, Plate, Touch, paint_focus, paint_plate, undimmed_painter};
use super::{icon_colour, report_shortcut};
use crate::{Icon, Theme, popup, theme};

pub struct IconButton<'a> {
    icon: Icon,
    name: &'a str,
    toggled: Option<bool>,
    enabled: bool,
    shortcut: Option<&'a str>,
    size: Option<f32>,
    width: Option<f32>,
    tip: Option<&'a str>,
    state: Option<ButtonState>,
}

/// The colour of an icon-only button's icon (decision 81): `text_secondary` at rest and
/// focused, `text_strong` under the pointer and while on, `accent_text` while held down,
/// as feedback and never a state, and `text_disabled` disabled.
fn colour(t: &Theme, touch: Touch, on: bool) -> Color32 {
    let c = &t.colors;
    if !touch.enabled {
        c.text_disabled
    } else if touch.pressed {
        c.accent_text
    } else if on || touch.hovered {
        c.text_strong
    } else {
        c.text_secondary
    }
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
            width: None,
            tip: None,
            state: None,
        }
    }

    /// Draws it in `state` whatever the pointer and the keys do, as the gallery shows each
    /// state.
    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = Some(state);
        self
    }

    /// The tooltip, translated, in place of the name: a button whose tooltip says more,
    /// as the grid bar's Sort names the order ("Sort: capture time, ascending").
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// A toggle: while `on`, its filled icon in `text_strong`, with no plate, as a button
    /// whose menu is open is. An icon with no filled form, one of lines and dots alone,
    /// shows on by its colour alone.
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

    /// The button's width alone, in points, its height staying its side: a section
    /// header's buttons, as narrow as their icon and a control high.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Its height, and its width unless [`width`](Self::width) is given, as
    /// [`show`](Self::show) draws it: its size where given, else its icon and 4 points round
    /// it, at least a control's height.
    pub fn side(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        self.size
            .unwrap_or_else(|| (self.icon.size() + 8.0).max(t.sizes.control))
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let side = self.side(ui);
        let width = self.width.unwrap_or(side);
        let enabled = self.enabled && ui.is_enabled() && self.state != Some(ButtonState::Disabled);
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, side), sense);
        if ui.is_rect_visible(rect) {
            let painter = &undimmed_painter(ui);
            let touch = Touch::of(ui.ctx(), &response, enabled, self.state);
            let open = Popup::is_id_open(ui.ctx(), Popup::default_response_id(&response))
                || popup::is_open(ui.ctx(), popup::button_menu(response.id));
            // On, a toggle that is on or a button whose menu is open: its filled icon, where
            // it has one; never a plate.
            let on = self.toggled == Some(true) || (enabled && open);
            // High contrast outlines the button under the pointer and held down, as its
            // other controls, with `button_edge`; the other themes' is transparent.
            if touch.enabled && (touch.hovered || touch.pressed) && t.colors.button_edge.a() > 0 {
                painter.rect_stroke(
                    rect,
                    t.shape.radius,
                    Stroke::new(t.shape.hairline, t.colors.button_edge),
                    StrokeKind::Inside,
                );
            }
            let size = self.icon.size();
            let icon = if on { self.icon.filled() } else { self.icon };
            icon.paint(
                painter,
                Rect::from_center_size(rect.center(), vec2(size, size)),
                colour(&t, touch, on),
            );
            paint_focus(painter, rect, t.shape.radius, touch, &t);
        }
        let (name, toggled) = (self.name, self.toggled);
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

/// A button with a word, and an icon before it, on a bar: Import…, Export…, Reset. Quiet,
/// a plate only while touched, since plates in a row of words would make a wall of boxes.
pub struct ToolButton<'a> {
    icon: Option<Icon>,
    text: &'a str,
    enabled: bool,
    tip: Option<&'a str>,
    shortcut: Option<&'a str>,
    state: Option<ButtonState>,
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
            state: None,
        }
    }

    /// Draws it in `state` whatever the pointer and the keys do, as the gallery shows each
    /// state.
    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = Some(state);
        self
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
        let enabled = self.enabled && ui.is_enabled() && self.state != Some(ButtonState::Disabled);
        let colour = if enabled {
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
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        if ui.is_rect_visible(rect) {
            let painter = &undimmed_painter(ui);
            let touch = Touch::of(ui.ctx(), &response, enabled, self.state);
            paint_plate(painter, rect, Plate::Quiet, touch, &t);
            let mut x = rect.left() + t.spacing.m;
            if let Some(icon) = self.icon {
                let s = icon.size();
                // Only the icon is a step quieter; the word keeps the text colour.
                icon.paint(
                    painter,
                    Rect::from_min_size(egui::pos2(x, rect.center().y - s / 2.0), vec2(s, s)),
                    icon_colour(&t, enabled, touch.lit()),
                );
                x += s + t.spacing.s;
            }
            painter.galley(
                egui::pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                colour,
            );
            paint_focus(painter, rect, t.shape.radius, touch, &t);
        }
        let text = self.text;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Builtin;

    #[test]
    fn an_icon_button_s_colour_follows_decision_81() {
        let touch = |enabled, hovered, pressed, focused| Touch {
            enabled,
            hovered,
            pressed,
            focused,
        };
        for which in Builtin::ALL {
            let t = Theme::builtin(which);
            let c = &t.colors;
            let rest = touch(true, false, false, false);
            let hover = touch(true, true, false, false);
            let held = touch(true, true, true, false);
            let focus = touch(true, false, false, true);
            let off = touch(false, true, true, false);
            assert_eq!(colour(&t, rest, false), c.text_secondary, "{which:?}");
            assert_eq!(colour(&t, hover, false), c.text_strong, "{which:?}");
            // Held down is feedback, never a state: a toggle that is on takes it too.
            assert_eq!(colour(&t, held, false), c.accent_text, "{which:?}");
            assert_eq!(colour(&t, held, true), c.accent_text, "{which:?}");
            assert_eq!(colour(&t, rest, true), c.text_strong, "{which:?}");
            assert_eq!(colour(&t, hover, true), c.text_strong, "{which:?}");
            assert_eq!(colour(&t, focus, false), c.text_secondary, "{which:?}");
            assert_eq!(colour(&t, off, true), c.text_disabled, "{which:?}");
        }
    }
}
