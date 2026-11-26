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

//! The Toolbar's edit state, as its customise mode draws it (decision 92; `docs/gui.md`,
//! The customise mode): every entry drawn enabled and at rest on a plate, the spacings as
//! tiles, the parts invisible at rest with faces, on one line that never folds and scrolls
//! where it does not fit, a fold mark on what the panel at rest would take under » at this
//! width, and Done at its end. It wiggles once as it starts, unless motion is reduced.
//! What the person does, a click, a drag, a key or a right-click, comes back as data
//! ([`ToolbarEdit`]); the app changes its draft by it. No part of it moves the window, and
//! no press runs an action; the caption buttons still report.

use egui::accesskit::{Action, ActionData, CustomAction, Orientation};
use egui::{
    Align2, Color32, CursorIcon, FontId, Id, Key, LayerId, Modifiers, Order, Pos2, Rect, Response,
    Role, Sense, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::{
    ACTION, ACTION_ICON, ACTION_STROKE, BUTTON, Drawn, LABEL_GAP, LABEL_TOP, Labelled, SaveSlot,
    SaveState, Toolbar, ToolbarAction, ToolbarEntry, action_icon, between,
};
use crate::components::{Button, CursorWell, focus_ring, menu_asked_of};
use crate::{Icon, theme, tr};

/// What the person did to the Toolbar while it is customised, by the entries' places in
/// [`Toolbar::entries`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolbarEdit {
    /// The entry chosen, by a click, a press let go before it moved 6 points, or by ←, →,
    /// Home or End.
    Select(usize),
    /// The entry at `from` moved so it stands at `to` once taken out and put back: by
    /// Alt+← and Alt+→ (Option on the Mac), a drag let go on the Toolbar, or a screen
    /// reader's Move Left and Move Right.
    Move { from: usize, to: usize },
    /// The entry taken out, by Delete, Backspace or a screen reader's Remove from Toolbar.
    Remove(usize),
    /// The customise menu asked for: a right-click, a Control-click on the Mac, Shift+F10,
    /// the Menu key or ShowContextMenu.
    Menu(EditMenu),
    /// Done pressed, or Enter with no menu open.
    Done,
    /// Esc, or Cmd+. on the Mac, while no drag is under way.
    Cancel,
}

/// Where the customise menu was asked for ([`ToolbarEdit::Menu`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditMenu {
    /// The entry it opened on, which it selects, or the gap it opened in.
    pub at: ToolbarSpot,
    /// Where it hangs from, in points.
    pub pos: Pos2,
    /// The control the focus goes back to as it closes, asked for from the keys.
    pub opener: Option<Id>,
}

/// An entry of the list, or a gap between two, by its place: gap `n` stands before entry
/// `n`, the last after every entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarSpot {
    Entry(usize),
    Gap(usize),
}

/// What the edit state is given each frame ([`Toolbar::customising`]): the words of each
/// entry, which the component adds none of its own to, the selection, and when the mode
/// started.
#[derive(Clone, Debug, Default)]
pub struct Customising {
    names: Vec<String>,
    tips: Vec<String>,
    marked: String,
    release: Vec<String>,
    selected: Option<usize>,
    done_tip: String,
    newer: bool,
    since: f64,
    still: bool,
    keep_focus: bool,
}

impl Customising {
    /// Each entry's accessible name, its full words ("Orientation: Rotate 90° clockwise",
    /// "Flexible space"), and its tooltip, in the list's order.
    pub fn new(names: Vec<String>, tips: Vec<String>) -> Self {
        Customising {
            names,
            tips,
            ..Customising::default()
        }
    }

    /// The entry selected, by its place.
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// The line a marked entry's tooltip ends with, and its description to screen readers:
    /// "Under » at this width".
    pub fn marked(mut self, line: impl Into<String>) -> Self {
        self.marked = line.into();
        self
    }

    /// What a drag's release does at each gap, gap `n` before entry `n`: "Moves it after
    /// Undo". One more than the entries.
    pub fn release(mut self, lines: Vec<String>) -> Self {
        self.release = lines;
        self
    }

    /// Done's tooltip: "Saves this Toolbar for the Default workspace". Where `newer`, a
    /// newer Apertrust wrote the workspace's file, and the save state's warning stands
    /// before Done.
    pub fn done(mut self, tip: impl Into<String>, newer: bool) -> Self {
        self.done_tip = tip.into();
        self.newer = newer;
        self
    }

    /// Keeps the focus on the line or Done: whenever nothing has it, the line takes it, as
    /// the mode's two stops are the only ones (the app's; the gallery leaves it off).
    pub fn keep_focus(mut self, keep: bool) -> Self {
        self.keep_focus = keep;
        self
    }

    /// How long ago the mode started, in seconds of egui's input time: the entries wiggle
    /// for the theme's `wiggle_ms` from then, unless `still`, the system asking for reduced
    /// motion. The caller repaints while they do, so a moment can be pinned.
    pub fn since(mut self, seconds: f64, still: bool) -> Self {
        self.since = seconds;
        self.still = still;
        self
    }
}

/// What the panel at rest worked out, which the edit state draws from.
pub(super) struct Frame {
    pub bar: Rect,
    pub inner: Rect,
    pub response: Response,
    pub saving: SaveSlot,
    /// Each entry the fold at rest takes at this width.
    pub folded: Vec<bool>,
    /// Whether » shows at rest at this width.
    pub more: bool,
}

/// A plate's height with labels, and without.
const PLATE_LABELLED: f32 = 54.0;
const PLATE: f32 = 36.0;
/// A plate's radius, and its inset from its slot at each side.
const RADIUS: f32 = 5.0;
const INSET: f32 = 2.0;
/// How far a press moves before it lifts its entry.
const LIFT: f32 = 6.0;
/// A flexible space's tile is at least this wide.
const FLEXIBLE: f32 = 40.0;
/// The wiggle's reach either way, in points, and its cycles.
const WIGGLE: f32 = 1.5;
const CYCLES: f32 = 2.0;
/// The fold mark's size, the release line's, and the scroll bar's height.
const MARK: f32 = 10.0;
const RELEASE: f32 = 12.0;
const SCROLL_BAR: f32 = 3.0;
/// How near a scrolled line's end a drag scrolls it, and how fast, in points a second.
const EDGE: f32 = 32.0;
const EDGE_SPEED: f32 = 400.0;
/// How wide a scrolled line's fade is at an end with more.
const FADE: f32 = 16.0;
/// Done stands this far from the caption buttons, or on the Mac from the bar's end.
const DONE_GAP: f32 = 16.0;
const DONE_END: f32 = 12.0;
/// The custom actions' ids: Move Left, Move Right and Remove from Toolbar.
const MOVE_LEFT: i32 = 0;
const MOVE_RIGHT: i32 = 1;
const REMOVE: i32 = 2;

/// What the edit state keeps between frames.
#[derive(Clone, Copy, Debug, Default)]
struct Kept {
    /// How far the line is scrolled, in points.
    scroll: f32,
    /// The selection as last drawn, so a new one is scrolled into view once.
    selected: Option<usize>,
    /// The entry lifted, while a drag is under way, and where it was grabbed.
    lifted: Option<(usize, Vec2)>,
    /// A drag cancelled by Esc or the focus leaving the window, until its release.
    cancelled: bool,
    /// Whether the edit state was drawn last frame: the line takes the focus as it starts.
    shown: bool,
}

/// One entry's place on the line, in the line's own points from its start.
struct Slot<'e> {
    drawn: Drawn<'e>,
    x: f32,
    width: f32,
}

pub(super) fn show(
    toolbar: &Toolbar<'_>,
    edit: Customising,
    mut well: Option<CursorWell<'_>>,
    ui: &mut Ui,
    frame: Frame,
) -> Option<ToolbarAction> {
    let ctx = ui.ctx().clone();
    let t = theme(&ctx);
    let Frame {
        bar,
        inner,
        response,
        saving,
        folded,
        more,
    } = frame;
    let id = response.id.with("customising");
    let mut kept: Kept = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    let mut action = None;
    let n = toolbar.entries.len();
    // A drag whose entry is gone, the list changed under it, lifts nothing.
    kept.lifted = kept.lifted.filter(|(at, _)| *at < n);
    let labelled = toolbar.labels;
    let mac = ctx.os() == egui::os::OperatingSystem::Mac;

    // The caption buttons, at the very end, still the window's.
    let (caption, captions) = toolbar.captions(ui, bar);
    if caption.is_some() {
        action = caption;
    }

    // Done, at the end: 16 points before the caption buttons, or 12 from the bar's end.
    let done_word = tr(&ctx, "action-toolbar-done");
    let done_width = Button::new(&done_word).primary(true).natural_width(ui);
    let done_right = match toolbar.caption_buttons {
        Some(style) => bar.max.x - style.width() - DONE_GAP,
        None => bar.max.x - DONE_END,
    };
    let done_rect = Rect::from_center_size(
        pos2(done_right - done_width / 2.0, bar.center().y),
        vec2(done_width, t.sizes.control),
    );
    let done = ui.scope_builder(UiBuilder::new().max_rect(done_rect), |ui| {
        Button::new(&done_word)
            .primary(true)
            .tip(&edit.done_tip)
            .show(ui)
    });
    let done = done.inner;
    ctx.accesskit_node_builder(done.id, |node| node.set_role(Role::DefaultButton));
    if done.clicked() {
        action = Some(ToolbarAction::Edit(ToolbarEdit::Done));
    }
    let mut end = done_rect.left() - t.spacing.m;
    // A newer build's file: the save state's warning before Done.
    if edit.newer {
        let sign = Icon::Warning.size();
        let at = Rect::from_center_size(pos2(end - sign / 2.0, bar.center().y), Vec2::splat(sign));
        Icon::Warning.paint(ui.painter(), at, t.colors.warning);
        end = at.left() - t.spacing.s;
    }
    // » as a face where the panel at rest has it, before Done.
    let more_rect = more.then(|| {
        let rect = Rect::from_center_size(
            pos2(end - BUTTON / 2.0, bar.center().y),
            Vec2::splat(BUTTON),
        );
        let icon = Icon::ToolbarMore;
        let square = Rect::from_center_size(rect.center(), Vec2::splat(icon.size()));
        icon.paint(ui.painter(), square, t.colors.text_secondary);
        end = rect.left() - t.spacing.xxs;
        rect
    });

    // The menu button, at the start, as at rest, with no plate: no entry, taking no click.
    let mut start = inner.min.x;
    let menu_rect = toolbar.menu.then(|| {
        let width = toolbar.menu_style.width();
        let rect = Rect::from_min_size(
            pos2(start, bar.center().y - BUTTON / 2.0),
            vec2(width, BUTTON),
        );
        paint_menu_button(ui, toolbar.menu_style, rect, &t);
        // A group's gap after it, as at rest.
        start = rect.right() + t.spacing.xxs + t.spacing.s;
        rect
    });
    let line = Rect::from_x_y_ranges(start..=end.max(start), bar.y_range());

    // Each entry's width at rest, nothing folded, a fixed space always drawn, and a flexible
    // space's share of what is left, at least 40 points.
    let well_width = well.as_ref().map_or(BUTTON, |well| well.width(ui));
    let drawn: Vec<Drawn> = (toolbar.entries.iter())
        .map(|entry| match entry {
            ToolbarEntry::Button(button) => Drawn::Button(button),
            ToolbarEntry::Space => Drawn::Space,
            ToolbarEntry::FlexibleSpace => Drawn::Flexible,
            ToolbarEntry::Cursors => Drawn::Cursors,
            ToolbarEntry::Jobs => Drawn::Jobs,
            ToolbarEntry::SaveState => Drawn::Save,
        })
        .collect();
    let width = |d: Drawn| match d {
        Drawn::Button(button) if labelled => Labelled::of(ui, button).width,
        Drawn::Button(_) => ACTION,
        Drawn::Space => t.spacing.xl,
        Drawn::Cursors => well_width,
        Drawn::Save => saving.width(false),
        Drawn::Jobs | Drawn::More | Drawn::Menu => BUTTON,
        Drawn::Flexible => 0.0,
    };
    let gaps: f32 = drawn.windows(2).map(|w| between(&t, w[0], w[1])).sum();
    let fixed: f32 = drawn.iter().map(|d| width(*d)).sum::<f32>() + gaps;
    let flexible = drawn.iter().filter(|d| **d == Drawn::Flexible).count();
    let share = if flexible > 0 {
        ((line.width() - fixed) / flexible as f32).max(FLEXIBLE)
    } else {
        0.0
    };
    let mut slots: Vec<Slot> = Vec::with_capacity(n);
    let mut x = 0.0;
    for (at, d) in drawn.iter().enumerate() {
        if at > 0 {
            x += between(&t, drawn[at - 1], *d);
        }
        let w = if *d == Drawn::Flexible {
            share
        } else {
            width(*d)
        };
        slots.push(Slot {
            drawn: *d,
            x,
            width: w,
        });
        x += w;
    }
    let content = x;
    let overflow = (content - line.width()).max(0.0);

    // The line: one focus stop, the Toolbar as a list box to screen readers.
    let line_id = id.with("line");
    let line_response = ui.interact(line, line_id, Sense::FOCUSABLE);
    let list_name = if n == 0 {
        tr(&ctx, "toolbar-empty")
    } else {
        tr(&ctx, "toolbar")
    };
    line_response.widget_info(|| WidgetInfo::labeled(Role::ListBox, true, &list_name));
    ctx.accesskit_node_builder(line_id, |node| {
        node.set_orientation(Orientation::Horizontal);
        node.set_size_of_set(n);
    });
    let focused = line_response.has_focus();
    if focused {
        // Tab, the arrows and Esc are the line's own: Tab goes to Done alone.
        ctx.memory_mut(|m| {
            m.set_focus_lock_filter(
                line_id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            );
        });
    }
    if done.has_focus() {
        ctx.memory_mut(|m| {
            m.set_focus_lock_filter(
                done.id,
                egui::EventFilter {
                    tab: true,
                    escape: true,
                    ..Default::default()
                },
            );
        });
    }
    // Tab moves between the line and Done, and nowhere else.
    if (focused || done.has_focus())
        && ctx.input_mut(|i| {
            i.consume_key(Modifiers::NONE, Key::Tab) || i.consume_key(Modifiers::SHIFT, Key::Tab)
        })
    {
        let to = if focused { done.id } else { line_id };
        ctx.memory_mut(|m| m.request_focus(to));
    }
    // The line takes the focus as the mode starts, and, kept, whenever nothing has it.
    let nothing = ctx.memory(|m| m.focused()).is_none() && !crate::popup::any_open(&ctx);
    if edit.keep_focus && (!kept.shown || nothing) {
        ctx.memory_mut(|m| m.request_focus(line_id));
    }

    // Wheel, a sideways swipe and Shift with the wheel scroll the line.
    if overflow > 0.0 && ui.rect_contains_pointer(bar) {
        let delta = ctx.input(|i| i.smooth_scroll_delta);
        let step = if delta.x != 0.0 { delta.x } else { delta.y };
        kept.scroll -= step;
    }
    // The selected entry kept in view as it changes.
    if edit.selected != kept.selected
        && let Some(slot) = edit.selected.and_then(|at| slots.get(at))
    {
        if slot.x < kept.scroll {
            kept.scroll = slot.x;
        } else if slot.x + slot.width > kept.scroll + line.width() {
            kept.scroll = slot.x + slot.width - line.width();
        }
    }
    kept.selected = edit.selected;
    // A drag near a scrolled line's end scrolls it.
    let pointer = ctx.input(|i| i.pointer.interact_pos());
    // Only while it is held, so a release lands where it was let go.
    if kept.lifted.is_some()
        && overflow > 0.0
        && ctx.input(|i| i.pointer.primary_down())
        && let Some(at) = pointer
    {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        if at.x < line.left() + EDGE {
            kept.scroll -= EDGE_SPEED * dt;
            ctx.request_repaint();
        } else if at.x > line.right() - EDGE {
            kept.scroll += EDGE_SPEED * dt;
            ctx.request_repaint();
        }
    }
    kept.scroll = kept.scroll.clamp(0.0, overflow);
    let on_screen = |x: f32| line.left() + x - kept.scroll;

    // The wiggle: once, from the frame the mode starts, neighbours in opposite phase, two
    // cycles easing out; none with reduced motion or the motion tokens at zero.
    let length = f64::from(t.motion.wiggle_ms) / 1000.0;
    let wiggling = !edit.still && t.motion.open_ms > 0.0 && length > 0.0 && edit.since < length;
    let wiggle = |at: usize| {
        if !wiggling {
            return 0.0;
        }
        let p = (edit.since / length) as f32;
        let ease = (1.0 - p) * (1.0 - p);
        let phase = if at.is_multiple_of(2) { 1.0 } else { -1.0 };
        phase * WIGGLE * ease * (std::f32::consts::TAU * CYCLES * p).sin()
    };

    // The entries, on the line's own child so screen readers find them in the list box.
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("customising entries")
            .max_rect(line)
            .accessibility_parent(line_id),
    );
    child.set_clip_rect(line.intersect(ui.clip_rect()));
    let painter = child.painter().clone();
    let plate_height = if labelled { PLATE_LABELLED } else { PLATE };
    let plate_of = |slot: &Slot, dx: f32| {
        let left = on_screen(slot.x) + dx;
        let rect = Rect::from_center_size(
            pos2(left + slot.width / 2.0, bar.center().y),
            vec2(slot.width, plate_height),
        );
        match slot.drawn {
            Drawn::Space => rect,
            _ => rect.shrink2(vec2(INSET, 0.0)),
        }
    };
    let dragging = kept.lifted.map(|(at, _)| at);
    let mut items: Vec<(usize, Response)> = Vec::with_capacity(n);
    // Each entry's custom actions, Move Left, Move Right and Remove from Toolbar.
    let customs = [
        (MOVE_LEFT, "action-toolbar-move-left"),
        (MOVE_RIGHT, "action-toolbar-move-right"),
        (REMOVE, "action-toolbar-remove"),
    ]
    .map(|(custom, message)| (custom, tr(&ctx, message)));
    for (at, slot) in slots.iter().enumerate() {
        let dx = if dragging.is_some() { 0.0 } else { wiggle(at) };
        let plate = plate_of(slot, dx);
        let item = child.interact(
            plate.intersect(line),
            id.with(("entry", at)),
            Sense::CLICK | Sense::DRAG,
        );
        let name = edit.names.get(at).cloned().unwrap_or_default();
        let selected = edit.selected == Some(at);
        item.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, selected, &name));
        let marked = folded.get(at).copied().unwrap_or(false);
        ctx.accesskit_node_builder(item.id, |node| {
            // Zero-based, the set's size on the list box (AccessKit's own rule).
            node.set_position_in_set(at);
            if selected {
                node.set_selected(true);
            } else {
                node.set_selected(false);
            }
            if marked {
                node.set_description(edit.marked.clone());
            }
            node.add_action(Action::CustomAction);
            for (custom, description) in &customs {
                node.push_custom_action(CustomAction {
                    id: *custom,
                    description: description.clone(),
                });
            }
        });
        let lifted = dragging == Some(at);
        if child.is_rect_visible(plate) {
            let fill = if lifted {
                t.colors.raised
            } else if selected {
                t.colors.accent_subtle
            } else if item.hovered() && dragging.is_none() {
                t.colors.button
            } else {
                t.colors.raised
            };
            let tile = matches!(slot.drawn, Drawn::Space | Drawn::Flexible);
            let fill = if tile && !selected {
                t.colors.sunken
            } else {
                fill
            };
            painter.rect_filled(plate, RADIUS, fill);
            let ink = if lifted {
                t.colors.text_disabled
            } else {
                t.colors.text_secondary
            };
            let face = Faces {
                ui: &child,
                painter: &painter,
                t: &t,
                bar,
                labelled,
                ink,
                saved: toolbar.saved,
                failed: toolbar.jobs.as_ref().is_some_and(|jobs| jobs.failed()),
            };
            match slot.drawn {
                Drawn::Cursors => {
                    // As at rest, but taking nothing: the well is an entry like any other.
                    if let Some(well) = well.take() {
                        let rect = Rect::from_center_size(
                            pos2(plate.center().x, bar.center().y),
                            vec2(well_width, t.sizes.cursor_well),
                        );
                        let mut faces = child.new_child(
                            UiBuilder::new()
                                .id_salt("customising cursors")
                                .max_rect(rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        );
                        faces.disable();
                        well.show(&mut faces);
                    }
                }
                drawn => face.paint(drawn, plate),
            }
            if marked {
                let at = pos2(plate.right() - t.spacing.xxs, plate.top() + t.spacing.xxs);
                painter.text(
                    at,
                    Align2::RIGHT_TOP,
                    "»",
                    FontId::proportional(MARK),
                    t.colors.text_secondary,
                );
            }
            if selected && focused {
                focus_ring(&painter, plate, &t);
            }
        }
        items.push((at, item));
    }

    // What the pointer did to the entries.
    let gap_at = |x: f32| {
        let at = (slots.iter()).position(|slot| x < on_screen(slot.x + slot.width / 2.0));
        at.unwrap_or(n)
    };
    let window_focused = ctx.input(|i| i.focused);
    let escape_now = ctx.input(|i| i.key_pressed(Key::Escape));
    let mut escaped = false;
    for (at, item) in &items {
        if dragging.is_none() && !item.dragged() {
            if item.hovered() {
                ctx.set_cursor_icon(CursorIcon::Grab);
            }
            let tip = edit.tips.get(*at).cloned().unwrap_or_default();
            let tip = if folded.get(*at).copied().unwrap_or(false) {
                format!("{tip}\n{}", edit.marked)
            } else {
                tip
            };
            if !tip.is_empty() {
                item.clone().on_hover_text(tip);
            }
        }
        if item.clicked() {
            action = Some(ToolbarAction::Edit(ToolbarEdit::Select(*at)));
            ctx.memory_mut(|m| m.request_focus(line_id));
        }
        // Lifted once the press has moved 6 points, never by egui's held-press timer alone.
        let moved = ctx.input(|i| {
            let origin = i.pointer.press_origin()?;
            Some(i.pointer.interact_pos()?.distance(origin))
        });
        let lifts = moved.is_some_and(|moved| moved >= LIFT);
        // The primary button's alone: a secondary press is the menu's.
        let primary = ctx.input(|i| i.pointer.primary_down());
        if item.dragged() && kept.lifted.is_none() && lifts && primary && !kept.cancelled {
            let origin = ctx.input(|i| i.pointer.press_origin());
            let plate = plate_of(&slots[*at], 0.0);
            let grab = origin.map_or(Vec2::ZERO, |p| p - plate.min);
            kept.lifted = Some((*at, grab));
            ctx.memory_mut(|m| m.request_focus(line_id));
        }
        if item.drag_stopped() {
            let lifted = kept.lifted.take();
            // egui lets go of a drag on Esc: the drag cancelled, not the mode.
            escaped |= lifted.is_some() && escape_now;
            // Nor a release as the window loses the focus, which cancels the drag.
            let was = std::mem::take(&mut kept.cancelled) || escape_now || !window_focused;
            if let (Some((from, _)), false, Some(p)) = (lifted, was, pointer)
                && bar.contains(p)
            {
                let gap = gap_at(p.x);
                let to = if gap > from { gap - 1 } else { gap };
                let edit = if to == from {
                    ToolbarEdit::Select(from)
                } else {
                    ToolbarEdit::Move { from, to }
                };
                action = Some(ToolbarAction::Edit(edit));
            }
        }
        // A screen reader's custom actions, as the keys.
        let mut custom = None;
        ctx.input_mut(|i| {
            i.consume_accesskit_action_requests(item.id, |request| {
                let asked = request.action == Action::CustomAction;
                if let (true, Some(ActionData::CustomAction(id))) = (asked, &request.data) {
                    custom = Some(*id);
                }
                asked
            });
        });
        // None during a drag, as the keys, so the lifted entry keeps its place.
        let edited = match custom.filter(|_| kept.lifted.is_none()) {
            Some(MOVE_LEFT) if *at > 0 => Some(ToolbarEdit::Move {
                from: *at,
                to: at - 1,
            }),
            Some(MOVE_RIGHT) if at + 1 < n => Some(ToolbarEdit::Move {
                from: *at,
                to: at + 1,
            }),
            Some(REMOVE) => Some(ToolbarEdit::Remove(*at)),
            _ => None,
        };
        if let Some(edited) = edited {
            action = Some(ToolbarAction::Edit(edited));
        }
        if menu_asked_of(&ctx, item.id, false) {
            action = Some(ToolbarAction::Edit(ToolbarEdit::Menu(EditMenu {
                at: ToolbarSpot::Entry(*at),
                pos: item.rect.left_bottom(),
                opener: Some(line_id),
            })));
        }
    }
    // A drag cancelled by the window losing the focus, its release then doing nothing.
    if kept.lifted.is_some() && !window_focused {
        kept.lifted = None;
        kept.cancelled = true;
    }
    if !ctx.input(|i| i.pointer.any_down()) {
        kept.cancelled = false;
    }

    // The drag under way: the lifted entry follows the pointer, the insertion line marks the
    // nearest gap, and a line says what a release does.
    if let (Some((from, grab)), Some(p)) = (kept.lifted, pointer) {
        let on_bar = bar.contains(p);
        ctx.set_cursor_icon(if on_bar {
            CursorIcon::Grabbing
        } else {
            CursorIcon::NoDrop
        });
        let slot = &slots[from];
        let plate = plate_of(slot, 0.0);
        let lifted = Rect::from_min_size(p - grab, plate.size());
        let layer = LayerId::new(Order::Tooltip, id.with("lifted"));
        let top = ctx.layer_painter(layer);
        top.add(ui.style().visuals.popup_shadow.as_shape(lifted, RADIUS));
        top.rect_filled(lifted, RADIUS, t.colors.raised);
        let face = Faces {
            ui,
            painter: &top,
            t: &t,
            bar: lifted.translate(vec2(0.0, bar.center().y - lifted.center().y)),
            labelled,
            ink: t.colors.text_secondary,
            saved: toolbar.saved,
            failed: false,
        };
        let shifted = |rect: Rect| rect.translate(vec2(0.0, lifted.center().y - bar.center().y));
        if slot.drawn == Drawn::Cursors {
            let rect =
                Rect::from_center_size(lifted.center(), vec2(well_width, t.sizes.cursor_well));
            top.rect_filled(rect, t.shape.radius_large, t.colors.sunken);
        } else {
            let painted = Faces {
                bar: shifted(bar),
                ..face
            };
            painted.paint(slot.drawn, lifted);
        }
        if on_bar {
            let gap = gap_at(p.x);
            let x = match gap {
                0 => on_screen(0.0),
                gap if gap >= n => on_screen(content),
                gap => {
                    let before = &slots[gap - 1];
                    (on_screen(before.x + before.width) + on_screen(slots[gap].x)) / 2.0
                }
            };
            let height = inner.height() - 12.0;
            let insertion = Rect::from_center_size(pos2(x, bar.center().y), vec2(2.0, height));
            top.rect_filled(insertion, 1.0, t.colors.accent);
            // Under the lifted entry, on a raised pill of its own so it reads over the bar.
            if let Some(words) = edit.release.get(gap) {
                let galley = top.layout_no_wrap(
                    words.clone(),
                    FontId::proportional(RELEASE),
                    t.colors.text_secondary,
                );
                let pill = Rect::from_min_size(
                    pos2(lifted.left(), lifted.bottom() + t.spacing.xs),
                    galley.size() + vec2(2.0 * t.spacing.s, t.spacing.xs),
                );
                top.add(ui.style().visuals.popup_shadow.as_shape(pill, RADIUS));
                top.rect_filled(pill, RADIUS, t.colors.raised);
                let at = pill.min + vec2(t.spacing.s, t.spacing.xs / 2.0);
                top.galley(at, galley, t.colors.text_secondary);
            }
        }
        ctx.request_repaint();
    }

    // The scroll bar under the line, and a fade at an end with more.
    if overflow > 0.0 {
        let y = bar.bottom() - t.shape.separator_width - SCROLL_BAR;
        let track = Rect::from_x_y_ranges(line.x_range(), y..=y + SCROLL_BAR);
        let thumb_width = (line.width() * line.width() / content).max(SCROLL_BAR * 4.0);
        let thumb_x = line.left() + (line.width() - thumb_width) * kept.scroll / overflow;
        let thumb = Rect::from_x_y_ranges(thumb_x..=thumb_x + thumb_width, track.y_range());
        ui.painter()
            .rect_filled(thumb, SCROLL_BAR / 2.0, t.colors.scroll_thumb);
        if kept.scroll > 0.0 {
            fade(ui, line, true, t.colors.surface);
        }
        if kept.scroll < overflow {
            fade(ui, line, false, t.colors.surface);
        }
    }

    // The keys, on the focused line: the selection, moving and removing; none during a drag,
    // so no entry takes over the one lifted.
    if focused && kept.lifted.is_none() {
        let key = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_key(m, k));
        let last = n.checked_sub(1);
        let edited = if let Some(at) = edit.selected.filter(|at| *at < n) {
            if key(Modifiers::ALT, Key::ArrowLeft) {
                (at > 0).then(|| ToolbarEdit::Move {
                    from: at,
                    to: at - 1,
                })
            } else if key(Modifiers::ALT, Key::ArrowRight) {
                (at + 1 < n).then(|| ToolbarEdit::Move {
                    from: at,
                    to: at + 1,
                })
            } else if key(Modifiers::NONE, Key::ArrowLeft) {
                Some(ToolbarEdit::Select(at.saturating_sub(1)))
            } else if key(Modifiers::NONE, Key::ArrowRight) {
                Some(ToolbarEdit::Select((at + 1).min(n - 1)))
            } else if key(Modifiers::NONE, Key::Delete) || key(Modifiers::NONE, Key::Backspace) {
                Some(ToolbarEdit::Remove(at))
            } else {
                None
            }
        } else if key(Modifiers::NONE, Key::ArrowRight) {
            (n > 0).then_some(ToolbarEdit::Select(0))
        } else if key(Modifiers::NONE, Key::ArrowLeft) {
            last.map(ToolbarEdit::Select)
        } else {
            None
        };
        let edited = edited.or_else(|| {
            if key(Modifiers::NONE, Key::Home) {
                (n > 0).then_some(ToolbarEdit::Select(0))
            } else if key(Modifiers::NONE, Key::End) {
                last.map(ToolbarEdit::Select)
            } else {
                None
            }
        });
        if let Some(edited) = edited {
            action = Some(ToolbarAction::Edit(edited));
        }
    }
    // The menu from the keys or a screen reader, on the line: for the selected entry, or the
    // list's start.
    if menu_asked_of(&ctx, line_id, focused) {
        let at = edit.selected.filter(|at| *at < n);
        let pos = match at {
            Some(at) => plate_of(&slots[at], 0.0).left_bottom(),
            None => line.left_bottom(),
        };
        action = Some(ToolbarAction::Edit(ToolbarEdit::Menu(EditMenu {
            at: at.map_or(ToolbarSpot::Gap(0), ToolbarSpot::Entry),
            pos,
            opener: Some(line_id),
        })));
    }
    // Esc cancels a drag, else the mode; Enter is Done. A menu open takes its own keys.
    if !crate::popup::any_open(&ctx) {
        let escape = ctx.input_mut(|i| {
            i.consume_key(Modifiers::NONE, Key::Escape)
                || (mac && i.consume_key(Modifiers::MAC_CMD, Key::Period))
        });
        if escape && (kept.lifted.is_some() || escaped) {
            kept.lifted = None;
            kept.cancelled = true;
        } else if escape {
            action = Some(ToolbarAction::Edit(ToolbarEdit::Cancel));
        } else if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            action = Some(ToolbarAction::Edit(ToolbarEdit::Done));
        }
    }

    // A right-click, or a Control-click on the Mac, on an entry selects it and opens the
    // menu; on the empty line, », or the menu button, the menu at the gap under the pointer;
    // never on the caption buttons or Done.
    if let Some((at, _)) = super::menu_click(ui, bar) {
        let caption = captions.iter().any(|r| r.contains(at));
        if !caption && !done.rect.contains(at) {
            let on = items.iter().find(|(_, item)| item.rect.contains(at));
            let on_menu = menu_rect.is_some_and(|r| r.contains(at)) || at.x < line.left();
            let spot = match on {
                Some((entry, _)) => ToolbarSpot::Entry(*entry),
                None if on_menu => ToolbarSpot::Gap(0),
                None if more_rect.is_some_and(|r| r.contains(at)) => ToolbarSpot::Gap(n),
                None => ToolbarSpot::Gap(gap_at(at.x)),
            };
            action = Some(ToolbarAction::Edit(ToolbarEdit::Menu(EditMenu {
                at: spot,
                pos: at,
                opener: None,
            })));
        }
    }

    kept.shown = true;
    ctx.data_mut(|d| d.insert_temp(id, kept));
    // Gone from the next frame that draws the Toolbar at rest.
    let rest = response.id.with("customising rest");
    ctx.data_mut(|d| d.insert_temp(rest, true));
    ui.advance_cursor_after_rect(bar);
    action
}

/// Forgets what the edit state kept, as the Toolbar is drawn at rest again, so the next
/// mode starts afresh: unscrolled, the focus taken as it starts.
pub(super) fn rest(ctx: &egui::Context, toolbar: Id) {
    let rest = toolbar.with("customising rest");
    if ctx.data_mut(|d| d.remove_temp::<bool>(rest)).is_some() {
        ctx.data_mut(|d| d.remove::<Kept>(toolbar.with("customising")));
    }
}

/// How each entry's face is drawn on its plate: enabled and at rest, whatever it is now.
#[derive(Clone, Copy)]
struct Faces<'f> {
    ui: &'f Ui,
    painter: &'f egui::Painter,
    t: &'f crate::Theme,
    bar: Rect,
    labelled: bool,
    ink: Color32,
    saved: SaveState<'f>,
    failed: bool,
}

impl Faces<'_> {
    fn paint(&self, drawn: Drawn, plate: Rect) {
        let t = self.t;
        let painter = self.painter;
        match drawn {
            Drawn::Button(button) => {
                let icon = button.icon;
                let size = action_icon(icon);
                if self.labelled {
                    let square = Rect::from_min_size(
                        pos2(
                            plate.center().x - ACTION_ICON / 2.0,
                            self.bar.top() + LABEL_TOP,
                        ),
                        Vec2::splat(ACTION_ICON),
                    );
                    let rect = Rect::from_center_size(square.center(), Vec2::splat(size));
                    icon.paint_stroked(painter, rect, self.ink, ACTION_STROKE);
                    let word = Labelled::of(self.ui, button).word;
                    let at = pos2(
                        plate.center().x - word.size().x / 2.0,
                        square.bottom() + LABEL_GAP,
                    );
                    let text = if self.ink == t.colors.text_disabled {
                        t.colors.text_disabled
                    } else {
                        t.colors.text
                    };
                    painter.galley(at, word, text);
                } else {
                    let rect = Rect::from_center_size(plate.center(), Vec2::splat(size));
                    icon.paint_stroked(painter, rect, self.ink, ACTION_STROKE);
                }
            }
            Drawn::Flexible => {
                let icon = Icon::MoveHorizontal;
                let rect = Rect::from_center_size(plate.center(), Vec2::splat(icon.size()));
                icon.paint(painter, rect, t.colors.text_disabled);
            }
            Drawn::Space => {
                let bar = Rect::from_center_size(plate.center(), vec2(2.0, plate.height() - 16.0));
                painter.rect_filled(bar, 1.0, t.colors.text_disabled);
            }
            Drawn::Jobs => {
                let icon = Icon::Process;
                let rect = Rect::from_center_size(plate.center(), Vec2::splat(icon.size()));
                let ink = if self.failed {
                    t.colors.warning
                } else {
                    self.ink
                };
                icon.paint(painter, rect, ink);
            }
            Drawn::Save => {
                let font = FontId::proportional(t.text.small);
                if let SaveState::Failed(_) = self.saved {
                    let sign = Icon::Warning.size();
                    let word = tr(self.ui.ctx(), "status-not-saved");
                    let galley = painter.layout_no_wrap(word, font, t.colors.warning);
                    let width = sign + t.spacing.xs + galley.size().x;
                    let left = plate.center().x - width / 2.0;
                    let icon = Rect::from_min_size(
                        pos2(left, plate.center().y - sign / 2.0),
                        Vec2::splat(sign),
                    );
                    Icon::Warning.paint(painter, icon, t.colors.warning);
                    let at = pos2(
                        icon.right() + t.spacing.xs,
                        plate.center().y - galley.size().y / 2.0,
                    );
                    painter.galley(at, galley, t.colors.warning);
                } else {
                    painter.text(
                        plate.center(),
                        Align2::CENTER_CENTER,
                        tr(self.ui.ctx(), "status-saving"),
                        font,
                        self.ink,
                    );
                }
            }
            Drawn::Cursors | Drawn::Menu | Drawn::More => {}
        }
    }
}

/// The menu button in `style` at `rect`, as at rest, taking nothing.
fn paint_menu_button(ui: &Ui, style: super::MenuButtonStyle, rect: Rect, t: &crate::Theme) {
    let painter = ui.painter();
    let ink = t.colors.text_secondary;
    match style {
        super::MenuButtonStyle::Menu => {
            let icon = Icon::Menu;
            let square = Rect::from_center_size(rect.center(), Vec2::splat(icon.size()));
            icon.paint(painter, square, ink);
        }
        super::MenuButtonStyle::Icon => {
            let mut x = rect.left() + super::ICON_PADDING;
            for (icon, gap) in [(Icon::AppIcon, super::CARET_GAP), (Icon::Caret, 0.0)] {
                let size = icon.size();
                let at = pos2(x, rect.center().y - size / 2.0);
                icon.paint(painter, Rect::from_min_size(at, Vec2::splat(size)), ink);
                x += size + gap;
            }
        }
    }
}

/// A fade from `colour` to clear over the line's start, or its end, where more lies beyond.
fn fade(ui: &Ui, line: Rect, start: bool, colour: Color32) {
    let mut mesh = egui::Mesh::default();
    let clear = Color32::TRANSPARENT;
    let (outer, inner) = if start {
        (line.left(), line.left() + FADE)
    } else {
        (line.right(), line.right() - FADE)
    };
    let (top, bottom) = (line.top(), line.bottom() - 1.0);
    let base = mesh.vertices.len() as u32;
    for (x, c) in [(outer, colour), (inner, clear)] {
        mesh.colored_vertex(pos2(x, top), c);
        mesh.colored_vertex(pos2(x, bottom), c);
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base + 1, base + 2, base + 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}
