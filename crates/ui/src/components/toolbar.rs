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

//! The Toolbar (`docs/gui.md`, The Toolbar): a list of entries drawn in order (decision 92;
//! [`ToolbarEntry`], [`Toolbar::entries`]), which the app works out from the workspace's
//! list, so it knows parts and buttons only as plain data: the menu button, drawn where
//! the app draws the menus, Windows and Linux ([`Toolbar::menu_button`], decision 39), as
//! the app's icon with a caret or as ☰ ([`MenuButtonStyle`]); buttons that run actions
//! ([`ToolbarButton`]); fixed and flexible spaces; the cursors' well
//! ([`Toolbar::cursors`]); the jobs spinner ([`Toolbar::jobs`]); and the edit's save state
//! ([`SaveState`]), which shows only when a save goes wrong. The entries before the first
//! flexible space stand at the start, those after the last at the end, and those between
//! two in the middle of the room the flexible spaces share. The spinner and the save state
//! each sit in a slot of its own, as wide as its widest form at the fold, kept whether it
//! shows anything or not, so neither ever moves anything. Each tooltip carries its
//! action's keys, which the app's actions registry gives. As it narrows it gives way one
//! step at a time ([`Fold`]), always keeping empty panel to move the window by.
//!
//! **Guards** (decision 92): with no menu button in the list, the menus hang from the
//! Toolbar's start ([`Toolbar::menu_anchor`]); with no save state, a failed write still
//! shows its warning at the end, and its live region stays; with no jobs, the spinner
//! still shows at the end while a failed job is left. With no entries at all, the whole
//! panel is empty area.
//!
//! It runs to the window's top edge and is the window's title bar ([`Toolbar::title_bar`];
//! `docs/gui.md`, The window frame and menus, decision 39): it keeps room at its start for
//! the window's own buttons where the system draws them there, as macOS does
//! ([`Toolbar::start_inset`]), and reports a drag starting or a double-click on its empty
//! areas, the same on every platform, for the app to move the window or do what the
//! system does on a double-click, and a right-click there, for the window's menu. It
//! draws the caption buttons Windows and Linux need, minimise, maximise or restore and
//! close, at its very end when asked ([`Toolbar::caption_buttons`]). It knows nothing of
//! platforms: the app says whether it is the title bar, how much room the system's
//! buttons take, whether to draw its own and whether the window is maximised.

use apertrust_base::settings::Choice;
use egui::accesskit::{HasPopup, Live};
use egui::{
    Align, FocusDirection, FontId, Id, Key, Layout, Modifiers, Pos2, Rect, Role, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use super::icon_button::colour;
use super::plate::{Touch, paint_focus, undimmed_painter};
use super::{
    CursorWell, CursorWellAction, IconButton, Jobs, ToolButton, focus_ring, pull_down,
    report_shortcut,
};
use crate::i18n::Args;
use crate::{Icon, popup, theme, tr, translator};

/// What the user did to the Toolbar this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    /// The menu button ([`Toolbar::menu_button`]), pressed or given ↓ while its menu is
    /// closed: the menus should open, hung from [`Toolbar::menu_anchor`] as the popup
    /// [`Toolbar::menu_popup`].
    Menu,
    /// The button of this id ([`ToolbarButton::new`]), pressed on the panel or chosen
    /// under ⋯.
    Button(&'static str),
    /// An empty part of the panel pressed and dragged, where the panel is the title bar:
    /// the window should follow the pointer. Reported once a press.
    MoveWindow,
    /// An empty part of the panel double-clicked, where the panel is the title bar: the
    /// window should do what a double-click on its title bar does.
    TitleBarDoubleClick,
    /// An empty part of the panel right-clicked, where the panel is the title bar: the
    /// window's menu should open.
    WindowMenu,
    /// The jobs spinner ([`Toolbar::jobs`]): the jobs window should open, or come to the
    /// front.
    Jobs,
    /// The save warning ([`SaveState::Failed`]): why the edit is not saved should show,
    /// with Try again.
    NotSaved,
    /// The cursors' well ([`Toolbar::cursors`]): what was pressed or chosen in it.
    Cursors(CursorWellAction),
    /// The caption buttons ([`Toolbar::caption_buttons`]): minimise the window.
    Minimise,
    /// Maximise the window, or restore it where it is maximised.
    MaximiseOrRestore,
    /// Close the window.
    Close,
}

/// One entry of the Toolbar's list (decision 92), drawn in the list's order. Each part
/// and button is meant to be listed once; the spaces any number of times.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolbarEntry {
    /// The menu button, drawn only where the app draws the menus
    /// ([`Toolbar::menu_button`]).
    MenuButton,
    /// A button that runs an action.
    Button(ToolbarButton),
    /// 16 points (`spacing.xl`) between its neighbours, in place of the usual gap; drawn
    /// only between two entries drawn, none of them a flexible space.
    Space,
    /// An equal share of the room the rest leave.
    FlexibleSpace,
    /// The cursors' well, as [`Toolbar::cursors`] gives it.
    Cursors,
    /// The jobs spinner's slot, as [`Toolbar::jobs`] gives it.
    Jobs,
    /// The save state's slot ([`Toolbar::saved`]).
    SaveState,
}

/// A button of the Toolbar that runs an action: its icon, its translated name and whether
/// it can run now, drawn disabled where it cannot.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarButton {
    id: &'static str,
    icon: Icon,
    name: String,
    enabled: bool,
    words: bool,
    last: bool,
}

impl ToolbarButton {
    /// The button `id`, which a press reports as [`ToolbarAction::Button`], drawn as `icon`
    /// and named `name`. Enabled by default.
    pub fn new(id: &'static str, icon: Icon, name: impl Into<String>) -> Self {
        ToolbarButton {
            id,
            icon,
            name: name.into(),
            enabled: true,
            words: false,
            last: false,
        }
    }

    /// Its id, which a press reports.
    pub fn id(&self) -> &'static str {
        self.id
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Shows its name beside its icon while there is room, as Import…, Export… and Reset
    /// do: the narrowing's first step folds it.
    pub fn words(mut self, words: bool) -> Self {
        self.words = words;
        self
    }

    /// Goes under ⋯ only at the narrowing's last step, after every other button, as Undo
    /// and Redo do.
    pub fn folds_last(mut self, last: bool) -> Self {
        self.last = last;
        self
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

/// How the menu button is drawn, as the menu button setting chooses on Windows and Linux
/// (`toolbar.menu_button`; `docs/gui.md`, The Toolbar).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MenuButtonStyle {
    /// The app's icon in one colour, then a caret: the default.
    #[default]
    Icon,
    /// ☰.
    Menu,
}

/// The menu button setting's options (decision 43).
impl Choice for MenuButtonStyle {
    const OPTIONS: &'static [(MenuButtonStyle, &'static str)] = &[
        (MenuButtonStyle::Icon, "icon"),
        (MenuButtonStyle::Menu, "menu"),
    ];
}

impl MenuButtonStyle {
    /// The button's width, in points: ☰ in the Toolbar's square, or the app's icon and its
    /// caret with their padding.
    fn width(self) -> f32 {
        match self {
            MenuButtonStyle::Menu => BUTTON,
            MenuButtonStyle::Icon => {
                2.0 * ICON_PADDING + Icon::AppIcon.size() + CARET_GAP + Icon::Caret.size()
            }
        }
    }
}

#[derive(Default)]
pub struct Toolbar<'a> {
    entries: Vec<ToolbarEntry>,
    cursors: Option<CursorWell<'a>>,
    saved: SaveState<'a>,
    jobs: Option<Jobs<'a>>,
    shortcuts: Vec<(ToolbarAction, &'a str)>,
    start_inset: f32,
    menu: bool,
    menu_style: MenuButtonStyle,
    title_bar: bool,
    caption_buttons: bool,
    maximised: bool,
}

const BUTTON: f32 = 30.0;
/// The room each side of the menu button's app icon and caret.
const ICON_PADDING: f32 = 5.0;
/// Between the menu button's app icon and its caret.
const CARET_GAP: f32 = 3.0;
/// A caption button's width, as Windows 11's; it is as tall as the panel.
const CAPTION_BUTTON: f32 = 46.0;
/// What the folded "Saving…" shows.
const ELLIPSIS: &str = "…";
/// The empty panel always left to move the window by, however narrow.
const EMPTY: f32 = 24.0;
/// The room beyond its own width a step needs to be undone, so a resize never flickers.
const SPARE: f32 = 16.0;

/// How far the panel gives way to its width, one step at a time (`docs/gui.md`, The
/// Toolbar, Narrowing). The menu button never folds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fold {
    /// Everything shown: the buttons with words, Import, Export and Reset, with them.
    None,
    /// The buttons are icons.
    Words,
    /// Every button but those that fold last, Undo and Redo, is under ⋯.
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

    /// Whether `button` is under ⋯.
    fn hides(self, button: &ToolbarButton) -> bool {
        match self {
            Fold::None | Fold::Words => false,
            Fold::Actions | Fold::Status => !button.last,
            Fold::Undo => true,
        }
    }
}

/// What the panel draws at a fold, in order: the entries that show, and ⋯ where buttons
/// are under it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drawn<'e> {
    Menu,
    Button(&'e ToolbarButton),
    Space,
    Flexible,
    Cursors,
    Jobs,
    Save,
    More,
}

impl<'a> Toolbar<'a> {
    pub fn new() -> Self {
        Toolbar::default()
    }

    /// What it draws, in order (decision 92). None by default: the panel is then all
    /// empty area, the guards apart.
    pub fn entries(mut self, entries: Vec<ToolbarEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Keeps `points` clear at the panel's start, in place of its margin there: room for
    /// the window's own buttons where the system draws them at the start, as macOS does, as
    /// the app measures them where they are.
    pub fn start_inset(mut self, points: f32) -> Self {
        self.start_inset = points;
        self
    }

    /// Says the app draws the menus here, where the system has no menu bar: on Windows and
    /// Linux (decision 39). The menu button, [`ToolbarEntry::MenuButton`], is drawn only
    /// then, as [`Toolbar::menu_button_style`] says. It never folds. Pressed, or given ↓,
    /// it reports [`ToolbarAction::Menu`] for the app to open the menus as the popup
    /// [`Toolbar::menu_popup`]; it is drawn on while that popup hangs from it, and a press
    /// then closes it. With no menu button in the list, the menus hang from the panel's
    /// start ([`Toolbar::menu_anchor`]). Off by default.
    pub fn menu_button(mut self, on: bool) -> Self {
        self.menu = on;
        self
    }

    /// How the menu button is drawn: the app's icon with a caret, the default, or ☰.
    /// Everything else about it is the same either way: its name, its popup, its tooltip
    /// and its keys.
    pub fn menu_button_style(mut self, style: MenuButtonStyle) -> Self {
        self.menu_style = style;
        self
    }

    /// The popup the menu button's menu opens as ([`popup::open`]).
    pub fn menu_popup() -> Id {
        Id::unique("apertrust-ui toolbar menu")
    }

    /// Where the menus hang from as last drawn: the menu button's rectangle, or where the
    /// app draws the menus with no menu button listed, the place one would take at the
    /// panel's start; None before a Toolbar has drawn either.
    pub fn menu_anchor(ctx: &egui::Context) -> Option<Rect> {
        ctx.data(|d| d.get_temp(menu_anchor_id()))
    }

    /// Makes the panel the window's title bar: a drag that starts on an empty part of it
    /// reports [`ToolbarAction::MoveWindow`] once it has moved, never on the press, so a
    /// double-click stays one and reports [`ToolbarAction::TitleBarDoubleClick`]. Its
    /// buttons take their presses as before; the spaces and the status slots count as
    /// empty but for the save warning and the spinner.
    /// A right-click on an empty part reports [`ToolbarAction::WindowMenu`].
    pub fn title_bar(mut self, on: bool) -> Self {
        self.title_bar = on;
        self
    }

    /// Draws the window's caption buttons at the panel's very end, after a group's gap,
    /// where the system draws none: minimise, maximise or restore, and close, each as tall
    /// as the panel and 46 points wide. Their clicks report [`ToolbarAction::Minimise`],
    /// [`ToolbarAction::MaximiseOrRestore`] and [`ToolbarAction::Close`]. Off by
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

    /// The save state, in [`ToolbarEntry::SaveState`]'s slot. Its slot is kept at its
    /// widest at every state, so it never moves the fold or the spinner. A press of the
    /// save warning comes back as [`ToolbarAction::NotSaved`]. Not listed, a failed write
    /// still shows at the panel's end.
    pub fn saved(mut self, saved: SaveState<'a>) -> Self {
        self.saved = saved;
        self
    }

    /// The jobs, whose spinner [`ToolbarEntry::Jobs`]'s slot draws while there are jobs to
    /// show, its live region kept in the tree, empty while there are none. Its press comes
    /// back as [`ToolbarAction::Jobs`]. Its slot is kept with or without jobs, a group's
    /// gap before the save state's where that follows, so neither moves the other or
    /// anything else. The spinner has no words, so it never folds. Not listed, it shows
    /// at the panel's end while a failed job is left.
    pub fn jobs(mut self, jobs: Jobs<'a>) -> Self {
        self.jobs = Some(jobs);
        self
    }

    /// The cursors' well (decision 104; `docs/gui.md`, The Toolbar), drawn where
    /// [`ToolbarEntry::Cursors`] stands, by default centred between two flexible spaces,
    /// a group's gap before it at least. It never folds: the buttons fold before it, and
    /// the empty panel left to move the window by is counted on both sides of it
    /// together. It is no empty area: a press on it never moves the window. What it
    /// reports comes back as [`ToolbarAction::Cursors`].
    pub fn cursors(mut self, well: CursorWell<'a>) -> Self {
        self.cursors = Some(well);
        self
    }

    /// The keys that run `action`, in the platform's spelling (`Ctrl+E`), for its tooltip;
    /// the app's actions registry handles them.
    pub fn shortcut(mut self, action: ToolbarAction, keys: &'a str) -> Self {
        self.shortcuts.push((action, keys));
        self
    }

    /// Whether the list holds `entry`.
    fn lists(&self, entry: &ToolbarEntry) -> bool {
        self.entries.contains(entry)
    }

    /// What the panel draws at `fold`, `well` the cursors' well given, in order: the
    /// entries that show, a fixed space
    /// only between two of them with no flexible space between, and ⋯, where any button
    /// is under it, just before the first flexible space, or last with none.
    fn arrange(&self, fold: Fold, well: bool) -> Vec<Drawn<'_>> {
        let shown = self.entries.iter().filter_map(|entry| match entry {
            ToolbarEntry::MenuButton => self.menu.then_some(Drawn::Menu),
            ToolbarEntry::Button(button) => (!fold.hides(button)).then_some(Drawn::Button(button)),
            ToolbarEntry::Space => Some(Drawn::Space),
            ToolbarEntry::FlexibleSpace => Some(Drawn::Flexible),
            ToolbarEntry::Cursors => well.then_some(Drawn::Cursors),
            ToolbarEntry::Jobs => self.jobs.is_some().then_some(Drawn::Jobs),
            ToolbarEntry::SaveState => Some(Drawn::Save),
        });
        let shown: Vec<Drawn> = shown.collect();
        let item = |d: &&Drawn| !matches!(d, Drawn::Space);
        let within = |d: &&Drawn| **d != Drawn::Flexible;
        let mut drawn: Vec<Drawn> = Vec::new();
        for (at, d) in shown.iter().enumerate() {
            if *d == Drawn::Space {
                let before = shown[..at]
                    .iter()
                    .rev()
                    .take_while(within)
                    .any(|d| item(&d));
                let after = shown[at + 1..].iter().take_while(within).any(|d| item(&d));
                if !(before && after) {
                    continue;
                }
            }
            drawn.push(*d);
        }
        let hidden = self.entries.iter().any(|entry| match entry {
            ToolbarEntry::Button(button) => fold.hides(button),
            _ => false,
        });
        if hidden {
            let at = drawn.iter().position(|d| *d == Drawn::Flexible);
            drawn.insert(at.unwrap_or(drawn.len()), Drawn::More);
        }
        drawn
    }

    pub fn show(mut self, ui: &mut Ui) -> Option<ToolbarAction> {
        let mut well = self.cursors.take();
        let t = theme(ui.ctx());
        let ctx = ui.ctx().clone();
        let (bar, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), t.sizes.toolbar), Sense::hover());
        let toolbar_name = tr(&ctx, "toolbar");
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
        let keys = |action: ToolbarAction| {
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

        // The save state's slot, at its widest form at each fold, Saving… or the warning,
        // whatever it shows, so a save never moves the fold, the spinner or anything else.
        let saving = SaveSlot::measure(ui);
        // The guards (decision 92): a part the list leaves out that still has something to
        // say shows at the end, after the list: a failed write, then before it the spinner
        // while a failed job is left.
        let save_guard = !self.lists(&ToolbarEntry::SaveState);
        let jobs_guard = !self.lists(&ToolbarEntry::Jobs)
            && (self.jobs.as_ref()).is_some_and(|jobs| jobs.failed());
        let failed = matches!(self.saved, SaveState::Failed(_));
        let guard_save = |fold: Fold| {
            let shows = save_guard && failed;
            if shows {
                saving.width(fold >= Fold::Status)
            } else {
                0.0
            }
        };
        let guard_jobs = if jobs_guard { BUTTON } else { 0.0 };
        // Where the list ends at `fold`: before the guards that show, a group's gap
        // before them.
        let list_end = |fold: Fold| {
            let save = guard_save(fold);
            let guards = save
                + guard_jobs
                + if save > 0.0 && jobs_guard {
                    t.spacing.m
                } else {
                    0.0
                };
            inner.max.x
                - if guards > 0.0 {
                    guards + t.spacing.m
                } else {
                    0.0
                }
        };

        let gap = t.spacing.xxs;
        let width = |drawn: Drawn, fold: Fold| match drawn {
            Drawn::Menu => self.menu_style.width(),
            Drawn::Button(button) if button.words && fold == Fold::None => {
                word_button(button, keys(ToolbarAction::Button(button.id))).width(ui)
            }
            Drawn::Button(_) | Drawn::Jobs | Drawn::More => BUTTON,
            Drawn::Space => t.spacing.xl,
            Drawn::Flexible => 0.0,
            Drawn::Cursors => well.as_ref().map_or(0.0, |well| well.width(ui)),
            Drawn::Save => saving.width(fold >= Fold::Status),
        };
        // Between two neighbours: a space or a flexible space is the whole room between
        // them; the menu button stands a group's gap before the rest, the cursors' well one
        // after what it follows, and the spinner one before the save state.
        let between = |a: Drawn, b: Drawn| match (a, b) {
            (Drawn::Space | Drawn::Flexible, _) | (_, Drawn::Space | Drawn::Flexible) => 0.0,
            (Drawn::Menu, _) => gap + t.spacing.s,
            (_, Drawn::Cursors) => t.spacing.xl,
            (Drawn::Jobs, Drawn::Save) => t.spacing.m,
            _ => gap,
        };
        // The room `drawn` takes but for the flexible spaces' shares.
        let taken = |drawn: &[Drawn], fold: Fold| {
            let widths: f32 = drawn.iter().map(|d| width(*d, fold)).sum();
            let gaps: f32 = drawn.windows(2).map(|w| between(w[0], w[1])).sum();
            widths + gaps
        };

        // How far to fold: the first step whose entries leave the empty panel, measured at
        // the current scale and language as egui lays them out; the cursors' well after a
        // flexible space asks for a group's gap of it.
        let id = response.id.with("fold");
        let last = ui.data(|d| d.get_temp::<Fold>(id)).unwrap_or(Fold::None);
        let fold = Fold::ALL
            .into_iter()
            .find(|&fold| {
                let drawn = self.arrange(fold, well.is_some());
                let after_flexible = drawn
                    .windows(2)
                    .any(|w| w == [Drawn::Flexible, Drawn::Cursors]);
                let well_gap = if after_flexible { t.spacing.xl } else { 0.0 };
                let spare = if fold < last { SPARE } else { 0.0 };
                taken(&drawn, fold) + well_gap + EMPTY + spare <= list_end(fold) - inner.min.x
            })
            .unwrap_or(Fold::Undo);
        ui.data_mut(|d| d.insert_temp(id, fold));
        let drawn = self.arrange(fold, well.is_some());

        // Where each entry goes: those before the first flexible space from the start,
        // those after the last to the end, and each run between two flexible spaces after
        // its share of the room left, its centre on a whole point, so the cursors' well
        // between two stands in the middle of the room between its neighbours.
        let end = list_end(fold);
        let runs: Vec<&[Drawn]> = drawn.split(|d| *d == Drawn::Flexible).collect();
        let flexible = runs.len() - 1;
        let share = ((end - inner.min.x - taken(&drawn, fold)) / flexible.max(1) as f32).max(0.0);
        let mut placed: Vec<(Drawn, f32, f32)> = Vec::new();
        let mut x = inner.min.x;
        for (at, run) in runs.iter().enumerate() {
            let run_width = taken(run, fold);
            if at > 0 && at == flexible {
                x = end - run_width;
            } else if at > 0 {
                x = (x + share + run_width / 2.0).round() - run_width / 2.0;
            }
            for (i, d) in run.iter().enumerate() {
                if i > 0 {
                    x += between(run[i - 1], *d);
                }
                let w = width(*d, fold);
                placed.push((*d, x, w));
                x += w;
            }
        }
        let room =
            |x: f32, w: f32| Rect::from_min_max(pos2(x, inner.top()), pos2(x + w, inner.bottom()));

        let mut menu_drawn = false;
        let mut well_rect = None;
        for (d, x, w) in placed {
            let rect = room(x, w);
            let builder = |salt: &str| UiBuilder::new().id_salt(salt).max_rect(rect);
            let left = |salt: &str| builder(salt).layout(Layout::left_to_right(Align::Center));
            let right = |salt: &str| builder(salt).layout(Layout::right_to_left(Align::Center));
            match d {
                Drawn::Menu => {
                    menu_drawn = true;
                    ui.scope_builder(left("menu button"), |ui| {
                        let (button, press) =
                            menu_button(ui, self.menu_style, keys(ToolbarAction::Menu));
                        pressed(button);
                        match press {
                            Some(MenuPress::Open) => action = Some(ToolbarAction::Menu),
                            Some(MenuPress::Close) => popup::close(&ctx, Toolbar::menu_popup()),
                            None => {}
                        }
                    });
                }
                Drawn::Button(button) => {
                    let done = ToolbarAction::Button(button.id);
                    let response = ui
                        .scope_builder(left(button.id), |ui| {
                            if button.words && fold == Fold::None {
                                word_button(button, keys(done)).show(ui)
                            } else {
                                icon_button(button.icon, &button.name, keys(done))
                                    .size(BUTTON)
                                    .enabled(button.enabled)
                                    .show(ui)
                            }
                        })
                        .inner;
                    if pressed(response) {
                        action = Some(done);
                    }
                }
                Drawn::More => {
                    let hidden: Vec<&ToolbarButton> = (self.entries.iter())
                        .filter_map(|entry| match entry {
                            ToolbarEntry::Button(button) if fold.hides(button) => Some(button),
                            _ => None,
                        })
                        .collect();
                    let chosen = ui
                        .scope_builder(left("more"), |ui| {
                            let more = IconButton::new(Icon::Options, &tr(&ctx, "toolbar-more"))
                                .size(BUTTON)
                                .show(ui);
                            pressed(more.clone());
                            // Each with its keys, and disabled where it can't run. Screen
                            // readers hear its name, and the keys as its shortcut, as on
                            // every control.
                            let name = tr(&ctx, "toolbar-more");
                            pull_down(ui, &more, &name, || {
                                (hidden.iter())
                                    .map(|button| {
                                        let item =
                                            popup::Item::new(button.id, button.name.as_str())
                                                .enabled(button.enabled);
                                        match keys(ToolbarAction::Button(button.id)) {
                                            Some(keys) => item.shortcut(keys),
                                            None => item,
                                        }
                                        .into()
                                    })
                                    .collect()
                            })
                        })
                        .inner;
                    let chosen = chosen.and_then(|chosen| hidden.iter().find(|b| b.id == chosen));
                    if let Some(button) = chosen {
                        action = Some(ToolbarAction::Button(button.id));
                    }
                }
                Drawn::Cursors => {
                    let Some(well) = well.take() else {
                        continue;
                    };
                    let rect = Rect::from_center_size(
                        pos2(rect.center().x, bar.center().y),
                        vec2(w, t.sizes.cursor_well),
                    );
                    let drawn = ui.scope_builder(
                        UiBuilder::new()
                            .id_salt("cursors")
                            .max_rect(rect)
                            .layout(Layout::left_to_right(Align::Center)),
                        |ui| well.show(ui),
                    );
                    if let Some(chosen) = drawn.inner {
                        action = Some(ToolbarAction::Cursors(chosen));
                    }
                    well_rect = Some(rect);
                }
                Drawn::Jobs => {
                    let Some(jobs) = self.jobs else {
                        continue;
                    };
                    ui.scope_builder(right("jobs"), |ui| {
                        if let Some(spinner) = jobs.spinner(ui)
                            && pressed(spinner)
                        {
                            action = Some(ToolbarAction::Jobs);
                        }
                    });
                }
                Drawn::Save => {
                    let folded = fold >= Fold::Status;
                    let save = ui.scope_builder(right("save state"), |ui| {
                        saving.show(ui, self.saved, folded, true)
                    });
                    if let Some(button) = save.inner
                        && pressed(button)
                    {
                        action = Some(ToolbarAction::NotSaved);
                    }
                }
                Drawn::Space | Drawn::Flexible => {}
            }
        }

        // The guards, after the list in the focus order as they stand: the spinner's, then
        // the save state's at the very end, its live region kept whether it shows or not.
        if let Some(jobs) = self.jobs.filter(|_| jobs_guard) {
            let save = guard_save(fold);
            let at = inner.max.x - save - if save > 0.0 { t.spacing.m } else { 0.0 };
            ui.scope_builder(
                UiBuilder::new()
                    .id_salt("jobs")
                    .max_rect(room(at - BUTTON, BUTTON))
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| {
                    if let Some(spinner) = jobs.spinner(ui)
                        && pressed(spinner)
                    {
                        action = Some(ToolbarAction::Jobs);
                    }
                },
            );
        }
        if save_guard {
            let width = guard_save(fold);
            let folded = fold >= Fold::Status;
            let save = ui.scope_builder(
                UiBuilder::new()
                    .id_salt("save state")
                    .max_rect(room(inner.max.x - width, width))
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| saving.show(ui, self.saved, folded, false),
            );
            if let Some(button) = save.inner
                && pressed(button)
            {
                action = Some(ToolbarAction::NotSaved);
            }
        }
        // With the menus drawn here and no menu button listed, they hang from where it
        // would stand, at the panel's start.
        if self.menu && !menu_drawn {
            let start = Rect::from_min_size(
                pos2(inner.min.x, bar.center().y - BUTTON / 2.0),
                vec2(self.menu_style.width(), BUTTON),
            );
            ctx.data_mut(|d| d.insert_temp(menu_anchor_id(), start));
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
                            ToolbarAction::Minimise,
                        ),
                        (icon, middle, false, ToolbarAction::MaximiseOrRestore),
                        (Icon::Close, "window-close", true, ToolbarAction::Close),
                    ] {
                        if pressed(caption_button(ui, icon, &tr(&ctx, id), close, &t)) {
                            action = Some(done);
                        }
                    }
                },
            );
        }
        // The well is no empty area, between its buttons too.
        controls.extend(well_rect);
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

    /// Draws `state` at the right of `ui`, its word left out where `folded`, Saving… only
    /// where `saving` (a guard shows only a failed write), and keeps the live region in the
    /// tree. Returns the save warning's response, where it shows.
    fn show(
        &self,
        ui: &mut Ui,
        state: SaveState<'_>,
        folded: bool,
        saving: bool,
    ) -> Option<egui::Response> {
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
            SaveState::Saving if !saving => None,
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
/// heard on every save. A photo left with its write failed is the Viewer's to tell.
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
        SaveState::Saved => {
            if heard.failed.take().is_some() {
                heard.said = tr(ui.ctx(), "status-saved");
            }
        }
        SaveState::Saving => {}
        SaveState::None => heard.failed = None,
    }
    let said = heard.said.clone();
    ui.data_mut(|d| d.insert_temp(id, heard));
    said
}

/// What a press of the menu button asks for.
enum MenuPress {
    Open,
    /// Its menu is open, hung from it.
    Close,
}

fn menu_anchor_id() -> Id {
    Id::unique("apertrust-ui toolbar menu anchor")
}

/// The menu button in `style`, with `keys` in its tooltip: a button that opens a menu,
/// drawn on while the menu hangs from it. Pressed, or given ↓ while focused, it asks for
/// the menu to open, or to close where it is open.
fn menu_button(
    ui: &mut Ui,
    style: MenuButtonStyle,
    keys: Option<&str>,
) -> (egui::Response, Option<MenuPress>) {
    let ctx = ui.ctx().clone();
    // Where the button goes, the next in its row, as the menu was hung from it.
    let room = ui.available_rect_before_wrap();
    let centre = pos2(room.left() + style.width() / 2.0, room.center().y);
    let open = popup::anchor(&ctx, Toolbar::menu_popup()).is_some_and(|a| a.contains(centre));
    let name = tr(&ctx, "toolbar-menu");
    let response = match style {
        MenuButtonStyle::Menu => {
            let mut button = icon_button(Icon::Menu, &name, keys).size(BUTTON);
            if open {
                button = button.toggled(true);
            }
            button.show(ui)
        }
        MenuButtonStyle::Icon => app_icon_button(ui, &name, keys, open),
    };
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
fn title_bar_gesture(ui: &Ui, bar: Rect, id: egui::Id, controls: &[Rect]) -> Option<ToolbarAction> {
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
        return Some(ToolbarAction::TitleBarDoubleClick);
    }
    // A right-click, on its release as egui's context menus open, where it was let go.
    if menu && at.is_some_and(is_empty) {
        return Some(ToolbarAction::WindowMenu);
    }
    // egui lets go of a press's target once it moves, so where it began is asked of the
    // panel.
    if dragging
        && origin.is_some_and(is_empty)
        && !ui.data(|d| d.get_temp::<bool>(reported).unwrap_or(false))
    {
        ui.data_mut(|d| d.insert_temp(reported, true));
        return Some(ToolbarAction::MoveWindow);
    }
    None
}

/// One of the window's caption buttons, 46 points wide and as tall as `ui` has room for:
/// no corners, `hover` behind it under the pointer, or, for `close`, the close button's
/// red with its own icon colour. The focus ring is drawn inside, since the button meets
/// the window's edges. The Toolbar draws minimise, maximise and close with it, and the
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

/// A button with its words beside its icon, with `keys` in its tooltip when it has any.
fn word_button<'a>(button: &'a ToolbarButton, keys: Option<&'a str>) -> ToolButton<'a> {
    let button = (ToolButton::new(&button.name).icon(button.icon)).enabled(button.enabled);
    match keys {
        Some(keys) => button.shortcut(keys),
        None => button,
    }
}

/// The menu button drawn as the app's icon, then the caret, named `name`, with `keys` in
/// its tooltip, as the Toolbar's icon buttons are drawn (decision 81): no plate,
/// `text_secondary` at rest, `text_strong` under the pointer and while its menu is `open`,
/// and high contrast's `button_edge` round it under the pointer.
fn app_icon_button(ui: &mut Ui, name: &str, keys: Option<&str>, open: bool) -> egui::Response {
    let t = theme(ui.ctx());
    let width = MenuButtonStyle::Icon.width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = &undimmed_painter(ui);
        let touch = Touch::of(ui.ctx(), &response, true, None);
        // No plate, as the icon buttons; high contrast outlines it under the pointer.
        if (touch.hovered || touch.pressed) && t.colors.button_edge.a() > 0 {
            painter.rect_stroke(
                rect,
                t.shape.radius,
                Stroke::new(t.shape.hairline, t.colors.button_edge),
                StrokeKind::Inside,
            );
        }
        let colour = colour(&t, touch, open);
        let mut x = rect.left() + ICON_PADDING;
        for (icon, gap) in [(Icon::AppIcon, CARET_GAP), (Icon::Caret, 0.0)] {
            let size = icon.size();
            let at = pos2(x, rect.center().y - size / 2.0);
            icon.paint(painter, Rect::from_min_size(at, vec2(size, size)), colour);
            x += size + gap;
        }
        paint_focus(painter, rect, t.shape.radius, touch, &t);
    }
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    let tip = match keys {
        Some(keys) => {
            report_shortcut(ui, response.id, keys);
            format!("{name} ({keys})")
        }
        None => name.to_owned(),
    };
    response.on_hover_text(tip)
}

/// An icon button, with `keys` in its tooltip when it has any.
fn icon_button<'a>(icon: Icon, name: &'a str, keys: Option<&'a str>) -> IconButton<'a> {
    let button = IconButton::new(icon, name);
    match keys {
        Some(keys) => button.shortcut(keys),
        None => button,
    }
}
