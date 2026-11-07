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

//! Interaction tests for the histogram and the other scopes, driven as a person or a screen
//! reader would and found through AccessKit (`docs/gui.md`, Testing), and a photograph of
//! them in use.

use apertrust_ui::components::{
    Channel, Clipped, Histogram, HistogramAction, HistogramBins, Scale, Scope, ScopeView,
    VectorscopeBins, WaveformBins,
};
use apertrust_ui::{Builtin, Locale, Theme};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Rect, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

/// The scopes, the app's view of them, and what they reported, applied as the app would.
struct Scopes {
    view: ScopeView,
    clipped: Option<Clipped>,
    /// Whether the caller has a vectorscope to give, and whether it can measure one.
    vectorscope: bool,
    can_measure_vectorscope: bool,
    /// Whether the app draws the clipping overlays.
    overlays: bool,
    actions: Vec<HistogramAction>,
}

/// A small measured photo: each channel a hump, red the darkest and blue the brightest,
/// rising across the photo, and its colours leaning one way.
struct Data {
    histogram: [Vec<f32>; 4],
    waveform: [Vec<f32>; 4],
    vectorscope: Vec<f32>,
}

const BINS: usize = 64;
const COLUMNS: usize = 32;
const LEVELS: usize = 32;
const GRID: usize = 24;

fn hump(x: f32, at: f32, width: f32) -> f32 {
    (-((x - at) / width).powi(2)).exp()
}

fn data() -> Data {
    let offset = |k: usize| [0.0, 0.08, 0.16, 0.06][k];
    let bins = |k: usize| -> Vec<f32> {
        (0..BINS)
            .map(|i| (1000.0 * hump((i as f32 + 0.5) / BINS as f32, 0.3 + offset(k), 0.15)).round())
            .collect()
    };
    let wave = |k: usize| -> Vec<f32> {
        (0..COLUMNS * LEVELS)
            .map(|i| {
                let (column, level) = (i / LEVELS, i % LEVELS);
                let rise = 0.15 + 0.6 * column as f32 / COLUMNS as f32 + offset(k);
                (40.0 * hump(level as f32 / LEVELS as f32, rise, 0.06)).floor()
            })
            .collect()
    };
    let vectorscope = (0..GRID * GRID)
        .map(|i| {
            let (x, y) = (
                (i % GRID) as f32 / GRID as f32,
                (i / GRID) as f32 / GRID as f32,
            );
            (50.0 * hump(x, 0.6, 0.1) * hump(y, 0.4, 0.08)).floor()
        })
        .collect();
    Data {
        histogram: [bins(0), bins(1), bins(2), bins(3)],
        waveform: [wave(0), wave(1), wave(2), wave(3)],
        vectorscope,
    }
}

fn scopes_in(view: ScopeView, locale: Locale, theme: Builtin) -> Harness<'static, Scopes> {
    let data = data();
    Harness::builder()
        .with_size(vec2(360.0, 160.0))
        .with_context_setup(move |ctx| {
            apertrust_ui::install(ctx, Theme::builtin(theme));
            apertrust_ui::set_locale(ctx, locale);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui_state(
            move |ui, state: &mut Scopes| {
                let [red, green, blue, luminance] = &data.histogram;
                let [w_red, w_green, w_blue, w_luminance] = &data.waveform;
                let mut scopes = Histogram::new(state.view)
                    .histogram(HistogramBins {
                        red,
                        green,
                        blue,
                        luminance,
                    })
                    .waveform(WaveformBins {
                        columns: COLUMNS,
                        levels: LEVELS,
                        red: w_red,
                        green: w_green,
                        blue: w_blue,
                        luminance: w_luminance,
                    })
                    .overlays(state.overlays);
                if let Some(clipped) = state.clipped {
                    scopes = scopes.clipped(clipped);
                }
                if !state.can_measure_vectorscope {
                    scopes = scopes.available(&[Scope::Histogram, Scope::Waveform, Scope::Parade]);
                }
                if state.vectorscope {
                    scopes = scopes.vectorscope(VectorscopeBins {
                        size: GRID,
                        counts: &data.vectorscope,
                        colours: &[],
                    });
                }
                if let Some(action) = scopes.show(ui) {
                    state.view.apply(action);
                    state.actions.push(action);
                }
            },
            Scopes {
                view,
                clipped: Some(Clipped {
                    shadows: 0.0012,
                    highlights: 0.004,
                }),
                vectorscope: true,
                can_measure_vectorscope: true,
                overlays: true,
                actions: Vec::new(),
            },
        )
}

fn scopes() -> Harness<'static, Scopes> {
    let mut harness = scopes_in(ScopeView::default(), Locale::EnGb, Builtin::Dark);
    harness.run();
    harness
}

/// Whether anything is drawn inside the control called `name`: hidden, it takes its place
/// in the focus order but draws nothing.
fn drawn(harness: &Harness<'_, Scopes>, name: &str) -> bool {
    let rect = harness
        .query_all_by_label(name)
        .find(|node| node.accesskit_node().role() != Role::Image)
        .unwrap_or_else(|| panic!("no control is called {name}"))
        .rect();
    harness.output().shapes.iter().any(|clipped| {
        let bounds = clipped.shape.visual_bounding_rect();
        bounds.is_positive() && rect.expand(1.0).contains_rect(bounds)
    })
}

fn graph(harness: &Harness<'_, Scopes>) -> Rect {
    harness.get_by_role(Role::Image).rect()
}

fn toggled(harness: &Harness<'_, Scopes>, name: &str) -> Option<Toggled> {
    harness.get_by_label(name).accesskit_node().data().toggled()
}

#[test]
fn the_controls_show_while_the_pointer_is_over_the_graph() {
    let mut harness = scopes();
    assert!(!drawn(&harness, "Waveform"));
    assert!(!drawn(&harness, "Show highlight clipping"));
    harness.hover_at(graph(&harness).center());
    harness.run();
    for name in [
        "Histogram",
        "Waveform",
        "Logarithmic scale",
        "Red channel",
        "Luminance",
        "Show shadow clipping",
        "Show highlight clipping",
    ] {
        assert!(drawn(&harness, name), "{name}");
    }
    harness.event(egui::Event::PointerGone);
    harness.run();
    assert!(!drawn(&harness, "Waveform"));
}

#[test]
fn tab_reaches_every_control_in_order_and_shows_them() {
    let mut harness = scopes();
    let mut reached = Vec::new();
    for _ in 0..11 {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness
            .query_by(|node| node.is_focused())
            .expect("something has focus");
        reached.push(focused.accesskit_node().label().unwrap_or_default());
        assert!(
            drawn(&harness, "Waveform"),
            "hidden with {reached:?} focused"
        );
    }
    assert_eq!(
        reached,
        [
            "Histogram",
            "Waveform",
            "RGB parade",
            "Vectorscope",
            "Logarithmic scale",
            "Red channel",
            "Green channel",
            "Blue channel",
            "Luminance",
            "Show shadow clipping",
            "Show highlight clipping",
        ]
    );
    // Space presses the one with focus.
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![HistogramAction::HighlightClipping(true)]
    );
}

#[test]
fn the_graph_is_chosen_with_its_button() {
    let mut harness = scopes();
    harness
        .get_by_role_and_label(Role::RadioButton, "Waveform")
        .click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![HistogramAction::Scope(Scope::Waveform)]
    );
    assert_eq!(
        harness
            .get_by_role(Role::Image)
            .accesskit_node()
            .label()
            .unwrap(),
        "Waveform"
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::RadioButton, "Waveform")
            .accesskit_node()
            .data()
            .toggled(),
        Some(Toggled::True)
    );
    // The scale is the histogram's; the parade has no luminance; the vectorscope no
    // channels.
    assert!(harness.query_by_label("Logarithmic scale").is_none());
    harness
        .get_by_role_and_label(Role::RadioButton, "RGB parade")
        .click();
    harness.run();
    assert!(harness.query_by_label("Red channel").is_some());
    assert!(harness.query_by_label("Luminance").is_none());
    harness
        .get_by_role_and_label(Role::RadioButton, "Vectorscope")
        .click();
    harness.run();
    assert!(harness.query_by_label("Red channel").is_none());
    assert_eq!(harness.state().view.scope, Scope::Vectorscope);
}

#[test]
fn a_graph_the_app_cannot_measure_cannot_be_chosen() {
    let mut harness = scopes_in(ScopeView::default(), Locale::EnGb, Builtin::Dark);
    harness.state_mut().can_measure_vectorscope = false;
    harness.run();
    harness.get_by_label("Vectorscope").click();
    harness.run();
    assert!(harness.state().actions.is_empty());
    assert!(
        harness
            .get_by_label("Vectorscope")
            .accesskit_node()
            .is_disabled()
    );
}

/// The app measures only the graph shown, so one not measured yet must still be chosen.
#[test]
fn a_graph_not_measured_yet_can_be_chosen_and_says_so() {
    let mut harness = scopes_in(ScopeView::default(), Locale::EnGb, Builtin::Dark);
    harness.state_mut().vectorscope = false;
    harness.run();
    harness.get_by_label("Vectorscope").click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![HistogramAction::Scope(Scope::Vectorscope)]
    );
    let image = harness.get_by_role(Role::Image);
    assert_eq!(
        image.accesskit_node().description().unwrap(),
        "Nothing measured yet"
    );
    let said = harness
        .output()
        .shapes
        .iter()
        .any(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => text.galley.text() == "Nothing measured yet",
            _ => false,
        });
    assert!(said, "the graph does not say that nothing is measured");
}

#[test]
fn the_clipping_toggles_wait_for_the_overlays() {
    let mut harness = scopes_in(ScopeView::default(), Locale::EnGb, Builtin::Dark);
    harness.state_mut().overlays = false;
    harness.run();
    for name in ["Show shadow clipping", "Show highlight clipping"] {
        harness.get_by_label(name).click();
        harness.run();
        assert!(harness.get_by_label(name).accesskit_node().is_disabled());
    }
    assert!(harness.state().actions.is_empty());
}

#[test]
fn the_clipping_toggles_report_through_the_output() {
    let mut harness = scopes();
    assert_eq!(
        toggled(&harness, "Show highlight clipping"),
        Some(Toggled::False)
    );
    harness.get_by_label("Show highlight clipping").click();
    harness.run();
    harness.get_by_label("Show shadow clipping").click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![
            HistogramAction::HighlightClipping(true),
            HistogramAction::ShadowClipping(true)
        ]
    );
    assert_eq!(
        toggled(&harness, "Show highlight clipping"),
        Some(Toggled::True)
    );
    harness.get_by_label("Show highlight clipping").click();
    harness.run();
    assert_eq!(
        harness.state().actions.last(),
        Some(&HistogramAction::HighlightClipping(false))
    );
}

#[test]
fn the_scale_switches_between_linear_and_logarithmic() {
    let mut harness = scopes();
    harness.get_by_label("Logarithmic scale").click();
    harness.run();
    assert_eq!(harness.state().view.scale, Scale::Logarithmic);
    assert_eq!(toggled(&harness, "Logarithmic scale"), Some(Toggled::True));
    harness.get_by_label("Logarithmic scale").click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![
            HistogramAction::Scale(Scale::Logarithmic),
            HistogramAction::Scale(Scale::Linear)
        ]
    );
}

#[test]
fn channels_show_and_hide() {
    let mut harness = scopes();
    assert_eq!(toggled(&harness, "Red channel"), Some(Toggled::True));
    assert_eq!(toggled(&harness, "Luminance"), Some(Toggled::False));
    harness.get_by_label("Red channel").click();
    harness.run();
    harness.get_by_label("Luminance").click();
    harness.run();
    assert_eq!(
        harness.state().actions,
        vec![
            HistogramAction::Channel(Channel::Red, false),
            HistogramAction::Channel(Channel::Luminance, true)
        ]
    );
    assert!(!harness.state().view.shows(Channel::Red));
}

#[test]
fn screen_readers_hear_the_graph_and_what_clips() {
    let mut harness = scopes();
    let image = harness.get_by_role(Role::Image);
    assert_eq!(image.accesskit_node().label().unwrap(), "Histogram");
    assert_eq!(
        image.accesskit_node().description().unwrap(),
        "Shadows clipped: 0.12%, highlights clipped: 0.4%"
    );
    harness.state_mut().clipped = Some(Clipped::default());
    harness.run();
    assert_eq!(
        harness
            .get_by_role(Role::Image)
            .accesskit_node()
            .description()
            .unwrap(),
        "No clipping"
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::RadioGroup, "Graph")
            .children_recursive()
            .filter(|node| node.accesskit_node().role() == Role::RadioButton)
            .count(),
        4
    );
}

/// The four graphs with the pointer over the histogram's scale, its tooltip open, in the
/// pseudo-locale.
#[test]
fn the_scopes_in_use_in_the_pseudo_locale() {
    let mut harness = Harness::builder()
        .with_size(vec2(600.0, 320.0))
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            apertrust_ui::set_locale(ctx, Locale::Pseudo);
            ctx.global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        })
        .build_ui(|ui| {
            let data = data();
            let [red, green, blue, luminance] = &data.histogram;
            let [w_red, w_green, w_blue, w_luminance] = &data.waveform;
            let panel = apertrust_ui::theme(ui.ctx()).colors.panel;
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, panel);
            let width = (ui.available_width() - 12.0) / 2.0;
            for pair in [
                [Scope::Histogram, Scope::Waveform],
                [Scope::Parade, Scope::Vectorscope],
            ] {
                ui.horizontal_top(|ui| {
                    for scope in pair {
                        let view = ScopeView {
                            scope,
                            channels: [true, true, true, scope == Scope::Histogram],
                            highlight_clipping: true,
                            ..ScopeView::default()
                        };
                        ui.allocate_ui(vec2(width, 0.0), |ui| {
                            Histogram::new(view)
                                .histogram(HistogramBins {
                                    red,
                                    green,
                                    blue,
                                    luminance,
                                })
                                .waveform(WaveformBins {
                                    columns: COLUMNS,
                                    levels: LEVELS,
                                    red: w_red,
                                    green: w_green,
                                    blue: w_blue,
                                    luminance: w_luminance,
                                })
                                .vectorscope(VectorscopeBins {
                                    size: GRID,
                                    counts: &data.vectorscope,
                                    colours: &[],
                                })
                                .clipped(Clipped {
                                    shadows: 0.0012,
                                    highlights: 0.004,
                                })
                                .id_salt(scope)
                                .overlays(true)
                                .show_controls(true)
                                .show(ui);
                        });
                    }
                });
            }
        });
    harness.run();
    let name = apertrust_ui::Translator::new(Locale::Pseudo).text("scope-log-scale");
    let scale = harness.get_by_label(&name).rect();
    harness.hover_at(scale.center());
    harness.run();
    harness.snapshot("scopes-in-use-pseudo");
}
