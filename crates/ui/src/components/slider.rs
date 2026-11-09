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

//! The slider row, in two lines (`docs/gui.md`, How the rows are laid out): the label and
//! its marker at the start of the top line, the quad and the value at its end, and the
//! track the row's full width under them.
//!
//! - The track is one line, plain or a colour band; the part moved from the default is
//!   drawn on it at the same thickness. Its handle, the marker Settings chooses
//!   ([`super::Marker`], decision 75), points at the value and stays whole at the track's
//!   ends: its travel, and the fill's, stop short of each end by the marker's stop, and a
//!   press anywhere on the strip maps onto that travel, so one at the very end gives the
//!   track's end.
//! - The marker, beside the label, shows ● on a slider you have kept, ○ under the pointer
//!   or focus (a click keeps it), and "A" with a tick at the old value while a Match
//!   proposal waits.
//! - The value, at rest, is text with its unit as darktable's `dt_bauhaus_slider_get_text`
//!   writes it (decision 42, `docs/gui.md`, Slider values and choices): "+0.42 EV",
//!   "25.00%", "1600 ISO", signed whenever the range spans zero, zero as "+0.00", in the
//!   small text, its end 5 points from the row's end. It is never cut: the label is, at its
//!   end with …, whole in its tooltip. Under the pointer it takes the `hover` band and an
//!   I-beam. A click on it, a digit, a minus sign or the decimal separator typed on the
//!   focused track, or Enter there, opens the value field in its place, the number alone
//!   with no `+`, selected, or replaced by the character typed. It takes a number only, for
//!   now (`ValueField::number_only`); Enter, Tab or a click elsewhere sets it, clamped to
//!   the range, Escape puts it back, and so does leaving it empty or with a sign or a
//!   separator alone; the value is text again.
//! - The quad is an optional action just before the value: a picker or a line tool.
//! - The track spans the soft range, as darktable's sliders do (`soft_range`), inside the
//!   range, which stays what a value may hold: a value beyond it, typed, pasted or set by
//!   a script, widens the track just enough to show it, and the track keeps that span
//!   until the row is reset, or until the soft range itself moves, as a range that follows
//!   other controls does (decision 30), when it starts again from the new one, as
//!   darktable's `dt_bauhaus_slider_set_soft_min` resets its visible range. A drag and the
//!   keys stay inside the span the track shows.
//!
//! Dragging sends a value on every change and once more, marked done, on release, so the
//! history can merge a drag into one step. A dragged or typed value is rounded to the
//! decimals shown, as darktable rounds it to its digits; the step is what a key moves,
//! growing with the track's span as darktable's does, and never less than the last decimal
//! shown. Arrow keys step (Shift ten steps, Alt a tenth), Home and End go to the track's
//! ends, Delete or a double-click on the label or the track resets, and Enter opens the
//! value field until darktable's popup comes (planned: decision 42; `docs/gui.md`, The
//! slider popup). Tab goes through the row as it reads: the marker, the quad, the value
//! field while it is open, then the track. Screen readers hear the value as it is drawn,
//! with its unit, and are told the range a value may hold, which a value they set may
//! reach, and step inside the track's span, as the keys do.

use std::ops::RangeInclusive;

use egui::accesskit::{Action, ActionData};
use egui::epaint::Mesh;
use egui::os::OperatingSystem;
use egui::{
    Align2, Color32, CursorIcon, Event, EventFilter, FontId, Key, Modifiers, Pos2, Rangef, Rect,
    Role, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::value_field::number_char;
use super::{
    IconButton, ValueField, field_inset, focus_ring, focus_ring_centred, marker, paint_keep_marker,
    truncated,
};
use crate::{Icon, Theme, theme, translator};

/// The track's look.
#[derive(Clone, Debug, PartialEq)]
pub enum Track {
    /// One plain line.
    Plain,
    /// A colour band, with colour stops at fractions of its length: colour is the point
    /// (temperature, tint, hue). These colours are data, not theme colours.
    Gradient(Vec<(f32, Color32)>),
}

impl Track {
    /// Cool to warm, through neutral at 40%.
    pub fn temperature() -> Track {
        Track::Gradient(vec![
            (0.0, Color32::from_rgb(0x5B, 0x7F, 0xD6)),
            (0.4, Color32::from_rgb(0xE6, 0xE6, 0xDF)),
            (1.0, Color32::from_rgb(0xE2, 0xC1, 0x4F)),
        ])
    }

    /// Green to magenta, through neutral.
    pub fn tint() -> Track {
        Track::Gradient(vec![
            (0.0, Color32::from_rgb(0x4E, 0x9F, 0x5C)),
            (0.5, Color32::from_rgb(0xE6, 0xE6, 0xDF)),
            (1.0, Color32::from_rgb(0xB8, 0x5A, 0xAE)),
        ])
    }

    /// The hue circle, red to red.
    pub fn hue() -> Track {
        let stops = [
            0xD9534F, 0xD9B44F, 0x8CC152, 0x4FC1C1, 0x4F7BD9, 0xA35FD1, 0xD9534F,
        ];
        Track::Gradient(
            stops
                .iter()
                .enumerate()
                .map(|(i, &rgb)| {
                    let [_, r, g, b] = u32::to_be_bytes(rgb);
                    (i as f32 / 6.0, Color32::from_rgb(r, g, b))
                })
                .collect(),
        )
    }
}

/// What the marker shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mark {
    #[default]
    None,
    /// You set it: Auto, Match and assistants leave it alone.
    Kept,
    /// Set by Auto or Match; shown as "A" while a Match proposal waits.
    Auto,
}

/// An action just before the row's value field.
#[derive(Clone, Copy, Debug)]
pub struct Quad<'a> {
    pub icon: Icon,
    /// Translated: the tooltip and the accessible name.
    pub name: &'a str,
    /// Drawn pressed while its tool is in use.
    pub active: bool,
}

/// What the user did to the row this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderAction {
    /// A new value; `done` on the last change of a drag, a click, a key or a typed value.
    Set { value: f64, done: bool },
    /// Back to the default.
    Reset,
    /// Keep the slider, or release it.
    ToggleKeep,
    /// The quad was pressed.
    Quad,
}

pub struct SliderRow<'a> {
    label: &'a str,
    name: &'a str,
    promise: Option<&'a str>,
    value: f64,
    default: f64,
    range: RangeInclusive<f64>,
    soft: Option<RangeInclusive<f64>>,
    step: f64,
    decimals: usize,
    fill: bool,
    track: Track,
    mark: Mark,
    before_match: Option<f64>,
    quad: Option<Quad<'a>>,
    enabled: bool,
    unit: Option<&'a str>,
}

impl<'a> SliderRow<'a> {
    /// `label` is the short, translated label; the full name defaults to it. `range` is
    /// what a value may hold, and the track spans it unless a soft range is given. The
    /// default value is 0, or the range's start if 0 is outside it.
    pub fn new(label: &'a str, value: f64, range: RangeInclusive<f64>) -> Self {
        let default = 0.0_f64.clamp(*range.start(), *range.end());
        SliderRow {
            label,
            name: label,
            promise: None,
            value,
            default,
            range,
            soft: None,
            step: 0.0,
            decimals: 0,
            fill: true,
            track: Track::Plain,
            mark: Mark::None,
            before_match: None,
            quad: None,
            enabled: true,
            unit: None,
        }
    }

    /// The control's full name, unique in its module: what screen readers say, and the
    /// row's id.
    pub fn name(mut self, full: &'a str) -> Self {
        self.name = full;
        self
    }

    /// The control's promise, in the tooltip.
    pub fn promise(mut self, text: &'a str) -> Self {
        self.promise = Some(text);
        self
    }

    pub fn default_value(mut self, value: f64) -> Self {
        self.default = value;
        self
    }

    /// darktable's soft range: what the track spans at rest, inside the range, which
    /// stays what a value may hold. A value beyond it widens the track to reach it until
    /// the row is reset, as darktable's `dt_bauhaus_slider_set` does.
    pub fn soft_range(mut self, soft: RangeInclusive<f64>) -> Self {
        self.soft = Some(soft);
        self
    }

    /// What a key, or a screen reader's increment, moves the value by while the track
    /// spans its rest: darktable's step there. Once the track widens, and when none is
    /// given, it is darktable's step for the span shown, as its zoom step makes it, never
    /// less; a move is never less than the last decimal shown. A dragged or typed value is
    /// rounded to the decimals shown instead.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// The decimals the value is shown with, darktable's digits. Whether it is signed
    /// follows from the range: signed while it spans zero, as darktable's
    /// `dt_bauhaus_slider_get_text` writes it.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// No fill from the default: for values with no "more" direction, such as a hue.
    pub fn no_fill(mut self) -> Self {
        self.fill = false;
        self
    }

    pub fn track(mut self, track: Track) -> Self {
        self.track = track;
        self
    }

    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = mark;
        self
    }

    /// While a Match proposal waits: the value before it, shown as a tick.
    pub fn before_match(mut self, value: f64) -> Self {
        self.before_match = Some(value);
        self
    }

    pub fn quad(mut self, quad: Quad<'a>) -> Self {
        self.quad = Some(quad);
        self
    }

    /// A disabled row is dimmed and ignores input: a module's Basic sliders in manual mode.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// The value's unit, as the id of the message that writes a value with it, such as
    /// `unit-ev-value` (`{ $value } EV`): the value at rest reads "+0.42 EV", and so do
    /// screen readers, while the value field holds the number alone, named with the unit
    /// its message less `-value` writes, `unit-ev`: "Exposure, in EV".
    pub fn unit(mut self, message: &'a str) -> Self {
        self.unit = Some(message);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<SliderAction> {
        let t = theme(ui.ctx());
        let tr = translator(ui.ctx());
        let id = ui.make_persistent_id(("slider", self.name));
        let (row, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), t.sizes.slider_row),
            Sense::hover(),
        );
        let fade = |c: Color32| {
            if self.enabled {
                c
            } else {
                c.gamma_multiply(0.6)
            }
        };
        let (min, max) = (*self.range.start(), *self.range.end());
        // darktable's sign rule (`dt_bauhaus_slider_get_text`): a sign whenever the range
        // spans zero, zero included.
        let signed = min < 0.0 && max > 0.0;
        let strip = Places::strip(row, &t);
        // Where the marker's centre runs: the strip less the marker's stop at each end.
        let handle = marker(ui.ctx());
        let travel = handle.travel(strip.x_range());
        // What the track spans, darktable's visible range: the soft range at rest, widened
        // to reach each value shown since the row was last reset, never beyond the range,
        // as darktable's `dt_bauhaus_slider_set` widens it. Kept in memory while widened,
        // with the rest it widened from: a moved rest starts the span again (decision 30).
        let span_id = id.with("span");
        let rest = self.soft.as_ref().map_or((min, max), |soft| {
            (soft.start().clamp(min, max), soft.end().clamp(min, max))
        });
        let widened = |(low, high): (f64, f64), v: f64| {
            let v = v.clamp(min, max);
            (low.min(v), high.max(v))
        };
        let kept = ui
            .data(|d| d.get_temp::<((f64, f64), (f64, f64))>(span_id))
            .filter(|(from, _)| *from == rest)
            .map_or(rest, |(_, (low, high))| {
                (low.max(min).min(rest.0), high.min(max).max(rest.1))
            });
        let (low, high) = widened(kept, self.value);
        let span = (high - low).max(f64::EPSILON);
        // A dragged or typed value as the row shows it, rounded to its decimals, as
        // darktable rounds one to its digits.
        let scale = 10f64.powi(self.decimals as i32);
        let round = |v: f64| (v * scale).round() / scale;
        let resolution = scale.recip();
        // What a key moves, as darktable's `dt_bauhaus_slider_get_step` with its zoom step
        // on: the step given while the track spans its rest, and once it widens darktable's
        // step for the span shown, never less. A move is never less than the last decimal
        // shown, darktable's `min_visible`, so Alt's tenth of a step still moves.
        let step = if self.step > 0.0 && (low, high) == rest {
            self.step
        } else {
            zoom_step(low, high).max(self.step)
        };
        let nudge = |delta: f64| {
            if delta != 0.0 && delta.abs() < resolution {
                resolution.copysign(delta)
            } else {
                delta
            }
        };

        let mut actions: Vec<SliderAction> = Vec::new();
        let mut shown = self.value;
        let value_id = id.with("value");
        let open_id = id.with("open");
        let text_id = id.with("value-text");
        // The value field opens on Enter or on a character that starts a number, typed on
        // the focused track: the field, focused at once, holds the number selected, so
        // that character, which it reads in the same frame, takes its place.
        let separator = tr.decimal_separator();
        let starts_number = |c: char| number_char(c, separator);
        let mut keyed_open = false;

        // The track: drag, click, double-click, keys, and a screen reader's actions. It is
        // added after the top line's parts, so that Tab reaches it last, as it reads; egui
        // decides what the pointer did to it as the frame begins, so that is read here.
        let pointer = ui.ctx().read_response(id);
        if self.enabled {
            if let Some(pointer) = &pointer {
                if pointer.double_clicked() {
                    actions.push(SliderAction::Reset);
                } else if let Some(at) = pointer.interact_pointer_pos() {
                    let done = pointer.drag_stopped() || pointer.clicked();
                    if pointer.is_pointer_button_down_on() || done {
                        let x = fraction_at(travel, at.x);
                        let value = round(low + f64::from(x) * span).clamp(low, high);
                        if value != self.value || done {
                            shown = value;
                            actions.push(SliderAction::Set { value, done });
                        }
                        ui.memory_mut(|m| {
                            if !m.has_focus(id) {
                                m.request_focus(id);
                            }
                        });
                    }
                }
            }
            if ui.memory(|m| m.has_focus(id)) {
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
                let keyed = ui.input(|i| {
                    let scale = if i.modifiers.shift {
                        10.0
                    } else if i.modifiers.alt {
                        0.1
                    } else {
                        1.0
                    };
                    let mut delta = 0.0;
                    for (key, sign) in [
                        (Key::ArrowRight, 1.0),
                        (Key::ArrowUp, 1.0),
                        (Key::ArrowLeft, -1.0),
                        (Key::ArrowDown, -1.0),
                        (Key::PageUp, 10.0),
                        (Key::PageDown, -10.0),
                    ] {
                        if i.key_pressed(key) {
                            delta += sign * scale * step;
                        }
                    }
                    if i.key_pressed(Key::Home) {
                        Some(low)
                    } else if i.key_pressed(Key::End) {
                        Some(high)
                    } else if delta != 0.0 {
                        Some((self.value + nudge(delta)).clamp(low, high))
                    } else {
                        None
                    }
                });
                if let Some(value) = keyed {
                    shown = value;
                    actions.push(SliderAction::Set { value, done: true });
                }
                if ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                    actions.push(SliderAction::Reset);
                }
                // Enter is consumed, or the field, focused at once, would take it as its end.
                let entered = ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                let typed = ui.input(|i| {
                    i.events.iter().any(|event| {
                        matches!(event, Event::Text(text)
                            if text.chars().next().is_some_and(starts_number))
                    })
                });
                keyed_open = entered || typed;
            }
            let (more, less, set) = ui.input(|i| {
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
            // A value set may reach the range, as a typed one may; a step stays inside the
            // track's span, as a key's does.
            let requested = set.map(|v| round(v).clamp(min, max)).or_else(|| {
                (more != less).then(|| {
                    let delta = nudge((more as f64 - less as f64) * step);
                    (self.value + delta).clamp(low, high)
                })
            });
            if let Some(value) = requested {
                shown = value;
                actions.push(SliderAction::Set { value, done: true });
            }
        }

        // The value field, open from a click on the value too, until it loses the focus. A
        // click opens it on the next frame: egui takes the focus from a widget a click
        // lands beside, and the field, new under the pointer, is beside it this frame.
        let clicked = self.enabled
            && ui
                .ctx()
                .read_response(text_id)
                .is_some_and(|text| text.clicked());
        let was = ui.data(|d| d.get_temp::<Field>(open_id));
        if clicked {
            ui.data_mut(|d| d.insert_temp(open_id, Field::Clicked));
            ui.ctx().request_repaint();
        }
        // A row disabled since the click opens nothing.
        let opening = self.enabled && (keyed_open || (!clicked && was == Some(Field::Clicked)));
        if opening {
            ui.data_mut(|d| d.insert_temp(open_id, Field::Open));
            ValueField::forget(ui.ctx(), value_id);
            ui.memory_mut(|m| m.request_focus(value_id));
        }
        // The field shows while it has the focus, or is losing it and so sets or puts back
        // the value. One whose focus went while it was not drawn, its row hidden as a tab
        // switched or disabled, or never came, closes unseen, and what was typed in it
        // goes with it, so it never comes back.
        let live = opening
            || ui.memory(|m| m.has_focus(value_id))
            || ui
                .ctx()
                .read_response(value_id)
                .is_some_and(|field| field.lost_focus());
        let state = ui.data(|d| d.get_temp::<Field>(open_id));
        let open = self.enabled && live && state == Some(Field::Open);
        let stale = match state {
            Some(Field::Open) => !open,
            Some(Field::Clicked) => !self.enabled,
            None => false,
        };
        if stale {
            ui.data_mut(|d| d.remove::<Field>(open_id));
            ValueField::forget(ui.ctx(), value_id);
        }
        // A number only, for now (decision 42), without the `+` the value at rest has:
        // typed up to the range, past the track's span, rounded to the decimals. Its name
        // says its unit, `unit-ev` for `unit-ev-value`.
        let unit = self
            .unit
            .and_then(|message| message.strip_suffix("-value"))
            .filter(|word| tr.has(word))
            .map(|word| tr.text(word));
        let mut field = ValueField::new(value_id, self.name, shown, self.range.clone())
            .decimals(self.decimals)
            .number_only()
            .step(resolution);
        if let Some(unit) = &unit {
            field = field.unit(unit);
        }
        // At rest the value as darktable writes it, with its unit, and the hover band 4
        // points before it; the field as wide as its number needs.
        let resting = ui.painter().layout_no_wrap(
            written(&tr, shown, self.decimals, signed, self.unit),
            FontId::proportional(t.text.small),
            fade(t.colors.text),
        );
        let value_width = if open {
            field.width(ui)
        } else {
            resting.size().x + t.spacing.xs + field_inset(&t)
        };
        // The label takes what the rest of the top line leaves, cut with … when long: the
        // value is never cut.
        let quad = self.quad.is_some();
        let body = FontId::proportional(t.text.body);
        let room = Places::across(row, &t, quad, f32::INFINITY, value_width)
            .label
            .width();
        let label_width = ui
            .painter()
            .layout_no_wrap(self.label.to_owned(), body.clone(), Color32::PLACEHOLDER)
            .size()
            .x;
        let label_galley =
            ui.painter()
                .layout_job(truncated(self.label, body, room, fade(t.colors.text)));
        let places = Places::across(row, &t, quad, label_galley.size().x, value_width);

        // The label: a double-click resets. No focus stop: the marker after it is one.
        let label = ui.interact(places.label, id.with("label"), Sense::CLICK);
        if self.enabled && label.double_clicked() {
            actions.push(SliderAction::Reset);
        }

        // The marker.
        let marker = ui.interact(
            places.marker,
            id.with("marker"),
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if self.enabled && marker.clicked() {
            actions.push(SliderAction::ToggleKeep);
        }
        let kept = self.mark == Mark::Kept;
        let mut keep_args = crate::i18n::Args::new();
        keep_args.set("control", self.name);
        let keep_name = tr.text_with("marker-keep", Some(&keep_args));
        marker.widget_info(|| WidgetInfo::selected(Role::Button, self.enabled, kept, &keep_name));
        let marker_tip = tr.text(if kept { "slider-kept" } else { "slider-keep" });

        // The quad and the value field, each in a child that leaves the row's cursor under
        // the row, where a scope would move it back up to its own bottom.
        if let Some(quad) = self.quad {
            let pressed = IconButton::new(quad.icon, quad.name)
                .size(t.sizes.field)
                .toggled(quad.active)
                .enabled(self.enabled)
                .show(&mut ui.new_child(UiBuilder::new().max_rect(places.quad)))
                .clicked();
            if pressed {
                actions.push(SliderAction::Quad);
            }
        }
        // The value: the field while it is open, a focus stop only then; at rest text,
        // which a click opens, no focus stop, since the track tells screen readers the
        // value as it reads.
        let value = if open {
            let (response, typed) =
                field.show(&mut ui.new_child(UiBuilder::new().max_rect(places.value)));
            if let Some(value) = typed {
                shown = value;
                actions.push(SliderAction::Set { value, done: true });
            }
            // Closed once it has set or put back the value. Enter and Escape give the
            // focus back to the track, so the keyboard keeps its place; Tab and a click
            // elsewhere have put it where they meant to.
            if response.lost_focus() {
                ui.data_mut(|d| d.remove::<Field>(open_id));
                if ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Escape)) {
                    ui.memory_mut(|m| m.request_focus(id));
                }
            }
            None
        } else {
            let sense = if self.enabled {
                Sense::CLICK
            } else {
                Sense::hover()
            };
            Some(ui.interact(places.value, text_id, sense))
        };

        // The track shows this frame's value, and keeps its span until a reset, which
        // takes it back to the soft range.
        let (low, high) = widened((low, high), shown);
        let reset = actions.contains(&SliderAction::Reset);
        ui.data_mut(|d| {
            if reset || (low, high) == rest {
                d.remove::<((f64, f64), (f64, f64))>(span_id);
            } else {
                d.insert_temp(span_id, (rest, (low, high)));
            }
        });
        let span = (high - low).max(f64::EPSILON);
        let fraction = |v: f64| ((v - low) / span).clamp(0.0, 1.0) as f32;
        let x_of = |v: f64| x_at(travel, fraction(v));

        // The track, last in the row's focus order. Screen readers hear the value as the
        // row writes it, with its unit, and the key that opens its field.
        let sense = if self.enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let track = ui.interact(places.track, id, sense);
        track.widget_info(|| WidgetInfo::slider(self.enabled, shown, self.name));
        let heard = written(&tr, shown, self.decimals, signed, self.unit);
        let mac = ui.ctx().os() == OperatingSystem::Mac;
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_value(heard);
            node.set_min_numeric_value(min);
            node.set_max_numeric_value(max);
            if self.step > 0.0 {
                node.set_numeric_value_step(self.step);
            }
            node.set_keyboard_shortcut(open_key(mac));
            node.add_action(Action::SetValue);
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
        });

        // Drawing.
        if ui.is_rect_visible(row) {
            let painter = ui.painter();
            painter.galley(
                pos2(
                    places.label.left(),
                    places.label.center().y - label_galley.size().y / 2.0,
                ),
                label_galley,
                fade(t.colors.text),
            );

            let pointer_near =
                ui.rect_contains_pointer(row) || track.has_focus() || marker.has_focus();
            let centre = places.marker.center();
            match self.mark {
                Mark::Auto if self.before_match.is_some() => {
                    let square = Rect::from_center_size(centre, vec2(13.0, 13.0));
                    painter.rect_stroke(
                        square,
                        t.shape.radius_small - 1.0,
                        Stroke::new(1.0, fade(t.colors.text)),
                        egui::StrokeKind::Inside,
                    );
                    painter.text(
                        centre,
                        egui::Align2::CENTER_CENTER,
                        "A",
                        FontId::new(9.0, egui::FontFamily::Name(crate::fonts::SEMIBOLD.into())),
                        fade(t.colors.text),
                    );
                }
                mark => {
                    let hinted = pointer_near && self.enabled;
                    paint_keep_marker(painter, centre, mark == Mark::Kept, hinted, &t, fade);
                }
            }
            if marker.has_focus() {
                focus_ring(painter, places.marker, &t);
            }

            // The value at rest, its end 5 points from the row's, where the field's text
            // sits, so nothing moves as the field opens.
            if let Some(value) = &value {
                if self.enabled && value.hovered() {
                    painter.rect_filled(places.value, t.shape.radius_small, t.colors.hover);
                }
                let end = pos2(
                    places.value.right() - field_inset(&t),
                    places.value.center().y,
                );
                let at = Align2::RIGHT_CENTER.anchor_size(end, resting.size()).min;
                painter.galley(at, resting, fade(t.colors.text));
            }

            let line = places.line;
            match &self.track {
                Track::Plain => {
                    painter.rect_filled(line, t.shape.track / 2.0, fade(t.colors.slider_track));
                }
                Track::Gradient(stops) => {
                    painter.add(gradient(line, stops, self.enabled));
                }
            }
            if self.fill {
                let (a, b) = (x_of(self.default), x_of(shown));
                let moved = Rect::from_x_y_ranges(a.min(b)..=a.max(b), line.y_range());
                let colour = match self.track {
                    Track::Plain => t.colors.slider_fill,
                    Track::Gradient(_) => Color32::from_white_alpha(166),
                };
                painter.rect_filled(moved, 0.0, fade(colour));
            }
            if let Some(before) = self.before_match {
                let tick =
                    Rect::from_center_size(pos2(x_of(before), line.center().y), vec2(2.0, 14.0));
                painter.rect_filled(tick, 0.0, fade(t.colors.accent_text));
            }
            // The marker points at the value, centred on the track's middle, inside the
            // strip, whose ends its stop keeps it within where it crosses the track.
            let centre = pos2(x_of(shown), line.center().y);
            let clipped = painter.with_clip_rect(places.track.intersect(painter.clip_rect()));
            handle.paint(&clipped, centre, &track, &t, fade);
            // Around the whole row, as far out as a dropdown row's hover band, so it runs
            // into neither the top line nor the next row.
            if track.has_focus() {
                focus_ring_centred(painter, row.expand2(vec2(t.spacing.xs, 0.0)), &t);
            }
        }

        // A label cut short is whole in its tooltip, before the promise.
        let mut tip = if label_width > places.label.width() + 0.5 {
            self.label.to_owned()
        } else {
            String::new()
        };
        for line in self
            .promise
            .map(str::to_owned)
            .into_iter()
            .chain(self.before_match.map(|before| {
                let mut args = crate::i18n::Args::new();
                args.set("value", tr.format_number(before, self.decimals, signed));
                tr.text_with("slider-before-match", Some(&args))
            }))
        {
            if !tip.is_empty() {
                tip.push('\n');
            }
            tip.push_str(&line);
        }
        if !tip.is_empty() {
            track.on_hover_text(tip.clone());
            label.on_hover_text(tip);
        }
        marker.on_hover_text(marker_tip);
        if let Some(value) = value
            && self.enabled
        {
            value
                .on_hover_cursor(CursorIcon::Text)
                .on_hover_text(tr.text(if mac {
                    "slider-value-tip-mac"
                } else {
                    "slider-value-tip"
                }));
        }

        // One action a frame: resetting wins over setting, setting over the rest.
        actions.sort_by_key(|a| match a {
            SliderAction::Reset => 0,
            SliderAction::Set { .. } => 1,
            SliderAction::ToggleKeep => 2,
            SliderAction::Quad => 3,
        });
        actions.into_iter().next()
    }
}

/// Where the value field is: clicked open, taking the focus on the next frame, or open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Clicked,
    Open,
}

/// The key that opens the value field on the focused track, as screen readers are told it
/// and as the app's actions spell it: Return on the Mac, Enter elsewhere. It opens
/// darktable's popup once that comes (decision 42).
fn open_key(mac: bool) -> &'static str {
    if mac { "Return" } else { "Enter" }
}

/// A value as darktable's `dt_bauhaus_slider_get_text` writes it: its decimals in the
/// language's format, `+` before zero and above while `signed`, then its unit through
/// `unit`, the message that places it, such as `unit-ev-value`.
fn written(
    tr: &crate::Translator,
    value: f64,
    decimals: usize,
    signed: bool,
    unit: Option<&str>,
) -> String {
    let number = tr.format_number(value, decimals, signed);
    match unit {
        Some(unit) => {
            let mut args = crate::i18n::Args::new();
            args.set("value", number);
            tr.text_with(unit, Some(&args))
        }
        None => number,
    }
}

/// How far the track's line starts under its 16-point strip's top, as the wireframe draws
/// it (`docs/gui.md`, How the rows are laid out): the marker, 5.5 points each way of the
/// line's middle with its outline, then stays inside the strip.
const TRACK_TOP: f32 = 6.0;

/// The x a fraction `f` of the track's span puts the marker at, along its `travel`.
fn x_at(travel: Rangef, f: f32) -> f32 {
    travel.min + f * travel.span()
}

/// The fraction of the track's span a pointer at `x` sets: along the marker's `travel`, so
/// a press at or past either end of the strip gives that end.
fn fraction_at(travel: Rangef, x: f32) -> f32 {
    if travel.span() <= 0.0 {
        return 0.5;
    }
    ((x - travel.min) / travel.span()).clamp(0.0, 1.0)
}

/// Where the row's parts go, from the theme's sizes. The top line, a field high, holds
/// the label and its marker at the start and the quad and the value at the end; under it
/// the track's strip runs the row's full width, with `spacing.xxs` under it, and what the
/// row has left is over the top line.
struct Places {
    label: Rect,
    marker: Rect,
    quad: Rect,
    value: Rect,
    /// The strip: where the pointer drags the value.
    track: Rect,
    /// The track as drawn, `shape.track` thick.
    line: Rect,
}

impl Places {
    /// `label` is the width of the label's text, which gets at most what the rest of the
    /// top line leaves; its marker follows it. `quad`: whether the row has one. `value`:
    /// the value's width, its text and hover band at rest or the field while open, at the
    /// line's end.
    fn across(row: Rect, t: &Theme, quad: bool, label: f32, value: f32) -> Places {
        let gap = t.spacing.xs;
        let s = &t.sizes;
        let track = Places::strip(row, t);
        let line = Rect::from_x_y_ranges(
            row.x_range(),
            track.top() + TRACK_TOP..=track.top() + TRACK_TOP + t.shape.track,
        );
        let top = Rect::from_x_y_ranges(row.x_range(), track.top() - s.field..=track.top());
        let value = Rect::from_x_y_ranges(top.right() - value..=top.right(), top.y_range());
        let quad_rect = Rect::from_x_y_ranges(
            value.left() - gap - s.field..=value.left() - gap,
            top.y_range(),
        );
        let end = if quad { quad_rect.left() } else { value.left() } - gap;
        let room = (end - gap - s.marker_column - top.left()).max(0.0);
        let label = Rect::from_min_size(top.min, vec2(label.min(room), top.height()));
        let marker = Rect::from_min_size(
            pos2(label.right() + gap, top.top()),
            vec2(s.marker_column, top.height()),
        );
        Places {
            label,
            marker,
            quad: quad_rect,
            value,
            track,
            line,
        }
    }

    /// The track's strip, the row's full width at its bottom, with `spacing.xxs` under it.
    fn strip(row: Rect, t: &Theme) -> Rect {
        let bottom = row.bottom() - t.spacing.xxs;
        Rect::from_x_y_ranges(row.x_range(), bottom - t.sizes.track_strip..=bottom)
    }
}

/// darktable's `dt_bauhaus_slider_get_step` for a slider given none, over the span from
/// `low` to `high` in the units shown: a hundredth of its top, the smaller of its width
/// and its larger end's size, taken to 1 or 5 times a power of ten, and 1 from a top of
/// 100.
fn zoom_step(low: f64, high: f64) -> f64 {
    let top = (high - low).min(low.abs().max(high.abs()));
    if top >= 100.0 {
        return 1.0;
    }
    let log = (top / 100.0).log10();
    let digits = (log + 0.1).floor();
    let step = 10f64.powf(digits);
    if log - digits > 0.5 { step * 5.0 } else { step }
}

/// A colour band across `rect`, dimmed when the row is disabled.
fn gradient(rect: Rect, stops: &[(f32, Color32)], enabled: bool) -> Mesh {
    let mut mesh = Mesh::default();
    let place = |f: f32| rect.left() + f.clamp(0.0, 1.0) * rect.width();
    for (i, &(at, colour)) in stops.iter().enumerate() {
        let colour = if enabled {
            colour
        } else {
            colour.gamma_multiply(0.6)
        };
        let x = place(at);
        mesh.colored_vertex(Pos2::new(x, rect.top()), colour);
        mesh.colored_vertex(Pos2::new(x, rect.bottom()), colour);
        if i > 0 {
            let n = mesh.vertices.len() as u32;
            mesh.add_triangle(n - 4, n - 3, n - 2);
            mesh.add_triangle(n - 3, n - 1, n - 2);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use egui::Rangef;

    use super::{fraction_at, x_at};
    use crate::components::Marker;

    /// A 300-point track, as a 320-point side panel's rows have.
    const TRACK: Rangef = Rangef {
        min: 10.0,
        max: 310.0,
    };

    #[test]
    fn the_ends_put_each_marker_flush_with_the_track_s_ends() {
        for marker in Marker::ALL {
            let travel = marker.travel(TRACK);
            assert_eq!(x_at(travel, 0.0), TRACK.min + marker.stop());
            assert_eq!(x_at(travel, 1.0), TRACK.max - marker.stop());
        }
    }

    #[test]
    fn the_default_and_the_fill_s_start_follow_the_same_travel() {
        // Exposure's default, 0 on −4 to +4, half way along: the track's middle for every
        // marker, since each stops alike at both ends.
        for marker in Marker::ALL {
            let travel = marker.travel(TRACK);
            assert_eq!(x_at(travel, 0.5), 160.0);
        }
        // A quarter of the way, inside the square's travel, not the track's.
        let travel = Marker::Square.travel(TRACK);
        assert_eq!(x_at(travel, 0.25), 15.75 + 0.25 * 288.5);
    }

    #[test]
    fn a_pointer_at_the_track_s_very_end_gives_its_end() {
        for marker in Marker::ALL {
            let travel = marker.travel(TRACK);
            assert_eq!(fraction_at(travel, TRACK.min), 0.0);
            assert_eq!(fraction_at(travel, TRACK.max), 1.0);
            // Anywhere inside the stop is the end too, and past it.
            assert_eq!(fraction_at(travel, TRACK.min + marker.stop() / 2.0), 0.0);
            assert_eq!(fraction_at(travel, TRACK.max + 20.0), 1.0);
            // The marker's own place maps back to its fraction.
            assert!((fraction_at(travel, x_at(travel, 0.3)) - 0.3).abs() < 1e-6);
        }
    }

    #[test]
    fn a_track_too_short_for_the_stops_keeps_the_marker_in_its_middle() {
        let travel = Marker::Circle.travel(Rangef::new(0.0, 10.0));
        assert_eq!(x_at(travel, 0.0), 5.0);
        assert_eq!(x_at(travel, 1.0), 5.0);
        assert_eq!(fraction_at(travel, 0.0), 0.5);
    }
}
