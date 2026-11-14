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

//! The drawn popups (`docs/gui.md`, The menu bar, The drawn menus). The last popup opened
//! takes its keys out of each frame's input before egui reads it, so they are its own and
//! nothing under it sees them; [`show`] draws every open one and takes the pointer, where
//! the app calls it in the frame. An answer lasts the frame it arrives on, and is gone
//! once read.
//!
//! - A dropdown, `Choice`'s list, opens under its anchor with its end edge on the anchor's
//!   and at least as wide, above it where it would run off the window's bottom, the
//!   chosen option checked and highlighted. Typed text filters it as darktable's combo
//!   boxes do, as `filter` has it: the matches in SemiBold, the other options at half
//!   strength; ↑ and ↓ move the highlight among the options that match and can be chosen;
//!   Enter or Space choose the highlighted one, with text typed Enter the only match that
//!   can be; Backspace takes a character back; Escape closes it.
//! - A menu opens under its anchor on `raised`, with the menus' border and shadow, and a
//!   submenu to the side of its line, towards the end where it fits, else towards the
//!   start.
//! - ↓ and ↑ move through the lines but the separators, disabled ones included, so
//!   screen readers learn they exist; → or Enter opens a submenu and ← closes it; Enter or
//!   Space chooses an item; Escape closes a level, the last giving the focus back; Tab
//!   closes them all; a letter moves to the next line starting with it.
//! - An item with an alternate shows it in its place while Option (Alt) is held, as
//!   macOS's alternate items do: a click, or Enter with it held, chooses the alternate.
//! - Under the pointer a line is chosen at once, and a submenu opens, or another closes,
//!   once the pointer has rested on it a moment, so it can cross other lines on its way
//!   to a submenu. A press outside closes them all.
//! - The focus is on the chosen line, so screen readers read it, in a menu with its name;
//!   the lines are menu items, a check box for one that can be checked, a submenu's
//!   telling that it opens a menu and whether it is open.

use std::borrow::Cow;
use std::sync::Arc;

use egui::accesskit::{HasPopup, Orientation, Role};
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Area, Color32, Context, Event, FontFamily, FontId, Frame, Id, Key, Order, Pos2, RawInput, Rect,
    Response, Sense, Stroke, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::{Answer, Entry, Kind, Request, Service, filter, words};
use crate::{Icon, Theme, fonts, theme};

/// How long the pointer rests on a line before its submenu opens, or another closes.
const PAUSE: f64 = 0.2;

/// The plugin that gives the last popup opened its keys, before each frame. It draws
/// nothing: egui would deadlock, in a debug build, on a widget under the pointer drawn
/// while a plugin runs.
pub(super) struct Drawn;

impl egui::plugin::Plugin for Drawn {
    fn debug_name(&self) -> &'static str {
        "apertrust popups"
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        take_keys(ctx, input);
    }
}

/// Every open popup and the answers of this frame, in egui's memory; and the installed
/// service, with the popups it shows and the answers it gave since the last frame.
#[derive(Clone, Default)]
pub(super) struct Popups {
    open: Vec<(Id, Open)>,
    answers: Vec<(Id, Answer)>,
    pub(super) service: Option<Arc<dyn Service>>,
    native: Vec<(Id, Native)>,
    /// The service's answers, each with where the focus goes back, for the next frame.
    arriving: Vec<(Id, Answer, Option<Id>)>,
}

/// A popup the service shows.
#[derive(Clone)]
struct Native {
    anchor: Rect,
    /// Where the focus goes back as it closes.
    restore: Option<Id>,
}

impl Popups {
    /// Opens `id` as the service's, in place of any popup open under it.
    pub(super) fn open_native(&mut self, id: Id, anchor: Rect, restore: Option<Id>) {
        self.open.retain(|(open, _)| *open != id);
        self.native.retain(|(open, _)| *open != id);
        self.native.push((id, Native { anchor, restore }));
    }

    pub(super) fn is_native(&self, id: Id) -> bool {
        self.native.iter().any(|(open, _)| *open == id)
    }

    /// The service's `answer` for `id`, kept for the next frame, if `id` is still its.
    pub(super) fn answered(&mut self, id: Id, answer: Answer) {
        if let Some(at) = self.native.iter().position(|(open, _)| *open == id) {
            let (_, native) = self.native.remove(at);
            self.arriving.push((id, answer, native.restore));
        }
    }

    pub(super) fn open(
        &mut self,
        id: Id,
        request: Request,
        restore: Option<Id>,
        pointer: Option<Pos2>,
        pass: u64,
    ) {
        // Opened again in place of itself, it still gives the focus back to what had it
        // first, not to one of its own lines.
        let restore = self
            .open
            .iter()
            .find(|(open, _)| *open == id)
            .map_or(restore, |(_, open)| open.restore);
        self.open.retain(|(open, _)| *open != id);
        self.native.retain(|(open, _)| *open != id);
        let top = &request.entries;
        let expanded = request.expanded.as_deref().and_then(|wanted| {
            top.iter()
                .position(|entry| matches!(entry, Entry::Submenu(submenu) if submenu.id == wanted))
        });
        let path = match (request.kind, expanded) {
            // The checked option, else the first that can be chosen.
            (Kind::Dropdown, _) => vec![
                top.iter()
                    .position(
                        |entry| matches!(entry, Entry::Item(item) if item.checked == Some(true)),
                    )
                    .or_else(|| filter::step(top, "", None, 1)),
            ],
            (_, Some(index)) => vec![Some(index), step(entries_of(top, &[Some(index)]), None, 1)],
            (_, None) => vec![step(top, None, 1)],
        };
        self.open.push((
            id,
            Open {
                request,
                path,
                hover: None,
                pointer,
                restore,
                rects: Vec::new(),
                drawn: pass,
                shown: None,
                filter: String::new(),
            },
        ));
    }

    /// Closes `id`, drawn or the service's, with `answer`; where the focus goes back, if it
    /// was open.
    pub(super) fn close(&mut self, id: Id, answer: Answer) -> Option<Option<Id>> {
        let restore = if let Some(at) = self.open.iter().position(|(open, _)| *open == id) {
            self.open.remove(at).1.restore
        } else {
            let at = self.native.iter().position(|(open, _)| *open == id)?;
            self.native.remove(at).1.restore
        };
        self.answers.push((id, answer));
        Some(restore)
    }

    pub(super) fn anchor(&self, id: Id) -> Option<Rect> {
        let drawn = self.open.iter().find(|(open, _)| *open == id);
        let native = self.native.iter().find(|(open, _)| *open == id);
        drawn
            .map(|(_, open)| open.request.anchor)
            .or_else(|| native.map(|(_, native)| native.anchor))
    }

    /// The answer of `id`, taken: read once, it is gone.
    pub(super) fn answer(&mut self, id: Id) -> Option<Answer> {
        let at = self
            .answers
            .iter()
            .position(|(answered, _)| *answered == id)?;
        Some(self.answers.remove(at).1)
    }

    fn update(&mut self, id: Id, state: Open) {
        if let Some((_, open)) = self.open.iter_mut().find(|(open, _)| *open == id) {
            *open = state;
        }
    }
}

/// Runs `f` on the popups in egui's memory. It holds the context's lock: `f` must not
/// call the context.
pub(super) fn with<R>(ctx: &Context, f: impl FnOnce(&mut Popups) -> R) -> R {
    ctx.data_mut(|d| f(d.get_temp_mut_or_default::<Popups>(Id::unique("apertrust-ui popups"))))
}

/// One open popup.
#[derive(Clone)]
struct Open {
    request: Request,
    /// The chosen line of each open level, from the top. Every level but the last shows
    /// the submenu chosen in it as the next level; a submenu the pointer opened has none
    /// chosen yet.
    path: Vec<Option<usize>>,
    /// The line the pointer rests on where resting acts: a submenu's line, or a line of a
    /// level with a submenu open from another; and since when.
    hover: Option<Hover>,
    /// Where the pointer was last frame, so only its moves choose a line.
    pointer: Option<Pos2>,
    /// What had the focus as it opened.
    restore: Option<Id>,
    /// Each level's rectangle as last drawn, for presses outside.
    rects: Vec<Rect>,
    /// The pass it was opened in or last drawn in ([`show`]).
    drawn: u64,
    /// The pass it was last shown in, so a second [`show`] in a pass leaves it be.
    shown: Option<u64>,
    /// What is typed to filter a dropdown.
    filter: String,
}

#[derive(Clone, Copy, PartialEq)]
struct Hover {
    level: usize,
    index: usize,
    since: f64,
}

/// How a popup closes this frame: its answer, and whether the focus goes back.
type Closing = (Answer, bool);

/// Draws every open popup, applies what the pointer did to it, and answers those that
/// close.
pub(super) fn show(ctx: &Context) {
    let open = with(ctx, |popups| popups.open.clone());
    let pass = ctx.cumulative_pass_nr();
    for (id, mut state) in open {
        if state.shown == Some(pass) {
            continue;
        }
        state.shown = Some(pass);
        state.drawn = pass;
        let closing = run(ctx, id, &mut state);
        settle(ctx, id, state, closing);
    }
}

/// Keeps `state` as the popup `id`'s, or closes it, giving the focus back if asked.
fn settle(ctx: &Context, id: Id, state: Open, closing: Option<Closing>) {
    match closing {
        None => with(ctx, |popups| popups.update(id, state)),
        Some((answer, give_back)) => {
            with(ctx, |popups| popups.close(id, answer));
            // A frame without the popup, though nothing else asks for one.
            ctx.request_repaint();
            if give_back && let Some(restore) = state.restore {
                ctx.memory_mut(|m| m.request_focus(restore));
            }
        }
    }
}

fn run(ctx: &Context, id: Id, open: &mut Open) -> Option<Closing> {
    let press = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.press_origin()));
    if let Some(Some(at)) = press
        && !open.request.inside.contains(at)
        && !open.rects.iter().any(|rect| rect.contains(at))
    {
        return Some((Answer::Dismissed, false));
    }
    let pointer = draw(ctx, id, open);
    if let Some(closing) = point(ctx, open, pointer) {
        return Some(closing);
    }
    focus(id, ctx, open);
    None
}

/// Before a frame: clears the last frame's answers, takes the service's since then as this
/// frame's, giving the focus back where each says, and gives the last popup opened its
/// keys, taken out of the frame's input. A popup the last pass neither opened nor drew is
/// closed and takes none, so a frame that skipped [`show`] never eats a key.
fn take_keys(ctx: &Context, input: &mut RawInput) {
    let (last, restores) = with(ctx, |popups| {
        popups.answers.clear();
        let mut restores = Vec::new();
        for (id, answer, restore) in std::mem::take(&mut popups.arriving) {
            popups.answers.push((id, answer));
            restores.extend(restore);
        }
        (popups.open.last().cloned(), restores)
    });
    for restore in restores {
        ctx.memory_mut(|m| m.request_focus(restore));
    }
    let Some((id, mut open)) = last else {
        return;
    };
    // The pass about to run is the one after the last completed.
    if open.drawn + 1 < ctx.cumulative_pass_nr() {
        with(ctx, |popups| popups.close(id, Answer::Dismissed));
        return;
    }
    // What a press with Alt typed stays with it, the app's, so Alt with a menu's access key
    // is read by the letter the layout typed, as ф on a Ukrainian keyboard.
    let mut after_alt = false;
    let (mine, others) = std::mem::take(&mut input.events)
        .into_iter()
        .partition::<Vec<_>, _>(|event| {
            let ours = is_ours(event) && !(after_alt && matches!(event, Event::Text(_)));
            after_alt =
                matches!(event, Event::Key { pressed: true, modifiers, .. } if modifiers.alt);
            ours
        });
    input.events = others;
    if mine.is_empty() {
        return;
    }
    let closing = keys(&mut open, mine);
    settle(ctx, id, open, closing);
}

/// What `events`, the popup's keys, do to it, in order.
fn keys(open: &mut Open, events: Vec<Event>) -> Option<Closing> {
    open.hover = None;
    if open.request.kind == Kind::Dropdown {
        return dropdown_keys(open, events);
    }
    for event in events {
        let level = open.path.len() - 1;
        let entries = entries_of(&open.request.entries, &open.path[..level]);
        let chosen = open.path[level];
        let line = chosen.and_then(|index| entries.get(index));
        match event {
            Event::Key { key, modifiers, .. } => match key {
                Key::ArrowDown => open.path[level] = step(entries, chosen, 1),
                Key::ArrowUp => open.path[level] = step(entries, chosen, -1),
                Key::ArrowRight | Key::Enter | Key::Space => match line {
                    Some(Entry::Submenu(submenu)) => {
                        open.path.push(step(&submenu.entries, None, 1));
                    }
                    Some(Entry::Item(item))
                        if key != Key::ArrowRight && item.held(modifiers.alt).enabled =>
                    {
                        let item = item.held(modifiers.alt);
                        return Some((Answer::Chosen(item.id.clone()), true));
                    }
                    _ => {}
                },
                Key::ArrowLeft => {
                    if level > 0 {
                        open.path.pop();
                    }
                }
                Key::Escape => {
                    if level == 0 {
                        return Some((Answer::Dismissed, true));
                    }
                    open.path.pop();
                }
                Key::Tab => return Some((Answer::Dismissed, true)),
                _ => {}
            },
            Event::Text(text) => {
                if let Some(letter) = text.chars().next().filter(|c| !c.is_whitespace()) {
                    open.path[level] =
                        starting_with(entries, chosen, letter, level == 0).or(chosen);
                }
            }
            _ => {}
        }
    }
    None
}

/// What `events` do to a dropdown (`docs/gui.md`, Choice rows: two styles): typed text
/// filters it, ↑ and ↓ move among the options that match and can be chosen, Enter or
/// Space choose.
fn dropdown_keys(open: &mut Open, events: Vec<Event>) -> Option<Closing> {
    let entries = &open.request.entries;
    for event in events {
        match event {
            Event::Key {
                key,
                repeat,
                pressed: true,
                modifiers,
                ..
            } => match key {
                Key::ArrowDown | Key::ArrowUp => {
                    let by = if key == Key::ArrowDown { 1 } else { -1 };
                    let to = filter::step(entries, &open.filter, open.path[0], by);
                    open.path[0] = to.or(open.path[0]);
                }
                // A held key opening the list must not choose from it.
                Key::Enter | Key::Space if repeat => {}
                // Once text is typed a space is part of it, from its own text event.
                Key::Space if !open.filter.is_empty() => {}
                Key::Enter | Key::Space => {
                    // The option shown: its alternate where Alt is held with the key.
                    let chosen = if open.filter.is_empty() {
                        // The highlighted option, which starts on the chosen one, so a bare
                        // Enter just closes; one that cannot be chosen closes it too.
                        match open.path[0].and_then(|at| entries.get(at)) {
                            Some(Entry::Item(item)) if item.held(modifiers.alt).enabled => {
                                Some(item.held(modifiers.alt))
                            }
                            _ => return Some((Answer::Dismissed, true)),
                        }
                    } else {
                        filter::only_choice(entries, &open.filter)
                            .map(|item| item.held(modifiers.alt))
                            .filter(|item| item.enabled)
                    };
                    // Several matches, or none that can be chosen: darktable keeps the list
                    // open for more typing.
                    if let Some(item) = chosen {
                        return Some((Answer::Chosen(item.id.clone()), true));
                    }
                }
                Key::Backspace | Key::Delete => {
                    if open.filter.pop().is_some() {
                        retarget(entries, &open.filter, &mut open.path[0]);
                    }
                }
                Key::Escape | Key::Tab => return Some((Answer::Dismissed, true)),
                _ => {}
            },
            Event::Text(text) => {
                let before = open.filter.len();
                for c in text.chars().filter(|c| !c.is_control()) {
                    // A space starts nothing; the Space key chooses.
                    if c != ' ' || !open.filter.is_empty() {
                        open.filter = filter::typed(entries, &open.filter, c);
                    }
                }
                if open.filter.len() != before {
                    retarget(entries, &open.filter, &mut open.path[0]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Moves a dropdown's `highlight` to the first option that matches `typed` and can be
/// chosen, unless it is on one.
fn retarget(entries: &[Entry], typed: &str, highlight: &mut Option<usize>) {
    let on_one = highlight
        .and_then(|at| entries.get(at))
        .is_some_and(|entry| filter::reachable(entry, typed));
    if !on_one && let Some(first) = filter::step(entries, typed, None, 1) {
        *highlight = Some(first);
    }
}

/// Whether the popup takes `event`: text typed, or a press of any key without Ctrl, Cmd
/// or Alt but the F-keys, so nothing under it acts on a letter typed to find a line, and
/// Enter with Alt alone, which chooses an item's alternate. A key with a modifier and the
/// F-keys stay the app's, and so does the text a press with Alt typed ([`take_keys`]).
fn is_ours(event: &Event) -> bool {
    match event {
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            !(modifiers.ctrl || modifiers.command || modifiers.mac_cmd)
                && (!modifiers.alt || *key == Key::Enter)
                && !is_function_key(*key)
        }
        Event::Text(_) => true,
        _ => false,
    }
}

/// F1 to F35.
fn is_function_key(key: Key) -> bool {
    key.name()
        .strip_prefix('F')
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

/// What the pointer did to the lines this frame.
#[derive(Default)]
struct Pointer {
    /// A line it moved over, by level and index.
    hovered: Option<(usize, usize)>,
    clicked: Option<(usize, usize)>,
}

/// Draws each open level, the next to the side of the line it opens from.
fn draw(ctx: &Context, id: Id, open: &mut Open) -> Pointer {
    let t = theme(ctx);
    let mut pointer = Pointer::default();
    let at = ctx.input(|i| i.pointer.hover_pos());
    let moved = at.is_some() && at != open.pointer;
    open.pointer = at;
    let alt = ctx.input(|i| i.modifiers.alt);
    let screen = ctx.content_rect();
    let deepest = open.path.len() - 1;
    let mut rects = Vec::with_capacity(open.path.len());
    // The line the level being drawn opens from, its id and its level's rectangle.
    let mut parent: Option<(Rect, Id, Rect)> = None;
    for level in 0..open.path.len() {
        let entries = entries_of(&open.request.entries, &open.path[..level]);
        let submenu = level.checked_sub(1).and_then(|above| {
            let index = open.path[above]?;
            match entries_of(&open.request.entries, &open.path[..above]).get(index) {
                Some(Entry::Submenu(submenu)) => Some(submenu),
                _ => None,
            }
        });
        let area_id = match submenu {
            Some(submenu) => id.with(("submenu", &submenu.id)),
            None => id.with("menu"),
        };
        // A submenu's name is its line's words.
        let name = match submenu {
            Some(submenu) if level == 1 => words(&submenu.text).0,
            Some(submenu) => submenu.text.clone(),
            None => open.request.name.clone(),
        };
        let dropdown = open.request.kind == Kind::Dropdown;
        let mut layout = Layout::of(ctx, &t, entries, level == 0, dropdown);
        // The frame's border and margin round the lines.
        let edge = t.spacing.xs + t.shape.popup_hairline;
        let anchor = open.request.anchor;
        if dropdown {
            // At least as wide as what it hangs from.
            layout.width = layout.width.max(anchor.width() - 2.0 * edge);
        }
        let position = match parent {
            None if dropdown => {
                // Its end edge on the anchor's, under it, or over it where the window's
                // bottom would cut it.
                let size = vec2(
                    layout.width + 2.0 * edge,
                    entries.len() as f32 * t.sizes.row + 2.0 * edge,
                );
                let under = anchor.bottom() + t.spacing.xxs;
                let over = anchor.top() - t.spacing.xxs - size.y;
                let y = if under + size.y > screen.bottom() && over >= screen.top() {
                    over
                } else {
                    under
                };
                pos2(anchor.right() - size.x, y)
            }
            None => open.request.anchor.left_bottom() + vec2(0.0, t.spacing.xxs),
            Some((line, _, above)) => {
                let width = layout.width + 2.0 * edge;
                let end = above.right() - t.spacing.xxs;
                let x = if end + width <= screen.right() {
                    end
                } else {
                    above.left() + t.spacing.xxs - width
                };
                pos2(x, line.top() - edge)
            }
        };
        // The line shown chosen: where the pointer rests to close this level's submenu,
        // else the one chosen.
        let shown = open
            .hover
            .filter(|h| h.level == level && level < deepest)
            .map(|h| h.index)
            .or(open.path[level]);
        let mut area = Area::new(area_id)
            .order(Order::Foreground)
            .fixed_pos(position)
            .constrain(true)
            .role(Role::Menu)
            .accessible_name(name);
        if let Some((_, line_id, _)) = parent {
            area = area.accessibility_parent(line_id);
        }
        let mut lines = Vec::new();
        let response = area.show(ctx, |ui| {
            Frame::menu(ui.style())
                .corner_radius(t.shape.menu_radius)
                .inner_margin(t.spacing.xs)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    for (index, entry) in entries.iter().enumerate() {
                        let open_below = level < deepest && open.path[level] == Some(index);
                        // The line keeps the item's id, so the focus stays on it as Option
                        // shows its alternate and hides it again.
                        let held = held(entry, alt);
                        let line = Line {
                            entry: &held,
                            id: line_id(id, entry),
                            chosen: shown == Some(index),
                            open: open_below,
                            top: level == 0,
                            underline: alt && level == 0,
                            filter: if dropdown { &open.filter } else { "" },
                        };
                        if let Some(response) = line.show(ui, &t, &layout) {
                            lines.push((index, response));
                        }
                    }
                });
        });
        let rect = response.response.rect;
        rects.push(rect);
        parent = None;
        for (index, response) in lines {
            if response.clicked() {
                pointer.clicked = Some((level, index));
            }
            if moved && response.hovered() {
                pointer.hovered = Some((level, index));
            }
            if open.path[level] == Some(index) {
                parent = Some((response.rect, response.id, rect));
            }
        }
    }
    open.rects = rects;
    pointer
}

/// What the pointer did, applied: a line clicked, or one it rests on.
fn point(ctx: &Context, open: &mut Open, pointer: Pointer) -> Option<Closing> {
    let now = ctx.input(|i| i.time);
    let alt = alt_clicked(ctx);
    let deepest = open.path.len() - 1;
    if let Some((level, index)) = pointer.clicked {
        match entries_of(&open.request.entries, &open.path[..level]).get(index) {
            Some(Entry::Item(item)) if item.held(alt).enabled => {
                return Some((Answer::Chosen(item.held(alt).id.clone()), true));
            }
            Some(Entry::Submenu(_)) => {
                open.path.truncate(level + 1);
                open.path[level] = Some(index);
                open.path.push(None);
                open.hover = None;
            }
            _ => {}
        }
        return None;
    }
    if let Some((level, index)) = pointer.hovered {
        let entries = entries_of(&open.request.entries, &open.path[..level]);
        // A dropdown's highlight is where an option can be chosen: the pointer passes over
        // one that cannot be, which is shown as it is.
        if open.request.kind == Kind::Dropdown
            && matches!(entries.get(index), Some(Entry::Item(item)) if !item.enabled)
        {
            return None;
        }
        let submenu = matches!(entries.get(index), Some(Entry::Submenu(_)));
        let rest = Hover {
            level,
            index,
            since: now,
        };
        if level == deepest {
            open.path[level] = Some(index);
            // Moving along the line it rests on keeps the time it came to it.
            let same = open
                .hover
                .is_some_and(|h| (h.level, h.index) == (level, index));
            if !same {
                open.hover = submenu.then_some(rest);
            }
        } else if open.path[level] == Some(index) {
            // Back on the line whose submenu is open.
            open.hover = None;
        } else if open
            .hover
            .is_none_or(|h| (h.level, h.index) != (level, index))
        {
            open.hover = Some(rest);
        }
    }
    if let Some(hover) = open.hover {
        let waited = now - hover.since;
        if waited >= PAUSE {
            open.path.truncate(hover.level + 1);
            open.path[hover.level] = Some(hover.index);
            let entries = entries_of(&open.request.entries, &open.path[..hover.level]);
            if matches!(entries.get(hover.index), Some(Entry::Submenu(_))) {
                open.path.push(None);
            }
            open.hover = None;
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after_secs((PAUSE - waited) as f32);
        }
    }
    None
}

/// Puts the focus on the chosen line of the deepest level, or, in a submenu the pointer
/// opened, on the line it opened from.
fn focus(id: Id, ctx: &Context, open: &Open) {
    let mut target = None;
    let mut entries = open.request.entries.as_slice();
    for chosen in &open.path {
        let Some(entry) = chosen.and_then(|index| entries.get(index)) else {
            break;
        };
        target = Some(line_id(id, entry));
        if let Entry::Submenu(submenu) = entry {
            entries = &submenu.entries;
        }
    }
    let Some(target) = target else {
        return;
    };
    ctx.memory_mut(|m| {
        if m.focused() != Some(target) {
            m.request_focus(target);
        }
    });
}

/// The entries of the level below the chosen submenus of `path`.
fn entries_of<'a>(top: &'a [Entry], path: &[Option<usize>]) -> &'a [Entry] {
    let mut entries = top;
    for chosen in path {
        match chosen.and_then(|index| entries.get(index)) {
            Some(Entry::Submenu(submenu)) => entries = &submenu.entries,
            _ => return &[],
        }
    }
    entries
}

/// The line `by` lines on from `from`, separators skipped, wrapping round; from none,
/// the first going down and the last going up.
fn step(entries: &[Entry], from: Option<usize>, by: isize) -> Option<usize> {
    let lines: Vec<usize> = (0..entries.len())
        .filter(|&i| !matches!(entries[i], Entry::Separator))
        .collect();
    let count = lines.len() as isize;
    if count == 0 {
        return None;
    }
    let at = match from.and_then(|from| lines.iter().position(|&i| i == from)) {
        Some(at) => (at as isize + by).rem_euclid(count),
        None if by > 0 => 0,
        None => count - 1,
    };
    Some(lines[at as usize])
}

/// The next line after `from`, wrapping round, whose words start with `letter`, in a
/// level at the `top` or not.
fn starting_with(entries: &[Entry], from: Option<usize>, letter: char, top: bool) -> Option<usize> {
    let wanted: String = letter.to_lowercase().collect();
    let start = from.map_or(0, |from| from + 1);
    (0..entries.len())
        .map(|i| (start + i) % entries.len())
        .find(|&i| {
            !matches!(entries[i], Entry::Separator)
                && shown(&entries[i], top)
                    .0
                    .to_lowercase()
                    .starts_with(&wanted)
        })
}

/// A line's words as shown, and its access key with the key's index among them. Only a
/// submenu at the `top` has one, marked by its `&`; every other line is shown as given.
fn shown(entry: &Entry, top: bool) -> (String, Option<(usize, char)>) {
    match entry {
        Entry::Submenu(submenu) if top => words(&submenu.text),
        Entry::Submenu(submenu) => (submenu.text.clone(), None),
        Entry::Item(item) => (item.text.clone(), None),
        Entry::Separator => (String::new(), None),
    }
}

/// Whether Alt (Option) was held for a click this frame: as the primary button was let go,
/// by its release's own modifiers, not the input's, which a change later in the frame
/// would have moved on; the input's where no release came, as a screen reader's click.
fn alt_clicked(ctx: &Context) -> bool {
    ctx.input(|i| {
        (i.events.iter().rev())
            .find_map(|event| match event {
                Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                    ..
                } => Some(modifiers.alt),
                _ => None,
            })
            .unwrap_or(i.modifiers.alt)
    })
}

/// `entry` as shown with Option (Alt) held or not, `alt`: an item's alternate in its place
/// while it is held.
fn held(entry: &Entry, alt: bool) -> Cow<'_, Entry> {
    match entry {
        Entry::Item(item) if alt && item.alternate.is_some() => {
            Cow::Owned(Entry::Item(item.held(true).clone()))
        }
        _ => Cow::Borrowed(entry),
    }
}

/// A line's widget id: the popup's with the entry's id, so the focus follows the entry.
fn line_id(popup: Id, entry: &Entry) -> Id {
    match entry {
        Entry::Item(item) => popup.with(("item", &item.id)),
        Entry::Submenu(submenu) => popup.with(("submenu-line", &submenu.id)),
        Entry::Separator => popup.with("separator"),
    }
}

/// A level's measures, from its widest words: the room a check mark takes when a line can
/// be checked, and its width, for the words and the keys or a submenu's chevron.
struct Layout {
    check: f32,
    width: f32,
}

impl Layout {
    /// A dropdown's words are measured in SemiBold, which a match is drawn in, so its
    /// width stays as options match and stop matching.
    fn of(ctx: &Context, t: &Theme, entries: &[Entry], top: bool, dropdown: bool) -> Layout {
        let font = if dropdown {
            FontId::new(t.text.body, FontFamily::Name(fonts::SEMIBOLD.into()))
        } else {
            FontId::proportional(t.text.body)
        };
        let measure = |text: &str, font: FontId| {
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(text.to_owned(), font, t.colors.text)
                    .size()
                    .x
            })
        };
        let mut check = 0.0;
        let mut words_width: f32 = 0.0;
        let mut end: f32 = 0.0;
        for entry in entries {
            match entry {
                // An alternate takes the item's place, so the level is as wide as either.
                Entry::Item(item) => {
                    for item in std::iter::once(item).chain(item.alternate.as_deref()) {
                        if item.checked.is_some() {
                            check = Icon::Check.size() + t.spacing.s;
                        }
                        words_width = words_width.max(measure(&item.text, font.clone()));
                        if let Some(keys) = &item.shortcut {
                            end = end.max(measure(keys, key_font(t)));
                        }
                    }
                }
                Entry::Submenu(_) => {
                    words_width = words_width.max(measure(&shown(entry, top).0, font.clone()));
                    end = end.max(Icon::Chevron.size());
                }
                Entry::Separator => {}
            }
        }
        let gap = if end > 0.0 { t.spacing.xl } else { 0.0 };
        let width = t.spacing.m + check + words_width.ceil() + gap + end.ceil() + t.spacing.m;
        Layout { check, width }
    }
}

/// The font of an item's keys, at its end.
fn key_font(t: &Theme) -> FontId {
    FontId::proportional(t.text.small)
}

/// One line of a level.
struct Line<'a> {
    entry: &'a Entry,
    id: Id,
    /// Under the pointer or the keys.
    chosen: bool,
    /// A submenu whose level is open.
    open: bool,
    /// In the level at the top, where a submenu's `&` marks its access key.
    top: bool,
    /// Underline an access key: Alt is held, and the line is at the top.
    underline: bool,
    /// What is typed to filter a dropdown, empty when nothing is or it is not one.
    filter: &'a str,
}

impl Line<'_> {
    /// Draws it; its response, but for a separator.
    fn show(self, ui: &mut Ui, t: &Theme, layout: &Layout) -> Option<Response> {
        if let Entry::Separator = self.entry {
            let (id, rect) = ui.allocate_space(vec2(layout.width, t.spacing.m + t.spacing.xxs));
            ui.painter().hline(
                rect.x_range().shrink(t.spacing.m),
                rect.center().y,
                Stroke::new(t.shape.popup_hairline, t.colors.menu_edge),
            );
            // Read as a separator, which the keys never focus. Read-only: a menu's separator
            // never moves, as a splitter between panes can.
            ui.interact(rect, id, Sense::hover())
                .widget_info(|| WidgetInfo::new(Role::Splitter));
            ui.ctx().accesskit_node_builder(id, |node| {
                node.set_orientation(Orientation::Horizontal);
                node.set_read_only();
            });
            return None;
        }
        let (_, rect) = ui.allocate_space(vec2(layout.width, t.sizes.row));
        let response = ui.interact(rect, self.id, Sense::click());
        let (shortcut, checked, enabled) = match self.entry {
            Entry::Item(item) => (item.shortcut.as_deref(), item.checked, item.enabled),
            _ => (None, None, true),
        };
        let (shown, access_key) = shown(self.entry, self.top);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            if self.chosen {
                painter.rect_filled(rect, t.shape.radius_large, t.colors.menu_hover);
            }
            let (colour, secondary) = if !enabled {
                (t.colors.text_disabled, t.colors.text_disabled)
            } else if self.chosen {
                (t.colors.text_on_menu_hover, t.colors.text_on_menu_hover)
            } else {
                (t.colors.text, t.colors.text_secondary)
            };
            let mut x = rect.left() + t.spacing.m;
            if checked == Some(true) {
                let size = Icon::Check.size();
                // The leading mark is a step quieter than its words until the line is lit.
                Icon::Check.paint(
                    painter,
                    Rect::from_min_size(pos2(x, rect.center().y - size / 2.0), vec2(size, size)),
                    if self.chosen { colour } else { secondary },
                );
            }
            x += layout.check;
            let key = access_key.filter(|_| self.underline);
            let job = if self.filter.is_empty() {
                underlined(&shown, key, t, colour)
            } else {
                filtered(&shown, self.filter, t, colour, enabled)
            };
            let galley = painter.layout_job(job);
            painter.galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                colour,
            );
            let end = rect.right() - t.spacing.m;
            if let Some(keys) = shortcut {
                let galley = painter.layout_no_wrap(keys.to_owned(), key_font(t), secondary);
                painter.galley(
                    pos2(
                        end - galley.size().x,
                        rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    secondary,
                );
            } else if let Entry::Submenu(_) = self.entry {
                let size = Icon::Chevron.size();
                Icon::Chevron.paint(
                    painter,
                    Rect::from_center_size(
                        pos2(end - size / 2.0, rect.center().y),
                        vec2(size, size),
                    ),
                    secondary,
                );
            }
        }
        response.widget_info(|| match checked {
            Some(on) => WidgetInfo::selected(Role::MenuItemCheckBox, enabled, on, &shown),
            None => WidgetInfo::labeled(Role::MenuItem, enabled, &shown),
        });
        if let Entry::Item(item) = self.entry
            && let Some(tip) = &item.tip
        {
            response.clone().on_hover_text(tip);
        }
        let submenu = matches!(self.entry, Entry::Submenu(_));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            if submenu {
                node.set_has_popup(HasPopup::Menu);
                node.set_expanded(self.open);
            }
            if let Some(keys) = shortcut {
                node.set_keyboard_shortcut(keys);
            }
            if let Some((_, key)) = access_key {
                node.set_access_key(key.to_string());
            }
        });
        Some(response)
    }
}

/// A dropdown's `text` while `typed` filters it, as darktable draws it: the characters that
/// match in SemiBold and the rest as it is, and an option that matches nothing at half
/// strength. A disabled option is never a match, so it is drawn as it is, dim already.
fn filtered(text: &str, typed: &str, t: &Theme, colour: Color32, enabled: bool) -> LayoutJob {
    let font = FontId::proportional(t.text.body);
    let mut job = LayoutJob::default();
    let found = filter::find(text, typed).filter(|_| enabled);
    let Some(found) = found else {
        let colour = if enabled {
            colour.gamma_multiply(0.5)
        } else {
            colour
        };
        job.append(text, 0.0, TextFormat::simple(font, colour));
        return job;
    };
    let bold = FontId::new(t.text.body, FontFamily::Name(fonts::SEMIBOLD.into()));
    job.append(
        &text[..found.start],
        0.0,
        TextFormat::simple(font.clone(), colour),
    );
    job.append(&text[found.clone()], 0.0, TextFormat::simple(bold, colour));
    job.append(&text[found.end..], 0.0, TextFormat::simple(font, colour));
    job
}

/// `text` in `colour`, the character at `key`'s index underlined.
fn underlined(
    text: &str,
    key: Option<(usize, char)>,
    t: &Theme,
    colour: egui::Color32,
) -> LayoutJob {
    let plain = TextFormat::simple(FontId::proportional(t.text.body), colour);
    let mut job = LayoutJob::default();
    let Some((at, _)) = key else {
        job.append(text, 0.0, plain);
        return job;
    };
    let split = |n: usize| text.char_indices().nth(n).map_or(text.len(), |(i, _)| i);
    let (start, end) = (split(at), split(at + 1));
    job.append(&text[..start], 0.0, plain.clone());
    job.append(
        &text[start..end],
        0.0,
        TextFormat {
            underline: Stroke::new(t.shape.hairline, colour),
            ..plain.clone()
        },
    );
    job.append(&text[end..], 0.0, plain);
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::popup::{Item, Submenu};

    fn entries() -> Vec<Entry> {
        vec![
            Item::new("open", "Open").into(),
            Entry::Separator,
            Item::new("export", "Export").enabled(false).into(),
            Submenu::new("recent", "Open Recent", vec![]).into(),
            Item::new("quit", "Quit").into(),
        ]
    }

    #[test]
    fn steps_skip_separators_and_wrap_round() {
        let entries = entries();
        assert_eq!(step(&entries, None, 1), Some(0));
        assert_eq!(step(&entries, None, -1), Some(4));
        assert_eq!(step(&entries, Some(0), 1), Some(2));
        assert_eq!(step(&entries, Some(2), -1), Some(0));
        assert_eq!(step(&entries, Some(4), 1), Some(0));
        assert_eq!(step(&[Entry::Separator], None, 1), None);
    }

    #[test]
    fn a_letter_finds_the_next_line_starting_with_it() {
        let entries = entries();
        assert_eq!(starting_with(&entries, Some(0), 'o', false), Some(3));
        assert_eq!(starting_with(&entries, Some(3), 'O', false), Some(0));
        assert_eq!(starting_with(&entries, None, 'e', false), Some(2));
        assert_eq!(starting_with(&entries, Some(0), 'z', false), None);
    }
}
