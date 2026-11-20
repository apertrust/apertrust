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

//! The browser bar's count (`docs/gui.md`, The browser bar): the photos selected over those
//! the Browser shows, "3/120" on one line in its row at a side and a stacked fraction in
//! its column at the bottom, "120" alone with none selected, and in words for screen
//! readers and in its tooltip.

use apertrust_ui::components::{Browser, BrowserBarAction, BrowserPosition, Thumb};
use apertrust_ui::{Locale, Theme};
use egui::accesskit::{Action, Live, Role};
use egui::{Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// What the panel is told, and what its browser bar reported.
struct Counted {
    position: BrowserPosition,
    selected: usize,
    total: usize,
    chosen: Vec<BrowserBarAction>,
}

/// The Browser at `position` counting `selected` of `total` photos, in `locale`; it draws
/// the first eight cells' worth at most.
fn counted(
    position: BrowserPosition,
    selected: usize,
    total: usize,
    locale: Locale,
) -> Harness<'static, Counted> {
    counted_in(600.0, position, selected, total, locale)
}

/// [`counted`] in a window `width` points wide.
fn counted_in(
    width: f32,
    position: BrowserPosition,
    selected: usize,
    total: usize,
    locale: Locale,
) -> Harness<'static, Counted> {
    Harness::builder()
        .with_size(egui::vec2(width, 300.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
        })
        .build_ui_state(
            |ui, state: &mut Counted| {
                let names: Vec<String> = (0..state.total.min(8))
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: i < state.selected,
                    picture: None,
                    rating: None,
                };
                let output = Browser::new(names.len(), Some(0), thumb)
                    .position(state.position)
                    .count(state.selected, state.total)
                    .show(ui);
                state.chosen.extend(output.bar);
            },
            Counted {
                position,
                selected,
                total,
                chosen: Vec::new(),
            },
        )
}

/// The texts drawn this frame inside `within`, with where they lie.
fn texts<T>(harness: &Harness<'_, T>, within: Rect) -> Vec<(String, Rect)> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                text.galley.rect.translate(text.pos.to_vec2()),
            )),
            _ => None,
        })
        .filter(|(_, rect)| within.intersects(*rect))
        .collect()
}

/// The browser bar, in any language.
fn bar<T>(harness: &Harness<'_, T>) -> Rect {
    harness.get_by_role(Role::Toolbar).rect()
}

fn button<T>(harness: &Harness<'_, T>, name: &str) -> Rect {
    harness.get_by_role_and_label(Role::Button, name).rect()
}

#[test]
fn the_browser_bar_shows_the_selected_count_over_the_total() {
    // A row across the panel's top at a side: "3/120" on one line, centred between Sort
    // and Zoom.
    let mut harness = counted(BrowserPosition::Left, 3, 120, Locale::EnGb);
    harness.run();
    let (row, sort, zoom) = (
        bar(&harness),
        button(&harness, "Sort"),
        button(&harness, "Thumbnail size"),
    );
    let drawn = texts(&harness, row);
    let [(text, rect)] = drawn.as_slice() else {
        panic!("{drawn:?}");
    };
    assert_eq!(text, "3/120");
    assert!(sort.right() < rect.left() && rect.right() < zoom.left());
    let middle = (sort.right() + zoom.left()) / 2.0;
    assert!((rect.center().x - middle).abs() <= 1.0, "{rect:?}");
    assert!((rect.center().y - sort.center().y).abs() <= 1.0, "{rect:?}");
    // A column at the bottom: 3 over 120, each centred across it, between Sort and Zoom.
    let mut harness = counted(BrowserPosition::Bottom, 3, 120, Locale::EnGb);
    harness.run();
    let (column, sort, zoom) = (
        bar(&harness),
        button(&harness, "Sort"),
        button(&harness, "Thumbnail size"),
    );
    let drawn = texts(&harness, column);
    let [(over, top), (under, bottom)] = drawn.as_slice() else {
        panic!("{drawn:?}");
    };
    assert_eq!((over.as_str(), under.as_str()), ("3", "120"));
    assert!(sort.bottom() < top.top() && top.bottom() < bottom.top());
    assert!(bottom.bottom() < zoom.top());
    assert!((top.center().x - bottom.center().x).abs() <= 0.5);
    assert!((top.center().x - sort.center().x).abs() <= 1.0);
    // With none selected, the total alone, on one line, in either form.
    for position in [BrowserPosition::Left, BrowserPosition::Bottom] {
        let mut harness = counted(position, 0, 120, Locale::EnGb);
        harness.run();
        let drawn = texts(&harness, bar(&harness));
        assert_eq!(
            drawn.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
            ["120"],
            "{position:?}"
        );
    }
    // Grouped in a row, as the language writes it, never in the column.
    let mut harness = counted(BrowserPosition::Left, 12, 1250, Locale::EnGb);
    harness.run();
    assert_eq!(texts(&harness, bar(&harness))[0].0, "12/1,250");
    let mut harness = counted(BrowserPosition::Left, 12, 1250, Locale::Pl);
    harness.run();
    assert_eq!(texts(&harness, bar(&harness))[0].0, "12/1250");
    // With no photos, nothing, and nothing for screen readers.
    let mut harness = counted(BrowserPosition::Bottom, 0, 0, Locale::EnGb);
    harness.run();
    assert_eq!(texts(&harness, bar(&harness)), []);
    let toolbar = harness.get_by_role_and_label(Role::Toolbar, "Browser bar");
    assert!(toolbar.query_by_role(Role::Label).is_none());
}

#[test]
fn the_count_reads_as_words_to_screen_readers() {
    let mut harness = counted(BrowserPosition::Bottom, 3, 120, Locale::EnGb);
    harness.run();
    // A label in the browser bar's toolbar, polite as it changes, and no Tab stop.
    let toolbar = harness.get_by_role_and_label(Role::Toolbar, "Browser bar");
    let label = toolbar.get_by_role_and_label(Role::Label, "3 of 120 photos selected");
    let node = label.accesskit_node();
    assert_eq!(node.live(), Live::Polite);
    assert!(!node.data().supports_action(Action::Focus));
    // Its tooltip says the same.
    let at = label.rect().center();
    harness.hover_at(at);
    harness.run();
    harness.run();
    let tips = texts(&harness, Rect::EVERYTHING);
    assert!(
        tips.iter().any(|(t, _)| t == "3 of 120 photos selected"),
        "{tips:?}"
    );
    // A right-click on it is the bar's: its options menu.
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, "Browser options")
            .is_some()
    );
    // With none selected, the photos alone; one, as one.
    for (selected, total, locale, words) in [
        (0, 120, Locale::EnGb, "120 photos"),
        (0, 1, Locale::EnGb, "1 photo"),
        (1, 1, Locale::EnGb, "1 of 1 photo selected"),
        (3, 120, Locale::Uk, "Вибрано 3 із 120 знімків"),
        (1, 21, Locale::Uk, "Вибрано 1 із 21 знімка"),
        (0, 3, Locale::Uk, "3 знімки"),
        (0, 12, Locale::Uk, "12 знімків"),
        (2, 22, Locale::Pl, "Zaznaczono 2 z 22 zdjęć"),
        (1, 1, Locale::Pl, "Zaznaczono 1 z 1 zdjęcia"),
        (0, 22, Locale::Pl, "22 zdjęcia"),
        (0, 25, Locale::Pl, "25 zdjęć"),
    ] {
        let mut harness = counted(BrowserPosition::Left, selected, total, locale);
        harness.run();
        harness.get_by_role_and_label(Role::Label, words);
    }
}

#[test]
fn the_column_s_count_fits_its_width() {
    for (selected, total, shown) in [
        (12, 1250, ["12", "1250"]),
        (99_999, 99_999, ["99999", "99999"]),
        (123_456, 123_456, ["123k", "123k"]),
    ] {
        let mut harness = counted(BrowserPosition::Bottom, selected, total, Locale::EnGb);
        harness.run();
        let column = bar(&harness);
        assert_eq!(column.width(), 36.0);
        let drawn = texts(&harness, column);
        assert_eq!(
            drawn.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(),
            shown,
        );
        for (text, rect) in &drawn {
            assert!(
                column.left() <= rect.left() && rect.right() <= column.right() - 2.0,
                "{text} {rect:?} in {column:?}"
            );
        }
    }
}

#[test]
fn the_column_s_count_is_photographed() {
    let mut harness = counted(BrowserPosition::Bottom, 3, 1250, Locale::EnGb);
    harness.set_size(egui::vec2(360.0, 200.0));
    harness.run();
    harness.snapshot("browser-count-column");
}

/// A count too wide for a narrow row, so not drawn, still has its words as a tooltip, so a
/// pointer can read it there.
#[test]
fn a_count_too_wide_to_draw_still_has_its_tooltip() {
    let mut harness = counted_in(120.0, BrowserPosition::Left, 3, 120, Locale::EnGb);
    harness.run();
    let bar = bar(&harness);
    let words = "3 of 120 photos selected";
    assert!(
        !texts(&harness, bar).iter().any(|(t, _)| t.contains('/')),
        "the count should not fit this row: {:?}",
        texts(&harness, bar)
    );
    let label = harness.get_by_role_and_label(Role::Label, words);
    harness.hover_at(label.rect().center());
    harness.run();
    harness.run();
    let tips = texts(&harness, Rect::EVERYTHING);
    assert!(tips.iter().any(|(t, _)| t == words), "{tips:?}");
}
