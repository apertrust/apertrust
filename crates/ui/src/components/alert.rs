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

//! The body of an alert, drawn in the theme (`docs/gui.md`, Dialogs and alerts; decision
//! 41): the content of a dialog window on a platform with no alert of its own to use. The
//! Mac shows `NSAlert` instead, Linux draws this in the dialog host, and Windows' task
//! dialog is planned.
//!
//! It is a function of a `Ui`, as every dialog's content is, so the dialog host draws it
//! in a window and the tests in a harness. Return presses the default button and Escape the
//! cancel, unless a button has the focus, which takes Return itself; with the suppression
//! box focused, Return still presses the default and Space ticks the box. The buttons sit
//! at the end of the last row in the platform's order, and screen readers and Tab meet them
//! in the order they are seen. Its node is an alert dialog named by the title, unless the
//! window it is drawn in is that node already ([`AlertBody::in_alert_window`]).

use egui::accesskit::Role as AccessRole;
use egui::{Button, Key, Label, Modifiers, RichText, Ui, WidgetInfo};

use crate::alert::{Request, Role};
use crate::theme;

/// The body's width, in points, inside its margin.
pub const WIDTH: f32 = 360.0;

/// Where the default button sits among the others.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonOrder {
    /// Last, so the others come before it: the Mac's and the GNOME and KDE alerts'.
    DefaultLast,
    /// First, the others after it as the request lists them: Windows'.
    DefaultFirst,
}

impl ButtonOrder {
    /// The platform's.
    pub const fn platform() -> ButtonOrder {
        if cfg!(windows) {
            ButtonOrder::DefaultFirst
        } else {
            ButtonOrder::DefaultLast
        }
    }

    /// The request's buttons by index in the order they are seen, from the row's start,
    /// `default` being the request's default button.
    pub fn arrange(self, count: usize, default: usize) -> Vec<usize> {
        let others = (0..count).filter(|i| *i != default);
        let mut order: Vec<usize> = (0..count).filter(|i| *i == default).chain(others).collect();
        if self == ButtonOrder::DefaultLast {
            order.reverse();
        }
        order
    }
}

/// The button an alert answered, and whether its suppression box was ticked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlertResponse {
    pub pressed: usize,
    pub ticked: bool,
}

/// An alert request drawn as a dialog's content.
pub struct AlertBody<'a> {
    request: &'a Request,
    ticked: &'a mut bool,
    order: ButtonOrder,
    /// Whether its scope is a node of its own, an alert dialog named by the title.
    node: bool,
}

impl<'a> AlertBody<'a> {
    pub fn new(request: &'a Request, ticked: &'a mut bool) -> Self {
        AlertBody {
            request,
            ticked,
            order: ButtonOrder::platform(),
            node: true,
        }
    }

    /// Drawn in a window whose own node is the alert dialog named by the title, as the
    /// dialog host's is: no node of its own, so screen readers read the title once.
    pub fn in_alert_window(mut self) -> Self {
        self.node = false;
        self
    }

    /// Where the default button sits; the platform's unless a test says otherwise.
    pub fn order(mut self, order: ButtonOrder) -> Self {
        self.order = order;
        self
    }

    /// Draws the alert in `ui` and returns the button pressed, by a click or by Return or
    /// Escape, with the suppression box's state, if one was.
    pub fn show(self, ui: &mut Ui) -> Option<AlertResponse> {
        let t = theme(ui.ctx());
        let request = self.request;
        let mut pressed = None;
        let scope = ui.scope(|ui| {
            ui.set_width(WIDTH);
            ui.spacing_mut().item_spacing = egui::vec2(t.spacing.s, t.spacing.m);
            ui.add(Label::new(RichText::new(&request.title).heading()).selectable(false));
            ui.add(Label::new(&request.message).selectable(false));
            // Return with the box focused presses the default button, as `NSAlert`'s does,
            // and Space ticks it: the box never sees Return.
            let mut enter_on_box = false;
            if let Some(label) = &request.suppression {
                let key = ui.make_persistent_id("alert suppression box");
                let last: Option<egui::Id> = ui.data(|d| d.get_temp(key));
                let on_box = last.is_some_and(|id| ui.memory(|m| m.has_focus(id)));
                enter_on_box =
                    on_box && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                let id = ui.checkbox(&mut *self.ticked, label).id;
                ui.data_mut(|d| d.insert_temp(key, id));
            }
            pressed = self.buttons(ui);
            if pressed.is_none() && enter_on_box && !request.buttons.is_empty() {
                pressed = Some(request.default);
            }
            // Return with no control focused: a focused control takes it itself, the body's
            // buttons above, or one drawn after the body, as the host's frame's close, which
            // must still see it.
            let focused = ui.ctx().memory(|m| m.focused()).is_some();
            let enter = !focused && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
            let escape = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
            if pressed.is_none() && enter && !request.buttons.is_empty() {
                pressed = Some(request.default);
            }
            if pressed.is_none() && escape {
                pressed = request.cancel();
            }
        });
        let title = &request.title;
        if self.node {
            scope
                .response
                .widget_info(|| WidgetInfo::labeled(AccessRole::AlertDialog, true, title));
        }
        pressed.map(|pressed| AlertResponse {
            pressed,
            ticked: *self.ticked,
        })
    }

    /// The buttons at the end of a row, in the platform's order. Their width is the last
    /// pass's, and a pass that finds it changed is drawn again.
    fn buttons(&self, ui: &mut Ui) -> Option<usize> {
        let t = theme(ui.ctx());
        let id = ui.make_persistent_id("alert buttons");
        let width: Option<f32> = ui.data(|d| d.get_temp(id));
        let mut clicked = None;
        let row = ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - width.unwrap_or(0.0)).max(0.0));
            let start = ui.cursor().min.x;
            let request = self.request;
            for i in self.order.arrange(request.buttons.len(), request.default) {
                let button = &request.buttons[i];
                let mut text = RichText::new(&button.label);
                let widget = match (i, button.role) {
                    (_, Role::Destructive) => Button::new(text.color(t.colors.error)),
                    (i, _) if i == request.default => {
                        text = text.color(t.colors.text_on_accent);
                        Button::new(text).fill(t.colors.accent)
                    }
                    _ => Button::new(text),
                };
                if ui.add(widget).clicked() {
                    clicked = Some(i);
                }
            }
            ui.min_rect().max.x - start
        });
        if width != Some(row.inner) {
            ui.data_mut(|d| d.insert_temp(id, row.inner));
            ui.ctx().request_discard("an alert's buttons measured");
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_last_on_the_mac_and_first_on_windows() {
        assert_eq!(ButtonOrder::DefaultLast.arrange(3, 0), [2, 1, 0]);
        assert_eq!(ButtonOrder::DefaultFirst.arrange(3, 0), [0, 1, 2]);
        assert_eq!(ButtonOrder::DefaultLast.arrange(0, 0), Vec::<usize>::new());
        // A default that is not the first goes where the default goes, the others keeping
        // the order they were listed in.
        assert_eq!(ButtonOrder::DefaultLast.arrange(3, 1), [2, 0, 1]);
        assert_eq!(ButtonOrder::DefaultFirst.arrange(3, 1), [1, 0, 2]);
        assert_eq!(ButtonOrder::DefaultLast.arrange(2, 1), [0, 1]);
    }
}
