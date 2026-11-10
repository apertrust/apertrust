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

//! A dropdown row's list with darktable's type-to-filter (`docs/gui.md`, Choice rows: two
//! styles; decision 42), driven by the keys and read through AccessKit and the text
//! drawn, in both of the row's styles. The filter's own rules are unit tests in
//! `popup/filter.rs`.

use apertrust_ui::components::{Choice, ChoiceAction, ChoiceStyle};
use apertrust_ui::fonts::SEMIBOLD;
use apertrust_ui::{Builtin, Theme, install};
use egui::accesskit::Role;
use egui::{Color32, Event, FontFamily, Key, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const OPTIONS: [&str; 4] = [
    "Daylight",
    "Black body",
    "Fluorescent F11 (narrow tri-band)",
    "Custom",
];
const ROW: &str = "Adaptation illuminant";

/// The row, applied as the app would, and everything it reported.
struct State {
    selected: usize,
    reported: Vec<ChoiceAction>,
    style: ChoiceStyle,
    /// Fluorescent is an option darktable lists that is not ported.
    disabled: Vec<usize>,
}

fn row(style: ChoiceStyle) -> Harness<'static, State> {
    row_in(Theme::default(), style)
}

fn row_in(theme: Theme, style: ChoiceStyle) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(vec2(400.0, 260.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(move |ctx| {
            install(ctx, theme.clone());
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, state: &mut State| {
                let action = Choice::new("Illuminant", &OPTIONS, state.selected)
                    .name(ROW)
                    .style(state.style)
                    .disabled_options(&state.disabled, "Coming in a later version")
                    .show(ui);
                if let Some(action) = action {
                    if let ChoiceAction::Set(i) = action {
                        state.selected = i;
                    }
                    state.reported.push(action);
                }
            },
            State {
                selected: 0,
                reported: Vec::new(),
                style,
                disabled: vec![2],
            },
        );
    harness.run();
    harness
}

/// Opens the list by a click on the row.
fn open(harness: &mut Harness<'_, State>) {
    harness.get_by_role_and_label(Role::ComboBox, ROW).click();
    harness.run();
    assert!(
        harness.query_by_role(Role::Menu).is_some(),
        "the list opens"
    );
}

fn is_open(harness: &Harness<'_, State>) -> bool {
    harness.query_by_role(Role::Menu).is_some()
}

/// Types `text` as the keyboard does.
fn type_text(harness: &mut Harness<'_, State>, text: &str) {
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

fn key(harness: &mut Harness<'_, State>, key: Key) {
    harness.key_press(key);
    harness.run();
}

/// How `option` is drawn in the list: its parts, each with whether it is SemiBold and its
/// colour, as laid out. The list's line is the one drawn text equal to the option.
fn parts(harness: &Harness<'_, State>, option: &str) -> Vec<(String, bool, Color32)> {
    let semibold = FontFamily::Name(SEMIBOLD.into());
    let mut found = harness.output().shapes.iter().filter_map(|clipped| {
        let egui::Shape::Text(shape) = &clipped.shape else {
            return None;
        };
        let job = &shape.galley.job;
        (job.text == option).then(|| {
            job.sections
                .iter()
                .filter(|section| !section.byte_range.is_empty())
                .map(|section| {
                    (
                        job.text[section.byte_range.start.0..section.byte_range.end.0].to_owned(),
                        section.format.font_id.family == semibold,
                        section.format.color,
                    )
                })
                .collect::<Vec<_>>()
        })
    });
    // The row's own option, drawn plain in the same text, comes first for the chosen one;
    // the list's is the last drawn.
    found
        .next_back()
        .unwrap_or_else(|| panic!("{option} is drawn"))
}

/// The text on the chosen option's band and the half-strength colour a non-match takes.
fn colours() -> (Color32, Color32) {
    let colors = Theme::default().colors;
    (colors.text_on_menu_hover, colors.text.gamma_multiply(0.5))
}

/// Every option drawn plain and at full strength: the highlighted one in the text on its
/// band, a disabled one in disabled text, the others in the text colour.
fn assert_all_plain(harness: &Harness<'_, State>) {
    let colors = apertrust_ui::theme(&harness.ctx).colors.clone();
    let highlighted = harness
        .get_by(|node| node.is_focused())
        .accesskit_node()
        .label()
        .unwrap_or_default();
    for (index, option) in OPTIONS.into_iter().enumerate() {
        let colour = if harness.state().disabled.contains(&index) {
            colors.text_disabled
        } else if option == highlighted {
            colors.text_on_menu_hover
        } else {
            colors.text
        };
        assert_eq!(
            parts(harness, option),
            [(option.to_owned(), false, colour)],
            "{option}"
        );
    }
}

#[test]
fn typing_filters_anywhere_in_the_words_whatever_the_case() {
    for style in [ChoiceStyle::Boxed, ChoiceStyle::Blended] {
        let mut harness = row(style);
        open(&mut harness);
        let (text, half) = colours();
        // Nothing typed: every option plain, at full strength.
        assert_all_plain(&harness);
        // Capitals, from the middle of Black body.
        type_text(&mut harness, "LACK B");
        assert_eq!(
            parts(&harness, "Black body"),
            [
                ("B".to_owned(), false, text),
                ("lack b".to_owned(), true, text),
                ("ody".to_owned(), false, text),
            ],
            "{style:?}"
        );
        // The rest are dimmed to half strength, still listed, not bold.
        for option in ["Daylight", "Custom"] {
            assert_eq!(
                parts(&harness, option),
                [(option.to_owned(), false, half)],
                "{option}"
            );
        }
    }
}

#[test]
fn the_highlighted_option_takes_the_text_on_its_band() {
    // In high contrast the band is light and its text dark, apart from every other line.
    let theme = Theme::builtin(Builtin::HighContrast);
    let colors = theme.colors.clone();
    assert_ne!(colors.text_on_menu_hover, colors.text_strong);
    let mut harness = row_in(theme, ChoiceStyle::Boxed);
    open(&mut harness);
    assert_all_plain(&harness);
    assert_eq!(parts(&harness, "Daylight")[0].2, colors.text_on_menu_hover);
    // A match on it keeps that text, its matched letters in SemiBold.
    type_text(&mut harness, "day");
    assert_eq!(
        parts(&harness, "Daylight"),
        [
            ("Day".to_owned(), true, colors.text_on_menu_hover),
            ("light".to_owned(), false, colors.text_on_menu_hover),
        ]
    );
}

#[test]
fn an_option_matches_wherever_the_typed_text_lies_in_it() {
    let mut harness = row(ChoiceStyle::Blended);
    open(&mut harness);
    // The end of Daylight, and the middle of nothing else: "ht" is in Daylight alone.
    type_text(&mut harness, "ht");
    assert_eq!(
        parts(&harness, "Daylight"),
        [
            ("Daylig".to_owned(), false, colours().0),
            ("ht".to_owned(), true, colours().0),
        ]
    );
    // Its start, Custom's, among others that hold the letters elsewhere.
    key(&mut harness, Key::Backspace);
    key(&mut harness, Key::Backspace);
    type_text(&mut harness, "cu");
    assert_eq!(
        parts(&harness, "Custom")[0],
        ("Cu".to_owned(), true, colours().0)
    );
}

#[test]
fn a_key_that_matches_nothing_is_dropped() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    type_text(&mut harness, "q");
    // Nothing was typed: every option plain.
    for option in OPTIONS {
        assert!(parts(&harness, option).iter().all(|(_, bold, _)| !bold));
    }
    // "bl" matches Black body alone; the z after it is dropped, so Enter, which takes the
    // only match, still finds one.
    type_text(&mut harness, "blz");
    key(&mut harness, Key::Enter);
    assert!(!is_open(&harness));
    assert_eq!(harness.state().reported, [ChoiceAction::Set(1)]);
}

#[test]
fn backspace_takes_the_last_key_back() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    // "cu" matches Custom alone; "c" also Black body, which Enter cannot choose between.
    type_text(&mut harness, "cu");
    key(&mut harness, Key::Backspace);
    assert_eq!(
        parts(&harness, "Black body")[1],
        ("c".to_owned(), true, Theme::default().colors.text)
    );
    key(&mut harness, Key::Enter);
    assert!(
        is_open(&harness),
        "two match, so the list stays for more typing"
    );
    assert!(harness.state().reported.is_empty());
    // Typed again, it matches one.
    type_text(&mut harness, "u");
    key(&mut harness, Key::Enter);
    assert_eq!(harness.state().reported, [ChoiceAction::Set(3)]);
    // All the way back: every option plain again, and Backspace on nothing is harmless.
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    type_text(&mut harness, "bl");
    for _ in 0..3 {
        key(&mut harness, Key::Backspace);
    }
    assert_all_plain(&harness);
}

#[test]
fn enter_takes_the_only_match_and_closes_the_list() {
    for style in [ChoiceStyle::Boxed, ChoiceStyle::Blended] {
        let mut harness = row(style);
        // Opened by the keys from the focused row, which is given the focus back.
        harness.get_by_role_and_label(Role::ComboBox, ROW).focus();
        harness.run();
        key(&mut harness, Key::Enter);
        assert!(is_open(&harness), "{style:?}");
        type_text(&mut harness, "black");
        key(&mut harness, Key::Enter);
        assert!(!is_open(&harness), "{style:?}");
        assert_eq!(harness.state().reported, [ChoiceAction::Set(1)]);
        let node = harness.get_by_role_and_label(Role::ComboBox, ROW);
        assert_eq!(node.accesskit_node().value().as_deref(), Some("Black body"));
        assert!(node.accesskit_node().is_focused());
    }
}

#[test]
fn enter_with_nothing_typed_closes_the_list_changing_nothing() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    key(&mut harness, Key::Enter);
    assert!(!is_open(&harness));
    assert!(harness.state().reported.is_empty());
    assert_eq!(harness.state().selected, 0);
}

#[test]
fn with_nothing_typed_the_arrows_move_the_highlight_and_enter_chooses_it() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    // The highlight starts on the chosen option, and passes over the disabled one.
    let focused = |harness: &Harness<'_, State>| {
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
            .unwrap_or_default()
    };
    assert_eq!(focused(&harness), "Daylight");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Black body");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Custom");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Daylight");
    key(&mut harness, Key::ArrowUp);
    assert_eq!(focused(&harness), "Custom");
    key(&mut harness, Key::Enter);
    assert_eq!(harness.state().reported, [ChoiceAction::Set(3)]);
}

#[test]
fn typing_moves_the_arrows_among_the_matches_only() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    let focused = |harness: &Harness<'_, State>| {
        harness
            .get_by(|node| node.is_focused())
            .accesskit_node()
            .label()
            .unwrap_or_default()
    };
    // "o" is in Black body, Fluorescent (disabled) and Custom: the highlight leaves
    // Daylight for the first it can take, and the arrows visit those two.
    type_text(&mut harness, "o");
    assert_eq!(focused(&harness), "Black body");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Custom");
    key(&mut harness, Key::ArrowDown);
    assert_eq!(focused(&harness), "Black body");
}

#[test]
fn a_disabled_option_never_matches_and_is_never_taken() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    // Only Fluorescent, which is not ported, holds an "f": as darktable counts only
    // entries that can be chosen, the key is dropped and nothing is a match.
    type_text(&mut harness, "f");
    assert_all_plain(&harness);
    // "o" is in Fluorescent too, which is drawn as it is, dim, with no match shown.
    type_text(&mut harness, "o");
    assert_eq!(
        parts(&harness, OPTIONS[2]),
        [(
            OPTIONS[2].to_owned(),
            false,
            Theme::default().colors.text_disabled
        )]
    );
    assert!(parts(&harness, "Custom")[0].1 || parts(&harness, "Custom")[1].1);
    // Black body and Custom both match, so Enter takes neither and the list stays.
    key(&mut harness, Key::Enter);
    assert!(is_open(&harness));
    // A click on the disabled option chooses nothing either.
    harness.get_by_label(OPTIONS[2]).click();
    harness.run();
    assert!(harness.state().reported.is_empty());
}

#[test]
fn space_is_a_character_once_text_is_typed_and_chooses_before() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    type_text(&mut harness, "black");
    // The key and its text, as the platform sends them.
    harness.key_press(Key::Space);
    harness.event(Event::Text(" ".to_owned()));
    harness.run();
    assert!(is_open(&harness));
    assert_eq!(
        parts(&harness, "Black body")[0],
        ("Black ".to_owned(), true, colours().0)
    );
    // A space starts no filter.
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    harness.event(Event::Text(" ".to_owned()));
    harness.run();
    assert!(parts(&harness, "Custom").iter().all(|(_, bold, _)| !bold));
    // With nothing typed Space chooses the highlighted option, as Enter does.
    key(&mut harness, Key::ArrowDown);
    key(&mut harness, Key::Space);
    assert_eq!(harness.state().reported, [ChoiceAction::Set(1)]);
}

#[test]
fn escape_closes_the_list_even_with_text_typed_and_changes_nothing() {
    let mut harness = row(ChoiceStyle::Blended);
    open(&mut harness);
    type_text(&mut harness, "cus");
    key(&mut harness, Key::Escape);
    assert!(!is_open(&harness));
    assert!(harness.state().reported.is_empty());
    // Opened again, the filter is empty.
    open(&mut harness);
    for option in OPTIONS {
        assert!(parts(&harness, option).iter().all(|(_, bold, _)| !bold));
    }
}

#[test]
fn the_list_still_names_every_option_to_screen_readers_while_it_filters() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    type_text(&mut harness, "cus");
    for (i, option) in OPTIONS.iter().enumerate() {
        let node = harness.get_by_label(option);
        // The chosen one is a checked item; Fluorescent is disabled.
        assert_eq!(node.accesskit_node().is_disabled(), i == 2, "{option}");
        assert_eq!(
            node.accesskit_node().role(),
            Role::MenuItemCheckBox,
            "{option}"
        );
    }
    assert_eq!(
        harness.get_by_label("Daylight").accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert_eq!(
        harness
            .get_by_role(Role::Menu)
            .accesskit_node()
            .label()
            .as_deref(),
        Some(ROW)
    );
}

#[test]
fn a_disabled_option_says_why_under_the_pointer() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    harness.get_by_label(OPTIONS[2]).hover();
    harness.run();
    harness.run();
    assert!(
        harness
            .query_by_label("Coming in a later version")
            .is_some()
    );
}

#[test]
fn a_click_on_the_row_while_the_list_is_open_closes_it() {
    let mut harness = row(ChoiceStyle::Boxed);
    open(&mut harness);
    harness.get_by_role_and_label(Role::ComboBox, ROW).click();
    harness.run();
    assert!(!is_open(&harness));
    assert!(harness.state().reported.is_empty());
}

#[test]
fn a_click_on_the_row_s_label_while_the_list_is_open_closes_it() {
    let mut harness = row(ChoiceStyle::Blended);
    open(&mut harness);
    let row = harness.get_by_role_and_label(Role::ComboBox, ROW).rect();
    // The label, at the row's start, away from the option the list hangs from.
    harness.hover_at(row.left_center() + vec2(4.0, 0.0));
    harness.run();
    harness.drag_at(row.left_center() + vec2(4.0, 0.0));
    harness.drop_at(row.left_center() + vec2(4.0, 0.0));
    harness.run();
    assert!(!is_open(&harness), "closed, not closed and opened again");
    assert!(harness.state().reported.is_empty());
}

#[test]
fn a_click_on_an_option_chooses_it_with_text_typed_too() {
    let mut harness = row(ChoiceStyle::Blended);
    open(&mut harness);
    type_text(&mut harness, "cus");
    // A dimmed option is still listed and can be chosen.
    harness.get_by_label("Black body").click();
    harness.run();
    assert_eq!(harness.state().reported, [ChoiceAction::Set(1)]);
    assert!(!is_open(&harness));
}

/// How a filtered list looks: the matches in SemiBold, the rest at half strength, the
/// disabled option dim as ever. Reviewed by eye like the gallery's.
#[test]
fn a_filtered_list_looks_as_darktable_s_does() {
    let mut harness = row(ChoiceStyle::Blended);
    open(&mut harness);
    type_text(&mut harness, "o");
    harness.snapshot("dropdown-filtered");
}

/// What has the focus, by its accessible name.
fn focused(harness: &Harness<'_, State>) -> String {
    harness
        .get_by(|node| node.is_focused())
        .accesskit_node()
        .label()
        .unwrap_or_default()
}

/// Opened with the pointer, the list gives the focus to the row as it closes, chosen or
/// not: a press leaves no focus to give back, so the row is the list's opener.
#[test]
fn a_list_opened_with_the_pointer_gives_the_focus_to_the_row() {
    for style in [ChoiceStyle::Boxed, ChoiceStyle::Blended] {
        let mut harness = row(style);
        open(&mut harness);
        harness.get_by_label("Black body").click();
        harness.run();
        assert_eq!(harness.state().reported, [ChoiceAction::Set(1)]);
        assert_eq!(focused(&harness), ROW, "{style:?}, chosen");
        let mut harness = row(style);
        open(&mut harness);
        key(&mut harness, Key::Escape);
        assert_eq!(focused(&harness), ROW, "{style:?}, Escape");
    }
}

/// Space, Enter or ↓ opens a focused row (`docs/gui.md`, Popups: Focus and keys).
#[test]
fn space_enter_or_down_opens_a_focused_row() {
    for opening in [Key::Space, Key::Enter, Key::ArrowDown] {
        let mut harness = row(ChoiceStyle::Blended);
        harness.get_by_role_and_label(Role::ComboBox, ROW).focus();
        harness.run();
        key(&mut harness, opening);
        assert!(is_open(&harness), "{opening:?}");
        key(&mut harness, Key::Escape);
        assert!(!is_open(&harness));
        assert_eq!(focused(&harness), ROW);
    }
}
