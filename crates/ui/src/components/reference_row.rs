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

//! The reference row, one line (decision 88; `docs/gui.md`, Tool panels from
//! definitions): a value kept across photos, as Exposure's reference under Picker target.
//!
//! - **Its parts,** start to end: the label; then at the row's end the reference's patch,
//!   its value ("L 62 %") and the photo it was measured on ("· IMG_2041", "· this photo"),
//!   in the small text, the whole value in `text_secondary`, or in `text_disabled`, the L
//!   and the name, once the photo is gone, its tooltip saying so; then the picker that
//!   measures it ([`PickerButton`]) and ⋯ ([`IconButton`]), whose menu holds Clear
//!   Reference and Set Reference from an Area. With none set it shows its text for none,
//!   and while its picker measures, its text for that, in `text_secondary`, with no patch.
//! - **The name is cut first,** with …, where the row is too narrow; the label next.
//!   Screen readers hear both whole.
//! - **To AccessKit,** a label, its words and its value; the picker a toggle button and ⋯
//!   a button with a menu, each a focus stop.

use egui::accesskit::Role;
use egui::{Align, Color32, FontId, Layout, Rect, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::{IconButton, PickerAction, PickerButton, pull_down, truncated};
use crate::Icon;
use crate::popup::{self, Entry, Item};
use crate::theme;

/// What the reference row shows of its reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Reference<'a> {
    /// None set: its text for none, translated.
    None(&'a str),
    /// Its picker measuring: its text for that, translated.
    Measuring(&'a str),
    /// One set: its patch, its value, translated ("L 62 %"), and its photo's words
    /// ("· IMG_2041"), none where no photo is known, with why it is dimmed where the photo
    /// is gone.
    Set {
        patch: Color32,
        value: &'a str,
        photo: Option<&'a str>,
        gone: Option<&'a str>,
    },
}

/// What the person did to the row this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceAction {
    /// Its picker's button was pressed.
    Picker(PickerAction),
    /// ⋯ › Clear Reference.
    Clear,
    /// ⋯ › Set Reference from an Area: as its picker's press.
    Set,
}

pub struct ReferenceRow<'a> {
    label: &'a str,
    promise: Option<&'a str>,
    reference: Reference<'a>,
    picker: PickerButton<'a>,
    /// ⋯'s name, and its menu's two items.
    more: (&'a str, &'a str, &'a str),
    enabled: bool,
}

impl<'a> ReferenceRow<'a> {
    /// `label` is the row's, translated; `picker` the button of the picker that measures
    /// it; `more` ⋯'s name, then Clear Reference's and Set Reference from an Area's words,
    /// translated.
    pub fn new(
        label: &'a str,
        reference: Reference<'a>,
        picker: PickerButton<'a>,
        more: (&'a str, &'a str, &'a str),
    ) -> Self {
        ReferenceRow {
            label,
            promise: None,
            reference,
            picker,
            more,
            enabled: true,
        }
    }

    /// The row's tooltip, translated.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    /// A disabled row is dimmed: ⋯ takes no press. Its picker is enabled or not as its own
    /// button says.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<ReferenceAction> {
        let t = theme(ui.ctx());
        let (row, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        let field = t.sizes.field;
        let gap = t.spacing.xs;
        // The two buttons at the row's end, the picker before ⋯.
        let buttons = 2.0 * field + gap;
        let mut action = None;
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(row.right() - buttons, row.top()),
                    row.max,
                ))
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let (clear, set) = (self.more.1, self.more.2);
                let can_clear = matches!(self.reference, Reference::Set { .. });
                let more = IconButton::new(Icon::Options, self.more.0)
                    .size(field)
                    .enabled(self.enabled)
                    .show(ui);
                // A disabled row's menu closes and is never answered, even one left open
                // as the row turned disabled.
                let chosen = if self.enabled {
                    pull_down(ui, &more, self.more.0, || {
                        vec![
                            Entry::Item(Item::new("clear", clear).enabled(can_clear)),
                            Entry::Item(Item::new("set", set)),
                        ]
                    })
                } else {
                    let menu = popup::button_menu(more.id);
                    popup::dismiss(ui.ctx(), menu);
                    let _ = popup::answer(ui.ctx(), menu);
                    None
                };
                match chosen.as_deref() {
                    Some("clear") => action = Some(ReferenceAction::Clear),
                    Some("set") => action = Some(ReferenceAction::Set),
                    _ => {}
                }
                let (_, pressed) = self.picker.show(ui);
                if let Some(pressed) = pressed {
                    action = Some(ReferenceAction::Picker(pressed));
                }
            },
        );

        let font = FontId::proportional(t.text.small);
        let body = FontId::proportional(t.text.body);
        let (text, secondary) = if self.enabled {
            (t.colors.text, t.colors.text_secondary)
        } else {
            (t.colors.text_disabled, t.colors.text_disabled)
        };
        // The reference's words, end to start, before the buttons.
        let end = row.right() - buttons - t.spacing.s;
        let painter = ui.painter();
        let (first, photo, gone, patch) = match self.reference {
            Reference::None(words) | Reference::Measuring(words) => (words, None, None, None),
            Reference::Set {
                patch,
                value,
                photo,
                gone,
            } => (value, photo, gone, Some(patch)),
        };
        // The whole value in `text_secondary`, L and name; both `text_disabled` once the
        // photo is gone.
        let value_colour = if gone.is_some() {
            t.colors.text_disabled
        } else {
            secondary
        };
        let first_galley = painter.layout_no_wrap(first.to_owned(), font.clone(), value_colour);
        let label_width = painter
            .layout_no_wrap(self.label.to_owned(), body.clone(), text)
            .size()
            .x;
        let side = t.text.small + t.spacing.xxs;
        let patch_room = patch.map_or(0.0, |_| side + gap);
        // The photo's name takes what the label leaves it, cut first.
        let fixed = first_galley.size().x + patch_room + t.spacing.l;
        let room = (end - row.left() - fixed).max(0.0);
        let photo_room = (room - label_width).max(0.0);
        let photo_galley = photo.map(|photo| {
            painter.layout_job(truncated(photo, font.clone(), photo_room, value_colour))
        });
        let centred = |height: f32| row.center().y - height / 2.0;
        let mut x = end;
        let mut photo_rect = None;
        if let Some(galley) = photo_galley {
            x -= galley.size().x;
            let at = pos2(x, centred(galley.size().y));
            photo_rect = Some(Rect::from_min_size(at, galley.size()));
            painter.galley(at, galley, value_colour);
            x -= gap;
        }
        x -= first_galley.size().x;
        painter.galley(
            pos2(x, centred(first_galley.size().y)),
            first_galley,
            value_colour,
        );
        if let Some(colour) = patch {
            x -= gap + side;
            let rect = Rect::from_min_size(pos2(x, row.center().y - side / 2.0), vec2(side, side));
            painter.rect_filled(rect, t.shape.radius_small, colour);
            painter.rect_stroke(
                rect,
                t.shape.radius_small,
                egui::Stroke::new(1.0, t.colors.border),
                egui::StrokeKind::Inside,
            );
        }
        let label_room = (x - t.spacing.l - row.left()).max(0.0);
        let label = painter.layout_job(truncated(self.label, body, label_room, text));
        painter.galley(pos2(row.left(), centred(label.size().y)), label, text);

        // One label to screen readers, its words and its value whole.
        let words = Rect::from_min_max(row.min, pos2(end, row.max.y));
        let response = ui.interact(
            words,
            ui.make_persistent_id(("reference", self.label)),
            Sense::hover(),
        );
        let heard = match photo {
            Some(photo) => format!("{} {first} {photo}", self.label),
            None => format!("{} {first}", self.label),
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Label, self.enabled, &heard));
        let mut tip: Vec<&str> = Vec::new();
        if let (Some(why), Some(rect)) = (gone, photo_rect)
            && ui.rect_contains_pointer(rect)
        {
            tip.push(why);
        }
        if let Some(promise) = self.promise {
            tip.push(promise);
        }
        if let Some(why) = gone {
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_description(why));
        }
        if !tip.is_empty() {
            response.on_hover_text(tip.join("\n"));
        }
        action
    }
}
