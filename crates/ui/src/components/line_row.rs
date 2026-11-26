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

//! The line row, one line (`docs/gui.md`, Line rows and end items; decision 107):
//! darktable's widgets packed in a box, a start and end items, as AgX's Auto tune levels
//! with its picker and its camera button. One 24-point line (`sizes.panel_line`).
//!
//! - **The start** is nothing; a label, a plain group's words; a control's label,
//!   darktable's reset label, a double-click, or Delete while it has the focus, resetting
//!   the control; or a readout, a text with its tooltip and a swatch filling the line up
//!   to the end items.
//! - **The end items** ([`EndItem`]) sit at the line's end, left to right in reading and
//!   Tab order, `spacing.xs` apart: a picker's button, an icon button 18 by 18 with no
//!   plate, a text button, or a menu button, an icon button opening a pull-down of the
//!   popup service's and reported expanded while it is open. A boolean's row draws the
//!   same items after its switch, at its end ([`super::ToggleRow::end`]).
//!
//! The start is cut first, with …, never an end item.

use egui::accesskit::Role;
use egui::{Align, Color32, Key, Label, Layout, Rect, Sense, Ui, UiBuilder, pos2, vec2};

use super::{Button, ColourSwatch, IconButton, PickerAction, PickerButton, pull_down};
use crate::popup::{self, Entry};
use crate::{Icon, theme};

/// What starts a [`LineRow`].
pub enum LineStart<'a> {
    None,
    /// Translated words, a plain group's, cut with … where the end items leave too little.
    Label(&'a str),
    /// A control's short label, darktable's reset label: its full name is what screen
    /// readers say, its promise the tooltip.
    Control {
        label: &'a str,
        name: &'a str,
        promise: Option<&'a str>,
        enabled: bool,
    },
    /// A readout's text, with its tooltip, and the swatch after it, its colour and
    /// tooltip, filling the line up to the end items.
    Readout {
        text: &'a str,
        promise: &'a str,
        swatch: Option<(Color32, &'a str)>,
    },
}

/// One item at a row's end (decision 107).
pub enum EndItem<'a> {
    /// A colour picker's button.
    Picker(PickerButton<'a>),
    /// An icon button, 18 by 18 with no plate: its translated name, the tooltip's first
    /// words and what screen readers say, and its promise after them.
    Icon {
        icon: Icon,
        name: &'a str,
        promise: Option<&'a str>,
        enabled: bool,
    },
    /// A text button, the design system's [`Button`], as an action row's: its short label,
    /// its full name, what screen readers say, and its promise, the tooltip.
    Text {
        label: &'a str,
        name: &'a str,
        promise: Option<&'a str>,
        enabled: bool,
    },
    /// An icon button that opens a pull-down of `entries`, as [`EndItem::Icon`] but that a
    /// press opens its menu, the popup service's, rather than acting; screen readers hear
    /// that it has a menu and whether it is open.
    Menu {
        icon: Icon,
        name: &'a str,
        promise: Option<&'a str>,
        enabled: bool,
        entries: Vec<Entry>,
    },
}

/// What the user did to an end item this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndAction {
    /// A picker's button pressed, as [`PickerButton`] reports it.
    Picker(PickerAction),
    /// An icon or a text button pressed.
    Pressed,
    /// A menu's item chosen, by its id.
    Chosen(String),
}

/// What the user did to a [`LineRow`] this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineAction {
    /// The control at its start back to its default.
    Reset,
    /// The end item at this index, as it reports it.
    End(usize, EndAction),
}

pub struct LineRow<'a> {
    start: LineStart<'a>,
    end: Vec<EndItem<'a>>,
}

impl<'a> LineRow<'a> {
    pub fn new(start: LineStart<'a>, end: Vec<EndItem<'a>>) -> Self {
        LineRow { start, end }
    }

    pub fn show(self, ui: &mut Ui) -> Option<LineAction> {
        let t = theme(ui.ctx());
        let (row, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        let width = end_width(ui, &self.end);
        let ends = Rect::from_min_max(pos2(row.right() - width, row.top()), row.max);
        // The start takes what the end items leave, a group's gap before them.
        let gap = if self.end.is_empty() {
            0.0
        } else {
            t.spacing.m
        };
        let start = Rect::from_min_max(
            row.min,
            pos2((ends.left() - gap).max(row.left()), row.bottom()),
        );
        let mut action = None;
        let mut inside = ui.new_child(
            UiBuilder::new()
                .max_rect(start)
                .layout(Layout::left_to_right(Align::Center)),
        );
        match self.start {
            LineStart::None => {}
            LineStart::Label(text) => {
                inside.add(Label::new(text).selectable(false).truncate());
            }
            LineStart::Control {
                label,
                name,
                promise,
                enabled,
            } => {
                let response = inside.add_enabled(
                    enabled,
                    Label::new(label)
                        .sense(Sense::click())
                        .selectable(false)
                        .truncate(),
                );
                ui.ctx()
                    .accesskit_node_builder(response.id, |node| node.set_label(name));
                let response = match promise {
                    Some(promise) => response
                        .on_hover_text(promise)
                        .on_disabled_hover_text(promise),
                    None => response,
                };
                // A double-click, or Delete or Backspace while it has the focus, as a
                // slider's label resets its control.
                let keyed = response.has_focus()
                    && ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
                if enabled && (response.double_clicked() || keyed) {
                    action = Some(LineAction::Reset);
                }
            }
            LineStart::Readout {
                text,
                promise,
                swatch,
            } => {
                // The text is cut in what the swatch's two quads leave, so a long language's
                // text never pushes the swatch away.
                let swatch_room = if swatch.is_some() {
                    2.0 * t.sizes.field + inside.spacing().item_spacing.x
                } else {
                    0.0
                };
                let room = (inside.available_width() - swatch_room).max(0.0);
                inside
                    .scope(|ui| {
                        ui.set_max_width(room);
                        ui.add(Label::new(text).selectable(false).truncate())
                    })
                    .inner
                    .on_hover_text(promise);
                if let Some((colour, tip)) = swatch {
                    let room = inside.available_width();
                    ColourSwatch::new(colour, tip).width(room).show(&mut inside);
                }
            }
        }
        let mut items = ui.new_child(
            UiBuilder::new()
                .max_rect(ends)
                .layout(Layout::left_to_right(Align::Center)),
        );
        if let Some((index, pressed)) = show_end(&mut items, self.end) {
            action = Some(LineAction::End(index, pressed));
        }
        action
    }
}

/// How wide `items` are drawn, `spacing.xs` between each two.
pub(super) fn end_width(ui: &Ui, items: &[EndItem]) -> f32 {
    let t = theme(ui.ctx());
    let widths = items.iter().map(|item| match item {
        EndItem::Picker(_) | EndItem::Icon { .. } | EndItem::Menu { .. } => t.sizes.field,
        EndItem::Text { label, .. } => Button::new(label).natural_width(ui),
    });
    let count = items.len() as f32;
    widths.sum::<f32>() + (count - 1.0).max(0.0) * t.spacing.xs
}

/// Draws `items` left to right from where `ui` is, `spacing.xs` apart, and says which was
/// pressed, and how.
pub(super) fn show_end(ui: &mut Ui, items: Vec<EndItem>) -> Option<(usize, EndAction)> {
    let t = theme(ui.ctx());
    ui.spacing_mut().item_spacing.x = t.spacing.xs;
    let mut action = None;
    for (index, item) in items.into_iter().enumerate() {
        let pressed = match item {
            EndItem::Picker(button) => button.show(ui).1.map(EndAction::Picker),
            EndItem::Icon {
                icon,
                name,
                promise,
                enabled,
            } => {
                let tip = joined(name, promise);
                let response = IconButton::new(icon, name)
                    .tip(&tip)
                    .size(t.sizes.field)
                    .enabled(enabled)
                    .show(ui);
                describe(ui, response.id, promise);
                (enabled && response.clicked()).then_some(EndAction::Pressed)
            }
            EndItem::Text {
                label,
                name,
                promise,
                enabled,
            } => {
                let mut button = Button::new(label).enabled(enabled);
                if let Some(promise) = promise {
                    button = button.tip(promise);
                }
                let response = button.show(ui);
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_role(Role::Button);
                    node.set_label(name);
                });
                (enabled && response.clicked()).then_some(EndAction::Pressed)
            }
            EndItem::Menu {
                icon,
                name,
                promise,
                enabled,
                entries,
            } => {
                let tip = joined(name, promise);
                let response = IconButton::new(icon, name)
                    .tip(&tip)
                    .size(t.sizes.field)
                    .enabled(enabled)
                    .show(ui);
                describe(ui, response.id, promise);
                // A disabled button's menu closes and is never answered, even one left
                // open as the button turned disabled.
                if enabled {
                    pull_down(ui, &response, name, || entries).map(EndAction::Chosen)
                } else {
                    let menu = popup::button_menu(response.id);
                    popup::dismiss(ui.ctx(), menu);
                    let _ = popup::answer(ui.ctx(), menu);
                    None
                }
            }
        };
        if let Some(pressed) = pressed {
            action = Some((index, pressed));
        }
    }
    action
}

/// An icon button's tooltip: its name, then its promise on a line of its own.
fn joined(name: &str, promise: Option<&str>) -> String {
    match promise {
        Some(promise) => format!("{name}\n{promise}"),
        None => name.to_owned(),
    }
}

/// Screen readers hear `promise` as the button's description, after its name.
fn describe(ui: &Ui, id: egui::Id, promise: Option<&str>) {
    if let Some(promise) = promise {
        ui.ctx()
            .accesskit_node_builder(id, |node| node.set_description(promise));
    }
}
