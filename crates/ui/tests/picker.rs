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

//! The picker button and the active picker's marks (`docs/pickers.md`, The picker button
//! and On the photo): the button driven by the pointer, the keys and a screen reader, each
//! press reported as darktable's button reads it, and the gallery's Pickers section
//! photographed at twice the size in the dark and the light theme. Accepted with
//! `UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test picker`, reviewed like a golden
//! image.

use apertrust_ui::components::{PickerAction, PickerButton};
use apertrust_ui::gallery::Gallery;
use apertrust_ui::{Builtin, Locale, Theme, Translator};
use egui::accesskit::{Role, Toggled};
use egui::os::OperatingSystem;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const NAME: &str = "Exposure: pick from the photo";

/// A picker button and the presses it reported, applied as the app would: `On` turns it
/// on, `Off` off.
struct Picker {
    on: bool,
    both: bool,
    quad: bool,
    disabled: Option<&'static str>,
    pressed: Vec<PickerAction>,
}

fn picker(on: bool, both: bool, disabled: Option<&'static str>) -> Harness<'static, Picker> {
    built(
        OperatingSystem::Nix,
        Picker {
            on,
            both,
            quad: false,
            disabled,
            pressed: Vec::new(),
        },
    )
}

/// A quad's picker that takes a point or an area, on `os`.
fn quad(os: OperatingSystem, on: bool) -> Harness<'static, Picker> {
    built(
        os,
        Picker {
            on,
            both: true,
            quad: true,
            disabled: None,
            pressed: Vec::new(),
        },
    )
}

fn built(os: OperatingSystem, picker: Picker) -> Harness<'static, Picker> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(120.0, 40.0))
        .with_os(os)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            |ui, state: &mut Picker| {
                let mut button = PickerButton::new(NAME, state.on)
                    .point_or_area(state.both)
                    .quad(state.quad);
                if let Some(why) = state.disabled {
                    button = button.disabled(why);
                }
                let (_, action) = button.show(ui);
                if let Some(action) = action {
                    state.on = matches!(action, PickerAction::On { .. });
                    state.pressed.push(action);
                }
            },
            picker,
        );
    harness.run();
    harness
}

fn pressed(harness: &mut Harness<'_, Picker>) -> Vec<PickerAction> {
    harness.run();
    std::mem::take(&mut harness.state_mut().pressed)
}

#[test]
fn a_click_turns_it_on_as_a_point_and_another_turns_it_off() {
    let mut harness = picker(false, true, None);
    harness.get_by_role_and_label(Role::Button, NAME).click();
    assert_eq!(
        pressed(&mut harness),
        vec![PickerAction::On { area: false }]
    );
    harness.get_by_role_and_label(Role::Button, NAME).click();
    assert_eq!(pressed(&mut harness), vec![PickerAction::Off]);
}

#[test]
fn a_right_click_or_a_ctrl_click_turns_it_on_as_an_area() {
    let mut harness = picker(false, true, None);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_secondary();
    assert_eq!(pressed(&mut harness), vec![PickerAction::On { area: true }]);

    let mut harness = picker(false, true, None);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_modifiers(Modifiers::COMMAND);
    assert_eq!(pressed(&mut harness), vec![PickerAction::On { area: true }]);
}

/// Ctrl alone picks an area, as darktable's `dt_modifier_is` compares the whole mask: on
/// the Mac Cmd, or Control, the Mac's right-click; Ctrl+Shift is a plain click.
#[test]
fn only_ctrl_alone_picks_an_area_and_the_mac_s_cmd_or_control() {
    for (os, modifiers, area) in [
        (OperatingSystem::Mac, Modifiers::MAC_CMD, true),
        (OperatingSystem::Mac, Modifiers::CTRL, true),
        (
            OperatingSystem::Mac,
            Modifiers::CTRL | Modifiers::SHIFT,
            false,
        ),
        (
            OperatingSystem::Mac,
            Modifiers::MAC_CMD | Modifiers::ALT,
            false,
        ),
        (
            OperatingSystem::Nix,
            Modifiers::COMMAND | Modifiers::SHIFT,
            false,
        ),
    ] {
        let mut harness = built(
            os,
            Picker {
                on: false,
                both: true,
                quad: false,
                disabled: None,
                pressed: Vec::new(),
            },
        );
        harness
            .get_by_role_and_label(Role::Button, NAME)
            .click_modifiers(modifiers);
        assert_eq!(
            pressed(&mut harness),
            vec![PickerAction::On { area }],
            "{os:?} {modifiers:?}"
        );
    }
}

/// darktable's quad takes every mouse button alike (`color_picker_proxy.c:259-263`): a
/// right-click is a click, Ctrl alone picks an area, and Shift+F10 stays the keyboard's way
/// to an area.
#[test]
fn on_a_quad_a_right_click_is_a_click_and_only_ctrl_picks_an_area() {
    let mut harness = quad(OperatingSystem::Nix, false);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_secondary();
    assert_eq!(
        pressed(&mut harness),
        vec![PickerAction::On { area: false }]
    );
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_secondary();
    assert_eq!(pressed(&mut harness), vec![PickerAction::Off]);

    let mut harness = quad(OperatingSystem::Nix, false);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_button_modifiers(egui::PointerButton::Secondary, Modifiers::COMMAND);
    assert_eq!(pressed(&mut harness), vec![PickerAction::On { area: true }]);

    let mut harness = quad(OperatingSystem::Mac, false);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_modifiers(Modifiers::MAC_CMD);
    assert_eq!(pressed(&mut harness), vec![PickerAction::On { area: true }]);

    let mut harness = quad(OperatingSystem::Nix, false);
    harness.get_by_role_and_label(Role::Button, NAME).focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::F10);
    assert_eq!(pressed(&mut harness), vec![PickerAction::On { area: true }]);

    let harness = quad(OperatingSystem::Nix, false);
    let description = harness
        .get_by_role_and_label(Role::Button, NAME)
        .accesskit_node()
        .description();
    assert_eq!(
        description.as_deref(),
        Some("Pick colour from the photo\nCtrl+click to select an area")
    );
}

/// darktable's press on the active button turns it off whatever the button or the keys
/// (`color_picker_proxy.c:237-252`).
#[test]
fn any_press_while_on_turns_it_off() {
    let mut harness = picker(true, true, None);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_secondary();
    assert_eq!(pressed(&mut harness), vec![PickerAction::Off]);

    let mut harness = picker(true, true, None);
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_modifiers(Modifiers::COMMAND);
    assert_eq!(pressed(&mut harness), vec![PickerAction::Off]);
}

#[test]
fn space_and_enter_click_it_and_shift_f10_or_the_menu_key_right_click_it() {
    for (modifiers, key, area) in [
        (Modifiers::NONE, Key::Space, false),
        (Modifiers::NONE, Key::Enter, false),
        (Modifiers::SHIFT, Key::F10, true),
        (Modifiers::NONE, Key::Menu, true),
    ] {
        let mut harness = picker(false, true, None);
        harness.get_by_role_and_label(Role::Button, NAME).focus();
        harness.run();
        harness.key_press_modifiers(modifiers, key);
        assert_eq!(
            pressed(&mut harness),
            vec![PickerAction::On { area }],
            "{modifiers:?} {key:?}"
        );
        harness.key_press_modifiers(modifiers, key);
        assert_eq!(
            pressed(&mut harness),
            vec![PickerAction::Off],
            "{modifiers:?} {key:?}"
        );
    }
}

#[test]
fn a_screen_reader_hears_a_toggle_button_pressed_while_on_and_its_tooltip() {
    let mut harness = picker(false, true, None);
    let toggled = |harness: &Harness<'_, Picker>| {
        harness
            .get_by_role_and_label(Role::Button, NAME)
            .accesskit_node()
            .data()
            .toggled()
    };
    assert_eq!(toggled(&harness), Some(Toggled::False));
    let tip = "Pick colour from the photo\nCtrl+click or right-click to select an area";
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, NAME)
            .accesskit_node()
            .description()
            .as_deref(),
        Some(tip)
    );
    harness
        .get_by_role_and_label(Role::Button, NAME)
        .click_accesskit();
    assert_eq!(
        pressed(&mut harness),
        vec![PickerAction::On { area: false }]
    );
    harness.run();
    assert_eq!(toggled(&harness), Some(Toggled::True));
}

#[test]
fn a_picker_of_one_kind_has_its_words_alone_and_the_mac_says_cmd() {
    let harness = picker(false, false, None);
    let description = harness
        .get_by_role_and_label(Role::Button, NAME)
        .accesskit_node()
        .description();
    assert_eq!(description.as_deref(), Some("Pick colour from the photo"));

    let mut harness = Harness::builder()
        .with_size(egui::vec2(120.0, 40.0))
        .with_os(OperatingSystem::Mac)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui(|ui| {
            PickerButton::new(NAME, false)
                .tip("Set the exposure adjustment using the selected area")
                .point_or_area(true)
                .show(ui);
        });
    harness.run();
    let description = harness
        .get_by_role_and_label(Role::Button, NAME)
        .accesskit_node()
        .description();
    assert_eq!(
        description.as_deref(),
        Some(
            "Set the exposure adjustment using the selected area\n\
             Cmd+click or right-click to select an area"
        )
    );
}

#[test]
fn a_disabled_button_ignores_presses_and_says_why() {
    let why = "Opening IMG_2041…";
    let mut harness = picker(false, true, Some(why));
    let node = harness.get_by_role_and_label(Role::Button, NAME);
    assert!(node.accesskit_node().is_disabled());
    assert_eq!(node.accesskit_node().description().as_deref(), Some(why));
    node.click();
    node.click_secondary();
    assert_eq!(pressed(&mut harness), Vec::new());
}

/// The tooltip's words in every language: the area's sentence follows the picker's own.
/// The pseudo-locale, which brackets the whole message, is left out.
#[test]
fn the_tooltip_s_words_are_translated() {
    for locale in [Locale::EnGb, Locale::EnUs, Locale::Uk, Locale::Pl] {
        let words = Translator::new(locale);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(120.0, 40.0))
            .with_os(OperatingSystem::Nix)
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
                apertrust_ui::set_locale(ctx, locale);
            })
            .build_ui(|ui| {
                PickerButton::new(NAME, false).point_or_area(true).show(ui);
            });
        harness.run();
        let description = harness
            .get_by_role_and_label(Role::Button, NAME)
            .accesskit_node()
            .description()
            .unwrap_or_default();
        assert!(
            description.starts_with(&words.text("picker-tip")),
            "{}: {description}",
            locale.id()
        );
        assert!(description.contains("Ctrl+"), "{}", locale.id());
    }
}

fn photograph(which: Builtin, name: &str) {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 680.0))
        .with_pixels_per_point(2.0)
        .with_context_setup(move |ctx| apertrust_ui::install(ctx, Theme::builtin(which)))
        .build_ui_state(
            |ui, gallery: &mut Gallery| {
                let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, panel);
                egui::Frame::new().inner_margin(12.0).show(ui, |ui| {
                    gallery.show_pickers(ui);
                });
            },
            Gallery::new(which),
        );
    harness.run();
    harness.snapshot(name);
}

#[test]
fn the_pickers_in_the_dark_theme() {
    photograph(Builtin::Dark, "picker-dark");
}

#[test]
fn the_pickers_in_the_light_theme() {
    photograph(Builtin::Light, "picker-light");
}
