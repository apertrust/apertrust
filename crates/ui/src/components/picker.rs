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

//! A colour picker's button and the active picker's marks on the photo (`docs/pickers.md`,
//! The picker button and On the photo), darktable's (`src/gui/color_picker_proxy.c`,
//! `src/views/darkroom.c:331-470` at the reference release).
//!
//! [`PickerButton`] draws whether its picker is on and reports the press as darktable's
//! `_color_picker_callback_button_press` reads it: on as a point or as an area, or off.
//! [`PickerMarks`] draws the active picker's point or area with the overlay's primitives
//! ([`crate::overlay`]). Neither knows the engine's types: the renderer hands them the state
//! and the place, and turns what the button reports into the session's command.

use egui::os::OperatingSystem;
use egui::{Color32, Key, Modifiers, Pos2, Rect, Response, Ui};

use super::{ButtonState, IconButton};
use crate::i18n::Args;
use crate::overlay::{Mark, Overlay, Point, Weight};
use crate::{Icon, theme, translator};

/// What the user did to a [`PickerButton`] this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerAction {
    /// Turn the picker on: as an area where `area`, for a picker that takes both kinds;
    /// else as a point, or as the one kind it takes.
    On { area: bool },
    /// Turn it off: it was pressed while on, as darktable's toggle button and quad turn
    /// their picker off (`color_picker_proxy.c:237-252`).
    Off,
}

/// A picker's button: Lucide's pipette, an [`IconButton`] toggle with no plate (decision
/// 81): while on, picking, the filled pipette (`pick-filled`) in `text_strong`; its other
/// states an icon button's.
///
/// - **The press** (`color_picker_proxy.c:177-193`, `:237-252`): a click while off turns
///   it on; a Ctrl-click turns it on as an area, Ctrl alone held as darktable's
///   `dt_modifier_is` compares the whole mask, so Ctrl+Shift is a click (on the Mac Cmd, or
///   Control, the Mac's right-click); any press while on turns it off.
/// - **A toggle button,** darktable's `dtgtk_togglebutton`, also takes a right-click as an
///   area (`:178-179`). **A quad** ([`PickerButton::quad`]) takes every mouse button alike,
///   as darktable's bauhaus quad does, calling back with no event (`:259-263`,
///   `:422-424`), so only Ctrl picks an area there.
/// - **Keys,** while it has the focus: Space or Enter as a click; Shift+F10 or the Menu key
///   as an area, on a quad too, the keyboard's way to one.
/// - **To AccessKit,** a toggle button named by its full name, pressed while on, its
///   tooltip its description.
pub struct PickerButton<'a> {
    name: &'a str,
    on: bool,
    tip: Option<&'a str>,
    area: bool,
    quad: bool,
    disabled: Option<&'a str>,
    shortcut: Option<&'a str>,
    state: Option<ButtonState>,
}

impl<'a> PickerButton<'a> {
    /// `name` is translated: its full name, the module's and the picker's words ("Exposure:
    /// pick from the photo"), what screen readers say. `on` while its picker is the active
    /// one.
    pub fn new(name: &'a str, on: bool) -> Self {
        PickerButton {
            name,
            on,
            tip: None,
            area: false,
            quad: false,
            disabled: None,
            shortcut: None,
            state: None,
        }
    }

    /// Its tooltip, translated: darktable's words for it ("set the exposure adjustment
    /// using the selected area"). By default darktable's quad tooltip, "pick color from
    /// image" (`color_picker_proxy.c:415`), `picker-tip`.
    pub fn tip(mut self, tip: &'a str) -> Self {
        self.tip = Some(tip);
        self
    }

    /// For a picker that takes a point or an area: its tooltip ends with darktable's
    /// "ctrl+click or right-click to select an area", on a quad "ctrl+click to select an
    /// area" (`colorequal.c:3010-3011`), Cmd on the Mac. Where darktable's own tooltip
    /// already holds that line (`rgbcurve.c:1500-1501`, `libs/colorpicker.c:778`), the
    /// module's message holds only its first line, and this adds the rest.
    pub fn point_or_area(mut self, both: bool) -> Self {
        self.area = both;
        self
    }

    /// A slider's or a choice's quad rather than a toggle button of its own: every mouse
    /// button presses it alike, as darktable's quad does, so a right-click is a click and
    /// only Ctrl picks an area.
    pub fn quad(mut self, quad: bool) -> Self {
        self.quad = quad;
        self
    }

    /// Dimmed, ignoring input, with `why`, translated, as its tooltip: on a photo not yet
    /// live and, for a module's picker, on a photo open for viewing only.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    /// The keys that do the same, in the platform's spelling, shown in the tooltip and told
    /// to screen readers: the Colour picker's Pick colour action where it has keys.
    pub fn shortcut(mut self, keys: &'a str) -> Self {
        self.shortcut = Some(keys);
        self
    }

    /// Draws it in `state` whatever the pointer and the keys do, as the gallery shows each
    /// state.
    pub fn state(mut self, state: ButtonState) -> Self {
        self.state = Some(state);
        self
    }

    /// Draws it and reports what was pressed.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<PickerAction>) {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let enabled = self.disabled.is_none() && self.state != Some(ButtonState::Disabled);
        let tip = match self.disabled {
            Some(why) => why.to_owned(),
            None => self.words(&ctx),
        };
        let mut button = IconButton::new(Icon::Pick, self.name)
            .tip(&tip)
            .toggled(self.on)
            .enabled(enabled)
            .size(t.sizes.field);
        if let Some(state) = self.state {
            button = button.state(state);
        }
        if let Some(keys) = self.shortcut {
            button = button.shortcut(keys);
        }
        let response = button.show(ui);
        ctx.accesskit_node_builder(response.id, |node| node.set_description(tip.as_str()));

        // The keyboard's right-click.
        let keyed = enabled
            && response.has_focus()
            && ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::Menu)
                    || i.consume_key(Modifiers::SHIFT, Key::F10)
            });
        let pressed = response.clicked() || response.secondary_clicked();
        let action = if self.on && (pressed || keyed) {
            Some(PickerAction::Off)
        } else if keyed || (!self.quad && response.secondary_clicked()) {
            Some(PickerAction::On { area: true })
        } else if pressed {
            let modifiers = ui.input(|i| i.modifiers);
            Some(PickerAction::On {
                area: command_alone(modifiers),
            })
        } else {
            None
        };
        (response, action)
    }

    /// The tooltip while enabled: the picker's words, then for one that takes both kinds
    /// how to choose an area, its modifier as the app spells its shortcuts.
    fn words(&self, ctx: &egui::Context) -> String {
        let tr = translator(ctx);
        let words = match self.tip {
            Some(tip) => tip.to_owned(),
            None => tr.text("picker-tip"),
        };
        if !self.area {
            return words;
        }
        let mut args = Args::new();
        args.set("tip", words);
        let mac = ctx.os() == OperatingSystem::Mac;
        args.set("command", if mac { "Cmd" } else { "Ctrl" });
        let id = if self.quad {
            "picker-tip-area-quad"
        } else {
            "picker-tip-area"
        };
        tr.text_with(id, Some(&args))
    }
}

/// Whether `modifiers` are Ctrl alone, Cmd or Control on the Mac, the Mac's right-click: as
/// darktable's `dt_modifier_is(state, GDK_CONTROL_MASK)`, which compares the whole mask, so
/// Shift or Alt held with it makes a plain click.
fn command_alone(modifiers: Modifiers) -> bool {
    (modifiers.command || modifiers.mac_cmd || modifiers.ctrl) && !modifiers.shift && !modifiers.alt
}

/// Where the active picker is on the photo, in the photo's coordinates, 0 to 1 across the
/// photo as it is shown, (0, 0) its top left: the region the picker's event gives, as drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PickerPlace {
    /// A point, with the colour it sampled as the screen shows it: the reading's frame mean,
    /// or the statistic the Colour picker module shows.
    Point { at: Pos2, colour: Color32 },
    /// An area, from its top left to its bottom right.
    Area(Rect),
}

/// The active picker's marks on the photo, darktable's `_darkroom_pickers_draw` for the
/// primary sample (`darkroom.c:331-470`) in the overlay's language: an area's rectangle at
/// the normal weight with a square handle on each corner (`:389-397`); a point's circle of 4
/// half-pixels with a crosshair through it and its sampled colour at the centre
/// (`:400-431`, `:457-468`). Focused, the theme's ring round it, shown only after a key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerMarks {
    place: PickerPlace,
    focused: bool,
}

impl PickerMarks {
    pub fn new(place: PickerPlace) -> Self {
        PickerMarks {
            place,
            focused: false,
        }
    }

    /// While the picker's point or area on the photo has the keyboard's focus.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Paints the marks on `overlay`, the photo's for this frame.
    pub fn paint(&self, overlay: &Overlay) {
        match self.place {
            PickerPlace::Area(area) => {
                overlay.stroke(&Mark::new(Weight::Normal).rect(area).corners(area));
                if self.focused {
                    overlay.focus_rect(area, Weight::Normal);
                }
            }
            PickerPlace::Point { at, colour } => {
                overlay.point(at, Point::Active, colour, Weight::Normal);
                if self.focused {
                    let reach = overlay.reach(Point::Active, Weight::Normal);
                    overlay.focus_circle(at, reach, Weight::Normal);
                }
            }
        }
    }
}
