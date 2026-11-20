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

//! A plain group (decision 100): darktable's section label, a title over the rows it holds,
//! always open, with no chevron, as Export's Storage options and White balance's Scene
//! illuminant temp (`docs/gui.md`, Tool panels from definitions). Its other kind, the
//! collapsible group, is [`CollapsibleGroup`]; both share the title's type
//! ([`group_label_text`]) and the first-row rule.
//!
//! - The title is caption-sized Medium capitals in `text_secondary`, 2 points over its
//!   first row, on one line.
//! - 8 points come above it, none where it is the first row of its container, a tool's
//!   body, a tab or a group ([`PlainGroup::first`]), so a tool's first row is 4 points under
//!   its header, or 8 under its tab strip, whatever it is. Nothing follows its last row.
//! - To screen readers it is a group named by its title, its rows its children; the title
//!   itself is no node of its own, the group's name saying it.
//!
//! [`CollapsibleGroup`]: super::CollapsibleGroup

use egui::text::LayoutJob;
use egui::{Color32, FontFamily, FontId, Role, Sense, TextFormat, Ui, UiBuilder, accesskit, vec2};

use crate::{Theme, fonts, theme};

pub struct PlainGroup<'a> {
    title: &'a str,
    first: bool,
}

impl<'a> PlainGroup<'a> {
    /// `title` is translated: darktable's section label.
    pub fn new(title: &'a str) -> Self {
        PlainGroup {
            title,
            first: false,
        }
    }

    /// Whether the group is the first row its container draws, a tool's body, a tab or a
    /// group: then no space comes above it (decision 100). The container says so, never
    /// the tool.
    pub fn first(mut self, first: bool) -> Self {
        self.first = first;
        self
    }

    /// Draws the title and, under it, `rows`, at the usual inset and with their usual
    /// spacing, inside a group named by the title.
    pub fn show<R>(self, ui: &mut Ui, rows: impl FnOnce(&mut Ui) -> R) -> R {
        let t = theme(ui.ctx());
        if !self.first {
            ui.add_space(t.spacing.m);
        }
        let title = self.title;
        group_scope(ui, ("plain group", title), title, |ui| {
            let colour = t.colors.text_secondary;
            let galley =
                ui.painter()
                    .layout_job(group_label_text(&t, title, f32::INFINITY, colour));
            // Sensing hover only, the title is no focus stop and no node of its own.
            let (rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), galley.size().y + t.spacing.xxs),
                Sense::hover(),
            );
            if ui.is_rect_visible(rect) {
                ui.painter().galley(rect.min, galley, colour);
            }
            rows(ui)
        })
    }
}

/// Draws `add` inside a group named `title` to screen readers, as both kinds hold their
/// rows (decision 100), the group's bounds what it drew, so a screen reader can show
/// where it is.
pub(super) fn group_scope<R>(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    title: &str,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let inner = ui.scope_builder(
        UiBuilder::new()
            .id_salt(salt)
            .accessibility_role(Role::Group)
            .accessibility_label(title),
        add,
    );
    let rect = inner.response.rect;
    ui.ctx().accesskit_node_builder(inner.response.id, |node| {
        node.set_bounds(accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
    });
    inner.inner
}

/// A group's title, as a plain group and a collapsible group's row draw it: caption sized
/// Medium capitals, a little apart, on one line at most `width` wide, cut at its end with …
/// when longer.
pub(super) fn group_label_text(t: &Theme, text: &str, width: f32, colour: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        TextFormat {
            font_id: FontId::new(t.text.caption, FontFamily::Name(fonts::MEDIUM.into())),
            color: colour,
            extra_letter_spacing: 0.55,
            ..Default::default()
        },
    );
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job
}
