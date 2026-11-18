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

//! The Tools panel's parts (`docs/gui.md`, The shell): the tab bar, with an icon and a name
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
//!
//! ⋮ holds the app's items first, registry actions such as Show Hidden Tools and
//! Workspace ▸, a line, then the tab bar's own, Show tab names and Customise tabs…. It may
//! carry a dot, `accent` and 6 points, at its top end, saying why in its tooltip, to screen
//! readers as its description and once, politely, as the dot appears (decision 96).
//!
//! Find Tool… puts [`FindBar`] in the tab bar's place, at its height, while the app lists
//! the tools found under it, each a section saying where it lives ([`Section::found`]).
//!
//! [`Section::found`]: super::Section::found

use std::ops::RangeInclusive;

use egui::accesskit::{Action, Live, Orientation};
use egui::text::LayoutJob;
use egui::{
    Context, CornerRadius, CursorIcon, EventFilter, FontFamily, FontId, Frame, Id, Key, Layout,
    Margin, Rect, Response, Role, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, TextFormat, Ui,
    UiBuilder, WidgetInfo, pos2, scroll_area::ScrollBarVisibility, vec2,
};

use super::{IconButton, focus_ring, pull_down, report_shortcut};
use crate::i18n::Args;
use crate::popup::{Entry, Item};
use crate::{Icon, theme, tr, translator};

/// One of the Tools panel's tabs.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabBarAction {
    Select(usize),
    ShowNames(bool),
    Customise,
    /// One of the app's items in ⋮ ([`TabBar::items`]) was chosen: its id.
    Chosen(String),
}

pub struct TabBar<'a> {
    tabs: &'a [PanelTab<'a>],
    selected: usize,
    names: bool,
    customise: bool,
    items: Vec<Entry>,
    dot: Option<&'a str>,
    /// The tab to give the focus this frame, where it shows.
    focus: Option<usize>,
}

/// The ids of ⋮'s own items, which no item of the app's takes.
const NAMES: &str = "apertrust-ui tab bar names";
const CUSTOMISE: &str = "apertrust-ui tab bar customise";

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
            items: Vec::new(),
            dot: None,
            focus: None,
        }
    }

    /// Gives the tab at `tab` the focus this frame, as a section moved there takes it
    /// (decision 83): its icon, or », the control drawn, where it is folded under »; ⋮
    /// where it is drawn disabled, as a tab emptied of its tools is.
    pub fn focus(mut self, tab: Option<usize>) -> Self {
        self.focus = tab;
        self
    }

    /// Whether Customise tabs… can run; it is shown disabled when not.
    pub fn customise(mut self, enabled: bool) -> Self {
        self.customise = enabled;
        self
    }

    /// The app's items, first in ⋮, a line after them: registry actions and their
    /// submenus, as the app's menus build them (Show Hidden Tools, Workspace ▸). A choice,
    /// a submenu's item's too, comes back as [`TabBarAction::Chosen`] with its id.
    pub fn items(mut self, items: Vec<Entry>) -> Self {
        self.items = items;
        self
    }

    /// A dot on ⋮ while `says` is some, translated: ⋮'s tooltip and description, heard
    /// once, politely, as it appears ("Hidden tools change this photo: Exposure").
    pub fn dot(mut self, says: Option<&'a str>) -> Self {
        self.dot = says;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<TabBarAction> {
        let t = theme(ui.ctx());
        let (tile_w, tile_h) = tile(self.names);
        let pad = t.spacing.xs;
        let (bar, list) =
            ui.allocate_exact_size(vec2(ui.available_width(), tile_h + pad), Sense::hover());
        // Looked up first: widget_info runs while egui holds the context.
        let list_name = tr(ui.ctx(), "tools-panel-tabs");
        list.widget_info(|| WidgetInfo::labeled(Role::TabList, true, &list_name));
        strip(ui, bar, &t);

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
            if self.focus == Some(i) {
                response.request_focus();
            }
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
            // A tab given the focus but folded under » gives it to », the control drawn.
            if self.focus.is_some_and(|tab| hidden.contains(&tab)) {
                more.request_focus();
            }
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
        let options = ui
            .scope_builder(
                UiBuilder::new()
                    .max_rect(options_rect)
                    .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
                |ui| {
                    let button = IconButton::new(Icon::OptionsVertical, &name);
                    match self.dot {
                        Some(says) => button.dot(true).tip(says).show(ui),
                        None => button.show(ui),
                    }
                },
            )
            .inner;
        // A tab given the focus that cannot take it, drawn disabled, gives it to ⋮.
        let tabs = self.tabs;
        if self
            .focus
            .is_some_and(|tab| !tabs.get(tab).is_some_and(|tab| tab.enabled))
        {
            options.request_focus();
        }
        if let Some(says) = self.dot {
            ui.ctx()
                .accesskit_node_builder(options.id, |node| node.set_description(says));
        }
        // A polite live region that takes no room, saying the dot's words while it shows,
        // so screen readers hear them once, as it appears.
        let said = self.dot.unwrap_or_default();
        let live = ui.interact(
            Rect::from_min_size(options_rect.left_top(), egui::Vec2::ZERO),
            list.id.with("dot live region"),
            Sense::hover(),
        );
        live.widget_info(|| WidgetInfo::labeled(Role::Status, true, said));
        ui.ctx()
            .accesskit_node_builder(live.id, |node| node.set_live(egui::accesskit::Live::Polite));
        let items = self.items;
        let chosen = pull_down(ui, &options, &name, || {
            let mut entries = items;
            if !entries.is_empty() {
                entries.push(Entry::Separator);
            }
            entries.push(
                Item::new(NAMES, tr(ui.ctx(), "panel-show-tab-names"))
                    .checked(self.names)
                    .into(),
            );
            entries.push(
                Item::new(CUSTOMISE, tr(ui.ctx(), "panel-customise-tabs"))
                    .enabled(self.customise)
                    .into(),
            );
            entries
        });
        match chosen {
            Some(id) if id == NAMES => action = Some(TabBarAction::ShowNames(!self.names)),
            Some(id) if id == CUSTOMISE => action = Some(TabBarAction::Customise),
            Some(id) => action = Some(TabBarAction::Chosen(id)),
            None => {}
        }
        action
    }
}

/// A tab's tile, wide and high, with its name or without: 32 high without, so the tab bar
/// is as deep as the viewer bar beside it.
fn tile(names: bool) -> (f32, f32) {
    if names { (58.0, 52.0) } else { (36.0, 32.0) }
}

/// The tab bar's recessed strip above the sections, `bar`, with its hairline at its foot.
fn strip(ui: &Ui, bar: Rect, t: &crate::Theme) {
    ui.painter().rect_filled(bar, 0.0, t.colors.sunken);
    ui.painter().hline(
        bar.x_range(),
        bar.bottom() - t.shape.hairline / 2.0,
        Stroke::new(t.shape.hairline, t.colors.separator),
    );
}

/// What the user did to Find Tool…'s field this frame (decision 96).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindBarAction {
    /// The text typed, whole, after each keystroke.
    Typed(String),
    /// ✕: the search closed.
    Close,
    /// Return in the field: the first tool found opened, found by the text typed this
    /// frame where the frame also changed it, as a paste and Return together do.
    OpenFirst { typed: Option<String> },
}

/// Find Tool…'s field, in the tab bar's place while the search is open (decision 96): the
/// same strip at the tab bar's height, with tab names or without, holding a field with
/// the search icon at its start and ✕ after it. Typing reports the whole text at once; ↓
/// takes the focus to the first tool found, Return opens it, and ✕ closes the search.
/// Escape is the app's, which knows whether a list or a value field below had it first.
///
/// Screen readers find a search box named "Find a tool", and hear the number of tools
/// found after each keystroke, politely ([`FindBar::count`]). The tools found are sections
/// the app draws under it, each given where it lives ([`Section::found`]): ↑ and ↓ move
/// between their headers.
///
/// [`Section::found`]: super::Section::found
pub struct FindBar<'a> {
    text: &'a str,
    names: bool,
    focus: bool,
    count: Option<usize>,
}

impl<'a> FindBar<'a> {
    /// The field holding `text`, as high as the tab bar with tab names, `names`, or
    /// without.
    pub fn new(text: &'a str, names: bool) -> Self {
        FindBar {
            text,
            names,
            focus: false,
            count: None,
        }
    }

    /// Gives the field the focus this frame, as the search opens.
    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    /// How many tools match, said to screen readers ("3 tools"); none while nothing is
    /// typed, when nothing is listed.
    pub fn count(mut self, count: Option<usize>) -> Self {
        self.count = count;
        self
    }

    /// The field's id, by which the app tells it from a value field below.
    pub fn field_id() -> Id {
        Id::unique("apertrust-ui find field")
    }

    pub fn show(self, ui: &mut Ui) -> Option<FindBarAction> {
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let pad = t.spacing.xs;
        let (_, tile_h) = tile(self.names);
        let (bar, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), tile_h + pad), Sense::hover());
        strip(ui, bar, &t);
        let id = FindBar::field_id();
        // The headers drawn last frame become the ones the keys move through; a search
        // opening starts afresh, and a ↓ the last frame's tools found never took is dropped.
        ctx.data_mut(|d| {
            let found = d.get_temp_mut_or_default::<FoundHeaders>(found_headers());
            if self.focus {
                *found = FoundHeaders::default();
            }
            found.drawn = std::mem::take(&mut found.drawing);
            found.first = false;
        });

        let side = t.sizes.row;
        let close_rect = Rect::from_center_size(
            pos2(bar.right() - pad - side / 2.0, bar.center().y),
            vec2(side, side),
        );
        let field = Rect::from_min_max(
            pos2(bar.left() + pad, bar.center().y - side / 2.0),
            pos2(close_rect.left() - GAP, bar.center().y + side / 2.0),
        );
        let painter = ui.painter();
        painter.rect_filled(field, t.shape.radius, t.colors.sunken);
        painter.rect_stroke(
            field,
            t.shape.radius,
            Stroke::new(t.shape.hairline, t.colors.border),
            StrokeKind::Inside,
        );
        let icon = t.sizes.icon;
        let icon_rect = Rect::from_center_size(
            pos2(field.left() + t.spacing.s + icon / 2.0, field.center().y),
            vec2(icon, icon),
        );
        Icon::Search.paint(painter, icon_rect, t.colors.text_secondary);

        let name = tr(&ctx, "find-tool-field");
        if self.focus {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        let mut text = self.text.to_owned();
        let edit_rect = Rect::from_min_max(
            pos2(icon_rect.right() + t.spacing.xs, field.top()),
            pos2(field.right() - t.spacing.xs, field.bottom()),
        );
        // The text in the middle of the field's height, as the icon is.
        let font = FontId::proportional(t.text.body);
        let row = ctx.fonts_mut(|f| f.row_height(&font));
        let above = ((side - row) / 2.0).max(0.0) as i8;
        let edit = TextEdit::singleline(&mut text)
            .id(id)
            .frame(Frame::NONE.inner_margin(Margin {
                top: above,
                ..Margin::ZERO
            }))
            .font(font)
            .text_color(t.colors.text)
            .desired_width(edit_rect.width())
            .hint_text(
                egui::RichText::new(&name)
                    .size(t.text.body)
                    .color(t.colors.text_secondary),
            );
        let mut inside = ui.new_child(UiBuilder::new().max_rect(edit_rect));
        inside.spacing_mut().interact_size.y = side;
        let visuals = inside.visuals_mut();
        visuals.selection.bg_fill = t.colors.field_selection;
        visuals.text_cursor.stroke.color = t.colors.text;
        // Whether the field had the keys before this frame's Return: egui's `lost_focus`
        // also holds a frame after ↓ gave a tool found the focus, whose Return is its own.
        let had_focus = ctx.memory(|m| m.has_focus(id));
        let response = inside.put(edit_rect, edit);
        let typed = (text != self.text).then_some(text);
        if response.has_focus() {
            focus_ring(ui.painter(), field, &t);
            // ↓ is the search's, to the tools found, never egui's move to what lies below.
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
            // ↓ goes to the first of the tools this frame's text finds, once they are drawn
            // under the field ([`found_header`]): where none is, the field keeps the focus.
            let down = ui.input(|i| i.modifiers.is_none() && i.key_pressed(Key::ArrowDown));
            if down {
                ctx.data_mut(|d| {
                    d.get_temp_mut_or_default::<FoundHeaders>(found_headers())
                        .first = true;
                });
            }
        }
        // Return opens the first tool found, the field keeping the focus for more typing.
        let returned = had_focus && response.lost_focus();
        let mut action = if returned && ui.input(|i| i.key_pressed(Key::Enter)) {
            ctx.memory_mut(|m| m.request_focus(id));
            Some(FindBarAction::OpenFirst { typed })
        } else {
            typed.map(FindBarAction::Typed)
        };
        ctx.accesskit_node_builder(id, |node| {
            node.set_role(Role::SearchInput);
            node.set_label(name.as_str());
        });

        let close = tr(&ctx, "find-tool-close");
        if button_at(ui, close_rect, Icon::Close, &close).clicked() {
            action = Some(FindBarAction::Close);
        }

        // A polite live region that takes no room: how many tools match, heard as the count
        // changes.
        let said = self
            .count
            .map(|count| {
                let mut args = Args::new();
                args.set("count", count);
                translator(&ctx).text_with("find-tool-count", Some(&args))
            })
            .unwrap_or_default();
        let live = ui.interact(
            Rect::from_min_size(bar.left_top(), egui::Vec2::ZERO),
            id.with("count"),
            Sense::hover(),
        );
        live.widget_info(|| WidgetInfo::labeled(Role::Status, true, &said));
        ctx.accesskit_node_builder(live.id, |node| node.set_live(Live::Polite));
        action
    }
}

/// The headers of the tools Find Tool… found, top first: those drawn this frame, as they
/// are, and the last frame's, which the keys move through; the header the keys moved the
/// focus to, scrolled into view where it is next drawn, inside the scroll area of the
/// tools found, at its rectangle then; and whether ↓ in the field asked for the first
/// header drawn this frame, the first of what the text found once the field had it.
#[derive(Clone, Debug, Default)]
struct FoundHeaders {
    drawing: Vec<Id>,
    drawn: Vec<Id>,
    scroll_to: Option<Id>,
    first: bool,
}

/// Asks for the header `id` among the tools found to be scrolled into view where it is
/// next drawn ([`found_header`]).
fn scroll_to_header(ctx: &Context, id: Id) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<FoundHeaders>(found_headers())
            .scroll_to = Some(id);
    });
}

fn found_headers() -> Id {
    Id::unique("apertrust-ui found headers")
}

/// A section's header among the tools Find Tool… found, `toggle` its chevron and title:
/// the first drawn takes the focus where ↓ in the field asked for it this frame, and,
/// while one has the focus, ↑ and ↓ take it to the header before or after it, over the
/// rows of an open one, each scrolled into view.
pub(super) fn found_header(ui: &Ui, toggle: &Response) {
    let ctx = ui.ctx();
    let (drawn, scroll, first) = ctx.data_mut(|d| {
        let found = d.get_temp_mut_or_default::<FoundHeaders>(found_headers());
        found.drawing.push(toggle.id);
        let first = found.first && found.drawing.len() == 1;
        if first {
            found.first = false;
        }
        let scroll = found.scroll_to == Some(toggle.id);
        if scroll {
            found.scroll_to = None;
        }
        (found.drawn.clone(), scroll, first)
    });
    if first {
        ctx.memory_mut(|m| m.request_focus(toggle.id));
    }
    // The keys moved the focus here: into view at where it is drawn now, in the scroll
    // area of the tools found.
    if scroll || first {
        ui.scroll_to_rect(toggle.rect, None);
    }
    if !toggle.has_focus() {
        return;
    }
    ctx.memory_mut(|m| {
        m.set_focus_lock_filter(
            toggle.id,
            EventFilter {
                vertical_arrows: true,
                ..Default::default()
            },
        )
    });
    // A header the keys moved the focus to this frame has had this frame's key.
    let Some(here) = drawn.iter().position(|id| *id == toggle.id) else {
        return;
    };
    if toggle.gained_focus() {
        return;
    }
    let (up, down) = ui.input(|i| {
        let plain = i.modifiers.is_none();
        (
            plain && i.key_pressed(Key::ArrowUp),
            plain && i.key_pressed(Key::ArrowDown),
        )
    });
    let to = if down {
        drawn.get(here + 1)
    } else if up {
        here.checked_sub(1).and_then(|i| drawn.get(i))
    } else {
        None
    };
    if let Some(&to) = to {
        ctx.memory_mut(|m| m.request_focus(to));
        scroll_to_header(ctx, to);
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

/// What the user did to the Tools panel's edge.
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

/// The Tools panel's edge towards the photo, which resizes it. It is the separator between
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
    /// `widths`; `name` is what screen readers call it ("Tools panel width"), translated.
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

/// The Tools panel's body: the tab's pinned sections, a 4-point band, then the rest,
/// scrolling with a quiet scroll bar, always there. Pinned sections take at most 70% of
/// the height and scroll on their own beyond it; a tab with none pinned has no band.
///
/// A tab whose one section's list runs past the panel, as History's, keeps that section's
/// header at the top of its scroll area ([`PanelBody::header`]): the list scrolls under
/// it, a hairline and a soft shadow under the header while it does.
#[derive(Default)]
pub struct PanelBody<'a> {
    pinned: Option<Sections<'a>>,
    header: Option<Sections<'a>>,
    /// The scrolling part's own id, where it is not a tab's ([`PanelBody::scroll_id`]).
    scroll_id: Option<&'a str>,
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

    /// The header of the section the scrolling part holds, drawn over it, which stays at
    /// the top while the section's body scrolls: a section's [`Section::show_header`], its
    /// body drawn in `scrolling` with [`section_body`](super::section_body).
    ///
    /// [`Section::show_header`]: super::Section::show_header
    pub fn header(mut self, header: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.header = Some(Box::new(header));
        self
    }

    /// Gives the scrolling part an id of its own, `name`, so it keeps its own place: Find
    /// Tool…'s tools found scroll apart from the tab they take the place of, which keeps
    /// its offset for when the search closes.
    pub fn scroll_id(mut self, name: &'a str) -> Self {
        self.scroll_id = Some(name);
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
        // As wide as the sections under it, clear of the scroll bar's column.
        let header = self.header.map(|header| {
            let width = ui.available_width() - ui.spacing().scroll.allocated_width();
            let area = Rect::from_min_size(ui.cursor().min, vec2(width, 0.0));
            let drawn = ui.scope_builder(UiBuilder::new().max_rect(area), header);
            drawn.response.rect
        });
        let mut scrolled = false;
        scroll_area(ui, &t, |ui| {
            let out = ScrollArea::vertical()
                .id_salt(self.scroll_id.unwrap_or("scrolling sections"))
                .auto_shrink([false, false])
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| ui.scope(|ui| restore(ui, scrolling)));
            scrolled = out.state.clamped_offset().y > 0.0;
        });
        // Over what scrolls under it, painted after it.
        if let Some(header) = header
            && scrolled
        {
            stuck(ui.painter(), header, &t);
        }
    }
}

/// Under a header the list scrolls under: the separator's hairline at its foot, then a
/// shadow a few points deep, fading, as the board draws it.
fn stuck(painter: &egui::Painter, header: Rect, t: &crate::Theme) {
    let foot = header.bottom();
    let hairline = Rect::from_min_max(
        pos2(header.left(), foot),
        pos2(header.right(), foot + t.shape.hairline),
    );
    painter.rect_filled(hairline, 0.0, t.colors.separator);
    for (row, alpha) in [0.16_f32, 0.09, 0.04].into_iter().enumerate() {
        let top = hairline.bottom() + row as f32;
        let band = Rect::from_min_max(pos2(header.left(), top), pos2(header.right(), top + 1.0));
        painter.rect_filled(
            band,
            0.0,
            egui::Color32::from_black_alpha((alpha * 255.0) as u8),
        );
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

/// Shows `area` with the Tools panel's quiet scroll bar, `add` drawing its content in the
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

/// [`quiet_scroll`] for `rows` rows of one height, `row_height`: `add` draws only those in
/// view, given their range, as egui's `ScrollArea::show_rows` does, as the log window's
/// lines are drawn. Returns the area's output, where it is scrolled to among it.
pub fn quiet_scroll_rows<R>(
    ui: &mut Ui,
    area: ScrollArea,
    row_height: f32,
    rows: usize,
    add: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    let t = theme(ui.ctx());
    let mut out = None;
    scroll_area(ui, &t, |ui| {
        out = Some(area.show_rows(ui, row_height, rows, |ui, range| {
            ui.scope(|ui| restore(ui, |ui| add(ui, range))).inner
        }));
    });
    out.expect("the scroll area shows its content")
}
