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

//! The tag, checked by what it paints and by what screen
//! readers hear.

use apertrust_ui::components::Tag;
use apertrust_ui::{Builtin, Theme};
use egui::Shape;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn drawn(tip: Option<&'static str>) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(120.0, 40.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(move |ui| {
            let tag = Tag::new("AWB");
            match tip {
                Some(tip) => tag.tip(tip).show(ui),
                None => tag.show(ui),
            };
        });
    harness.run();
    harness
}

/// Light with dark letters, as a checked box: `check` behind, the letters in `check_mark`,
/// no border; 14 points high, the letters' width and 4 points each side wide.
#[test]
fn a_tag_is_check_behind_check_mark_with_no_border() {
    let harness = drawn(None);
    let t = apertrust_ui::theme(&harness.ctx);
    let mut fills = Vec::new();
    let mut letters = Vec::new();
    for clipped in &harness.output().shapes {
        match &clipped.shape {
            // The panel behind it aside.
            Shape::Rect(rect) if rect.fill != t.colors.panel => {
                fills.push((rect.rect, rect.fill, rect.stroke.width));
            }
            Shape::Text(text) => letters.push(text.fallback_color),
            _ => {}
        }
    }
    let [(rect, fill, border)] = fills[..] else {
        panic!("one rectangle, not {fills:?}");
    };
    assert_eq!((fill, border), (t.colors.check, 0.0));
    assert_eq!(rect.height(), 14.0);
    assert!(rect.width() > 2.0 * t.spacing.xs, "{rect:?}");
    assert_eq!(letters, [t.colors.check_mark]);
}

/// Screen readers hear its letters as a label, and what it stands for as its description;
/// it takes no focus.
#[test]
fn screen_readers_hear_its_letters_and_what_they_stand_for() {
    let mut harness = drawn(Some("White balance: Auto"));
    let tag = harness.get_by_role_and_label(Role::Label, "AWB");
    let node = tag.accesskit_node();
    assert_eq!(node.description().as_deref(), Some("White balance: Auto"));
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert_eq!(harness.ctx.memory(|m| m.focused()), None);
    let bare = drawn(None);
    let node = bare.get_by_label("AWB").accesskit_node();
    assert_eq!(node.description(), None);
}
