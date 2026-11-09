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

//! The alert's body for the platforms with no alert of their own (`docs/gui.md`, Dialogs
//! and alerts; decision 41), driven through AccessKit: its role and name, its buttons in
//! the platform's order, what Return, Escape and a click answer; and the alerts' words in
//! every language.

use apertrust_ui::alert::{Request, Role};
use apertrust_ui::components::{AlertBody, ButtonOrder};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Locale, Theme, Translator};
use egui::accesskit::Role as Access;
use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

fn overwrite() -> Request {
    Request::new("Export in overwrite mode?", "The old files go.")
        .button("Overwrite", Role::Destructive)
        .button("Cancel", Role::Cancel)
}

/// The same, the way Export's overwrite question is asked: the cancel is the default.
fn cancelling() -> Request {
    overwrite().default_button(1)
}

fn plain() -> Request {
    Request::new("Saved", "Nothing else to do.").button("OK", Role::Default)
}

/// The body drawn in a harness, with the answers it gave.
struct Run {
    request: Request,
    order: ButtonOrder,
    ticked: bool,
    answers: Vec<usize>,
    ticked_answers: Vec<bool>,
}

fn body(request: Request, order: ButtonOrder) -> Harness<'static, Run> {
    Harness::builder()
        .with_size(vec2(480.0, 200.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, run: &mut Run| {
                let answer = AlertBody::new(&run.request, &mut run.ticked)
                    .order(run.order)
                    .show(ui);
                if let Some(answer) = answer {
                    run.answers.push(answer.pressed);
                    run.ticked_answers.push(answer.ticked);
                }
            },
            Run {
                request,
                order,
                ticked: false,
                answers: Vec::new(),
                ticked_answers: Vec::new(),
            },
        )
}

#[test]
fn the_body_is_an_alert_dialog_named_by_its_title_with_the_message_and_the_buttons() {
    let mut harness = body(overwrite(), ButtonOrder::DefaultLast);
    harness.run();
    harness.get_by_role_and_label(Access::AlertDialog, "Export in overwrite mode?");
    harness.get_by_label("The old files go.");
    harness.get_by_role_and_label(Access::Button, "Overwrite");
    harness.get_by_role_and_label(Access::Button, "Cancel");
}

#[test]
fn in_an_alert_window_the_body_leaves_the_node_to_the_window() {
    let request = overwrite();
    let mut ticked = false;
    let mut harness = Harness::builder()
        .with_size(vec2(480.0, 200.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(move |ui| {
            AlertBody::new(&request, &mut ticked)
                .in_alert_window()
                .show(ui);
        });
    harness.run();
    assert_eq!(harness.query_all_by_role(Access::AlertDialog).count(), 0);
    harness.get_by_role_and_label(Access::Button, "Overwrite");
}

#[test]
fn the_suppression_box_is_unticked_and_its_answer_keeps_the_choice() {
    let request = cancelling().suppression("Don't ask again");
    let mut harness = body(request, ButtonOrder::DefaultLast);
    harness.run();
    let checkbox = harness.get_by_role_and_label(Access::CheckBox, "Don't ask again");
    assert_eq!(
        checkbox.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    checkbox.click();
    harness.run();
    harness
        .get_by_role_and_label(Access::Button, "Overwrite")
        .click();
    harness.run();
    assert_eq!(harness.state().answers, [0]);
    assert_eq!(harness.state().ticked_answers, [true]);
}

#[test]
fn with_the_box_focused_space_ticks_it_and_return_presses_the_default() {
    let request = cancelling().suppression("Don't ask again");
    let mut harness = body(request, ButtonOrder::DefaultLast);
    harness.run();
    // The box comes first in the focus order.
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(
        harness
            .get_by_role_and_label(Access::CheckBox, "Don't ask again")
            .is_focused()
    );
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(harness.state().answers.is_empty());
    assert!(harness.state().ticked);
    harness.key_press(egui::Key::Enter);
    harness.run();
    // Cancel, the default, with the box as it was left.
    assert_eq!(harness.state().answers, [1]);
    assert_eq!(harness.state().ticked_answers, [true]);
}

#[test]
fn a_focused_control_after_the_body_takes_return_itself() {
    // As the dialog host's frame draws its close after the alert.
    struct Outside {
        request: Request,
        ticked: bool,
        answers: Vec<usize>,
        clicked: usize,
    }
    let mut harness = Harness::builder()
        .with_size(vec2(480.0, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, run: &mut Outside| {
                let answer = AlertBody::new(&run.request, &mut run.ticked).show(ui);
                run.answers.extend(answer.map(|answer| answer.pressed));
                if ui.button("Close").clicked() {
                    run.clicked += 1;
                }
            },
            Outside {
                request: plain(),
                ticked: false,
                answers: Vec::new(),
                clicked: 0,
            },
        );
    harness.run();
    // Tab meets OK, then the close after the body.
    harness.key_press(egui::Key::Tab);
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(harness.get_by_label("Close").is_focused());
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().clicked, 1);
    assert!(harness.state().answers.is_empty());
}

#[test]
fn the_buttons_are_met_in_the_order_they_are_seen() {
    let order_of = |order| {
        let mut harness = body(overwrite(), order);
        harness.run();
        let overwrite = harness.get_by_role_and_label(Access::Button, "Overwrite");
        let cancel = harness.get_by_role_and_label(Access::Button, "Cancel");
        (overwrite.rect().min.x, cancel.rect().min.x)
    };
    // The Mac's: Cancel, then the default at the end.
    let (overwrite, cancel) = order_of(ButtonOrder::DefaultLast);
    assert!(cancel < overwrite);
    // Windows': the default first.
    let (overwrite, cancel) = order_of(ButtonOrder::DefaultFirst);
    assert!(overwrite < cancel);
}

#[test]
fn the_default_sits_where_the_platform_puts_it_whatever_the_order_of_the_request() {
    let order_of = |order| {
        let mut harness = body(cancelling(), order);
        harness.run();
        let overwrite = harness.get_by_role_and_label(Access::Button, "Overwrite");
        let cancel = harness.get_by_role_and_label(Access::Button, "Cancel");
        (overwrite.rect().min.x, cancel.rect().min.x)
    };
    // The Mac's: the default, Cancel, at the end, Overwrite before it.
    let (overwrite, cancel) = order_of(ButtonOrder::DefaultLast);
    assert!(overwrite < cancel);
    // Windows': the default first.
    let (overwrite, cancel) = order_of(ButtonOrder::DefaultFirst);
    assert!(cancel < overwrite);
}

#[test]
fn with_the_cancel_as_the_default_return_and_escape_both_answer_it() {
    let mut harness = body(cancelling(), ButtonOrder::DefaultLast);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().answers, [1, 1]);
    // And Overwrite answers only to a click.
    harness
        .get_by_role_and_label(Access::Button, "Overwrite")
        .click();
    harness.run();
    assert_eq!(harness.state().answers, [1, 1, 0]);
}

#[test]
fn a_click_answers_the_index_of_the_button_in_the_request() {
    let mut harness = body(overwrite(), ButtonOrder::DefaultLast);
    harness.run();
    harness
        .get_by_role_and_label(Access::Button, "Cancel")
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Access::Button, "Overwrite")
        .click();
    harness.run();
    assert_eq!(harness.state().answers, [1, 0]);
}

#[test]
fn return_presses_the_first_button_and_escape_the_cancel() {
    let mut harness = body(overwrite(), ButtonOrder::DefaultLast);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().answers, [0, 1]);
}

#[test]
fn escape_answers_nothing_where_there_is_no_cancel() {
    let mut harness = body(plain(), ButtonOrder::DefaultLast);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().answers.is_empty());
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().answers, [0]);
}

#[test]
fn a_focused_button_takes_return_itself() {
    let mut harness = body(overwrite(), ButtonOrder::DefaultLast);
    harness.run();
    harness
        .get_by_role_and_label(Access::Button, "Cancel")
        .focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    // The focused Cancel presses itself; the body does not add Return's own answer.
    assert_eq!(harness.state().answers, [1]);
}

fn quit(words: &Translator, stoppable: f64, waiting: f64) -> String {
    let mut args = Args::new();
    args.set("stoppable", stoppable);
    args.set("waiting", waiting);
    words.text_with("alert-quit-message", Some(&args))
}

#[test]
fn every_language_words_the_alerts_and_the_quit_message_follows_what_it_stops() {
    for locale in [Locale::EnGb, Locale::Uk, Locale::Pl] {
        let words = Translator::new(locale);
        for id in [
            "alert-cancel",
            "alert-overwrite-title",
            "alert-overwrite-message",
            "alert-overwrite-button",
            "alert-overwrite-suppress",
            "alert-quit-title",
            "alert-quit-button",
        ] {
            let text = words.text(id);
            assert_ne!(text, id, "{locale:?} lacks {id}");
            assert!(!text.is_empty());
        }
        // Three cases: all stop, some wait, none stops; each is its own sentence.
        let stops = quit(&words, 2.0, 0.0);
        let mixed = quit(&words, 1.0, 1.0);
        let waits = quit(&words, 0.0, 2.0);
        assert!(
            stops != mixed && mixed != waits && stops != waits,
            "{locale:?}"
        );
        for text in [&stops, &mixed, &waits] {
            assert_ne!(text, "alert-quit-message");
            assert!(!text.contains('$') && !text.contains('{'), "{text}");
        }
    }
    let english = Translator::new(Locale::EnGb);
    assert_eq!(
        english.text("alert-overwrite-title"),
        "Export in overwrite mode?"
    );
    assert!(quit(&english, 0.0, 1.0).starts_with("Apertrust quits once"));
}
