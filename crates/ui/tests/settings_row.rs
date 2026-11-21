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

//! The Settings dialog's row (`docs/gui.md`, The Settings dialog): its changed mark and the
//! resets, the dimmed row and the restart note, driven by the pointer and the keys and read
//! through AccessKit.

use apertrust_ui::components::settings::{Combo, SettingRow};
use apertrust_ui::{Theme, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const THEMES: [&str; 2] = ["Dark", "Light"];

/// A Theme row: off its default where `changed`, dimmed where `disabled`, its setting
/// applying after a restart where `after`; and how many resets it reported.
struct Row {
    changed: bool,
    disabled: bool,
    after: bool,
    resets: usize,
}

fn row(changed: bool, disabled: bool, after: bool) -> Harness<'static, Row> {
    let mut harness = Harness::builder()
        .with_size(vec2(640.0, 60.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| {
            install(ctx, Theme::default());
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, state: &mut Row| {
                let mut row = SettingRow::new("theme", "Theme")
                    .help("The interface's colours")
                    .after_restart(state.after);
                if state.changed {
                    row = row.changed("Dark");
                }
                if state.disabled {
                    row = row.disabled("Set by --conf for this run");
                }
                let selected = Some(usize::from(state.changed));
                let out = row.show(ui, |ui| Combo::new("Theme", &THEMES, selected).show(ui));
                if out.reset {
                    state.resets += 1;
                    state.changed = false;
                }
            },
            Row {
                changed,
                disabled,
                after,
                resets: 0,
            },
        );
    harness.run();
    harness
}

const MARK: &str = "Reset to Dark";

fn press(harness: &mut Harness<'_, Row>, pos: Pos2, button: PointerButton) {
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

fn label_centre(harness: &Harness<'_, Row>) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Label, "Theme")
        .rect()
        .center()
}

#[test]
fn the_mark_shows_only_off_the_default_and_a_click_on_it_resets() {
    let harness = row(false, false, false);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, MARK)
            .is_none()
    );
    let mut harness = row(true, false, false);
    let label = harness.get_by_role_and_label(Role::Label, "Theme");
    let described = label.accesskit_node().description();
    assert_eq!(described.as_deref(), Some("Changed from its default"));
    // After the label, on its line.
    let mark = harness.get_by_role_and_label(Role::Button, MARK).rect();
    assert!(mark.left() >= label.rect().right());
    assert!((mark.center().y - label.rect().center().y).abs() < 1.0);
    // Its tooltip names the default.
    harness.hover_at(mark.center());
    harness.run_steps(5);
    let tips = harness.query_all_by_label(MARK).count();
    assert!(tips >= 2, "the button and its tooltip: {tips}");
    press(&mut harness, mark.center(), PointerButton::Primary);
    assert_eq!(harness.state().resets, 1);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, MARK)
            .is_none()
    );
}

#[test]
fn a_double_click_delete_or_the_menu_on_the_label_resets() {
    let mut harness = row(true, false, false);
    let at = label_centre(&harness);
    harness.hover_at(at);
    for _ in 0..2 {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        harness.run();
    }
    assert_eq!(harness.state().resets, 1);
    harness.state_mut().changed = true;
    harness.run();
    harness.get_by_role_and_label(Role::Label, "Theme").focus();
    harness.run();
    harness.key_press(Key::Delete);
    harness.run();
    assert_eq!(harness.state().resets, 2);
    harness.state_mut().changed = true;
    harness.run_steps(30);
    press(&mut harness, at, PointerButton::Secondary);
    harness.get_by_label("Reset").click();
    harness.run();
    assert_eq!(harness.state().resets, 3);
}

#[test]
fn a_dimmed_row_has_no_mark_says_why_and_resets_nothing() {
    let mut harness = row(true, true, false);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, MARK)
            .is_none()
    );
    let label = harness.get_by_role_and_label(Role::Label, "Theme");
    assert!(label.accesskit_node().is_disabled());
    // Nothing on it to do: it takes no focus.
    label.focus();
    harness.run();
    let label = harness.get_by_role_and_label(Role::Label, "Theme");
    assert!(!label.accesskit_node().is_focused());
    let label = harness.get_by_role_and_label(Role::Label, "Theme");
    let described = label.accesskit_node().description();
    assert_eq!(described.as_deref(), Some("Set by --conf for this run"));
    let combo = harness.get_by_role_and_label(Role::ComboBox, "Theme");
    assert!(combo.accesskit_node().is_disabled());
    // Its tooltip says why.
    let at = label_centre(&harness);
    harness.hover_at(at);
    harness.run_steps(5);
    assert!(
        harness
            .query_all_by_label("Set by --conf for this run")
            .count()
            >= 1
    );
    press(&mut harness, at, PointerButton::Primary);
    press(&mut harness, at, PointerButton::Primary);
    assert_eq!(harness.state().resets, 0);
}

#[test]
fn the_restart_note_follows_the_label_and_its_mark() {
    for changed in [false, true] {
        let harness = row(changed, false, true);
        let label = harness.get_by_role_and_label(Role::Label, "Theme").rect();
        let note = harness
            .get_by_role_and_label(Role::Label, "after a restart")
            .rect();
        let after = match harness.query_by_role_and_label(Role::Button, MARK) {
            Some(mark) => mark.rect().right(),
            None => label.right(),
        };
        assert!(note.left() >= after, "{changed}");
        assert!(
            (note.center().y - label.center().y).abs() < 1.5,
            "{changed}"
        );
        let combo = harness
            .get_by_role_and_label(Role::ComboBox, "Theme")
            .rect();
        assert!(note.right() < combo.left(), "{changed}");
    }
    let harness = row(false, false, false);
    assert!(harness.query_by_label("after a restart").is_none());
}

/// A switch's row whose label wraps is its target down to the label's last line: a click
/// there is reported, for the caller to toggle the switch.
#[test]
fn a_whole_row_reaches_a_wrapped_label_s_last_line() {
    use apertrust_ui::components::settings::Switch;
    const LONG: &str = "Ask before overwriting an exported file when its name is already taken";
    let mut harness = Harness::builder()
        .with_size(vec2(420.0, 160.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, clicks: &mut usize| {
                let mut on = true;
                let out = SettingRow::new("long", LONG)
                    .whole_row(true)
                    .show(ui, |ui| Switch::new(&mut on, LONG).show(ui));
                if out.clicked {
                    *clicks += 1;
                }
            },
            0,
        );
    harness.run();
    let target = harness.get_by_role_and_label(Role::Label, LONG).rect();
    assert!(target.height() > 32.0, "the label wraps: {target:?}");
    let low = pos2(target.left() + 20.0, target.bottom() - 4.0);
    harness.run_steps(30);
    harness.hover_at(low);
    harness.run();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: low,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(*harness.state(), 1);
}

/// A switch's whole row as a settings page draws it: its value, its default off; and how
/// many toggles and resets it reported.
struct Toggle {
    on: bool,
    toggles: usize,
    resets: usize,
}

fn toggle_row(on: bool) -> Harness<'static, Toggle> {
    use apertrust_ui::components::settings::Switch;
    let mut harness = Harness::builder()
        .with_size(vec2(640.0, 60.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut Toggle| {
                let mut row = SettingRow::new("keep", "Keep a log").whole_row(true);
                if state.on {
                    row = row.changed("Off");
                }
                let mut on = state.on;
                let out = row.show(ui, |ui| Switch::new(&mut on, "Keep a log").show(ui));
                if out.reset {
                    state.resets += 1;
                    state.on = false;
                } else if out.clicked || on != state.on {
                    state.toggles += 1;
                    state.on = !state.on;
                }
            },
            Toggle {
                on,
                toggles: 0,
                resets: 0,
            },
        );
    harness.run();
    harness
}

fn clicks(harness: &mut Harness<'_, Toggle>, at: Pos2, count: usize) {
    harness.run_steps(30);
    harness.hover_at(at);
    harness.run();
    for _ in 0..count {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
    }
    harness.run();
}

fn toggle_label(harness: &Harness<'_, Toggle>) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Label, "Keep a log")
        .rect()
        .left_center()
        + vec2(20.0, 0.0)
}

/// Each click on a switch's row toggles it at once, as darktable's label toggles on every
/// press; the second of a double-click resets it instead, so a double-click leaves it at its
/// default, on or off it as it began; a third toggles again. A click on the switch and
/// Space on the focused label toggle it too. Each case: the value before, the quick clicks
/// on its words, then the toggles, resets and value after.
#[test]
fn a_whole_row_toggles_at_once_and_a_double_click_resets() {
    for (on, count, after) in [
        (false, 1, (1, 0, true)),
        (true, 1, (1, 0, false)),
        (false, 2, (1, 1, false)),
        (true, 2, (1, 1, false)),
        (false, 3, (2, 1, true)),
    ] {
        let mut harness = toggle_row(on);
        let at = toggle_label(&harness);
        harness.run_steps(30);
        harness.hover_at(at);
        harness.run();
        for _ in 0..count {
            for pressed in [true, false] {
                harness.event(Event::PointerButton {
                    pos: at,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                });
            }
            harness.step();
            harness.step();
        }
        // Nothing waits: the frame of the last click told it all.
        let state = harness.state();
        assert_eq!(
            (state.toggles, state.resets, state.on),
            after,
            "{on}, {count}"
        );
    }
    let mut harness = toggle_row(false);
    let switch = harness
        .get_by_role_and_label(Role::Switch, "Keep a log")
        .rect()
        .center();
    clicks(&mut harness, switch, 1);
    assert_eq!(harness.state().toggles, 1);
    harness
        .get_by_role_and_label(Role::Label, "Keep a log")
        .focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(harness.state().toggles, 2);
}

/// Two primary clicks on a switch's row 0.4 s apart, a middle click half way between: two
/// toggles, never a double-click's reset, though egui counts the second primary click quick
/// after the middle one.
#[test]
fn a_middle_click_between_two_slow_clicks_makes_no_double() {
    let mut harness = toggle_row(false);
    let at = toggle_label(&harness);
    harness.run_steps(30);
    harness.hover_at(at);
    harness.run();
    for button in [
        PointerButton::Primary,
        PointerButton::Middle,
        PointerButton::Primary,
    ] {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: at,
                button,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        // A fifth of a second at a 60th a frame.
        harness.run_steps(12);
    }
    let state = harness.state();
    assert_eq!((state.toggles, state.resets, state.on), (2, 0, false));
}

/// Quick clicks near enough for egui's double-click, at the bottom of the first of two
/// changed rows and the top of the second: a double-click is two on one row's label, the
/// first not the other row's. Each case: the rows clicked, in turn, then each row's value,
/// toggles and resets.
#[test]
fn a_double_click_is_two_clicks_on_one_row() {
    use apertrust_ui::components::settings::Switch;
    const NAMES: [&str; 2] = ["Keep a log", "Ask before overwriting"];
    type Rows = [(bool, usize, usize); 2];
    let cases: [(&[usize], Rows); 4] = [
        (&[0, 1], [(false, 1, 0), (false, 1, 0)]),
        (&[0, 1, 1], [(false, 1, 0), (false, 1, 1)]),
        (&[0, 0], [(false, 1, 1), (true, 0, 0)]),
        (&[1, 0, 0], [(false, 1, 1), (false, 1, 0)]),
    ];
    for (clicked, after) in cases {
        let mut harness = Harness::builder()
            .with_size(vec2(640.0, 100.0))
            .with_step_dt(1.0 / 60.0)
            .with_context_setup(|ctx| install(ctx, Theme::default()))
            .build_ui_state(
                |ui, rows: &mut Rows| {
                    for (name, (on, toggles, resets)) in NAMES.iter().zip(rows.iter_mut()) {
                        let mut row = SettingRow::new(name, name).whole_row(true);
                        if *on {
                            row = row.changed("Off");
                        }
                        let mut shown = *on;
                        let out = row.show(ui, |ui| Switch::new(&mut shown, name).show(ui));
                        if out.reset {
                            *resets += 1;
                            *on = false;
                        } else if out.clicked {
                            *toggles += 1;
                            *on = !*on;
                        }
                    }
                },
                [(true, 0, 0), (true, 0, 0)],
            );
        harness.run();
        let first = harness.get_by_role_and_label(Role::Label, NAMES[0]).rect();
        let second = harness.get_by_role_and_label(Role::Label, NAMES[1]).rect();
        let at = [
            pos2(first.left() + 20.0, first.bottom() - 0.5),
            pos2(first.left() + 20.0, second.top() + 0.5),
        ];
        assert!(at[0].distance(at[1]) < 6.0, "{at:?}");
        harness.run_steps(30);
        for &row in clicked {
            harness.hover_at(at[row]);
            for pressed in [true, false] {
                harness.event(Event::PointerButton {
                    pos: at[row],
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                });
            }
        }
        harness.run_steps(30);
        assert_eq!(*harness.state(), after, "{clicked:?}");
    }
}

/// A dimmed row says why over its control as well as over its label.
#[test]
fn a_dimmed_row_says_why_over_its_control() {
    let mut harness = row(false, true, false);
    let combo = harness
        .get_by_role_and_label(Role::ComboBox, "Theme")
        .rect();
    harness.hover_at(combo.center());
    harness.run_steps(5);
    assert!(
        harness
            .query_all_by_label("Set by --conf for this run")
            .count()
            >= 1
    );
}

/// However narrow the row, down to the column itself, the label, its changed mark and the
/// restart note never run into the control: beside it where they fit, else above it.
#[test]
fn a_narrow_row_keeps_its_label_out_of_the_control() {
    let column = Theme::default().sizes.setting_column;
    let inset = Theme::default().spacing.s;
    for width in [640.0, column + 2.0 * inset + 80.0, column + 2.0 * inset] {
        let mut harness = Harness::builder()
            .with_size(vec2(width + 20.0, 140.0))
            .with_context_setup(|ctx| install(ctx, Theme::default()))
            .build_ui(move |ui| {
                ui.set_width(width);
                SettingRow::new("theme", "Theme of the whole interface")
                    .changed("Dark")
                    .after_restart(true)
                    .show(ui, |ui| Combo::new("Theme", &THEMES, Some(1)).show(ui));
            });
        harness.run();
        let combo = harness
            .get_by_role_and_label(Role::ComboBox, "Theme")
            .rect();
        for part in [
            harness
                .get_by_role_and_label(Role::Label, "Theme of the whole interface")
                .rect(),
            harness.get_by_role_and_label(Role::Button, MARK).rect(),
            harness
                .get_by_role_and_label(Role::Label, "after a restart")
                .rect(),
        ] {
            let apart = part.right() <= combo.left() || part.bottom() <= combo.top();
            assert!(apart, "{width}: {part:?} and the control {combo:?}");
        }
    }
}
