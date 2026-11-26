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

//! The Settings dialog's own components (`docs/gui.md`, The Settings dialog): its row and
//! the controls a row holds. They share the tools' tokens and a row's behaviour, not their
//! sizes or looks: every box is `sizes.setting_control` high (24) in the body text, in a
//! column `sizes.setting_column` wide (260) at the row's end, which boxes and wells fill and
//! at whose end a switch, a number and a button sit.
//!
//! - [`SettingRow`]: the label, the changed mark after it, the restart note, the column,
//!   the dimmed state.
//! - [`Combo`] and [`Segmented`] for a choice, [`Switch`] for a boolean, [`Slider`] and
//!   [`Number`] for a number, [`Text`], [`TextArea`] and [`Folder`] for a text, [`Chips`]
//!   for a list of a choice, [`Readout`] for a value shown and [`Button`] for an action.
//! - [`Search`], Find a setting at the top of the page list.
//!
//! A box is `raised` (a combo, a button); a well is `sunken` (a segmented choice, a field, a
//! number, the chips); corners are `shape.radius`. Each is drawn by its fill alone, with no
//! frame, but in a theme whose plates have an edge (`button_edge`, the high-contrast
//! theme's), where wells and boxes would otherwise be the panel's black (`edge`). Each
//! takes its width from the `Ui` it is drawn in, the row's column.

mod button;
mod chips;
mod combo;
mod number;
mod readout;
mod row;
mod search;
mod segmented;
mod slider;
mod switch;
mod text;

pub use button::Button;
pub use chips::Chips;
pub use combo::Combo;
pub use number::Number;
pub use readout::Readout;
pub use row::{Note, RowGlyph, RowOutput, SettingRow};
pub use search::Search;
pub use segmented::Segmented;
pub use slider::Slider;
pub use switch::Switch;
pub use text::{Folder, FolderOutput, Text, TextArea};

use egui::{
    Align, Color32, FontId, Layout, Painter, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, pos2,
    vec2,
};

use super::FieldLook;
use crate::Theme;
use crate::fonts::MONO;

/// The body text, every settings control's.
fn body(t: &Theme) -> FontId {
    FontId::proportional(t.text.body)
}

/// The mono face at the body size, for a pattern or a path.
fn mono(t: &Theme) -> FontId {
    FontId::new(t.text.body, egui::FontFamily::Name(MONO.into()))
}

/// A field's look in a settings well: a box high, `text_strong`, `sunken` with no frame but
/// the theme's edge ([`edge`]); in the mono face where it holds a pattern.
fn field_look(t: &Theme, monospaced: bool) -> FieldLook {
    FieldLook {
        height: t.sizes.setting_control,
        font: if monospaced { mono(t) } else { body(t) },
        colour: t.colors.text_strong,
        fill: Some(t.colors.sunken),
        frame: (t.colors.button_edge.a() > 0).then_some(t.colors.button_edge),
        lead: 0.0,
    }
}

/// The edge a settings control is drawn with: none, but a hairline in `button_edge` in a
/// theme whose plates have one, the high-contrast theme, where `sunken`, `raised` and the
/// panel are all black.
fn edge(t: &Theme) -> Stroke {
    if t.colors.button_edge.a() > 0 {
        Stroke::new(t.shape.hairline, t.colors.button_edge)
    } else {
        Stroke::NONE
    }
}

/// A well: `sunken`, with no frame but the theme's [`edge`].
fn paint_well(painter: &Painter, rect: Rect, t: &Theme) {
    painter.rect(
        rect,
        t.shape.radius,
        t.colors.sunken,
        edge(t),
        StrokeKind::Inside,
    );
}

/// A box: `raised`, with no frame but the theme's [`edge`].
fn paint_box(painter: &Painter, rect: Rect, t: &Theme) {
    painter.rect(
        rect,
        t.shape.radius,
        t.colors.raised,
        edge(t),
        StrokeKind::Inside,
    );
}

/// The accent ring round a control while it has the focus from a key, as every control's
/// ([`super::focus_ring`]).
fn ring(painter: &Painter, rect: Rect, t: &Theme) {
    super::focus_ring(painter, rect, t);
}

/// A child of `ui` `width` × `height` at the end of its width, the strip before it taken
/// too: where a switch, a number and a button sit in the column.
fn at_end(ui: &mut Ui, width: f32, height: f32) -> Ui {
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let rect = Rect::from_min_size(
        pos2(strip.right() - width, strip.top()),
        vec2(width, height),
    );
    ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    )
}

/// Whether the primary button is pressed on `response`'s widget: the settings controls
/// take the primary button alone, a press of another drawing and changing nothing.
fn pressed(response: &egui::Response) -> bool {
    response.is_pointer_button_down_on()
        && response
            .ctx
            .input(|i| i.pointer.button_down(egui::PointerButton::Primary))
}

/// `value` rounded to the nearest of `grid`'s multiples, the values a number is kept at as
/// it is shown, where it has one, then to `decimals`: one rounding for what is typed,
/// stepped and kept. A value half way rounds away from 0, as the store rounds it, the
/// float's stray last digits put aside first (0.6 over 0.4 is 1.4999999999999998).
fn on_grid(value: f64, grid: Option<f64>, decimals: usize) -> f64 {
    let value = match grid {
        Some(grid) if grid > 0.0 => {
            let steps = ((value / grid) * 1e9).round() / 1e9;
            steps.round() * grid
        }
        _ => value,
    };
    let scale = 10f64.powi(decimals as i32);
    (value * scale).round() / scale + 0.0
}

/// `colour` as a disabled control draws it.
fn dimmed(t: &Theme, enabled: bool, colour: Color32) -> Color32 {
    if enabled {
        colour
    } else {
        t.colors.text_disabled
    }
}
