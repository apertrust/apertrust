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

//! The gallery photographed in every built-in theme and in the pseudo-locale (`docs/gui.md`,
//! Testing). The images are in `tests/snapshots`; a changed one is reviewed like a changed
//! golden image, and accepted with `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test
//! gallery`.
//!
//! It renders through wgpu, on CI's software GPU, and through AccessKit, whose tree the
//! harness checks too.

use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Locale, Theme, Translator};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The gallery's page, at a fixed size that holds all of it, on the panel's colour.
const PAGE: egui::Vec2 = egui::vec2(600.0, 8050.0);

fn photograph(which: Builtin, locale: Locale, name: &str) {
    let mut gallery = Gallery::new(which).with_menu_open();
    gallery.locale = locale;
    let mut harness = Harness::builder()
        .with_size(PAGE)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui(|ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            gallery.show(ui);
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_gallery_in_the_dark_theme() {
    photograph(Builtin::Dark, Locale::EnGb, "gallery-dark");
}

#[test]
fn the_gallery_in_the_light_theme() {
    photograph(Builtin::Light, Locale::EnGb, "gallery-light");
}

#[test]
fn the_gallery_in_the_high_contrast_theme() {
    photograph(Builtin::HighContrast, Locale::EnGb, "gallery-high-contrast");
}

/// The components' own words in the pseudo-locale, accented and 40% longer, to show text
/// that is clipped or escaped translation. The gallery's labels stay English.
#[test]
fn the_gallery_in_the_pseudo_locale() {
    photograph(Builtin::Dark, Locale::Pseudo, "gallery-pseudo");
}

/// The side panel at its narrowest, 240 points, in the pseudo-locale, with the pointer on
/// its edge: the tab bar folds tabs into », and the sections and the scopes still fit.
#[test]
fn the_side_panel_at_its_narrowest_in_the_pseudo_locale() {
    let mut gallery = Gallery::new(Builtin::Dark);
    gallery.locale = Locale::Pseudo;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(280.0, 600.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, Locale::Pseudo);
        })
        .build_ui(|ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            gallery.show_side_panel(ui, 240.0);
        });
    harness.run();
    let words = Translator::new(Locale::Pseudo);
    let name = words.text("panel-width");
    let edge = harness.get_by_role_and_label(Role::Splitter, &name);
    assert_eq!(edge.accesskit_node().numeric_value(), Some(240.0));
    assert!(
        harness
            .query_by_label(&words.text("panel-more-tabs"))
            .is_some()
    );
    harness.hover_at(edge.rect().center());
    harness.run();
    harness.snapshot("side-panel-narrowest-pseudo");
}

#[test]
fn the_theme_switcher_changes_the_theme() {
    let mut gallery = Gallery::new(Builtin::Dark);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(820.0, 400.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| gallery.show(ui));
    harness.get_by_label("light").click();
    harness.run();
    assert_eq!(
        *apertrust_ui::theme(&harness.ctx),
        Theme::builtin(Builtin::Light)
    );
}
