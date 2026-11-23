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

//! Interaction tests for the hue curve, the colour equalizer's graph of a curve's nodes,
//! driven as a person or a screen reader would and found through AccessKit (`docs/gui.md`,
//! Testing). The Colour equalizer's own tests drive it on the tool.

use apertrust_ui::components::{HueCurve, HueCurveAction, Wheel, set_wheel_scrolls_panel};
use apertrust_ui::{Builtin, Theme};
use egui::accesskit::{Action, ActionRequest, Role};
use egui::{
    Event, Modifiers, MouseWheelSource, MouseWheelUnit, Pos2, Rect, TouchPhase, pos2, vec2,
};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

const NAME: &str = "Colour equalizer, curve";
const NODES: [&str; 8] = [
    "Hue: red",
    "Hue: orange",
    "Hue: yellow",
    "Hue: green",
    "Hue: cyan",
    "Hue: blue",
    "Hue: lavender",
    "Hue: magenta",
];
const RANGE: [f32; 2] = [-180.0, 180.0];

/// The graph's nodes and where the first sits, and what it reported, applied as the app
/// would.
struct Graph {
    nodes: Vec<f32>,
    offset: f32,
    enabled: bool,
    actions: Vec<HueCurveAction>,
}

impl Graph {
    fn apply(&mut self, action: HueCurveAction) {
        match &action {
            HueCurveAction::Move { node, value, .. } => self.nodes[*node] = *value,
            HueCurveAction::ResetNode(i) => self.nodes[*i] = 0.0,
            HueCurveAction::Reset => self.nodes = vec![0.0; 8],
        }
        self.actions.push(action);
    }
}

fn graph_with(nodes: [f32; 8], setup: impl FnOnce(&mut Graph)) -> Harness<'static, Graph> {
    let mut state = Graph {
        nodes: nodes.to_vec(),
        offset: 0.0,
        enabled: true,
        actions: Vec::new(),
    };
    setup(&mut state);
    let mut harness = Harness::builder()
        .with_size(vec2(320.0, 240.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| {
            apertrust_ui::install(ctx, Theme::builtin(Builtin::Dark));
            // As darktable's default, the wheel over the graph changes it plain.
            set_wheel_scrolls_panel(ctx, false);
        })
        .build_ui_state(
            |ui, graph: &mut Graph| {
                ui.set_width(296.0);
                let action = HueCurve::new(NAME, &graph.nodes, &NODES, RANGE)
                    .offset(graph.offset)
                    .unit("unit-degrees-value")
                    .wheel(Wheel::Pointer)
                    .enabled(graph.enabled)
                    .show(ui);
                if let Some(action) = action {
                    graph.apply(action);
                }
            },
            state,
        );
    harness.run();
    harness
}

fn graph(nodes: [f32; 8]) -> Harness<'static, Graph> {
    graph_with(nodes, |_| {})
}

fn plot(harness: &Harness<'_, Graph>) -> Rect {
    harness.get_by_role_and_label(Role::Group, NAME).rect()
}

/// Where node `k` lies at `value`, the first node at `offset` of the width.
fn at(plot: Rect, k: usize, offset: f32, value: f32) -> Pos2 {
    pos2(
        plot.left() + (offset + k as f32 / 8.0) * plot.width(),
        plot.top() + (RANGE[1] - value) / (RANGE[1] - RANGE[0]) * plot.height(),
    )
}

/// One notch of a mouse wheel at the pointer, up positive.
fn notch(harness: &mut Harness<'_, Graph>, up: f32) {
    let event = Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, up),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    };
    harness.event(event);
    harness.run_steps(12);
}

#[test]
fn a_drag_on_a_node_moves_it_to_the_pointer_and_ends_on_release() {
    let mut harness = graph([0.0; 8]);
    let plot = plot(&harness);
    let (from, to) = (at(plot, 2, 0.0, 0.0), at(plot, 2, 0.0, -90.0));
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.step();
    let actions = &harness.state().actions;
    let Some(HueCurveAction::Move { node, value, done }) = actions.last() else {
        panic!("{actions:?}");
    };
    assert!(
        *node == 2 && *done && (value + 90.0).abs() < 0.5,
        "{actions:?}"
    );
    assert!(
        actions[..actions.len() - 1].iter().all(|a| matches!(
            a,
            HueCurveAction::Move {
                node: 2,
                done: false,
                ..
            }
        )),
        "{actions:?}"
    );
}

#[test]
fn the_wheel_steps_the_node_nearest_the_pointer() {
    let mut harness = graph([0.0; 8]);
    let plot = plot(&harness);
    // Above blue, far from its height: the wheel takes the nearest node across.
    harness.hover_at(at(plot, 5, 0.0, 120.0));
    harness.run();
    notch(&mut harness, 1.0);
    notch(&mut harness, 1.0);
    notch(&mut harness, -1.0);
    assert_eq!(
        harness.state().actions,
        [
            HueCurveAction::Move {
                node: 5,
                value: 1.0,
                done: true
            },
            HueCurveAction::Move {
                node: 5,
                value: 2.0,
                done: true
            },
            HueCurveAction::Move {
                node: 5,
                value: 1.0,
                done: true
            },
        ]
    );
}

#[test]
fn node_placement_shifts_where_the_nodes_are_taken() {
    // A placement of +22.5°: red a sixteenth in.
    let mut harness = graph_with([0.0; 8], |graph| graph.offset = 22.5 / 360.0);
    let plot = plot(&harness);
    let (from, to) = (
        at(plot, 0, 22.5 / 360.0, 0.0),
        at(plot, 0, 22.5 / 360.0, 45.0),
    );
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.step();
    assert!(
        matches!(
            harness.state().actions.last(),
            Some(HueCurveAction::Move {
                node: 0,
                done: true,
                ..
            })
        ),
        "{:?}",
        harness.state().actions
    );
}

#[test]
fn screen_readers_hear_a_slider_for_each_node() {
    let mut harness = graph([0.0, 0.0, 0.0, 40.0, 0.0, 0.0, 0.0, 0.0]);
    let green = harness.get_by_role_and_label(Role::Slider, "Hue: green");
    let node = green.accesskit_node();
    assert_eq!(node.numeric_value(), Some(40.0));
    assert_eq!(node.min_numeric_value(), Some(-180.0));
    assert_eq!(node.max_numeric_value(), Some(180.0));
    assert_eq!(node.value().as_deref(), Some("+40.00°"));
    // Its Increment steps it, as a key would.
    let (target_node, target_tree) = green.accesskit_node().locate();
    harness.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::Increment,
        target_node,
        target_tree,
        data: None,
    }));
    harness.run();
    assert_eq!(
        harness.state().actions,
        [HueCurveAction::Move {
            node: 3,
            value: 41.0,
            done: true
        }]
    );
}

#[test]
fn a_disabled_graph_ignores_the_pointer_and_the_wheel() {
    let mut harness = graph_with([0.0; 8], |graph| graph.enabled = false);
    let plot = plot(&harness);
    let (from, to) = (at(plot, 2, 0.0, 0.0), at(plot, 2, 0.0, 90.0));
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.step();
    notch(&mut harness, 1.0);
    assert!(harness.state().actions.is_empty());
}
