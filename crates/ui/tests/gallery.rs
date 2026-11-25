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

use apertrust_ui::components::{CaptionStyle, GridCell, Toolbar};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Locale, Theme, Translator};
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, SnapshotResults, try_image_snapshot};

/// The gallery's page width, in points.
const WIDTH: f32 = 600.0;

/// The tallest part of the page one image holds: GPUs make textures of at most 8192 pixels
/// a side, so a taller gallery is photographed in parts, each from a little above where the
/// one before ends, the last to the page's end.
const PART: f32 = 8000.0;
const OVERLAP: f32 = 40.0;

/// Photographs the whole gallery on the panel's colour: `name`, its top, with the menu
/// button's menu open, and where the page is taller than [`PART`], `name-end`, the next
/// part, and `name-end-2` and so on while more is left, down to Spacing.
fn photograph(which: Builtin, locale: Locale, name: &str) {
    let height = page_height(which, locale);
    // Every part's results in one, as kittest asks of several harnesses in a test.
    let mut results = SnapshotResults::new();
    let mut top = part(which, locale, 0.0, height.min(PART), true);
    top.snapshot(name);
    results.extend_harness(&mut top);
    let mut from = PART - OVERLAP;
    let mut index = 1;
    while from + OVERLAP < height {
        let mut end = part(which, locale, from, (height - from).min(PART), false);
        let suffix = if index == 1 {
            "end".to_owned()
        } else {
            format!("end-{index}")
        };
        end.snapshot(format!("{name}-{suffix}"));
        results.extend_harness(&mut end);
        from += PART - OVERLAP;
        index += 1;
    }
    results.unwrap();
}

/// How tall the gallery is drawn, measured on a page taller than it, never rendered.
fn page_height(which: Builtin, locale: Locale) -> f32 {
    let harness = part(which, locale, 0.0, 4.0 * PART, true);
    harness
        .ctx
        .data(|d| d.get_temp::<f32>(height_id()))
        .expect("the page measured")
        .ceil()
}

fn height_id() -> egui::Id {
    egui::Id::unique("gallery page height")
}

/// The gallery from `from` points down, `height` points of it.
fn part(which: Builtin, locale: Locale, from: f32, height: f32, menu: bool) -> Harness<'static> {
    let mut gallery = Gallery::new(which);
    if menu {
        gallery = gallery.with_menu_open();
    }
    gallery.locale = locale;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(WIDTH, height))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui(move |ui| {
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            // The page drawn `from` points up, in a child that starts above the window.
            let page = egui::Rect::from_min_size(
                ui.max_rect().min - egui::vec2(0.0, from),
                egui::vec2(ui.max_rect().width(), f32::INFINITY),
            );
            let mut page = ui.new_child(egui::UiBuilder::new().max_rect(page));
            gallery.show(&mut page);
            let height = page.min_rect().height();
            ui.data_mut(|d| d.insert_temp(height_id(), height));
        });
    harness.run();
    harness
}

#[test]
fn the_gallery_in_the_dark_theme() {
    photograph(Builtin::Dark, Locale::EnGb, "gallery-dark");
}

#[test]
fn the_gallery_in_the_grey_theme() {
    photograph(Builtin::Grey, Locale::EnGb, "gallery-grey");
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

/// The Tools panel at its narrowest, 240 points, in the pseudo-locale, with the pointer on
/// its edge: the tab bar folds tabs into », and the sections and the scopes still fit.
#[test]
fn the_tools_panel_at_its_narrowest_in_the_pseudo_locale() {
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
            gallery.show_tools_panel(ui, 240.0);
        });
    harness.run();
    let words = Translator::new(Locale::Pseudo);
    let name = words.text("tools-panel-width");
    let edge = harness.get_by_role_and_label(Role::Splitter, &name);
    assert_eq!(edge.accesskit_node().numeric_value(), Some(240.0));
    assert!(
        harness
            .query_by_label(&words.text("panel-more-tabs"))
            .is_some()
    );
    harness.hover_at(edge.rect().center());
    harness.run();
    harness.snapshot("tools-panel-narrowest-pseudo");
}

/// The Browser cell's No preview yet plate (`docs/gui.md`, The cell) at Small, Medium,
/// Large and Extra large, labels on, then off, in every built-in theme: the glyph and the
/// file's type, or the file name where no caption shows, a ring and a mark on two of them.
#[test]
fn the_no_preview_plate_in_every_theme() {
    let mut results = SnapshotResults::new();
    for (which, name) in [
        (Builtin::Dark, "grid-cell-no-preview-dark"),
        (Builtin::Grey, "grid-cell-no-preview-grey"),
        (Builtin::Light, "grid-cell-no-preview-light"),
        (Builtin::HighContrast, "grid-cell-no-preview-high-contrast"),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(700.0, 590.0))
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(which));
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui(|ui| {
                // The Browser's own ground.
                let surface = apertrust_ui::theme(ui.ctx()).colors.surface;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, surface);
                ui.add_space(10.0);
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                // Each cell's own file, so each keeps an id of its own.
                let rows = [
                    (
                        true,
                        [
                            "_1010042.ORF",
                            "SAM_0042.SRW",
                            "CF000042.IIQ",
                            "B0000042.3FR",
                        ],
                    ),
                    (
                        false,
                        [
                            "_1010043.ORF",
                            "SAM_0043.SRW",
                            "CF000043.IIQ",
                            "Wedding_0043.3FR",
                        ],
                    ),
                ];
                for (labels, files) in rows {
                    ui.horizontal_top(|ui| {
                        ui.add_space(10.0);
                        for (side, file) in [84.0, 120.0, 176.0, 256.0].into_iter().zip(files) {
                            GridCell::new(file)
                                .no_preview(true)
                                .side(side)
                                .labels(labels)
                                .selected(labels && side == 120.0)
                                .active(!labels && side == 176.0)
                                .edited(!labels && side == 176.0)
                                .show(ui);
                        }
                    });
                }
            });
        harness.run();
        harness.snapshot(name);
        results.extend_harness(&mut harness);
    }
    results.unwrap();
}

/// The Settings controls (`docs/gui.md`, The Settings dialog), each in its row, in every
/// built-in theme: a combo changed and one at its default, a slider, segmented choices,
/// switches, chips, a text area, a number, a pattern, a folder, a readout, an action, and
/// the switch in each state.
#[test]
fn the_settings_controls_in_every_theme() {
    let mut results = SnapshotResults::new();
    for (which, name) in [
        (Builtin::Dark, "settings-controls-dark"),
        (Builtin::Grey, "settings-controls-grey"),
        (Builtin::Light, "settings-controls-light"),
        (Builtin::HighContrast, "settings-controls-high-contrast"),
    ] {
        let mut gallery = Gallery::new(which);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(600.0, 800.0))
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(which));
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui(move |ui| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new()
                    .inner_margin(20.0)
                    .show(ui, |ui| gallery.show_settings(ui));
            });
        harness.run();
        harness.snapshot(name);
        results.extend_harness(&mut harness);
    }
    results.unwrap();
}

/// Every focus stop of the settings controls has a name for screen readers, a label's its
/// words: each control,
/// its buttons and its value, none a part the keys have nothing to do with.
#[test]
fn every_focus_stop_of_the_settings_controls_is_named() {
    use egui::accesskit::Action;
    use egui_kittest::kittest::By;
    let mut gallery = Gallery::new(Builtin::Dark);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 800.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
        })
        .build_ui(move |ui| {
            egui::Frame::new()
                .inner_margin(20.0)
                .show(ui, |ui| gallery.show_settings(ui));
        });
    harness.run();
    let stops = harness
        .query_all(By::new().predicate(|node| node.data().supports_action(Action::Focus)))
        .count();
    assert!(stops > 20, "the controls' focus stops: {stops}");
    let unnamed: Vec<String> = harness
        .query_all(By::new().predicate(|node| {
            node.data().supports_action(Action::Focus)
                && node
                    .label()
                    .or_else(|| (node.role() == Role::Label).then(|| node.value()).flatten())
                    .is_none_or(|name| name.trim().is_empty())
        }))
        .map(|node| {
            let node = node.accesskit_node();
            format!("{:?} at {:?}", node.role(), node.bounding_box())
        })
        .collect();
    assert!(unnamed.is_empty(), "unnamed focus stops: {unnamed:?}");
}

/// The size of one state's photograph of Linux's caption buttons: the end of a Toolbar
/// with labels, 62 points tall.
const CAPTION_STRIP: egui::Vec2 = egui::vec2(180.0, 62.0);

/// The end of an empty Toolbar, the title bar with the caption buttons in Linux's style,
/// in theme `which`, the window `maximised` or not; tooltips never show, so each state is
/// the buttons' own.
fn linux_caption(which: Builtin, maximised: bool) -> Harness<'static> {
    Harness::builder()
        .with_size(CAPTION_STRIP)
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(which));
            apertrust_ui::set_locale(ctx, Locale::EnGb);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = f32::INFINITY);
        })
        .build_ui(move |ui| {
            // The Toolbar from edge to edge, as in the window.
            let whole = ui.ctx().content_rect();
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(whole));
            Toolbar::new()
                .labels(true)
                .title_bar(true)
                .caption_buttons(Some(CaptionStyle::Linux))
                .maximised(maximised)
                .show(&mut ui);
        })
}

/// Linux's caption buttons in every built-in theme, each theme's states one above the
/// next in one picture: at rest; Minimise, Maximise and Close under the pointer; each held
/// down; Minimise focused by the keyboard; and maximised, Restore in the middle.
#[test]
fn linux_s_caption_buttons_in_every_theme() {
    let mut results = SnapshotResults::new();
    for (which, name) in [
        (Builtin::Dark, "caption-buttons-linux-dark"),
        (Builtin::Grey, "caption-buttons-linux-grey"),
        (Builtin::Light, "caption-buttons-linux-light"),
        (Builtin::HighContrast, "caption-buttons-linux-high-contrast"),
    ] {
        let buttons = ["Minimise", "Maximise", "Close"];
        let mut states = Vec::new();
        let mut rest = linux_caption(which, false);
        rest.run();
        states.push(rest.render().unwrap());
        for pressed in [false, true] {
            for button in buttons {
                let mut harness = linux_caption(which, false);
                harness.run();
                // At the circle's lower right, so the pointer drawn there hides no glyph.
                let at = harness.get_by_label(button).rect().center() + egui::vec2(7.0, 7.0);
                harness.hover_at(at);
                if pressed {
                    harness.event(egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                harness.run();
                states.push(harness.render().unwrap());
            }
        }
        let mut focused = linux_caption(which, false);
        focused.run();
        focused.get_by_label("Minimise").focus();
        focused.run();
        assert!(focused.get_by_label("Minimise").is_focused());
        states.push(focused.render().unwrap());
        let mut maximised = linux_caption(which, true);
        maximised.run();
        assert!(maximised.query_by_label("Restore").is_some());
        states.push(maximised.render().unwrap());

        // One picture, the states one under another, on a canvas rendered as tall as them.
        let rows = states.len() as f32;
        let mut canvas = Harness::builder()
            .with_size(egui::vec2(CAPTION_STRIP.x, CAPTION_STRIP.y * rows))
            .build_ui(|_| {});
        let mut picture = canvas.render().unwrap();
        for (row, state) in states.iter().enumerate() {
            let top = row as u32 * state.height();
            for (x, y, pixel) in state.enumerate_pixels() {
                picture.put_pixel(x, top + y, *pixel);
            }
        }
        results.add(try_image_snapshot(&picture, name));
    }
    results.unwrap();
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
