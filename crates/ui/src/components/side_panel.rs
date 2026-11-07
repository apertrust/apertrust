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

//! The side panel's parts (`docs/gui.md`, The shell): the tab bar, with an icon and a name
//! on each tab, » for the tabs that don't fit and ⋮ for the tab options; the body, with
//! the pinned sections over a 4-point band and the rest scrolling below them, with a quiet
//! scroll bar that is always there; and the edge towards the photo, which resizes the
//! panel. Which sections go where is the user's workspace, kept by the app, and so is the
//! panel's width.
//!
//! A tab's name is set as the other labels are, in sentence case, and fits its tile: when
//! one is too wide, every name shrinks together, by at most [`NAME_SHRINK`] points, and a
//! name still too wide ends in an ellipsis. The full name stays in the tooltip and for screen readers.
//! A tab with nothing behind it yet is drawn disabled, as an action that cannot run is: it
//! cannot be chosen, and its tooltip says it is to come (decision 38).

use std::ops::RangeInclusive;

use egui::accesskit::{Action, Orientation};
use egui::text::LayoutJob;
use egui::{
    Context, CornerRadius, CursorIcon, EventFilter, FontFamily, FontId, Id, Key, Layout, Rect,
    Response, Role, ScrollArea, Sense, Stroke, TextFormat, Ui, UiBuilder, WidgetInfo, pos2,
    scroll_area::ScrollBarVisibility, vec2,
};

use super::{IconButton, focus_ring, pull_down, report_shortcut};
use crate::i18n::Args;
use crate::popup::Item;
use crate::{Icon, theme, tr, translator};

/// One of the side panel's tabs.
#[derive(Clone, Debug)]
pub struct PanelTab<'a> {
    pub icon: Icon,
    /// Translated.
    pub name: &'a str,
    /// The keys that open it, for the tooltip (`Ctrl+2`), which also go to screen readers;
    /// the app's actions registry handles them.
    pub shortcut: Option<String>,
    /// Whether it can be chosen. One that cannot, with nothing behind it yet, is drawn
    /// disabled and reported so to screen readers, never chosen by a click, here or in »,
    /// and its tooltip says it is to come, without keys.
    pub enabled: bool,
}

/// What the user did to the tab bar this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabBarAction {
    Select(usize),
    ShowNames(bool),
    Customise,
}

pub struct TabBar<'a> {
    tabs: &'a [PanelTab<'a>],
    selected: usize,
    names: bool,
    customise: bool,
}

const GAP: f32 = 2.0;
const MORE: f32 = 28.0;
const OPTIONS: f32 = 24.0;
/// How far, in points, the tab names may shrink to fit their tiles.
const NAME_SHRINK: f32 = 2.0;

impl<'a> TabBar<'a> {
    pub fn new(tabs: &'a [PanelTab<'a>], selected: usize, names: bool) -> Self {
        TabBar {
            tabs,
            selected,
            names,
            customise: true,
        }
    }

    /// Whether Customise tabs… can run; it is shown disabled when not.
    pub fn customise(mut self, enabled: bool) -> Self {
        self.customise = enabled;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<TabBarAction> {
        let t = theme(ui.ctx());
        let (tile_w, tile_h) = if self.names {
            (58.0, 52.0)
        } else {
            (46.0, 40.0)
        };
        let pad = t.spacing.xs;
        let (bar, list) =
            ui.allocate_exact_size(vec2(ui.available_width(), tile_h + pad), Sense::hover());
        // Looked up first: widget_info runs while egui holds the context.
        let list_name = tr(ui.ctx(), "panel-tabs");
        list.widget_info(|| WidgetInfo::labeled(Role::TabList, true, &list_name));
        // A recessed strip above the sections.
        ui.painter().rect_filled(bar, 0.0, t.colors.sunken);
        ui.painter().hline(
            bar.x_range(),
            bar.bottom() - t.shape.hairline / 2.0,
            egui::Stroke::new(t.shape.hairline, t.colors.separator),
        );

        // Which tabs fit; the chosen one always shows.
        let room = bar.width() - 2.0 * pad - OPTIONS - GAP;
        let fits = |n: usize| n as f32 * (tile_w + GAP) - GAP <= room;
        let mut shown: Vec<usize> = (0..self.tabs.len()).collect();
        let mut hidden = Vec::new();
        if !fits(self.tabs.len()) {
            let n = (((room - MORE - GAP) + GAP) / (tile_w + GAP))
                .floor()
                .max(1.0) as usize;
            shown = (0..n).collect();
            if self.selected >= n {
                shown[n - 1] = self.selected;
            }
            hidden = (0..self.tabs.len())
                .filter(|i| !shown.contains(i))
                .collect();
        }

        let name_room = tile_w - 2.0 * t.spacing.xxs;
        let names = self
            .names
            .then(|| NameStyle::fitting(ui, self.tabs, name_room, &t));
        let mut action = None;
        let mut x = bar.left() + pad;
        for &i in &shown {
            let tab = &self.tabs[i];
            let rect = Rect::from_min_size(pos2(x, bar.top() + pad), vec2(tile_w, tile_h));
            x += tile_w + GAP;
            let sense = if tab.enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let response = ui.interact(rect, list.id.with(i), sense);
            let on = i == self.selected;
            let colour = if !tab.enabled {
                t.colors.text_disabled
            } else if on || response.hovered() {
                t.colors.text
            } else {
                t.colors.text_secondary
            };
            if on {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius {
                        nw: t.shape.radius_large as u8,
                        ne: t.shape.radius_large as u8,
                        sw: 0,
                        se: 0,
                    },
                    t.colors.panel,
                );
            }
            let icon_centre = if self.names {
                pos2(rect.center().x, rect.top() + 6.0 + 11.0)
            } else {
                rect.center()
            };
            let icon = if on { tab.icon.filled() } else { tab.icon };
            icon.paint(
                ui.painter(),
                Rect::from_center_size(icon_centre, vec2(22.0, 22.0)),
                colour,
            );
            if let Some(style) = names {
                let mut job = style.name(tab.name, colour);
                job.wrap.max_width = name_room;
                job.wrap.max_rows = 1;
                job.wrap.break_anywhere = true;
                let galley = ui.painter().layout_job(job);
                ui.painter().galley(
                    pos2(
                        rect.center().x - galley.size().x / 2.0,
                        icon_centre.y + 11.0 + 5.0,
                    ),
                    galley,
                    colour,
                );
            }
            if response.has_focus() {
                focus_ring(ui.painter(), rect.shrink(2.0), &t);
            }
            response.widget_info(|| WidgetInfo::selected(Role::Tab, tab.enabled, on, tab.name));
            // A tab that cannot be chosen has no keys meanwhile.
            let tip = if !tab.enabled {
                later(ui.ctx(), tab.name)
            } else if let Some(keys) = &tab.shortcut {
                report_shortcut(ui, response.id, keys);
                format!("{} ({keys})", tab.name)
            } else {
                tab.name.to_owned()
            };
            if response.on_hover_text(tip).clicked() && tab.enabled && !on {
                action = Some(TabBarAction::Select(i));
            }
        }

        // » with the tabs that don't fit, then ⋮ with the tab options.
        let mut right = bar.right() - pad;
        let options_rect = Rect::from_min_max(
            pos2(right - OPTIONS, bar.top() + pad),
            pos2(right, bar.bottom()),
        );
        right -= OPTIONS + GAP;
        if !hidden.is_empty() {
            let more_rect = Rect::from_min_max(
                pos2(right - MORE, bar.top() + pad),
                pos2(right, bar.bottom()),
            );
            let name = tr(ui.ctx(), "panel-more-tabs");
            let more = button_at(ui, more_rect, Icon::More, &name);
            let chosen = pull_down(ui, &more, &name, || {
                hidden
                    .iter()
                    .map(|&i| {
                        let tab = &self.tabs[i];
                        let item = Item::new(i.to_string(), tab.name).enabled(tab.enabled);
                        match tab.enabled {
                            true => item,
                            false => item.tip(later(ui.ctx(), tab.name)),
                        }
                        .into()
                    })
                    .collect()
            });
            if let Some(i) = chosen.and_then(|chosen| chosen.parse().ok()) {
                action = Some(TabBarAction::Select(i));
            }
        }
        let name = tr(ui.ctx(), "panel-tab-options");
        let options = button_at(ui, options_rect, Icon::OptionsVertical, &name);
        let chosen = pull_down(ui, &options, &name, || {
            vec![
                Item::new("names", tr(ui.ctx(), "panel-show-tab-names"))
                    .checked(self.names)
                    .into(),
                Item::new("customise", tr(ui.ctx(), "panel-customise-tabs"))
                    .enabled(self.customise)
                    .into(),
            ]
        });
        match chosen.as_deref() {
            Some("names") => action = Some(TabBarAction::ShowNames(!self.names)),
            Some("customise") => action = Some(TabBarAction::Customise),
            _ => {}
        }
        action
    }
}

/// The tooltip of a tab with nothing behind it yet, `name`: that it is to come.
fn later(ctx: &Context, name: &str) -> String {
    let mut args = Args::new();
    args.set("tab", name);
    translator(ctx).text_with("panel-tab-later", Some(&args))
}

/// An icon button centred in `rect`; `name` is translated.
fn button_at(ui: &mut Ui, rect: Rect, icon: Icon, name: &str) -> egui::Response {
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
        |ui| IconButton::new(icon, name).show(ui),
    )
    .inner
}

/// How the tab names are set: one size for all of them.
#[derive(Clone, Copy)]
struct NameStyle {
    size: f32,
}

impl NameStyle {
    /// The theme's caption size if every name fits in `room`; otherwise the largest size, in
    /// half points down to [`NAME_SHRINK`] smaller, at which they all fit, or the smallest.
    fn fitting(ui: &Ui, tabs: &[PanelTab<'_>], room: f32, t: &crate::Theme) -> NameStyle {
        let fits = |style: NameStyle| {
            tabs.iter().all(|tab| {
                let job = style.name(tab.name, t.colors.text);
                ui.painter().layout_job(job).size().x <= room
            })
        };
        let mut style = NameStyle {
            size: t.text.caption,
        };
        let smallest = style.size - NAME_SHRINK;
        while style.size > smallest && !fits(style) {
            style.size -= 0.5;
        }
        style
    }

    /// A tab's name under its icon, as the other labels are set: Regular, as written.
    fn name(self, text: &str, colour: egui::Color32) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.append(
            text,
            0.0,
            TextFormat {
                font_id: FontId::new(self.size, FontFamily::Proportional),
                color: colour,
                ..Default::default()
            },
        );
        job
    }
}

/// What the user did to the side panel's edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanelEdgeAction {
    /// The panel's new width, in whole points, within the widths it may take.
    Resize(f32),
    /// Back to the panel's own width: a double-click on the edge, or Delete while it has
    /// focus.
    Reset,
}

/// How far the arrow keys move a focused edge, in points, and with Shift; screen readers
/// step it by the first.
const KEY_STEP: f32 = 10.0;
const KEY_STEP_LARGE: f32 = 50.0;

/// The side panel's edge towards the photo, which resizes it. It is the separator between
/// the panel and the photo, inside the panel, and brightens from `separator` to `border`
/// while the pointer is over it, drags it or it has focus.
///
/// - **The pointer**: dragged, the panel follows it within the widths it may take, the
///   point taken hold of staying under it; double-clicked, the panel goes back to its own
///   width. The column-resize cursor shows over it, and it takes the pointer a few points
///   either side of the line (egui's `resize_grab_radius_side`). A drag leaves the focus
///   where it was, so the arrow keys still do what they did.
/// - **The keyboard**: it takes focus with Tab, with a focus ring. Then ← and → move it
///   10 points, 50 with Shift (← widens a panel at the window's end), Home and End take
///   the panel to its narrowest and widest, and Delete (or Backspace, the Mac's Delete)
///   goes back to its own width, as the slider row does.
/// - **Screen readers** find a splitter, named as the caller says, whose value is the
///   width in points, as dragged this frame, and which they step by 10 points.
///
/// It is made before the panel is laid out, so that [`PanelEdge::width`] gives the width a
/// drag asks for this frame and the panel follows the pointer without a frame's delay,
/// and shown after the panel and what lies beside it, so its grab zone is on top of both.
pub struct PanelEdge<'a> {
    id: Id,
    /// Translated, for screen readers.
    name: &'a str,
    /// Whether the panel is at the start of the window, its edge at its end.
    at_start: bool,
    /// The panel's width, within `widths`.
    width: f32,
    widths: RangeInclusive<f32>,
}

impl<'a> PanelEdge<'a> {
    /// The edge of a panel at the end of the window, `width` points wide, drawn within
    /// `widths`; `name` is what screen readers call it ("Side panel width"), translated.
    pub fn new(name: &'a str, width: f32, widths: RangeInclusive<f32>) -> Self {
        let edge = PanelEdge {
            id: Id::unique(("apertrust-ui panel edge", name)),
            name,
            at_start: false,
            width,
            widths,
        };
        PanelEdge {
            width: edge.within(width),
            ..edge
        }
    }

    /// For a panel at the start of the window instead, its edge at its end.
    pub fn at_start(mut self, at_start: bool) -> Self {
        self.at_start = at_start;
        self
    }

    /// The width to lay the panel out at this frame: its own, or where a drag of the edge
    /// has taken it.
    pub fn width(&self, ctx: &Context) -> f32 {
        ctx.read_response(self.id)
            .and_then(|response| self.dragged(ctx, &response))
            .unwrap_or(self.width)
    }

    /// Draws the edge of `panel`, laid out at [`PanelEdge::width`], and reports what the
    /// user did. Shown after the panel and what lies beside it.
    pub fn show(self, ui: &mut Ui, panel: Rect) -> Option<PanelEdgeAction> {
        let t = theme(ui.ctx());
        let x = if self.at_start {
            panel.right()
        } else {
            panel.left()
        };
        let grab = ui.style().interaction.resize_grab_radius_side;
        let rect = Rect::from_x_y_ranges(x - grab..=x + grab, panel.y_range());
        let response = ui.interact(rect, self.id, Sense::click_and_drag());
        let (least, most) = (*self.widths.start(), *self.widths.end());

        // What the pointer, the keys and screen readers ask for, in that order.
        let mut reset = response.double_clicked();
        let mut width = self.dragged(ui.ctx(), &response).unwrap_or(self.width);
        if response.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    self.id,
                    EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            });
            // A panel at the end widens as its edge moves towards the start.
            let (wider, narrower) = if self.at_start {
                (Key::ArrowRight, Key::ArrowLeft)
            } else {
                (Key::ArrowLeft, Key::ArrowRight)
            };
            ui.input(|i| {
                let step = if i.modifiers.shift {
                    KEY_STEP_LARGE
                } else {
                    KEY_STEP
                };
                if i.key_pressed(wider) {
                    width += step;
                }
                if i.key_pressed(narrower) {
                    width -= step;
                }
                if i.key_pressed(Key::Home) {
                    width = least;
                }
                if i.key_pressed(Key::End) {
                    width = most;
                }
                reset |= i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace);
            });
        }
        let steps = ui.input(|i| {
            i.num_accesskit_action_requests(self.id, Action::Increment) as f32
                - i.num_accesskit_action_requests(self.id, Action::Decrement) as f32
        });
        let width = self.within(width + steps * KEY_STEP);

        let active = response.hovered() || response.dragged();
        if active {
            ui.ctx().set_cursor_icon(CursorIcon::ResizeColumn);
        }
        let colour = if active || response.has_focus() {
            t.colors.border
        } else {
            t.colors.separator
        };
        let inside = if self.at_start { -0.5 } else { 0.5 } * t.shape.separator_width;
        ui.painter().vline(
            x + inside,
            panel.y_range(),
            Stroke::new(t.shape.separator_width, colour),
        );
        if response.has_focus() {
            // Inside the panel's height, so the ring never runs onto the bars.
            focus_ring(ui.painter(), rect.shrink2(vec2(0.0, 2.0)), &t);
        }

        // For screen readers: a splitter between the panel and the photo, its value the
        // width as this frame leaves it.
        let name = self.name;
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::Splitter, true, name);
            info.value = Some(f64::from(width));
            info
        });
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_orientation(Orientation::Vertical);
            node.set_min_numeric_value(f64::from(least));
            node.set_max_numeric_value(f64::from(most));
            node.set_numeric_value_step(f64::from(KEY_STEP));
            if width < most {
                node.add_action(Action::Increment);
            }
            if least < width {
                node.add_action(Action::Decrement);
            }
        });

        let action = if reset {
            Some(PanelEdgeAction::Reset)
        } else {
            (width != self.width).then_some(PanelEdgeAction::Resize(width))
        };
        if action.is_some() {
            ui.ctx().request_repaint();
        }
        action
    }

    /// The width a drag of the edge asks for: the width it started at, moved as far as
    /// the pointer since the press, in whole points within the widths. Where the pointer
    /// took hold of the edge stays under it.
    fn dragged(&self, ctx: &Context, response: &Response) -> Option<f32> {
        if !(response.dragged() || response.drag_stopped()) {
            return None;
        }
        let pointer = response.interact_pointer_pos()?.x;
        // The width less how far along the drag the press was, kept from the drag's start,
        // since the press origin goes with the release. A panel at the end widens towards
        // the start.
        let sign = if self.at_start { 1.0 } else { -1.0 };
        let anchor = if response.drag_started() {
            let origin = ctx.input(|i| i.pointer.press_origin())?.x;
            let anchor = self.width - sign * origin;
            ctx.data_mut(|d| d.insert_temp(self.id, anchor));
            anchor
        } else {
            ctx.data(|d| d.get_temp::<f32>(self.id))?
        };
        Some(self.within((anchor + sign * pointer).round()))
    }

    /// `width` within the widths, which may be one width alone.
    fn within(&self, width: f32) -> f32 {
        width.min(*self.widths.end()).max(*self.widths.start())
    }
}

/// The side panel's body: the tab's pinned sections, a 4-point band, then the rest,
/// scrolling with a quiet scroll bar, always there. Pinned sections take at most 70% of
/// the height and scroll on their own beyond it; a tab with none pinned has no band.
#[derive(Default)]
pub struct PanelBody<'a> {
    pinned: Option<Sections<'a>>,
}

/// Draws sections into the panel.
type Sections<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

impl<'a> PanelBody<'a> {
    pub fn new() -> Self {
        PanelBody::default()
    }

    /// The pinned sections.
    pub fn pinned(mut self, pinned: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.pinned = Some(Box::new(pinned));
        self
    }

    /// Draws the pinned sections, then `scrolling`.
    pub fn show(self, ui: &mut Ui, scrolling: impl FnOnce(&mut Ui)) {
        let t = theme(ui.ctx());
        let height = ui.available_height();
        if let Some(pinned) = self.pinned {
            scroll_area(ui, &t, |ui| {
                ScrollArea::vertical()
                    .id_salt("pinned sections")
                    .max_height(height * 0.7)
                    .auto_shrink([false, true])
                    .show(ui, |ui| ui.scope(|ui| restore(ui, pinned)));
            });
            let (band, _) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
            ui.painter().rect_filled(band, 0.0, t.colors.separator);
        }
        scroll_area(ui, &t, |ui| {
            ScrollArea::vertical()
                .id_salt("scrolling sections")
                .auto_shrink([false, false])
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| ui.scope(|ui| restore(ui, scrolling)));
        });
    }
}

/// The style the sections inside a scroll area draw with, saved before the scroll bar's
/// colours are set.
#[derive(Clone)]
struct Saved(std::sync::Arc<egui::Style>);

/// Draws `add` with the scroll bar's colours: egui draws a solid bar's handle in the
/// interactive widgets' fill, which the theme keeps for wells. The bar is quiet: a dim
/// thumb (`scroll_thumb`) on a `sunken` track, a step brighter under the pointer and
/// another while dragged. The sections inside get the normal style back ([`restore`]).
pub(super) fn scroll_area(ui: &mut Ui, t: &crate::Theme, add: impl FnOnce(&mut Ui)) {
    let saved = Saved(ui.style().clone());
    ui.scope(|ui| {
        ui.data_mut(|d| d.insert_temp(egui::Id::unique("apertrust-ui saved style"), saved));
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.bg_fill = t.colors.scroll_thumb;
        widgets.hovered.bg_fill = t.colors.scroll_thumb_hover;
        widgets.active.bg_fill = t.colors.scroll_thumb_active;
        ui.visuals_mut().extreme_bg_color = t.colors.sunken;
        add(ui);
    });
}

pub(super) fn restore<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    if let Some(Saved(style)) =
        ui.data(|d| d.get_temp::<Saved>(egui::Id::unique("apertrust-ui saved style")))
    {
        ui.set_style(style);
    }
    add(ui)
}

/// Shows `area` with the side panel's quiet scroll bar, `add` drawing its content in the
/// normal style: for any other scrolling text, as About's Credits and Licence.
pub fn quiet_scroll<R>(ui: &mut Ui, area: ScrollArea, add: impl FnOnce(&mut Ui) -> R) -> R {
    let t = theme(ui.ctx());
    let mut out = None;
    scroll_area(ui, &t, |ui| {
        out = Some(
            area.show(ui, |ui| ui.scope(|ui| restore(ui, add)).inner)
                .inner,
        );
    });
    out.expect("the scroll area shows its content")
}
