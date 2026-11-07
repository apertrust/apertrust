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

//! The grid panel's grid bar, driven through AccessKit and photographed (`docs/gui.md`,
//! The grid bar).

use apertrust_ui::Theme;
use apertrust_ui::components::{
    GridBarAction, GridEdge, GridEdgeAction, GridPanel, GridPanelAction, GridPosition, Sort,
    SortKey, Thumb, ThumbnailSize, grid_panel_height, grid_panel_width,
};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn install(ctx: &egui::Context) {
    apertrust_ui::install(ctx, Theme::default());
}

/// What the panel is told, and what its grid bar reported.
struct Grid {
    count: usize,
    sort: Sort,
    size: ThumbnailSize,
    labels: bool,
    chosen: Vec<GridBarAction>,
}

/// The panel with `count` photos, as the app keeps it: each choice taken in at once.
fn grid(count: usize) -> Harness<'static, Grid> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 360.0))
        .with_context_setup(install)
        .build_ui_state(
            |ui, grid: &mut Grid| {
                let names: Vec<String> =
                    (0..grid.count).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: false,
                    picture: None,
                    rating: None,
                };
                let output = GridPanel::new(grid.count, Some(0), thumb)
                    .sort(grid.sort)
                    .size(grid.size)
                    .labels(grid.labels)
                    .show(ui);
                if let Some(chosen) = output.bar {
                    match chosen {
                        GridBarAction::Sort(key) => grid.sort.key = key,
                        GridBarAction::Descending(descending) => grid.sort.descending = descending,
                        GridBarAction::Size(size) => grid.size = size,
                        GridBarAction::Position(_) => {}
                        GridBarAction::Labels => grid.labels = !grid.labels,
                    }
                    grid.chosen.push(chosen);
                }
            },
            Grid {
                count,
                sort: Sort {
                    key: SortKey::CaptureTime,
                    descending: false,
                },
                size: ThumbnailSize::Medium,
                labels: true,
                chosen: Vec::new(),
            },
        )
}

/// A menu line's whether it can be chosen and whether it is checked.
fn line(harness: &Harness<'_, Grid>, name: &str) -> (bool, Option<Toggled>) {
    let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, name);
    let node = node.accesskit_node();
    (!node.is_disabled(), node.toggled())
}

#[test]
fn the_grid_bar_is_a_toolbar_of_sort_and_zoom_named_for_screen_readers() {
    let mut harness = grid(12);
    harness.run();
    let bar = harness.get_by_role_and_label(Role::Toolbar, "Grid bar");
    let buttons: Vec<String> = bar
        .query_all_by_role(Role::Button)
        .map(|b| b.accesskit_node().label().unwrap_or_default())
        .collect();
    assert_eq!(buttons, ["Sort", "Labels", "Thumbnail size"]);
    let bar = bar.rect();
    let sort = harness.get_by_role_and_label(Role::Button, "Sort");
    let data = sort.accesskit_node();
    assert_eq!(data.value().as_deref(), Some("Capture time, ascending"));
    assert_eq!(data.data().has_popup(), Some(HasPopup::Menu));
    let zoom = harness.get_by_role_and_label(Role::Button, "Thumbnail size");
    assert_eq!(zoom.accesskit_node().value().as_deref(), Some("Medium"));
    // Their tooltips name what they hold.
    sort.hover();
    harness.run();
    assert!(
        harness
            .query_by_label("Sort: capture time, ascending")
            .is_some()
    );
    // At the bottom the bar is a column at the panel's left end, as tall as the panel, the
    // cells after it and its buttons stacked.
    let cell = harness.get_by_label("IMG_0000.ARW").rect();
    let list = harness
        .get_by_role_and_label(Role::List, "Grid panel")
        .rect();
    assert!(bar.right() <= cell.left());
    assert_eq!(bar.left(), list.left());
    assert_eq!(bar.bottom(), list.bottom());
    let sort = harness.get_by_role_and_label(Role::Button, "Sort").rect();
    let zoom = harness
        .get_by_role_and_label(Role::Button, "Thumbnail size")
        .rect();
    assert_eq!(sort.center().x, zoom.center().x);
    assert!(sort.bottom() < zoom.top());
    // Labels right under Sort, Zoom alone at the column's bottom.
    let labels = harness.get_by_role_and_label(Role::Button, "Labels").rect();
    assert_eq!(labels.center().x, sort.center().x);
    assert!(sort.bottom() <= labels.top() && labels.top() - sort.bottom() <= 4.0);
    assert!(zoom.top() - labels.bottom() > 40.0);
    assert_eq!(zoom.bottom(), bar.bottom() - 4.0);
}

#[test]
fn labels_shows_the_captions_and_counts_them_in_the_cells_and_the_panel() {
    let mut harness = grid(12);
    harness.run();
    let labels = harness.get_by_role_and_label(Role::Button, "Labels");
    assert_eq!(labels.accesskit_node().toggled(), Some(Toggled::True));
    // A Medium cell is its square and a 40-point caption, the panel as tall as its row.
    let cell = harness.get_by_label("IMG_0000.ARW").rect();
    assert_eq!(cell.size(), egui::vec2(120.0, 160.0));
    let list = |harness: &Harness<'_, Grid>| {
        harness
            .get_by_role_and_label(Role::List, "Grid panel")
            .rect()
    };
    let ctx = harness.ctx.clone();
    assert_eq!(
        list(&harness).height(),
        grid_panel_height(&ctx, ThumbnailSize::Medium, true, 1)
    );
    assert_eq!(
        grid_panel_height(&ctx, ThumbnailSize::Medium, true, 2)
            - grid_panel_height(&ctx, ThumbnailSize::Medium, true, 1),
        120.0 + 40.0 + 8.0
    );
    // Pressed, it reports itself; the caller turns the captions off, and the cells and the
    // panel lose them.
    harness
        .get_by_role_and_label(Role::Button, "Labels")
        .click();
    harness.run();
    assert_eq!(harness.state().chosen, [GridBarAction::Labels]);
    assert!(!harness.state().labels);
    let labels = harness.get_by_role_and_label(Role::Button, "Labels");
    assert_eq!(labels.accesskit_node().toggled(), Some(Toggled::False));
    assert_eq!(
        harness.get_by_label("IMG_0000.ARW").rect().size(),
        egui::vec2(120.0, 120.0)
    );
    assert_eq!(
        list(&harness).height(),
        grid_panel_height(&ctx, ThumbnailSize::Medium, false, 1)
    );
    assert_eq!(
        grid_panel_height(&ctx, ThumbnailSize::Medium, true, 1)
            - grid_panel_height(&ctx, ThumbnailSize::Medium, false, 1),
        40.0
    );
    // A Small cell's caption is the name alone, 20 points.
    assert_eq!(ThumbnailSize::Small.cell(true), egui::vec2(84.0, 104.0));
}

#[test]
fn with_no_photos_the_grid_bar_s_buttons_are_disabled() {
    let mut harness = grid(0);
    harness.run();
    for name in ["Sort", "Labels", "Thumbnail size"] {
        let button = harness.get_by_role_and_label(Role::Button, name);
        assert!(button.accesskit_node().is_disabled(), "{name}");
    }
}

#[test]
fn sort_s_menu_checks_the_key_and_the_direction_and_reports_a_choice() {
    let mut harness = grid(12);
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Sort").click();
    harness.run();
    assert_eq!(line(&harness, "Capture time"), (true, Some(Toggled::True)));
    assert_eq!(line(&harness, "File name"), (true, Some(Toggled::False)));
    assert_eq!(line(&harness, "Edited"), (true, Some(Toggled::False)));
    // Rating waits for the ratings.
    assert_eq!(line(&harness, "Rating"), (false, Some(Toggled::False)));
    assert_eq!(line(&harness, "Ascending"), (true, Some(Toggled::True)));
    assert_eq!(line(&harness, "Descending"), (true, Some(Toggled::False)));
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "File name")
        .click();
    harness.run();
    assert_eq!(
        harness.state().chosen,
        [GridBarAction::Sort(SortKey::FileName)]
    );
    // The direction, chosen the same way, turns the icon and the value.
    harness.get_by_role_and_label(Role::Button, "Sort").click();
    harness.run();
    assert_eq!(line(&harness, "File name"), (true, Some(Toggled::True)));
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Descending")
        .click();
    harness.run();
    assert_eq!(
        harness.state().chosen.last(),
        Some(&GridBarAction::Descending(true))
    );
    let sort = harness.get_by_role_and_label(Role::Button, "Sort");
    assert_eq!(
        sort.accesskit_node().value().as_deref(),
        Some("File name, descending")
    );
}

#[test]
fn zoom_s_menu_checks_the_size_and_a_choice_sizes_the_cells_and_the_panel() {
    let mut harness = grid(12);
    harness.run();
    assert_eq!(harness.get_by_label("IMG_0000.ARW").rect().width(), 120.0);
    harness
        .get_by_role_and_label(Role::Button, "Thumbnail size")
        .click();
    harness.run();
    for (name, on) in [
        ("Small", false),
        ("Medium", true),
        ("Large", false),
        ("Extra large", false),
    ] {
        let toggled = if on { Toggled::True } else { Toggled::False };
        assert_eq!(line(&harness, name), (true, Some(toggled)), "{name}");
    }
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Large")
        .click();
    harness.run();
    assert_eq!(
        harness.state().chosen,
        [GridBarAction::Size(ThumbnailSize::Large)]
    );
    assert_eq!(harness.get_by_label("IMG_0000.ARW").rect().width(), 176.0);
    let list = harness
        .get_by_role_and_label(Role::List, "Grid panel")
        .rect();
    let height = grid_panel_height(&harness.ctx, ThumbnailSize::Large, true, 1);
    assert_eq!(list.height(), height);
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Thumbnail size")
            .accesskit_node()
            .value()
            .as_deref(),
        Some("Large")
    );
}

#[test]
fn the_grid_bar_is_photographed() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            640.0,
            grid_panel_height(&egui::Context::default(), ThumbnailSize::Small, true, 1) + 40.0,
        ))
        .with_context_setup(install)
        .build_ui(|ui| {
            let names: Vec<String> = (0..9).map(|i| format!("IMG_{i:04}.ARW")).collect();
            let thumb = |i: usize| Thumb {
                name: &names[i],
                edited: i == 2,
                status: None,
                selected: false,
                picture: None,
                rating: None,
            };
            GridPanel::new(names.len(), Some(1), thumb)
                .sort(Sort {
                    key: SortKey::FileName,
                    descending: true,
                })
                .size(ThumbnailSize::Small)
                .show(ui);
        });
    harness.run();
    harness.snapshot("grid-bar");
}

/// A window with the grid panel along its bottom as the app lays it out: its rows the
/// edge's, the edge shown last, and what the edge reported.
struct Bottom {
    rows: usize,
    most: usize,
    count: usize,
    actions: Vec<GridEdgeAction>,
    opened: Vec<usize>,
}

fn bottom(rows: usize, most: usize, count: usize) -> Harness<'static, Bottom> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        // Frames a sixtieth of a second apart, so two clicks make a double-click (0.3 s).
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, state: &mut Bottom| {
                let ctx = ui.ctx().clone();
                let edge = GridEdge::new(state.rows, state.most, ThumbnailSize::Small);
                let lines = edge.lines(&ctx);
                let names: Vec<String> = (0..state.count)
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let panel = egui::Panel::bottom("grid panel")
                    .exact_size(grid_panel_height(&ctx, ThumbnailSize::Small, true, lines))
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        let thumb = |i: usize| Thumb {
                            name: &names[i],
                            edited: false,
                            status: None,
                            selected: false,
                            picture: None,
                            rating: None,
                        };
                        GridPanel::new(state.count, None, thumb)
                            .size(ThumbnailSize::Small)
                            .lines(lines)
                            .show(ui)
                    });
                if let Some(GridPanelAction::Open(i)) = panel.inner.action {
                    state.opened.push(i);
                }
                egui::CentralPanel::default().show(ui, |_| {});
                if let Some(action) = edge.show(ui, panel.response.rect) {
                    state.rows = match action {
                        GridEdgeAction::Resize(rows) => rows,
                        GridEdgeAction::Reset => 1,
                    };
                    state.actions.push(action);
                }
            },
            Bottom {
                rows,
                most,
                count,
                actions: Vec::new(),
                opened: Vec::new(),
            },
        )
}

fn splitter<'h>(harness: &'h Harness<'_, Bottom>) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::Splitter, "Grid panel height")
}

/// Drags the edge `by` points down the window, in frames of their own.
fn drag_edge(harness: &mut Harness<'_, Bottom>, by: f32) {
    let from = splitter(harness).rect().center();
    let to = from + egui::vec2(0.0, by);
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.drop_at(to);
    harness.run();
}

/// From one column's start to the next's: a Small cell and the gap.
const PITCH: f32 = 84.0 + 8.0;
/// From one row's start to the next's: a Small cell, its caption of the name alone and the
/// gap.
const ROW: f32 = 84.0 + 20.0 + 8.0;

#[test]
fn dragging_the_grid_panel_s_edge_adds_and_takes_whole_rows() {
    let mut harness = bottom(1, 4, 30);
    harness.run();
    let top = splitter(&harness).rect().center().y;
    // Short of the next row's middle, nothing; past it, a row, and the edge moves a row.
    drag_edge(&mut harness, -0.4 * ROW);
    assert_eq!(harness.state().rows, 1);
    assert!(harness.state().actions.is_empty());
    drag_edge(&mut harness, -0.6 * ROW);
    assert_eq!(harness.state().rows, 2);
    assert_eq!(splitter(&harness).rect().center().y, top - ROW);
    // Never more than the most, nor fewer than one.
    drag_edge(&mut harness, -10.0 * ROW);
    assert_eq!(harness.state().rows, 4);
    drag_edge(&mut harness, 10.0 * ROW);
    assert_eq!(harness.state().rows, 1);
    // The panel follows the pointer in the frame it moves.
    let from = splitter(&harness).rect().center();
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from - egui::vec2(0.0, 2.0 * ROW));
    harness.step();
    assert_eq!(splitter(&harness).rect().center().y, top - 2.0 * ROW);
    let node = splitter(&harness).accesskit_node();
    assert_eq!(node.numeric_value(), Some(3.0));
    assert_eq!(node.value().as_deref(), Some("3 rows"));
    harness.drop_at(from - egui::vec2(0.0, 2.0 * ROW));
    harness.run();
    assert_eq!(harness.state().rows, 3);
    // A drag leaves the focus where it was.
    assert!(!splitter(&harness).is_focused());
}

#[test]
fn the_focused_edge_steps_by_rows_and_takes_the_ends_and_reset_keys() {
    let mut harness = bottom(2, 4, 30);
    harness.run();
    splitter(&harness).focus();
    harness.run();
    for (key, rows) in [
        (egui::Key::ArrowUp, 3),
        (egui::Key::ArrowUp, 4),
        (egui::Key::ArrowUp, 4),
        (egui::Key::ArrowDown, 3),
        (egui::Key::Home, 1),
        (egui::Key::End, 4),
        (egui::Key::Delete, 1),
    ] {
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().rows, rows, "{key:?}");
    }
    assert!(splitter(&harness).is_focused());
    assert_eq!(
        splitter(&harness).accesskit_node().value().as_deref(),
        Some("1 row")
    );
}

#[test]
fn a_double_click_on_the_edge_goes_back_to_one_row_and_screen_readers_step_it() {
    let mut harness = bottom(3, 4, 30);
    harness.run();
    harness.hover_at(splitter(&harness).rect().center());
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        egui::CursorIcon::ResizeRow
    );
    for _ in 0..2 {
        splitter(&harness).click();
        harness.step();
    }
    harness.run();
    assert_eq!(harness.state().actions, [GridEdgeAction::Reset]);
    assert_eq!(harness.state().rows, 1);
    let node = splitter(&harness);
    assert_eq!(node.accesskit_node().max_numeric_value(), Some(4.0));
    node.increment_accesskit();
    harness.run();
    assert_eq!(harness.state().rows, 2);
    // A window that fits fewer shows fewer and keeps the rows, until the edge moves.
    harness.state_mut().rows = 6;
    harness.run();
    assert_eq!(
        splitter(&harness).accesskit_node().numeric_value(),
        Some(4.0)
    );
    assert_eq!(harness.state().rows, 6);
}

#[test]
fn with_several_rows_the_arrows_move_as_the_cells_lie() {
    // Columns of two: 0 and 1, 2 and 3, 4.
    let mut harness = bottom(2, 4, 5);
    harness.run();
    let column = |harness: &Harness<'_, Bottom>, name: &str| {
        let rect = harness.get_by_label(name).rect();
        (rect.left(), rect.top())
    };
    let (left, top) = column(&harness, "IMG_0000.ARW");
    assert_eq!(column(&harness, "IMG_0001.ARW"), (left, top + ROW));
    assert_eq!(column(&harness, "IMG_0002.ARW"), (left + PITCH, top));
    harness.get_by_label("IMG_0000.ARW").focus();
    harness.run();
    let enter = |harness: &mut Harness<'_, Bottom>| {
        harness.key_press(egui::Key::Enter);
        harness.run();
        *harness.state().opened.last().unwrap()
    };
    for (key, at) in [
        // Down its column, and nothing past its end.
        (egui::Key::ArrowDown, 1),
        (egui::Key::ArrowDown, 1),
        // Along the strip, the same row of the next column, the last photo where the last
        // column is short.
        (egui::Key::ArrowRight, 3),
        (egui::Key::ArrowRight, 4),
        (egui::Key::ArrowRight, 4),
        (egui::Key::ArrowLeft, 2),
        (egui::Key::ArrowUp, 2),
        (egui::Key::ArrowLeft, 0),
        (egui::Key::End, 4),
        (egui::Key::Home, 0),
    ] {
        harness.key_press(key);
        harness.run();
        harness.run();
        assert_eq!(enter(&mut harness), at, "{key:?}");
    }
}

/// Right-clicks at `pos`, in frames of their own.
fn right_click<T>(harness: &mut Harness<'_, T>, pos: egui::Pos2) {
    harness.hover_at(pos);
    harness.run();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
}

#[test]
fn a_right_click_on_the_grid_bar_opens_its_position_menu() {
    let mut harness = grid(12);
    harness.run();
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Grid bar")
        .rect();
    // The bar's empty middle, between Sort at its top and Zoom at its bottom.
    right_click(&mut harness, bar.center());
    let menu = harness.get_by_role_and_label(Role::Menu, "Grid panel options");
    assert!(menu.query_by_label("Position").is_some());
    harness.get_by_label("Position").click();
    harness.run();
    assert_eq!(line(&harness, "Bottom"), (true, Some(Toggled::True)));
    assert_eq!(line(&harness, "Left"), (true, Some(Toggled::False)));
    assert_eq!(line(&harness, "Right"), (true, Some(Toggled::False)));
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Left")
        .click();
    harness.run();
    assert_eq!(
        harness.state().chosen,
        [GridBarAction::Position(GridPosition::Left)]
    );
    // It opens with no photos too.
    let mut harness = grid(0);
    harness.run();
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Grid bar")
        .rect();
    // The bar's empty middle, between Sort at its top and Zoom at its bottom.
    right_click(&mut harness, bar.center());
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, "Grid panel options")
            .is_some()
    );
}

/// A window with the grid panel as a column at `position`, `columns` wide as its edge
/// leaves it, beside the photo, and what the edge reported and the panel opened.
struct Column {
    position: GridPosition,
    columns: usize,
    count: usize,
    actions: Vec<GridEdgeAction>,
    opened: Vec<usize>,
}

fn column(position: GridPosition, columns: usize, count: usize) -> Harness<'static, Column> {
    Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, state: &mut Column| {
                let ctx = ui.ctx().clone();
                let size = ThumbnailSize::Small;
                let edge = GridEdge::new(state.columns, 4, size).position(state.position);
                let lines = edge.lines(&ctx);
                let names: Vec<String> = (0..state.count)
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let side = match state.position {
                    GridPosition::Left => egui::Panel::left("grid panel"),
                    _ => egui::Panel::right("grid panel"),
                };
                let panel = side
                    .exact_size(grid_panel_width(&ctx, size, lines))
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        let thumb = |i: usize| Thumb {
                            name: &names[i],
                            edited: i == 2,
                            status: None,
                            selected: false,
                            picture: None,
                            rating: None,
                        };
                        GridPanel::new(state.count, Some(1), thumb)
                            .size(size)
                            .lines(lines)
                            .position(state.position)
                            .show(ui)
                    });
                if let Some(GridPanelAction::Open(i)) = panel.inner.action {
                    state.opened.push(i);
                }
                egui::CentralPanel::default().show(ui, |_| {});
                if let Some(action) = edge.show(ui, panel.response.rect) {
                    state.columns = match action {
                        GridEdgeAction::Resize(columns) => columns,
                        GridEdgeAction::Reset => 1,
                    };
                    state.actions.push(action);
                }
            },
            Column {
                position,
                columns,
                count,
                actions: Vec::new(),
                opened: Vec::new(),
            },
        )
}

fn width_splitter<'h>(harness: &'h Harness<'_, Column>) -> egui_kittest::Node<'h> {
    harness.get_by_role_and_label(Role::Splitter, "Grid panel width")
}

#[test]
fn at_a_side_the_cells_fill_rows_of_the_columns_and_the_bar_is_on_top() {
    let mut harness = column(GridPosition::Left, 2, 9);
    harness.run();
    let at = |harness: &Harness<'_, Column>, name: &str| harness.get_by_label(name).rect().min;
    let first = at(&harness, "IMG_0000.ARW");
    assert_eq!(at(&harness, "IMG_0001.ARW"), first + egui::vec2(PITCH, 0.0));
    assert_eq!(at(&harness, "IMG_0002.ARW"), first + egui::vec2(0.0, ROW));
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Grid bar")
        .rect();
    assert!(bar.bottom() <= first.y);
    // At a side the bar is a row across the panel's top, its buttons side by side.
    let sort = harness.get_by_role_and_label(Role::Button, "Sort").rect();
    let zoom = harness
        .get_by_role_and_label(Role::Button, "Thumbnail size")
        .rect();
    assert_eq!(sort.center().y, zoom.center().y);
    assert!(sort.right() < zoom.left());
    // Labels right after Sort, Zoom alone at the row's end.
    let labels = harness.get_by_role_and_label(Role::Button, "Labels").rect();
    assert_eq!(labels.center().y, sort.center().y);
    assert!(sort.right() <= labels.left() && labels.left() - sort.right() <= 4.0);
    assert!(zoom.left() - labels.right() > 40.0);
    assert_eq!(zoom.right(), bar.right() - 8.0);
    // At the window's left, its edge on its right.
    let list = harness
        .get_by_role_and_label(Role::List, "Grid panel")
        .rect();
    assert!(list.left() <= 8.0);
    assert_eq!(width_splitter(&harness).rect().center().x, list.right());
    // At the right, its edge on its left, next to the window's end.
    let mut harness = column(GridPosition::Right, 1, 9);
    harness.run();
    let list = harness
        .get_by_role_and_label(Role::List, "Grid panel")
        .rect();
    assert!(list.right() >= 1000.0 - 8.0);
    assert_eq!(width_splitter(&harness).rect().center().x, list.left());
}

#[test]
fn at_a_side_the_arrows_move_as_the_cells_lie() {
    // Rows of two: 0 and 1, 2 and 3, 4.
    let mut harness = column(GridPosition::Right, 2, 5);
    harness.run();
    harness.get_by_label("IMG_0000.ARW").focus();
    harness.run();
    for (key, at) in [
        // Along its row, and nothing past its end.
        (egui::Key::ArrowRight, 1),
        (egui::Key::ArrowRight, 1),
        // Down the strip, the same place in the next row, the last photo where the last
        // row is short.
        (egui::Key::ArrowDown, 3),
        (egui::Key::ArrowDown, 4),
        (egui::Key::ArrowUp, 2),
        (egui::Key::ArrowLeft, 2),
        (egui::Key::ArrowUp, 0),
        (egui::Key::End, 4),
    ] {
        harness.key_press(key);
        harness.run();
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().opened.last(), Some(&at), "{key:?}");
    }
}

#[test]
fn at_a_side_the_edge_resizes_in_columns_towards_the_photo() {
    for (position, towards) in [(GridPosition::Left, 1.0), (GridPosition::Right, -1.0)] {
        let mut harness = column(position, 1, 30);
        harness.run();
        let from = width_splitter(&harness).rect().center();
        harness.hover_at(from);
        harness.run();
        assert_eq!(
            harness.output().platform_output.cursor_icon,
            egui::CursorIcon::ResizeColumn
        );
        let to = from + egui::vec2(towards * 0.6 * PITCH, 0.0);
        harness.drag_at(from);
        harness.run();
        harness.hover_at(to);
        harness.run();
        harness.drop_at(to);
        harness.run();
        assert_eq!(harness.state().columns, 2, "{position:?}");
        let node = width_splitter(&harness).accesskit_node();
        assert_eq!(node.value().as_deref(), Some("2 columns"));
        // The arrow towards the photo adds a column, the other takes one away.
        width_splitter(&harness).focus();
        harness.run();
        let (more, fewer) = if towards > 0.0 {
            (egui::Key::ArrowRight, egui::Key::ArrowLeft)
        } else {
            (egui::Key::ArrowLeft, egui::Key::ArrowRight)
        };
        harness.key_press(more);
        harness.run();
        assert_eq!(harness.state().columns, 3, "{position:?}");
        harness.key_press(fewer);
        harness.key_press(fewer);
        harness.run();
        assert_eq!(harness.state().columns, 1, "{position:?}");
    }
}

#[test]
fn the_grid_panel_at_the_left_and_the_right_is_photographed() {
    let mut results = egui_kittest::SnapshotResults::new();
    for (position, name) in [
        (GridPosition::Left, "grid-panel-left"),
        (GridPosition::Right, "grid-panel-right"),
    ] {
        let mut harness = column(position, 2, 40);
        harness.run();
        harness.run();
        results.add(harness.try_snapshot(name));
    }
}

/// How many pixels of `band`, in points, the panel's render paints in the scroll bar's
/// thumb at rest, the theme's `scroll_thumb`.
fn thumb_pixels<T>(harness: &mut Harness<'_, T>, band: egui::Rect) -> usize {
    let thumb = Theme::default().colors.scroll_thumb;
    let ppp = harness.ctx.pixels_per_point();
    let image = harness.render().expect("rendered");
    let near = |a: u8, b: u8| a.abs_diff(b) <= 2;
    let (x0, y0) = ((band.left() * ppp) as u32, (band.top() * ppp) as u32);
    let (x1, y1) = (
        ((band.right() * ppp) as u32).min(image.width()),
        ((band.bottom() * ppp) as u32).min(image.height()),
    );
    let mut found = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            if near(r, thumb.r()) && near(g, thumb.g()) && near(b, thumb.b()) {
                found += 1;
            }
        }
    }
    found
}

/// The band along the strip's far edge where the scroll bar goes: under a row's cells,
/// at a column's end.
fn bar_band(list: egui::Rect, column: bool) -> egui::Rect {
    if column {
        egui::Rect::from_min_max(egui::pos2(list.right() - 14.0, list.top()), list.max)
    } else {
        egui::Rect::from_min_max(egui::pos2(list.left(), list.bottom() - 14.0), list.max)
    }
}

#[test]
fn the_scroll_bar_shows_while_the_cells_overflow_and_not_when_they_fit() {
    // A row: 30 Medium cells overflow 900 points, 3 fit.
    for (count, shows) in [(30, true), (3, false)] {
        let mut harness = grid(count);
        harness.run();
        let list = harness
            .get_by_role_and_label(Role::List, "Grid panel")
            .rect();
        let found = thumb_pixels(&mut harness, bar_band(list, false));
        assert_eq!(found > 100, shows, "{count} photos: {found} pixels");
    }
    // A column of two: 40 Small cells overflow 600 points, 4 fit.
    for (count, shows) in [(40, true), (4, false)] {
        let mut harness = column(GridPosition::Left, 2, count);
        harness.run();
        let list = harness
            .get_by_role_and_label(Role::List, "Grid panel")
            .rect();
        let found = thumb_pixels(&mut harness, bar_band(list, true));
        assert_eq!(found > 100, shows, "{count} photos: {found} pixels");
    }
}

/// Turns the wheel over `pos`, `delta` points as egui counts them.
fn wheel<T>(harness: &mut Harness<'_, T>, pos: egui::Pos2, delta: egui::Vec2) {
    harness.hover_at(pos);
    harness.run();
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(30);
}

#[test]
fn the_wheel_scrolls_the_strip_a_vertical_one_a_row_of_cells_too() {
    let mut harness = grid(60);
    harness.run();
    let at = harness.get_by_label("IMG_0002.ARW").rect().center();
    assert!(harness.query_by_label("IMG_0020.ARW").is_none());
    // A plain wheel turns up and down; a row scrolls sideways with it.
    wheel(&mut harness, at, egui::vec2(0.0, -2400.0));
    assert!(harness.query_by_label("IMG_0000.ARW").is_none());
    assert!(harness.query_by_label("IMG_0020.ARW").is_some());
    // And a trackpad's sideways swipe as well, back towards the start.
    wheel(&mut harness, at, egui::vec2(2400.0, 0.0));
    assert!(harness.query_by_label("IMG_0000.ARW").is_some());

    // A column scrolls up and down.
    let mut harness = column(GridPosition::Right, 2, 60);
    harness.run();
    let at = harness.get_by_label("IMG_0002.ARW").rect().center();
    wheel(&mut harness, at, egui::vec2(0.0, -1300.0));
    assert!(harness.query_by_label("IMG_0000.ARW").is_none());
    assert!(harness.query_by_label("IMG_0030.ARW").is_some());
}

#[test]
fn the_scroll_bar_is_photographed() {
    let mut harness = grid(30);
    harness.run();
    let list = harness
        .get_by_role_and_label(Role::List, "Grid panel")
        .rect();
    // Under the pointer, the thumb a step brighter.
    harness.hover_at(egui::pos2(list.left() + 60.0, list.bottom() - 8.0));
    harness.run();
    harness.snapshot("grid-panel-scroll-bar");
}
