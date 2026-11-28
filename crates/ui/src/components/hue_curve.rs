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

//! The hue curve: the colour equalizer's graph of one curve's nodes, as darktable draws it
//! over its sliders (`src/iop/colorequal.c`, `_iop_colorequalizer_draw` and the `_area_*`
//! callbacks), in Apertrust's theme. The nodes lie evenly spaced along the hue circle
//! across the graph, the first where the caller says ([`HueCurve::offset`]), the first
//! drawn again a circle on; each sits at its value, the graph's height spanning its range,
//! with a bar from the ground, the range's middle, to it. Through them the curve the
//! caller gives, the one the operation renders; behind them what the caller paints, and
//! under the graph its hue strip.
//!
//! It knows no hue, no interpolation and no colour: the caller gives the nodes, the curve
//! sampled over one circle from the first node, where that node sits, and the colours.
//!
//! - **Pointer:** as darktable's (`_area_motion_notify_callback`,
//!   `_area_button_press_callback`), the node nearest the pointer across is the one it is
//!   on while within 10 points of its height, and filled. A press on it and a drag move it
//!   to the pointer's height, within the graph. A double-click resets it, or every node
//!   where the pointer is on none (`_area_reset_nodes`), as the tooltip says. The wheel
//!   steps the nearest node as its slider's wheel does (`_area_scrolled_callback`).
//! - **Keys:** Tab focuses the graph. Page Up and Page Down choose the previous and the
//!   next node, Home and End the first and the last; the arrow keys move the chosen one by
//!   its step (Shift ten steps, Ctrl a tenth, Option on the Mac), up and right raising it,
//!   never by less than the last decimal shown; Delete resets it and Shift with Delete
//!   resets them all. Other keys with Ctrl, and the arrows with another of the app's
//!   modifiers, are left to the app.
//! - **Screen readers:** the graph is a group named by its full name, described by its
//!   keys; each node is a slider named by its node, the chosen one active.
//!
//! A drag reports every change and once more, marked done, on release, so history merges
//! it into one step.

use egui::accesskit::{Action, ActionData};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::os::OperatingSystem;
use egui::{
    Color32, EventFilter, Id, Key, Pos2, Rect, Role, Sense, Shape, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::curve::pressed_once;
use super::wheel::{Wheel, key_scale, notches};
use super::{focus_ring, focus_shows};
use crate::i18n::Args;
use crate::{theme, translator};

/// The graph's height over its width.
const ASPECT: f32 = 0.5;
/// darktable's nodes (`_iop_colorequalizer_draw`): rings of radius 4, 3 wide; its bars 6
/// wide and its curve 2.
const NODE_RADIUS: f32 = 4.0;
const NODE_STROKE: f32 = 3.0;
const BAR_WIDTH: f32 = 6.0;
const CURVE_WIDTH: f32 = 2.0;
/// How near a node's height the pointer is on it: darktable's 10 points.
const ON_NODE: f32 = 10.0;
/// darktable's grid, 8 × 8 (`dt_draw_grid`).
const GRID: usize = 8;
/// One key press, unless the caller sets it.
const STEP: f32 = 1.0;
/// How dim a disabled graph is, as a disabled slider row.
const DISABLED: f32 = 0.6;

/// What the user did to the graph this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum HueCurveAction {
    /// The node, counted from 0, the first first, to `value`: a drag's every change and
    /// once more, `done`, on release; a key's, the wheel's or a screen reader's step, done.
    Move { node: usize, value: f32, done: bool },
    /// The node back to its default.
    ResetNode(usize),
    /// Every node back to its default.
    Reset,
}

pub struct HueCurve<'a> {
    name: &'a str,
    nodes: &'a [f32],
    names: &'a [&'a str],
    range: [f32; 2],
    offset: f32,
    curve: Drawn<'a>,
    strip: Option<&'a [Color32]>,
    background: Option<(&'a [Color32], usize)>,
    unit: Option<&'a str>,
    step: f32,
    decimals: usize,
    wheel: Wheel,
    enabled: bool,
}

impl<'a> HueCurve<'a> {
    /// `name` is the graph's full, translated name ("Colour equalizer, curve"): what screen
    /// readers say, and with the first node's its id. `nodes` are their values, the first
    /// first, each named by `names`, its node's full name; `range` is what each can take,
    /// the graph's height, its ground in the middle.
    pub fn new(name: &'a str, nodes: &'a [f32], names: &'a [&'a str], range: [f32; 2]) -> Self {
        HueCurve {
            name,
            nodes,
            names,
            range,
            offset: 0.0,
            curve: Drawn::Lines,
            strip: None,
            background: None,
            unit: None,
            step: STEP,
            decimals: 2,
            wheel: Wheel::Off,
            enabled: true,
        }
    }

    /// The curve through the nodes, at evenly spaced hues over one circle from the first
    /// node to it again, at least two; without it the nodes are joined by straight lines.
    pub fn curve(mut self, samples: &'a [f32]) -> Self {
        if samples.len() >= 2 {
            self.curve = Drawn::Samples(samples);
        }
        self
    }

    /// No curve at all, as where the tool's code failed to give one.
    pub fn no_curve(mut self) -> Self {
        self.curve = Drawn::Nothing;
        self
    }

    /// Where the first node sits across the graph, a share of its width, 0 unless set: the
    /// colour equalizer's Node placement over 360°.
    pub fn offset(mut self, offset: f32) -> Self {
        if offset.is_finite() {
            self.offset = offset;
        }
        self
    }

    /// The strip under the graph, colours evenly spaced from its start to its end; none
    /// draws no strip.
    pub fn strip(mut self, colours: &'a [Color32]) -> Self {
        if colours.len() >= 2 {
            self.strip = Some(colours);
        }
        self
    }

    /// The graph's background, a grid's corners row by row from the top, `columns` a row;
    /// none, or one that is no grid, paints the sunken ground.
    pub fn background(mut self, colours: &'a [Color32], columns: usize) -> Self {
        if columns >= 2 && colours.len() >= 2 * columns && colours.len().is_multiple_of(columns) {
            self.background = Some((colours, columns));
        }
        self
    }

    /// The message that speaks a node's value with its unit, `$value` its number
    /// ("unit-degrees-value"); none speaks the number alone.
    pub fn unit(mut self, message: &'a str) -> Self {
        self.unit = Some(message);
        self
    }

    /// How far one press of an arrow key or a notch of the wheel moves a node: 1 unless set.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// The decimals the nodes show, two unless set: no key, notch or screen reader's step
    /// moves a node by less than the last of them, as a slider row's never does.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Where the graph takes the scroll wheel, as a slider row does; never unless set.
    pub fn wheel(mut self, wheel: Wheel) -> Self {
        self.wheel = wheel;
        self
    }

    /// A disabled graph is dimmed and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<HueCurveAction> {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let first = self.names.first().copied().unwrap_or_default();
        let id = ui.make_persistent_id(("hue curve", self.name, first));
        let n = self.nodes.len();
        let [low, high] = self.range;
        if n < 2 || low.partial_cmp(&high) != Some(std::cmp::Ordering::Less) {
            return None;
        }
        // A value that is not a number reads as the ground; none lies past the graph.
        let ground = (low + high) / 2.0;
        let within = |v: f32| {
            if v.is_finite() {
                v.clamp(low, high)
            } else {
                ground
            }
        };
        let nodes: Vec<f32> = self.nodes.iter().map(|&v| within(v)).collect();

        // The graph across the whole width, as darktable aligns it with the sliders; the
        // hue strip under it.
        let width = ui.available_width();
        let graph_height = width * ASPECT;
        let strip_gap = t.spacing.xs;
        let strip_height = t.spacing.m;
        let (outer, _) = ui.allocate_exact_size(
            vec2(width, graph_height + strip_gap + strip_height),
            Sense::hover(),
        );
        if width <= 0.0 {
            return None;
        }
        let plot = Plot {
            area: Rect::from_min_size(outer.min, vec2(width, graph_height)),
            offset: self.offset,
            n,
            range: self.range,
        };

        let mut state: State = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        state.selected = state.selected.filter(|&s| s < n);
        state.drag = state.drag.filter(|drag| drag.node < n);
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let response = ui.interact(plot.area, id, sense);
        let down = response.is_pointer_button_down_on();
        let pressed = down && ui.input(|i| i.pointer.primary_pressed());
        let mut actions = Vec::new();
        // The node the pointer is nearest across, and whether it is on it.
        let hover = ui
            .input(|i| i.pointer.interact_pos())
            .filter(|&p| plot.area.contains(p))
            .map(|p| {
                let node = plot.nearest(p.x);
                (node, (plot.y_at(nodes[node]) - p.y).abs() < ON_NODE)
            });

        // The pointer: a press on a node starts a drag of it, which sets it to the
        // pointer's height; the release ends it.
        if self.enabled {
            if pressed {
                if !response.has_focus() {
                    response.request_focus();
                }
                state.drag = match hover {
                    Some((node, true)) => {
                        state.selected = Some(node);
                        Some(Drag {
                            node,
                            value: nodes[node],
                            moved: false,
                        })
                    }
                    _ => None,
                };
            }
            if response.double_clicked() {
                state.drag = None;
                actions.push(match hover {
                    Some((node, true)) => HueCurveAction::ResetNode(node),
                    _ => HueCurveAction::Reset,
                });
            } else if let Some(drag) = state.drag.as_mut() {
                if let Some(at) = ui.input(|i| i.pointer.interact_pos()) {
                    let value = plot.value_at(at.y);
                    if value != drag.value {
                        drag.value = value;
                        drag.moved = true;
                        actions.push(drag.action(false));
                    }
                }
                if !down {
                    // One action a frame: the step this frame took ends the gesture with it.
                    actions.clear();
                    if drag.moved {
                        actions.push(drag.action(true));
                    }
                    state.drag = None;
                }
            }
            // The wheel steps the node nearest the pointer, as its slider's wheel does;
            // `notches` runs every frame, as a slider's does, so it drops what was left of
            // a notch once the pointer leaves the graph.
            let turned = notches(ui, id, plot.area, self.wheel, state.drag.is_none());
            if let (Some((node, _)), Some(turned)) = (hover, turned) {
                let value = turned
                    .steps
                    .iter()
                    .fold(nodes[node], |value, &(notches, steps)| {
                        within(value + notches as f32 * self.nudge(steps as f32 * self.step))
                    });
                state.selected = Some(node);
                if value != nodes[node] {
                    actions.push(HueCurveAction::Move {
                        node,
                        value,
                        done: true,
                    });
                }
                response.request_focus();
            }
        } else {
            state.drag = None;
        }

        // Keys: only those the app leaves a focused control, none with Ctrl but the arrows'
        // tenth of a step (`docs/gui.md`, Keyboard and actions).
        if self.enabled && response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            });
            let current = state.selected.unwrap_or(0);
            let os = ui.ctx().os();
            let (choose, rise, reset, shift) = ui.input(|i| {
                let command = i.modifiers.command || i.modifiers.ctrl;
                let scale = key_scale(os, i.modifiers);
                let mut rise = 0.0;
                for (key, steps) in [
                    (Key::ArrowUp, 1.0),
                    (Key::ArrowRight, 1.0),
                    (Key::ArrowDown, -1.0),
                    (Key::ArrowLeft, -1.0),
                ] {
                    if let Some(scale) = scale
                        && i.key_pressed(key)
                    {
                        rise += steps * scale as f32 * self.step;
                    }
                }
                let choose = if command {
                    None
                } else if i.key_pressed(Key::Home) {
                    Some(0)
                } else if i.key_pressed(Key::End) {
                    Some(n - 1)
                } else if i.key_pressed(Key::PageUp) {
                    Some(current.saturating_sub(1))
                } else if i.key_pressed(Key::PageDown) {
                    Some((current + 1).min(n - 1))
                } else {
                    None
                };
                // Once a press, as the curve resets.
                let reset =
                    !command && (pressed_once(i, Key::Delete) || pressed_once(i, Key::Backspace));
                (choose, rise, reset, i.modifiers.shift)
            });
            if choose.is_some() || rise != 0.0 || reset {
                state.selected = Some(choose.unwrap_or(current));
            }
            let chosen = state.selected.unwrap_or(current);
            if reset {
                state.drag = None;
                actions.push(if shift {
                    HueCurveAction::Reset
                } else {
                    HueCurveAction::ResetNode(chosen)
                });
            } else if rise != 0.0 && state.drag.is_none() {
                // Not during a drag, as the wheel: its release would report the drag's
                // value after the key's.
                let value = within(nodes[chosen] + self.nudge(rise));
                if value != nodes[chosen] {
                    actions.push(HueCurveAction::Move {
                        node: chosen,
                        value,
                        done: true,
                    });
                }
            }
        }

        // What screen readers hear: the graph as a group of sliders, one per node, the
        // chosen one active; its description says what the keys do.
        let keys = tr.text(if ui.ctx().os() == OperatingSystem::Mac {
            "hue-curve-keys-mac"
        } else {
            "hue-curve-keys"
        });
        response.widget_info(|| WidgetInfo::labeled(Role::Group, self.enabled, self.name));
        let active = state.selected.map(|s| node_id(id, s).accesskit_id());
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_description(keys);
            if let Some(active) = active {
                node.set_active_descendant(active);
            }
        });
        let reach = NODE_RADIUS + NODE_STROKE;
        let nodes_ui = ui.new_child(UiBuilder::new().max_rect(outer).accessibility_parent(id));
        for (i, &value) in nodes.iter().enumerate() {
            let centre = pos2(plot.x_of(i as i32), plot.y_at(value));
            let node = nodes_ui.interact(
                Rect::from_center_size(centre, Vec2::splat(2.0 * reach)),
                node_id(id, i),
                Sense::hover(),
            );
            let name = self.names.get(i).copied().unwrap_or_default();
            node.widget_info(|| WidgetInfo::slider(self.enabled, f64::from(value), name));
            let number = tr.format_number(f64::from(value), self.decimals, true);
            let spoken = match self.unit {
                Some(unit) => {
                    let mut args = Args::new();
                    args.set("value", number);
                    tr.text_with(unit, Some(&args))
                }
                None => number,
            };
            let step = f64::from(self.step);
            ui.ctx().accesskit_node_builder(node.id, |node| {
                node.set_value(spoken);
                node.set_min_numeric_value(f64::from(low));
                node.set_max_numeric_value(f64::from(high));
                node.set_numeric_value_step(step);
                node.add_action(Action::SetValue);
                node.add_action(Action::Increment);
                node.add_action(Action::Decrement);
            });
            if self.enabled
                && let Some(to) = asked(ui, node.id, value, self.step, |d| self.nudge(d))
                && within(to) != value
            {
                state.selected = Some(i);
                actions.push(HueCurveAction::Move {
                    node: i,
                    value: within(to),
                    done: true,
                });
            }
        }

        // Drawing.
        if ui.is_rect_visible(outer) {
            let fade = |c: Color32| {
                if self.enabled {
                    c
                } else {
                    c.gamma_multiply(DISABLED)
                }
            };
            let painter = ui.painter_at(outer);
            let focused = response.has_focus();
            let dragging = state.drag.is_some();
            let on = hover.filter(|&(_, on)| on && self.enabled).map(|(i, _)| i);
            let filled =
                |i: usize| on == Some(i) || ((focused || dragging) && state.selected == Some(i));

            match self.background {
                Some((colours, columns)) => {
                    painter.add(grid_mesh(plot.area, colours, columns, &fade));
                }
                None => {
                    painter.rect_filled(plot.area, 0.0, t.colors.sunken);
                }
            }
            let grid = Stroke::new(t.shape.hairline, t.colors.divider);
            for k in 1..GRID {
                let f = k as f32 / GRID as f32;
                painter.vline(
                    plot.area.left() + f * plot.area.width(),
                    plot.area.y_range(),
                    grid,
                );
                painter.hline(
                    plot.area.x_range(),
                    plot.area.top() + f * plot.area.height(),
                    grid,
                );
            }
            painter.rect_stroke(plot.area, 0.0, grid, StrokeKind::Inside);
            painter.hline(
                plot.area.x_range(),
                plot.y_at(ground),
                Stroke::new(t.shape.hairline, fade(t.colors.text_secondary)),
            );

            // The curve, the bars and the nodes, inside the graph as darktable clips them.
            let inside = painter.with_clip_rect(plot.area);
            if !matches!(self.curve, Drawn::Nothing) {
                let columns = plot.area.width().ceil().max(2.0) as usize;
                let line: Vec<Pos2> = (0..=columns)
                    .map(|k| {
                        let x = plot.area.left() + plot.area.width() * k as f32 / columns as f32;
                        pos2(x, plot.y_at(within(self.height_at(&nodes, &plot, x))))
                    })
                    .collect();
                inside.line(line, Stroke::new(CURVE_WIDTH, fade(t.colors.text)));
            }
            // Each node where it lies, and again a circle either side, as darktable draws
            // the first at both ends.
            let drawn: Vec<(usize, Pos2)> = (-1..=n as i32)
                .map(|k| (k.rem_euclid(n as i32) as usize, k))
                .map(|(i, k)| (i, pos2(plot.x_of(k), plot.y_at(nodes[i]))))
                .filter(|(_, p)| {
                    (p.x - plot.area.center().x).abs() <= plot.area.width() / 2.0 + reach
                })
                .collect();
            for &(_, p) in &drawn {
                inside.line_segment(
                    [pos2(p.x, plot.y_at(ground)), p],
                    Stroke::new(BAR_WIDTH, fade(t.colors.slider_fill)),
                );
            }
            for &(i, p) in &drawn {
                // Filled with the ground, as darktable's, which stays opaque when dimmed.
                let fill = if filled(i) {
                    fade(t.colors.text)
                } else {
                    t.colors.sunken
                };
                inside.circle(
                    p,
                    NODE_RADIUS,
                    fill,
                    Stroke::new(NODE_STROKE, fade(t.colors.text)),
                );
                if focused && state.selected == Some(i) && focus_shows(painter.ctx()) {
                    painter.circle_stroke(
                        p,
                        NODE_RADIUS + NODE_STROKE / 2.0 + 3.0,
                        Stroke::new(1.5, t.colors.focus_ring),
                    );
                }
            }
            // With no node chosen yet, the graph itself shows the focus.
            if focused && state.selected.is_none() {
                focus_ring(&painter, plot.area, &t);
            }
            if let Some(strip) = self.strip {
                let under = Rect::from_min_size(
                    pos2(plot.area.left(), plot.area.bottom() + strip_gap),
                    vec2(plot.area.width(), strip_height),
                );
                let colours: Vec<Color32> = strip.iter().chain(strip).copied().collect();
                painter.add(grid_mesh(under, &colours, strip.len(), &fade));
            }
        }

        if !actions.is_empty() {
            // The caller applies what the graph reported; draw it on the next frame.
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| d.insert_temp(id, state));
        response.on_hover_text(tr.text("equalizer-tip"));

        // One action a frame: resetting wins, then a node's reset, then a move.
        actions.sort_by_key(|a| match a {
            HueCurveAction::Reset => 0,
            HueCurveAction::ResetNode(_) => 1,
            HueCurveAction::Move { .. } => 2,
        });
        actions.into_iter().next()
    }

    /// A step's move, never less than the last decimal shown, as a slider row's.
    fn nudge(&self, delta: f32) -> f32 {
        let resolution = 10f32.powi(-(self.decimals as i32));
        if delta != 0.0 && delta.abs() < resolution {
            resolution.copysign(delta)
        } else {
            delta
        }
    }

    /// The curve's value at `x` across the graph: the caller's samples over the circle from
    /// the first node, or straight lines between the nodes.
    fn height_at(&self, nodes: &[f32], plot: &Plot, x: f32) -> f32 {
        let circle = plot.circle_at(x);
        let (samples, periodic) = match self.curve {
            Drawn::Samples(samples) => (samples, false),
            Drawn::Lines | Drawn::Nothing => (nodes, true),
        };
        // Straight lines run from the last node back to the first.
        let last = if periodic {
            samples.len()
        } else {
            samples.len() - 1
        };
        let at = circle * last as f32;
        let i = (at as usize).min(last - 1);
        let next = samples[(i + 1) % samples.len()];
        samples[i] + (next - samples[i]) * (at - i as f32)
    }
}

/// The graph's area, and how hues and values map onto it.
struct Plot {
    area: Rect,
    /// Where the first node sits across, a share of the width.
    offset: f32,
    n: usize,
    range: [f32; 2],
}

impl Plot {
    /// Where node `k` lies across, `k` counted around the circle, so `n` is the first again.
    fn x_of(&self, k: i32) -> f32 {
        self.area.left() + (self.offset + k as f32 / self.n as f32) * self.area.width()
    }

    /// How far round the circle from the first node `x` lies, from 0 to 1.
    fn circle_at(&self, x: f32) -> f32 {
        ((x - self.area.left()) / self.area.width() - self.offset).rem_euclid(1.0)
    }

    /// The node nearest `x` across, darktable's `(x − x₀) / spacing + 0.5` round the circle.
    fn nearest(&self, x: f32) -> usize {
        let spacing = self.area.width() / self.n as f32;
        (((x - self.x_of(0)) / spacing).round() as i32).rem_euclid(self.n as i32) as usize
    }

    /// The range's top at the graph's top, its bottom at its foot.
    fn y_at(&self, value: f32) -> f32 {
        let [low, high] = self.range;
        self.area.top() + (high - value) / (high - low) * self.area.height()
    }

    /// The value at height `y`, within the range.
    fn value_at(&self, y: f32) -> f32 {
        let [low, high] = self.range;
        let share = ((y - self.area.top()) / self.area.height()).clamp(0.0, 1.0);
        high - share * (high - low)
    }
}

/// The curve the graph draws.
#[derive(Clone, Copy)]
enum Drawn<'a> {
    /// The caller's samples, evenly spaced over the circle from the first node to it again.
    Samples(&'a [f32]),
    /// Straight lines between the nodes, round the circle.
    Lines,
    Nothing,
}

/// What the graph keeps between frames, in egui's memory. Never a value the caller owns.
#[derive(Clone, Debug, Default)]
struct State {
    /// The node the keys act on.
    selected: Option<usize>,
    drag: Option<Drag>,
}

/// A drag under way: the node, and the value it has moved it to, unrounded, whatever the
/// caller rounds what it shows to.
#[derive(Clone, Debug)]
struct Drag {
    node: usize,
    value: f32,
    moved: bool,
}

impl Drag {
    fn action(&self, done: bool) -> HueCurveAction {
        HueCurveAction::Move {
            node: self.node,
            value: self.value,
            done,
        }
    }
}

/// A screen reader's step of node `id`, moved as `nudge` moves a key's, or the value it
/// typed.
fn asked(ui: &Ui, id: Id, value: f32, step: f32, nudge: impl Fn(f32) -> f32) -> Option<f32> {
    let (more, less, typed) = ui.input(|i| {
        (
            i.num_accesskit_action_requests(id, Action::Increment),
            i.num_accesskit_action_requests(id, Action::Decrement),
            i.accesskit_action_requests(id, Action::SetValue)
                .filter_map(|request| match request.data {
                    Some(ActionData::NumericValue(v)) => Some(v),
                    _ => None,
                })
                .last(),
        )
    });
    if let Some(typed) = typed {
        Some(typed as f32)
    } else if more != less {
        Some(value + nudge((more as f32 - less as f32) * step))
    } else {
        None
    }
}

fn node_id(graph: Id, index: usize) -> Id {
    graph.with(("node", index))
}

/// `rect` filled from a grid of colours at its corners, row by row from the top, `columns`
/// a row, each blended across its cell: colours that are data, not tokens.
fn grid_mesh(
    rect: Rect,
    colours: &[Color32],
    columns: usize,
    fade: &impl Fn(Color32) -> Color32,
) -> Shape {
    let rows = colours.len() / columns;
    let mut mesh = Mesh::default();
    for (k, &colour) in colours.iter().enumerate() {
        let (row, column) = (k / columns, k % columns);
        let pos = pos2(
            rect.left() + rect.width() * column as f32 / (columns - 1) as f32,
            rect.top() + rect.height() * row as f32 / (rows - 1) as f32,
        );
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color: fade(colour),
        });
    }
    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let a = (row * columns + column) as u32;
            let (b, c, d) = (a + 1, a + columns as u32 + 1, a + columns as u32);
            mesh.add_triangle(a, b, c);
            mesh.add_triangle(a, c, d);
        }
    }
    Shape::mesh(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plot(offset: f32) -> Plot {
        Plot {
            area: Rect::from_min_size(pos2(0.0, 10.0), vec2(320.0, 160.0)),
            offset,
            n: 8,
            range: [-180.0, 180.0],
        }
    }

    #[test]
    fn the_graph_maps_darktable_s_axes() {
        let plot = plot(0.0);
        assert_eq!(plot.x_of(0), 0.0);
        assert_eq!(plot.x_of(2), 80.0);
        assert_eq!(plot.x_of(8), 320.0);
        assert_eq!(plot.y_at(180.0), 10.0);
        assert_eq!(plot.y_at(0.0), 90.0);
        assert_eq!(plot.y_at(-180.0), 170.0);
        assert_eq!(plot.value_at(50.0), 90.0);
        // Past the graph, its ends.
        assert_eq!(plot.value_at(0.0), 180.0);
        assert_eq!(plot.value_at(500.0), -180.0);
    }

    #[test]
    fn the_nearest_node_wraps_round_the_circle() {
        let plot = plot(0.0);
        assert_eq!(plot.nearest(19.0), 0);
        assert_eq!(plot.nearest(21.0), 1);
        // Near the end, the first node drawn again.
        assert_eq!(plot.nearest(310.0), 0);
        assert_eq!(plot.nearest(290.0), 7);
    }

    #[test]
    fn node_placement_shifts_the_nodes_and_the_curve() {
        // A placement of −23°: the first node just before the start, drawn again just
        // before the end.
        let plot = plot(-23.0 / 360.0);
        assert!((plot.x_of(0) + 320.0 * 23.0 / 360.0).abs() < 1e-3);
        assert_eq!(plot.nearest(305.0), 0);
        assert!((plot.circle_at(plot.x_of(0))).abs() < 1e-6);
        assert!((plot.circle_at(plot.x_of(4)) - 0.5).abs() < 1e-6);
    }
}
