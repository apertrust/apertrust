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

//! The grid cell and the grid panel's cells (`docs/gui.md`, The grid panel, The cell): the
//! picture fitted or filling its square, the rings round it, the corner marks that always
//! show, the caption and its first line's clicks, what screen readers hear, and the
//! panel's keys.

use apertrust_ui::components::{
    CellAction, CellPicture, CellRating, CellStatus, ColourLabel, ColourLabels, GridCell,
    GridPanel, GridPanelAction, Thumb,
};
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::{Key, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn install(ctx: &egui::Context) {
    apertrust_ui::install(ctx, Theme::default());
}

/// What a cell was given, to draw one.
#[derive(Clone, Copy)]
struct Cell {
    edited: bool,
    view_only: bool,
    unreadable: bool,
    selected: bool,
    active: bool,
    labels: bool,
    side: f32,
    rating: Option<CellRating>,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            edited: false,
            view_only: false,
            unreadable: false,
            selected: false,
            active: false,
            labels: true,
            side: GridCell::MEDIUM,
            rating: None,
        }
    }
}

/// Three stars, not rejected, red and blue.
fn rated() -> Option<CellRating> {
    Some(CellRating {
        stars: 3,
        rejected: false,
        labels: ColourLabels::of([ColourLabel::Red, ColourLabel::Blue]),
    })
}

/// A cell drawn as `state` says, 10 points in, and what its first line reported.
fn cell(state: Cell) -> Harness<'static, Vec<CellAction>> {
    Harness::builder()
        .with_size(egui::vec2(160.0, 200.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<CellAction>| {
                let status = if state.unreadable {
                    Some(CellStatus::Unreadable("not an Apertrust edit"))
                } else if state.view_only {
                    Some(CellStatus::ViewOnly)
                } else {
                    None
                };
                ui.add_space(10.0);
                let output = GridCell::new("IMG_2041.ARW")
                    .side(state.side)
                    .labels(state.labels)
                    .rating(state.rating)
                    .edited(state.edited)
                    .status(status)
                    .selected(state.selected)
                    .active(state.active)
                    .show(ui);
                actions.extend(output.action);
            },
            Vec::new(),
        )
}

/// The texts the harness painted, in order.
fn painted<T>(harness: &Harness<'_, T>) -> Vec<String> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// The pictures painted: icons and thumbnails.
fn meshes<T>(harness: &Harness<'_, T>) -> usize {
    harness
        .output()
        .shapes
        .iter()
        .filter(|clipped| matches!(clipped.shape, Shape::Mesh(_)))
        .count()
}

/// The rings round the picture: their rectangles, widths and colours, the strokes painted
/// outside a rectangle.
fn rings<T>(harness: &Harness<'_, T>) -> Vec<(egui::Rect, f32, egui::Color32)> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect)
                if rect.stroke_kind == egui::StrokeKind::Outside && rect.stroke.width > 0.0 =>
            {
                Some((rect.rect, rect.stroke.width, rect.stroke.color))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_corner_marks_show_always_with_labels_on_or_off_and_nothing_more_on_hover() {
    // The pencil, the lock and the warning each one more picture than a plain cell has.
    let count = |state: Cell| {
        let mut harness = cell(state);
        harness.run();
        meshes(&harness)
    };
    for labels in [true, false] {
        let plain = count(Cell {
            labels,
            ..Cell::default()
        });
        for state in [
            Cell {
                edited: true,
                labels,
                ..Cell::default()
            },
            Cell {
                view_only: true,
                labels,
                ..Cell::default()
            },
            Cell {
                unreadable: true,
                labels,
                ..Cell::default()
            },
        ] {
            assert_eq!(count(state), plain + 1, "labels {labels}");
        }
        assert_eq!(
            count(Cell {
                edited: true,
                view_only: true,
                labels,
                ..Cell::default()
            }),
            plain + 2
        );
    }
    // On hover the cell shows no more than at rest: no file type, no row of marks.
    let mut harness = cell(Cell {
        edited: true,
        ..Cell::default()
    });
    harness.run();
    let (texts, pictures) = (painted(&harness), meshes(&harness));
    harness.get_by_label("IMG_2041.ARW").hover();
    harness.run();
    assert_eq!(meshes(&harness), pictures);
    // The tooltip follows the pointer; the cell's own texts come first.
    assert_eq!(painted(&harness)[..texts.len()], texts[..]);
    assert!(!painted(&harness).iter().any(|text| text == "ARW"));
}

/// A cell drawn with a thumbnail `size` pixels made into a texture, or with none, filling
/// its square or not, and the texture's id.
fn pictured(
    size: Option<[usize; 2]>,
    square: bool,
    state: Cell,
) -> (Harness<'static, ()>, egui::TextureId) {
    let texture = std::sync::Arc::new(std::sync::OnceLock::<egui::TextureHandle>::new());
    let made = std::sync::Arc::clone(&texture);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(160.0, 200.0))
        .with_context_setup(install)
        .build_ui(move |ui| {
            let [w, h] = size.unwrap_or([3, 2]);
            let handle = made.get_or_init(|| {
                let image = egui::ColorImage::new([w, h], vec![egui::Color32::RED; w * h]);
                ui.ctx()
                    .load_texture("thumbnail", image, egui::TextureOptions::LINEAR)
            });
            let picture = size.map(|_| CellPicture {
                texture: handle.id(),
                size: handle.size_vec2(),
            });
            ui.add_space(10.0);
            GridCell::new("IMG_2041.ARW")
                .picture(picture)
                .square(square)
                .selected(state.selected)
                .active(state.active)
                .labels(state.labels)
                .show(ui);
        });
    harness.run();
    let id = texture.get().expect("made on the first frame").id();
    (harness, id)
}

/// The rectangles meshes of `texture` cover, and the part of the texture each draws.
fn drawn(harness: &Harness<'_, ()>, texture: egui::TextureId) -> Vec<(egui::Rect, egui::Rect)> {
    harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Mesh(mesh) if mesh.texture_id == texture => {
                let (mut lo, mut hi) = (egui::pos2(1.0, 1.0), egui::pos2(0.0, 0.0));
                for vertex in &mesh.vertices {
                    lo = lo.min(vertex.uv);
                    hi = hi.max(vertex.uv);
                }
                Some((mesh.calc_bounds(), egui::Rect::from_min_max(lo, hi)))
            }
            _ => None,
        })
        .collect()
}

/// The cell's square: its top, as wide as it is.
fn square_of(harness: &Harness<'_, ()>) -> egui::Rect {
    let cell = harness.get_by_label("IMG_2041.ARW").rect();
    egui::Rect::from_min_size(cell.min, egui::vec2(cell.width(), cell.width()))
}

#[test]
fn a_picture_is_fitted_at_its_square_s_bottom_centre_with_no_frame_round_it() {
    let near = |a: egui::Rect, b: egui::Rect| {
        (a.min - b.min).length() < 0.01 && (a.max - b.max).length() < 0.01
    };
    // A landscape picture is as wide as the square and sits on its bottom.
    let (harness, texture) = pictured(Some([3, 2]), false, Cell::default());
    let [(picture, uv)] = drawn(&harness, texture)[..] else {
        panic!("the thumbnail drawn once: {:?}", drawn(&harness, texture));
    };
    let square = square_of(&harness);
    let expected = egui::Rect::from_min_max(
        egui::pos2(square.left(), square.bottom() - 80.0),
        square.max,
    );
    assert!(near(picture, expected), "{picture:?} in {square:?}");
    assert!(near(
        uv,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
    ));
    // Nothing is painted round it: no box, no border, no ring while not selected.
    assert!(rings(&harness).is_empty(), "{:?}", rings(&harness));
    // A filled box would lie inside the window's own ground.
    let cell = harness.get_by_label("IMG_2041.ARW").rect().expand(8.0);
    let boxes: Vec<egui::Rect> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT => Some(rect.rect),
            _ => None,
        })
        .filter(|rect| cell.contains_rect(*rect))
        .collect();
    assert!(boxes.is_empty(), "a box behind the picture: {boxes:?}");
    // A portrait one is as tall as the square, centred across it.
    let (harness, texture) = pictured(Some([2, 3]), false, Cell::default());
    let [(picture, _)] = drawn(&harness, texture)[..] else {
        panic!("drawn once");
    };
    let square = square_of(&harness);
    assert!(
        (picture.height() - 120.0).abs() < 0.01 && (picture.width() - 80.0).abs() < 0.01,
        "{picture:?}"
    );
    assert!((picture.center().x - square.center().x).abs() < 0.01);
    assert_eq!(picture.bottom(), square.bottom());

    // With square thumbnails it fills the square, cut evenly at its longer sides.
    let (harness, texture) = pictured(Some([3, 2]), true, Cell::default());
    let [(picture, uv)] = drawn(&harness, texture)[..] else {
        panic!("drawn once");
    };
    assert!(near(picture, square_of(&harness)), "{picture:?}");
    assert!(
        (uv.min.x - 1.0 / 6.0).abs() < 0.01 && (uv.max.x - 5.0 / 6.0).abs() < 0.01,
        "{uv:?}"
    );
    assert!((uv.min.y, uv.max.y) == (0.0, 1.0), "{uv:?}");

    // The picture stands in for the name in its square; the caption still names it.
    let (harness, _) = pictured(Some([3, 2]), false, Cell::default());
    assert_eq!(painted(&harness), ["IMG_2041.ARW"]);
    // Without one the name stands in a box of the square's size, and in the caption.
    let (harness, texture) = pictured(None, false, Cell::default());
    assert!(drawn(&harness, texture).is_empty());
    assert_eq!(painted(&harness), ["IMG_2041.ARW", "IMG_2041.ARW"]);
}

#[test]
fn the_rings_hug_the_picture_two_points_out_thin_for_selected_and_thick_for_active() {
    let t = Theme::default();
    let ring = |square: bool, state: Cell| {
        let (harness, texture) = pictured(Some([3, 2]), square, state);
        let (picture, _) = drawn(&harness, texture)[0];
        let [ring] = rings(&harness)[..] else {
            panic!("one ring: {:?}", rings(&harness));
        };
        (picture, ring)
    };
    let selected = Cell {
        selected: true,
        ..Cell::default()
    };
    let active = Cell {
        active: true,
        selected: true,
        ..Cell::default()
    };
    for square in [false, true] {
        let (picture, (rect, width, colour)) = ring(square, selected);
        assert_eq!((width, colour), (1.0, t.colors.frame_ring));
        assert_eq!(rect, picture.expand(2.0), "square {square}");
        let (picture, (rect, width, colour)) = ring(square, active);
        assert_eq!((width, colour), (3.0, t.colors.frame_ring));
        assert_eq!(rect, picture.expand(2.0), "square {square}");
    }
    // The icons' grey; never the accent.
    assert_ne!(t.colors.frame_ring, t.colors.accent);
    assert_ne!(t.colors.frame_ring, t.colors.accent_text);
}

#[test]
fn the_caption_shows_the_name_under_the_square_at_medium_and_the_name_alone_at_small() {
    // Medium with a rating: the first line's buttons, then the name, under the square.
    let mut harness = cell(Cell {
        rating: rated(),
        ..Cell::default()
    });
    harness.run();
    let item = harness
        .get_by_role_and_label(Role::ListItem, "IMG_2041.ARW")
        .rect();
    assert_eq!(item.size(), egui::vec2(120.0, 160.0));
    let reject = harness.get_by_role_and_label(Role::Button, "Reject").rect();
    let star = harness
        .get_by_role_and_label(Role::Button, "Rate 1 star")
        .rect();
    let circle = harness
        .get_by_role_and_label(Role::Button, "Colour labels")
        .rect();
    // The first line 10 points under the square, the reject at its start, the circle at its
    // end, the five stars centred between, all 12 points.
    let square_bottom = item.top() + 120.0;
    for mark in [reject, star, circle] {
        assert_eq!(mark.top(), square_bottom + 10.0);
        assert_eq!(mark.size(), egui::vec2(12.0, 12.0));
    }
    assert_eq!(reject.left(), item.left() + 4.0);
    assert_eq!(circle.right(), item.right() - 4.0);
    let last = harness
        .get_by_role_and_label(Role::Button, "Rate 5 stars")
        .rect();
    assert!((star.left() - item.left() - (item.right() - last.right())).abs() < 0.01);
    // The square's name, then the caption's.
    assert_eq!(painted(&harness), ["IMG_2041.ARW", "IMG_2041.ARW"]);

    // Without a rating the first line is not drawn and the name takes its place.
    let mut harness = cell(Cell::default());
    harness.run();
    assert!(harness.query_by_label("Reject").is_none());
    assert!(harness.query_by_label("Rate 1 star").is_none());
    assert_eq!(painted(&harness), ["IMG_2041.ARW", "IMG_2041.ARW"]);

    // Small: a 20-point caption of the name alone, whatever the rating.
    let mut harness = cell(Cell {
        side: 84.0,
        rating: rated(),
        ..Cell::default()
    });
    harness.run();
    let item = harness
        .get_by_role_and_label(Role::ListItem, "IMG_2041.ARW")
        .rect();
    assert_eq!(item.size(), egui::vec2(84.0, 104.0));
    assert!(harness.query_by_label("Reject").is_none());
    assert_eq!(painted(&harness).len(), 2);

    // Labels off: the square alone.
    let mut harness = cell(Cell {
        labels: false,
        rating: rated(),
        ..Cell::default()
    });
    harness.run();
    let item = harness
        .get_by_role_and_label(Role::ListItem, "IMG_2041.ARW")
        .rect();
    assert_eq!(item.size(), egui::vec2(120.0, 120.0));
    assert!(harness.query_by_label("Reject").is_none());
    assert_eq!(painted(&harness), ["IMG_2041.ARW"]);
}

#[test]
fn a_long_name_is_cut_in_its_middle_keeping_its_extension() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(160.0, 200.0))
        .with_context_setup(install)
        .build_ui(|ui| {
            GridCell::new("Wedding_2026-10-31_ceremony_0042.DNG").show(ui);
        });
    harness.run();
    let caption = painted(&harness).pop().unwrap();
    assert!(
        caption.contains('…') && caption.starts_with("Wedd") && caption.ends_with("42.DNG"),
        "{caption}"
    );
}

#[test]
fn the_first_line_s_clicks_rate_reject_and_open_the_labels_list() {
    let mut harness = cell(Cell {
        rating: rated(),
        ..Cell::default()
    });
    harness.run();
    // The stars, each a button, on while within the rating.
    let star = |harness: &Harness<'_, Vec<CellAction>>, name: &str| {
        harness
            .get_by_role_and_label(Role::Button, name)
            .accesskit_node()
            .toggled()
    };
    assert_eq!(star(&harness, "Rate 3 stars"), Some(Toggled::True));
    assert_eq!(star(&harness, "Rate 4 stars"), Some(Toggled::False));
    harness
        .get_by_role_and_label(Role::Button, "Rate 4 stars")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Reject")
        .click();
    harness.run();
    assert_eq!(
        *harness.state(),
        [CellAction::Rate(4), CellAction::ToggleReject]
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Reject")
            .accesskit_node()
            .toggled(),
        Some(Toggled::False)
    );

    // The circle: a button with a menu, its value the labels set.
    let circle = harness.get_by_role_and_label(Role::Button, "Colour labels");
    assert_eq!(
        circle.accesskit_node().value().as_deref(),
        Some("Red, Blue")
    );
    assert_eq!(
        circle.accesskit_node().data().has_popup(),
        Some(HasPopup::Menu)
    );
    circle.click();
    harness.run();
    let line = |harness: &Harness<'_, Vec<CellAction>>, name: &str| {
        let node = harness.get_by_role_and_label(Role::MenuItemCheckBox, name);
        node.accesskit_node().toggled()
    };
    for (name, on) in [
        ("Red", true),
        ("Yellow", false),
        ("Green", false),
        ("Blue", true),
        ("Purple", false),
    ] {
        let toggled = if on { Toggled::True } else { Toggled::False };
        assert_eq!(line(&harness, name), Some(toggled), "{name}");
    }
    // darktable's keys shown, F1 to F5.
    let red = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Red");
    assert_eq!(red.accesskit_node().data().keyboard_shortcut(), Some("F1"));
    let purple = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Purple");
    assert_eq!(
        purple.accesskit_node().data().keyboard_shortcut(),
        Some("F5")
    );
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Green")
        .click();
    harness.run();
    assert_eq!(
        harness.state().last(),
        Some(&CellAction::ToggleLabel(ColourLabel::Green))
    );
    // Clear labels, after a separator.
    harness
        .get_by_role_and_label(Role::Button, "Colour labels")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::MenuItem, "Clear labels")
        .click();
    harness.run();
    assert_eq!(harness.state().last(), Some(&CellAction::ClearLabels));
    // A click on the caption's buttons never opens the photo: it is the cell's own click.
    assert!(
        !harness
            .get_by_role_and_label(Role::ListItem, "IMG_2041.ARW")
            .accesskit_node()
            .is_selected()
            .unwrap_or(false)
    );
}

#[test]
fn a_cell_is_a_list_item_named_by_its_file_with_its_marks_in_its_description() {
    let mut harness = cell(Cell {
        edited: true,
        unreadable: true,
        selected: true,
        ..Cell::default()
    });
    harness.run();
    let item = harness.get_by_role_and_label(Role::ListItem, "IMG_2041.ARW");
    let node = item.accesskit_node();
    assert_eq!(node.toggled(), Some(Toggled::True));
    assert_eq!(
        node.description().as_deref(),
        Some("Edited. Its edit could not be read: not an Apertrust edit")
    );

    let mut view_only = cell(Cell {
        view_only: true,
        ..Cell::default()
    });
    view_only.run();
    let item = view_only.get_by_role_and_label(Role::ListItem, "IMG_2041.ARW");
    assert_eq!(item.accesskit_node().toggled(), Some(Toggled::False));
    assert_eq!(
        item.accesskit_node().description().as_deref(),
        Some("View only: a newer Apertrust wrote its edit")
    );

    // The rating, reject and labels, whatever the caption shows.
    for labels in [true, false] {
        let mut harness = cell(Cell {
            labels,
            rating: rated(),
            ..Cell::default()
        });
        harness.run();
        let item = harness.get_by_role_and_label(Role::ListItem, "IMG_2041.ARW");
        assert_eq!(
            item.accesskit_node().description().as_deref(),
            Some("3 stars. Colour labels: Red, Blue"),
            "labels {labels}"
        );
    }
    let mut rejected = cell(Cell {
        rating: Some(CellRating {
            rejected: true,
            ..CellRating::default()
        }),
        ..Cell::default()
    });
    rejected.run();
    let item = rejected.get_by_role_and_label(Role::ListItem, "IMG_2041.ARW");
    assert_eq!(
        item.accesskit_node().description().as_deref(),
        Some("Rejected")
    );
    assert_eq!(
        rejected
            .get_by_role_and_label(Role::Button, "Reject")
            .accesskit_node()
            .toggled(),
        Some(Toggled::True)
    );
}

/// The panel with `count` photos, the one at `active` open, and what it reported.
fn panel(count: usize, active: Option<usize>) -> Harness<'static, Vec<GridPanelAction>> {
    Harness::builder()
        .with_size(egui::vec2(700.0, 200.0))
        .with_context_setup(install)
        .build_ui_state(
            move |ui, actions: &mut Vec<GridPanelAction>| {
                let names: Vec<String> = (0..count).map(|i| format!("IMG_{i:04}.ARW")).collect();
                let thumb = |i: usize| Thumb {
                    name: &names[i],
                    edited: i.is_multiple_of(3),
                    status: None,
                    selected: false,
                    picture: None,
                    rating: None,
                };
                if let Some(action) = GridPanel::new(count, active, thumb).show(ui).action {
                    actions.push(action);
                }
            },
            Vec::new(),
        )
}

#[test]
fn the_arrow_keys_move_between_cells_and_enter_opens_the_one_focused() {
    let mut harness = panel(40, Some(0));
    harness.run();
    harness.get_by_label("IMG_0001.ARW").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    // Enter on the cell reached: two steps right and one left from IMG_0001.
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), vec![GridPanelAction::Open(2)]);

    // End goes to the last cell, scrolling to it; Home comes back.
    harness.key_press(Key::End);
    harness.run();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().last(), Some(&GridPanelAction::Open(39)));
    harness.key_press(Key::Home);
    harness.run();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().last(), Some(&GridPanelAction::Open(0)));
}

#[test]
fn the_cells_hang_under_the_grid_panel_s_list_so_a_screen_reader_hears_n_items() {
    let mut harness = panel(4, Some(0));
    harness.run();
    let list = harness.get_by_role_and_label(Role::List, "Grid panel");
    // Under the list, through the layout's own containers, which a reader skips.
    assert_eq!(list.query_all_by_role(Role::ListItem).count(), 4);
    // And no cell hangs anywhere else.
    assert_eq!(harness.query_all_by_role(Role::ListItem).count(), 4);
    let cell = harness.get_by_label("IMG_0000.ARW");
    let mut ancestor = cell.parent();
    let mut under_list = false;
    while let Some(node) = ancestor {
        under_list |= node.accesskit_node().role() == Role::List;
        ancestor = node.parent();
    }
    assert!(under_list);
}

#[test]
fn the_active_photo_is_scrolled_into_view() {
    // 40 cells are 5,000 points of cells in 700; the active one, far along, is drawn.
    let mut harness = panel(40, Some(30));
    harness.run();
    harness.run();
    assert!(harness.query_by_label("IMG_0030.ARW").is_some());
    assert!(harness.query_by_label("IMG_0001.ARW").is_none());
}

#[test]
fn the_panel_reports_the_photos_whose_cells_it_drew() {
    let shown = |count: usize, active: Option<usize>| {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(700.0, 200.0))
            .with_context_setup(install)
            .build_ui_state(
                move |ui, shown: &mut std::ops::Range<usize>| {
                    let names: Vec<String> =
                        (0..count).map(|i| format!("IMG_{i:04}.ARW")).collect();
                    let thumb = |i: usize| Thumb {
                        name: &names[i],
                        edited: false,
                        status: None,
                        selected: false,
                        picture: None,
                        rating: None,
                    };
                    *shown = GridPanel::new(count, active, thumb).show(ui).shown;
                },
                0..0,
            );
        harness.run();
        harness.run();
        harness.state().clone()
    };
    // 700 points, less the grid bar's column, hold four cells of 128 and part of a fifth.
    assert_eq!(shown(40, Some(0)), 0..5);
    // Scrolled to the last, the cells at the end.
    assert_eq!(shown(40, Some(39)).end, 40);
    assert!(shown(40, Some(39)).start > 30);
    // Fewer than fit, and none.
    assert_eq!(shown(3, None), 0..3);
    assert_eq!(shown(0, None), 0..0);
}

#[test]
fn an_empty_panel_says_why_and_a_filled_one_has_no_such_text() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 200.0))
        .with_context_setup(install)
        .build_ui(|ui| {
            let none = |_: usize| unreachable!("no photo to ask for");
            GridPanel::new(0, None, none)
                .empty("No photos available")
                .show(ui);
        });
    harness.run();
    assert!(
        painted_text(&harness)
            .iter()
            .any(|text| text == "No photos available")
    );
    harness.get_by_role_and_label(Role::List, "Grid panel");
}

fn painted_text(harness: &Harness<'_, ()>) -> Vec<String> {
    painted(harness)
}

#[test]
fn the_cell_s_states_are_photographed_in_the_light_and_dark_themes() {
    let mut results = egui_kittest::SnapshotResults::new();
    for (which, name) in [
        (Builtin::Light, "grid-cell-states-light"),
        (Builtin::Dark, "grid-cell-states-dark"),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(700.0, 200.0))
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(which));
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui(|ui| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.surface;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.spacing_mut().item_spacing.x = 10.0;
                    let rating = |stars, rejected, labels: &[ColourLabel]| {
                        Some(CellRating {
                            stars,
                            rejected,
                            labels: ColourLabels::of(labels.iter().copied()),
                        })
                    };
                    GridCell::new("IMG_2041.ARW")
                        .rating(rating(3, false, &[ColourLabel::Red, ColourLabel::Yellow]))
                        .show(ui);
                    GridCell::new("IMG_2042.ARW")
                        .selected(true)
                        .rating(rating(0, false, &[]))
                        .show(ui);
                    GridCell::new("IMG_2043.ARW")
                        .active(true)
                        .selected(true)
                        .edited(true)
                        .rating(rating(5, false, &ColourLabel::ALL))
                        .show(ui);
                    GridCell::new("IMG_2044.CR3")
                        .status(Some(CellStatus::ViewOnly))
                        .rating(rating(2, true, &[ColourLabel::Blue]))
                        .show(ui);
                    GridCell::new("IMG_2045.NEF")
                        .status(Some(CellStatus::Unreadable("not an Apertrust edit")))
                        .show(ui);
                });
            });
        harness.run();
        harness.snapshot(name);
        results.extend(harness.take_snapshot_results());
    }
}
