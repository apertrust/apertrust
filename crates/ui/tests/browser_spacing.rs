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

//! The space round the Browser's cells (`docs/gui.md`, Its layout): `TOP`, 8 points, on
//! every side of them inside the strip, the scroll bar in its own lane at the far edge, the
//! cells 12 points apart but for those one above another in a column, 16; measured through
//! AccessKit and the rendered pixels, and photographed in every theme.

use apertrust_ui::components::{
    Browser, BrowserPosition, Thumb, ThumbnailSize, browser_height, browser_width,
};
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::Role;
use egui::{Color32, Rect, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

/// The space round the cells, inside the strip.
const TOP: f32 = 8.0;

/// A window with the Browser at `position`, `lines` lines of `size` cells as the app lays
/// it out, the active photo the one the test sets.
struct Strip {
    position: BrowserPosition,
    size: ThumbnailSize,
    lines: usize,
    count: usize,
    active: usize,
}

fn strip(
    theme: Builtin,
    position: BrowserPosition,
    size: ThumbnailSize,
    lines: usize,
    count: usize,
) -> Harness<'static, Strip> {
    let window = if position.is_column() {
        vec2(700.0, 560.0)
    } else {
        vec2(900.0, 300.0)
    };
    Harness::builder()
        .with_size(window)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(theme)))
        .build_ui_state(
            |ui, state: &mut Strip| {
                let ctx = ui.ctx().clone();
                let names: Vec<String> = (0..state.count)
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let panel = match state.position {
                    BrowserPosition::Bottom => egui::Panel::bottom("browser")
                        .exact_size(browser_height(&ctx, state.size, true, state.lines)),
                    BrowserPosition::Left => egui::Panel::left("browser")
                        .exact_size(browser_width(&ctx, state.size, state.lines)),
                    BrowserPosition::Right => egui::Panel::right("browser")
                        .exact_size(browser_width(&ctx, state.size, state.lines)),
                };
                let active = state.active;
                panel
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        let thumb = |i: usize| Thumb {
                            name: &names[i],
                            edited: false,
                            status: None,
                            selected: i == active || i == active + 1,
                            picture: None,
                            no_preview: false,
                            rating: None,
                        };
                        Browser::new(state.count, Some(active), thumb)
                            .size(state.size)
                            .lines(state.lines)
                            .position(state.position)
                            .show(ui);
                    });
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(panel))
                    .show(ui, |_| {});
            },
            Strip {
                position,
                size,
                lines,
                count,
                active: 0,
            },
        )
}

fn cell(harness: &Harness<'_, Strip>, i: usize) -> Rect {
    harness.get_by_label(&format!("IMG_{i:04}.ARW")).rect()
}

fn list(harness: &Harness<'_, Strip>) -> Rect {
    harness.get_by_role_and_label(Role::List, "Browser").rect()
}

/// The pixels of `band`, in points, painted in `colour`, give or take 2 a channel.
fn painted(harness: &mut Harness<'_, Strip>, band: Rect, colour: Color32) -> Vec<(u32, u32)> {
    let ppp = harness.ctx.pixels_per_point();
    let image = harness.render().expect("rendered");
    let near = |a: u8, b: u8| a.abs_diff(b) <= 2;
    let (x0, y0) = ((band.left() * ppp) as u32, (band.top() * ppp) as u32);
    let (x1, y1) = (
        ((band.right() * ppp) as u32).min(image.width()),
        ((band.bottom() * ppp) as u32).min(image.height()),
    );
    let mut found = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            if near(r, colour.r()) && near(g, colour.g()) && near(b, colour.b()) {
                found.push((x, y));
            }
        }
    }
    found
}

/// The scroll bar's thumb inside `band`, in points: where the theme's `scroll_thumb` is
/// painted there.
fn thumb(harness: &mut Harness<'_, Strip>, band: Rect) -> Rect {
    let ppp = harness.ctx.pixels_per_point();
    let colour = Theme::default().colors.scroll_thumb;
    let found = painted(harness, band, colour);
    assert!(found.len() > 100, "no scroll bar in {band:?}");
    let (mut min, mut max) = (pos2(f32::MAX, f32::MAX), pos2(f32::MIN, f32::MIN));
    for (x, y) in found {
        let (x, y) = (x as f32 / ppp, y as f32 / ppp);
        min = min.min(pos2(x, y));
        max = max.max(pos2(x + 1.0 / ppp, y + 1.0 / ppp));
    }
    Rect::from_min_max(min, max)
}

/// Sets the active photo and lets the strip scroll to it.
fn activate(harness: &mut Harness<'_, Strip>, active: usize) {
    harness.state_mut().active = active;
    harness.run();
    harness.run();
}

#[test]
fn a_row_s_first_and_last_cells_keep_their_rings_inside_the_strip() {
    let mut harness = strip(
        Builtin::Dark,
        BrowserPosition::Bottom,
        ThumbnailSize::Medium,
        1,
        30,
    );
    harness.run();
    let ring = Theme::default().colors.frame_ring;
    for active in [0, 29] {
        activate(&mut harness, active);
        let rect = cell(&harness, active);
        // The active ring, 3 points wide, 2 out from the cell's square.
        let square = Rect::from_min_size(rect.min, vec2(rect.width(), rect.width()));
        let ys = square.top() + 8.0..=square.bottom() - 8.0;
        for (side, xs) in [
            ("left", square.left() - 5.0..=square.left() - 2.0),
            ("right", square.right() + 2.0..=square.right() + 5.0),
        ] {
            let band = Rect::from_x_y_ranges(xs, ys.clone());
            let whole = (band.width() * band.height()) as usize;
            let found = painted(&mut harness, band, ring).len();
            assert!(
                found * 10 >= whole * 9,
                "photo {active}'s ring on its {side}: {found} of {whole} pixels"
            );
        }
    }
}

#[test]
fn a_row_has_the_same_space_above_and_below_its_cells() {
    for size in [ThumbnailSize::Small, ThumbnailSize::Medium] {
        let mut harness = strip(Builtin::Dark, BrowserPosition::Bottom, size, 1, 30);
        harness.run();
        let list = list(&harness);
        let first = cell(&harness, 0);
        let separator = Theme::default().shape.separator_width;
        let above = first.top() - (list.top() + separator);
        let lane = Rect::from_min_max(pos2(first.left(), first.bottom()), list.max);
        let bar = thumb(&mut harness, lane);
        let below = bar.top() - first.bottom();
        assert!((above - TOP).abs() <= 1.0, "{size:?}: {above} above");
        assert!(
            (below - above).abs() <= 1.0,
            "{size:?}: {above} above, {below} below"
        );
        // The bar in its own lane, at the strip's bottom.
        assert!(
            (list.bottom() - bar.bottom()).abs() <= 1.0,
            "{size:?}: {bar:?}"
        );
    }
}

#[test]
fn a_column_has_the_same_space_either_side_of_its_cells() {
    let separator = Theme::default().shape.separator_width;
    for (position, columns) in [
        (BrowserPosition::Left, 1),
        (BrowserPosition::Left, 2),
        (BrowserPosition::Right, 1),
        (BrowserPosition::Right, 2),
    ] {
        let mut harness = strip(Builtin::Dark, position, ThumbnailSize::Small, columns, 40);
        harness.run();
        let list = list(&harness);
        let (first, last) = (cell(&harness, 0), cell(&harness, columns - 1));
        // The separator is on the side towards the Viewer: the left's right, the right's
        // left.
        let start = match position {
            BrowserPosition::Right => list.left() + separator,
            _ => list.left(),
        };
        let lane = Rect::from_min_max(pos2(last.right(), first.top()), list.max);
        let bar = thumb(&mut harness, lane);
        let (before, after) = (first.left() - start, bar.left() - last.right());
        let at = format!("{position:?} with {columns}");
        assert!((before - TOP).abs() <= 1.0, "{at}: {before} before");
        assert!(
            (after - before).abs() <= 1.0,
            "{at}: {before} before, {after} after"
        );
        let end = match position {
            BrowserPosition::Left => list.right() - separator,
            _ => list.right(),
        };
        assert!(
            (end - bar.right()).abs() <= 1.0,
            "{at}: {bar:?} in {list:?}"
        );
    }
}

#[test]
fn cells_in_a_column_are_16_points_apart() {
    let mut harness = strip(
        Builtin::Dark,
        BrowserPosition::Left,
        ThumbnailSize::Small,
        2,
        40,
    );
    harness.run();
    // Rows of two: 0 and 1, then 2 and 3.
    assert_eq!(cell(&harness, 2).top() - cell(&harness, 0).bottom(), 16.0);
    assert_eq!(cell(&harness, 1).left() - cell(&harness, 0).right(), 12.0);
    // A row's cells are 12 apart both ways.
    let mut harness = strip(
        Builtin::Dark,
        BrowserPosition::Bottom,
        ThumbnailSize::Small,
        2,
        40,
    );
    harness.run();
    // Columns of two: 0 over 1, then 2 over 3.
    assert_eq!(cell(&harness, 1).top() - cell(&harness, 0).bottom(), 12.0);
    assert_eq!(cell(&harness, 2).left() - cell(&harness, 0).right(), 12.0);
}

/// The Browser at the bottom and at the left, scrolled to its start, its first photo active
/// and the next selected, the scroll bar showing, in each theme.
#[test]
fn the_space_round_the_cells_is_photographed_in_every_theme() {
    let mut results = egui_kittest::SnapshotResults::new();
    for which in Builtin::ALL {
        for (position, size, lines, name) in [
            (BrowserPosition::Bottom, ThumbnailSize::Medium, 1, "bottom"),
            (BrowserPosition::Left, ThumbnailSize::Small, 2, "left"),
        ] {
            let mut harness = strip(which, position, size, lines, 30);
            harness.run();
            harness.run();
            harness.snapshot(format!("browser-spacing-{name}-{}", which.id()));
            results.extend(harness.take_snapshot_results());
        }
    }
}
