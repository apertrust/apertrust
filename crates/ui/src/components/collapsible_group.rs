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

//! A collapsible group (decision 29): darktable's collapsible section, a row inside a
//! module that shows or hides the controls under it (`docs/gui.md`, Module panels from
//! definitions).
//!
//! - Its row is darktable's form, not a section header's: the title in [`group_label`]'s
//!   type, a `divider` hairline from just after it to a chevron at the row's end, on the
//!   value fields' end edge, pointing to the end while closed and down while open. While
//!   closed, a dot after the title says controls in it are set, as the caller counts them.
//! - The whole row is the target and one focus stop, with the hover band across it under
//!   the pointer and the focus ring after a key; Enter or Space toggles it. A closed
//!   group's rows are not drawn, so Tab passes them by.
//! - To screen readers it is a button with its open state, named by the title and
//!   described as "N set" while N controls in it are set. While it is open, its rows are
//!   the children of a group named by the title.
//! - Whether it is open is the caller's, kept in settings, never in the edit: the group
//!   reports a toggle and keeps nothing.
//! - The row is a panel line (`sizes.panel_line`, 24 points) with 4 points above it, and
//!   while the group is open 4 points follow its last row.
//! - A group with nothing behind it yet, AgX's Show curve before its graph, is drawn
//!   disabled, as a tab with no rows is ([`CollapsibleGroup::disabled`]): its title and
//!   chevron dimmed, closed whatever the caller keeps, its tooltip saying it is to come,
//!   reporting no toggle, and to screen readers a disabled button.
//!
//! [`group_label`]: super::group_label

use egui::{Rect, Role, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2};

use super::focus_ring;
use super::tabs::group_label_text;
use crate::i18n::Args;
use crate::{Icon, theme, translator};

/// The dot after the title of a closed group with controls set.
const DOT_RADIUS: f32 = 2.5;

pub struct CollapsibleGroup<'a> {
    title: &'a str,
    open: bool,
    set: usize,
    /// The tooltip of a group with nothing behind it yet, drawn disabled.
    disabled: Option<&'a str>,
}

impl<'a> CollapsibleGroup<'a> {
    /// `title` is translated: darktable's label for the section. `open` is the caller's.
    pub fn new(title: &'a str, open: bool) -> Self {
        CollapsibleGroup {
            title,
            open,
            set: 0,
            disabled: None,
        }
    }

    /// Draws the group disabled, with nothing behind it yet: dimmed, closed, `tip` its
    /// tooltip, as a tab with no rows says it is to come.
    pub fn disabled(mut self, tip: &'a str) -> Self {
        self.disabled = Some(tip);
        self
    }

    /// How many of its controls are set, off their defaults for the photo, as the caller
    /// reads them from the mirror: a dot after the title while closed, and "N set" to
    /// screen readers.
    pub fn set(mut self, controls: usize) -> Self {
        self.set = controls;
        self
    }

    /// Draws the group's row and, while it is open, `rows` under it, at the usual inset
    /// and with their usual spacing. Returns true when the user opened or closed it this
    /// frame; the caller keeps the new state.
    pub fn show(self, ui: &mut Ui, rows: impl FnOnce(&mut Ui)) -> bool {
        let t = theme(ui.ctx());
        let id = ui.make_persistent_id(("collapsible group", self.title));
        let enabled = self.disabled.is_none();
        let open = self.open && enabled;
        ui.add_space(t.spacing.xs);
        let (row, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.panel_line),
            Sense::hover(),
        );
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(row, id, sense);

        let title = self.title;
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, title));
        let set = (self.set > 0).then(|| {
            let mut args = Args::new();
            args.set("count", self.set);
            translator(ui.ctx()).text_with("collapsible-group-set", Some(&args))
        });
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_expanded(open);
            if let Some(set) = set {
                node.set_description(set);
            }
        });

        if ui.is_rect_visible(row) {
            let painter = ui.painter();
            let band = row.expand2(vec2(t.spacing.xs, 0.0));
            if response.hovered() && enabled {
                painter.rect_filled(band, t.shape.radius, t.colors.hover);
            }
            let colour = if enabled {
                t.colors.text_secondary
            } else {
                t.colors.text_disabled
            };
            let middle = row.center().y;
            let gap = t.spacing.s;
            // At the row's end, its box on the value fields' end edge.
            let side = Icon::ChevronEnd.size();
            let chevron =
                Rect::from_center_size(pos2(row.right() - side / 2.0, middle), vec2(side, side));
            let dot = !open && self.set > 0;
            let dot_room = if dot { 2.0 * DOT_RADIUS + gap } else { 0.0 };
            // The title takes what the shortest hairline, the dot and the gaps leave.
            let room = chevron.left() - gap - t.spacing.l - gap - dot_room - row.left();
            let galley = painter.layout_job(group_label_text(&t, title, room.max(0.0), colour));
            let mut x = row.left() + galley.size().x + gap;
            painter.galley(
                pos2(row.left(), middle - galley.size().y / 2.0),
                galley,
                colour,
            );
            if dot {
                painter.circle_filled(pos2(x + DOT_RADIUS, middle), DOT_RADIUS, t.colors.text);
                x += 2.0 * DOT_RADIUS + gap;
            }
            painter.hline(
                x..=chevron.left() - gap,
                painter.round_to_pixel_center(middle),
                Stroke::new(t.shape.hairline, t.colors.divider),
            );
            let icon = if open {
                Icon::ChevronDown
            } else {
                Icon::ChevronEnd
            };
            icon.paint(painter, chevron, colour);
            if response.has_focus() {
                focus_ring(painter, band, &t);
            }
        }

        if let Some(tip) = self.disabled {
            // The row senses hover only, so its tooltip is a live one.
            response.clone().on_hover_text(tip);
        }
        if open {
            ui.scope_builder(
                UiBuilder::new()
                    .id_salt(("collapsible group rows", title))
                    .accessibility_role(Role::Group)
                    .accessibility_label(title),
                rows,
            );
            // The group's end shows: 4 points after its last row while it is open.
            ui.add_space(t.spacing.xs);
        }
        enabled && response.clicked()
    }
}
