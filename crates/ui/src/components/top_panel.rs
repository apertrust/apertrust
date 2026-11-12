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

//! The top panel (`docs/gui.md`, The top panel): at its start, on Windows and Linux, the
//! menu button, ☰, which opens the menus ([`TopPanel::menu_button`], decision 39), then the
//! mark, a button that opens About, and the common actions, Import, Export, Reset, Undo
//! and Redo, then after a gap Copy edit and Apply edit; at its end, when the app asks, the
//! jobs spinner ([`TopPanel::jobs`]), then the edit's save state ([`SaveState`]), which
//! shows only when a save goes wrong. Each sits in a slot of its own, as wide as its widest
//! form at the fold, kept whether it shows anything or not, so neither ever moves anything.
//! Each tooltip carries its action's keys, which the app's actions registry gives.
//! As it narrows it gives way one step at a time ([`Fold`]), always keeping empty panel to
//! move the window by.
//!
//! It runs to the window's top edge and is the window's title bar ([`TopPanel::title_bar`];
//! `docs/gui.md`, The window frame and menus, decision 39): it keeps room at its start for
//! the window's own buttons where the system draws them there, as macOS does
//! ([`TopPanel::start_inset`]), and reports a drag starting or a double-click on its empty
//! areas, the same on every platform, for the app to move the window or do what the
//! system does on a double-click, and a right-click there, for the window's menu. It
//! draws the caption buttons Windows and Linux need, minimise, maximise or restore and
//! close, at its very end when asked ([`TopPanel::caption_buttons`]). It knows nothing of
//! platforms: the app says whether it is the title bar, how much room the system's
//! buttons take, whether to draw its own and whether the window is maximised.

use egui::accesskit::{HasPopup, Live};
use egui::{
    Align, FocusDirection, FontId, Id, Key, Layout, Modifiers, Pos2, Rect, Role, Sense, Stroke, Ui,
    UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::{IconButton, Jobs, ToolButton, focus_ring, pull_down, report_shortcut};
use crate::i18n::Args;
use crate::{Icon, popup, theme, tr, translator};

/// What the user did to the top panel this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopPanelAction {
    /// The ☰ ([`TopPanel::menu_button`]), pressed or given ↓ while its menu is closed: the
    /// menus should open, hung from [`TopPanel::menu_anchor`] as the popup
    /// [`TopPanel::menu_popup`].
    Menu,
    /// The mark: About Apertrust.
    About,
    Import,
    Export,
    Reset,
    Undo,
    Redo,
    CopyEdit,
    ApplyEdit,
    /// An empty part of the panel pressed and dragged, where the panel is the title bar:
    /// the window should follow the pointer. Reported once a press.
    MoveWindow,
    /// An empty part of the panel double-clicked, where the panel is the title bar: the
    /// window should do what a double-click on its title bar does.
    TitleBarDoubleClick,
    /// An empty part of the panel right-clicked, where the panel is the title bar: the
    /// window's menu should open.
    WindowMenu,
    /// The jobs spinner ([`TopPanel::jobs`]): the jobs window should open, or come to the
    /// front.
    Jobs,
    /// The save warning ([`SaveState::Failed`]): why the edit is not saved should show,
    /// with Try again.
    NotSaved,
    /// The caption buttons ([`TopPanel::caption_buttons`]): minimise the window.
    Minimise,
    /// Maximise the window, or restore it where it is maximised.
    MaximiseOrRestore,
    /// Close the window.
    Close,
}

/// Which common actions can run now; the rest are drawn disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Available {
    pub import: bool,
    pub export: bool,
    pub reset: bool,
    pub undo: bool,
    pub redo: bool,
    pub copy: bool,
    pub apply: bool,
}

impl Default for Available {
    fn default() -> Self {
        Available {
            import: true,
            export: true,
            reset: true,
            undo: true,
            redo: true,
            copy: true,
            apply: true,
        }
    }
}

/// Where saving the edit stands, for the save state at the panel's end (`docs/gui.md`,
/// The edit, saved). Only a save going wrong shows: nothing while saving works.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SaveState<'a> {
    /// No edit to save: no photo open, one opening, or one open for viewing only, whose
    /// caption says so (decision 43).
    #[default]
    None,
    /// Saving works: nothing shows.
    Saved,
    /// A write of the open photo's edit still unconfirmed about two seconds after it
    /// started, as on a slow network folder: "Saving…".
    Saving,
    /// The last write failed, for this reason, in the system's words: the warning sign and
    /// "Not saved", a button, until a write succeeds.
    Failed(&'a str),
}

#[derive(Default)]
pub struct TopPanel<'a> {
    available: Available,
    saved: SaveState<'a>,
    jobs: Option<Jobs<'a>>,
    undo_tip: Option<&'a str>,
    shortcuts: Vec<(TopPanelAction, &'a str)>,
    start_inset: f32,
    menu: bool,
    title_bar: bool,
    caption_buttons: bool,
    maximised: bool,
}

const BUTTON: f32 = 30.0;
/// A caption button's width, as Windows 11's; it is as tall as the panel.
const CAPTION_BUTTON: f32 = 46.0;
/// What the folded "Saving…" shows.
const ELLIPSIS: &str = "…";
/// The empty panel always left to move the window by, however narrow.
const EMPTY: f32 = 24.0;
/// The room beyond its own width a step needs to be undone, so a resize never flickers.
const SPARE: f32 = 16.0;

/// How far the panel gives way to its width, one step at a time (`docs/gui.md`, The top
/// panel, Narrowing). The ☰ and the mark never fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fold {
    /// Everything shown: Import, Export and Reset with their words.
    None,
    /// The actions are icons.
    Words,
    /// Every action but Undo and Redo is under ⋯.
    Actions,
    /// The save state loses its word.
    Status,
    /// Undo and Redo join ⋯.
    Undo,
}

impl Fold {
    const ALL: [Fold; 5] = [
        Fold::None,
        Fold::Words,
        Fold::Actions,
        Fold::Status,
        Fold::Undo,
    ];

    /// Whether `action` is under ⋯.
    fn hides(self, action: TopPanelAction) -> bool {
        match self {
            Fold::None | Fold::Words => false,
            Fold::Actions | Fold::Status => {
                !matches!(action, TopPanelAction::Undo | TopPanelAction::Redo)
            }
            Fold::Undo => true,
        }
    }
}

/// One of the common actions.
struct Common {
    icon: Icon,
    /// Its translated name; Undo's says what it would undo.
    name: String,
    enabled: bool,
    action: TopPanelAction,
}

impl Common {
    /// Whether it shows its name beside its icon while there is room: Import, Export and
    /// Reset.
    fn has_words(&self) -> bool {
        matches!(
            self.action,
            TopPanelAction::Import | TopPanelAction::Export | TopPanelAction::Reset
        )
    }
}

impl<'a> TopPanel<'a> {
    pub fn new() -> Self {
        TopPanel::default()
    }

    /// Keeps `points` clear at the panel's start, in place of its margin there: room for
    /// the window's own buttons where the system draws them at the start, as macOS does, as
    /// the app measures them where they are.
    pub fn start_inset(mut self, points: f32) -> Self {
        self.start_inset = points;
        self
    }

    /// Draws the menu button, ☰, first, before the mark, where the system has no menu bar:
    /// on Windows and Linux (decision 39). It never folds. Pressed, or given ↓, it reports
    /// [`TopPanelAction::Menu`] for the app to open the menus as the popup
    /// [`TopPanel::menu_popup`]; it is drawn on while that popup hangs from it, and a press
    /// then closes it. Off by default.
    pub fn menu_button(mut self, on: bool) -> Self {
        self.menu = on;
        self
    }

    /// The popup the ☰'s menu opens as ([`popup::open`]).
    pub fn menu_popup() -> Id {
        Id::unique("apertrust-ui top panel menu")
    }

    /// The ☰'s rectangle as last drawn, for the menu to hang from; None before a top panel
    /// has drawn one.
    pub fn menu_anchor(ctx: &egui::Context) -> Option<Rect> {
        ctx.data(|d| d.get_temp(menu_anchor_id()))
    }

    /// Makes the panel the window's title bar: a drag that starts on an empty part of it
    /// reports [`TopPanelAction::MoveWindow`] once it has moved, never on the press, so a
    /// double-click stays one and reports [`TopPanelAction::TitleBarDoubleClick`]. Its
    /// buttons, the mark among them, take their presses as before; the status slots count
    /// as empty but for the save warning and the spinner.
    /// A right-click on an empty part reports [`TopPanelAction::WindowMenu`].
    pub fn title_bar(mut self, on: bool) -> Self {
        self.title_bar = on;
        self
    }

    /// Draws the window's caption buttons at the panel's very end, after a group's gap,
    /// where the system draws none: minimise, maximise or restore, and close, each as tall
    /// as the panel and 46 points wide. Their clicks report [`TopPanelAction::Minimise`],
    /// [`TopPanelAction::MaximiseOrRestore`] and [`TopPanelAction::Close`]. Off by
    /// default.
    pub fn caption_buttons(mut self, on: bool) -> Self {
        self.caption_buttons = on;
        self
    }

    /// Whether the window is maximised, so the caption buttons offer Restore in place of
    /// Maximise.
    pub fn maximised(mut self, maximised: bool) -> Self {
        self.maximised = maximised;
        self
    }

    pub fn available(mut self, available: Available) -> Self {
        self.available = available;
        self
    }

    /// The save state, at the panel's very end. Its slot is kept at its widest at every
    /// state, so it never moves the fold or the spinner. A press of the save warning comes
    /// back as [`TopPanelAction::NotSaved`].
    pub fn saved(mut self, saved: SaveState<'a>) -> Self {
        self.saved = saved;
        self
    }

    /// Draws the jobs spinner before the save state while there are jobs to show, and keeps
    /// its live region in the tree, empty while there are none. Its press comes back as
    /// [`TopPanelAction::Jobs`]. Its slot is kept with or without jobs, a group's gap before
    /// the save state's, so neither moves the other or anything else. The spinner has no
    /// words, so it never folds. Off by default.
    pub fn jobs(mut self, jobs: Jobs<'a>) -> Self {
        self.jobs = Some(jobs);
        self
    }

    /// What Undo would undo, for its tooltip: "Undo Exposure: Brightness +8".
    pub fn undo_tip(mut self, tip: &'a str) -> Self {
        self.undo_tip = Some(tip);
        self
    }

    /// The keys that run `action`, in the platform's spelling (`Ctrl+E`), for its tooltip;
    /// the app's actions registry handles them.
    pub fn shortcut(mut self, action: TopPanelAction, keys: &'a str) -> Self {
        self.shortcuts.push((action, keys));
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<TopPanelAction> {
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let (bar, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), t.sizes.top_bar), Sense::hover());
        let toolbar_name = tr(&ctx, "top-panel");
        response.widget_info(|| WidgetInfo::labeled(Role::Toolbar, true, &toolbar_name));
        ui.painter().rect_filled(bar, 0.0, t.colors.surface);
        ui.painter().hline(
            bar.x_range(),
            bar.bottom() - t.shape.separator_width / 2.0,
            Stroke::new(t.shape.separator_width, t.colors.separator),
        );
        let mut action = None;
        // Every control's rectangle, enabled or not: none of them is an empty area.
        let mut controls = Vec::new();
        let mut pressed = |response: egui::Response| {
            controls.push(response.rect);
            response.clicked()
        };
        let shortcuts = &self.shortcuts;
        let keys = |action: TopPanelAction| {
            shortcuts
                .iter()
                .find(|(a, _)| *a == action)
                .map(|(_, keys)| *keys)
        };
        let mut inner = bar.shrink2(vec2(t.spacing.m, 0.0));
        inner.min.x = bar.min.x + self.start_inset.max(t.spacing.m);
        // The caption buttons take the panel's end, a group's gap before them.
        let caption = self.caption_buttons.then(|| {
            let mut caption = bar;
            caption.min.x = bar.max.x - 3.0 * CAPTION_BUTTON;
            inner.max.x = caption.min.x - t.spacing.xl;
            caption
        });

        let a = self.available;
        let undo = self
            .undo_tip
            .map_or_else(|| tr(&ctx, "action-undo"), str::to_owned);
        let common = [
            (
                Icon::Import,
                "action-import",
                a.import,
                TopPanelAction::Import,
            ),
            (
                Icon::Export,
                "action-export",
                a.export,
                TopPanelAction::Export,
            ),
            (Icon::Reset, "action-reset", a.reset, TopPanelAction::Reset),
            (Icon::Undo, "action-undo", a.undo, TopPanelAction::Undo),
            (Icon::Redo, "action-redo", a.redo, TopPanelAction::Redo),
            (
                Icon::Copy,
                "action-copy-edit",
                a.copy,
                TopPanelAction::CopyEdit,
            ),
            (
                Icon::Apply,
                "action-apply-edit",
                a.apply,
                TopPanelAction::ApplyEdit,
            ),
        ]
        .map(|(icon, id, enabled, action)| Common {
            icon,
            name: if action == TopPanelAction::Undo {
                undo.clone()
            } else {
                tr(&ctx, id)
            },
            enabled,
            action,
        });
        let tool = |c| tool_button(c, keys(c.action));

        // How far to fold: the first step whose parts leave the empty panel, measured at
        // the current scale and language as egui lays them out, `gap` apart.
        let gap = t.spacing.xxs;
        let start = |fold: Fold| {
            // The ☰, where there is one, and the mark.
            let mut parts = vec![BUTTON; if self.menu { 2 } else { 1 }];
            // After the mark, and after the ☰ the same.
            let mut spaces = t.spacing.s * if self.menu { 2.0 } else { 1.0 };
            for c in common.iter().filter(|c| !fold.hides(c.action)) {
                if c.action == TopPanelAction::CopyEdit {
                    spaces += t.spacing.xl - gap;
                }
                let words = c.has_words() && fold == Fold::None;
                parts.push(if words { tool(c).width(ui) } else { BUTTON });
            }
            if common.iter().any(|c| fold.hides(c.action)) {
                parts.push(BUTTON);
            }
            parts.iter().sum::<f32>() + gap * (parts.len() as f32 - 1.0) + spaces
        };
        // The save state's slot, at its widest form at each fold, Saving… or the warning,
        // whatever it shows, so a save never moves the fold, the spinner or anything else.
        let saving = SaveSlot::measure(ui);
        let status_end = |fold: Fold| saving.width(fold >= Fold::Status);
        // The jobs spinner's slot, a group's gap before the save state's, kept with or
        // without jobs. It has no words, so it is the same at every fold.
        let spinner_end = if self.jobs.is_some() {
            BUTTON + t.spacing.m
        } else {
            0.0
        };
        let end = |fold: Fold| status_end(fold) + spinner_end;
        let id = response.id.with("fold");
        let last = ui.data(|d| d.get_temp::<Fold>(id)).unwrap_or(Fold::None);
        let fold = Fold::ALL
            .into_iter()
            .find(|&fold| {
                let spare = if fold < last { SPARE } else { 0.0 };
                start(fold) + EMPTY + end(fold) + spare <= inner.width()
            })
            .unwrap_or(Fold::Undo);
        ui.data_mut(|d| d.insert_temp(id, fold));

        // At the start: the mark and the common actions, the folded ones under ⋯.
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                if self.menu {
                    let (button, press) = menu_button(ui, keys(TopPanelAction::Menu));
                    pressed(button);
                    match press {
                        Some(MenuPress::Open) => action = Some(TopPanelAction::Menu),
                        Some(MenuPress::Close) => popup::close(&ctx, TopPanel::menu_popup()),
                        None => {}
                    }
                    // As far from the mark as the mark is from Import.
                    ui.add_space(t.spacing.s);
                }
                // The mark, in secondary text so it isn't read as the first of the actions.
                let about = tr(&ctx, "action-about");
                let mark = mark(ui, &about, keys(TopPanelAction::About));
                if pressed(mark) {
                    action = Some(TopPanelAction::About);
                }
                ui.add_space(t.spacing.s);
                for c in common.iter().filter(|c| !fold.hides(c.action)) {
                    // Copy edit and Apply edit are a group of their own.
                    if c.action == TopPanelAction::CopyEdit {
                        ui.add_space(t.spacing.xl - gap);
                    }
                    let response = if c.has_words() && fold == Fold::None {
                        tool(c).show(ui)
                    } else {
                        icon_button(c.icon, &c.name, keys(c.action))
                            .size(BUTTON)
                            .enabled(c.enabled)
                            .show(ui)
                    };
                    if pressed(response) {
                        action = Some(c.action);
                    }
                }
                if common.iter().any(|c| fold.hides(c.action)) {
                    let more = IconButton::new(Icon::Options, &tr(&ctx, "top-panel-more"))
                        .size(BUTTON)
                        .show(ui);
                    pressed(more.clone());
                    // Each with its keys, and disabled where it can't run. Screen readers
                    // hear its name, and the keys as its shortcut, as on every control.
                    let name = tr(&ctx, "top-panel-more");
                    let chosen = pull_down(ui, &more, &name, || {
                        common
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| fold.hides(c.action))
                            .map(|(i, c)| {
                                let item = popup::Item::new(i.to_string(), c.name.as_str())
                                    .enabled(c.enabled);
                                match keys(c.action) {
                                    Some(keys) => item.shortcut(keys),
                                    None => item,
                                }
                                .into()
                            })
                            .collect()
                    });
                    if let Some(i) = chosen.and_then(|chosen| chosen.parse::<usize>().ok()) {
                        action = Some(common[i].action);
                    }
                }
            },
        );

        // At the end: the save state, in its slot, right-aligned.
        let folded = fold >= Fold::Status;
        let mut slot = inner;
        slot.min.x = inner.max.x - status_end(fold);
        let save = ui.scope_builder(
            UiBuilder::new()
                .id_salt("save state")
                .max_rect(slot)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| saving.show(ui, self.saved, folded),
        );
        if let Some(button) = save.inner
            && pressed(button)
        {
            action = Some(TopPanelAction::NotSaved);
        }

        // Before it: the jobs spinner, in its own slot, a group's gap before.
        if let Some(jobs) = self.jobs {
            let mut room = inner;
            room.max.x = slot.min.x - t.spacing.m;
            room.min.x = room.max.x - BUTTON;
            ui.scope_builder(
                UiBuilder::new()
                    .id_salt("jobs")
                    .max_rect(room)
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    if let Some(spinner) = jobs.spinner(ui)
                        && pressed(spinner)
                    {
                        action = Some(TopPanelAction::Jobs);
                    }
                },
            );
        }

        // At the very end: the window's caption buttons, after everything else in the focus
        // order too.
        if let Some(caption) = caption {
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(caption)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (icon, middle) = if self.maximised {
                        (Icon::Restore, "window-restore")
                    } else {
                        (Icon::Maximise, "window-maximise")
                    };
                    for (icon, id, close, done) in [
                        (
                            Icon::Minimise,
                            "window-minimise",
                            false,
                            TopPanelAction::Minimise,
                        ),
                        (icon, middle, false, TopPanelAction::MaximiseOrRestore),
                        (Icon::Close, "window-close", true, TopPanelAction::Close),
                    ] {
                        if pressed(caption_button(ui, icon, &tr(&ctx, id), close, &t)) {
                            action = Some(done);
                        }
                    }
                },
            );
        }
        if self.title_bar {
            action = action.or_else(|| title_bar_gesture(ui, bar, response.id, &controls));
        }
        // What follows starts under the panel, whatever its last part left.
        ui.advance_cursor_after_rect(bar);
        action
    }
}

/// The save state's slot at the panel's end, measured at the current scale and language:
/// "Saving…" or the save warning, whichever is wider, with its word or, folded, without.
struct SaveSlot {
    full: f32,
    folded: f32,
}

/// What the save state's live region last heard of, kept in egui's memory.
#[derive(Clone, Default)]
struct SaveHeard {
    /// Why the last write failed, while it is the open photo's.
    failed: Option<String>,
    said: String,
}

impl SaveSlot {
    fn measure(ui: &Ui) -> SaveSlot {
        let t = theme(ui.ctx());
        let word = |text: String| {
            let font = FontId::proportional(t.text.small);
            (ui.painter())
                .layout_no_wrap(text, font, t.colors.text_secondary)
                .size()
                .x
        };
        let sign = 2.0 * t.spacing.s + Icon::Warning.size();
        let warning = sign + t.spacing.xs + word(tr(ui.ctx(), "status-not-saved"));
        SaveSlot {
            full: word(tr(ui.ctx(), "status-saving")).max(warning),
            folded: word(ELLIPSIS.to_owned()).max(sign),
        }
    }

    fn width(&self, folded: bool) -> f32 {
        if folded { self.folded } else { self.full }
    }

    /// Draws `state` at the right of `ui`, its word left out where `folded`, and keeps the
    /// live region in the tree. Returns the save warning's response, where it shows.
    fn show(&self, ui: &mut Ui, state: SaveState<'_>, folded: bool) -> Option<egui::Response> {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let words = translator(&ctx);
        let font = FontId::proportional(t.text.small);
        let because = |why: &str| {
            let mut args = Args::new();
            args.set("error", why);
            words.text_with("status-not-saved-reason", Some(&args))
        };
        let said = announce(ui, state, &because);
        let at = ui.next_widget_position();
        let button = match state {
            SaveState::None | SaveState::Saved => None,
            SaveState::Saving => {
                let name = words.text("status-saving");
                let shown = if folded {
                    ELLIPSIS.to_owned()
                } else {
                    name.clone()
                };
                let galley = (ui.painter()).layout_no_wrap(shown, font, t.colors.text_secondary);
                let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
                ui.painter()
                    .galley(rect.min, galley, t.colors.text_secondary);
                response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
                response.on_hover_text(words.text("status-saving-tip"));
                None
            }
            SaveState::Failed(why) => {
                let name = words.text("status-not-saved");
                let galley = (!folded)
                    .then(|| (ui.painter()).layout_no_wrap(name.clone(), font, t.colors.warning));
                let sign = Icon::Warning.size();
                let word = galley.as_ref().map_or(0.0, |g| t.spacing.xs + g.size().x);
                let width = 2.0 * t.spacing.s + sign + word;
                let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON), Sense::click());
                if ui.is_rect_visible(rect) {
                    let painter = ui.painter();
                    if response.hovered() || response.is_pointer_button_down_on() {
                        painter.rect_filled(rect, t.shape.radius, t.colors.hover);
                    }
                    let icon = Rect::from_min_size(
                        pos2(rect.left() + t.spacing.s, rect.center().y - sign / 2.0),
                        Vec2::splat(sign),
                    );
                    Icon::Warning.paint(painter, icon, t.colors.warning);
                    if let Some(galley) = galley {
                        let at = pos2(
                            icon.right() + t.spacing.xs,
                            rect.center().y - galley.size().y / 2.0,
                        );
                        painter.galley(at, galley, t.colors.warning);
                    }
                    if response.has_focus() {
                        focus_ring(painter, rect, &t);
                    }
                }
                // Heard as a button named Not saved, the reason its description; its
                // tooltip says both.
                response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
                ctx.accesskit_node_builder(response.id, |node| node.set_description(why));
                Some(response.on_hover_text(because(why)))
            }
        };
        // An assertive live region that takes no room, saying what `announce` keeps.
        let live = ui.interact(
            Rect::from_min_size(at, Vec2::ZERO),
            ui.scope_id().with("save live region"),
            Sense::hover(),
        );
        live.widget_info(|| WidgetInfo::labeled(Role::Status, true, &said));
        ctx.accesskit_node_builder(live.id, |node| node.set_live(Live::Assertive));
        button
    }
}

/// What the save state's live region says now: "Not saved: …" with the reason once a write
/// fails, "Saved" once one succeeds after a failure, kept until the next, so nothing is
/// heard on every save. A photo left with its write failed is the central panel's to tell.
fn announce(ui: &Ui, state: SaveState<'_>, because: &dyn Fn(&str) -> String) -> String {
    let id = ui.scope_id().with("save heard");
    let mut heard: SaveHeard = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    match state {
        SaveState::Failed(why) => {
            if heard.failed.as_deref() != Some(why) {
                heard.said = because(why);
                heard.failed = Some(why.to_owned());
            }
        }
        SaveState::Saved | SaveState::Saving => {
            if heard.failed.take().is_some() {
                heard.said = tr(ui.ctx(), "status-saved");
            }
        }
        SaveState::None => heard.failed = None,
    }
    let said = heard.said.clone();
    ui.data_mut(|d| d.insert_temp(id, heard));
    said
}

/// What a press of the ☰ asks for.
enum MenuPress {
    Open,
    /// Its menu is open, hung from it.
    Close,
}

fn menu_anchor_id() -> Id {
    Id::unique("apertrust-ui top panel menu anchor")
}

/// The ☰, with `keys` in its tooltip: a button that opens a menu, drawn on while the menu
/// hangs from it. Pressed, or given ↓ while focused, it asks for the menu to open, or
/// to close where it is open.
fn menu_button(ui: &mut Ui, keys: Option<&str>) -> (egui::Response, Option<MenuPress>) {
    let ctx = ui.ctx().clone();
    // Where the button goes, the next in its row, as the menu was hung from it.
    let room = ui.available_rect_before_wrap();
    let centre = pos2(room.left() + BUTTON / 2.0, room.center().y);
    let open = popup::anchor(&ctx, TopPanel::menu_popup()).is_some_and(|a| a.contains(centre));
    let name = tr(&ctx, "top-panel-menu");
    let mut button = icon_button(Icon::Menu, &name, keys).size(BUTTON);
    if open {
        button = button.toggled(true);
    }
    let response = button.show(ui);
    ctx.data_mut(|d| d.insert_temp(menu_anchor_id(), response.rect));
    // A button that opens a menu, never a toggle.
    ctx.accesskit_node_builder(response.id, |node| {
        node.clear_toggled();
        node.set_has_popup(HasPopup::Menu);
        node.set_expanded(open);
    });
    let down = !open
        && response.has_focus()
        && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
    if down {
        // egui's focus would move down too.
        ui.memory_mut(|m| m.move_focus(FocusDirection::None));
    }
    let press = match (response.clicked() || down, open) {
        (false, _) => None,
        (true, false) => Some(MenuPress::Open),
        (true, true) => Some(MenuPress::Close),
    };
    (response, press)
}

/// What the user did to the empty areas of the panel, `bar`, where it is the title bar: a
/// double-click, a right-click, or a press there that has turned into a drag, reported
/// once a press. A press becomes a drag only once it has moved or been held, so a click
/// stays a click and two make a double-click.
///
/// Read from the pointer, not from a widget under the panel's controls, which would take
/// from each the margin within which egui gives a near press to the closest control: a
/// press within that margin is the control's, never the title bar's.
fn title_bar_gesture(
    ui: &Ui,
    bar: Rect,
    id: egui::Id,
    controls: &[Rect],
) -> Option<TopPanelAction> {
    let margin = ui.style().interaction.interact_radius;
    let layer = ui.layer_id();
    // On the panel, under no popup over it, and off every control, a disabled one too.
    let is_empty = |at: Pos2| {
        bar.contains(at)
            && ui.ctx().layer_id_at(at) == Some(layer)
            && !controls
                .iter()
                .any(|control| control.expand(margin).contains(at))
    };
    let (pressed, down, dragging, double, menu, at, origin) = ui.input(|i| {
        let pointer = &i.pointer;
        let down = pointer.primary_down();
        (
            pointer.primary_pressed(),
            down,
            down && pointer.is_decidedly_dragging(),
            pointer.button_double_clicked(egui::PointerButton::Primary),
            pointer.button_clicked(egui::PointerButton::Secondary),
            pointer.interact_pos(),
            pointer.press_origin(),
        )
    });
    // A new press may come without the last one's release, which the system's window
    // drag can swallow, so each press may report a move again.
    let reported = id.with("moved");
    if pressed || !down {
        ui.data_mut(|d| d.remove::<bool>(reported));
    }
    // A double-click's clicks are close together, egui counts them so, and each lands
    // where it was pressed.
    if double && at.is_some_and(is_empty) {
        return Some(TopPanelAction::TitleBarDoubleClick);
    }
    // A right-click, on its release as egui's context menus open, where it was let go.
    if menu && at.is_some_and(is_empty) {
        return Some(TopPanelAction::WindowMenu);
    }
    // egui lets go of a press's target once it moves, so where it began is asked of the
    // panel.
    if dragging
        && origin.is_some_and(is_empty)
        && !ui.data(|d| d.get_temp::<bool>(reported).unwrap_or(false))
    {
        ui.data_mut(|d| d.insert_temp(reported, true));
        return Some(TopPanelAction::MoveWindow);
    }
    None
}

/// One of the window's caption buttons, 46 points wide and as tall as `ui` has room for:
/// no corners, `hover` behind it under the pointer, or, for `close`, the close button's
/// red with its own icon colour. The focus ring is drawn inside, since the button meets
/// the window's edges. The top panel draws minimise, maximise and close with it, and the
/// dialog host a dialog's close where it draws the window's frame.
pub fn caption_button(
    ui: &mut Ui,
    icon: Icon,
    name: &str,
    close: bool,
    t: &crate::Theme,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(CAPTION_BUTTON, ui.available_height()), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let lit = response.hovered() || response.is_pointer_button_down_on();
        // Above the panel's bottom line, which stays.
        let fill = rect.with_max_y(rect.max.y - t.shape.separator_width);
        let colour = if lit && close {
            painter.rect_filled(fill, 0.0, t.colors.close_hover);
            t.colors.text_on_close_hover
        } else {
            if lit {
                painter.rect_filled(fill, 0.0, t.colors.hover);
            }
            t.colors.text
        };
        let size = icon.size();
        icon.paint(
            painter,
            Rect::from_center_size(rect.center(), vec2(size, size)),
            colour,
        );
        if response.has_focus() {
            // The ring lies outside the rectangle it is given: here, inside the button.
            focus_ring(painter, rect.shrink(2.0), t);
        }
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    response.on_hover_text(name)
}

/// A common action with its words beside its icon, with `keys` in its tooltip when it has
/// any.
fn tool_button<'a>(c: &'a Common, keys: Option<&'a str>) -> ToolButton<'a> {
    let button = ToolButton::new(&c.name).icon(c.icon).enabled(c.enabled);
    match keys {
        Some(keys) => button.shortcut(keys),
        None => button,
    }
}

/// An icon button, with `keys` in its tooltip when it has any.
/// The mark, which a click turns into About: drawn plain, with no hover and no tooltip,
/// so it reads as the app's mark rather than a button. Screen readers hear it as a button
/// named `name`, with `keys`, and a key can focus and press it.
fn mark(ui: &mut Ui, name: &str, keys: Option<&str>) -> egui::Response {
    let t = theme(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(BUTTON, BUTTON), Sense::click());
    if ui.is_rect_visible(rect) {
        let size = Icon::Mark.size();
        Icon::Mark.paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(size, size)),
            t.colors.text_secondary,
        );
        if response.has_focus() {
            focus_ring(ui.painter(), rect, &t);
        }
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    if let Some(keys) = keys {
        report_shortcut(ui, response.id, keys);
    }
    response
}

fn icon_button<'a>(icon: Icon, name: &'a str, keys: Option<&'a str>) -> IconButton<'a> {
    let button = IconButton::new(icon, name);
    match keys {
        Some(keys) => button.shortcut(keys),
        None => button,
    }
}
