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

//! Interaction tests for the equalizer, the tone equalizer's graph of its bands, driven as
//! a person or a screen reader would and found through AccessKit (`docs/gui.md`, Testing),
//! and a photograph of it under the pointer.

use apertrust_ui::components::{Equalizer, EqualizerAction, MaskHistogram, Settle};
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Event, Key, Modifiers, Pos2, Rect, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const NAME: &str = "Tone equalizer, curve";
const BANDS: [&str; 9] = [
    "−8 EV", "−7 EV", "−6 EV", "−5 EV", "−4 EV", "−3 EV", "−2 EV", "−1 EV", "+0 EV",
];

/// The graph's bands, its smoothing and histogram, and what it reported, applied as the
/// app would.
struct Graph {
    bands: Vec<f32>,
    smoothing: f32,
    /// The bins over −8 to 0 EV, and the mask's 5th and 95th percentiles.
    histogram: Option<(Vec<f32>, [f32; 2])>,
    /// Where a drag's bands settle after each step, as the module's curve gives them.
    settle: Option<Box<Settle<'static>>>,
    actions: Vec<EqualizerAction>,
}

impl Graph {
    fn apply(&mut self, action: EqualizerAction) {
        match &action {
            EqualizerAction::Move { to, .. } => self.bands.clone_from(to),
            EqualizerAction::ResetBand(i) => self.bands[*i] = 0.0,
            EqualizerAction::Reset => self.bands = vec![0.0; 9],
        }
        self.actions.push(action);
    }
}

fn graph_with(bands: [f32; 9], setup: impl FnOnce(&mut Graph)) -> Harness<'static, Graph> {
    let mut state = Graph {
        bands: bands.to_vec(),
        smoothing: std::f32::consts::SQRT_2,
        histogram: None,
        settle: None,
        actions: Vec::new(),
    };
    setup(&mut state);
    let mut harness = Harness::builder()
        .with_size(vec2(320.0, 240.0))
        // Frames a sixtieth of a second apart, so two clicks make a double-click (0.3 s).
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark)))
        .build_ui_state(
            |ui, graph: &mut Graph| {
                ui.set_width(296.0);
                let histogram = graph
                    .histogram
                    .as_ref()
                    .map(|(bins, percentiles)| MaskHistogram {
                        bins,
                        range: [-8.0, 0.0],
                        percentiles: *percentiles,
                    });
                let mut equalizer = Equalizer::new(NAME, &graph.bands, &BANDS)
                    .smoothing(graph.smoothing)
                    .histogram(histogram);
                if let Some(settle) = &graph.settle {
                    equalizer = equalizer.settle(settle.as_ref());
                }
                let action = equalizer.show(ui);
                if let Some(action) = action {
                    graph.apply(action);
                }
            },
            state,
        );
    harness.run();
    harness
}

fn graph(bands: [f32; 9]) -> Harness<'static, Graph> {
    graph_with(bands, |_| {})
}

/// The graph's plot, less the room its nodes take round it.
fn plot(harness: &Harness<'_, Graph>) -> Rect {
    harness
        .get_by_role_and_label(Role::Group, NAME)
        .rect()
        .shrink(7.0)
}

/// Where a band's exposure and a correction, both in EV, lie on the plot.
fn at(plot: Rect, exposure: f32, correction: f32) -> Pos2 {
    pos2(
        plot.left() + (exposure + 8.0) / 8.0 * plot.width(),
        plot.center().y - correction / 4.0 * plot.height(),
    )
}

/// Moves the pointer along `path`, the button down from its first point to its last.
fn drag(harness: &mut Harness<'_, Graph>, path: &[Pos2]) {
    harness.hover_at(path[0]);
    harness.step();
    harness.drag_at(path[0]);
    harness.step();
    for &p in &path[1..] {
        harness.hover_at(p);
        harness.step();
    }
    harness.drop_at(path[path.len() - 1]);
    harness.step();
}

/// darktable's falloff (`set_new_params_interactive`): a band `d` EV from the pointer's
/// exposure takes this much of the drag's rise, at smoothing σ.
fn falloff(d: f32, sigma: f32) -> f32 {
    let blending = sigma * sigma / 2.0;
    (-d * d / (2.0 * blending * blending)).exp()
}

fn close(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4)
}

#[test]
fn a_drag_moves_the_bands_around_the_pointer_by_darktable_s_falloff() {
    for sigma in [std::f32::consts::SQRT_2, 2.0, 0.8] {
        let mut harness = graph_with([0.0; 9], |graph| graph.smoothing = sigma);
        let plot = plot(&harness);
        // Up a quarter of the graph, 1 EV, from −2 EV.
        drag(&mut harness, &[at(plot, -2.0, 0.0), at(plot, -2.0, 1.0)]);
        let expected: Vec<f32> = (0..9)
            .map(|i| falloff(i as f32 - 6.0, sigma).min(2.0))
            .collect();
        let actions = &harness.state().actions;
        let [
            EqualizerAction::Move {
                to, done: false, ..
            },
            EqualizerAction::Move {
                from,
                to: last,
                done: true,
            },
        ] = &actions[..]
        else {
            panic!("σ {sigma}: {actions:?}");
        };
        assert!(close(to, &expected), "σ {sigma}: {to:?}");
        // The release ends the gesture, which began at the bands as they were.
        assert_eq!(last, to);
        assert_eq!(from, &vec![0.0; 9]);
    }
}

#[test]
fn each_step_of_a_drag_rises_at_the_exposure_the_pointer_left() {
    let mut harness = graph([0.0; 9]);
    let plot = plot(&harness);
    // Up 0.5 EV at −6 EV, across to −1 EV, then down 1 EV there: as darktable's motion
    // events, each rise is centred where the pointer was before it.
    drag(
        &mut harness,
        &[
            at(plot, -6.0, 0.0),
            at(plot, -6.0, 0.5),
            at(plot, -1.0, 0.5),
            at(plot, -1.0, -0.5),
        ],
    );
    let expected: Vec<f32> = (0..9)
        .map(|i| {
            let exposure = i as f32 - 8.0;
            0.5 * falloff(exposure + 6.0, std::f32::consts::SQRT_2)
                - falloff(exposure + 1.0, std::f32::consts::SQRT_2)
        })
        .collect();
    assert!(
        close(&harness.state().bands, &expected),
        "{:?}",
        harness.state().bands
    );
}

#[test]
fn each_step_of_a_drag_settles_the_bands_as_the_caller_says() {
    // As darktable sets the bands to its curve's correction at each node after each step:
    // here a stand-in that halves them, twice over two steps.
    let mut harness = graph_with([0.0; 9], |graph| {
        graph.settle = Some(Box::new(|bands| {
            Some(bands.iter().map(|b| b / 2.0).collect())
        }));
    });
    let plot = plot(&harness);
    drag(
        &mut harness,
        &[
            at(plot, -4.0, 0.0),
            at(plot, -4.0, 1.0),
            at(plot, -4.0, 2.0),
        ],
    );
    let expected: Vec<f32> = (0..9)
        .map(|i| {
            let f = falloff(i as f32 - 4.0, std::f32::consts::SQRT_2);
            (f / 2.0 + f) / 2.0
        })
        .collect();
    assert!(
        close(&harness.state().bands, &expected),
        "{:?}",
        harness.state().bands
    );
    // Where no curve fits the bands, the step is undone and the drag ends, as darktable's.
    let mut harness = graph_with([0.0; 9], |graph| graph.settle = Some(Box::new(|_| None)));
    drag(&mut harness, &[at(plot, -4.0, 0.0), at(plot, -4.0, 1.0)]);
    assert!(
        harness.state().actions.is_empty(),
        "{:?}",
        harness.state().actions
    );
    assert_eq!(harness.state().bands, vec![0.0; 9]);
}

#[test]
fn the_fit_takes_the_bands_as_raised_with_no_limit() {
    // darktable multiplies the gains by the falloff and fits them as they are; only its
    // curve's correction keeps the settled bands within ±2 EV.
    let mut harness = graph_with([0.0, 0.0, 0.0, 0.0, 1.8, 0.0, 0.0, 0.0, 0.0], |graph| {
        graph.settle = Some(Box::new(|bands| {
            Some(bands.iter().map(|b| b / 2.0).collect())
        }));
    });
    let plot = plot(&harness);
    drag(&mut harness, &[at(plot, -4.0, 0.0), at(plot, -4.0, 1.0)]);
    let band = harness.state().bands[4];
    assert!((band - 1.4).abs() < 1e-5, "{band}");
}

#[test]
fn leaving_the_graph_ends_the_drag() {
    // As darktable's `area_enter_leave_notify`: the drag is committed as it leaves, and a
    // move outside, or back in with the button still down, moves nothing more.
    let mut harness = graph([0.0; 9]);
    let plot = plot(&harness);
    let outside = pos2(plot.center().x, plot.bottom() + 40.0);
    drag(
        &mut harness,
        &[
            at(plot, -4.0, 0.0),
            at(plot, -4.0, 1.0),
            outside,
            at(plot, -4.0, -1.0),
        ],
    );
    let expected: Vec<f32> = (0..9)
        .map(|i| falloff(i as f32 - 4.0, std::f32::consts::SQRT_2))
        .collect();
    assert!(
        close(&harness.state().bands, &expected),
        "{:?}",
        harness.state().bands
    );
    let actions = &harness.state().actions;
    assert!(
        matches!(
            actions[..],
            [
                EqualizerAction::Move { done: false, .. },
                EqualizerAction::Move { done: true, .. }
            ]
        ),
        "{actions:?}"
    );
}

#[test]
fn the_last_move_before_the_release_counts() {
    // A move and the release in one frame, as darktable's last motion event before the
    // button's release: the step is taken, and ends the gesture.
    let mut harness = graph([0.0; 9]);
    let plot = plot(&harness);
    harness.hover_at(at(plot, -4.0, 0.0));
    harness.step();
    harness.drag_at(at(plot, -4.0, 0.0));
    harness.step();
    harness.drop_at(at(plot, -4.0, 1.0));
    harness.step();
    let actions = &harness.state().actions;
    let [EqualizerAction::Move { to, done: true, .. }] = &actions[..] else {
        panic!("{actions:?}");
    };
    assert!((to[4] - 1.0).abs() < 1e-5, "{to:?}");
}

#[test]
fn a_band_stays_within_two_stops() {
    let mut harness = graph([1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.8]);
    let plot = plot(&harness);
    drag(&mut harness, &[at(plot, -8.0, 0.0), at(plot, -8.0, 1.9)]);
    assert_eq!(harness.state().bands[0], 2.0);
    drag(&mut harness, &[at(plot, 0.0, 0.0), at(plot, 0.0, -1.9)]);
    assert_eq!(harness.state().bands[8], -2.0);
}

#[test]
fn a_double_click_resets_every_band() {
    let mut harness = graph([0.5, 0.0, -0.3, 0.0, 1.0, 0.0, 0.0, 0.2, 0.0]);
    for _ in 0..30 {
        harness.step();
    }
    for _ in 0..2 {
        harness.get_by_role_and_label(Role::Group, NAME).click();
        harness.step();
    }
    harness.step();
    assert_eq!(harness.state().actions, [EqualizerAction::Reset]);
    assert_eq!(harness.state().bands, vec![0.0; 9]);
}

fn press(harness: &mut Harness<'_, Graph>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.step();
}

/// The last move's band and its new value: a key moves one band, its move done.
fn moved(harness: &Harness<'_, Graph>) -> (usize, f32) {
    let Some(EqualizerAction::Move { from, to, done }) = harness.state().actions.last() else {
        panic!("{:?}", harness.state().actions);
    };
    assert!(*done);
    let changed: Vec<usize> = (0..9).filter(|&i| from[i] != to[i]).collect();
    assert_eq!(changed.len(), 1, "{from:?} to {to:?}");
    (changed[0], to[changed[0]])
}

#[test]
fn keys_choose_a_node_move_it_and_reset() {
    let mut harness = graph([0.0; 9]);
    harness.get_by_role_and_label(Role::Group, NAME).focus();
    harness.run();
    // The first node at first; up and right raise it by its step, Shift ten, Alt a tenth.
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(moved(&harness), (0, 0.01));
    press(&mut harness, Modifiers::NONE, Key::PageDown);
    press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    let (band, value) = moved(&harness);
    assert!(band == 1 && (value - 0.1).abs() < 1e-6, "{band}: {value}");
    // Alt's tenth, 0.001, is less than the last decimal shown, so it moves by that.
    press(&mut harness, Modifiers::ALT, Key::ArrowDown);
    let (band, value) = moved(&harness);
    assert!(band == 1 && (value - 0.09).abs() < 1e-6, "{band}: {value}");
    press(&mut harness, Modifiers::NONE, Key::End);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(moved(&harness), (8, -0.01));
    press(&mut harness, Modifiers::NONE, Key::PageUp);
    press(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(moved(&harness).0, 7);
    // Keys with Ctrl are the app's.
    let before = harness.state().actions.len();
    press(&mut harness, Modifiers::CTRL, Key::ArrowUp);
    assert_eq!(harness.state().actions.len(), before);
    // Delete resets the chosen node, Shift with it every node.
    press(&mut harness, Modifiers::NONE, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(
        harness.state().actions.last(),
        Some(&EqualizerAction::ResetBand(0))
    );
    press(&mut harness, Modifiers::SHIFT, Key::Delete);
    assert_eq!(
        harness.state().actions.last(),
        Some(&EqualizerAction::Reset)
    );
    assert_eq!(harness.state().bands, vec![0.0; 9]);
}

#[test]
fn screen_readers_hear_a_slider_for_each_band() {
    let mut harness = graph([0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, -1.25]);
    let graph = harness.get_by_role_and_label(Role::Group, NAME);
    let description = graph.accesskit_node().description().unwrap_or_default();
    assert!(
        description.starts_with("Page Up and Page Down"),
        "{description}"
    );
    for (i, name) in BANDS.iter().enumerate() {
        let node = harness.get_by_role_and_label(Role::Slider, name);
        let value = harness.state().bands[i];
        let node = node.accesskit_node();
        assert_eq!(node.numeric_value(), Some(f64::from(value)), "{name}");
        assert_eq!(node.min_numeric_value(), Some(-2.0));
        assert_eq!(node.max_numeric_value(), Some(2.0));
        assert!(node.data().supports_action(Action::Increment), "{name}");
    }
    let last = harness.get_by_role_and_label(Role::Slider, "+0 EV");
    assert_eq!(last.accesskit_node().value().as_deref(), Some("−1.25 EV"));
    // A screen reader's step moves its band by the band's step.
    let (target_node, target_tree) = harness
        .get_by_role_and_label(Role::Slider, "−4 EV")
        .accesskit_node()
        .locate();
    harness.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::Increment,
        target_node,
        target_tree,
        data: None,
    }));
    harness.run();
    assert_eq!(moved(&harness), (4, 0.51));
}

#[test]
fn the_description_says_where_the_mask_overflows_the_bands() {
    for (percentiles, said) in [
        ([-6.0, -1.0], None),
        (
            [-8.0, -1.0],
            Some("The mask's shadows go past the −8 EV band."),
        ),
        (
            [-6.0, -0.05],
            Some("The mask's highlights go past the +0 EV band."),
        ),
        (
            [-7.95, 0.0],
            Some(
                "The mask's shadows go past the −8 EV band, and its highlights past the +0 EV band.",
            ),
        ),
    ] {
        let harness = graph_with([0.0; 9], |graph| {
            graph.histogram = Some((vec![1.0; 256], percentiles));
        });
        let graph = harness.get_by_role_and_label(Role::Group, NAME);
        let description = graph.accesskit_node().description().unwrap_or_default();
        match said {
            Some(said) => assert!(description.starts_with(said), "{description}"),
            None => assert!(description.starts_with("Page Up"), "{description}"),
        }
    }
}

/// The graph under the pointer at −3.8 EV: the ring on the curve shows how far a drag
/// reaches, σ/√2, and the −4 EV node, within reach, is filled.
#[test]
fn under_the_pointer_the_ring_shows_the_reach() {
    let mut harness = graph([0.0, 0.0, 0.2, 0.6, 0.9, 0.6, 0.1, -0.4, -0.8]);
    let plot = plot(&harness);
    harness.hover_at(at(plot, -3.8, 0.0));
    harness.step();
    harness.snapshot("equalizer-hovered");
}
