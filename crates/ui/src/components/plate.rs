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

//! What checkboxes, text buttons and icon buttons share (`docs/gui.md`, Checkboxes, text
//! buttons and icon buttons): the state each is drawn in, the plate under what is pressed,
//! and the focus ring with its gap of ground.
//!
//! What you press is a plate a step apart from any ground, `button`: a step further under
//! the pointer, a step back while pressed, with no border but the high-contrast theme's
//! `button_edge`. Focus never changes the fill; it is one ring for the three controls.

use egui::{Color32, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use super::focus_shows;
use crate::Theme;

/// A state to draw a checkbox, a text button or an icon button in whatever the pointer and
/// the keys do, as the gallery shows each control's states side by side
/// (`Button::state`, `Checkbox::state`, `IconButton::state`, `ToolButton::state`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonState {
    #[default]
    Rest,
    Hover,
    Pressed,
    /// Focused by a key: the ring.
    Focus,
    Disabled,
}

impl ButtonState {
    pub const ALL: [ButtonState; 5] = [
        ButtonState::Rest,
        ButtonState::Hover,
        ButtonState::Pressed,
        ButtonState::Focus,
        ButtonState::Disabled,
    ];
}

/// How a control is touched as it is drawn.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Touch {
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    /// Focused while focus rings show: after a key, not after a click.
    pub focused: bool,
}

impl Touch {
    /// From the pointer and the keys on `response`, or as `pinned` says.
    pub fn of(
        ctx: &egui::Context,
        response: &Response,
        enabled: bool,
        pinned: Option<ButtonState>,
    ) -> Touch {
        match pinned {
            Some(state) => Touch {
                enabled: enabled && state != ButtonState::Disabled,
                hovered: state == ButtonState::Hover,
                pressed: state == ButtonState::Pressed,
                focused: state == ButtonState::Focus,
            },
            None => Touch {
                enabled,
                hovered: enabled && response.hovered(),
                pressed: enabled && response.is_pointer_button_down_on(),
                focused: response.has_focus() && focus_shows(ctx),
            },
        }
    }

    /// Whether an icon in the control is lit: under the pointer, pressed or focused by a
    /// key (`icon_colour`).
    pub fn lit(&self) -> bool {
        self.enabled && (self.hovered || self.pressed || self.focused)
    }
}

/// Allocates a text button or a checkbox `size` big, clickable while `enabled`. A disabled
/// one is allocated in a disabled scope, as egui's own are: it ignores input, takes no
/// focus, is said disabled to screen readers and keeps a tooltip given with
/// `on_disabled_hover_text` whatever lies over it. Paint it with the caller's painter, at
/// full strength, since its tokens dim it.
pub(crate) fn allocate(ui: &mut Ui, size: Vec2, enabled: bool) -> (Rect, Response) {
    if enabled {
        return ui.allocate_exact_size(size, Sense::click());
    }
    ui.scope(|ui| {
        ui.disable();
        ui.allocate_exact_size(size, Sense::click())
    })
    .inner
}

/// `ui`'s painter at the strength its own fades give it, but a disabled parent's: egui fades
/// everything in a disabled `Ui` by `disabled_alpha`, and these controls dim by their tokens
/// instead, so a disabled one looks the same whether it or its parent is disabled.
pub(crate) fn undimmed_painter(ui: &Ui) -> Painter {
    let mut painter = ui.painter().clone();
    if !ui.is_enabled() {
        let faded = ui.visuals().disabled_alpha();
        if faded > 0.0 {
            painter.set_opacity((painter.opacity() / faded).min(1.0));
        }
    }
    painter
}

/// The plate a control is drawn on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Plate {
    /// A text button's: always there.
    Default,
    /// A dialog's one primary button: amber, and drawn as a default one while disabled,
    /// so amber always means it will act.
    Primary,
    /// An icon button's and a word's on a bar: there only while touched.
    Quiet,
}

/// The fill and edge of `plate` when `touch`ed, none where it shows no plate.
pub(crate) fn plate(t: &Theme, plate: Plate, touch: Touch) -> Option<(Color32, Stroke)> {
    let c = &t.colors;
    let edge = |colour| Stroke::new(t.shape.hairline, colour);
    if !touch.enabled {
        // A disabled plate keeps its shape, its edge, where there is one, dimmed as the
        // words are. A quiet one shows none.
        let colour = if c.button_edge.a() > 0 {
            c.text_disabled
        } else {
            c.button_edge
        };
        return (plate != Plate::Quiet).then_some((c.button, edge(colour)));
    }
    let fill = match (plate, touch.pressed, touch.hovered) {
        (Plate::Primary, true, _) => c.accent_pressed,
        (Plate::Primary, false, true) => c.accent_hover,
        (Plate::Primary, false, false) => c.accent,
        (_, true, _) => c.button_pressed,
        (Plate::Default, false, true) => c.button_hover,
        (Plate::Default, false, false) => c.button,
        (Plate::Quiet, false, true) => c.button,
        (Plate::Quiet, false, false) => return None,
    };
    let stroke = if plate == Plate::Primary {
        Stroke::NONE
    } else {
        edge(c.button_edge)
    };
    Some((fill, stroke))
}

/// Paints `plate` behind a control in `rect`.
pub(crate) fn paint_plate(painter: &Painter, rect: Rect, plate_: Plate, touch: Touch, t: &Theme) {
    if let Some((fill, stroke)) = plate(t, plate_, touch) {
        painter.rect(rect, t.shape.radius, fill, stroke, StrokeKind::Inside);
    }
}

/// The words on `plate` when `touch`ed.
pub(crate) fn words_colour(t: &Theme, plate: Plate, touch: Touch) -> Color32 {
    if !touch.enabled {
        t.colors.text_disabled
    } else if plate == Plate::Primary {
        t.colors.text_on_accent
    } else {
        t.colors.text
    }
}

/// The ring round a focused checkbox, text button or icon button whose corners are
/// `radius`: 1.5 points of `focus_ring`, `shape.focus_gap` of ground between it and the
/// control. Only where `touch` is focused.
pub(crate) fn paint_focus(painter: &Painter, rect: Rect, radius: f32, touch: Touch, t: &Theme) {
    if !touch.focused {
        return;
    }
    let gap = t.shape.focus_gap;
    painter.rect_stroke(
        rect.expand(gap),
        radius + gap,
        Stroke::new(1.5, t.colors.focus_ring),
        StrokeKind::Outside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Builtin;

    fn touched(enabled: bool, hovered: bool, pressed: bool) -> Touch {
        Touch {
            enabled,
            hovered,
            pressed,
            focused: false,
        }
    }

    #[test]
    fn hover_lifts_a_plate_and_a_press_dips_it_with_no_border() {
        let t = Theme::builtin(Builtin::Dark);
        let c = &t.colors;
        let fill = |p, touch| plate(&t, p, touch).map(|(fill, _)| fill);
        let rest = touched(true, false, false);
        let hover = touched(true, true, false);
        let pressed = touched(true, true, true);
        assert_eq!(fill(Plate::Default, rest), Some(c.button));
        assert_eq!(fill(Plate::Default, hover), Some(c.button_hover));
        assert_eq!(fill(Plate::Default, pressed), Some(c.button_pressed));
        assert_eq!(fill(Plate::Primary, rest), Some(c.accent));
        assert_eq!(fill(Plate::Primary, hover), Some(c.accent_hover));
        assert_eq!(fill(Plate::Primary, pressed), Some(c.accent_pressed));
        assert_eq!(fill(Plate::Quiet, rest), None);
        assert_eq!(fill(Plate::Quiet, hover), Some(c.button));
        assert_eq!(fill(Plate::Quiet, pressed), Some(c.button_pressed));
        // No border appears under the pointer or on a press in the dark theme.
        for p in [Plate::Default, Plate::Primary, Plate::Quiet] {
            for touch in [hover, pressed] {
                let (_, stroke) = plate(&t, p, touch).unwrap();
                assert_eq!(stroke.color.a(), 0, "{p:?}");
            }
        }
        // Focus changes no fill.
        let focused = Touch {
            focused: true,
            ..rest
        };
        assert_eq!(fill(Plate::Default, focused), Some(c.button));
        assert_eq!(fill(Plate::Quiet, focused), None);
    }

    #[test]
    fn a_disabled_primary_button_is_a_disabled_default_one() {
        for which in Builtin::ALL {
            let t = Theme::builtin(which);
            let off = touched(false, true, true);
            assert_eq!(
                plate(&t, Plate::Primary, off),
                plate(&t, Plate::Default, off)
            );
            assert_eq!(plate(&t, Plate::Quiet, off), None);
            assert_eq!(
                words_colour(&t, Plate::Primary, off),
                t.colors.text_disabled
            );
        }
    }

    #[test]
    fn every_plate_has_an_edge_in_high_contrast() {
        let t = Theme::builtin(Builtin::HighContrast);
        let c = &t.colors;
        for (p, touch) in [
            (Plate::Default, touched(true, false, false)),
            (Plate::Default, touched(true, true, false)),
            (Plate::Quiet, touched(true, true, false)),
            (Plate::Quiet, touched(true, true, true)),
        ] {
            let (_, stroke) = plate(&t, p, touch).unwrap();
            assert_eq!(stroke, Stroke::new(t.shape.hairline, c.button_edge));
        }
        let (_, disabled) = plate(&t, Plate::Default, touched(false, false, false)).unwrap();
        assert_eq!(disabled.color, c.text_disabled);
    }
}
