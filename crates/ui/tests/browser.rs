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

//! The Browser's browser bar, driven through AccessKit and photographed (`docs/gui.md`,
//! The browser bar), and what its clicks report (The pointer).

use apertrust_ui::components::{
    Browser, BrowserAction, BrowserBarAction, BrowserEdge, BrowserEdgeAction, BrowserPosition,
    CellAction, CellMenu, CellPicture, CellRating, Sort, SortKey, Thumb, ThumbnailSize,
    browser_height, browser_width,
};
use apertrust_ui::{Icon, Theme};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn install(ctx: &egui::Context) {
    apertrust_ui::install(ctx, Theme::default());
}

/// What the panel is told, and what its browser bar reported.
struct BrowserState {
    count: usize,
    sort: Sort,
    size: ThumbnailSize,
    labels: bool,
    chosen: Vec<BrowserBarAction>,
    /// The cells' menus asked for.
    menus: Vec<CellMenu>,
    /// What was done to the photos.
    actions: Vec<BrowserAction>,
}

/// The panel with `count` photos, as the app keeps it: each choice taken in at once.
fn browser(count: usize) -> Harness<'static, BrowserState> {
    browser_on(count, egui::os::OperatingSystem::Nix)
}

/// [`browser`] on `os`.
fn browser_on(count: usize, os: egui::os::OperatingSystem) -> Harness<'static, BrowserState> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 360.0))
        .with_os(os)
        .with_context_setup(install)
        .build_ui_state(
            |ui, browser: &mut BrowserState| {
                let names: Vec<String> = (0..browser.count)
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: false,
                    picture: None,
                    rating: None,
                };
                let output = Browser::new(browser.count, Some(0), thumb)
                    .sort(browser.sort)
                    .size(browser.size)
                    .labels(browser.labels)
                    .show(ui);
                if let Some(chosen) = output.bar {
                    match chosen {
                        BrowserBarAction::Sort(key) => browser.sort.key = key,
                        BrowserBarAction::Descending(descending) => {
                            browser.sort.descending = descending
                        }
                        BrowserBarAction::Size(size) => browser.size = size,
                        BrowserBarAction::Position(_) => {}
                        BrowserBarAction::Labels => browser.labels = !browser.labels,
                    }
                    browser.chosen.push(chosen);
                }
                browser.menus.extend(output.menu);
                browser.actions.extend(output.action);
            },
            BrowserState {
                count,
                sort: Sort {
                    key: SortKey::CaptureTime,
                    descending: false,
                },
                size: ThumbnailSize::Medium,
                labels: true,
                chosen: Vec::new(),
                menus: Vec::new(),
                actions: Vec::new(),
            },
        )
}

/// A menu line's whether it can be chosen and whether it is checked.
fn line(harness: &Harness<'_, BrowserState>, name: &str) -> (bool, Option<Toggled>) {
    let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, name);
    let node = node.accesskit_node();
    (!node.is_disabled(), node.toggled())
}

#[test]
fn the_browser_bar_is_a_toolbar_of_sort_and_zoom_named_for_screen_readers() {
    let mut harness = browser(12);
    harness.run();
    let bar = harness.get_by_role_and_label(Role::Toolbar, "Browser bar");
    let buttons: Vec<String> = bar
        .query_all_by_role(Role::Button)
        .map(|b| b.accesskit_node().label().unwrap_or_default())
        .collect();
    assert_eq!(buttons, ["Sort", "Thumbnail size"]);
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
    let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
    assert!(bar.right() <= cell.left());
    assert_eq!(bar.left(), list.left());
    assert_eq!(bar.bottom(), list.bottom());
    let sort = harness.get_by_role_and_label(Role::Button, "Sort").rect();
    let zoom = harness
        .get_by_role_and_label(Role::Button, "Thumbnail size")
        .rect();
    assert_eq!(sort.center().x, zoom.center().x);
    // Sort at the column's top, Zoom alone at its bottom.
    assert_eq!(sort.top(), bar.top() + 4.0);
    assert!(zoom.top() - sort.bottom() > 40.0);
    assert_eq!(zoom.bottom(), bar.bottom() - 4.0);
}

/// The browser bar's options menu, opened by a right-click on its empty middle.
fn options(harness: &mut Harness<'_, BrowserState>) {
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Browser bar")
        .rect();
    right_click(harness, bar.center());
}

#[test]
fn show_labels_shows_the_captions_and_counts_them_in_the_cells_and_the_panel() {
    let mut harness = browser(12);
    harness.run();
    options(&mut harness);
    assert_eq!(line(&harness, "Show labels"), (true, Some(Toggled::True)));
    // A Medium cell is its square and a 40-point caption, the panel as tall as its row.
    let cell = harness.get_by_label("IMG_0000.ARW").rect();
    assert_eq!(cell.size(), egui::vec2(120.0, 160.0));
    let list = |harness: &Harness<'_, BrowserState>| {
        harness.get_by_role_and_label(Role::List, "Browser").rect()
    };
    let ctx = harness.ctx.clone();
    assert_eq!(
        list(&harness).height(),
        browser_height(&ctx, ThumbnailSize::Medium, true, 1)
    );
    assert_eq!(
        browser_height(&ctx, ThumbnailSize::Medium, true, 2)
            - browser_height(&ctx, ThumbnailSize::Medium, true, 1),
        120.0 + 40.0 + 8.0
    );
    // Chosen, it reports itself; the caller turns the captions off, and the cells and the
    // panel lose them.
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Show labels")
        .click();
    harness.run();
    assert_eq!(harness.state().chosen, [BrowserBarAction::Labels]);
    assert!(!harness.state().labels);
    options(&mut harness);
    assert_eq!(line(&harness, "Show labels"), (true, Some(Toggled::False)));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(
        harness.get_by_label("IMG_0000.ARW").rect().size(),
        egui::vec2(120.0, 120.0)
    );
    assert_eq!(
        list(&harness).height(),
        browser_height(&ctx, ThumbnailSize::Medium, false, 1)
    );
    assert_eq!(
        browser_height(&ctx, ThumbnailSize::Medium, true, 1)
            - browser_height(&ctx, ThumbnailSize::Medium, false, 1),
        40.0
    );
    // A Small cell's caption is the name alone, 20 points.
    assert_eq!(ThumbnailSize::Small.cell(true), egui::vec2(84.0, 104.0));
}

#[test]
fn with_no_photos_the_browser_bar_s_buttons_are_disabled() {
    let mut harness = browser(0);
    harness.run();
    for name in ["Sort", "Thumbnail size"] {
        let button = harness.get_by_role_and_label(Role::Button, name);
        assert!(button.accesskit_node().is_disabled(), "{name}");
    }
}

#[test]
fn sort_s_menu_checks_the_key_and_the_direction_and_reports_a_choice() {
    let mut harness = browser(12);
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
        [BrowserBarAction::Sort(SortKey::FileName)]
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
        Some(&BrowserBarAction::Descending(true))
    );
    let sort = harness.get_by_role_and_label(Role::Button, "Sort");
    assert_eq!(
        sort.accesskit_node().value().as_deref(),
        Some("File name, descending")
    );
}

#[test]
fn zoom_s_menu_checks_the_size_and_a_choice_sizes_the_cells_and_the_panel() {
    let mut harness = browser(12);
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
        [BrowserBarAction::Size(ThumbnailSize::Large)]
    );
    assert_eq!(harness.get_by_label("IMG_0000.ARW").rect().width(), 176.0);
    let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
    let height = browser_height(&harness.ctx, ThumbnailSize::Large, true, 1);
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
fn the_browser_bar_is_photographed() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            640.0,
            browser_height(&egui::Context::default(), ThumbnailSize::Small, true, 1) + 40.0,
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
            Browser::new(names.len(), Some(1), thumb)
                .sort(Sort {
                    key: SortKey::FileName,
                    descending: true,
                })
                .size(ThumbnailSize::Small)
                .show(ui);
        });
    harness.run();
    harness.snapshot("browser-bar");
}

/// A window with the Browser along its bottom as the app lays it out: its rows the
/// edge's, the edge shown last, and what the edge reported.
struct Bottom {
    rows: usize,
    most: usize,
    count: usize,
    actions: Vec<BrowserEdgeAction>,
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
                let edge = BrowserEdge::new(state.rows, state.most, ThumbnailSize::Small);
                let lines = edge.lines(&ctx);
                let names: Vec<String> = (0..state.count)
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let panel = egui::Panel::bottom("browser")
                    .exact_size(browser_height(&ctx, ThumbnailSize::Small, true, lines))
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
                        Browser::new(state.count, None, thumb)
                            .size(ThumbnailSize::Small)
                            .lines(lines)
                            .show(ui)
                    });
                if let Some(BrowserAction::Open(i)) = panel.inner.action {
                    state.opened.push(i);
                }
                egui::CentralPanel::default().show(ui, |_| {});
                if let Some(action) = edge.show(ui, panel.response.rect) {
                    state.rows = match action {
                        BrowserEdgeAction::Resize(rows) => rows,
                        BrowserEdgeAction::Reset => 1,
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
    harness.get_by_role_and_label(Role::Splitter, "Browser height")
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
fn dragging_the_browser_s_edge_adds_and_takes_whole_rows() {
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
    assert_eq!(harness.state().actions, [BrowserEdgeAction::Reset]);
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
fn a_right_click_on_the_browser_bar_opens_its_options_menu() {
    let mut harness = browser(12);
    harness.run();
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Browser bar")
        .rect();
    // The bar's empty middle, between Sort at its top and Zoom at its bottom.
    right_click(&mut harness, bar.center());
    let menu = harness.get_by_role_and_label(Role::Menu, "Browser options");
    assert!(menu.query_by_label("Show labels").is_some());
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
        [BrowserBarAction::Position(BrowserPosition::Left)]
    );
    // It opens with no photos too.
    let mut harness = browser(0);
    harness.run();
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Browser bar")
        .rect();
    // The bar's empty middle, between Sort at its top and Zoom at its bottom.
    right_click(&mut harness, bar.center());
    assert!(
        harness
            .query_by_role_and_label(Role::Menu, "Browser options")
            .is_some()
    );
}

/// Decision 91: a cell's right-click asks for its photo's menu at the pointer, and the menu
/// key and Shift+F10 on the focused cell under it, each with the cell, which the menu's
/// focus goes back to; the app builds the menu.
#[test]
fn a_cell_s_right_click_and_menu_keys_ask_for_its_photo_s_menu() {
    let mut harness = browser(12);
    harness.run();
    let cell = harness.get_by_label("IMG_0002.ARW").rect();
    right_click(&mut harness, cell.center());
    let asked = harness.state().menus.clone();
    assert_eq!(asked.len(), 1);
    assert_eq!((asked[0].photo, asked[0].at), (2, cell.center()));
    // No photo opens on a right-click: the app makes it active.
    harness.get_by_label("IMG_0003.ARW").focus();
    harness.run();
    let under = harness.get_by_label("IMG_0003.ARW").rect().left_bottom();
    for (modifiers, key) in [
        (egui::Modifiers::NONE, egui::Key::Menu),
        (egui::Modifiers::SHIFT, egui::Key::F10),
    ] {
        harness.key_press_modifiers(modifiers, key);
        harness.run();
        let menu = *harness.state().menus.last().unwrap();
        assert_eq!((menu.photo, menu.at), (3, under), "{key:?}");
    }
    assert_eq!(harness.state().menus.len(), 3);
    // Focused elsewhere, the keys ask for nothing.
    harness.get_by_role_and_label(Role::Button, "Sort").focus();
    harness.run();
    harness.key_press(egui::Key::Menu);
    harness.run();
    assert_eq!(harness.state().menus.len(), 3);
}

/// Decision 91: AccessKit's ShowContextMenu on a cell asks for its photo's menu under it,
/// as its keys do.
#[test]
fn screen_readers_ask_for_a_cell_s_menu() {
    let mut harness = browser(12);
    harness.run();
    let cell = harness.get_by_label("IMG_0004.ARW");
    let under = cell.rect().left_bottom();
    let node = cell.accesskit_node();
    let show = egui::accesskit::Action::ShowContextMenu;
    assert!(
        node.data().supports_action(show),
        "the cell offers its menu"
    );
    let (target_node, target_tree) = node.locate();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            target_node,
            target_tree,
            action: egui::accesskit::Action::ShowContextMenu,
            data: None,
        },
    ));
    harness.run();
    let menus = &harness.state().menus;
    assert_eq!(menus.len(), 1);
    assert_eq!((menus[0].photo, menus[0].at), (4, under));
}

/// Decision 91: on the Mac a Control-click on a cell asks for its menu, as a right-click
/// does, and opens no photo; elsewhere it is a click with Ctrl, which adds the photo to the
/// selection.
#[test]
fn a_control_click_on_a_cell_opens_its_menu_on_the_mac_alone() {
    use egui::os::OperatingSystem;
    for os in [OperatingSystem::Mac, OperatingSystem::Nix] {
        let mut harness = browser_on(12, os);
        harness.run();
        let at = harness.get_by_label("IMG_0002.ARW").rect().center();
        // Control, which is Ctrl, the command key, off the Mac.
        let ctrl = egui::Modifiers {
            ctrl: true,
            command: os != OperatingSystem::Mac,
            ..egui::Modifiers::NONE
        };
        harness.get_by_label("IMG_0002.ARW").click_modifiers(ctrl);
        harness.run();
        let browser = harness.state();
        if os == OperatingSystem::Mac {
            assert_eq!(browser.menus.len(), 1, "{os:?}");
            assert_eq!((browser.menus[0].photo, browser.menus[0].at), (2, at));
            assert!(browser.actions.is_empty(), "{os:?}: {:?}", browser.actions);
        } else {
            assert!(browser.menus.is_empty(), "{os:?}");
            assert_eq!(browser.actions, [BrowserAction::ToggleSelected(2)]);
        }
    }
}

/// Decision 91: a Control-click on the Mac asks for the menu even when Control is let go
/// in the frame the button comes up, after it: the click's own modifiers count.
#[test]
fn a_control_click_counts_control_as_the_button_came_up() {
    let mut harness = browser_on(12, egui::os::OperatingSystem::Mac);
    harness.run();
    let at = harness.get_by_label("IMG_0002.ARW").rect().center();
    let ctrl = egui::Modifiers {
        ctrl: true,
        ..egui::Modifiers::NONE
    };
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: ctrl,
    };
    harness.input_mut().events.extend([
        egui::Event::PointerMoved(at),
        egui::Event::ModifiersChanged(ctrl),
        button(true),
    ]);
    harness.step();
    // The button comes up, then Control, in one frame.
    harness.input_mut().events.extend([
        button(false),
        egui::Event::ModifiersChanged(egui::Modifiers::NONE),
    ]);
    harness.step();
    harness.run();
    let browser = harness.state();
    assert_eq!(browser.menus.len(), 1);
    assert_eq!((browser.menus[0].photo, browser.menus[0].at), (2, at));
    assert!(browser.actions.is_empty(), "{:?}", browser.actions);
}

/// A window with the Browser as a column at `position`, `columns` wide as its edge
/// leaves it, beside the photo, and what the edge reported and the panel opened.
struct Column {
    position: BrowserPosition,
    columns: usize,
    count: usize,
    actions: Vec<BrowserEdgeAction>,
    opened: Vec<usize>,
}

fn column(position: BrowserPosition, columns: usize, count: usize) -> Harness<'static, Column> {
    Harness::builder()
        .with_size(egui::vec2(1000.0, 600.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(install)
        .build_ui_state(
            |ui, state: &mut Column| {
                let ctx = ui.ctx().clone();
                let size = ThumbnailSize::Small;
                let edge = BrowserEdge::new(state.columns, 4, size).position(state.position);
                let lines = edge.lines(&ctx);
                let names: Vec<String> = (0..state.count)
                    .map(|i| format!("IMG_{i:04}.ARW"))
                    .collect();
                let side = match state.position {
                    BrowserPosition::Left => egui::Panel::left("browser"),
                    _ => egui::Panel::right("browser"),
                };
                let panel = side
                    .exact_size(browser_width(&ctx, size, lines))
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
                        Browser::new(state.count, Some(1), thumb)
                            .size(size)
                            .lines(lines)
                            .position(state.position)
                            .show(ui)
                    });
                if let Some(BrowserAction::Open(i)) = panel.inner.action {
                    state.opened.push(i);
                }
                egui::CentralPanel::default().show(ui, |_| {});
                if let Some(action) = edge.show(ui, panel.response.rect) {
                    state.columns = match action {
                        BrowserEdgeAction::Resize(columns) => columns,
                        BrowserEdgeAction::Reset => 1,
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
    harness.get_by_role_and_label(Role::Splitter, "Browser width")
}

#[test]
fn at_a_side_the_cells_fill_rows_of_the_columns_and_the_bar_is_on_top() {
    let mut harness = column(BrowserPosition::Left, 2, 9);
    harness.run();
    let at = |harness: &Harness<'_, Column>, name: &str| harness.get_by_label(name).rect().min;
    let first = at(&harness, "IMG_0000.ARW");
    assert_eq!(at(&harness, "IMG_0001.ARW"), first + egui::vec2(PITCH, 0.0));
    assert_eq!(at(&harness, "IMG_0002.ARW"), first + egui::vec2(0.0, ROW));
    let bar = harness
        .get_by_role_and_label(Role::Toolbar, "Browser bar")
        .rect();
    assert!(bar.bottom() <= first.y);
    // At a side the bar is a row across the panel's top, its buttons side by side.
    let sort = harness.get_by_role_and_label(Role::Button, "Sort").rect();
    let zoom = harness
        .get_by_role_and_label(Role::Button, "Thumbnail size")
        .rect();
    assert_eq!(sort.center().y, zoom.center().y);
    // Sort at the row's start, Zoom alone at its end.
    assert_eq!(sort.left(), bar.left() + 8.0);
    assert!(zoom.left() - sort.right() > 40.0);
    assert_eq!(zoom.right(), bar.right() - 8.0);
    // At the window's left, its edge on its right.
    let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
    assert!(list.left() <= 8.0);
    assert_eq!(width_splitter(&harness).rect().center().x, list.right());
    // At the right, its edge on its left, next to the window's end.
    let mut harness = column(BrowserPosition::Right, 1, 9);
    harness.run();
    let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
    assert!(list.right() >= 1000.0 - 8.0);
    assert_eq!(width_splitter(&harness).rect().center().x, list.left());
}

#[test]
fn at_a_side_the_arrows_move_as_the_cells_lie() {
    // Rows of two: 0 and 1, 2 and 3, 4.
    let mut harness = column(BrowserPosition::Right, 2, 5);
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
    for (position, towards) in [(BrowserPosition::Left, 1.0), (BrowserPosition::Right, -1.0)] {
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
fn the_browser_at_the_left_and_the_right_is_photographed() {
    let mut results = egui_kittest::SnapshotResults::new();
    for (position, name) in [
        (BrowserPosition::Left, "browser-left"),
        (BrowserPosition::Right, "browser-right"),
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
        let mut harness = browser(count);
        harness.run();
        let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
        let found = thumb_pixels(&mut harness, bar_band(list, false));
        assert_eq!(found > 100, shows, "{count} photos: {found} pixels");
    }
    // A column of two: 40 Small cells overflow 600 points, 4 fit.
    for (count, shows) in [(40, true), (4, false)] {
        let mut harness = column(BrowserPosition::Left, 2, count);
        harness.run();
        let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
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
    let mut harness = browser(60);
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
    let mut harness = column(BrowserPosition::Right, 2, 60);
    harness.run();
    let at = harness.get_by_label("IMG_0002.ARW").rect().center();
    wheel(&mut harness, at, egui::vec2(0.0, -1300.0));
    assert!(harness.query_by_label("IMG_0000.ARW").is_none());
    assert!(harness.query_by_label("IMG_0030.ARW").is_some());
}

#[test]
fn the_scroll_bar_is_photographed() {
    let mut harness = browser(30);
    harness.run();
    let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
    // Under the pointer, the thumb a step brighter.
    harness.hover_at(egui::pos2(list.left() + 60.0, list.bottom() - 8.0));
    harness.run();
    harness.snapshot("browser-scroll-bar");
}

/// With no photos, the panel shows the struck-through photo, the message under it and the
/// hint under that, inside the strip and wrapped to a one-column side's width.
#[test]
fn an_empty_panel_says_why_under_its_icon_in_a_row_and_a_column() {
    let mut results = egui_kittest::SnapshotResults::new();
    let message = "No photos available";
    let hint = "Choose a subfolder in Collections.";
    for (position, name) in [
        (BrowserPosition::Bottom, "browser-empty"),
        (BrowserPosition::Left, "browser-empty-left"),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(640.0, 300.0))
            .with_context_setup(install)
            .build_ui(move |ui| {
                let ctx = ui.ctx().clone();
                let size = ThumbnailSize::Small;
                let thumb = |_| unreachable!("no photos");
                let panel = Browser::new(0, None, thumb)
                    .size(size)
                    .position(position)
                    .empty(message)
                    .empty_hint(hint);
                if position == BrowserPosition::Bottom {
                    egui::Panel::bottom("browser")
                        .exact_size(browser_height(&ctx, size, true, 1))
                        .frame(egui::Frame::NONE)
                        .show(ui, |ui| panel.show(ui));
                } else {
                    egui::Panel::left("browser")
                        .exact_size(browser_width(&ctx, size, 1))
                        .resizable(false)
                        .frame(egui::Frame::NONE)
                        .show(ui, |ui| panel.show(ui));
                }
            });
        harness.run();
        let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
        let shapes = &harness.output().shapes;
        let text = |words: &str| {
            shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == words => {
                        Some(text.visual_bounding_rect())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{words} is drawn, {position:?}"))
        };
        // The icon: an icon's texture of its size, in the panel but not in its browser bar,
        // whose buttons are icons too.
        let bar = harness
            .get_by_role_and_label(Role::Toolbar, "Browser bar")
            .rect();
        let side = Icon::PhotoOff.size();
        let icons: Vec<egui::Rect> = shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                    Some(mesh.calc_bounds())
                }
                _ => None,
            })
            .filter(|rect| {
                (rect.width() - side).abs() < 1.0
                    && (rect.height() - side).abs() < 1.0
                    && list.contains_rect(*rect)
                    && !bar.intersects(*rect)
            })
            .collect();
        let [icon] = icons[..] else {
            panic!("one icon in the panel, {icons:?}, {position:?}");
        };
        let (said, hinted) = (text(message), text(hint));
        for part in [icon, said, hinted] {
            assert!(
                list.contains_rect(part),
                "{part:?} in {list:?}, {position:?}"
            );
        }
        assert!(icon.bottom() <= said.top() && said.bottom() <= hinted.top());
        // Each centred under the icon, to the pixel the icon is snapped to.
        for part in [said, hinted] {
            assert!(
                (part.center().x - icon.center().x).abs() <= 1.0,
                "{part:?} under {icon:?}, {position:?}"
            );
        }
        results.add(harness.try_snapshot(name));
    }
}

/// What the Browser with pictures reported: what was done to the photos, and on their
/// captions' first lines.
#[derive(Default)]
struct Clicked {
    actions: Vec<BrowserAction>,
    cells: Vec<(usize, CellAction)>,
}

/// The Browser with `count` rated photos, each with a portrait picture, a third of its
/// square empty round it, and what it reported.
fn pictured(count: usize) -> Harness<'static, Clicked> {
    let texture = std::sync::Arc::new(std::sync::OnceLock::<egui::TextureHandle>::new());
    Harness::builder()
        .with_size(egui::vec2(900.0, 360.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, clicked: &mut Clicked| {
                let portrait = texture.get_or_init(|| {
                    let image = egui::ColorImage::new([2, 3], vec![egui::Color32::RED; 6]);
                    (ui.ctx()).load_texture("thumbnail", image, egui::TextureOptions::LINEAR)
                });
                let names: Vec<String> = (0..count).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: false,
                    status: None,
                    selected: i < 2,
                    picture: Some(CellPicture {
                        texture: portrait.id(),
                        size: portrait.size_vec2(),
                    }),
                    rating: Some(CellRating::default()),
                };
                let output = Browser::new(count, Some(0), thumb).show(ui);
                clicked.actions.extend(output.action);
                clicked.cells.extend(output.cell);
            },
            Clicked::default(),
        )
}

/// A click of the primary button at `pos` with `modifiers` held, in frames of their own.
fn click_at<T>(harness: &mut Harness<'_, T>, pos: egui::Pos2, modifiers: egui::Modifiers) {
    click_letting_go(harness, pos, modifiers, false);
}

/// [`click_at`], `modifiers` let go in the frame the button comes up, after it, where
/// `together`, else in the next.
fn click_letting_go<T>(
    harness: &mut Harness<'_, T>,
    pos: egui::Pos2,
    modifiers: egui::Modifiers,
    together: bool,
) {
    harness.event(egui::Event::ModifiersChanged(modifiers));
    harness.hover_at(pos);
    harness.step();
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    harness.event(button(true));
    harness.step();
    harness.event(button(false));
    if !together {
        harness.step();
    }
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    harness.step();
    harness.step();
}

/// The Browser's empty space in `harness`: inside the second cell, beside its picture and
/// on its caption above the first line; between the first two cells; after the last, and
/// at the panel's end.
fn empty_space<T>(harness: &Harness<'_, T>, last: usize) -> Vec<egui::Pos2> {
    let cell = harness.get_by_label("IMG_0001.ARW").rect();
    let first = harness.get_by_label("IMG_0000.ARW").rect();
    let end = harness
        .get_by_label(&format!("IMG_{last:04}.ARW"))
        .rect()
        .right();
    let list = harness.get_by_role_and_label(Role::List, "Browser").rect();
    // The square's bottom, the caption's top.
    let square = cell.top() + cell.width();
    vec![
        egui::pos2(cell.left() + 8.0, cell.top() + 60.0),
        egui::pos2(cell.center().x, square + 4.0),
        egui::pos2(first.right() + 4.0, first.center().y),
        egui::pos2(end + 40.0, cell.center().y),
        egui::pos2(list.right() - 2.0, cell.center().y),
    ]
}

/// A plain click on the Browser's empty space selects nothing (`docs/gui.md`, The
/// pointer): a cell's margin round its picture or on its caption, a gap between cells, the
/// area after the last cell to the panel's end; a gap too where the cells overflow the
/// panel on a touch screen, which a drag scrolls them on.
#[test]
fn a_click_on_the_browser_s_empty_space_reports_clear_selection() {
    let mut harness = pictured(3);
    harness.run();
    for at in empty_space(&harness, 2) {
        click_at(&mut harness, at, egui::Modifiers::NONE);
        let clicked = harness.state();
        assert_eq!(clicked.actions, [BrowserAction::ClearSelection], "{at:?}");
        assert!(clicked.cells.is_empty(), "{at:?}: {:?}", clicked.cells);
        harness.state_mut().actions.clear();
    }
    // On a touch screen, whose drag scrolls the cells under it.
    let mut harness = pictured(12);
    for phase in [egui::TouchPhase::Start, egui::TouchPhase::End] {
        harness.event(egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(0),
            phase,
            pos: egui::pos2(890.0, 350.0),
            force: None,
        });
    }
    harness.run();
    assert!(harness.ctx.input(|input| input.has_touch_screen()));
    let first = harness.get_by_label("IMG_0000.ARW").rect();
    click_at(
        &mut harness,
        egui::pos2(first.right() + 4.0, first.center().y),
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().actions, [BrowserAction::ClearSelection]);
}

/// Ctrl or Cmd-click and Shift-click on empty space do nothing, so a missed Ctrl-click
/// never drops a selection, Ctrl let go as the button comes up included.
#[test]
fn ctrl_and_shift_clicks_on_empty_space_report_nothing() {
    let mut harness = pictured(3);
    harness.run();
    for modifiers in [
        egui::Modifiers::COMMAND,
        egui::Modifiers::CTRL,
        egui::Modifiers::SHIFT,
    ] {
        for (at, together) in empty_space(&harness, 2)
            .into_iter()
            .flat_map(|at| [(at, false), (at, true)])
        {
            click_letting_go(&mut harness, at, modifiers, together);
            let actions = &harness.state().actions;
            assert!(
                actions.is_empty(),
                "{at:?} {modifiers:?}, let go together {together}: {actions:?}"
            );
        }
    }
}

/// The picture keeps its click, which opens the photo, and so do Ctrl and Shift-click on
/// it; a star rates, and the rest of the rating's row and the file name select nothing.
#[test]
fn a_click_on_a_picture_its_rating_or_its_name_keeps_its_own() {
    let mut harness = pictured(3);
    harness.run();
    let cell = harness.get_by_label("IMG_0001.ARW");
    let picture = egui::pos2(cell.rect().center().x, cell.rect().top() + 60.0);
    let star = cell
        .get_by_role_and_label(Role::Button, "Rate 2 stars")
        .rect();
    let reject = cell.get_by_role_and_label(Role::Button, "Reject").rect();
    let name = egui::pos2(cell.rect().center().x, cell.rect().bottom() - 6.0);
    // The rating's row between the reject mark and the stars.
    let row = egui::pos2((reject.right() + star.left()) / 2.0 - 10.0, star.center().y);
    for (modifiers, action) in [
        (egui::Modifiers::NONE, BrowserAction::Open(1)),
        (egui::Modifiers::COMMAND, BrowserAction::ToggleSelected(1)),
        (egui::Modifiers::SHIFT, BrowserAction::SelectRange(1)),
    ] {
        click_at(&mut harness, picture, modifiers);
        assert_eq!(harness.state().actions, [action]);
        harness.state_mut().actions.clear();
    }
    click_at(&mut harness, star.center(), egui::Modifiers::NONE);
    assert_eq!(harness.state().cells, [(1, CellAction::Rate(2))]);
    for at in [row, name] {
        click_at(&mut harness, at, egui::Modifiers::NONE);
        let actions = &harness.state().actions;
        assert!(actions.is_empty(), "{at:?}: {actions:?}");
    }
}
