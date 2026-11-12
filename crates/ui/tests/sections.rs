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

//! Interaction tests for the section frame and the side panel's tab bar, driven through
//! AccessKit (`docs/gui.md`, Testing).

use apertrust_ui::components::{
    PanelBody, PanelTab, Section, SectionAction, SliderRow, TabBar, TabBarAction, section_list,
};
use apertrust_ui::icons::STROKE;
use apertrust_ui::{Icon, Locale, Theme};
use egui::Key;
use egui::accesskit::{HasPopup, Role, Toggled};
use egui::epaint::TextShape;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// A module section, open or closed, with tabs, and what it last reported.
#[derive(Default)]
struct Module {
    open: bool,
    tab: usize,
    actions: Vec<SectionAction>,
    /// Whether the peek keys are held, as the app's actions registry would report.
    peek_held: bool,
    peeking: bool,
}

fn module(open: bool) -> Harness<'static, Module> {
    Harness::builder()
        .with_size(egui::vec2(400.0, 260.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, state: &mut Module| {
                let out = Section::new("White balance", state.open)
                    .module()
                    .tabs(&["Basic", "Advanced"], state.tab)
                    .peek(state.peek_held)
                    .show(ui, |ui| {
                        SliderRow::new("Tint", 0.0, -100.0..=100.0).show(ui);
                    });
                for action in &out.actions {
                    match action {
                        SectionAction::Toggle => state.open = !state.open,
                        SectionAction::Tab(i) => state.tab = *i,
                        _ => {}
                    }
                }
                state.actions.extend(out.actions);
                state.peeking = out.peeking;
            },
            Module {
                open,
                ..Default::default()
            },
        )
}

#[test]
fn the_header_opens_and_closes_the_section() {
    let mut harness = module(false);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Tint")
            .is_none()
    );
    harness
        .get_by_role_and_label(Role::Button, "White balance")
        .click();
    harness.run();
    assert_eq!(harness.state().actions, vec![SectionAction::Toggle]);
    assert!(harness.state().open);
    assert!(
        harness
            .query_by_role_and_label(Role::Slider, "Tint")
            .is_some()
    );
}

#[test]
fn the_header_buttons_report_in_the_wireframe_s_order() {
    let mut harness = module(true);
    harness.run();
    for name in ["Reset White balance", "White balance presets"] {
        harness.get_by_label(name).click();
        harness.run();
    }
    assert_eq!(
        harness.state().actions,
        vec![SectionAction::Reset, SectionAction::Presets]
    );
}

/// A module's section and a utility's, closed, one under the other, presets and options
/// asked of both, enabled or not, and what they reported.
fn module_and_utility(enabled: bool) -> Harness<'static, Vec<SectionAction>> {
    Harness::builder()
        .with_size(egui::vec2(400.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            move |ui, actions: &mut Vec<SectionAction>| {
                for section in [
                    Section::new("Exposure", false).module(),
                    Section::new("Scopes", false),
                ] {
                    let out = section.presets(enabled).options(enabled).show(ui, |_| {});
                    actions.extend(out.actions);
                }
            },
            Vec::new(),
        )
}

/// A module's header ends in its options button, drawn disabled even when asked to run,
/// with a tooltip saying why; a utility's runs. Each button is `sizes.section_button` wide,
/// `spacing.section_button_gap` from the next, and the last ends where both headers' do.
#[test]
fn a_module_s_options_are_disabled_and_a_utility_s_run() {
    let mut harness = module_and_utility(true);
    harness.run();
    let t = Theme::default();
    let rect = |name: &str| harness.get_by_label(name).rect();
    let reset = rect("Reset Exposure");
    let presets = rect("Exposure presets");
    let options = rect("Exposure options");
    for button in [reset, presets, options] {
        assert_eq!(button.width(), t.sizes.section_button, "{button:?}");
    }
    assert_eq!(presets.left() - reset.right(), t.spacing.section_button_gap);
    assert_eq!(
        options.left() - presets.right(),
        t.spacing.section_button_gap
    );
    assert_eq!(options.right(), rect("Scopes options").right());

    let module = harness.get_by_label("Exposure options");
    assert!(module.accesskit_node().is_disabled());
    // Screen readers hear why without a hover.
    assert_eq!(
        module.accesskit_node().description(),
        Some("No options yet".to_owned())
    );
    module.hover();
    harness.run();
    harness.get_by_label("No options yet");
    harness.get_by_label("Exposure options").click();
    harness.run();
    assert!(harness.state().is_empty());
    harness.get_by_label("Scopes options").click();
    harness.run();
    assert_eq!(*harness.state(), vec![SectionAction::Options]);
}

/// The header's last button's ink, not its box, ends where the rows do, `body_margin` from
/// the section's end: ⋯'s last dot ends 4 of its 24 units inside its box less half its
/// line, which the icon's own test holds.
#[test]
fn the_header_s_ink_ends_where_the_rows_do() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 60.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, end: &mut f32| {
                *end = ui.max_rect().right();
                Section::new("Exposure", false).module().show(ui, |_| {});
            },
            0.0,
        );
    harness.run();
    let t = Theme::default();
    let options = harness.get_by_label("Exposure options").rect();
    let ink = options.right() - (4.0 / 24.0 * Icon::Options.size() - STROKE / 2.0);
    let inset = *harness.state() - ink;
    // egui rounds a widget's rect to 1/32 of a point (`emath::GUI_ROUNDING`).
    assert!(
        (inset - Section::body_margin(&t)).abs() <= egui::emath::GUI_ROUNDING,
        "the ink ends {inset} points from the section's end"
    );
}

/// A module that is off and renders nothing (decision 32) shows nothing more, but its
/// toggle's description starts with "Not applied" for screen readers; one that is on says
/// nothing of it.
#[test]
fn a_module_not_applied_says_so_to_screen_readers_alone() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(|ui| {
            Section::new("Grain", false)
                .module()
                .not_applied(true)
                .show(ui, |_| {});
            Section::new("Exposure", false)
                .module()
                .not_applied(false)
                .show(ui, |_| {});
        });
    harness.run();
    let said = |name: &str| {
        harness
            .get_by_role_and_label(Role::Button, name)
            .accesskit_node()
            .description()
            .unwrap_or_default()
    };
    assert!(
        said("Grain").starts_with("Not applied"),
        "{}",
        said("Grain")
    );
    assert!(
        !said("Exposure").contains("Not applied"),
        "{}",
        said("Exposure")
    );
}

#[test]
fn a_module_header_has_no_auto_button() {
    let mut harness = module(true);
    harness.run();
    assert!(harness.query_by_label("Auto").is_none());
}

#[test]
fn presets_and_options_drawn_disabled_do_not_run() {
    let mut harness = module_and_utility(false);
    harness.run();
    for name in ["Exposure presets", "Exposure options", "Scopes options"] {
        let button = harness.get_by_label(name);
        assert!(button.accesskit_node().is_disabled(), "{name}");
        button.click();
        harness.run();
    }
    harness.get_by_label("Reset Exposure").click();
    harness.run();
    assert_eq!(*harness.state(), vec![SectionAction::Reset]);
}

#[test]
fn the_tabs_switch() {
    let mut harness = module(true);
    harness.run();
    harness.get_by_role_and_label(Role::Tab, "Advanced").click();
    harness.run();
    assert_eq!(harness.state().tab, 1);
    assert_eq!(harness.state().actions, vec![SectionAction::Tab(1)]);
}

#[test]
fn a_tab_with_nothing_behind_it_is_disabled_and_says_so() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, actions: &mut Vec<SectionAction>| {
                let out = Section::new("Tone curve", true)
                    .module()
                    .tabs(&["Luma", "RGB", "Advanced"], 0)
                    .disabled_tabs(&[1], "Coming in a later version")
                    .show(ui, |_| {});
                actions.extend(out.actions);
            },
            Vec::new(),
        );
    harness.run();
    let later = harness.get_by_role_and_label(Role::Tab, "RGB");
    assert!(later.accesskit_node().is_disabled());
    later.hover();
    harness.run();
    harness.get_by_label("Coming in a later version");
    harness.get_by_role_and_label(Role::Tab, "RGB").click();
    harness.run();
    assert!(harness.state().is_empty());
    // The others still switch.
    harness.get_by_role_and_label(Role::Tab, "Advanced").click();
    harness.run();
    assert_eq!(*harness.state(), vec![SectionAction::Tab(2)]);
}

/// A module section with `tabs`, at `width` in the side panel's body, and the tab chosen.
fn tabbed(tabs: Vec<String>, width: f32, locale: Locale) -> Harness<'static, usize> {
    Harness::builder()
        .with_size(egui::vec2(width + 40.0, 160.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            move |ui, tab: &mut usize| {
                let names: Vec<&str> = tabs.iter().map(String::as_str).collect();
                ui.allocate_ui(egui::vec2(width, 140.0), |ui| {
                    PanelBody::new().show(ui, |ui| {
                        let out = Section::new("Module", true)
                            .module()
                            .tabs(&names, *tab)
                            .show(ui, |_| {});
                        for action in out.actions {
                            if let SectionAction::Tab(i) = action {
                                *tab = i;
                            }
                        }
                    });
                });
            },
            0,
        )
}

/// Each tab's rectangle, by name, in order, and the row's.
fn tab_rects<State>(
    harness: &Harness<'_, State>,
    names: &[String],
) -> (Vec<egui::Rect>, egui::Rect) {
    let tabs = names
        .iter()
        .map(|name| harness.get_by_role_and_label(Role::Tab, name).rect())
        .collect();
    (tabs, harness.get_by_role(Role::TabList).rect())
}

/// Colour calibration's seven tabs in `locale`, CAT first.
fn calibration_tabs(locale: Locale) -> Vec<String> {
    let words = apertrust_ui::Translator::new(locale);
    [
        "cat",
        "red",
        "green",
        "blue",
        "colourfulness",
        "brightness",
        "grey",
    ]
    .map(|tab| words.text(&format!("module-channelmixerrgb-tab-{tab}")))
    .to_vec()
}

#[test]
fn each_tab_is_as_wide_as_its_name_from_the_row_s_start() {
    let names = ["Grey", "R", "Colourfulness", "Brightness"].map(String::from);
    let mut harness = tabbed(names.to_vec(), 600.0, Locale::EnGb);
    harness.run();
    let (tabs, row) = tab_rects(&harness, &names);
    // Wider as their names are longer: R, Grey, Brightness, Colourfulness.
    let width = |i: usize| tabs[i].width();
    assert!(
        width(1) < width(0) && width(0) < width(3) && width(3) < width(2),
        "{tabs:?}"
    );
    // The chosen one's name, SemiBold, with 8 points at each end.
    let chosen = drawn_name(&harness, "Grey");
    assert!(
        (width(0) - 16.0 - chosen.galley.size().x).abs() < 0.01,
        "{tabs:?}"
    );
    // One after another from the row's start, the rest of the row empty.
    assert_eq!(tabs[0].left(), row.left());
    for pair in tabs.windows(2) {
        assert!((pair[1].left() - pair[0].right()).abs() < 0.01, "{tabs:?}");
    }
    assert!(tabs[3].right() < row.right() - 300.0, "{tabs:?} in {row:?}");
    for name in &names {
        assert!(!drawn_name(&harness, name).galley.elided, "{name} is cut");
    }
    // Choosing another tab, SemiBold now, moves none of them.
    harness.get_by_role_and_label(Role::Tab, "R").click();
    harness.run();
    assert_eq!(*harness.state(), 1);
    assert_eq!(tab_rects(&harness, &names).0, tabs);
}

/// The Tone curve's six tabs in English.
fn tone_curve_tabs() -> Vec<String> {
    let words = apertrust_ui::Translator::new(Locale::EnGb);
    ["luma", "rgb", "red", "green", "blue", "advanced"]
        .map(|tab| words.text(&format!("module-tone_curve-tab-{tab}")))
        .to_vec()
}

/// At the side panel's 320 points Colour calibration's and the Tone curve's English tabs
/// are too wide with 8 points at each end: the padding gives way, all tabs alike, and
/// every name stays whole.
#[test]
fn at_the_side_panel_s_usual_width_the_padding_gives_way_before_the_names() {
    for names in [calibration_tabs(Locale::EnGb), tone_curve_tabs()] {
        let mut wide = tabbed(names.clone(), 600.0, Locale::EnGb);
        wide.run();
        let (natural, _) = tab_rects(&wide, &names);
        let mut harness = tabbed(names.clone(), 320.0, Locale::EnGb);
        harness.run();
        let (tabs, row) = tab_rects(&harness, &names);
        // Each tab loses the same room, under 8 points of its 16.
        let lost = natural[0].width() - tabs[0].width();
        assert!(lost > 0.0 && lost <= 8.0, "{names:?}: {lost}");
        for (i, name) in names.iter().enumerate() {
            assert!(
                (natural[i].width() - tabs[i].width() - lost).abs() < 1e-3,
                "{name} is {:?}, {:?} at full width",
                tabs[i],
                natural[i]
            );
            assert!(!drawn_name(&harness, name).galley.elided, "{name} is cut");
        }
        assert!(
            (tabs[names.len() - 1].right() - row.right()).abs() < 0.01,
            "{tabs:?} in {row:?}"
        );
    }
}

/// Colour calibration's seven tabs at the side panel's narrowest, 240 points: 4 points at
/// each end, the names sharing the rest in proportion to their widths, one after another
/// on one row, each cut at its end with an ellipsis; a single letter, R, G or B, cannot be
/// cut and stays whole in its tab.
#[test]
fn at_the_narrowest_side_panel_the_tabs_shrink_in_proportion_on_one_row() {
    for locale in [Locale::Uk, Locale::Pseudo] {
        let names = calibration_tabs(locale);
        let mut wide = tabbed(names.clone(), 600.0, locale);
        wide.run();
        let (natural, _) = tab_rects(&wide, &names);
        // A name's own width: its tab's at full width, less 8 points at each end.
        let name_width = |i: usize| natural[i].width() - 16.0;
        let mut harness = tabbed(names.clone(), 240.0, locale);
        harness.run();
        let (tabs, row) = tab_rects(&harness, &names);
        let share = (tabs[0].width() - 8.0) / name_width(0);
        assert!(share < 0.8, "{locale:?}: {share}");
        for (i, name) in names.iter().enumerate() {
            let tab = tabs[i];
            assert!(
                ((tab.width() - 8.0) / name_width(i) - share).abs() < 1e-3,
                "{locale:?}: {name} is {tab:?}, {natural:?} at full width"
            );
            assert_eq!(tab.y_range(), row.y_range(), "{locale:?}: {name}");
            let drawn = drawn_name(&harness, name);
            assert_eq!(
                drawn.galley.elided,
                name.chars().count() > 1,
                "{locale:?}: {name}"
            );
            // One line, where a wrapped name would run under the tab's line.
            assert_eq!(drawn.galley.rows.len(), 1, "{locale:?}: {name}");
            let drawn = drawn.visual_bounding_rect();
            assert!(
                tab.contains_rect(drawn),
                "{locale:?}: {name} drawn over {drawn:?}, outside its tab {tab:?}"
            );
        }
        assert_eq!(tabs[0].left(), row.left());
        for pair in tabs.windows(2) {
            assert!((pair[1].left() - pair[0].right()).abs() < 0.01, "{tabs:?}");
        }
        assert!(tabs[6].right() <= row.right() + 0.01, "{tabs:?} in {row:?}");
    }
}

#[test]
fn a_cut_tab_says_its_whole_name_in_its_tooltip_and_to_screen_readers() {
    let names = calibration_tabs(Locale::Uk);
    let mut harness = tabbed(names.clone(), 240.0, Locale::Uk);
    harness.run();
    // Colourfulness, Brightness and Grey, after CAT, R, G and B.
    for name in &names[4..] {
        assert!(drawn_name(&harness, name).galley.elided, "{name} is whole");
        // The tab itself carries the whole name for screen readers.
        let tab = harness.get_by_role_and_label(Role::Tab, name);
        tab.hover();
        harness.run();
        assert!(
            harness
                .query_all_by_label(name)
                .any(|node| node.accesskit_node().role() != Role::Tab),
            "no tooltip says {name}"
        );
    }
}

/// What has the keyboard focus, by its accessible name.
fn focused<State>(harness: &Harness<'_, State>) -> String {
    let node = harness.get_by(|node| node.is_focused());
    node.accesskit_node().label().unwrap_or_default()
}

/// The keys move between tabs of any width: the arrows to the neighbour on either side,
/// however narrow, Tab and Shift+Tab in order, Enter or Space choosing.
#[test]
fn the_keys_move_between_tabs_and_choose_one() {
    let names = calibration_tabs(Locale::Uk);
    let mut harness = tabbed(names.clone(), 240.0, Locale::Uk);
    harness.run();
    harness.get_by_role_and_label(Role::Tab, &names[0]).focus();
    harness.run();
    for name in &names[1..] {
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        assert_eq!(&focused(&harness), name);
    }
    for name in names[..names.len() - 1].iter().rev() {
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert_eq!(&focused(&harness), name);
    }
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), names[1]);
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 1);
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    harness.run();
    assert_eq!(focused(&harness), names[0]);
    harness.key_press(egui::Key::Space);
    harness.run();
    assert_eq!(*harness.state(), 0);
}

#[test]
fn the_section_peeks_while_the_peek_keys_are_held_over_it_or_its_header_has_focus() {
    let mut harness = module(true);
    harness.run();
    let header = harness
        .get_by_role_and_label(Role::Button, "White balance")
        .rect();
    harness.hover_at(header.center());
    harness.run();
    assert!(!harness.state().peeking);
    harness.state_mut().peek_held = true;
    harness.run();
    assert!(harness.state().peeking);
    harness.state_mut().peek_held = false;
    harness.run();
    assert!(!harness.state().peeking);

    // Away from the pointer, a section peeks only while its header has focus.
    harness.event(egui::Event::PointerGone);
    harness.state_mut().peek_held = true;
    harness.run();
    assert!(!harness.state().peeking);
    harness
        .get_by_role_and_label(Role::Button, "White balance")
        .focus();
    harness.run();
    assert!(harness.state().peeking);
}

#[test]
fn only_the_section_under_the_pointer_peeks() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 400.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, peeking: &mut [bool; 2]| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (i, title) in ["White balance", "Exposure"].into_iter().enumerate() {
                    let out = Section::new(title, true).peek(true).show(ui, |ui| {
                        SliderRow::new("Tint", 0.0, -100.0..=100.0)
                            .name(&format!("{title} tint"))
                            .show(ui);
                    });
                    peeking[i] = out.peeking;
                }
            },
            [false; 2],
        );
    harness.run();
    let below = harness
        .get_by_role_and_label(Role::Button, "Exposure")
        .rect();
    harness.hover_at(below.center());
    harness.run();
    assert_eq!(*harness.state(), [false, true]);
    // Over the first section's body, not its header.
    let body = harness
        .get_by_role_and_label(Role::Slider, "White balance tint")
        .rect();
    harness.hover_at(body.center());
    harness.run();
    assert_eq!(*harness.state(), [true, false]);
    // Below both.
    harness.hover_at(egui::pos2(200.0, 395.0));
    harness.run();
    assert_eq!(*harness.state(), [false, false]);
}

/// Edit's tabs (decision 38), Photos and History drawn disabled until something is
/// behind them, each with its key.
fn tab_bar(width: f32) -> Harness<'static, (usize, bool, Vec<TabBarAction>)> {
    Harness::builder()
        .with_size(egui::vec2(width, 240.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::default());
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            |ui, state: &mut (usize, bool, Vec<TabBarAction>)| {
                let tabs: Vec<PanelTab> = [
                    (Icon::Folder, "Photos", false),
                    (Icon::Base, "Base", true),
                    (Icon::Color, "Colour", true),
                    (Icon::Correct, "Correct", true),
                    (Icon::Effect, "Effect", true),
                    (Icon::History, "History", false),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (icon, name, enabled))| PanelTab {
                    icon,
                    name,
                    shortcut: Some(format!("Ctrl+{}", i + 1)),
                    enabled,
                })
                .collect();
                if let Some(action) = TabBar::new(&tabs, state.0, state.1).show(ui) {
                    match action {
                        TabBarAction::Select(i) => state.0 = i,
                        TabBarAction::ShowNames(on) => state.1 = on,
                        TabBarAction::Customise => {}
                    }
                    state.2.push(action);
                }
            },
            (1, true, Vec::new()),
        )
}

#[test]
fn a_tab_is_chosen_by_clicking_it() {
    let mut harness = tab_bar(440.0);
    harness.run();
    assert!(harness.query_by_label("More tabs").is_none());
    harness.get_by_role_and_label(Role::Tab, "Correct").click();
    harness.run();
    assert_eq!(harness.state().0, 3);
}

#[test]
fn tabs_that_do_not_fit_are_in_the_more_menu() {
    let mut harness = tab_bar(220.0);
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Tab, "Effect")
            .is_none()
    );
    harness.get_by_label("More tabs").click();
    harness.run();
    harness.get_by_label("Effect").click();
    harness.run();
    assert_eq!(harness.state().2, vec![TabBarAction::Select(4)]);
}

/// Photos and History, with nothing behind them yet: drawn disabled, reported so to
/// screen readers, never chosen, in the bar or under », and saying they are to come,
/// with no key.
#[test]
fn a_tab_with_nothing_behind_it_cannot_be_chosen_and_says_it_is_to_come() {
    let mut harness = tab_bar(440.0);
    harness.run();
    let photos = harness.get_by_role_and_label(Role::Tab, "Photos");
    assert!(photos.accesskit_node().is_disabled());
    assert_eq!(photos.accesskit_node().data().keyboard_shortcut(), None);
    let base = harness.get_by_role_and_label(Role::Tab, "Base");
    assert!(!base.accesskit_node().is_disabled());
    assert_eq!(
        base.accesskit_node().data().keyboard_shortcut(),
        Some("Ctrl+2")
    );
    harness.get_by_role_and_label(Role::Tab, "Photos").click();
    harness.run();
    assert_eq!(harness.state().0, 1);
    assert!(harness.state().2.is_empty());
    harness.get_by_role_and_label(Role::Tab, "Photos").hover();
    harness.run();
    assert!(
        harness
            .query_by_label("Photos: coming in a later version")
            .is_some()
    );
    // History is under » in a narrow bar, disabled there too.
    let mut harness = tab_bar(220.0);
    harness.run();
    harness.get_by_label("More tabs").click();
    harness.run();
    let history = harness.get_by_label("History");
    assert!(history.accesskit_node().is_disabled());
    history.click();
    harness.run();
    assert_eq!(harness.state().0, 1);
    assert!(harness.state().2.is_empty());
}

#[test]
fn the_options_menu_hides_tab_names_and_opens_customise() {
    let mut harness = tab_bar(440.0);
    harness.run();
    harness.get_by_label("Tab options").click();
    harness.run();
    harness.get_by_label("Show tab names").click();
    harness.run();
    assert_eq!(harness.state().2, vec![TabBarAction::ShowNames(false)]);
    assert!(!harness.state().1);
    harness.get_by_label("Tab options").click();
    harness.run();
    harness.get_by_label("Customise tabs…").click();
    harness.run();
    assert_eq!(harness.state().2.last(), Some(&TabBarAction::Customise));
}

/// » is a pull-down of the popup service: a list of menu items, a tab that cannot be chosen
/// disabled and saying it is to come, the keys and the focus as every popup has them.
#[test]
fn the_more_menu_is_a_popup_list_and_gives_the_focus_back() {
    let mut harness = tab_bar(220.0);
    harness.run();
    // A pull-down's button says it has a menu and whether it is open (`pull_down`).
    let expanded = |harness: &Harness<'_, _>| {
        let data = harness
            .get_by_role_and_label(Role::Button, "More tabs")
            .accesskit_node();
        assert_eq!(data.data().has_popup(), Some(HasPopup::Menu));
        data.data().is_expanded()
    };
    assert_eq!(expanded(&harness), Some(false));
    // Focused as a key left it, so the list can give the focus back.
    harness.get_by_label("More tabs").focus();
    harness.run();
    harness.get_by_label("More tabs").click();
    harness.run();
    assert_eq!(expanded(&harness), Some(true));
    let menu = harness.get_by_role(Role::Menu);
    assert_eq!(menu.accesskit_node().label().as_deref(), Some("More tabs"));
    let effect = harness.get_by_role_and_label(Role::MenuItem, "Effect");
    assert!(!effect.accesskit_node().is_disabled());
    let history = harness.get_by_role_and_label(Role::MenuItem, "History");
    assert!(history.accesskit_node().is_disabled());
    // The keys: the first line that can be chosen has the focus, ↓ stays on the one, Esc
    // closes the list with the focus back on the button.
    assert_eq!(focused(&harness), "Colour");
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert_eq!(focused(&harness), "Correct");
    harness
        .get_by_role_and_label(Role::MenuItem, "History")
        .hover();
    harness.run();
    assert!(
        harness
            .query_by_label("History: coming in a later version")
            .is_some()
    );
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert_eq!(focused(&harness), "More tabs");
    assert_eq!(expanded(&harness), Some(false));
    assert!(harness.state().2.is_empty());
    // Opened again, Enter chooses its first line.
    harness.get_by_label("More tabs").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().2, vec![TabBarAction::Select(2)]);
    assert!(harness.query_by_role(Role::Menu).is_none());
}

/// ⋮ holds items only: Show tab names is an item with a check mark, not a checkbox.
#[test]
fn the_options_menu_is_a_popup_list_with_a_checked_item() {
    let mut harness = tab_bar(440.0);
    harness.run();
    harness.get_by_label("Tab options").click();
    harness.run();
    assert!(harness.query_by_role(Role::CheckBox).is_none());
    let names = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Show tab names");
    assert_eq!(names.accesskit_node().toggled(), Some(Toggled::True));
    let customise = harness.get_by_role_and_label(Role::MenuItem, "Customise tabs…");
    assert!(!customise.accesskit_node().is_disabled());
    assert_eq!(focused(&harness), "Show tab names");
    // Choosing it toggles the setting and closes the menu.
    names.click();
    harness.run();
    assert_eq!(harness.state().2, vec![TabBarAction::ShowNames(false)]);
    assert!(harness.query_by_role(Role::Menu).is_none());
    // Opened again, it is unchecked; Esc gives the focus back to the button.
    harness.get_by_label("Tab options").focus();
    harness.run();
    harness.get_by_label("Tab options").click();
    harness.run();
    let names = harness.get_by_role_and_label(Role::MenuItemCheckBox, "Show tab names");
    assert_eq!(names.accesskit_node().toggled(), Some(Toggled::False));
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_role(Role::Menu).is_none());
    assert_eq!(focused(&harness), "Tab options");
    assert_eq!(harness.state().2.len(), 1);
}

#[test]
fn customise_tabs_is_a_disabled_item_while_it_cannot_run() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(440.0, 240.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            |ui, chosen: &mut Vec<TabBarAction>| {
                let tabs = [PanelTab {
                    icon: Icon::Base,
                    name: "Base",
                    shortcut: None,
                    enabled: true,
                }];
                if let Some(action) = TabBar::new(&tabs, 0, true).customise(false).show(ui) {
                    chosen.push(action);
                }
            },
            Vec::new(),
        );
    harness.run();
    harness.get_by_label("Tab options").click();
    harness.run();
    let customise = harness.get_by_role_and_label(Role::MenuItem, "Customise tabs…");
    assert!(customise.accesskit_node().is_disabled());
    customise.click();
    harness.run();
    assert!(harness.state().is_empty());
}

/// Edit's tabs, as the app has them (decision 38).
const TAB_IDS: [(Icon, &str); 6] = [
    (Icon::Folder, "tab-photos"),
    (Icon::Base, "tab-base"),
    (Icon::Color, "tab-color"),
    (Icon::Correct, "tab-correct"),
    (Icon::Effect, "tab-effect"),
    (Icon::History, "tab-history"),
];

/// The side panel's tabs, named in `locale`, in a panel wide enough to show all six. A
/// disabled tab's name is measured and drawn as any other's, so every tab here can be
/// chosen.
fn named_tabs(locale: Locale) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(440.0, 120.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, locale);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui(|ui| {
            let names: Vec<String> = TAB_IDS
                .iter()
                .map(|(_, id)| apertrust_ui::tr(ui.ctx(), id))
                .collect();
            let tabs: Vec<PanelTab> = TAB_IDS
                .iter()
                .zip(&names)
                .map(|((icon, _), name)| PanelTab {
                    icon: *icon,
                    name,
                    shortcut: None,
                    enabled: true,
                })
                .collect();
            TabBar::new(&tabs, 1, true).show(ui);
        })
}

/// The text the tab bar drew for the tab called `name`.
fn drawn_name<State>(harness: &Harness<'_, State>, name: &str) -> TextShape {
    harness
        .output()
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if text.galley.text() == name => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} is not drawn"))
}

#[test]
fn long_tab_names_fit_their_tiles_and_keep_their_full_names() {
    let mut harness = named_tabs(Locale::Pseudo);
    harness.run();
    let translator = apertrust_ui::Translator::new(Locale::Pseudo);
    for (_, id) in TAB_IDS {
        let name = translator.text(id);
        let tile = harness.get_by_role_and_label(Role::Tab, &name).rect();
        let text = drawn_name(&harness, &name);
        // Shrunk to fit whole, not shortened: at 9 points, the most the names may shrink,
        // as `docs/gui.md` (Translation) says.
        assert!(!text.galley.elided, "{name} is shortened");
        let size = text.galley.job.sections[0].format.font_id.size;
        let caption = Theme::default().text.caption;
        assert_eq!(size, caption - 2.0, "{name}");
        let drawn = text.visual_bounding_rect();
        assert!(
            drawn.left() >= tile.left() && drawn.right() <= tile.right(),
            "{name} is drawn over {drawn:?}, wider than its tile {tile:?}"
        );
        // The full name in the tooltip, as well as the tab's for screen readers.
        harness.get_by_role_and_label(Role::Tab, &name).hover();
        harness.run();
        assert!(
            harness
                .query_all_by_label(&name)
                .any(|node| node.accesskit_node().role() != Role::Tab),
            "no tooltip says {name}"
        );
    }
}

#[test]
fn a_name_too_long_to_shrink_into_its_tile_ends_in_an_ellipsis() {
    let name = "A tab whose name no tile could hold";
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 120.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(move |ui| {
            let tabs = [
                PanelTab {
                    icon: Icon::Base,
                    name,
                    shortcut: None,
                    enabled: true,
                },
                PanelTab {
                    icon: Icon::Color,
                    name: "Colour",
                    shortcut: None,
                    enabled: true,
                },
            ];
            TabBar::new(&tabs, 1, true).show(ui);
        });
    harness.run();
    let tile = harness.get_by_role_and_label(Role::Tab, name).rect();
    let drawn = drawn_name(&harness, name);
    assert!(drawn.galley.elided);
    let drawn = drawn.visual_bounding_rect();
    assert!(drawn.left() >= tile.left() && drawn.right() <= tile.right());
    // Its neighbour shrinks with it, and fits whole.
    let colour = drawn_name(&harness, "Colour");
    assert!(!colour.galley.elided);
    assert!(colour.galley.job.sections[0].format.font_id.size < Theme::default().text.caption);
}

#[test]
fn tab_names_that_fit_keep_the_theme_s_size() {
    let mut harness = named_tabs(Locale::EnGb);
    harness.run();
    let caption = Theme::default().text.caption;
    for name in ["Photos", "Base", "Colour", "Correct", "Effect", "History"] {
        let drawn = drawn_name(&harness, name);
        assert!(!drawn.galley.elided, "{name} is shortened");
        let format = &drawn.galley.job.sections[0].format;
        assert_eq!(format.font_id.size, caption, "{name}");
        assert_eq!(format.extra_letter_spacing, 0.0, "{name}");
    }
}

/// The longest Ukrainian module title, Highlight reconstruction's, in a section at the side
/// panel's narrowest, 240 points, with every button a header has: cut at its end before
/// the buttons, and whole in its tooltip and to screen readers.
#[test]
fn a_long_title_is_cut_before_the_header_s_buttons_at_the_narrowest_side_panel() {
    let words = apertrust_ui::Translator::new(Locale::Uk);
    let title = words.text("module-highlights-title");
    let shown = title.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(280.0, 120.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::default());
            apertrust_ui::set_locale(ctx, Locale::Uk);
        })
        .build_ui(move |ui| {
            ui.allocate_ui(egui::vec2(240.0, 100.0), |ui| {
                Section::new(&shown, false).module().show(ui, |_| {});
            });
        });
    harness.run();
    // Screen readers hear it whole.
    let toggle = harness.get_by_role_and_label(Role::Button, &title).rect();
    let drawn = drawn_name(&harness, &title);
    assert!(drawn.galley.elided, "{title} fits whole");
    let mut args = apertrust_ui::i18n::Args::new();
    args.set("section", &title);
    let reset = harness
        .get_by_role_and_label(Role::Button, &words.text_with("section-reset", Some(&args)))
        .rect();
    let end = drawn.visual_bounding_rect().right();
    assert!(
        end <= toggle.right() && toggle.right() <= reset.left(),
        "{end} before {toggle:?} before {reset:?}"
    );
    // The tooltip has it whole, on a line of its own before what the header does.
    harness.get_by_role_and_label(Role::Button, &title).hover();
    harness.run();
    assert!(
        harness
            .query_by_label_contains(&format!("{title}\n"))
            .is_some(),
        "no tooltip says {title}"
    );
}

/// Ukrainian's and Polish's names, set as written, fit whole, shrinking no further than
/// they need: Ukrainian's at the theme's size, and Polish's at the most the rule allows,
/// two points less, for darktable's Podstawowe (decision 38).
#[test]
fn ukrainian_and_polish_names_shrink_no_further_than_they_need() {
    let caption = Theme::default().text.caption;
    for (locale, size) in [(Locale::Uk, caption), (Locale::Pl, caption - 2.0)] {
        let mut harness = named_tabs(locale);
        harness.run();
        let translator = apertrust_ui::Translator::new(locale);
        for (_, id) in TAB_IDS {
            let name = translator.text(id);
            let tile = harness.get_by_role_and_label(Role::Tab, &name).rect();
            let drawn = drawn_name(&harness, &name);
            assert!(!drawn.galley.elided, "{name} is shortened");
            let format = &drawn.galley.job.sections[0].format;
            assert_eq!(format.font_id.size, size, "{name}");
            assert_eq!(format.extra_letter_spacing, 0.0, "{name}");
            let drawn = drawn.visual_bounding_rect();
            assert!(
                drawn.left() >= tile.left() && drawn.right() <= tile.right(),
                "{name} is drawn over {drawn:?}, wider than its tile {tile:?}"
            );
        }
    }
}

/// An open module of one toggle, with the tabs asked for, in a 400-point window.
fn rhythm(tabs: &'static [&'static str]) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(400.0, 200.0))
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui(move |ui| {
            // A module panel stacks its rows with no item spacing.
            ui.spacing_mut().item_spacing.y = 0.0;
            Section::new("Exposure", true)
                .module()
                .tabs(tabs, 0)
                .show(ui, |ui| {
                    apertrust_ui::components::ToggleRow::new("Use camera bias", true).show(ui);
                });
        })
}

/// The module header is 30 points (`sizes.section_header`) and all of it is the click
/// target; the buttons are centred in it.
#[test]
fn the_header_is_thirty_points_and_clickable_all_over() {
    let t = Theme::default();
    assert_eq!(t.sizes.section_header, 30.0);
    let mut harness = module(false);
    harness.run();
    let toggle = harness
        .get_by_role_and_label(Role::Button, "White balance")
        .rect();
    assert_eq!(toggle.height(), t.sizes.section_header);
    let reset = harness.get_by_label("Reset White balance").rect();
    assert!(
        (reset.center().y - toggle.center().y).abs() < 0.01,
        "{reset:?} in {toggle:?}"
    );
    for y in [toggle.top() + 0.5, toggle.bottom() - 0.5] {
        let at = egui::pos2(toggle.left() + 40.0, y);
        harness.hover_at(at);
        harness.run();
        harness.drag_at(at);
        harness.drop_at(at);
        harness.run();
        harness.state_mut().open = false;
    }
    assert_eq!(harness.state().actions, vec![SectionAction::Toggle; 2]);
}

/// Without tabs the first row starts 4 points under the header (`spacing.xs`).
#[test]
fn a_module_without_tabs_starts_four_points_under_its_header() {
    let t = Theme::default();
    let mut harness = rhythm(&[]);
    harness.run();
    let header = harness
        .get_by_role_and_label(Role::Button, "Exposure")
        .rect();
    let row = harness
        .get_by_role_and_label(Role::CheckBox, "Use camera bias")
        .rect();
    // The checkbox is centred in its 24-point row.
    let first = row.center().y - t.sizes.panel_line / 2.0;
    assert!(
        (first - (header.bottom() + t.spacing.xs)).abs() < 0.01,
        "{header:?} then {row:?}"
    );
}

/// With tabs the strip is a row high (26) right under the header, and the first name is
/// on the content's 12-point edge.
#[test]
fn the_tab_strip_is_a_row_high_and_its_first_name_is_on_the_content_edge() {
    let t = Theme::default();
    let mut harness = rhythm(&["Basic", "Advanced"]);
    harness.run();
    let header = harness
        .get_by_role_and_label(Role::Button, "Exposure")
        .rect();
    let strip = harness.get_by_role(Role::TabList).rect();
    assert_eq!(strip.height(), t.sizes.row);
    assert!(
        (strip.top() - header.bottom()).abs() < 0.01,
        "{header:?} {strip:?}"
    );
    let edge = header.left() - t.spacing.xxs + t.spacing.l;
    let left = drawn_name(&harness, "Basic").pos.x;
    assert!((left - edge).abs() < 0.01, "{left} against the edge {edge}");
}

/// A section holding a [`section_list`] of `lines` lines, then a line after it, recording
/// the section body's rectangle.
fn listed(lines: usize) -> Harness<'static, egui::Rect> {
    Harness::builder()
        .with_size(egui::vec2(320.0, 700.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::default()))
        .build_ui_state(
            move |ui, body: &mut egui::Rect| {
                PanelBody::new().show(ui, |ui| {
                    Section::new("Listed", true).show(ui, |ui| {
                        *body = ui.max_rect();
                        section_list(ui, "lines", |ui| {
                            for i in 0..lines {
                                ui.label(format!("Line {i}"));
                            }
                        });
                    });
                    ui.label("After the section");
                });
            },
            egui::Rect::NOTHING,
        )
}

/// A list inside a section keeps the section's height whatever it holds and scrolls; its
/// scroll bar shows only with the pointer over the list, and then in the body's end margin.
#[test]
fn a_section_list_keeps_its_height_scrolls_and_floats_its_bar_in_the_end_margin() {
    let mut short = listed(3);
    short.run();
    let mut long = listed(60);
    long.run();
    let end =
        |harness: &Harness<'_, egui::Rect>| harness.get_by_label("After the section").rect().top();
    assert_eq!(end(&short), end(&long));
    // The list is `section_list` high, the body's bottom margin and the hairline after it.
    let t = apertrust_ui::theme(&long.ctx);
    let first = long.get_by_label("Line 0").rect();
    let tall = end(&long) - first.top();
    assert!(tall >= t.sizes.section_list, "{tall}");
    assert!(tall < t.sizes.section_list + 2.0 * t.spacing.l, "{tall}");

    // Painted in the body's end margin, with some colour: the bar.
    let body = *long.state();
    let margin = egui::Rangef::new(body.right(), body.right() + Section::body_margin(&t));
    let bar = |harness: &Harness<'_, egui::Rect>| {
        harness
            .output()
            .shapes
            .iter()
            .any(|shape| match &shape.shape {
                egui::Shape::Rect(rect) => {
                    rect.fill.a() > 0
                        && rect.rect.width() > 0.0
                        && margin.contains(rect.rect.left())
                        && margin.contains(rect.rect.right())
                }
                _ => false,
            })
    };
    assert!(!bar(&long), "hidden with the pointer away");
    long.hover_at(first.center());
    long.run_steps(20);
    assert!(bar(&long), "shown with the pointer over the list");
    // A list that fits has none.
    short.hover_at(short.get_by_label("Line 0").rect().center());
    short.run_steps(20);
    assert!(!bar(&short));

    // The lines scroll under the wheel, the section staying as it was.
    assert!(long.get_by_label("Line 59").rect().top() > end(&long));
    long.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -4000.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    long.run_steps(60);
    assert!(long.get_by_label("Line 59").rect().bottom() < end(&long));
    assert_eq!(end(&long), end(&short));
}
