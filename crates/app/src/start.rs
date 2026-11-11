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

//! The start window (`docs/gui.md`, The start window; the wireframe's Start window frame):
//! what the main window shows, at the start window's own size, while nothing is open, so the
//! main window never shows empty. The app decides when (`main.rs`, `shows_start`) and gives
//! the window its size; this draws what it holds and says what the person asked for.
//!
//! - A strip at the top, where the Mac's window buttons sit and, on Windows and Linux, the
//!   close button the app draws; a drag on it moves the window.
//! - Two columns. The first third, on `sunken`, a `divider` rule at its end, holds
//!   what a splash screen would: the full logo, the version cut at its `~`, the copyright
//!   and what the app is; then room for a start-up status, empty for now; and at its foot
//!   the ways in, New Library…, New Project…, Open Folder… and Browse…, a quiet list of
//!   icon and words, the library's two and Browse… disabled until Phase 5.
//! - The other two thirds: the recents, newest first, each with its kind's icon, its name
//!   and its path, the chosen one on `accent` with its icon and words in `text_on_accent`.
//!   One found gone, or whose opening failed, is marked so, with Locate… and Remove from
//!   the List. To screen readers a list box of options, each named by its name and
//!   described by its kind, its path and why it cannot be opened, where it cannot. Under
//!   them at the end Open, the accent button, level with the last of the ways in.
//! - The keys: ↑, ↓, Home and End choose in the list and Enter opens the recent chosen
//!   while the list has the focus, as its chosen row does; Escape closes the window, as its
//!   close button does. Tab follows the reading order, the first column then the second:
//!   from the list it goes to Open, and Shift+Tab from the list to the ways in.

use std::path::{Path, PathBuf};

use apertrust_ui::components::{Button, caption_button, focus_ring_centred, middle_cut};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Icon, fonts, theme, tr, translator};
use eframe::egui::{
    self, Align, Key, Layout, Modifiers, Rect, Sense, UiBuilder, accesskit::Role, pos2, vec2,
};

use crate::export;
use crate::library::{Missing, Recent};

/// The start window's size, in points: the wireframe's.
pub const SIZE: egui::Vec2 = vec2(820.0, 520.0);

/// The strip at its top, in points, where the window buttons are, the Mac's centred in it.
pub const STRIP: f32 = 40.0;

/// A recent's row, and the column its icon is centred in, in points.
const ROW: f32 = 52.0;
const ICON_COLUMN: f32 = 44.0;
const ICON: f32 = 28.0;

/// The logo's width in the first column, in points, where the column has room.
const LOGO_WIDTH: f32 = 220.0;

/// What the person asked of the start window.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// The recent at this path, opened: Open, Enter or a double-click.
    Open(PathBuf),
    /// Open Folder…'s dialog.
    OpenFolder,
    /// Locate…, for the missing recent at this path.
    Locate(PathBuf),
    /// Remove from the List, for the missing recent at this path.
    Remove(PathBuf),
    /// The window closed: its close button or Escape.
    Close,
    /// A drag on the strip, which moves the window.
    Move,
    /// A right-click on the strip, which opens the window's menu.
    Menu,
}

/// The list as it stands from frame to frame: the recent chosen, by place, and whether its
/// row is to take the focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct List {
    pub chosen: usize,
    focus: bool,
}

impl List {
    /// As the window shows: the first recent chosen, its row taking the focus.
    pub fn shown() -> List {
        List {
            chosen: 0,
            focus: true,
        }
    }
}

impl Asked {
    /// What it asks as Browse would ask it, which the app carries out as Browse's: a recent
    /// opened as a way in, Open Folder…'s dialog, Locate… and Remove from the List; None for
    /// what is the window's own.
    pub fn library(&self) -> Option<crate::library::Action> {
        use crate::library::Action;
        match self {
            Asked::Open(path) => Some(Action::Open(path.clone())),
            Asked::OpenFolder => Some(Action::OpenFolder),
            Asked::Locate(path) => Some(Action::Locate(path.clone())),
            Asked::Remove(path) => Some(Action::Remove(path.clone())),
            Asked::Close | Asked::Move | Asked::Menu => None,
        }
    }
}

/// The main window as the start window, as the app keeps it from frame to frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    /// Whether the main window shows as the start window.
    pub shown: bool,
    /// Whether it was closed on the Mac, the app running on with no window shown until the
    /// Dock icon is clicked or something opens.
    pub hidden: bool,
    pub list: List,
    /// Whether its close was asked, by its close button, Escape or Close Window.
    pub close: bool,
    /// Whether the Mac's close button on the main window asked to close what is open, which
    /// shows the start window in its place.
    pub close_main: bool,
    /// Whether the window is still to take the start window's size and place, once it is
    /// neither full screen nor maximised.
    pub reshape: bool,
    /// The interface's scale the window was last sized at.
    pub zoom: f32,
    /// The start window's size, in the window system's points, while the main window, given
    /// its own size back, may not have it yet, so its size is not kept meanwhile.
    pub settling: Option<egui::Vec2>,
}

impl Window {
    /// As the app starts, `shown` or not, at the interface's scale `zoom`.
    pub fn new(shown: bool, zoom: f32) -> Window {
        Window {
            shown,
            hidden: false,
            list: List::shown(),
            close: false,
            close_main: false,
            reshape: false,
            zoom,
            settling: None,
        }
    }

    /// Whether the main window's size and place are its own to keep now, as `size`, its
    /// size in the window system's points, says: not while it is the start window, nor
    /// while it still has the start window's size after it.
    pub fn keeps_placement(&mut self, size: egui::Vec2) -> bool {
        if self.shown {
            return false;
        }
        match self.settling {
            Some(start) if (start - size).abs().max_elem() < 1.0 => false,
            _ => {
                self.settling = None;
                true
            }
        }
    }
}

/// Whether `recent` can be opened: any but one found gone; one whose opening failed this run
/// tries again, as Open Recent's does.
fn opens(recent: &Recent<'_>) -> bool {
    recent.missing != Some(Missing::Gone)
}

/// A recent's row's id, by place.
fn row_id(place: usize) -> egui::Id {
    egui::Id::unique("start recent").with(place)
}

/// What [`show`] draws the start window with, beside its recents.
#[derive(Clone, Copy, Debug)]
pub struct Shown<'a> {
    /// The person's home folder, from which paths under it are written as `~`.
    pub home: Option<&'a Path>,
    /// Whether the strip ends in the close button, where the app draws the window's frame.
    pub close: bool,
    /// Whether its keys are its own: not while a modal dialog or alert, About among them,
    /// is over it, whose Escape is its own.
    pub keys: bool,
    /// The build's version with its commit, `0.1.0+197~g9201df0`, which the window shows
    /// cut at its `~`.
    pub version: &'a str,
}

/// The version as the start window shows it, cut at its `~` as darktable's splash cuts
/// it: `0.1.0+197` of `0.1.0+197~g9201df0`.
fn short_version(version: &str) -> &str {
    version.split('~').next().unwrap_or(version)
}

/// Draws the start window over the whole of `ui`'s window: `recents`, newest first, `list`
/// the one chosen, as `shown` says. Returns what the person asked for, in order.
pub fn show(
    ui: &mut egui::Ui,
    recents: &[Recent<'_>],
    list: &mut List,
    shown: Shown<'_>,
) -> Vec<Asked> {
    let Shown {
        home,
        close,
        keys: own_keys,
        version,
    } = shown;
    let ctx = ui.ctx().clone();
    let t = theme(&ctx);
    let mut asked = if own_keys {
        keys(&ctx, recents, list)
    } else {
        Vec::new()
    };
    let whole = ctx.content_rect();
    ui.painter().rect_filled(whole, 0.0, t.colors.panel);

    // The two columns: the first third on `sunken`, a `divider` rule at its end, as the
    // Settings dialog's pages column has.
    let split = (whole.left() + whole.width() / 3.0).round();
    let first = Rect::from_min_max(whole.min, pos2(split, whole.bottom()));
    let second = Rect::from_min_max(pos2(split, whole.top()), whole.max);
    ui.painter().rect_filled(first, 0.0, t.colors.sunken);
    let line = t.shape.separator_width;
    let edge = Rect::from_x_y_ranges(split - line..=split, first.y_range());
    ui.painter().rect_filled(edge, 0.0, t.colors.divider);
    // Open's place at the second column's foot, which the ways in end level with.
    let foot = Rect::from_min_max(
        pos2(
            second.left() + t.spacing.xl,
            whole.bottom() - t.spacing.xl - t.sizes.control,
        ),
        pos2(second.right() - t.spacing.xl, whole.bottom() - t.spacing.xl),
    );
    let inset = t.spacing.m + t.spacing.xxs;

    // The strip: a drag on it moves the window, a right-click opens its menu; not
    // focusable, so Tab never stops on it.
    let strip = Rect::from_min_size(whole.min, vec2(whole.width(), STRIP));
    let empty = ui.interact(
        strip,
        egui::Id::unique("start strip"),
        Sense::CLICK | Sense::DRAG,
    );
    if empty.drag_started_by(egui::PointerButton::Primary) {
        asked.push(Asked::Move);
    } else if empty.secondary_clicked() {
        asked.push(Asked::Menu);
    }

    // The first column's top, what a splash screen would show: the logo, the version, the
    // copyright and what the app is. The room under them is a start-up status's, to come.
    let words = translator(&ctx);
    let mut splash = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(first.left() + t.spacing.xxl, strip.bottom() + t.spacing.xxl),
                pos2(first.right() - t.spacing.xxl, first.bottom()),
            ))
            .layout(Layout::top_down(Align::Min)),
    );
    splash.style_mut().interaction.selectable_labels = false;
    splash.spacing_mut().item_spacing.y = 0.0;
    let width = LOGO_WIDTH.min(splash.available_width());
    crate::about::logo(&t).show(&mut splash, width, &words.text("app-name"));
    // Words a translation may make longer wrap within the column, after the translator's
    // own line break.
    let quiet = |text: &str, size: f32, wrap: egui::TextWrapMode| {
        let text = egui::RichText::new(text)
            .size(size)
            .color(t.colors.text_secondary);
        egui::Label::new(text).wrap_mode(wrap)
    };
    use egui::TextWrapMode::{Extend, Wrap};
    splash.add_space(t.spacing.m);
    splash.add(quiet(short_version(version), t.text.section_title, Extend));
    splash.add_space(t.spacing.xxs);
    splash.add(quiet(&words.text("start-copyright"), t.text.small, Wrap));
    splash.add_space(t.spacing.xl);
    splash.add(quiet(&words.text("start-tagline"), t.text.heading, Wrap));

    // The ways in, at the first column's foot, the last ending level with Open, their icons
    // under the logo's edge. Drawn in reading order, the first column before the second, so
    // Tab and screen readers meet them before the recents: the library's two and Browse…
    // come with Phase 5, and have no action yet.
    let later = words.text("section-tab-later");
    let ways = [
        (Icon::Library, "action-new-library", None),
        (Icon::Project, "action-new-project", None),
        (Icon::Folder, "action-open-folder", Some(Asked::OpenFolder)),
        (Icon::Search, "start-browse", None),
    ];
    let gap = t.spacing.xxs;
    let height = ways.len() as f32 * (t.sizes.row + gap) - gap;
    let mut start = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(first.left() + t.spacing.xxl - inset, foot.bottom() - height),
                pos2(first.right() - t.spacing.xxl + inset, foot.bottom()),
            ))
            .layout(Layout::top_down(Align::Min)),
    );
    start.spacing_mut().item_spacing.y = gap;
    for (icon, id, action) in ways {
        let tip = action.is_none().then_some(later.as_str());
        let response = way_in(&mut start, icon, &words.text(id), tip);
        if let Some(action) = action.filter(|_| response.clicked()) {
            asked.push(action);
        }
    }

    // The second column: the recents under their heading, and Open alone at the foot.
    let heading = tr(&ctx, "start-recents");
    let font = egui::FontId::new(
        t.text.section_title,
        egui::FontFamily::Name(fonts::MEDIUM.into()),
    );
    let galley = ui
        .painter()
        .layout_no_wrap(heading.clone(), font, t.colors.text_strong);
    let at = pos2(second.left() + t.spacing.xxl, strip.bottom() + t.spacing.xs);
    let heading_bottom = at.y + galley.size().y;
    ui.painter().galley(at, galley, t.colors.text_strong);

    let rows = Rect::from_min_max(
        pos2(second.left() + inset, heading_bottom + t.spacing.m),
        pos2(second.right() - inset, foot.top() - t.spacing.l),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rows)
            .id_salt("start recents")
            .layout(Layout::top_down(Align::Min))
            .accessibility_role(Role::ListBox)
            .accessibility_label(&heading),
        |ui| {
            if recents.is_empty() {
                let none = tr(ui.ctx(), "start-recents-none");
                let text = egui::RichText::new(none)
                    .size(t.text.body)
                    .color(t.colors.text_secondary);
                ui.put(rows, egui::Label::new(text));
                return;
            }
            let area = egui::ScrollArea::vertical()
                .id_salt("start recents list")
                .auto_shrink([false, false]);
            apertrust_ui::components::quiet_scroll(ui, area, |ui| {
                ui.spacing_mut().item_spacing.y = t.spacing.xxs;
                for (place, recent) in recents.iter().enumerate() {
                    asked.extend(row(ui, recent, place, list, home));
                }
            });
        },
    );

    let mut end = ui.new_child(
        UiBuilder::new()
            .max_rect(foot)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let chosen = recents.get(list.chosen).filter(|recent| opens(recent));
    let open = words.text("start-open");
    let pressed = Button::new(&open)
        .primary(true)
        .enabled(chosen.is_some())
        .show(&mut end)
        .clicked();
    if let Some(recent) = chosen.filter(|_| pressed) {
        asked.push(Asked::Open(recent.path.to_owned()));
    }

    // Last in the focus order, as the dialogs' close is.
    if close {
        let mut end = ui.new_child(
            UiBuilder::new()
                .max_rect(strip)
                .layout(Layout::right_to_left(Align::Center)),
        );
        let name = tr(&ctx, "window-close");
        if caption_button(&mut end, Icon::Close, &name, true, &t).clicked() {
            asked.push(Asked::Close);
        }
    }
    asked
}

/// One of the ways in, as wide as `ui`: its icon and its `words`, with no plate, on
/// `hover` under the pointer and in the focus ring after a key; until then its icon a step
/// quieter, in `text_secondary`, its words in `text`. Disabled where it comes `later`,
/// which its tooltip says.
fn way_in(ui: &mut egui::Ui, icon: Icon, words: &str, later: Option<&str>) -> egui::Response {
    let t = theme(ui.ctx());
    let enabled = later.is_none();
    let size = vec2(ui.available_width(), t.sizes.row);
    // A disabled one is allocated in a disabled scope, as the design system's buttons are:
    // it ignores input, takes no focus and keeps its tooltip.
    let (rect, response) = ui
        .add_enabled_ui(enabled, |ui| ui.allocate_exact_size(size, Sense::click()))
        .inner;
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let hovered = enabled && response.hovered();
        if hovered {
            painter.rect_filled(rect, t.shape.radius, t.colors.hover);
        }
        let (icon_ink, ink) = if !enabled {
            (t.colors.text_disabled, t.colors.text_disabled)
        } else if hovered || response.has_focus() {
            (t.colors.text, t.colors.text_strong)
        } else {
            (t.colors.text_secondary, t.colors.text)
        };
        let pad = t.spacing.m + t.spacing.xxs;
        let side = t.sizes.icon_large;
        let at = Rect::from_min_size(
            pos2(rect.left() + pad, rect.center().y - side / 2.0),
            vec2(side, side),
        );
        icon.paint(painter, at, icon_ink);
        let font = egui::FontId::proportional(t.text.body);
        let galley = painter.layout_no_wrap(words.to_owned(), font, ink);
        let y = rect.center().y - galley.size().y / 2.0;
        painter.galley(pos2(at.right() + t.spacing.m, y), galley, ink);
        if response.has_focus() {
            focus_ring_centred(painter, rect, &t);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(Role::Button, enabled, words));
    match later {
        Some(tip) => response.on_hover_text(tip).on_disabled_hover_text(tip),
        None => response,
    }
}

/// The list's keys, read before anything is drawn: while the list has the focus, its chosen
/// row's, or nothing has, ↑, ↓, Home and End choose, and Enter opens the recent chosen;
/// Escape closes the window wherever the focus is.
fn keys(ctx: &egui::Context, recents: &[Recent<'_>], list: &mut List) -> Vec<Asked> {
    let mut asked = Vec::new();
    let count = recents.len();
    list.chosen = list.chosen.min(count.saturating_sub(1));
    let focused = ctx.memory(|m| m.focused());
    let on_list = count > 0 && focused.is_none_or(|id| id == row_id(list.chosen));
    if on_list {
        let last = count - 1;
        let moved = ctx.input_mut(|i| {
            let mut pressed = |key| i.consume_key(Modifiers::NONE, key);
            if pressed(Key::ArrowDown) {
                Some((list.chosen + 1).min(last))
            } else if pressed(Key::ArrowUp) {
                Some(list.chosen.saturating_sub(1))
            } else if pressed(Key::Home) {
                Some(0)
            } else if pressed(Key::End) {
                Some(last)
            } else {
                None
            }
        });
        if let Some(chosen) = moved {
            list.chosen = chosen;
            list.focus = true;
        }
        let enter = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        if let Some(recent) = recents.get(list.chosen).filter(|r| enter && opens(r)) {
            asked.push(Asked::Open(recent.path.to_owned()));
        }
    }
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        asked.push(Asked::Close);
    }
    asked
}

/// One recent's row at `place`: its kind's icon, its name, and under it its path, after why
/// it cannot be opened where it cannot; at its end its kind, or Locate… and Remove from the
/// List for one that is missing. The chosen one is on `accent`, its icon and words in
/// `text_on_accent`, and it alone takes the focus, which keeps ↑ and ↓ for the list. A click
/// chooses it, a double-click opens it.
fn row(
    ui: &mut egui::Ui,
    recent: &Recent<'_>,
    place: usize,
    list: &mut List,
    home: Option<&Path>,
) -> Vec<Asked> {
    let ctx = ui.ctx().clone();
    let t = theme(&ctx);
    let words = translator(&ctx);
    let chosen = place == list.chosen;
    let id = row_id(place);
    let sense = if chosen { Sense::click() } else { Sense::CLICK };
    let width = ui.available_width();
    let rect = ui.allocate_space(vec2(width, ROW)).1;
    let response = ui.interact(rect, id, sense);
    let mut asked = Vec::new();
    if chosen && std::mem::take(&mut list.focus) {
        response.request_focus();
        response.scroll_to_me(None);
    }
    if response.has_focus() {
        // ↑ and ↓ choose in the list, never move the focus out of it.
        let keep = egui::EventFilter {
            vertical_arrows: true,
            ..Default::default()
        };
        ctx.memory_mut(|m| m.set_focus_lock_filter(id, keep));
    }
    if response.clicked() {
        list.chosen = place;
        list.focus = true;
    }
    if response.double_clicked() && opens(recent) {
        asked.push(Asked::Open(recent.path.to_owned()));
    }

    let name = recent.path.file_name().map_or_else(
        || recent.path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let path = export::shown(recent.path, home);
    let kind = recent.kind.name(&words);
    let gone = match recent.missing {
        Some(Missing::Gone) => Some(words.text("start-recent-gone")),
        Some(Missing::Failed(_)) => Some(words.text("start-recent-failed")),
        None => None,
    };

    let painter = ui.painter().clone();
    // Its icon, its name, its path and kind, and the word marking a missing one.
    let (icon_ink, name_ink, quiet, mark) = if chosen {
        painter.rect_filled(rect, t.shape.radius, t.colors.accent);
        let on = t.colors.text_on_accent;
        (on, on, on, on)
    } else {
        if response.hovered() {
            painter.rect_filled(rect, t.shape.radius, t.colors.hover);
        }
        // Greyed only where it cannot be opened, as Open Recent's: one whose opening failed
        // this run tries again.
        let quiet = t.colors.text_secondary;
        match recent.missing {
            Some(Missing::Gone) => (
                t.colors.text_disabled,
                t.colors.text_disabled,
                quiet,
                t.colors.warning,
            ),
            Some(Missing::Failed(_)) | None => {
                (t.colors.text, t.colors.text_strong, quiet, t.colors.warning)
            }
        }
    };
    let icon = Rect::from_center_size(
        pos2(
            rect.left() + t.spacing.l + ICON_COLUMN / 2.0,
            rect.center().y,
        ),
        vec2(ICON, ICON),
    );
    Icon::Folder.paint(&painter, icon, icon_ink);

    // At the end, its kind; or, for a missing one, Locate… and Remove from the List over
    // the row, laid out from their start, so screen readers and Tab meet them as they are
    // seen, each measured as the button measures its words.
    let text_left = rect.left() + t.spacing.l + ICON_COLUMN + t.spacing.m;
    let mut end = rect.right() - t.spacing.l;
    if gone.is_some() {
        let locate = words.text("browse-locate");
        let remove = words.text("browse-remove");
        let body = egui::FontId::proportional(t.text.body);
        let width = |text: &str| {
            let galley = painter.layout_no_wrap(text.to_owned(), body.clone(), quiet);
            galley.size().x + 2.0 * t.spacing.m
        };
        let room = width(&locate) + t.spacing.s + width(&remove);
        let mut buttons = ui.new_child(
            UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(end - room, rect.top()),
                    pos2(end, rect.bottom()),
                ))
                .layout(Layout::left_to_right(Align::Center)),
        );
        buttons.spacing_mut().item_spacing.x = t.spacing.s;
        if Button::new(&locate).show(&mut buttons).clicked() {
            asked.push(Asked::Locate(recent.path.to_owned()));
        }
        if Button::new(&remove).show(&mut buttons).clicked() {
            asked.push(Asked::Remove(recent.path.to_owned()));
        }
        end -= room + t.spacing.l;
    } else {
        let caption = egui::FontId::proportional(t.text.caption);
        let galley = painter.layout_no_wrap(kind.clone(), caption, quiet);
        end -= galley.size().x;
        let at = pos2(end, rect.center().y - galley.size().y / 2.0);
        painter.galley(at, galley, quiet);
        end -= t.spacing.l;
    }

    // The name over the path, each cut in its middle where too long; a missing one's path
    // after the word that marks it.
    let room = (end - text_left).max(0.0);
    let name_font = egui::FontId::proportional(t.text.title);
    let small = egui::FontId::proportional(t.text.small);
    let shown_name = middle_cut(&painter, &name, "", &name_font, room);
    let name_galley = painter.layout_no_wrap(shown_name, name_font, name_ink);
    let marked = gone.map(|gone| {
        let word = painter.layout_no_wrap(gone, small.clone(), mark);
        let dot = painter.layout_no_wrap(" · ".to_owned(), small.clone(), quiet);
        (word, dot)
    });
    let before = marked
        .as_ref()
        .map_or(0.0, |(word, dot)| word.size().x + dot.size().x);
    let shown_path = middle_cut(&painter, &path, "", &small, (room - before).max(0.0));
    let path_galley = painter.layout_no_wrap(shown_path, small, quiet);
    let top = rect.center().y - (name_galley.size().y + path_galley.size().y) / 2.0;
    let below = top + name_galley.size().y;
    painter.galley(pos2(text_left, top), name_galley, name_ink);
    let mut x = text_left;
    for galley in marked.into_iter().flat_map(|(word, dot)| [word, dot]) {
        let width = galley.size().x;
        painter.galley(pos2(x, below), galley, quiet);
        x += width;
    }
    painter.galley(pos2(x, below), path_galley, quiet);
    if response.has_focus() {
        focus_ring_centred(&painter, rect, &t);
    }
    // To screen readers: its name, then its kind and path, and why it cannot be opened.
    response.widget_info(|| egui::WidgetInfo::labeled(Role::ListBoxOption, true, &name));
    let mut args = Args::new();
    args.set("kind", kind);
    args.set("path", path);
    let about = match recent.missing {
        Some(missing) => {
            args.set("why", crate::library::missing_words(&ctx, missing));
            words.text_with("start-recent-about-missing", Some(&args))
        }
        None => words.text_with("start-recent-about", Some(&args)),
    };
    ctx.accesskit_node_builder(id, |node| {
        node.set_selected(chosen);
        node.set_description(about);
    });
    asked
}

#[cfg(test)]
mod tests;
