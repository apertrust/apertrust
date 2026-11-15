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

//! A section header's tooltip (`docs/gui.md`, The header's tooltip): when it shows and
//! goes, where it sits, what screen readers hear, and how it looks in each theme, with a
//! name the header cuts and in a narrow window. The images are in `tests/snapshots`,
//! accepted with `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test about`.

use apertrust_ui::components::{About, Place, Section};
use apertrust_ui::{Builtin, Theme, Translator};
use egui::accesskit::Role;
use egui::epaint::TextShape;
use egui::{Key, Rect, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// A tool's tooltip from its messages, as the app builds it from the tool's definition:
/// its sentence, its kind, what it works on and darktable's lines.
fn tool(tr: &Translator, id: &str, kind: &str, works: &str, facts: [&str; 3]) -> About {
    About {
        description: tr.text(&format!("tool-{id}-description")),
        kind: Some(tr.text(&format!("tool-kind-{kind}"))),
        place: Some(Place::WorksOn(tr.text(&format!("tool-works-{works}")))),
        facts: Some(facts.map(|fact| tr.text(&format!("tool-fact-{fact}")))),
    }
}

fn scene_tool(tr: &Translator, id: &str, kind: &str, process: &str, output: &str) -> About {
    tool(
        tr,
        id,
        kind,
        "scene",
        ["linear-rgb-scene-referred", process, output],
    )
}

/// Every header the scenes draw, by its tool's id, with its tooltip.
fn headers(tr: &Translator) -> Vec<(String, About)> {
    let raw = |id: &str, process: &str, output: &str| {
        tool(
            tr,
            id,
            "corrective",
            "raw",
            ["linear-raw-scene-referred", process, output],
        )
    };
    let about = |id: &str| match id {
        "toneequal" => scene_tool(
            tr,
            id,
            "corrective-and-creative",
            "quasi-linear-rgb",
            "quasi-linear-rgb-scene-referred",
        ),
        "exposure" => scene_tool(
            tr,
            id,
            "corrective-and-creative",
            "linear-rgb",
            "linear-rgb-scene-referred",
        ),
        "flip" => scene_tool(
            tr,
            id,
            "corrective",
            "geometric-rgb",
            "linear-rgb-scene-referred",
        ),
        "hazeremoval" => scene_tool(
            tr,
            id,
            "corrective",
            "frequential-rgb",
            "linear-rgb-scene-referred",
        ),
        "channelmixerrgb" => scene_tool(
            tr,
            id,
            "corrective-or-creative",
            "linear-rgb-or-xyz",
            "linear-rgb-scene-referred",
        ),
        "primaries" => scene_tool(
            tr,
            id,
            "corrective-or-creative",
            "linear-rgb",
            "linear-rgb-scene-referred",
        ),
        "highlights" => raw(id, "reconstruction-raw", "linear-raw-scene-referred"),
        "temperature" => raw(id, "linear-raw", "linear-raw-scene-referred"),
        "rawprepare" => tool(
            tr,
            id,
            "mandatory",
            "raw",
            [
                "linear-raw-scene-referred",
                "linear-raw",
                "linear-raw-scene-referred",
            ],
        ),
        "scopes" => About {
            description: tr.text("tool-scopes-description"),
            kind: Some(tr.text("tool-kind-utility")),
            place: Some(Place::Measures(tr.text("tool-works-display"))),
            facts: None,
        },
        other => panic!("no tooltip for {other}"),
    };
    [
        "toneequal",
        "exposure",
        "flip",
        "hazeremoval",
        "channelmixerrgb",
        "primaries",
        "highlights",
        "temperature",
        "rawprepare",
        "scopes",
    ]
    .into_iter()
    .map(|id| (id.to_owned(), about(id)))
    .collect()
}

/// What the scene's sections reported, and what it draws: the window's size, the panel's
/// width and which side it is on, and the headers, by id.
struct Scene {
    panel: f32,
    start: bool,
    ids: Vec<&'static str>,
    peek: bool,
}

/// A window of `size` with a side panel `panel` points wide at its end (or its start), its
/// sections the tools `ids`, closed, each with its tooltip, over the photo's surround.
fn scene(
    which: Builtin,
    size: egui::Vec2,
    panel: f32,
    start: bool,
    ids: &[&'static str],
) -> Harness<'static, Scene> {
    Harness::builder()
        .with_size(size)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            |ui, scene: &mut Scene| {
                let t = apertrust_ui::theme(ui.ctx());
                let tr = apertrust_ui::translator(ui.ctx());
                let screen = ui.ctx().content_rect();
                ui.painter()
                    .rect_filled(screen, 0.0, t.colors.photo_surround);
                let rect = if scene.start {
                    Rect::from_min_max(
                        screen.min,
                        pos2(screen.left() + scene.panel, screen.bottom()),
                    )
                } else {
                    Rect::from_min_max(pos2(screen.right() - scene.panel, screen.top()), screen.max)
                };
                ui.painter().rect_filled(rect, 0.0, t.colors.panel);
                let edge = if scene.start {
                    rect.right()
                } else {
                    rect.left()
                };
                ui.painter().vline(
                    edge,
                    rect.y_range(),
                    egui::Stroke::new(t.shape.hairline, t.colors.separator),
                );
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                child.spacing_mut().item_spacing.y = 0.0;
                let all = headers(&tr);
                for id in &scene.ids {
                    let about = all
                        .iter()
                        .find(|(tool, _)| tool == id)
                        .map(|(_, about)| about.clone())
                        .expect("a header the scene knows");
                    let title = if *id == "scopes" {
                        tr.text("section-scopes")
                    } else {
                        tr.text(&format!("tool-{id}-title"))
                    };
                    let mut section = Section::new(&title, false).about(about).peek(scene.peek);
                    if *id != "scopes" {
                        section = section.edits();
                    }
                    section.show(&mut child, |_| {});
                }
            },
            Scene {
                panel,
                start,
                ids: ids.to_vec(),
                peek: false,
            },
        )
}

/// Tone equalizer, Exposure and Orientation at a 320-point panel's end in a 900-point
/// window, drawn once.
fn three() -> Harness<'static, Scene> {
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &["toneequal", "exposure", "flip"],
    );
    harness.run();
    harness
}

/// The text the tooltip drew whose words are `text`, if it drew it this frame.
fn drawn<State>(harness: &Harness<'_, State>, text: &str) -> Option<TextShape> {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape.clone()),
            _ => None,
        })
}

fn header<State>(harness: &Harness<'_, State>, name: &str) -> Rect {
    harness.get_by_role_and_label(Role::Button, name).rect()
}

fn english() -> Translator {
    Translator::new(apertrust_ui::Locale::EnGb)
}

/// Hovers the header `name` at its title's start and lets the pointer rest `seconds`, in
/// the harness's quarter-second steps.
fn rest_on<State>(harness: &mut Harness<'_, State>, name: &str, seconds: f32) {
    let rect = header(harness, name);
    harness.hover_at(pos2(rect.left() + 40.0, rect.center().y));
    harness.run();
    harness.run_steps((seconds / 0.25).round() as usize);
}

#[test]
fn it_shows_after_the_pointer_rests_and_never_over_the_header_s_buttons() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &["toneequal", "exposure", "flip"],
    );
    harness.run();
    rest_on(&mut harness, "Exposure", 0.25);
    assert!(
        drawn(&harness, &description).is_none(),
        "shown before 0.6 s"
    );
    harness.run_steps(2);
    assert!(drawn(&harness, &description).is_some(), "not shown at rest");

    // Over the header's reset button: the button's own tooltip, not this.
    harness.event(egui::Event::PointerGone);
    harness.run_steps(4);
    let reset = harness.get_by_label("Reset Exposure").rect();
    harness.hover_at(reset.center());
    harness.run_steps(4);
    assert!(
        drawn(&harness, &description).is_none(),
        "shown over a button"
    );
}

#[test]
fn a_sweep_down_the_headers_shows_each_at_once() {
    let tr = english();
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &["toneequal", "exposure", "flip"],
    );
    harness.run();
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(drawn(&harness, &tr.text("tool-exposure-description")).is_some());
    let next = header(&harness, "Orientation");
    harness.hover_at(pos2(next.left() + 40.0, next.center().y));
    harness.run();
    assert!(
        drawn(&harness, &tr.text("tool-flip-description")).is_some(),
        "the next header's waits"
    );
    assert!(drawn(&harness, &tr.text("tool-exposure-description")).is_none());
}

/// The pointer leaving for the photo takes it away at once, with no frame left waiting.
#[test]
fn it_goes_as_the_pointer_leaves_the_header() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &["toneequal", "exposure", "flip"],
    );
    harness.run();
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(drawn(&harness, &description).is_some());
    harness.hover_at(pos2(100.0, 250.0));
    harness.run();
    assert!(
        drawn(&harness, &description).is_none(),
        "left over the photo"
    );
}

#[test]
fn a_click_hides_it_until_the_pointer_leaves_and_comes_back() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = three();
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(drawn(&harness, &description).is_some());
    let at = header(&harness, "Exposure");
    let at = pos2(at.left() + 40.0, at.center().y);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        });
        harness.run();
        assert!(drawn(&harness, &description).is_none(), "a click left it");
    }
    harness.run_steps(6);
    assert!(
        drawn(&harness, &description).is_none(),
        "back before leaving"
    );
    harness.hover_at(pos2(100.0, 250.0));
    harness.run_steps(2);
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(
        drawn(&harness, &description).is_some(),
        "not back after leaving and returning"
    );
}

#[test]
fn holding_the_peek_keys_hides_it() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &["toneequal", "exposure", "flip"],
    );
    harness.run();
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(drawn(&harness, &description).is_some());
    harness.state_mut().peek = true;
    harness.run();
    assert!(drawn(&harness, &description).is_none());
}

#[test]
fn a_header_focused_by_a_key_shows_it_and_a_key_hides_it_until_the_focus_moves() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = three();
    harness.key_press(Key::Tab);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Exposure")
        .focus();
    harness.run();
    assert!(drawn(&harness, &description).is_none());
    harness.run_steps(3);
    assert!(
        drawn(&harness, &description).is_some(),
        "not shown on focus"
    );
    // A key that leaves the focus where it is.
    harness.key_press(Key::A);
    harness.run_steps(4);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Exposure")
            .accesskit_node()
            .is_focused(),
        "the key moved the focus"
    );
    assert!(drawn(&harness, &description).is_none(), "a key left it");
    // The focus moves on: the next header's shows.
    harness
        .get_by_role_and_label(Role::Button, "Orientation")
        .focus();
    harness.run();
    harness.run_steps(3);
    assert!(drawn(&harness, &tr.text("tool-flip-description")).is_some());
}

/// Whether the header `name` has the keyboard's focus.
fn focused<State>(harness: &Harness<'_, State>, name: &str) -> bool {
    harness
        .get_by_role_and_label(Role::Button, name)
        .accesskit_node()
        .is_focused()
}

/// Presses `key` with `modifiers` until the header `name` has the focus, as a keyboard
/// user moves through the panel, and nothing after.
fn tab_to<State>(harness: &mut Harness<'_, State>, name: &str, modifiers: egui::Modifiers) {
    for _ in 0..30 {
        harness.key_press_modifiers(modifiers, Key::Tab);
        harness.run();
        if focused(harness, name) {
            return;
        }
    }
    panic!("Tab never reached {name}");
}

/// A header reached by Tab, or Shift+Tab, alone shows its tooltip after the delay: the
/// key that brought the focus is no key that hides it.
#[test]
fn a_header_reached_by_tab_or_shift_tab_shows_it() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = three();
    tab_to(&mut harness, "Exposure", egui::Modifiers::NONE);
    harness.run_steps(3);
    assert!(
        drawn(&harness, &description).is_some(),
        "not shown after Tab"
    );

    tab_to(&mut harness, "Orientation", egui::Modifiers::NONE);
    tab_to(&mut harness, "Exposure", egui::Modifiers::SHIFT);
    assert!(drawn(&harness, &description).is_none(), "shown at once");
    harness.run_steps(3);
    assert!(
        drawn(&harness, &description).is_some(),
        "not shown after Shift+Tab"
    );
}

/// A key in the frame the pointer leaves the header it shows for hides the tooltip of the
/// header with the keyboard's focus, which takes the target then: the key that brings the
/// focus is the only one let through. Esc would clear egui's focus itself, so a letter.
#[test]
fn a_key_as_the_pointer_leaves_hides_the_focused_header_s() {
    let tr = english();
    let mut harness = three();
    tab_to(&mut harness, "Exposure", egui::Modifiers::NONE);
    rest_on(&mut harness, "Orientation", 1.0);
    assert!(drawn(&harness, &tr.text("tool-flip-description")).is_some());
    assert!(focused(&harness, "Exposure"));
    // Both in one frame, which the harness's own queue would split into two.
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(pos2(100.0, 250.0)));
    for pressed in [true, false] {
        harness.input_mut().events.push(egui::Event::Key {
            key: Key::A,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.step();
    harness.run();
    harness.run_steps(6);
    assert!(focused(&harness, "Exposure"), "the key moved the focus");
    assert!(
        drawn(&harness, &tr.text("tool-exposure-description")).is_none(),
        "the focused header's shows after a key"
    );
}

/// Esc hides it under the pointer too, where the focus has nothing to do with it, until
/// the pointer leaves.
#[test]
fn esc_hides_it_under_the_pointer() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = three();
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(drawn(&harness, &description).is_some());
    harness.key_press(Key::Escape);
    harness.run_steps(4);
    assert!(drawn(&harness, &description).is_none(), "Esc left it");
}

#[test]
fn a_scroll_hides_it() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    let mut harness = three();
    rest_on(&mut harness, "Exposure", 1.0);
    assert!(drawn(&harness, &description).is_some());
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, -20.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Default::default(),
    });
    harness.run_steps(4);
    assert!(drawn(&harness, &description).is_none(), "a scroll left it");
}

/// The rectangle the tooltip whose description is `text` fills: the frame's, found by its
/// fill round the description.
fn frame_of<State>(harness: &Harness<'_, State>, text: &str) -> Rect {
    let at = drawn(harness, text).expect("shown").pos;
    let raised = apertrust_ui::theme(&harness.ctx).colors.raised;
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) if rect.fill == raised && rect.rect.contains(at) => {
                Some(rect.rect)
            }
            _ => None,
        })
        .expect("its frame")
}

/// Beside the panel, a header near the window's foot moves the tooltip up to stay inside
/// the window, 8 points from its edge.
#[test]
fn beside_the_panel_it_moves_up_to_stay_in_the_window() {
    let tr = english();
    let description = tr.text("tool-temperature-description");
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &[
            "toneequal",
            "exposure",
            "flip",
            "hazeremoval",
            "channelmixerrgb",
            "primaries",
            "highlights",
            "temperature",
        ],
    );
    harness.run();
    rest_on(&mut harness, "White balance", 1.0);
    let frame = frame_of(&harness, &description);
    let at = header(&harness, "White balance");
    assert!(frame.top() < at.top(), "{frame:?} not moved up from {at:?}");
    assert!((frame.bottom() - (300.0 - 8.0)).abs() < 1.0, "{frame:?}");
}

/// In a narrow window, a header near the window's foot puts the tooltip above it.
#[test]
fn in_a_narrow_window_it_flips_above_a_header_near_the_foot() {
    let tr = english();
    let description = tr.text("tool-temperature-description");
    let mut harness = scene(
        Builtin::Dark,
        vec2(500.0, 360.0),
        320.0,
        false,
        &[
            "toneequal",
            "exposure",
            "flip",
            "hazeremoval",
            "channelmixerrgb",
            "primaries",
            "highlights",
            "temperature",
        ],
    );
    harness.run();
    rest_on(&mut harness, "White balance", 1.0);
    let frame = frame_of(&harness, &description);
    let at = header(&harness, "White balance");
    assert!(
        (frame.bottom() - (at.top() - 4.0)).abs() < 1.0,
        "{frame:?} not above {at:?}"
    );
    assert!((frame.left() - (180.0 + 8.0)).abs() < 1.0, "{frame:?}");
}

#[test]
fn screen_readers_hear_its_words_without_the_name() {
    let tr = english();
    let mut harness = scene(
        Builtin::Dark,
        vec2(900.0, 300.0),
        320.0,
        false,
        &["exposure", "scopes"],
    );
    harness.run();
    let said = |name: &str| {
        harness
            .get_by_role_and_label(Role::Button, name)
            .accesskit_node()
            .description()
            .unwrap_or_default()
    };
    assert_eq!(
        said("Exposure"),
        [
            tr.text("tool-exposure-description").as_str(),
            "Kind: Fixes a problem and shapes the look",
            "Works on: The light as captured",
            "Input: linear, RGB, scene-referred",
            "Process: linear, RGB",
            "Output: linear, RGB, scene-referred",
        ]
        .join("\n")
    );
    // A utility: what it measures, and none of darktable's lines.
    assert_eq!(
        said("Scopes"),
        [
            tr.text("tool-scopes-description").as_str(),
            "Kind: Edits nothing of its own",
            "Measures: The finished picture",
        ]
        .join("\n")
    );
}

#[test]
fn beside_the_panel_its_top_is_level_with_the_header_and_mirrored_on_the_start() {
    let tr = english();
    let description = tr.text("tool-exposure-description");
    for start in [false, true] {
        let mut harness = scene(
            Builtin::Dark,
            vec2(900.0, 300.0),
            320.0,
            start,
            &["toneequal", "exposure", "flip"],
        );
        harness.run();
        rest_on(&mut harness, "Exposure", 1.0);
        let text = drawn(&harness, &description).expect("shown");
        let at = header(&harness, "Exposure");
        // 8 points of padding at its top, 10 at its sides.
        assert!(
            (text.pos.y - 8.0 - at.top()).abs() < 1.0,
            "{start}: {:?}",
            text.pos
        );
        if start {
            assert!(
                (text.pos.x - 10.0 - (320.0 + 8.0)).abs() < 1.0,
                "{:?}",
                text.pos
            );
        } else {
            let right = text.pos.x - 10.0 + 340.0;
            assert!((right - (900.0 - 320.0 - 8.0)).abs() < 1.0, "{right}");
        }
    }
}

#[test]
fn in_a_narrow_window_it_hangs_under_the_header_inside_the_panel() {
    let tr = english();
    let description = tr.text("tool-hazeremoval-description");
    let mut harness = scene(
        Builtin::Dark,
        vec2(500.0, 360.0),
        320.0,
        false,
        &["primaries", "hazeremoval", "flip"],
    );
    harness.run();
    rest_on(&mut harness, "Haze removal", 1.0);
    let text = drawn(&harness, &description).expect("shown");
    let at = header(&harness, "Haze removal");
    assert!(
        (text.pos.x - 10.0 - (180.0 + 8.0)).abs() < 1.0,
        "{:?}",
        text.pos
    );
    assert!(
        text.pos.y - 8.0 >= at.bottom(),
        "{:?} over {at:?}",
        text.pos
    );
    harness.snapshot("section-about-narrow");
}

#[test]
fn the_tooltip_in_the_dark_theme() {
    let mut harness = scene(
        Builtin::Dark,
        vec2(720.0, 300.0),
        280.0,
        false,
        &["toneequal", "exposure", "flip", "highlights"],
    );
    harness.run();
    rest_on(&mut harness, "Exposure", 1.0);
    harness.snapshot("section-about-dark");
}

#[test]
fn the_tooltip_in_the_light_theme() {
    let mut harness = scene(
        Builtin::Light,
        vec2(720.0, 300.0),
        280.0,
        false,
        &["primaries", "channelmixerrgb", "exposure"],
    );
    harness.run();
    rest_on(&mut harness, "Colour calibration", 1.0);
    harness.snapshot("section-about-light");
}

#[test]
fn the_tooltip_in_the_high_contrast_theme() {
    let mut harness = scene(
        Builtin::HighContrast,
        vec2(720.0, 300.0),
        280.0,
        false,
        &["highlights", "temperature", "rawprepare"],
    );
    harness.run();
    rest_on(&mut harness, "White balance", 1.0);
    harness.snapshot("section-about-high-contrast");
}

/// At the side panel's narrowest, 240 points, the header cuts Highlight reconstruction's
/// name, so the tooltip starts with it whole, here with the header focused by a key.
#[test]
fn a_name_the_header_cuts_comes_first_whole() {
    let mut harness = scene(
        Builtin::Dark,
        vec2(680.0, 300.0),
        240.0,
        false,
        &["temperature", "highlights", "rawprepare"],
    );
    harness.run();
    harness.key_press(Key::Tab);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Highlight reconstruction")
        .focus();
    harness.run();
    harness.run_steps(3);
    let whole = harness.output().shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(text)
            if text.galley.text() == "Highlight reconstruction" && !text.galley.elided)
    });
    assert!(whole, "the name is not drawn whole");
    harness.snapshot("section-about-cut-name");
}
