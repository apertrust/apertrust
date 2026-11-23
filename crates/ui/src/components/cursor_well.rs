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

//! The cursors' well (decision 104; `docs/cursors.md`, The well): one `sunken` track on the
//! Toolbar, one button per group of cursors, where the cursor that decides what a press on
//! the photo does is chosen.
//!
//! - **A group's button** is two targets: its face, 22 by 26 points with its 16-point icon
//!   at its end, shows the group's active cursor and chooses it; its chevron, 11 by 26
//!   right against it, opens the group's list. A group of one with no settings is one face
//!   24 points wide, its icon centred, with no chevron; a group of one with settings has
//!   the chevron for its settings alone.
//! - **No plates** in any state (decision 81): an icon in `text_secondary` at rest,
//!   `text_strong` under the pointer, `accent_text` while held down; the chosen cursor's
//!   icon in `accent_text`, a quarter heavier ([`STROKE_HEAVY`]), and nothing else; a
//!   chevron in `text_strong` while its list is open; disabled `text_disabled`, its reason
//!   the tooltip. A chosen cursor with amounts shows a 4-point triangle at its face's bottom
//!   end corner. High contrast edges the track and outlines a target under the pointer in
//!   `button_edge`, as an icon button.
//! - **The list** is a pull-down of the popup service (decision 40), hung under the
//!   chevron: the group's cursors with their keys, a check on the active one, one not ready
//!   disabled with its reason; then, under a separator, the group's settings, a toggle an
//!   item with a check, a choice a heading over its options, the one in use checked.
//! - **Keys** (`docs/cursors.md`, Keyboard and screen readers): the well is one Tab stop,
//!   landing on the chosen cursor's face, or the first face; ← and → move between the faces
//!   and chevrons; Space or Enter presses one; ↓ on a face opens its group's list, on the
//!   chosen face with amounts its amounts, as Enter does there.
//! - **To AccessKit,** a radio group named "Cursors"; each face a radio button named by its
//!   cursor with its key ("White balance picker, K"), checked while its cursor is chosen,
//!   its tooltip its description, or a disabled one's reason; each chevron a button named
//!   for its group's list ("Pick cursors and settings") that opens a menu and says whether
//!   it is open.
//!
//! It knows no cursor, tool or setting: the app hands it each group's words, icons and
//! state, and turns what it reports into the chosen cursor, each group's active one and the
//! group settings, which it keeps.

use egui::accesskit::{Live, Role};
use egui::{
    Color32, EventFilter, FocusDirection, Id, Key, Modifiers, Rect, Response, Sense, Shape, Stroke,
    StrokeKind, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::plate::{ButtonState, Touch, paint_focus, undimmed_painter};
use super::pull_down;
use crate::i18n::Args;
use crate::icons::{STROKE, STROKE_HEAVY};
use crate::popup::{self, Entry, Item, Request};
use crate::{Icon, Theme, theme, translator};

/// The side of the triangle marking a chosen cursor with amounts.
const TRIANGLE: f32 = 4.0;

/// One cursor of a group, as its face and its group's list show it.
#[derive(Clone, Copy, Debug)]
pub struct WellCursor<'a> {
    icon: Icon,
    name: &'a str,
    key: Option<&'a str>,
    how: Option<&'a str>,
    disabled: Option<&'a str>,
    amounts: bool,
}

impl<'a> WellCursor<'a> {
    /// `name` is translated: "White balance picker".
    pub fn new(icon: Icon, name: &'a str) -> Self {
        WellCursor {
            icon,
            name,
            key: None,
            how: None,
            disabled: None,
            amounts: false,
        }
    }

    /// Its key, as the app spells shortcuts ("K"), in its name, its tooltip and its line.
    pub fn key(mut self, key: &'a str) -> Self {
        self.key = Some(key);
        self
    }

    /// What a press with it does, translated, after its name and key in its tooltip.
    pub fn how(mut self, how: &'a str) -> Self {
        self.how = Some(how);
        self
    }

    /// Not ready, with `why`, translated: its line disabled with the reason as its tip, and
    /// while it is its group's active one, its face too.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    /// Whether it has amounts, continuous values a second press on its chosen face opens.
    pub fn amounts(mut self, amounts: bool) -> Self {
        self.amounts = amounts;
        self
    }
}

/// One of a group's settings, which its cursors share, under them in its list.
#[derive(Clone, Copy, Debug)]
pub enum WellSetting<'a> {
    /// An item with a check, checked while `on`.
    Toggle { name: &'a str, on: bool },
    /// A heading, `name`, over its options, the `chosen` one checked.
    Choice {
        name: &'a str,
        options: &'a [&'a str],
        chosen: usize,
    },
}

/// One button of the well: a group of cursors, with its active one and its settings.
#[derive(Clone, Copy, Debug)]
pub struct WellGroup<'a> {
    name: &'a str,
    cursors: &'a [WellCursor<'a>],
    active: usize,
    settings: &'a [WellSetting<'a>],
}

impl<'a> WellGroup<'a> {
    /// `name` is translated: "Pick", what its chevron's name says. `active` is the cursor
    /// its face shows, an index into `cursors`.
    pub fn new(name: &'a str, cursors: &'a [WellCursor<'a>], active: usize) -> Self {
        WellGroup {
            name,
            cursors,
            active,
            settings: &[],
        }
    }

    pub fn settings(mut self, settings: &'a [WellSetting<'a>]) -> Self {
        self.settings = settings;
        self
    }

    /// Whether it has a chevron: more than one cursor, or settings.
    fn listed(&self) -> bool {
        self.cursors.len() > 1 || !self.settings.is_empty()
    }

    /// Its face's width and its chevron's, none where it has none.
    fn widths(&self, t: &Theme) -> (f32, f32) {
        if self.listed() {
            (t.sizes.cursor_face, t.sizes.cursor_chevron)
        } else {
            (t.sizes.cursor_face_alone, 0.0)
        }
    }

    /// Its list's lines: the cursors, but for a group of one, then its settings under a
    /// separator.
    fn entries(&self) -> Vec<Entry> {
        let mut entries = Vec::new();
        if self.cursors.len() > 1 {
            for (i, cursor) in self.cursors.iter().enumerate() {
                let mut item = Item::new(format!("cursor:{i}"), cursor.name)
                    .checked(i == self.active)
                    .enabled(cursor.disabled.is_none());
                if let Some(key) = cursor.key {
                    item = item.shortcut(key);
                }
                if let Some(why) = cursor.disabled {
                    item = item.tip(why);
                }
                entries.push(item.into());
            }
            if !self.settings.is_empty() {
                entries.push(Entry::Separator);
            }
        }
        for (s, setting) in self.settings.iter().enumerate() {
            match *setting {
                WellSetting::Toggle { name, on } => {
                    entries.push(Item::new(format!("toggle:{s}"), name).checked(on).into());
                }
                WellSetting::Choice {
                    name,
                    options,
                    chosen,
                } => {
                    entries.push(Entry::Heading(name.to_owned()));
                    for (o, option) in options.iter().enumerate() {
                        let item = Item::new(format!("choice:{s}:{o}"), *option);
                        entries.push(item.checked(o == chosen).into());
                    }
                }
            }
        }
        entries
    }

    /// What choosing the item `id` of its list asks for.
    fn answer(&self, group: usize, id: &str) -> Option<CursorWellAction> {
        let mut parts = id.split(':');
        let kind = parts.next()?;
        let mut number = || parts.next()?.parse::<usize>().ok();
        match kind {
            // An index past the group's lines, the groups reshaped since the list opened,
            // or a cursor that stopped being ready while it was open, asks for nothing.
            "cursor" => {
                let cursor = number()
                    .filter(|&i| self.cursors.get(i).is_some_and(|c| c.disabled.is_none()))?;
                Some(CursorWellAction::Cursor { group, cursor })
            }
            "toggle" => {
                let setting = number()?;
                match self.settings.get(setting)? {
                    WellSetting::Toggle { on, .. } => Some(CursorWellAction::Toggle {
                        group,
                        setting,
                        on: !on,
                    }),
                    WellSetting::Choice { .. } => None,
                }
            }
            "choice" => {
                let setting = number()?;
                let option = number()?;
                match self.settings.get(setting)? {
                    WellSetting::Choice { options, .. } if option < options.len() => {
                        Some(CursorWellAction::Choice {
                            group,
                            setting,
                            option,
                        })
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// What the user did to the well this frame. The app keeps the chosen cursor, each group's
/// active one and the settings, and draws the well again from them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorWellAction {
    /// Choose `cursor` of `group`, which becomes the group's active one: its face pressed
    /// while another cursor is chosen, its active one, or a line of its list.
    Cursor { group: usize, cursor: usize },
    /// Open the amounts of `group`'s chosen cursor: its face pressed again while chosen, or
    /// given Enter or ↓, where the cursor has amounts.
    Amounts { group: usize },
    /// `group`'s chosen cursor's face pressed again where it has no amounts: the app turns a
    /// picker cursor's picker off, as a second press on its picker button does, and does
    /// nothing for another.
    Again { group: usize },
    /// The toggle `setting` of `group`'s list, now `on`.
    Toggle {
        group: usize,
        setting: usize,
        on: bool,
    },
    /// The option `option` of the choice `setting` of `group`'s list.
    Choice {
        group: usize,
        setting: usize,
        option: usize,
    },
}

/// The cursors' well: its groups, in the well's order, and which one's active cursor is
/// the chosen one.
pub struct CursorWell<'a> {
    groups: &'a [WellGroup<'a>],
    chosen: Option<usize>,
    disabled: Option<&'a str>,
    pinned: Option<(usize, ButtonState)>,
}

impl<'a> CursorWell<'a> {
    pub fn new(groups: &'a [WellGroup<'a>]) -> Self {
        CursorWell {
            groups,
            chosen: None,
            disabled: None,
            pinned: None,
        }
    }

    /// The group whose active cursor is the chosen one, its face's icon in the accent.
    pub fn chosen(mut self, group: usize) -> Self {
        self.chosen = Some(group);
        self
    }

    /// Every face and chevron dimmed, ignoring input and out of the focus order, with
    /// `why`, translated, as their tooltip: no photo shown.
    pub fn disabled(mut self, why: &'a str) -> Self {
        self.disabled = Some(why);
        self
    }

    /// Draws `group`'s face in `state` whatever the pointer and the keys do, as the gallery
    /// shows each state.
    pub fn state(mut self, group: usize, state: ButtonState) -> Self {
        self.pinned = Some((group, state));
        self
    }

    /// The width it takes, in points: its groups' buttons `spacing.s` apart, in a track
    /// with `spacing.xs` at each end. Phase 2's Select, Pan and Pick take 101.
    pub fn width(&self, ui: &Ui) -> f32 {
        let t = theme(ui.ctx());
        let buttons: f32 = (self.groups.iter())
            .map(|g| {
                let (face, chevron) = g.widths(&t);
                face + chevron
            })
            .sum();
        let gaps = self.groups.len().saturating_sub(1) as f32 * t.spacing.s;
        buttons + gaps + 2.0 * t.spacing.xs
    }

    pub fn show(self, ui: &mut Ui) -> Option<CursorWellAction> {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let words = translator(&ctx);
        let size = vec2(self.width(ui), t.sizes.cursor_well);
        let (track, well) = ui.allocate_exact_size(size, Sense::hover());
        // A face's and a chevron's height: the track's but for its padding.
        let height = t.sizes.cursor_well - 2.0 * t.spacing.xxs;
        let live = self.disabled.is_none() && ui.is_enabled();
        let name = words.text("cursor-well");
        well.widget_info(|| WidgetInfo::labeled(Role::RadioGroup, live, &name));
        let painter = undimmed_painter(ui);
        painter.rect_filled(track, t.shape.radius_large, t.colors.sunken);
        if t.colors.button_edge.a() > 0 {
            painter.rect_stroke(
                track,
                t.shape.radius_large,
                Stroke::new(t.shape.hairline, t.colors.button_edge),
                StrokeKind::Inside,
            );
        }

        // Every target's place, id and whether it can be pressed, in the well's order.
        let mut targets = Vec::new();
        let mut x = track.left() + t.spacing.xs;
        for (g, group) in self.groups.iter().enumerate() {
            let (face, chevron) = group.widths(&t);
            let top = track.top() + t.spacing.xxs;
            let ready = group
                .cursors
                .get(group.active)
                .is_some_and(|c| c.disabled.is_none());
            targets.push(Target {
                group: g,
                chevron: false,
                rect: Rect::from_min_size(pos2(x, top), vec2(face, height)),
                id: well.id.with(("face", g)),
                enabled: live && ready,
            });
            x += face;
            if group.listed() {
                targets.push(Target {
                    group: g,
                    chevron: true,
                    rect: Rect::from_min_size(pos2(x, top), vec2(chevron, height)),
                    id: well.id.with(("chevron", g)),
                    enabled: live,
                });
                x += chevron;
            }
            x += t.spacing.s;
        }
        let stop = self.tab_stop(&ctx, &targets);

        let inner = ui.new_child(
            UiBuilder::new()
                .max_rect(track)
                .accessibility_parent(well.id),
        );
        let mut action = None;
        let mut responses = Vec::with_capacity(targets.len());
        for target in &targets {
            let sense = if !target.enabled {
                Sense::hover()
            } else if Some(target.id) == stop {
                Sense::click()
            } else {
                // Pressed by the pointer, the keys of the one focused and screen readers,
                // but no Tab stop: ← and → move between them.
                Sense::CLICK
            };
            let response = inner.interact(target.rect, target.id, sense);
            let group = &self.groups[target.group];
            let done = if target.chevron {
                self.chevron(&inner, &painter, target, group, &response)
            } else {
                self.face(&inner, &painter, target, group, &targets, &response)
            };
            action = action.or(done);
            responses.push(response);
        }
        self.arrows(ui, &targets, &responses);
        // The chosen cursor's name alone, which screen readers hear politely as it changes,
        // by a key or from a list (`docs/cursors.md`, Keyboard and screen readers).
        let said = (self.chosen)
            .and_then(|g| self.groups.get(g))
            .and_then(|g| g.cursors.get(g.active))
            .map_or("", |c| c.name);
        let status = inner.interact(
            Rect::from_min_size(track.left_top(), egui::Vec2::ZERO),
            well.id.with("said"),
            Sense::hover(),
        );
        status.widget_info(|| WidgetInfo::labeled(Role::Status, true, said));
        ctx.accesskit_node_builder(status.id, |node| node.set_live(Live::Polite));
        action
    }

    /// The target the Tab key lands on: the one focused, while one is; else the chosen
    /// cursor's face, or the first face that can be pressed.
    fn tab_stop(&self, ctx: &egui::Context, targets: &[Target]) -> Option<Id> {
        let focused = ctx.memory(|m| m.focused());
        let usable = |t: &&Target| t.enabled;
        targets
            .iter()
            .filter(usable)
            .find(|t| Some(t.id) == focused)
            .or_else(|| {
                (targets.iter().filter(usable)).find(|t| !t.chevron && Some(t.group) == self.chosen)
            })
            .or_else(|| targets.iter().filter(usable).find(|t| !t.chevron))
            .or_else(|| targets.iter().find(usable))
            .map(|t| t.id)
    }

    /// ← and → on a focused target move the focus to the next one that can be pressed, in
    /// the well's order, stopping at its ends.
    fn arrows(&self, ui: &Ui, targets: &[Target], responses: &[Response]) {
        let Some(at) = responses.iter().position(Response::has_focus) else {
            return;
        };
        let id = responses[at].id;
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
        let (back, on) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowLeft),
                i.consume_key(Modifiers::NONE, Key::ArrowRight),
            )
        });
        if !back && !on {
            return;
        }
        ui.memory_mut(|m| m.move_focus(FocusDirection::None));
        let next = if on {
            (at + 1..targets.len()).find(|&i| targets[i].enabled)
        } else {
            (0..at).rev().find(|&i| targets[i].enabled)
        };
        if let Some(next) = next {
            ui.memory_mut(|m| m.request_focus(targets[next].id));
        }
    }

    /// Draws `group`'s face and reports a press.
    fn face(
        &self,
        ui: &Ui,
        painter: &egui::Painter,
        target: &Target,
        group: &WellGroup<'_>,
        targets: &[Target],
        response: &Response,
    ) -> Option<CursorWellAction> {
        let ctx = ui.ctx();
        let t = &theme(ctx);
        let cursor = group.cursors.get(group.active)?;
        let chosen = self.chosen == Some(target.group);
        // A chosen cursor that cannot act stays chosen in looks; only a disabled well
        // dims it.
        let live = self.disabled.is_none() && ui.is_enabled();
        let pinned = self.pinned.filter(|(g, _)| *g == target.group);
        let touch = Touch::of(ctx, response, target.enabled, pinned.map(|(_, s)| s));
        let rect = target.rect;
        outline(painter, rect, touch, t);
        let side = t.sizes.icon_large;
        let icon = if group.listed() {
            // At its end, next to its chevron.
            Rect::from_min_size(
                pos2(rect.right() - side, rect.center().y - side / 2.0),
                vec2(side, side),
            )
        } else {
            Rect::from_center_size(rect.center(), vec2(side, side))
        };
        let stroke = if chosen { STROKE_HEAVY } else { STROKE };
        cursor
            .icon
            .paint_stroked(painter, icon, face_colour(t, touch, chosen && live), stroke);
        if chosen && cursor.amounts {
            let corner = rect.right_bottom() - vec2(t.spacing.xxs, t.spacing.xxs);
            painter.add(Shape::convex_polygon(
                vec![
                    corner - vec2(0.0, TRIANGLE),
                    corner,
                    corner - vec2(TRIANGLE, 0.0),
                ],
                t.colors.text_secondary,
                Stroke::NONE,
            ));
        }
        paint_focus(painter, rect, t.shape.radius, touch, t);

        let words = translator(ctx);
        let mut args = Args::new();
        args.set("name", cursor.name);
        let name = match cursor.key {
            Some(key) => {
                args.set("key", key);
                words.text_with("cursor-name-key", Some(&args))
            }
            None => cursor.name.to_owned(),
        };
        let tip = match (self.disabled.or(cursor.disabled), cursor.key, cursor.how) {
            (Some(why), _, _) => why.to_owned(),
            (None, Some(_), Some(how)) => {
                args.set("how", how);
                words.text_with("cursor-tip", Some(&args))
            }
            (None, None, Some(how)) => {
                args.set("how", how);
                words.text_with("cursor-tip-how", Some(&args))
            }
            (None, Some(_), None) => words.text_with("cursor-tip-key", Some(&args)),
            (None, None, None) => cursor.name.to_owned(),
        };
        let enabled = target.enabled;
        response.widget_info(|| WidgetInfo::selected(Role::RadioButton, enabled, chosen, &name));
        ctx.accesskit_node_builder(response.id, |node| node.set_description(tip.as_str()));
        if let Some(key) = cursor.key {
            ctx.accesskit_node_builder(response.id, |node| node.set_keyboard_shortcut(key));
        }
        response
            .clone()
            .on_hover_text(&tip)
            .on_disabled_hover_text(&tip);
        if !enabled {
            return None;
        }

        let amounts = chosen && cursor.amounts;
        let down = response.has_focus()
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
        if down {
            // egui's focus would move down too.
            ui.memory_mut(|m| m.move_focus(FocusDirection::None));
            if amounts {
                return Some(CursorWellAction::Amounts {
                    group: target.group,
                });
            }
            // Its group's list, as its chevron opens it, giving the focus back to the face.
            let chevron = targets
                .iter()
                .find(|c| c.chevron && c.group == target.group && c.enabled);
            if let Some(chevron) = chevron {
                let request =
                    Request::pull_down(chevron.rect, list_name(ctx, group), group.entries())
                        .opened_by(response.id);
                popup::open(ctx, popup::button_menu(chevron.id), request);
            }
            return None;
        }
        if !response.clicked() {
            return None;
        }
        if !chosen {
            Some(CursorWellAction::Cursor {
                group: target.group,
                cursor: group.active,
            })
        } else if amounts {
            Some(CursorWellAction::Amounts {
                group: target.group,
            })
        } else {
            Some(CursorWellAction::Again {
                group: target.group,
            })
        }
    }

    /// Draws `group`'s chevron, opens its list when pressed, and reports what the list
    /// chose.
    fn chevron(
        &self,
        ui: &Ui,
        painter: &egui::Painter,
        target: &Target,
        group: &WellGroup<'_>,
        response: &Response,
    ) -> Option<CursorWellAction> {
        let ctx = ui.ctx();
        let t = &theme(ctx);
        let name = list_name(ctx, group);
        // A list left open as the well is disabled closes, choosing nothing.
        if !target.enabled {
            popup::close(ctx, popup::button_menu(response.id));
        }
        let chosen = pull_down(ui, response, &name, || group.entries());
        let open = popup::is_open(ctx, popup::button_menu(response.id));
        let touch = Touch::of(ctx, response, target.enabled, None);
        let rect = target.rect;
        outline(painter, rect, touch, t);
        let side = Icon::ChevronDown.size();
        // At its start, against its face.
        let icon = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - side / 2.0),
            vec2(side, side),
        );
        Icon::ChevronDown.paint(painter, icon, chevron_colour(t, touch, open));
        paint_focus(painter, rect, t.shape.radius, touch, t);
        let enabled = target.enabled;
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, &name));
        let tip = self.disabled.unwrap_or(&name);
        response
            .clone()
            .on_hover_text(tip)
            .on_disabled_hover_text(tip);
        group.answer(target.group, &chosen?)
    }
}

/// One face or chevron of the well.
struct Target {
    group: usize,
    chevron: bool,
    rect: Rect,
    id: Id,
    enabled: bool,
}

/// The name of `group`'s list and of its chevron: "Pick cursors and settings", or its
/// cursors or its settings alone.
fn list_name(ctx: &egui::Context, group: &WellGroup<'_>) -> String {
    let has = match (group.cursors.len() > 1, !group.settings.is_empty()) {
        (true, true) => "both",
        (true, false) => "cursors",
        _ => "settings",
    };
    let mut args = Args::new();
    args.set("group", group.name);
    args.set("has", has);
    translator(ctx).text_with("cursor-list", Some(&args))
}

/// High contrast's outline round a target under the pointer or held down, as an icon
/// button's; the other themes' `button_edge` is transparent.
fn outline(painter: &egui::Painter, rect: Rect, touch: Touch, t: &Theme) {
    if touch.enabled && (touch.hovered || touch.pressed) && t.colors.button_edge.a() > 0 {
        painter.rect_stroke(
            rect,
            t.shape.radius,
            Stroke::new(t.shape.hairline, t.colors.button_edge),
            StrokeKind::Inside,
        );
    }
}

/// A face's icon (`docs/cursors.md`, The well): `accent_text` while chosen, its cursor
/// ready or not, or held down, `text_strong` under the pointer, `text_secondary` at rest
/// and focused, `text_disabled` disabled.
fn face_colour(t: &Theme, touch: Touch, chosen: bool) -> Color32 {
    let c = &t.colors;
    if touch.pressed || chosen {
        c.accent_text
    } else if !touch.enabled {
        c.text_disabled
    } else if touch.hovered {
        c.text_strong
    } else {
        c.text_secondary
    }
}

/// A chevron's: as a face's, but `text_strong` while its list is open, never the accent but
/// held down.
fn chevron_colour(t: &Theme, touch: Touch, open: bool) -> Color32 {
    let c = &t.colors;
    if !touch.enabled {
        c.text_disabled
    } else if touch.pressed {
        c.accent_text
    } else if touch.hovered || open {
        c.text_strong
    } else {
        c.text_secondary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Builtin;

    fn touch(enabled: bool, hovered: bool, pressed: bool, focused: bool) -> Touch {
        Touch {
            enabled,
            hovered,
            pressed,
            focused,
        }
    }

    #[test]
    fn a_face_s_colour_follows_cursors_md() {
        for which in Builtin::ALL {
            let t = Theme::builtin(which);
            let c = &t.colors;
            let rest = touch(true, false, false, false);
            let hover = touch(true, true, false, false);
            let held = touch(true, true, true, false);
            let focus = touch(true, false, false, true);
            let off = touch(false, true, false, false);
            assert_eq!(face_colour(&t, rest, false), c.text_secondary, "{which:?}");
            assert_eq!(face_colour(&t, hover, false), c.text_strong, "{which:?}");
            assert_eq!(face_colour(&t, held, false), c.accent_text, "{which:?}");
            assert_eq!(face_colour(&t, focus, false), c.text_secondary, "{which:?}");
            // Chosen: the accent, under the pointer too, and nothing else.
            assert_eq!(face_colour(&t, rest, true), c.accent_text, "{which:?}");
            assert_eq!(face_colour(&t, hover, true), c.accent_text, "{which:?}");
            // A chosen cursor not ready keeps the accent; an unchosen one is dimmed.
            assert_eq!(face_colour(&t, off, true), c.accent_text, "{which:?}");
            assert_eq!(face_colour(&t, off, false), c.text_disabled, "{which:?}");
            assert_eq!(chevron_colour(&t, rest, true), c.text_strong, "{which:?}");
            assert_eq!(
                chevron_colour(&t, rest, false),
                c.text_secondary,
                "{which:?}"
            );
        }
    }

    #[test]
    fn a_list_holds_the_cursors_then_the_settings_and_answers_by_its_lines() {
        let picks = [
            WellCursor::new(Icon::PickWhiteBalance, "White balance picker").key("K"),
            WellCursor::new(Icon::PickExposure, "Exposure picker").disabled("Not yet"),
        ];
        let options = ["The whole edit", "The parts"];
        let settings = [
            WellSetting::Toggle {
                name: "Show the last pick",
                on: true,
            },
            WellSetting::Choice {
                name: "A click applies",
                options: &options,
                chosen: 1,
            },
        ];
        let group = WellGroup::new("Pick", &picks, 0).settings(&settings);
        let entries = group.entries();
        assert_eq!(
            entries,
            vec![
                Item::new("cursor:0", "White balance picker")
                    .checked(true)
                    .shortcut("K")
                    .into(),
                Item::new("cursor:1", "Exposure picker")
                    .checked(false)
                    .enabled(false)
                    .tip("Not yet")
                    .into(),
                Entry::Separator,
                Item::new("toggle:0", "Show the last pick")
                    .checked(true)
                    .into(),
                Entry::Heading("A click applies".to_owned()),
                Item::new("choice:1:0", "The whole edit")
                    .checked(false)
                    .into(),
                Item::new("choice:1:1", "The parts").checked(true).into(),
            ]
        );
        assert_eq!(
            group.answer(2, "cursor:0"),
            Some(CursorWellAction::Cursor {
                group: 2,
                cursor: 0
            })
        );
        // A cursor not ready, as one that stopped being ready while its list was open.
        assert_eq!(group.answer(2, "cursor:1"), None);
        assert_eq!(
            group.answer(2, "toggle:0"),
            Some(CursorWellAction::Toggle {
                group: 2,
                setting: 0,
                on: false
            })
        );
        assert_eq!(
            group.answer(2, "choice:1:0"),
            Some(CursorWellAction::Choice {
                group: 2,
                setting: 1,
                option: 0
            })
        );
        assert_eq!(group.answer(2, "toggle:1"), None);
        assert_eq!(group.answer(2, "cursor:2"), None);
        assert_eq!(group.answer(2, "choice:1:2"), None);
        assert_eq!(group.answer(2, "choice:0:0"), None);
        // A group of one lists its settings alone.
        let one = [WellCursor::new(Icon::Apply, "Copy and apply")];
        let entries = WellGroup::new("Copy and apply", &one, 0)
            .settings(&settings[1..])
            .entries();
        assert!(matches!(entries[0], Entry::Heading(_)));
    }
}
