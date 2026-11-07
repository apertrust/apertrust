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

//! The equalizer: the tone equalizer's graph of its bands, as darktable draws it at
//! the top of its Advanced tab (`src/iop/toneequal.c`, `_init_drawing` and `area_draw`),
//! in Apertrust's theme. Each band is a node over its exposure, 1 EV apart up to +0 EV,
//! −8 EV to +0 EV for darktable's nine, its correction from +2 EV at the top to −2 EV at
//! the bottom, with a bar from the ground, 0 EV, to it; through them the curve the caller
//! gives, the one the operation renders, in the warning colour when no curve fits them;
//! and behind them, once the engine has measured it, the mask's histogram, with
//! darktable's warning sign at the top right while the mask's 95th percentile lies above
//! −0.1 EV and at the top left while its 5th lies below −7.9 EV.
//!
//! It knows nothing of the fit: the caller gives the bands, the curve sampled over their
//! exposures, whether the fit failed, and the curve's smoothing σ, which sets how far a
//! drag reaches.
//!
//! - **Pointer:** as darktable's (`area_motion_notify`, `set_new_params_interactive`,
//!   `area_enter_leave_notify`), a drag anywhere on the graph moves the bands around the
//!   pointer's exposure: each step of the pointer moves every band by its rise, the graph's
//!   height being 4 EV, times a Gaussian of the band's distance from the exposure the
//!   pointer left, of variance σ⁴/4, then settles them on the curve that gives, as the
//!   caller says ([`Equalizer::settle`]), which keeps them within ±2 EV as darktable's
//!   `pixel_correction` does; without a settle each band is kept within ±2 EV itself. The
//!   pointer leaving the graph ends the drag, as the release does. Under the pointer a ring
//!   on the curve, of radius σ/√2, shows how far a drag reaches, and the node in reach of
//!   it is filled. A double-click resets every band, as the tooltip says.
//! - **Keys:** Tab focuses the graph. Page Up and Page Down choose the previous and the
//!   next node, Home and End the first and the last; the arrow keys move the chosen one by
//!   its step (Shift ten steps, Alt a tenth), up and right raising it, never by less than
//!   the last decimal shown; Delete resets it and Shift with Delete resets them all. Ctrl is
//!   left to the app.
//! - **Screen readers:** the graph is a group named by its full name, described by its
//!   keys and by where the mask overflows the bands, which darktable's sign says with no
//!   words; each node is a slider named by its band, the chosen one active.
//!
//! A drag reports every change and once more, marked done, on release, so history merges
//! it into one step.

use egui::accesskit::{Action, ActionData};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::os::OperatingSystem;
use egui::{
    Align2, Color32, EventFilter, FontId, Id, Key, Painter, Pos2, Rect, Role, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::curve::pressed_once;
use super::{focus_ring, focus_shows};
use crate::i18n::Args;
use crate::{Theme, Translator, theme, translator};

/// The correction the graph spans either way of its ground, EV: darktable's spans 4 EV.
const SPAN: f32 = 2.0;
/// The graph's height over its width: the wireframe's 160 over 256, at the side panel's
/// 320 points.
const ASPECT: f32 = 0.625;
/// darktable's nodes (`area_draw`): rings of radius 4, 3 wide; its bars 6 wide, its curve 3
/// and the ring under the pointer 1.5.
const NODE_RADIUS: f32 = 4.0;
const NODE_STROKE: f32 = 3.0;
const BAR_WIDTH: f32 = 6.0;
const CURVE_WIDTH: f32 = 3.0;
const RING_WIDTH: f32 = 1.5;
/// How close in x to a node the pointer must be to reach it: darktable's 0.45 of the
/// nodes' spacing.
const NEAR: f32 = 0.45;
/// The tallest bin's height, as a share of the graph's: darktable's 0.96.
const HISTOGRAM_TOP: f32 = 0.96;
/// Where darktable warns that the mask overflows the bands: its 95th percentile above
/// −0.1 EV, its 5th below −7.9 EV.
const OVERFLOWS_HIGH: f32 = -0.1;
const OVERFLOWS_LOW: f32 = -7.9;
/// The warning sign's side: darktable's two lines of text.
const WARNING: f32 = 18.0;
/// One key press, unless the caller sets it: darktable's bands' 0.01 EV.
const STEP: f32 = 0.01;
/// darktable's default smoothing, √2.
const SMOOTHING: f32 = std::f32::consts::SQRT_2;
/// How dim a disabled graph is, as a disabled slider row.
const DISABLED: f32 = 0.6;

/// The histogram behind the bands: what the engine measured of the mask, as plain data.
#[derive(Clone, Copy, Debug)]
pub struct MaskHistogram<'a> {
    /// Counts, evenly spaced over `range`, darkest first; the end bins hold what lies
    /// beyond it. The tallest reaches 0.96 of the graph.
    pub bins: &'a [f32],
    /// The exposures, in EV, of the first bin and the last: −8 and 0 for the mask.
    pub range: [f32; 2],
    /// The mask's 5th and 95th percentiles, in EV.
    pub percentiles: [f32; 2],
}

impl MaskHistogram<'_> {
    /// Whether the mask overflows the bands at the bottom and at the top, as darktable's
    /// warnings say.
    fn overflows(&self) -> (bool, bool) {
        let [low, high] = self.percentiles;
        (low < OVERFLOWS_LOW, high > OVERFLOWS_HIGH)
    }
}

/// What the user did to the graph this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum EqualizerAction {
    /// The bands move from `from`, where the gesture found them, to `to`: a drag's every
    /// change and once more, `done`, on release; a key's step, done.
    Move {
        from: Vec<f32>,
        to: Vec<f32>,
        done: bool,
    },
    /// The band, counted from 0, the darkest first, back to its default.
    ResetBand(usize),
    /// Every band back to its default.
    Reset,
}

/// Where a drag's bands settle after a step: the bands, or none where no curve fits them.
pub type Settle<'a> = dyn Fn(&[f32]) -> Option<Vec<f32>> + 'a;

pub struct Equalizer<'a> {
    name: &'a str,
    bands: &'a [f32],
    names: &'a [&'a str],
    curve: Drawn<'a>,
    settle: Option<&'a Settle<'a>>,
    failed: bool,
    smoothing: f32,
    histogram: Option<MaskHistogram<'a>>,
    step: f32,
    decimals: usize,
    enabled: bool,
}

impl<'a> Equalizer<'a> {
    /// `name` is the graph's full, translated name, unique in its module ("Tone equalizer,
    /// curve"): what screen readers say, and its id. `bands` are their corrections in EV,
    /// the darkest first, each a node named by `names`, its band's full name.
    pub fn new(name: &'a str, bands: &'a [f32], names: &'a [&'a str]) -> Self {
        Equalizer {
            name,
            bands,
            names,
            curve: Drawn::Lines,
            settle: None,
            failed: false,
            smoothing: SMOOTHING,
            histogram: None,
            step: STEP,
            decimals: 2,
            enabled: true,
        }
    }

    /// The curve through the bands, in EV, at evenly spaced exposures from the first
    /// band's to the last's, at least two; without it the bands are joined by straight
    /// lines.
    pub fn curve(mut self, samples: &'a [f32]) -> Self {
        if samples.len() >= 2 {
            self.curve = Drawn::Samples(samples);
        }
        self
    }

    /// No curve at all, as where the module's code failed to give one.
    pub fn no_curve(mut self) -> Self {
        self.curve = Drawn::Nothing;
        self
    }

    /// Where the bands settle after each step of a drag: darktable's
    /// `set_new_params_interactive` replaces them with the curve's correction at each node,
    /// the curve the bands moved by the step give, so they sit on the curve drawn. `None`
    /// from it where no curve fits them, as darktable's "the interpolation is unstable":
    /// the step is undone and the drag ends. Without it the bands stay as the step moved
    /// them.
    pub fn settle(mut self, settle: &'a Settle<'a>) -> Self {
        self.settle = Some(settle);
        self
    }

    /// Whether no curve fits the bands, as darktable's "the interpolation is unstable":
    /// the curve is drawn in the warning colour.
    pub fn failed(mut self, failed: bool) -> Self {
        self.failed = failed;
        self
    }

    /// The width σ of the Gaussians the curve is a sum of, in EV, darktable's √2 unless
    /// set: a drag moves the bands by a Gaussian of variance σ⁴/4, and the ring under the
    /// pointer has a radius of σ/√2.
    pub fn smoothing(mut self, sigma: f32) -> Self {
        if sigma.is_finite() && sigma > 0.0 {
            self.smoothing = sigma;
        }
        self
    }

    /// The mask's histogram behind the bands; none, as until the engine measures it,
    /// draws none and no warning.
    pub fn histogram(mut self, histogram: Option<MaskHistogram<'a>>) -> Self {
        self.histogram = histogram;
        self
    }

    /// How far one press of an arrow key moves a node, EV: 0.01 unless set.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// The decimals the bands show, two unless set: no key or screen reader's step moves a
    /// band by less than the last of them, as a slider row's never does, so Alt's tenth of
    /// a step still moves.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// A disabled graph is dimmed and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<EqualizerAction> {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let id = ui.make_persistent_id(("equalizer", self.name));
        let n = self.bands.len();
        // A value that is not a number reads as the ground; none lies past the graph.
        let bands: Vec<f32> = self.bands.iter().map(|&v| within(v)).collect();

        // The wireframe's layout at the side panel's 320 points, 296 wide: the labels across
        // the top, 20 high; the tone bar at the start, 8 wide, 8 before the graph; the
        // labels down the end, 24 wide; the tone bar under the graph, 8 below it.
        let width = ui.available_width();
        let top = t.spacing.xl + t.spacing.xs;
        let graph_width = (width - t.spacing.xl - t.spacing.xxl).max(0.0);
        let graph_height = graph_width * ASPECT;
        let (outer, _) = ui.allocate_exact_size(
            vec2(width, top + graph_height + 2.0 * t.spacing.m),
            Sense::hover(),
        );
        let plot = Plot {
            area: Rect::from_min_size(
                outer.min + vec2(t.spacing.xl, top),
                vec2(graph_width, graph_height),
            ),
            first: -(n.saturating_sub(1) as f32),
        };
        if n < 2 || graph_width <= 0.0 {
            return None;
        }

        let mut state: State = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        state.selected = state.selected.filter(|&s| s < n);
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let reach = NODE_RADIUS + NODE_STROKE;
        let response = ui.interact(plot.area.expand(reach), id, sense);
        let down = response.is_pointer_button_down_on();
        let pressed = down && ui.input(|i| i.pointer.primary_pressed());
        let mut actions = Vec::new();

        // The pointer: a press starts a drag from where it is, and each step of the pointer
        // moves the bands; the release ends it.
        if self.enabled {
            if pressed {
                if !response.has_focus() {
                    response.request_focus();
                }
                let origin = ui.input(|i| i.pointer.press_origin());
                state.drag = origin.map(|last| Drag {
                    last,
                    from: bands.clone(),
                    to: bands.clone(),
                    moved: false,
                    ended: false,
                });
            }
            if response.double_clicked() {
                state.drag = None;
                actions.push(EqualizerAction::Reset);
            } else if let Some(drag) = state.drag.as_mut() {
                // The pointer's last move before a release counts, as darktable's last
                // motion event before its button's release does; leaving the graph ends the
                // drag, as darktable's `area_enter_leave_notify` commits and ends it.
                let at = ui.input(|i| i.pointer.interact_pos());
                if let Some(at) = at
                    && !outer.contains(at)
                {
                    drag.ended = true;
                } else if let Some(at) = at
                    && at != drag.last
                {
                    let exposure = plot.exposure_at(drag.last.x);
                    let rise = (drag.last.y - at.y) / plot.area.height() * 2.0 * SPAN;
                    drag.last = at;
                    let before = drag.to.clone();
                    let clamped = self.settle.is_none();
                    if drag.raise(exposure, rise, self.smoothing, plot.first, clamped) {
                        match self
                            .settle
                            .map_or_else(|| Some(drag.to.clone()), |f| f(&drag.to))
                        {
                            Some(settled) if settled.len() == n => {
                                drag.to = settled.iter().map(|&v| within(v)).collect();
                                if drag.to != before {
                                    drag.moved = true;
                                    actions.push(drag.action(false));
                                }
                            }
                            // No curve fits the bands: as darktable, the step is undone and
                            // the drag ends.
                            _ => {
                                drag.to = before;
                                drag.ended = true;
                            }
                        }
                    }
                }
                if !down || drag.ended {
                    // One action a frame: the step this frame took ends the gesture with it.
                    actions.retain(|action| !matches!(action, EqualizerAction::Move { .. }));
                    if drag.moved {
                        actions.push(drag.action(true));
                    }
                    state.drag = None;
                }
            }
        } else {
            state.drag = None;
        }

        // Keys: only those the app leaves a focused control, none with Ctrl (`docs/gui.md`,
        // Keyboard and actions).
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
            let keys = ui.input(|i| {
                let command = i.modifiers.command || i.modifiers.ctrl;
                let scale = if i.modifiers.shift {
                    10.0
                } else if i.modifiers.alt {
                    0.1
                } else {
                    1.0
                };
                let mut rise = 0.0;
                for (key, steps) in [
                    (Key::ArrowUp, 1.0),
                    (Key::ArrowRight, 1.0),
                    (Key::ArrowDown, -1.0),
                    (Key::ArrowLeft, -1.0),
                ] {
                    if !command && i.key_pressed(key) {
                        rise += steps * scale * self.step;
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
            let (choose, rise, reset, shift) = keys;
            if choose.is_some() || rise != 0.0 || reset {
                state.selected = Some(choose.unwrap_or(current));
            }
            let chosen = state.selected.unwrap_or(current);
            if reset {
                actions.push(if shift {
                    EqualizerAction::Reset
                } else {
                    EqualizerAction::ResetBand(chosen)
                });
            } else if let Some(action) = set_band(&bands, chosen, bands[chosen] + self.nudge(rise))
            {
                actions.push(action);
            }
        }

        // What screen readers hear: the graph as a group of sliders, one per band, the
        // chosen one active; its description says what the keys do and where the mask
        // overflows the bands.
        let keys = tr.text(if ui.ctx().os() == OperatingSystem::Mac {
            "equalizer-keys-mac"
        } else {
            "equalizer-keys"
        });
        let overflow = self
            .histogram
            .and_then(|h| overflow_text(&tr, h.overflows()));
        let description = match overflow {
            Some(overflow) => format!("{overflow} {keys}"),
            None => keys,
        };
        response.widget_info(|| WidgetInfo::labeled(Role::Group, self.enabled, self.name));
        let active = state.selected.map(|s| node_id(id, s).accesskit_id());
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_description(description);
            if let Some(active) = active {
                node.set_active_descendant(active);
            }
        });
        let nodes_ui = ui.new_child(UiBuilder::new().max_rect(outer).accessibility_parent(id));
        for (i, &value) in bands.iter().enumerate() {
            let node = nodes_ui.interact(
                Rect::from_center_size(plot.node(i, value), Vec2::splat(2.0 * reach)),
                node_id(id, i),
                Sense::hover(),
            );
            let name = self.names.get(i).copied().unwrap_or_default();
            node.widget_info(|| WidgetInfo::slider(self.enabled, f64::from(value), name));
            let mut args = Args::new();
            args.set(
                "value",
                tr.format_number(f64::from(value), self.decimals, true),
            );
            let spoken = tr.text_with("unit-ev-value", Some(&args));
            let step = f64::from(self.step);
            ui.ctx().accesskit_node_builder(node.id, |node| {
                node.set_value(spoken);
                node.set_min_numeric_value(f64::from(-SPAN));
                node.set_max_numeric_value(f64::from(SPAN));
                node.set_numeric_value_step(step);
                node.add_action(Action::SetValue);
                node.add_action(Action::Increment);
                node.add_action(Action::Decrement);
            });
            if self.enabled
                && let Some(to) = asked(ui, node.id, value, self.step, |d| self.nudge(d))
                && let Some(action) = set_band(&bands, i, to)
            {
                state.selected = Some(i);
                actions.push(action);
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
            let pointer = ui
                .rect_contains_pointer(plot.area)
                .then(|| ui.input(|i| i.pointer.hover_pos()))
                .flatten()
                .filter(|_| self.enabled);
            let dragging = state.drag.is_some();
            let near = pointer.and_then(|p| plot.near(p.x, n));
            let focused = response.has_focus();
            let filled =
                |i: usize| near == Some(i) || ((focused || dragging) && state.selected == Some(i));
            self.draw_axes(&painter, &plot, outer, &t, &tr, fade);
            if let Some(histogram) = self.histogram {
                draw_histogram(&painter, &plot, histogram, fade(t.colors.graph_histogram));
                let (low, high) = histogram.overflows();
                let side = Vec2::splat(WARNING);
                let inset = Vec2::splat(t.spacing.xxs);
                if low {
                    let at = Rect::from_min_size(plot.area.left_top() + inset, side);
                    paint_warning(&painter, at, fade(t.colors.warning), t.colors.sunken);
                }
                if high {
                    let corner = plot.area.right_top() + vec2(-inset.x - WARNING, inset.y);
                    let at = Rect::from_min_size(corner, side);
                    paint_warning(&painter, at, fade(t.colors.warning), t.colors.sunken);
                }
            }
            let colour = if self.failed {
                t.colors.warning
            } else {
                t.colors.text
            };
            let columns = plot.area.width().ceil().max(2.0) as usize;
            let line: Vec<Pos2> = (0..=columns)
                .map(|k| {
                    let x = plot.area.left() + plot.area.width() * k as f32 / columns as f32;
                    pos2(
                        x,
                        plot.y_at(self.height_at(&bands, plot.exposure_at(x), plot.first)),
                    )
                })
                .collect();
            if !matches!(self.curve, Drawn::Nothing) {
                painter.line(line, Stroke::new(CURVE_WIDTH, fade(colour)));
            }
            let ground = plot.y_at(0.0);
            for (i, &value) in bands.iter().enumerate() {
                if value.abs() > 1e-6 {
                    let node = plot.node(i, value);
                    painter.line_segment(
                        [pos2(node.x, ground), node],
                        Stroke::new(BAR_WIDTH, fade(t.colors.slider_fill)),
                    );
                }
            }
            for (i, &value) in bands.iter().enumerate() {
                let centre = plot.node(i, value);
                // Filled with the ground, as darktable's, which stays opaque when dimmed.
                let fill = if filled(i) {
                    fade(t.colors.text)
                } else {
                    t.colors.sunken
                };
                painter.circle(
                    centre,
                    NODE_RADIUS,
                    fill,
                    Stroke::new(NODE_STROKE, fade(t.colors.text)),
                );
                if focused && state.selected == Some(i) && focus_shows(painter.ctx()) {
                    painter.circle_stroke(
                        centre,
                        NODE_RADIUS + NODE_STROKE / 2.0 + 3.0,
                        Stroke::new(1.5, t.colors.focus_ring),
                    );
                }
            }
            // With no node chosen yet, the graph itself shows the focus.
            if focused && state.selected.is_none() {
                focus_ring(&painter, plot.area, &t);
            }
            // How far a drag reaches: a ring on the curve under the pointer.
            if let Some(p) = pointer {
                let exposure = plot.exposure_at(p.x);
                let height = self.height_at(&bands, exposure, plot.first);
                let radius = self.smoothing / std::f32::consts::SQRT_2 * plot.per_ev();
                painter.circle_stroke(
                    pos2(p.x, plot.y_at(height)),
                    radius,
                    Stroke::new(RING_WIDTH, t.colors.text),
                );
            }
        }

        if !actions.is_empty() {
            // The caller applies what the graph reported; draw it on the next frame.
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| d.insert_temp(id, state));
        response.on_hover_text(tr.text("equalizer-tip"));

        // One action a frame: resetting wins, then a band's reset, then a move.
        actions.sort_by_key(|a| match a {
            EqualizerAction::Reset => 0,
            EqualizerAction::ResetBand(_) => 1,
            EqualizerAction::Move { .. } => 2,
        });
        actions.into_iter().next()
    }

    /// A key's move, never less than the last decimal shown, as a slider row's.
    fn nudge(&self, delta: f32) -> f32 {
        let resolution = 10f32.powi(-(self.decimals as i32));
        if delta != 0.0 && delta.abs() < resolution {
            resolution.copysign(delta)
        } else {
            delta
        }
    }

    /// The labels, the tone bars at the start and under the graph, its ground, grid and
    /// frame, and its ground line at 0 EV.
    fn draw_axes(
        &self,
        painter: &Painter,
        plot: &Plot,
        outer: Rect,
        t: &Theme,
        tr: &Translator,
        fade: impl Fn(Color32) -> Color32,
    ) {
        let font = FontId::proportional(t.text.caption);
        let label = |value: f32| {
            // darktable's "%+.0f": +0, not 0.
            if value == 0.0 {
                format!("+{}", tr.format_number(0.0, 0, false))
            } else {
                tr.format_number(f64::from(value), 0, true)
            }
        };
        let across = outer.top() + (plot.area.top() - outer.top() - t.spacing.xs) / 2.0;
        for i in 0..self.bands.len() {
            let exposure = plot.first + i as f32;
            painter.text(
                pos2(plot.x_at(exposure), across),
                Align2::CENTER_CENTER,
                label(exposure),
                font.clone(),
                t.colors.text_secondary,
            );
        }
        let down = (plot.area.right() + outer.right()) / 2.0;
        for k in 0..5 {
            let correction = SPAN - k as f32;
            painter.text(
                pos2(down, plot.y_at(correction)),
                Align2::CENTER_CENTER,
                label(correction),
                font.clone(),
                t.colors.text_secondary,
            );
        }

        // The tones the axes stand for, black to white, as darktable's gradients: colours
        // that are data, not tokens.
        let start = Rect::from_x_y_ranges(
            outer.left()..=outer.left() + t.spacing.m,
            plot.area.y_range(),
        );
        let under = Rect::from_x_y_ranges(
            plot.area.x_range(),
            plot.area.bottom() + t.spacing.m..=plot.area.bottom() + 2.0 * t.spacing.m,
        );
        painter.add(gradient(start, false));
        painter.add(gradient(under, true));

        painter.rect(
            plot.area,
            0.0,
            t.colors.sunken,
            Stroke::new(t.shape.hairline, t.colors.divider),
            StrokeKind::Inside,
        );
        let grid = Stroke::new(t.shape.hairline, t.colors.divider);
        for k in 1..8 {
            let f = k as f32 / 8.0;
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
        painter.hline(
            plot.area.x_range(),
            plot.y_at(0.0),
            Stroke::new(t.shape.hairline, fade(t.colors.text_secondary)),
        );
    }

    /// The curve's correction at `exposure`, EV: the caller's samples, or straight lines
    /// between the bands, within the graph.
    fn height_at(&self, bands: &[f32], exposure: f32, first: f32) -> f32 {
        let samples = match self.curve {
            Drawn::Samples(samples) => samples,
            Drawn::Lines | Drawn::Nothing => bands,
        };
        let at = ((exposure - first) / -first).clamp(0.0, 1.0) * (samples.len() - 1) as f32;
        let i = (at as usize).min(samples.len() - 2);
        let y = samples[i] + (samples[i + 1] - samples[i]) * (at - i as f32);
        if y.is_finite() {
            y.clamp(-SPAN, SPAN)
        } else {
            0.0
        }
    }
}

/// The graph inside its frame, and how exposures and corrections map onto it.
struct Plot {
    area: Rect,
    /// The first band's exposure, EV; the last's is 0.
    first: f32,
}

impl Plot {
    /// Points per EV across.
    fn per_ev(&self) -> f32 {
        self.area.width() / -self.first
    }

    fn x_at(&self, exposure: f32) -> f32 {
        self.area.left() + (exposure - self.first) * self.per_ev()
    }

    /// darktable's `(0.5 − correction / 4) · height`.
    fn y_at(&self, correction: f32) -> f32 {
        self.area.top() + (0.5 - correction / (2.0 * SPAN)) * self.area.height()
    }

    fn exposure_at(&self, x: f32) -> f32 {
        self.first + (x - self.area.left()) / self.per_ev()
    }

    fn node(&self, i: usize, value: f32) -> Pos2 {
        pos2(self.x_at(self.first + i as f32), self.y_at(value))
    }

    /// The node within reach of the pointer at `x`, as darktable's `area_motion_notify`
    /// finds it: within 0.45 of the nodes' spacing.
    fn near(&self, x: f32, n: usize) -> Option<usize> {
        (0..n).find(|&i| (x - self.x_at(self.first + i as f32)).abs() < NEAR * self.per_ev())
    }
}

/// The curve the graph draws.
#[derive(Clone, Copy)]
enum Drawn<'a> {
    /// The caller's samples, evenly spaced over the bands' exposures.
    Samples(&'a [f32]),
    /// Straight lines between the bands.
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

/// A drag under way: where the pointer was last, the bands where it found them, and the
/// bands it has moved them to, unrounded, whatever the caller rounds what it shows to.
#[derive(Clone, Debug)]
struct Drag {
    last: Pos2,
    from: Vec<f32>,
    to: Vec<f32>,
    moved: bool,
    /// It ended before the release: the pointer left the graph, or no curve fits the bands
    /// a step gave.
    ended: bool,
}

impl Drag {
    /// darktable's `set_new_params_interactive`: every band raised by `rise` EV times a
    /// Gaussian of its distance from `exposure`, of darktable's `gaussian_denom(σ² / 2)`,
    /// with no limit, as darktable fits them, unless `clamped`, when each stays within
    /// ±2 EV. Whether a band moved.
    fn raise(
        &mut self,
        exposure: f32,
        rise: f32,
        smoothing: f32,
        first: f32,
        clamped: bool,
    ) -> bool {
        let blending = smoothing * smoothing / 2.0;
        let denominator = 2.0 * blending * blending;
        let mut moved = false;
        for (i, band) in self.to.iter_mut().enumerate() {
            let d = first + i as f32 - exposure;
            let mut raised = *band + rise * (-d * d / denominator).exp();
            if clamped {
                raised = raised.clamp(-SPAN, SPAN);
            }
            moved |= raised != *band;
            *band = raised;
        }
        moved
    }

    fn action(&self, done: bool) -> EqualizerAction {
        EqualizerAction::Move {
            from: self.from.clone(),
            to: self.to.clone(),
            done,
        }
    }
}

/// A band's correction within the graph, the ground where it is not a number.
fn within(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(-SPAN, SPAN)
    } else {
        0.0
    }
}

/// The bands with band `i` at `to`, within ±2 EV, if that moves it.
fn set_band(bands: &[f32], i: usize, to: f32) -> Option<EqualizerAction> {
    let to = to.clamp(-SPAN, SPAN);
    (to != bands[i]).then(|| {
        let mut moved = bands.to_vec();
        moved[i] = to;
        EqualizerAction::Move {
            from: bands.to_vec(),
            to: moved,
            done: true,
        }
    })
}

/// A screen reader's step of node `id`, moved as `nudge` moves a key's, or the value it
/// typed, in EV.
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

/// Where the mask overflows the bands, in words, as darktable's signs show it.
fn overflow_text(tr: &Translator, (low, high): (bool, bool)) -> Option<String> {
    let id = match (low, high) {
        (true, true) => "equalizer-overflows-both",
        (true, false) => "equalizer-overflows-shadows",
        (false, true) => "equalizer-overflows-highlights",
        (false, false) => return None,
    };
    Some(tr.text(id))
}

/// A bar from black to white, along its width or up its height.
fn gradient(rect: Rect, across: bool) -> Mesh {
    let mut mesh = Mesh::default();
    let (black, white) = (Color32::BLACK, Color32::WHITE);
    let corners = if across {
        [black, white, white, black]
    } else {
        [white, white, black, black]
    };
    let points = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    for (pos, color) in points.into_iter().zip(corners) {
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color,
        });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The histogram as darktable fills it: from the graph's foot up to the line through each
/// bin at its exposure, the tallest at 0.96 of the graph, within the graph. It is filled a
/// column of pixels at a time, each up to the line's height at its centre, in whole pixels:
/// the bins are narrower than a pixel, and their steep edges pass so near pixel centres
/// that GPUs fill them differently, where rectangles on the pixel grid every GPU fills
/// alike.
fn draw_histogram(painter: &Painter, plot: &Plot, histogram: MaskHistogram, colour: Color32) {
    let bins = histogram.bins;
    let tallest = bins
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(0.0, f32::max);
    let [low, high] = histogram.range;
    let (first, last) = (plot.x_at(low), plot.x_at(high));
    if bins.len() < 2 || tallest <= 0.0 || last <= first {
        return;
    }
    let count = |k: usize| {
        if bins[k].is_finite() {
            bins[k].max(0.0)
        } else {
            0.0
        }
    };
    // The line's height above the foot at `x`, between the bins either side of it.
    let height_at = |x: f32| {
        let at = (x - first) / (last - first) * (bins.len() - 1) as f32;
        let k = (at as usize).min(bins.len() - 2);
        let level = count(k) + (count(k + 1) - count(k)) * (at - k as f32);
        level / tallest * HISTOGRAM_TOP * plot.area.height()
    };
    let ppp = painter.pixels_per_point();
    // The pixel grid's line above the first row whose centre is at or below `y`, in pixels:
    // where a GPU's fill from an edge at `y` starts.
    let row = |y: f32| (y * ppp - 0.5).ceil();
    let foot = row(plot.area.bottom());
    let mut mesh = Mesh::default();
    let columns = (plot.area.left() * ppp).floor() as i32..(plot.area.right() * ppp).ceil() as i32;
    for column in columns {
        let centre = (column as f32 + 0.5) / ppp;
        if !plot.area.x_range().contains(centre) || !(first..=last).contains(&centre) {
            continue;
        }
        let top = row(plot.area.bottom() - height_at(centre));
        if top < foot {
            let (left, right) = (column as f32 / ppp, (column + 1) as f32 / ppp);
            let rect = Rect::from_x_y_ranges(left..=right, top / ppp..=foot / ppp);
            mesh.add_colored_rect(rect, colour);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// darktable's warning sign (`dtgtk_cairo_paint_warning`) in `rect`: a triangle in
/// `colour` with an exclamation mark cut out of it, drawn in `ground`, the graph's.
fn paint_warning(painter: &Painter, rect: Rect, colour: Color32, ground: Color32) {
    let at = |x: f32, y: f32| rect.min + vec2(x, y) * rect.width();
    painter.add(Shape::convex_polygon(
        vec![at(0.5, 0.067), at(1.0, 0.933), at(0.0, 0.933)],
        colour,
        Stroke::NONE,
    ));
    let line = [
        (0.415, 0.317),
        (0.585, 0.317),
        (0.585, 0.448),
        (0.552, 0.71),
        (0.447, 0.71),
        (0.415, 0.448),
    ];
    let dot = [(0.42, 0.75), (0.58, 0.75), (0.58, 0.89), (0.42, 0.89)];
    for mark in [&line[..], &dot[..]] {
        let points = mark.iter().map(|&(x, y)| at(x, y)).collect();
        painter.add(Shape::convex_polygon(points, ground, Stroke::NONE));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plot() -> Plot {
        Plot {
            area: Rect::from_min_size(pos2(16.0, 20.0), vec2(256.0, 160.0)),
            first: -8.0,
        }
    }

    #[test]
    fn the_graph_maps_darktable_s_axes() {
        let plot = plot();
        assert_eq!(plot.x_at(-8.0), 16.0);
        assert_eq!(plot.x_at(0.0), 272.0);
        assert_eq!(plot.y_at(2.0), 20.0);
        assert_eq!(plot.y_at(0.0), 100.0);
        assert_eq!(plot.y_at(-2.0), 180.0);
        assert_eq!(plot.exposure_at(144.0), -4.0);
        // Within 0.45 of the spacing of a node, not between two.
        assert_eq!(plot.near(144.0 + 14.0, 9), Some(4));
        assert_eq!(plot.near(144.0 + 16.0, 9), None);
    }

    #[test]
    fn a_drag_s_falloff_is_darktable_s() {
        // At darktable's default σ, √2, a Gaussian of variance 1 EV²: the bands around
        // −4 EV take e^(−d²/2) of the rise.
        let mut drag = Drag {
            last: Pos2::ZERO,
            from: vec![0.0; 9],
            to: vec![0.0; 9],
            moved: false,
            ended: false,
        };
        assert!(drag.raise(-4.0, 1.0, SMOOTHING, -8.0, true));
        for (i, &band) in drag.to.iter().enumerate() {
            let d = i as f32 - 4.0;
            assert!((band - (-d * d / 2.0).exp()).abs() < 1e-6, "{i}: {band}");
        }
        // Each band stays within ±2 EV, but for the fit, which takes them as raised.
        assert!(drag.raise(-4.0, 5.0, SMOOTHING, -8.0, true));
        assert_eq!(drag.to[4], 2.0);
        assert!(drag.raise(-4.0, 1.0, SMOOTHING, -8.0, false));
        assert_eq!(drag.to[4], 3.0);
    }
}
