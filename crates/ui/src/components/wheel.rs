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

//! The scroll wheel over a row and the keys' fine step (`docs/gui.md`, The scroll wheel
//! over a row), as darktable's bauhaus widgets take them (`_widget_scroll`,
//! `_slider_add_step`, `dt_gui_ignore_scroll`).
//!
//! - **Mouse wheel scrolls the Tools panel**, on by default ([`set_wheel_scrolls_panel`]):
//!   a plain scroll scrolls the panel, and only with the finer modifier, Ctrl or Option on
//!   the Mac, does it change a slider by its step, Shift with it ten steps, or step a choice
//!   row's options. Off, darktable's default (`darkroom/ui/sidebar_scroll_default`), the
//!   wheel over a row changes it, one step a notch, Shift ten, the finer modifier a tenth.
//! - A row takes the wheel only where it is given a [`Wheel`]: the Tools panel's, never the
//!   Settings dialog's. Export's rows take it only while they have the focus (decision 90).
//! - A notch is a line of the wheel, a mouse wheel's click. A trackpad's distance in points
//!   is made notches as darktable's `_scroll_attenuate` makes the Mac's, and either kind
//!   adds up in the row until it makes a whole notch, darktable's discrete scrolling; the
//!   system's momentum after the fingers lift never changes a row.
//! - A row that takes a notch keeps the panel still for the rest of that scroll.

use egui::os::OperatingSystem;
use egui::{Event, Id, Modifiers, MouseWheelSource, MouseWheelUnit, Rect, TouchPhase, Ui, Vec2};

/// Where a row takes the scroll wheel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wheel {
    /// Never: the wheel scrolls what holds the row, as in the Settings dialog.
    #[default]
    Off,
    /// Under the pointer: a tool's rows in the Tools panel.
    Pointer,
    /// Under the pointer while it has the focus: the Export tool's, so a scroll through its
    /// settings never changes where or what it writes (decision 90).
    Focused,
}

/// Sets whether the mouse wheel scrolls the Tools panel, `tools_panel.wheel_scrolls_panel`:
/// on, a row takes it only with the finer modifier; off, as darktable's default, it takes
/// it plain.
pub fn set_wheel_scrolls_panel(ctx: &egui::Context, on: bool) {
    ctx.data_mut(|data| data.insert_temp(setting_id(), on));
}

/// Whether the mouse wheel scrolls the Tools panel; on until [`set_wheel_scrolls_panel`]
/// says otherwise.
pub fn wheel_scrolls_panel(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(setting_id()))
        .unwrap_or(true)
}

fn setting_id() -> Id {
    Id::unique("apertrust-ui wheel scrolls panel")
}

/// Whether `m` holds the finer modifier, darktable's Ctrl: Option on the Mac, where Ctrl
/// with the wheel is the system's screen zoom and Ctrl with the arrows its spaces.
pub(crate) fn finer(os: OperatingSystem, m: Modifiers) -> bool {
    if os == OperatingSystem::Mac {
        m.alt
    } else {
        m.ctrl || m.command
    }
}

/// What a key's step is multiplied by with `m` held, darktable's multipliers: Shift ten,
/// the finer modifier a tenth ([`finer`]), else one. None where `m` leaves the key to the
/// app: Cmd or Ctrl on the Mac, Alt elsewhere.
pub(crate) fn key_scale(os: OperatingSystem, m: Modifiers) -> Option<f64> {
    let app = if os == OperatingSystem::Mac {
        m.command || m.mac_cmd || m.ctrl
    } else {
        m.alt || m.mac_cmd
    };
    if app {
        None
    } else if m.shift {
        Some(10.0)
    } else if finer(os, m) {
        Some(0.1)
    } else {
        Some(1.0)
    }
}

/// The whole notches a row took this frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Notches {
    /// The notches in the order they came, as runs that came with the same modifiers: their
    /// whole notches, signed, up positive, and the steps each makes on a slider, one, ten
    /// with Shift and a tenth with the finer modifier while the wheel changes rows plain.
    /// A slider moves each notch by its own steps, never less than the value's last
    /// decimal; a choice an option back a notch up, whatever is held, as darktable's
    /// combobox.
    pub steps: Vec<(i32, f64)>,
}

/// The notches the row `id` at `rect` takes from this frame's wheel, by `wheel`'s rule,
/// none while it is disabled. A row that took a notch keeps the panel still for the rest of
/// that scroll, the frames egui spreads a notch over and a trackpad's gesture, its momentum
/// included, even once the pointer has left it. What is less than a notch waits in the row
/// for the next.
pub(crate) fn notches(ui: &Ui, id: Id, rect: Rect, wheel: Wheel, enabled: bool) -> Option<Notches> {
    if wheel == Wheel::Off {
        return None;
    }
    let kept = id.with("wheel");
    let scrolling = ui.input(|i| i.is_scrolling());
    let mut turning = ui.data(|d| d.get_temp::<Turning>(kept)).unwrap_or_default();
    let under = match wheel {
        Wheel::Off | Wheel::Pointer => true,
        Wheel::Focused => ui.memory(|m| m.has_focus(id)),
    } && enabled
        && ui.is_enabled()
        && ui.rect_contains_pointer(rect);
    if !under {
        // Still the scroll it took a notch of, while no new scroll begins elsewhere: a mouse
        // wheel's notch, or a trackpad gesture's start, never the rest of the same gesture
        // or its momentum. What was left of a notch goes.
        let new_scroll = ui.input(|i| {
            i.events.iter().any(|event| {
                matches!(event, Event::MouseWheel { source, phase, .. }
                    if *source == MouseWheelSource::Wheel || *phase == TouchPhase::Start)
            })
        });
        if turning.taking && scrolling && !new_scroll {
            ui.ctx().input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            let taking = Turning {
                left: 0.0,
                taking: true,
            };
            ui.data_mut(|d| d.insert_temp(kept, taking));
        } else {
            ui.data_mut(|d| d.remove::<Turning>(kept));
        }
        return None;
    }
    let os = ui.ctx().os();
    let plain = !wheel_scrolls_panel(ui.ctx());
    let takes = |m: Modifiers| plain || finer(os, m);
    // Each event's whole notches scaled by its own modifiers.
    let (mut steps, mut took, mut left_to_panel) = (Vec::new(), false, false);
    // What the frame's events the row does not take scroll the panel, in points, as egui
    // counts them.
    let line = ui.ctx().options(|o| o.input_options.line_scroll_speed);
    let page = ui.ctx().content_rect().height();
    let mut panels = Vec2::ZERO;
    ui.input(|i| {
        for event in &i.events {
            let Event::MouseWheel {
                unit,
                delta,
                phase,
                source,
                modifiers,
            } = event
            else {
                continue;
            };
            if *source == MouseWheelSource::Momentum {
                continue;
            }
            // A new gesture starts from nothing, whatever is held: what was left of an
            // earlier one is not carried into it.
            if *phase == TouchPhase::Start {
                turning.left = 0.0;
            }
            if !takes(*modifiers) {
                // The person scrolls on without the modifier: the panel's again.
                left_to_panel = true;
                panels += match unit {
                    MouseWheelUnit::Point => *delta,
                    MouseWheelUnit::Line => *delta * line,
                    MouseWheelUnit::Page => *delta * page,
                };
                continue;
            }
            {
                let total = turning.left + in_notches(*unit, *delta);
                let whole = total.trunc();
                turning.left = total - whole;
                let scale = if modifiers.shift {
                    10.0
                } else if plain && finer(os, *modifiers) {
                    0.1
                } else {
                    1.0
                };
                if whole != 0.0 {
                    steps.push((whole as i32, scale));
                }
                took = true;
            }
        }
    });
    turning.taking = took || (turning.taking && scrolling && !left_to_panel);
    if turning.taking {
        // The notch taken never scrolls the panel; in a frame that also brings a scroll
        // without the modifier, that scroll still does.
        ui.ctx().input_mut(|i| i.smooth_scroll_delta = panels);
    }
    ui.data_mut(|d| d.insert_temp(kept, turning));
    // Notches that cancel in their count may still move a slider: a plain one up and a
    // Shift one down are nine steps down.
    (!steps.is_empty()).then_some(Notches { steps })
}

/// A row's wheel between frames: what is left of a notch, and whether it took a notch of
/// the scroll under way.
#[derive(Clone, Copy, Debug, Default)]
struct Turning {
    left: f32,
    taking: bool,
}

/// A wheel event's `delta` in notches, up and left positive: a line or a page is one, as a
/// mouse wheel's click; a trackpad's points darktable's way (`_scroll_attenuate` on the
/// Mac), 0.06 × points^0.7, so a move of 55 points in one event is a notch and small ones
/// count for more, a notch in about 20 points of two.
fn in_notches(unit: MouseWheelUnit, delta: Vec2) -> f32 {
    match unit {
        MouseWheelUnit::Line | MouseWheelUnit::Page => delta.x + delta.y,
        MouseWheelUnit::Point => {
            let attenuate = |d: f32| (0.06 * d.abs().powf(0.7)).copysign(d);
            attenuate(delta.x) + attenuate(delta.y)
        }
    }
}

#[cfg(test)]
mod tests {
    use egui::os::OperatingSystem;
    use egui::{Modifiers, MouseWheelUnit, vec2};

    use super::{finer, in_notches, key_scale};

    const MAC: OperatingSystem = OperatingSystem::Mac;
    const NIX: OperatingSystem = OperatingSystem::Nix;

    #[test]
    fn the_finer_modifier_is_option_on_the_mac_and_ctrl_elsewhere() {
        assert!(finer(MAC, Modifiers::ALT));
        assert!(!finer(MAC, Modifiers::CTRL));
        assert!(!finer(MAC, Modifiers::MAC_CMD | Modifiers::COMMAND));
        assert!(finer(NIX, Modifiers::CTRL | Modifiers::COMMAND));
        assert!(!finer(NIX, Modifiers::ALT));
    }

    #[test]
    fn a_key_s_step_is_ten_with_shift_and_a_tenth_with_the_finer_modifier() {
        let ctrl = Modifiers::CTRL | Modifiers::COMMAND;
        assert_eq!(key_scale(NIX, Modifiers::NONE), Some(1.0));
        assert_eq!(key_scale(NIX, Modifiers::SHIFT), Some(10.0));
        assert_eq!(key_scale(NIX, ctrl), Some(0.1));
        assert_eq!(key_scale(NIX, ctrl | Modifiers::SHIFT), Some(10.0));
        // Alt is the app's on Windows and Linux, as Cmd and Ctrl are on the Mac.
        assert_eq!(key_scale(NIX, Modifiers::ALT), None);
        assert_eq!(key_scale(MAC, Modifiers::ALT), Some(0.1));
        assert_eq!(
            key_scale(MAC, Modifiers::MAC_CMD | Modifiers::COMMAND),
            None
        );
        assert_eq!(key_scale(MAC, Modifiers::CTRL), None);
    }

    #[test]
    fn a_line_is_a_notch_and_points_add_up_as_darktable_s_mac_scroll() {
        assert_eq!(in_notches(MouseWheelUnit::Line, vec2(0.0, 1.0)), 1.0);
        assert_eq!(in_notches(MouseWheelUnit::Line, vec2(0.0, -3.0)), -3.0);
        // Shift's scroll, sideways on the Mac, counts as much.
        assert_eq!(in_notches(MouseWheelUnit::Line, vec2(1.0, 0.0)), 1.0);
        let small = in_notches(MouseWheelUnit::Point, vec2(0.0, 2.0));
        assert!((0.09..0.1).contains(&small), "{small}");
        let large = in_notches(MouseWheelUnit::Point, vec2(0.0, -100.0));
        assert!((-1.6..-1.5).contains(&large), "{large}");
    }
}
