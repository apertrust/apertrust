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

//! The dropdown row, always one line (`docs/gui.md`, How the rows are laid out), in one of
//! two styles (decision 42, `docs/gui.md`, Choice rows: two styles). Either way the label
//! is at the start and the row's quad just before the option, with a point of the row over
//! and under them.
//!
//! - **Boxed**, the default, everywhere but module panels: the box at the end, its end edge
//!   on the value fields', a field high and styled as a value field, its text followed by
//!   ⌄. It is as wide as its widest option and never narrower than a value field, so a
//!   choice never changes it.
//! - **Blended**, darktable's combobox, in module panels: no box, the option right-aligned
//!   in the small text and followed by ⌄ in the secondary text, whose stroke ends where
//!   slider values end, and the focus ring round the whole row. Its place is as wide as
//!   its widest option, with no minimum, so a choice never changes it either.
//! - A row given a mark has the kept marker after its label, as a slider row's: ● while
//!   kept, ○ under the pointer or focus, and a click keeps or releases it. Its place is the
//!   row's whether kept or not, so keeping moves nothing.
//! - The whole row opens the list, the label, the gap and the option but not the marker or
//!   the quad, and a neutral band runs across it under the pointer, a Blended row's under
//!   its marker and quad too. The list opens under the option, its end edge on the
//!   option's.
//! - It is a combo box to screen readers, named by the control's full name, the chosen
//!   option its value, and a focus stop before its marker and quad. Space, Enter or ↓
//!   opens the list.
//! - The list is the popup service's dropdown, [`popup::Kind::Dropdown`] (decision 42),
//!   opened and answered by the row itself, which calls [`popup::show`] while it is open:
//!   under the option, its end edge on the option's, the chosen option checked and
//!   highlighted, with darktable's type-to-filter (`popup/filter.rs`). A click on the row
//!   again closes it.
//! - When the label, a 12-point gap, the marker, the quad and the option at its widest
//!   don't fit, the label and the option share what the gap, the marker and the quad leave
//!   in proportion to their widths, as darktable's combo boxes do, and each is cut at its
//!   end with …. The widest option decides, so a choice never changes the layout. The
//!   tooltip shows whatever is cut whole, and screen readers hear the whole label and
//!   option.
//! - An option that cannot be chosen, as one darktable lists that its operation does not
//!   port (decision 29), is listed disabled, with a tooltip that says why.
//! - An option chosen that the photo does not list, from another photo's edit (decision
//!   30), is listed last, chosen and disabled, with a tooltip that says so. While it is
//!   chosen, that tooltip is a line of the row's own and describes it to screen readers.
//! - A separator may set the options after it apart, as Settings' CPU after the graphics
//!   cards; the keys and the filter pass over it.
//! - With no option chosen, the row shows a text of its own the list never offers: a
//!   preset row's Custom while its controls hold none of its options (decision 2). It
//!   counts towards the option's width, so the row keeps its layout whether it shows or
//!   not.

use egui::{
    Color32, FontId, Key, Modifiers, Rect, Role, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::{
    IconButton, Mark, Quad, field_inset, focus_ring, focus_ring_centred, paint_keep_marker,
    truncated,
};
use crate::popup::{self, Answer, Item, Request};
use crate::{Icon, Theme, theme, translator};

/// How a dropdown row looks (decision 42). Not the definitions'
/// `apertrust_modules::ChoiceStyle`, a list or segments: a module panel draws a list in
/// this row's Blended style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChoiceStyle {
    /// The sunken box at the row's end: the Settings dialog, the other panels and utility
    /// sections, the Export dialog, Customise tabs.
    #[default]
    Boxed,
    /// darktable's combobox, in module panels: the option at the row's end with no box,
    /// ending where slider values end.
    Blended,
}

/// What the user did to the row this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceAction {
    /// Another option, by its index.
    Set(usize),
    /// Keep the control, or release it: the marker was pressed.
    ToggleKeep,
    /// The quad was pressed.
    Quad,
}

pub struct Choice<'a> {
    label: &'a str,
    name: &'a str,
    promise: Option<&'a str>,
    options: &'a [&'a str],
    selected: Option<usize>,
    /// What the row shows while no option is chosen.
    none_chosen: Option<&'a str>,
    style: ChoiceStyle,
    /// The kept marker's state, on a row that has one.
    mark: Option<Mark>,
    quad: Option<Quad<'a>>,
    enabled: bool,
    /// Options that cannot be chosen, by index, and their tooltip.
    disabled_options: (&'a [usize], &'a str),
    /// An option the photo does not list, by index, and what it says of it.
    unlisted: Option<(usize, &'a str)>,
    /// The option the list draws a separator before, by index.
    separator: Option<usize>,
    /// What screen readers hear of the row besides its name and value.
    description: Option<&'a str>,
    /// The Boxed box's least width, so boxes in a column share one.
    min_box: f32,
}

impl<'a> Choice<'a> {
    /// `label` is the short, translated label; the full name defaults to it. The options
    /// are translated. `selected` is the chosen option's index, or `None` while none is
    /// ([`Choice::none_chosen`]).
    pub fn new(label: &'a str, options: &'a [&'a str], selected: impl Into<Option<usize>>) -> Self {
        Choice {
            label,
            name: label,
            promise: None,
            options,
            selected: selected.into(),
            none_chosen: None,
            style: ChoiceStyle::Boxed,
            mark: None,
            quad: None,
            enabled: true,
            disabled_options: (&[], ""),
            unlisted: None,
            separator: None,
            description: None,
            min_box: 0.0,
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

    /// The row's look: Boxed unless given, Blended in module panels.
    pub fn style(mut self, style: ChoiceStyle) -> Self {
        self.style = style;
        self
    }

    /// The kept marker after the label, as a slider row's ([`super::SliderRow::mark`]): ●
    /// while kept, ○ under the pointer or focus, a click reporting
    /// [`ChoiceAction::ToggleKeep`]. A row given none has no marker: a preset row, which
    /// stores nothing.
    pub fn mark(mut self, mark: Mark) -> Self {
        self.mark = Some(mark);
        self
    }

    /// An action just before the option, such as White balance Mode's picker.
    pub fn quad(mut self, quad: Quad<'a>) -> Self {
        self.quad = Some(quad);
        self
    }

    /// A disabled row is dimmed and ignores input: a control that can change nothing on
    /// the photo.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Options that cannot be chosen, by index: listed disabled, with `tip`, translated,
    /// as their tooltip.
    pub fn disabled_options(mut self, options: &'a [usize], tip: &'a str) -> Self {
        self.disabled_options = (options, tip);
        self
    }

    /// An option the photo does not list, by index, that an edit from another photo holds
    /// (decision 30): listed disabled with `tip`, translated, as its tooltip, and while it
    /// is chosen, `tip` is a line of the row's tooltip and its description to screen
    /// readers.
    pub fn unlisted(mut self, option: usize, tip: &'a str) -> Self {
        self.unlisted = Some((option, tip));
        self
    }

    /// What the row shows, translated, while no option is chosen: a preset row's Custom.
    /// The list never offers it, and the option's place is as wide as it too, so the row
    /// keeps its layout whether it shows or not.
    /// The Boxed box at least `width` points wide, as wide as the other controls of a
    /// column it shares; it is still as wide as its widest option where that is wider.
    pub fn min_box_width(mut self, width: f32) -> Self {
        self.min_box = width;
        self
    }

    pub fn none_chosen(mut self, text: &'a str) -> Self {
        self.none_chosen = Some(text);
        self
    }

    /// A separator in the list before the option `option`, by index, setting the options
    /// after it apart: Settings' CPU after the graphics cards.
    pub fn separator_before(mut self, option: usize) -> Self {
        self.separator = Some(option);
        self
    }

    /// What screen readers hear of the row besides its name and value, translated: what is
    /// said under a Settings row, heard as the box gets the focus. It takes the place of an
    /// unlisted option's.
    pub fn description(mut self, text: &'a str) -> Self {
        self.description = Some(text);
        self
    }

    /// The list under `anchor`, each option an item whose id is its index: the chosen one
    /// checked, the others the photo cannot use or the module does not port disabled, with
    /// their tooltips, and a separator where the row has one.
    fn list(&self, anchor: Rect) -> Request {
        let (disabled, disabled_tip) = self.disabled_options;
        let items = self
            .options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let item = Item::new(i.to_string(), *option).checked(self.selected == Some(i));
                if let Some((_, tip)) = self.unlisted.filter(|(unlisted, _)| *unlisted == i) {
                    item.enabled(false).tip(tip)
                } else if disabled.contains(&i) {
                    item.enabled(false).tip(disabled_tip)
                } else {
                    item
                }
            })
            .collect();
        let mut request = Request::dropdown(anchor, self.name, items);
        if let Some(at) = self.separator.filter(|at| *at < request.entries.len()) {
            request.entries.insert(at, crate::popup::Entry::Separator);
        }
        request
    }

    /// How wide its box is in the Boxed style: as wide as its widest option, never narrower
    /// than a value field, as [`super::ValueField::width`] tells a field's. A row of the
    /// Settings dialog lays its box out with it, after the gap the row keeps before it.
    pub fn box_width(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        let small = FontId::proportional(t.text.small);
        let widest = self
            .options
            .iter()
            .chain(&self.none_chosen)
            .map(|option| width_of(ui, option, &small))
            .fold(0.0, f32::max);
        (widest.ceil() + box_padding(&t)).max(t.sizes.value_column)
    }

    pub fn show(self, ui: &mut Ui) -> Option<ChoiceAction> {
        let t = theme(ui.ctx());
        let id = ui.make_persistent_id(("choice", self.name));
        let (text, secondary) = if self.enabled {
            (t.colors.text, t.colors.text_secondary)
        } else {
            (t.colors.text_disabled, t.colors.text_disabled)
        };
        let small = FontId::proportional(t.text.small);
        let body = FontId::proportional(t.text.body);
        let widest = self
            .options
            .iter()
            .chain(&self.none_chosen)
            .map(|option| width_of(ui, option, &small))
            .fold(0.0, f32::max);
        // The option's place at its widest option: Boxed, the box, never narrower than a
        // value field; Blended, the text and its chevron alone.
        let blended = self.style == ChoiceStyle::Blended;
        let padding = if blended {
            blended_padding(&t)
        } else {
            box_padding(&t)
        };
        let option = widest.ceil() + padding;
        let option = if blended {
            option
        } else {
            option.max(t.sizes.value_column).max(self.min_box)
        };
        let label_width = width_of(ui, self.label, &body);
        // A Blended row is a module panel's one-line row, `panel_line` high; a Boxed one
        // keeps the option with a point of the row over and under it.
        let height = if blended {
            t.sizes.panel_line
        } else {
            t.sizes.field + t.spacing.xxs
        };
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let places = Places::across(
            row,
            &t,
            label_width,
            option,
            self.mark.is_some(),
            self.quad.is_some(),
        );

        // The row opens the list; the marker and the quad, on top of it, do not.
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui.interact(row, id, sense);
        let current = match self.selected {
            Some(i) => self.options.get(i).copied(),
            None => self.none_chosen,
        }
        .unwrap_or_default();
        let room = places.option.width() - padding;
        let label_cut = label_width > places.label.width();
        let current_cut = width_of(ui, current, &small) > room;
        let label =
            ui.painter()
                .layout_job(truncated(self.label, body, places.label.width(), text));
        // The marker follows the label as drawn, cut or not.
        let marker_at = Places::marker(row, &t, places.label.left() + label.size().x);
        if ui.is_rect_visible(row) {
            let painter = ui.painter();
            // A Blended row's band stays under its marker and quad, as the row's. A Boxed
            // row has none: its box is the control, and hovering it changes nothing.
            if self.enabled && blended && response.contains_pointer() {
                painter.rect_filled(
                    row.expand2(vec2(t.spacing.xs, 0.0)),
                    t.shape.radius,
                    t.colors.hover,
                );
            }
            painter.galley(
                pos2(
                    places.label.left(),
                    places.label.center().y - label.size().y / 2.0,
                ),
                label,
                text,
            );

            let option = places.option;
            let chosen = painter.layout_job(truncated(current, small, room, text));
            let chevron = Icon::ChevronDown.size();
            if blended {
                // The option ends a gap before ⌄, whose stroke ends where slider values do.
                let end = option.right() - padding;
                painter.galley(
                    pos2(
                        end - chosen.size().x,
                        option.center().y - chosen.size().y / 2.0,
                    ),
                    chosen,
                    text,
                );
                Icon::ChevronDown.paint(
                    painter,
                    Rect::from_min_size(
                        pos2(end + t.spacing.xs, option.center().y - chevron / 2.0),
                        vec2(chevron, chevron),
                    ),
                    secondary,
                );
                // Round the whole row, as far out as its band, as a slider row's.
                if response.has_focus() {
                    focus_ring_centred(painter, row.expand2(vec2(t.spacing.xs, 0.0)), &t);
                }
            } else {
                let colours = (text, secondary);
                paint_box(
                    painter,
                    &t,
                    option,
                    chosen,
                    None,
                    colours,
                    response.has_focus(),
                );
            }
        }
        let name = self.name;
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::ComboBox, self.enabled, name);
            info.current_text_value = Some(current.to_owned());
            info
        });
        // The chosen option the photo does not list, said of the row.
        let unlisted = self
            .unlisted
            .filter(|(option, _)| self.selected == Some(*option))
            .map(|(_, tip)| tip);
        if let Some(said) = self.description.or(unlisted) {
            ui.ctx()
                .accesskit_node_builder(id, |node| node.set_description(said));
        }

        let mut action = None;
        // The marker, as a slider row's: a button of its own, after the row in the focus
        // order and over it for the pointer.
        if let Some(mark) = self.mark {
            let tr = translator(ui.ctx());
            let marker = ui.interact(
                marker_at,
                id.with("marker"),
                if self.enabled {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            if self.enabled && marker.clicked() {
                action = Some(ChoiceAction::ToggleKeep);
            }
            let kept = mark == Mark::Kept;
            let mut args = crate::i18n::Args::new();
            args.set("control", self.name);
            let keep_name = tr.text_with("marker-keep", Some(&args));
            marker
                .widget_info(|| WidgetInfo::selected(Role::Button, self.enabled, kept, &keep_name));
            if ui.is_rect_visible(marker_at) {
                let near =
                    ui.rect_contains_pointer(row) || response.has_focus() || marker.has_focus();
                let fade = |c: Color32| {
                    if self.enabled {
                        c
                    } else {
                        c.gamma_multiply(0.6)
                    }
                };
                let painter = ui.painter();
                let hinted = near && self.enabled;
                paint_keep_marker(painter, marker_at.center(), kept, hinted, &t, fade);
                if marker.has_focus() {
                    focus_ring(painter, marker_at, &t);
                }
            }
            let tip = if kept { "slider-kept" } else { "slider-keep" };
            marker.on_hover_text(tr.text(tip));
        }
        if let Some(quad) = self.quad {
            let pressed = IconButton::new(quad.icon, quad.name)
                .size(t.sizes.field)
                .toggled(quad.active)
                .enabled(self.enabled)
                .show(&mut ui.new_child(UiBuilder::new().max_rect(places.quad)))
                .clicked();
            if pressed {
                action = Some(ChoiceAction::Quad);
            }
        }
        // The list is the popup service's dropdown (decision 42): the row, Space, Enter
        // or ↓ opens it, and the row closes it again. ↓ is consumed, so egui's focus doesn't
        // also move down into the list.
        let list = id.with("list");
        let open = popup::is_open(ui.ctx(), list);
        let down = self.enabled
            && response.has_focus()
            && !open
            && ui.input_mut(|i| {
                i.modifiers.is_none() && i.consume_key(Modifiers::NONE, Key::ArrowDown)
            });
        if self.enabled && (response.clicked() || down) {
            if open {
                popup::close(ui.ctx(), list);
            } else {
                let request = self.list(places.option).inside(row).opened_by(id);
                popup::open(ui.ctx(), list, request);
            }
        }
        // Drawn and answered in the frame it opens in, where the app does not draw it
        // before the row.
        if popup::is_open(ui.ctx(), list) {
            popup::show(ui.ctx());
        }
        if let Some(Answer::Chosen(chosen)) = popup::answer(ui.ctx(), list)
            && let Ok(i) = chosen.parse::<usize>()
            && self.selected != Some(i)
        {
            action = Some(ChoiceAction::Set(i));
        }
        // Whatever is cut shows whole in the tooltip, before the promise.
        let tip: Vec<&str> = [
            label_cut.then_some(self.label),
            current_cut.then_some(current),
            unlisted,
            self.promise,
        ]
        .into_iter()
        .flatten()
        .collect();
        if !tip.is_empty() {
            let tip = tip.join("\n");
            response
                .on_hover_text(tip.clone())
                .on_disabled_hover_text(tip);
        }
        action
    }
}

/// A Boxed choice's box alone, as wide as the caller gives it, which the caller hangs a
/// list of its own from with [`pull_down`](super::pull_down): Browse's top line, whose
/// list holds submenus a [`Choice`]'s cannot. It looks as the Boxed row's box does: the
/// sunken field, its text in the small size after an icon where it has one, ⌄ at its end,
/// the focus ring round it, and no band under the pointer; to screen readers a combo box
/// with its name and value.
pub struct ChoiceBox<'a> {
    name: &'a str,
    value: &'a str,
    icon: Option<Icon>,
    enabled: bool,
}

impl<'a> ChoiceBox<'a> {
    /// `name` is what screen readers call it, `value` what it says whole, translated.
    pub fn new(name: &'a str, value: &'a str) -> Self {
        ChoiceBox {
            name,
            value,
            icon: None,
            enabled: true,
        }
    }

    /// An icon before its text, in `text_secondary`.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Whether it can be opened, as it can by default: drawn disabled where it cannot, as
    /// a disabled `Choice`'s box is, reporting no click.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws it `width` wide, its text what `fit` makes of the value for the room left in
    /// the small font, the value whole in its tooltip where `fit` cut it.
    pub fn show(
        self,
        ui: &mut Ui,
        width: f32,
        fit: impl FnOnce(&egui::Painter, &FontId, f32) -> String,
    ) -> egui::Response {
        let t = theme(ui.ctx());
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(width, t.sizes.field), sense);
        let small = FontId::proportional(t.text.small);
        let icon_room = self.icon.map_or(0.0, |_| icon_side(&t) + t.spacing.xs);
        let room = (rect.width() - box_padding(&t) - icon_room).max(0.0);
        let shown = fit(ui.painter(), &small, room);
        let colours = if self.enabled {
            (t.colors.text, t.colors.text_secondary)
        } else {
            (t.colors.text_disabled, t.colors.text_disabled)
        };
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let galley = painter.layout_no_wrap(shown.clone(), small, colours.0);
            paint_box(
                painter,
                &t,
                rect,
                galley,
                self.icon,
                colours,
                response.has_focus(),
            );
        }
        let (name, value, enabled) = (self.name, self.value, self.enabled);
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::ComboBox, enabled, name);
            info.current_text_value = Some(value.to_owned());
            info
        });
        if shown != value {
            response.on_hover_text(value).on_disabled_hover_text(value)
        } else {
            response
        }
    }
}

/// The side of an icon before a box's text: the small text's size and a little.
fn icon_side(t: &Theme) -> f32 {
    t.text.small + t.spacing.xxs
}

/// A Boxed choice's box in `rect`, as `Choice` and [`ChoiceBox`] draw it: the sunken field,
/// `shown` after `icon` where there is one, ⌄ at its end, `CHEVRON_END` in, and the focus
/// ring round it while `focused`; its text in `colours.0`, its icon and ⌄ in `colours.1`,
/// which say whether it is enabled.
fn paint_box(
    painter: &egui::Painter,
    t: &Theme,
    rect: Rect,
    shown: std::sync::Arc<egui::Galley>,
    icon: Option<Icon>,
    colours: (Color32, Color32),
    focused: bool,
) {
    let (text, secondary) = colours;
    painter.rect_filled(rect, t.shape.radius_small, t.colors.sunken);
    let middle = rect.center().y;
    let mut x = rect.left() + field_inset(t);
    if let Some(icon) = icon {
        let side = icon_side(t);
        let at = Rect::from_min_size(pos2(x, middle - side / 2.0), vec2(side, side));
        icon.paint(painter, at, secondary);
        x += side + t.spacing.xs;
    }
    painter.galley(pos2(x, middle - shown.size().y / 2.0), shown, text);
    let chevron = Icon::ChevronDown.size();
    Icon::ChevronDown.paint(
        painter,
        Rect::from_center_size(
            pos2(rect.right() - CHEVRON_END - chevron / 2.0, middle),
            vec2(chevron, chevron),
        ),
        secondary,
    );
    if focused {
        focus_ring(painter, rect, t);
    }
}

/// `text`'s width on one line in `font`.
fn width_of(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER)
        .size()
        .x
}

/// How far ⌄ sits from the box's end: the wireframe's 3 points of padding inside its
/// 1-point border, as the browser draws it.
const CHEVRON_END: f32 = 4.0;

/// The box's room besides its text: the text's start padding, the gap before ⌄, ⌄ and its
/// end padding.
fn box_padding(t: &Theme) -> f32 {
    field_inset(t) + t.spacing.xs + Icon::ChevronDown.size() + CHEVRON_END
}

/// How far ⌄'s stroke ends inside its 10-point box: `chevron-down.svg`'s path ends at 8,
/// with a round cap half its 1.4-point stroke wide.
const CHEVRON_STROKE_INSET: f32 = 1.3;

/// A Blended option's room besides its text: the gap before ⌄, ⌄, and what is left after
/// it, so that its stroke ends a value field's inset from the row's end, where slider
/// values end (`SliderRow`).
fn blended_padding(t: &Theme) -> f32 {
    t.spacing.xs + Icon::ChevronDown.size() + field_inset(t) - CHEVRON_STROKE_INSET
}

/// Where the row's parts go, on one line: the label at the start, with room for the
/// marker after it, and the quad and the option at the end, each a field high.
struct Places {
    label: Rect,
    quad: Rect,
    /// The box, or a Blended row's option and ⌄.
    option: Rect,
}

impl Places {
    /// `label` is the label's width and `option` the option's at its widest. When they
    /// don't both fit beside the gap, the marker and the quad, they share the rest in
    /// proportion to those widths, as darktable's combo boxes do.
    fn across(row: Rect, t: &Theme, label: f32, option: f32, marker: bool, quad: bool) -> Places {
        let s = &t.sizes;
        let marker = if marker {
            t.spacing.xs + s.marker_column
        } else {
            0.0
        };
        let buttons = if quad { s.field + t.spacing.xs } else { 0.0 };
        let room = (row.width() - t.spacing.l - marker - buttons).max(0.0);
        let option = if label + option <= room {
            option
        } else {
            room * option / (label + option)
        };
        let centred = |start: f32, end: f32| {
            Rect::from_x_y_ranges(
                start..=end,
                row.center().y - s.field / 2.0..=row.center().y + s.field / 2.0,
            )
        };
        let option = centred(row.right() - option, row.right());
        let quad_end = option.left() - t.spacing.xs;
        let quad_rect = centred(quad_end - s.field, quad_end);
        let label_end = if quad {
            quad_rect.left()
        } else {
            option.left()
        } - t.spacing.l
            - marker;
        Places {
            label: Rect::from_x_y_ranges(row.left()..=label_end.max(row.left()), row.y_range()),
            quad: quad_rect,
            option,
        }
    }

    /// The marker, a field high, after a label that ends at `label_end`, as a slider row's.
    fn marker(row: Rect, t: &Theme, label_end: f32) -> Rect {
        let s = &t.sizes;
        Rect::from_min_size(
            pos2(label_end + t.spacing.xs, row.center().y - s.field / 2.0),
            vec2(s.marker_column, s.field),
        )
    }
}
