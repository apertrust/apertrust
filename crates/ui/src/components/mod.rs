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

//! The components every screen is built from (`docs/gui.md`, Components).
//!
//! Each is a builder: given its value, `show` draws it and reports what the user did,
//! and the app turns that into a session command. A component holds no value the engine
//! owns, only transient state such as a drag or a field's draft, in egui's memory. It
//! reads the theme and the language from the context, and reports its role, its full
//! name and its value to AccessKit, so screen readers and the interaction tests find it.
//!
//! Checkboxes, text buttons and icon buttons are the design system's own ([`Checkbox`],
//! [`Button`], [`IconButton`], [`ToolButton`]), drawn from one plate and one focus ring;
//! the few egui buttons left take the plate through the theme. Menus are the popup
//! service's plain lists, never egui's `Popup`.

mod action_row;
mod alert;
mod button;
mod buttons_row;
mod central_bar;
mod checkbox;
mod choice;
mod collapsible_group;
mod curve;
mod equalizer;
mod grid_cell;
mod grid_edge;
mod grid_panel;
mod histogram;
mod icon_button;
mod jobs;
mod marker;
mod photo_frame;
mod plate;
mod section;
mod segmented;
mod side_panel;
mod slider;
mod swatches;
mod tabs;
mod text_field;
mod toggle_row;
mod top_panel;
mod value_field;

pub use action_row::ActionRow;
pub use alert::{AlertBody, AlertResponse, ButtonOrder};
pub use button::Button;
pub use buttons_row::{ButtonsAction, ButtonsRow};
pub use central_bar::{CentralBar, CentralBarAction, Toggle, Zoom};
pub use checkbox::Checkbox;
pub use choice::{Choice, ChoiceAction, ChoiceBox, ChoiceStyle};
pub use collapsible_group::CollapsibleGroup;
pub use curve::{Axis, Curve, CurveAction};
pub use equalizer::{Equalizer, EqualizerAction, MaskHistogram, Settle};
pub use grid_cell::{
    CellAction, CellPicture, CellRating, CellStatus, ColourLabel, ColourLabels, GridCell,
    GridCellOutput,
};
pub use grid_edge::{GridEdge, GridEdgeAction};
pub use grid_panel::{
    GridBarAction, GridPanel, GridPanelAction, GridPanelOutput, GridPosition, Sort, SortKey, Thumb,
    ThumbnailSize, grid_panel_height, grid_panel_lines, grid_panel_thickness, grid_panel_width,
};
pub use histogram::{
    Channel, Clipped, Histogram, HistogramAction, HistogramBins, Scale, Scope, ScopeView,
    VectorscopeBins, WaveformBins,
};
pub use icon_button::{IconButton, ToolButton};
pub use jobs::{
    ActivityAction, FileManager, Job, JobArg, JobFailure, JobId, JobMessage, JobPhase, Jobs,
    kind_icon,
};
pub use marker::{Marker, marker, set_marker};
pub use photo_frame::{
    CAPTION_GAP, CAPTION_HEIGHT, CaptionFit, CaptionMark, CaptionName, Margin, PhotoCaption,
    PhotoFrame, Placed, middle_cut,
};
pub use plate::ButtonState;
pub use section::{Section, SectionAction, SectionOutput};
pub use segmented::Segmented;
pub use side_panel::{
    PanelBody, PanelEdge, PanelEdgeAction, PanelTab, TabBar, TabBarAction, quiet_scroll,
    quiet_scroll_rows,
};
pub use slider::{Mark, Quad, SliderAction, SliderRow, Track};
pub use swatches::{Swatch, SwatchFill, Swatches};
pub use tabs::{Tabs, group_label};
pub use text_field::{PathField, PathOutput, TextField, Variable};
pub use toggle_row::ToggleRow;
pub use top_panel::{Available, SaveState, TopPanel, TopPanelAction, caption_button};
pub use value_field::ValueField;

use egui::text::LayoutJob;
use egui::{
    Align, Color32, FocusDirection, FontId, Key, Modifiers, Painter, Pos2, Rect, Response, Stroke,
    StrokeKind,
};

use crate::Theme;
use crate::popup::{self, Answer, Entry, Request};

/// A pull-down menu hung from `button`, the popup service's (decision 40): the button, or
/// ↓ while it has the focus, opens it, and a second press closes it. `entries` is built
/// only as it opens. Returns the id of the item chosen this frame; the focus goes back to
/// the button as the service gives it. Drawn, and answered, in the frame it opens in.
pub fn pull_down(
    ui: &egui::Ui,
    button: &Response,
    name: &str,
    entries: impl FnOnce() -> Vec<Entry>,
) -> Option<String> {
    let ctx = ui.ctx();
    let id = popup::button_menu(button.id);
    let open = popup::is_open(ctx, id);
    let down = !open
        && button.has_focus()
        && ui
            .input_mut(|i| i.modifiers.is_none() && i.consume_key(Modifiers::NONE, Key::ArrowDown));
    if down {
        // egui's focus would move down too.
        ui.memory_mut(|m| m.move_focus(FocusDirection::None));
    }
    if button.clicked() || down {
        if open {
            popup::close(ctx, id);
        } else {
            let request = Request::pull_down(button.rect, name, entries()).opened_by(button.id);
            popup::open(ctx, id, request);
        }
    }
    if popup::is_open(ctx, id) {
        popup::show(ctx);
    }
    match popup::answer(ctx, id) {
        Some(Answer::Chosen(chosen)) => Some(chosen),
        _ => None,
    }
}

/// The menu of the control `response` belongs to, a pull-down of the popup service's
/// opening at the pointer as the control is right-clicked, or under it as the keyboard's
/// menu key or Shift+F10 is pressed while it has the focus (`docs/gui.md`, Popups). `name`
/// is what screen readers call it, `entries` built only as it opens. Returns the id of the
/// item chosen this frame.
pub fn context_menu(
    ui: &egui::Ui,
    response: &Response,
    name: &str,
    entries: impl FnOnce() -> Vec<Entry>,
) -> Option<String> {
    let ctx = ui.ctx();
    let id = response.id.with("context menu");
    let keys = response.has_focus()
        && ui.input_mut(|i| {
            i.consume_key(Modifiers::NONE, Key::Menu) || i.consume_key(Modifiers::SHIFT, Key::F10)
        });
    if response.secondary_clicked() || keys {
        let at = if keys {
            response.rect.left_bottom()
        } else {
            response
                .interact_pointer_pos()
                .unwrap_or(response.rect.left_bottom())
        };
        let anchor = Rect::from_min_size(at, egui::Vec2::ZERO);
        let request = Request::pull_down(anchor, name, entries()).opened_by(response.id);
        popup::open(ctx, id, request);
    }
    if popup::is_open(ctx, id) {
        popup::show(ctx);
    }
    match popup::answer(ctx, id) {
        Some(Answer::Chosen(chosen)) => Some(chosen),
        _ => None,
    }
}

/// `text` on one line at most `width` wide, cut at its end with … when longer.
fn truncated(text: &str, font: FontId, width: f32, colour: Color32) -> LayoutJob {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, colour);
    job.wrap.max_width = width;
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.halign = Align::LEFT;
    job
}

/// How far text sits from the ends of a value field or a dropdown's box: the wireframe's 4
/// points of padding inside its 1-point border, as the browser draws it.
fn field_inset(t: &Theme) -> f32 {
    t.spacing.xs + t.shape.hairline
}

/// Tells screen readers the keys that run a control, as its tooltip spells them.
fn report_shortcut(ui: &egui::Ui, id: egui::Id, keys: &str) {
    ui.ctx()
        .accesskit_node_builder(id, |node| node.set_keyboard_shortcut(keys));
}

/// Whether focus rings show: after a key or a screen reader's request, not after a click
/// or a drag, as browsers' `:focus-visible` does. A slider being dragged has focus, and a
/// ring around it then says nothing the hand doesn't already know.
#[derive(Debug)]
pub(crate) struct FocusVisible {
    keyboard: bool,
}

impl Default for FocusVisible {
    // Before any input, focus given by the app shows.
    fn default() -> Self {
        Self { keyboard: true }
    }
}

impl egui::plugin::Plugin for FocusVisible {
    fn debug_name(&self) -> &'static str {
        "apertrust focus rings"
    }

    fn input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        for event in &input.events {
            match event {
                egui::Event::Key { pressed: true, .. } | egui::Event::AccessKitActionRequest(_) => {
                    self.keyboard = true;
                }
                egui::Event::PointerButton { pressed: true, .. } | egui::Event::Touch { .. } => {
                    self.keyboard = false;
                }
                _ => {}
            }
        }
    }
}

/// The kept marker beside a row's label, as slider and choice rows draw it: ● while
/// `kept`, ○ while `hinted`, under the pointer or focus on a row that takes input, and
/// nothing otherwise. `fade` dims ● on a disabled row.
fn paint_keep_marker(
    painter: &Painter,
    centre: Pos2,
    kept: bool,
    hinted: bool,
    t: &Theme,
    fade: impl Fn(Color32) -> Color32,
) {
    if kept {
        painter.circle_filled(centre, 4.0, fade(t.colors.kept_marker));
    } else if hinted {
        painter.circle_stroke(centre, 3.6, Stroke::new(1.2, t.colors.text_secondary));
    }
}

/// The colour of an icon in a button: `text_secondary` at rest, a step quieter than the
/// words beside it, and `text` while `lit` (`docs/gui.md`, Quieter icons, lighter headings,
/// 2-point borders). An icon that shows a state that is on, or that is data, is not drawn
/// with this.
fn icon_colour(t: &Theme, enabled: bool, lit: bool) -> Color32 {
    if !enabled {
        t.colors.text_disabled
    } else if lit {
        t.colors.text
    } else {
        t.colors.text_secondary
    }
}

/// Whether the control `response` belongs to lights its icon: under the pointer, pressed,
/// or focused while focus rings show (keyboard focus, not a click's).
fn icon_lit(ctx: &egui::Context, response: &Response) -> bool {
    response.hovered()
        || response.is_pointer_button_down_on()
        || (response.has_focus() && focus_shows(ctx))
}

/// Whether focus rings show now ([`FocusVisible`]).
fn focus_shows(ctx: &egui::Context) -> bool {
    ctx.with_plugin(|focus: &mut FocusVisible| focus.keyboard)
        .unwrap_or(true)
}

/// The ring around a focused control, while focus rings show.
fn focus_ring(painter: &Painter, rect: Rect, t: &Theme) {
    ring(painter, rect, t, StrokeKind::Outside);
}

/// The ring centred on `rect`'s edge, while focus rings show: around a whole row, whose
/// neighbours an outside ring would run into, as a list's line in the app is.
pub fn focus_ring_centred(painter: &Painter, rect: Rect, t: &Theme) {
    ring(painter, rect, t, StrokeKind::Middle);
}

fn ring(painter: &Painter, rect: Rect, t: &Theme, kind: StrokeKind) {
    if !focus_shows(painter.ctx()) {
        return;
    }
    painter.rect_stroke(
        rect,
        t.shape.radius,
        Stroke::new(1.5, t.colors.focus_ring),
        kind,
    );
}

#[cfg(test)]
mod tests {
    use egui::plugin::Plugin;
    use egui::{Event, Key, Modifiers, PointerButton, RawInput, pos2};

    use super::FocusVisible;

    fn after(focus: &mut FocusVisible, event: Event) -> bool {
        let mut input = RawInput {
            events: vec![event],
            ..RawInput::default()
        };
        focus.input_hook(&egui::Context::default(), &mut input);
        focus.keyboard
    }

    #[test]
    fn rings_show_after_a_key_and_hide_after_a_click() {
        let mut focus = FocusVisible::default();
        assert!(focus.keyboard);
        let click = Event::PointerButton {
            pos: pos2(0.0, 0.0),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        };
        assert!(!after(&mut focus, click));
        let tab = Event::Key {
            key: Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        assert!(after(&mut focus, tab));
        // Moving the pointer alone changes nothing.
        assert!(after(&mut focus, Event::PointerMoved(pos2(5.0, 5.0))));
    }
}
