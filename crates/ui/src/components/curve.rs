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

//! The curve: points on a square plot, the curve through them, and black and white point
//! handles under it. The Tone curve module draws its curves with it; the hue curve and the
//! tone equalizer's band curve are to follow.
//!
//! It knows nothing of splines or of what the axes measure. It is given the points, x and y
//! from 0 to 1 in increasing x, and the curve to draw, as samples or as a function to
//! sample; without either it joins the points with straight lines. An axis may place its
//! values on a logarithmic or another scale, for display only: the points and what the
//! curve reports stay in the caller's values.
//!
//! - **Pointer:** pressing on the curve adds a point on it, and pressing on empty space
//!   adds one there; either can be dragged at once. Dragging a point moves it, kept between
//!   its neighbours; a double-click removes it. The first and last points stay, unless the
//!   handles are the curve's ends.
//! - **Keys:** Tab focuses the plot. Page Up and Page Down choose the previous and the next
//!   point, Home and End the first and the last. The arrow keys move the chosen point, by a
//!   hundredth of the plot unless the caller sets the step (Shift ten steps, Alt a tenth).
//!   Insert or + adds a point midway to the next one, Delete removes the chosen one, and
//!   Shift with Delete removes them all: it resets the curve. Ctrl is left to the app, and
//!   only moves repeat while a key is held.
//! - **Handles:** black and white points under the plot, either levels that move on their
//!   own ([`Curve::levels`]) or the curve's own ends ([`Curve::ends`]). Each is a slider of
//!   its own, after the plot in the focus order, with the slider row's keys; a double-click
//!   or Delete puts it back at its end.
//!
//! A drag reports every change and once more, marked done, on release, so history merges
//! it into one step. While the pointer is over the plot, or it has focus, the point at
//! hand (or the pointer) shows its input and output in a corner.

use egui::accesskit::{Action, ActionData};
use egui::epaint::Mesh;
use egui::os::OperatingSystem;
use egui::{
    Color32, Event, EventFilter, FontId, Id, InputState, Key, Pos2, Rect, Response, Role, Sense,
    Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::{focus_ring, focus_shows, marker};
use crate::i18n::Args;
use crate::{Translator, theme, translator};

/// How close in x two points may come: darktable's rgbcurve keeps its nodes this far apart
/// (`DT_IOP_RGBCURVE_MIN_X_DISTANCE`). Handles that are the curve's ends keep it too.
const MIN_GAP: f32 = 0.0025;
/// How close levels' black and white points may come. darktable's rgblevels keeps each 0.05
/// from its grey point, which lies between them; with no grey point, they keep 0.05 apart.
const LEVELS_GAP: f32 = 0.05;
/// One key press, unless the caller sets it: a hundredth of the plot.
const STEP: f32 = 0.01;
/// Page Up and Page Down on a handle: ten steps, as on a slider.
const PAGE: f32 = 10.0;
const POINT_RADIUS: f32 = 4.5;
/// How far from a point, or from the curve, a press still takes it.
const HIT: f32 = 8.0;
const CURVE_WIDTH: f32 = 1.5;
/// The strip under the plot that holds the handles, and how far their centre sits under
/// its top: the marker, the slider row's, 5.5 points each way of it, starts a point down.
const HANDLE_STRIP: f32 = 14.0;
const HANDLE_CENTRE: f32 = 6.5;
/// How faint the histogram is: a share of the secondary text's colour.
const HISTOGRAM_OPACITY: f32 = 0.12;
/// The readout's backing, translucent so the curve and histogram show through it.
const READOUT_OPACITY: f32 = 0.7;
/// How dim a disabled curve is, as a disabled slider row.
const DISABLED: f32 = 0.6;
/// How far a point the caller added may sit from where it was asked for (the caller may
/// round it), and for how many frames the curve looks for it.
const ADDED_WITHIN: f32 = 1e-3;
const ADDED_FRAMES: u8 = 8;

/// How an axis places its values, 0 to 1, along the plot. For display only: the points and
/// the actions stay in the caller's values.
#[derive(Clone, Copy, Debug, Default)]
pub enum Axis {
    #[default]
    Linear,
    /// darktable's logarithmic graph scale, `ln(1 + b·v) / ln(1 + b)`: the larger `b`, the
    /// more room the shadows get. `b` of 0 or less is linear.
    Log(f32),
    /// Any increasing mapping of 0..1 onto 0..1, with its inverse.
    Custom {
        to_plot: fn(f32) -> f32,
        from_plot: fn(f32) -> f32,
    },
}

impl Axis {
    /// Where the value `v` sits along the axis, from 0 to 1.
    fn place(self, v: f32) -> f32 {
        match self {
            Axis::Log(b) if b > 0.0 => (b * v).ln_1p() / b.ln_1p(),
            Axis::Custom { to_plot, .. } => to_plot(v),
            _ => v,
        }
    }

    /// The value at `f` along the axis, the inverse of [`Axis::place`].
    fn value(self, f: f32) -> f32 {
        match self {
            Axis::Log(b) if b > 0.0 => (b.ln_1p() * f).exp_m1() / b,
            Axis::Custom { from_plot, .. } => from_plot(f),
            _ => f,
        }
    }

    /// `v` moved `delta` along the axis, within it.
    fn nudge(self, v: f32, delta: f32) -> f32 {
        if delta == 0.0 {
            v
        } else {
            self.value((self.place(v) + delta).clamp(0.0, 1.0))
        }
    }
}

/// What the user did to the curve this frame. Points are counted from 0, in increasing x.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveAction {
    /// Point `index` moves to (`x`, `y`); `done` on the last change of a drag and on a key.
    Move {
        index: usize,
        x: f32,
        y: f32,
        done: bool,
    },
    /// A new point at (`x`, `y`), to go among the others by its x.
    Add { x: f32, y: f32 },
    /// Point `index` goes; never the first or the last, unless the handles are the ends.
    Remove(usize),
    /// The black point handle moves; `done` as for `Move`.
    BlackPoint { value: f32, done: bool },
    /// The white point handle moves; `done` as for `Move`.
    WhitePoint { value: f32, done: bool },
    /// The curve back to its default, which the caller knows.
    Reset,
}

/// The curve to draw through the points.
#[derive(Clone, Copy)]
enum Samples<'a> {
    /// Straight lines between the points.
    Lines,
    /// Outputs at evenly spaced inputs from 0 to 1.
    Even(&'a [f32]),
    Function(&'a dyn Fn(f32) -> f32),
}

/// The black and white point handles, and whether they are the curve's own ends.
#[derive(Clone, Copy)]
struct Handles {
    black: f32,
    white: f32,
    ends: bool,
}

pub struct Curve<'a> {
    name: &'a str,
    points: &'a [(f32, f32)],
    samples: Samples<'a>,
    handles: Option<Handles>,
    x_axis: Axis,
    y_axis: Axis,
    histogram: Option<&'a [f32]>,
    max_points: usize,
    step: f32,
    enabled: bool,
}

impl<'a> Curve<'a> {
    /// `name` is the curve's full, translated name, unique in its module ("Tone curve,
    /// Red"): what screen readers say, and the curve's id. `points` are (x, y) from 0 to 1,
    /// in increasing x.
    pub fn new(name: &'a str, points: &'a [(f32, f32)]) -> Self {
        Curve {
            name,
            points,
            samples: Samples::Lines,
            handles: None,
            x_axis: Axis::Linear,
            y_axis: Axis::Linear,
            histogram: None,
            max_points: usize::MAX,
            step: STEP,
            enabled: true,
        }
    }

    /// The curve as outputs at evenly spaced inputs from 0 to 1, at least two.
    pub fn samples(mut self, outputs: &'a [f32]) -> Self {
        if outputs.len() >= 2 {
            self.samples = Samples::Even(outputs);
        }
        self
    }

    /// The curve as a function of the input, sampled once per point of the plot's width.
    pub fn function(mut self, curve: &'a dyn Fn(f32) -> f32) -> Self {
        self.samples = Samples::Function(curve);
        self
    }

    /// Black and white point handles under the plot, at these inputs, as levels: they move
    /// on their own, at least 0.05 apart, and the curve's first and last points stay.
    pub fn levels(mut self, black: f32, white: f32) -> Self {
        self.handles = Some(Handles {
            black,
            white,
            ends: false,
        });
        self
    }

    /// Black and white point handles that are the curve's own ends, (`black`, 0) and
    /// (`white`, 1). The points are those between them, every one can go, and points and
    /// handles keep to their order. The curve drawn should run from end to end.
    pub fn ends(mut self, black: f32, white: f32) -> Self {
        self.handles = Some(Handles {
            black,
            white,
            ends: true,
        });
        self
    }

    /// The same scale on both axes.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.x_axis = axis;
        self.y_axis = axis;
        self
    }

    pub fn axes(mut self, x: Axis, y: Axis) -> Self {
        self.x_axis = x;
        self.y_axis = y;
        self
    }

    /// A faint histogram behind the curve: bins evenly spaced over the inputs, at any scale;
    /// the tallest fills the plot.
    pub fn histogram(mut self, bins: &'a [f32]) -> Self {
        self.histogram = Some(bins);
        self
    }

    /// The most points the curve takes: adding stops there.
    pub fn max_points(mut self, max: usize) -> Self {
        self.max_points = max;
        self
    }

    /// How far one press of an arrow key moves a point or a handle, as a share of the plot:
    /// a hundredth unless set.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// A disabled curve is dimmed and ignores input.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<CurveAction> {
        // A value that is not a number would draw nowhere and break the limits: read it as
        // the axis's start, or a white point as its end.
        let finite = |v: f32, or: f32| if v.is_finite() { v } else { or };
        let points: Vec<(f32, f32)> = self
            .points
            .iter()
            .map(|&(x, y)| (finite(x, 0.0), finite(y, 0.0)))
            .collect();
        let handles = self.handles.map(|h| Handles {
            black: finite(h.black, 0.0),
            white: finite(h.white, 1.0),
            ..h
        });
        Curve {
            points: &points,
            handles,
            ..self
        }
        .draw(ui)
    }

    fn draw(self, ui: &mut Ui) -> Option<CurveAction> {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let id = ui.make_persistent_id(("curve", self.name));
        let side = ui.available_width();
        let strip_height = if self.handles.is_some() {
            HANDLE_STRIP
        } else {
            0.0
        };
        let (outer, _) = ui.allocate_exact_size(vec2(side, side + strip_height), Sense::hover());
        let frame = Rect::from_min_size(outer.min, vec2(side, side));
        let plot = Plot {
            area: frame.shrink(t.spacing.s),
            x: self.x_axis,
            y: self.y_axis,
        };
        let fade = |c: Color32| {
            if self.enabled {
                c
            } else {
                c.gamma_multiply(DISABLED)
            }
        };
        let n = self.points.len();
        let mut state: State = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        state.selected = state.selected.filter(|_| n > 0).map(|s| s.min(n - 1));
        let mut actions: Vec<CurveAction> = Vec::new();

        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let response = ui.interact(frame, id, sense);
        let down = response.is_pointer_button_down_on();
        let pressed = down && ui.input(|i| i.pointer.primary_pressed());
        let origin = ui.input(|i| i.pointer.press_origin());

        // A point asked for lately: choose it once the caller has it, and drag it while the
        // pointer that added it is still down.
        if let Some((x, frames)) = state.added.take() {
            match self
                .points
                .iter()
                .position(|p| (p.0 - x).abs() <= ADDED_WITHIN)
            {
                Some(i) => {
                    state.selected = Some(i);
                    if down && !pressed && state.drag.is_none() {
                        let at = plot.at(self.points[i]);
                        state.drag = Some(Drag {
                            index: i,
                            offset: origin.map_or(Vec2::ZERO, |o| at - o),
                        });
                    }
                }
                None if frames > 1 => state.added = Some((x, frames - 1)),
                None => {}
            }
        }

        // Pressing takes the point under the pointer, or adds one.
        if self.enabled
            && pressed
            && let Some(origin) = origin
        {
            if !response.has_focus() {
                response.request_focus();
            }
            state.drag = None;
            if let Some(i) = self.point_near(&plot, origin) {
                state.selected = Some(i);
                state.drag = Some(Drag {
                    index: i,
                    offset: plot.at(self.points[i]) - origin,
                });
            } else if let Some((x, y)) = self.new_point(&plot, origin) {
                actions.push(CurveAction::Add { x, y });
                state.added = Some((x, ADDED_FRAMES));
            }
        }

        // Dragging, and what the release does.
        if let Some(drag) = state.drag.filter(|d| self.enabled && d.index < n) {
            let index = drag.index;
            let to = response
                .interact_pointer_pos()
                .map(|p| self.clamped(index, plot.values(p + drag.offset)));
            if response.double_clicked() {
                if self.removable(index) {
                    actions.push(CurveAction::Remove(index));
                }
            } else if response.dragged() || response.drag_stopped() {
                let done = response.drag_stopped();
                let (x, y) = to.unwrap_or(self.points[index]);
                if (x, y) != self.points[index] || done {
                    actions.push(CurveAction::Move { index, x, y, done });
                }
            }
            if !down {
                state.drag = None;
            }
        } else {
            state.drag = None;
        }

        // Keys: only those the app leaves a focused control, none with Ctrl (`docs/gui.md`,
        // Keyboard and actions). Moving repeats while a key is held; adding, removing and
        // resetting happen once a press.
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
            let current = state.selected.or((n > 0).then_some(0));
            let keys = ui.input(|i| {
                let command = i.modifiers.command || i.modifiers.ctrl;
                let scale = if i.modifiers.shift {
                    10.0
                } else if i.modifiers.alt {
                    0.1
                } else {
                    1.0
                };
                let mut nudge = Vec2::ZERO;
                for (key, direction) in [
                    (Key::ArrowRight, vec2(1.0, 0.0)),
                    (Key::ArrowLeft, vec2(-1.0, 0.0)),
                    (Key::ArrowUp, vec2(0.0, 1.0)),
                    (Key::ArrowDown, vec2(0.0, -1.0)),
                ] {
                    if !command && i.key_pressed(key) {
                        nudge += direction * scale * self.step;
                    }
                }
                let choose = if command {
                    None
                } else if i.key_pressed(Key::Home) {
                    current.map(|_| 0)
                } else if i.key_pressed(Key::End) {
                    current.map(|_| n - 1)
                } else if i.key_pressed(Key::PageUp) {
                    current.map(|c| c.saturating_sub(1))
                } else if i.key_pressed(Key::PageDown) {
                    current.map(|c| (c + 1).min(n - 1))
                } else {
                    None
                };
                let add = !command && (pressed_once(i, Key::Insert) || pressed_once(i, Key::Plus));
                let delete =
                    !command && (pressed_once(i, Key::Delete) || pressed_once(i, Key::Backspace));
                Keys {
                    choose,
                    nudge,
                    add,
                    remove: delete && !i.modifiers.shift,
                    reset: delete && i.modifiers.shift,
                }
            });
            state.selected = keys.choose.or(current);
            if let Some(index) = state.selected
                && keys.nudge != Vec2::ZERO
            {
                // Only the axis the key moves along changes, whatever its scale.
                let (x, y) = self.points[index];
                let moved = self.clamped(
                    index,
                    (plot.x.nudge(x, keys.nudge.x), plot.y.nudge(y, keys.nudge.y)),
                );
                if moved != (x, y) {
                    actions.push(CurveAction::Move {
                        index,
                        x: moved.0,
                        y: moved.1,
                        done: true,
                    });
                }
            }
            if keys.add
                && let Some((x, y)) = self.midway(&plot, state.selected)
            {
                actions.push(CurveAction::Add { x, y });
                state.added = Some((x, ADDED_FRAMES));
            }
            if keys.reset {
                actions.push(CurveAction::Reset);
            } else if keys.remove
                && let Some(index) = state.selected.filter(|&i| self.removable(i))
            {
                actions.push(CurveAction::Remove(index));
            }
        }

        // What screen readers hear: the plot as a list of points, the chosen one active.
        // Each point's coordinates are in its name, which is what VoiceOver reads of a list
        // item.
        let help = tr.text(if ui.ctx().os() == OperatingSystem::Mac {
            "curve-keys-mac"
        } else {
            "curve-keys"
        });
        response.widget_info(|| WidgetInfo::labeled(Role::ListBox, self.enabled, self.name));
        let active = state.selected.map(|s| point_id(id, s).accesskit_id());
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_description(help);
            if let Some(active) = active {
                node.set_active_descendant(active);
            }
        });
        let points_ui = ui.new_child(UiBuilder::new().max_rect(frame).accessibility_parent(id));
        for (i, &(x, y)) in self.points.iter().enumerate() {
            let label = point_text(&tr, i, x, y);
            let point = points_ui.interact(
                Rect::from_center_size(plot.at((x, y)), Vec2::splat(2.0 * HIT)),
                point_id(id, i),
                Sense::hover(),
            );
            point.widget_info(|| WidgetInfo::labeled(Role::ListBoxOption, self.enabled, &label));
            let chosen = state.selected == Some(i);
            ui.ctx()
                .accesskit_node_builder(point.id, |node| node.set_selected(chosen));
        }

        // The handles, each a slider of its own.
        let strip = Rect::from_min_size(frame.left_bottom(), vec2(side, strip_height));
        let mut handles = Vec::new();
        if let Some(Handles { black, white, ends }) = self.handles {
            let mut args = Args::new();
            args.set("control", self.name);
            // Levels keep apart from each other; ends keep to the points' order too.
            let (black_high, white_low) = if ends {
                (
                    below(self.points.first().map_or(white, |p| p.0), MIN_GAP),
                    above(self.points.last().map_or(black, |p| p.0), MIN_GAP),
                )
            } else {
                (below(white, LEVELS_GAP), above(black, LEVELS_GAP))
            };
            for (level, message, is_black) in [
                (
                    Level {
                        id: id.with("black"),
                        value: black,
                        low: 0.0,
                        high: black_high,
                        default: 0.0,
                        step: self.step,
                    },
                    "curve-black-point",
                    true,
                ),
                (
                    Level {
                        id: id.with("white"),
                        value: white,
                        low: white_low,
                        high: 1.0,
                        default: 1.0,
                        step: self.step,
                    },
                    "curve-white-point",
                    false,
                ),
            ] {
                let name = tr.text_with(message, Some(&args));
                let (handle, set) =
                    level_handle(ui, &plot, strip, level, self.enabled, &mut state.grab);
                // In percent, to a hundredth, as a screen reader should say it: 4, not
                // 3.9999998.
                let heard = |v: f32| (f64::from(v) * 10_000.0).round() / 100.0;
                let (value, low, high, step) = (
                    heard(level.value),
                    heard(level.low),
                    heard(level.settle(level.high)),
                    heard(level.step),
                );
                handle.widget_info(|| WidgetInfo::slider(self.enabled, value, &name));
                ui.ctx().accesskit_node_builder(handle.id, |node| {
                    node.set_min_numeric_value(low);
                    node.set_max_numeric_value(high);
                    node.set_numeric_value_step(step);
                    node.add_action(Action::SetValue);
                    node.add_action(Action::Increment);
                    node.add_action(Action::Decrement);
                });
                if let Some((value, done)) = set {
                    actions.push(if is_black {
                        CurveAction::BlackPoint { value, done }
                    } else {
                        CurveAction::WhitePoint { value, done }
                    });
                }
                handles.push((level.value, handle));
            }
        }

        // Drawing.
        if ui.is_rect_visible(outer) {
            let painter = ui.painter();
            painter.rect(
                frame,
                t.shape.radius,
                t.colors.sunken,
                Stroke::new(t.shape.hairline, t.colors.divider),
                StrokeKind::Inside,
            );
            let inside = painter.with_clip_rect(frame.intersect(painter.clip_rect()));
            if let Some(bins) = self.histogram {
                inside.add(histogram(
                    &plot,
                    bins,
                    fade(t.colors.text_secondary).gamma_multiply(HISTOGRAM_OPACITY),
                ));
            }
            let grid = Stroke::new(t.shape.hairline, t.colors.divider);
            for k in 1..4 {
                let f = k as f32 / 4.0;
                let x = plot.area.left() + f * plot.area.width();
                let y = plot.area.bottom() - f * plot.area.height();
                inside.vline(x, plot.area.y_range(), grid);
                inside.hline(plot.area.x_range(), y, grid);
            }
            let identity: Vec<Pos2> = (0..=64)
                .map(|k| {
                    let v = k as f32 / 64.0;
                    plot.at((v, v))
                })
                .collect();
            inside.extend(Shape::dashed_line(
                &identity,
                Stroke::new(t.shape.hairline, fade(t.colors.border)),
                4.0,
                4.0,
            ));
            let columns = plot.area.width().ceil().max(2.0) as usize;
            let line: Vec<Pos2> = (0..=columns)
                .map(|k| {
                    let x = plot.x.value(k as f32 / columns as f32);
                    plot.at((x, self.height_at(x)))
                })
                .collect();
            inside.line(line, Stroke::new(CURVE_WIDTH, fade(t.colors.text)));

            // The readout, under the points so it never hides one: the point dragged or
            // under the pointer, else the pointer, else the chosen point while focused. Its
            // backing is translucent, so the curve and histogram show through.
            let pointer = ui
                .rect_contains_pointer(frame)
                .then(|| ui.input(|i| i.pointer.hover_pos()))
                .flatten()
                .filter(|_| self.enabled);
            let hovered = pointer.and_then(|p| self.point_near(&plot, p));
            let dragged = state.drag.map(|d| d.index).filter(|&i| i < n);
            let focused = response.has_focus();
            let readout = dragged
                .or(hovered)
                .map(|i| self.points[i])
                .or_else(|| pointer.map(|p| plot.values(p)))
                .or_else(|| state.selected.filter(|_| focused).map(|i| self.points[i]));
            if let Some((x, y)) = readout {
                let galley = painter.layout_no_wrap(
                    value_text(&tr, x, y),
                    FontId::proportional(t.text.small),
                    t.colors.text_secondary,
                );
                // In the top-left corner, unless what it describes is there.
                let at = plot.at((x, y));
                let corner = if at.x < plot.area.center().x && at.y < plot.area.center().y {
                    plot.area.right_bottom() - galley.size()
                } else {
                    plot.area.left_top()
                };
                inside.rect_filled(
                    Rect::from_min_size(corner, galley.size()).expand(t.spacing.xxs),
                    t.shape.radius_small,
                    t.colors.sunken.gamma_multiply(READOUT_OPACITY),
                );
                inside.galley(corner, galley, t.colors.text_secondary);
            }

            for (i, &point) in self.points.iter().enumerate() {
                let centre = plot.at(point);
                let chosen = (focused || dragged.is_some()) && state.selected == Some(i);
                let radius = if hovered == Some(i) || dragged == Some(i) {
                    POINT_RADIUS + 1.0
                } else {
                    POINT_RADIUS
                };
                painter.circle(
                    centre,
                    radius,
                    fade(if chosen {
                        t.colors.accent
                    } else {
                        t.colors.slider_handle
                    }),
                    Stroke::new(1.5, t.colors.slider_handle_outline),
                );
                if chosen && focused && focus_shows(painter.ctx()) {
                    painter.circle_stroke(
                        centre,
                        radius + 3.0,
                        Stroke::new(1.5, t.colors.focus_ring),
                    );
                }
            }
            // With no point to choose, as a curve whose handles are its ends starts, the
            // plot itself shows the focus.
            if focused && state.selected.is_none() {
                focus_ring(painter, frame, &t);
            }

            // The handles mark their inputs with the slider row's marker. The plot sits
            // inside its frame, so they stay whole at its ends.
            let shape = marker(ui.ctx());
            for (value, handle) in &handles {
                let centre = pos2(plot.x_at(*value), strip.top() + HANDLE_CENTRE);
                shape.paint(painter, centre, handle, &t, fade);
                if handle.has_focus() {
                    focus_ring(painter, handle.rect.shrink2(vec2(1.0, 0.0)), &t);
                }
            }
        }

        if !actions.is_empty() || state.added.is_some() {
            // The caller applies what the curve reported; draw it on the next frame.
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| d.insert_temp(id, state));

        // One action a frame: resetting wins, then removing, adding and moving.
        actions.sort_by_key(|a| match a {
            CurveAction::Reset => 0,
            CurveAction::Remove(_) => 1,
            CurveAction::Add { .. } => 2,
            CurveAction::Move { .. } => 3,
            CurveAction::BlackPoint { .. } | CurveAction::WhitePoint { .. } => 4,
        });
        actions.into_iter().next()
    }

    /// The curve's output at the input `x`, within 0 to 1.
    fn height_at(&self, x: f32) -> f32 {
        let y = match self.samples {
            Samples::Function(curve) => curve(x),
            Samples::Even(outputs) => {
                let at = x.clamp(0.0, 1.0) * (outputs.len() - 1) as f32;
                let i = (at as usize).min(outputs.len() - 2);
                outputs[i] + (outputs[i + 1] - outputs[i]) * (at - i as f32)
            }
            Samples::Lines => joined(self.points, x),
        };
        if y.is_finite() {
            y.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// The point within reach of `p`, the nearest if several are.
    fn point_near(&self, plot: &Plot, p: Pos2) -> Option<usize> {
        self.points
            .iter()
            .enumerate()
            .map(|(i, &point)| (i, plot.at(point).distance(p)))
            .filter(|&(_, distance)| distance <= HIT)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The handles, when they are the curve's own ends.
    fn end_handles(&self) -> Option<(f32, f32)> {
        self.handles.filter(|h| h.ends).map(|h| (h.black, h.white))
    }

    /// Whether point `index` may go: any but the first and the last, or any at all when the
    /// handles are the curve's ends.
    fn removable(&self, index: usize) -> bool {
        let n = self.points.len();
        index < n && (self.end_handles().is_some() || (index > 0 && index + 1 < n))
    }

    /// Where the points may lie in x: the plot, or between the handles when they are the
    /// curve's ends.
    fn span(&self) -> (f32, f32) {
        self.end_handles().map_or((0.0, 1.0), |(black, white)| {
            (above(black, MIN_GAP), below(white, MIN_GAP))
        })
    }

    /// The point a press at `p` adds: on the curve if the press is near it, else where the
    /// press is. None when the curve is full, another point is closer than the gap, or it
    /// is outside the ends.
    fn new_point(&self, plot: &Plot, p: Pos2) -> Option<(f32, f32)> {
        if self.points.len() >= self.max_points {
            return None;
        }
        let (x, y) = plot.values(p);
        let (start, end) = self.span();
        if x < start || x > end || !self.points.iter().all(|q| apart(q.0, x)) {
            return None;
        }
        let on_curve = self.height_at(x);
        let near = (plot.at((x, on_curve)).y - p.y).abs() <= HIT;
        Some((x, if near { on_curve } else { y }))
    }

    /// A point midway, along the plot, between the chosen point and the next one (the one
    /// before it for the last), on the curve. When the handles are the ends they count as
    /// neighbours, so a curve with no points takes one midway between them.
    fn midway(&self, plot: &Plot, chosen: Option<usize>) -> Option<(f32, f32)> {
        if self.points.len() >= self.max_points {
            return None;
        }
        let ends = self.end_handles();
        let xs: Vec<f32> = ends
            .map(|e| e.0)
            .into_iter()
            .chain(self.points.iter().map(|p| p.0))
            .chain(ends.map(|e| e.1))
            .collect();
        let at = chosen.map_or(0, |i| i + usize::from(ends.is_some()));
        if xs.len() < 2 || at >= xs.len() {
            return None;
        }
        let (a, b) = if at + 1 < xs.len() {
            (xs[at], xs[at + 1])
        } else {
            (xs[at - 1], xs[at])
        };
        let x = plot.x.value((plot.x.place(a) + plot.x.place(b)) / 2.0);
        (apart(a, x) && apart(x, b)).then(|| (x, self.height_at(x)))
    }

    /// (`x`, `y`) kept within the plot and, in x, at least the gap from point `index`'s
    /// neighbours.
    fn clamped(&self, index: usize, (x, y): (f32, f32)) -> (f32, f32) {
        let (start, end) = self.span();
        let low = index
            .checked_sub(1)
            .map_or(start, |before| above(self.points[before].0, MIN_GAP));
        let high = self
            .points
            .get(index + 1)
            .map_or(end, |after| below(after.0, MIN_GAP));
        (within(x, low, high), within(y, 0.0, 1.0))
    }
}

/// The plot's square inside its frame, and how values map onto it.
struct Plot {
    area: Rect,
    x: Axis,
    y: Axis,
}

impl Plot {
    fn at(&self, (x, y): (f32, f32)) -> Pos2 {
        pos2(
            self.x_at(x),
            self.area.bottom() - self.y.place(y) * self.area.height(),
        )
    }

    fn x_at(&self, x: f32) -> f32 {
        self.area.left() + self.x.place(x) * self.area.width()
    }

    /// The values at `p`, within the plot.
    fn values(&self, p: Pos2) -> (f32, f32) {
        let fx = ((p.x - self.area.left()) / self.area.width()).clamp(0.0, 1.0);
        let fy = ((self.area.bottom() - p.y) / self.area.height()).clamp(0.0, 1.0);
        (self.x.value(fx), self.y.value(fy))
    }
}

/// The value just far enough above `x` to be `gap` from it in f32, where `x + gap` can fall
/// a hair short.
fn above(x: f32, gap: f32) -> f32 {
    let mut v = x + gap;
    while v - x < gap {
        v = v.next_up();
    }
    v
}

/// The value just far enough below `x` to be `gap` from it in f32.
fn below(x: f32, gap: f32) -> f32 {
    let mut v = x - gap;
    while x - v < gap {
        v = v.next_down();
    }
    v
}

/// Whether two points' x are at least the gap apart, measured as the caller measures it.
fn apart(a: f32, b: f32) -> bool {
    (b - a).abs() >= MIN_GAP
}

/// `v` within `low` and `high`, never panicking: a `high` below `low` gives `low`.
fn within(v: f32, low: f32, high: f32) -> f32 {
    v.max(low).min(high.max(low))
}

/// Whether `key` went down this frame, not counting a held key's repeats.
pub(super) fn pressed_once(i: &InputState, key: Key) -> bool {
    i.events.iter().any(|event| {
        matches!(
            event,
            Event::Key { key: k, pressed: true, repeat: false, .. } if *k == key
        )
    })
}

/// What the curve keeps between frames, in egui's memory. Never a value the caller owns.
#[derive(Clone, Copy, Debug, Default)]
struct State {
    /// The point the keys act on.
    selected: Option<usize>,
    drag: Option<Drag>,
    /// The x of a point just asked for, and for how many more frames to look for it: it is
    /// chosen once the caller has it.
    added: Option<(f32, u8)>,
    /// How far a dragged handle sits from the pointer, in x.
    grab: f32,
}

/// A point being dragged, and where it sits from the pointer, so it never jumps to it.
#[derive(Clone, Copy, Debug)]
struct Drag {
    index: usize,
    offset: Vec2,
}

/// What the keys asked for this frame.
struct Keys {
    choose: Option<usize>,
    nudge: Vec2,
    add: bool,
    remove: bool,
    reset: bool,
}

/// A handle's value, limits and key step.
#[derive(Clone, Copy)]
struct Level {
    id: Id,
    value: f32,
    low: f32,
    high: f32,
    /// Where a double-click or Delete puts it: its end of the axis.
    default: f32,
    step: f32,
}

impl Level {
    fn settle(self, v: f32) -> f32 {
        within(v, self.low, self.high)
    }
}

/// One of the level handles under the plot: its response, and a new value with whether it
/// is done.
fn level_handle(
    ui: &Ui,
    plot: &Plot,
    strip: Rect,
    level: Level,
    enabled: bool,
    grab: &mut f32,
) -> (Response, Option<(f32, bool)>) {
    let x = plot.x_at(level.value);
    let rect = Rect::from_x_y_ranges(x - HIT..=x + HIT, strip.y_range());
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, level.id, sense);
    if !enabled {
        return (response, None);
    }
    let place = plot.x.place(level.value);
    let along = |f: f32| level.settle(plot.x.value(f.clamp(0.0, 1.0)));
    let mut set = None;
    if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.primary_pressed()) {
        if !response.has_focus() {
            response.request_focus();
        }
        if let Some(origin) = ui.input(|i| i.pointer.press_origin()) {
            *grab = x - origin.x;
        }
    }
    if response.double_clicked() {
        set = Some((level.settle(level.default), true));
    } else if (response.dragged() || response.drag_stopped())
        && let Some(p) = response.interact_pointer_pos()
    {
        let value = along((p.x + *grab - plot.area.left()) / plot.area.width());
        let done = response.drag_stopped();
        if value != level.value || done {
            set = Some((value, done));
        }
    }
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                level.id,
                EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });
        let keyed = ui.input(|i| {
            let scale = if i.modifiers.shift {
                10.0
            } else if i.modifiers.alt {
                0.1
            } else {
                1.0
            };
            let mut delta = 0.0;
            for (key, steps) in [
                (Key::ArrowRight, 1.0),
                (Key::ArrowUp, 1.0),
                (Key::ArrowLeft, -1.0),
                (Key::ArrowDown, -1.0),
                (Key::PageUp, PAGE),
                (Key::PageDown, -PAGE),
            ] {
                if i.key_pressed(key) {
                    delta += steps * scale * level.step;
                }
            }
            if i.modifiers.command || i.modifiers.ctrl {
                // Keys with Ctrl are the app's.
                None
            } else if i.key_pressed(Key::Home) {
                Some(level.settle(level.low))
            } else if i.key_pressed(Key::End) {
                Some(level.settle(level.high))
            } else if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                Some(level.settle(level.default))
            } else if delta != 0.0 {
                Some(along(place + delta))
            } else {
                None
            }
        });
        if let Some(value) = keyed {
            set = Some((value, true));
        }
    }
    // A screen reader's steps, and a value in percent.
    let (more, less, typed) = ui.input(|i| {
        (
            i.num_accesskit_action_requests(level.id, Action::Increment),
            i.num_accesskit_action_requests(level.id, Action::Decrement),
            i.accesskit_action_requests(level.id, Action::SetValue)
                .filter_map(|request| match request.data {
                    Some(ActionData::NumericValue(v)) => Some(v),
                    _ => None,
                })
                .last(),
        )
    });
    if let Some(percent) = typed {
        set = Some((level.settle(percent as f32 / 100.0), true));
    } else if more != less {
        set = Some((
            along(place + (more as f32 - less as f32) * level.step),
            true,
        ));
    }
    (response, set)
}

fn point_id(curve: Id, index: usize) -> Id {
    curve.with(("point", index))
}

/// A value in percent, to a tenth, as the messages show it.
fn percent(v: f32) -> f64 {
    (f64::from(v) * 1000.0).round() / 10.0
}

/// "Input 42%, output 51%", in the language's numbers: the readout.
fn value_text(tr: &Translator, x: f32, y: f32) -> String {
    let mut args = Args::new();
    args.set("input", percent(x));
    args.set("output", percent(y));
    tr.text_with("curve-point-value", Some(&args))
}

/// "Point 3: input 42%, output 51%": a point's name for screen readers, counted from 1.
fn point_text(tr: &Translator, index: usize, x: f32, y: f32) -> String {
    let mut args = Args::new();
    args.set("index", index + 1);
    args.set("input", percent(x));
    args.set("output", percent(y));
    tr.text_with("curve-point", Some(&args))
}

/// The points joined by straight lines, flat past the ends.
fn joined(points: &[(f32, f32)], x: f32) -> f32 {
    let after = points.partition_point(|p| p.0 <= x);
    match (
        after.checked_sub(1).map(|i| points[i]),
        points.get(after).copied(),
    ) {
        (Some(a), Some(b)) => a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0).max(f32::EPSILON),
        (Some(only), None) | (None, Some(only)) => only.1,
        (None, None) => x,
    }
}

/// The histogram's bins as bars from the plot's foot, the tallest reaching its top.
fn histogram(plot: &Plot, bins: &[f32], colour: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let tallest = bins.iter().copied().fold(0.0, f32::max);
    if tallest <= 0.0 {
        return mesh;
    }
    let count = bins.len() as f32;
    for (i, &bin) in bins.iter().enumerate() {
        let height = bin / tallest * plot.area.height();
        if height > 0.0 {
            let x0 = plot.x_at(i as f32 / count);
            let x1 = plot.x_at((i + 1) as f32 / count);
            mesh.add_colored_rect(
                Rect::from_min_max(
                    pos2(x0, plot.area.bottom() - height.min(plot.area.height())),
                    pos2(x1, plot.area.bottom()),
                ),
                colour,
            );
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_logarithmic_axis_inverts_and_keeps_its_ends() {
        let axis = Axis::Log(64.0);
        assert!(axis.place(0.0).abs() < 1e-6);
        assert!((axis.place(1.0) - 1.0).abs() < 1e-6);
        // The shadows get the room: a tenth of the input takes most of half the plot.
        assert!(axis.place(0.1) > 0.4);
        for v in [0.0, 0.01, 0.18, 0.5, 0.9, 1.0] {
            assert!((axis.value(axis.place(v)) - v).abs() < 1e-5, "{v}");
        }
        assert_eq!(Axis::Log(0.0).place(0.3), 0.3);
        // A nudge along one axis leaves the value alone when it is zero.
        assert_eq!(axis.nudge(0.37, 0.0), 0.37);
    }

    #[test]
    fn straight_lines_join_the_points_and_stay_flat_past_them() {
        let points = [(0.2, 0.1), (0.6, 0.9)];
        assert!((joined(&points, 0.4) - 0.5).abs() < 1e-6);
        assert_eq!(joined(&points, 0.0), 0.1);
        assert_eq!(joined(&points, 1.0), 0.9);
        assert_eq!(joined(&[], 0.3), 0.3);
    }

    #[test]
    fn the_gap_holds_in_f32_wherever_a_point_sits() {
        // `x + 0.0025` falls short of 0.0025 above `x` for most x in f32.
        for k in 0..1000 {
            let x = k as f32 / 1000.0;
            assert!(above(x, MIN_GAP) - x >= MIN_GAP, "{x}");
            assert!(x - below(x, MIN_GAP) >= MIN_GAP, "{x}");
            assert!(above(x, MIN_GAP) - x < MIN_GAP + 1e-6, "{x}");
        }
        assert!(above(f32::NAN, MIN_GAP).is_nan());
        assert_eq!(within(0.5, 0.6, 0.4), 0.6);
        assert_eq!(within(f32::NAN, 0.1, 0.9), 0.1);
    }
}
