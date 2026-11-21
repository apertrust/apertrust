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

//! A boolean's switch in a settings row: `sizes.setting_switch_width` ×
//! `setting_switch_height` (32 × 18) at the column's end, its knob
//! `setting_switch_knob` (12) across. Quiet as a tool's switch, a size up, in a tool's
//! switch's colours ([`crate::components::Switch`]): on is `button` with a `text_strong`
//! knob, off `sunken` with a `control_outline` knob, the high-contrast theme's edged. The
//! row around it is its target too ([`super::SettingRow::whole_row`]). To screen readers it
//! is a switch with its state.

use egui::{Rect, Response, Role, Ui, WidgetInfo, vec2};

use super::super::ButtonState;
use super::super::plate::{Touch, allocate, undimmed_painter};
use super::super::switch::paint;
use crate::theme;

pub struct Switch<'a> {
    on: &'a mut bool,
    name: &'a str,
    state: Option<ButtonState>,
}

impl<'a> Switch<'a> {
    /// `name` is the row's label, translated; a click or Space toggles `on`.
    pub fn new(on: &'a mut bool, name: &'a str) -> Self {
        Switch {
            on,
            name,
            state: None,
        }
    }

    /// Draws it in `state` whatever the pointer and the keys do, as the gallery shows each
    /// state.
    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = Some(state);
        self
    }

    /// Draws it at the end of `ui`'s width, a box high; the response is marked changed when
    /// a click or a key toggled it.
    pub fn show(self, ui: &mut Ui) -> Response {
        let t = theme(ui.ctx());
        let enabled = ui.is_enabled() && self.state != Some(ButtonState::Disabled);
        let width = t.sizes.setting_switch_width;
        let height = t.sizes.setting_control.max(t.sizes.setting_switch_height);
        let mut end = super::at_end(ui, width, height);
        let (rect, mut response) = allocate(&mut end, vec2(width, height), enabled);
        if response.clicked() {
            *self.on = !*self.on;
            response.mark_changed();
        }
        let on = *self.on;
        if ui.is_rect_visible(rect) {
            let touch = Touch::of(ui.ctx(), &response, enabled, self.state);
            let track =
                Rect::from_center_size(rect.center(), vec2(width, t.sizes.setting_switch_height));
            let knob = t.sizes.setting_switch_knob;
            paint(&undimmed_painter(ui), track, knob, on, touch, &t);
        }
        response.widget_info(|| WidgetInfo::selected(Role::Switch, enabled, on, self.name));
        response
    }
}
