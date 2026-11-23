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
//! ([`SaveState`]), which shows only when a save goes wrong. Its buttons show their words
//! under their icons while labels are on ([`Toolbar::labels`]). The entries before the first
//! flexible space stand at the start, those after the last at the end, and those between
//! two in the middle of the room the flexible spaces share. The spinner and the save state
//! each sit in a slot of its own, as wide as its widest form at the fold, kept whether it
//! shows anything or not, so neither ever moves anything. Each tooltip carries its
//! action's keys, which the app's actions registry gives. As it narrows it gives way one
//! step at a time ([`Fold`]), its labels first, then its buttons going under »
//! (`toolbar-more`), and its parts never, always keeping empty panel to move the window by.
//!
//! **Guards** (decision 92): with no menu button in the list, the menus hang from the
//! Toolbar's start ([`Toolbar::menu_anchor`]); with no save state, a failed write still
//! shows its warning at the end, and its live region stays; with no jobs, the spinner
//! still shows at the end while a failed job is left. With no entries at all, the whole
//! panel is empty area.
//!
//! **The toolbar menu** (decision 92): a right-click anywhere on the panel, on an entry or
//! its empty areas, title bar or not, but the caption buttons and, on the title bar, the
//! menu button, asks for it with the entry under the pointer, as do the menu key, Shift+F10 and a screen reader's
//! ShowContextMenu on a focused button, the menu button, the spinner or a face or chevron
//! of the well ([`ToolbarAction::ContextMenu`]); the app builds it. Where the panel is the
//! title bar, a right-click on the menu button asks for the window's menu instead, as on a
//! title bar's icon ([`ToolbarAction::WindowMenu`]), which also stays on the system's own
//! key.
//!
//! It runs to the window's top edge and is the window's title bar ([`Toolbar::title_bar`];
//! `docs/gui.md`, The window frame and menus, decision 39): it keeps room at its start for
//! the window's own buttons where the system draws them there, as macOS does
//! ([`Toolbar::start_inset`]), and reports a drag starting or a double-click on its empty
//! areas, the same on every platform, for the app to move the window or do what the
//! system does on a double-click. It
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
    CursorWell, CursorWellAction, IconButton, Jobs, focus_ring, menu_asked, menu_asked_of,
    pull_down, report_shortcut,
};
use crate::i18n::Args;
use crate::{Icon, popup, theme, tr, translator};

/// What the user did to the Toolbar this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolbarAction {
    /// The menu button ([`Toolbar::menu_button`]), pressed or given ↓ while its menu is
    /// closed: the menus should open, hung from [`Toolbar::menu_anchor`] as the popup
    /// [`Toolbar::menu_popup`].
    Menu,
    /// The button of this id ([`ToolbarButton::new`]), pressed on the panel or chosen
    /// under ».
    Button(&'static str),
    /// An empty part of the panel pressed and dragged, where the panel is the title bar:
    /// the window should follow the pointer. Reported once a press.
    MoveWindow,
    /// An empty part of the panel double-clicked, where the panel is the title bar: the
    /// window should do what a double-click on its title bar does.
    TitleBarDoubleClick,
    /// The menu button right-clicked, where the panel is the title bar: the window's menu
    /// should open at the pointer, as on a title bar's icon (decision 92).
    WindowMenu,
    /// The toolbar menu asked for (decision 92): by a right-click anywhere on the panel but
    /// its caption buttons and, on the title bar, the menu button, on an entry or its empty
    /// areas, or a Control-click on the Mac, or by the menu key, Shift+F10 or a screen reader's
    /// ShowContextMenu on a focused button.
    ContextMenu(ToolbarMenu),
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

/// Where the toolbar menu was asked for ([`ToolbarAction::ContextMenu`]), for the app to
/// build it: on which entry, where it hangs from, and the control the focus goes back to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolbarMenu {
    /// The entry it was asked on, by its place in [`Toolbar::entries`]: a button, the menu
    /// button, the cursors' well, the spinner or the save state. None on the empty areas
    /// and ».
    pub entry: Option<usize>,
    /// Where it hangs from, in points: the pointer, or under the button from the keys.
    pub at: Pos2,
    /// The button the focus goes back to as it closes, asked for from the keys; none from
    /// the pointer, the focus going back to what had it.
    pub opener: Option<Id>,
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
    /// only between two entries drawn, none of them a flexible space. As the panel
    /// narrows, the fixed spaces narrow to 8 points, then go one at a time from the end.
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

/// A button of the Toolbar that runs an action: its icon, its translated name, its word
/// under its icon while labels are on, and whether it can run now, drawn disabled where it
/// cannot.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolbarButton {
    id: &'static str,
    icon: Icon,
    name: String,
    label: Option<String>,
    enabled: bool,
    first: bool,
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
            label: None,
            enabled: true,
            first: false,
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

    /// Its word under its icon while the Toolbar shows labels ([`Toolbar::labels`]),
    /// translated, short and on one line: the action's `.toolbar` word (decision 92). Its
    /// name where none is given.
    pub fn label(mut self, word: impl Into<String>) -> Self {
        self.label = Some(word.into());
        self
    }

    /// The word under its icon.
    fn word(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }

    /// Goes under » before the buttons that do not, as a tool's items do (decision 92).
    pub fn folds_first(mut self, first: bool) -> Self {
        self.first = first;
        self
    }

    /// Goes under » only at the narrowing's last step, after every other button and the
    /// status words, as Undo and Redo do.
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
    focus: Vec<usize>,
    labels: bool,
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
/// A labelled button's icon's square below the panel's top, its side, and its word below
/// it.
const LABEL_TOP: f32 = 6.0;
const LABEL_ICON: f32 = 16.0;
const LABEL_GAP: f32 = 3.0;

/// How far the panel gives way to its width (decision 92; `docs/gui.md`, The Toolbar,
/// Narrowing): the overflow's steps, taken in this order, one at a time
/// ([`Toolbar::steps`]). The parts never fold, the menu button among them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Fold {
    /// The buttons' labels fold, all at once.
    labels: bool,
    /// The fixed spaces narrow to 8 points.
    narrow: bool,
    /// How many fixed spaces have gone, from the list's end.
    spaces: usize,
    /// How many buttons are under », in the order they go ([`Toolbar::fold_order`]).
    actions: usize,
    /// The save state folds to its sign.
    status: bool,
    /// The buttons that fold last, Undo and Redo, are under » too.
    last: bool,
}

/// What the panel draws at a fold, in order: the entries that show, and » where buttons
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

    /// Gives the focus to the first of `entries`, by their places in the list, that it draws
    /// able to take it: an enabled button not under », the menu button or the spinner. None
    /// by default; the app asks for it for one frame, as after Hide from the keys.
    pub fn focus(mut self, entries: Vec<usize>) -> Self {
        self.focus = entries;
        self
    }

    /// Draws each button's word under its icon (decision 92, Toolbar Labels): the item as
    /// wide as its word or its icon, the wider, with 8 points at each side, the panel as
    /// tall as without. The overflow's first step folds them all at once. Off by default.
    pub fn labels(mut self, on: bool) -> Self {
        self.labels = on;
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

    /// The overflow's steps, from nothing folded to all that folds (decision 92): the
    /// labels fold, where they show; the fixed spaces narrow to 8 points, then go one at a
    /// time from the end; the buttons go under » one at a time ([`Toolbar::fold_order`]);
    /// the status words fold; and last the buttons that fold last, Undo and Redo,
    /// together.
    fn steps(&self) -> Vec<Fold> {
        let spaces = (self.entries.iter())
            .filter(|entry| **entry == ToolbarEntry::Space)
            .count();
        let mut fold = Fold::default();
        let mut steps = vec![fold];
        if self.labels {
            fold.labels = true;
            steps.push(fold);
        }
        fold.narrow = true;
        steps.push(fold);
        for _ in 0..spaces {
            fold.spaces += 1;
            steps.push(fold);
        }
        for _ in self.fold_order() {
            fold.actions += 1;
            steps.push(fold);
        }
        fold.status = true;
        steps.push(fold);
        fold.last = true;
        steps.push(fold);
        steps
    }

    /// The buttons that go under » before the status words fold, by their place in the
    /// list, in the order they go: in reverse reading order, those that fold first, a
    /// tool's items, before the rest.
    fn fold_order(&self) -> Vec<usize> {
        let buttons = |first: bool| {
            (self.entries.iter().enumerate().rev()).filter_map(move |(at, entry)| match entry {
                ToolbarEntry::Button(button) if !button.last && button.first == first => Some(at),
                _ => None,
            })
        };
        buttons(true).chain(buttons(false)).collect()
    }

    /// Whether each entry has gone at `fold`, by its place in the list: a button under »,
    /// or a fixed space removed.
    fn folded(&self, fold: Fold) -> Vec<bool> {
        let mut folded: Vec<bool> = (self.entries.iter())
            .map(|entry| matches!(entry, ToolbarEntry::Button(b) if b.last && fold.last))
            .collect();
        for at in self.fold_order().into_iter().take(fold.actions) {
            folded[at] = true;
        }
        let spaces = (self.entries.iter().enumerate().rev())
            .filter(|(_, entry)| **entry == ToolbarEntry::Space)
            .take(fold.spaces);
        for (at, _) in spaces {
            folded[at] = true;
        }
        folded
    }

    /// The buttons under » at `fold`, in the list's order.
    fn under_more(&self, fold: Fold) -> Vec<&ToolbarButton> {
        (self.entries.iter().zip(self.folded(fold)))
            .filter_map(|(entry, folded)| match entry {
                ToolbarEntry::Button(button) if folded => Some(button),
                _ => None,
            })
            .collect()
    }

    /// What the panel draws at `fold`, `well` the cursors' well given, in order: the
    /// entries that show, a fixed space only between two of them with no flexible space
    /// between, and », where any button is under it, just before the first flexible
    /// space, or last with none.
    fn arrange(&self, fold: Fold, well: bool) -> Vec<Drawn<'_>> {
        let folded = self.folded(fold);
        let shown = (self.entries.iter().zip(&folded)).filter_map(|(entry, folded)| match entry {
            _ if *folded => None,
            ToolbarEntry::MenuButton => self.menu.then_some(Drawn::Menu),
            ToolbarEntry::Button(button) => Some(Drawn::Button(button)),
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
        let hidden = (self.entries.iter().zip(&folded))
            .any(|(entry, folded)| *folded && matches!(entry, ToolbarEntry::Button(_)));
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
                saving.width(fold.status)
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
        let labelled = |fold: Fold| self.labels && !fold.labels;
        let width = |drawn: Drawn, fold: Fold| match drawn {
            Drawn::Menu => self.menu_style.width(),
            Drawn::Button(button) if labelled(fold) => Labelled::of(ui, button).width,
            Drawn::Button(_) | Drawn::Jobs | Drawn::More => BUTTON,
            Drawn::Space if fold.narrow => t.spacing.m,
            Drawn::Space => t.spacing.xl,
            Drawn::Flexible => 0.0,
            Drawn::Cursors => well.as_ref().map_or(0.0, |well| well.width(ui)),
            Drawn::Save => saving.width(fold.status),
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
        // flexible space asks for a group's gap of it. A step before the last frame's
        // needs room to spare.
        let id = response.id.with("fold");
        let last = ui.data(|d| d.get_temp::<usize>(id)).unwrap_or(0);
        let steps = self.steps();
        let (step, fold) = (steps.iter().copied().enumerate())
            .find(|&(step, fold)| {
                let drawn = self.arrange(fold, well.is_some());
                let after_flexible = drawn
                    .windows(2)
                    .any(|w| w == [Drawn::Flexible, Drawn::Cursors]);
                let well_gap = if after_flexible { t.spacing.xl } else { 0.0 };
                let spare = if step < last { SPARE } else { 0.0 };
                taken(&drawn, fold) + well_gap + EMPTY + spare <= list_end(fold) - inner.min.x
            })
            .unwrap_or((steps.len() - 1, steps[steps.len() - 1]));
        ui.data_mut(|d| d.insert_temp(id, step));
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

        // Where each entry drawn stands, by its place in the list, for the toolbar menu
        // asked for on it; and the menu asked for from the keys on a focused button.
        let entry_of = |d: Drawn| {
            (self.entries.iter()).position(|entry| match (d, entry) {
                (Drawn::Button(drawn), ToolbarEntry::Button(listed)) => drawn.id == listed.id,
                (Drawn::Menu, ToolbarEntry::MenuButton)
                | (Drawn::Cursors, ToolbarEntry::Cursors)
                | (Drawn::Jobs, ToolbarEntry::Jobs)
                | (Drawn::Save, ToolbarEntry::SaveState) => true,
                _ => false,
            })
        };
        let mut items: Vec<(Option<usize>, Rect)> = Vec::new();
        let mut keyed = None;
        // The entries drawn that can take the focus, by their places in the list.
        let mut focusable: Vec<(Option<usize>, Id)> = Vec::new();
        let mut menu_drawn = false;
        let mut well_rect = None;
        let mut more_rect = None;
        for (d, x, w) in placed {
            let rect = room(x, w);
            if let Drawn::Menu | Drawn::Button(_) = d {
                items.push((entry_of(d), rect));
            }
            let builder = |salt: &str| UiBuilder::new().id_salt(salt).max_rect(rect);
            let left = |salt: &str| builder(salt).layout(Layout::left_to_right(Align::Center));
            let right = |salt: &str| builder(salt).layout(Layout::right_to_left(Align::Center));
            match d {
                Drawn::Menu => {
                    menu_drawn = true;
                    ui.scope_builder(left("menu button"), |ui| {
                        let (button, press) =
                            menu_button(ui, self.menu_style, keys(ToolbarAction::Menu));
                        if menu_asked(&ctx, &button) {
                            keyed = Some(keyed_menu(entry_of(d), &button));
                        }
                        focusable.push((entry_of(d), button.id));
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
                            if labelled(fold) {
                                Labelled::of(ui, button).show(ui, bar, keys(done))
                            } else {
                                icon_button(button.icon, &button.name, keys(done))
                                    .size(BUTTON)
                                    .enabled(button.enabled)
                                    .show(ui)
                            }
                        })
                        .inner;
                    if menu_asked(&ctx, &response) {
                        keyed = Some(keyed_menu(entry_of(d), &response));
                    }
                    if button.enabled {
                        focusable.push((entry_of(d), response.id));
                    }
                    if pressed(response) {
                        action = Some(done);
                    }
                }
                Drawn::More => {
                    let hidden = self.under_more(fold);
                    let chosen = ui
                        .scope_builder(left("more"), |ui| {
                            let more = IconButton::new(Icon::More, &tr(&ctx, "toolbar-more"))
                                .size(BUTTON)
                                .show(ui);
                            pressed(more.clone());
                            more_rect = Some(more.rect);
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
                    // Its Tab stop, the chosen cursor's face or the first, takes the focus
                    // asked for as a button does: the one target of the well that can.
                    let stop = ctx.viewport(|v| {
                        (v.this_pass.widgets.get_layer(ui.layer_id()))
                            .find(|w| {
                                w.enabled && w.sense.is_focusable() && rect.contains_rect(w.rect)
                            })
                            .map(|w| w.id)
                    });
                    if let Some(stop) = stop {
                        focusable.push((entry_of(d), stop));
                    }
                    // The menu keys on a focused face or chevron ask for the menu on the
                    // well, as on a button.
                    let focused = ctx.memory(|m| m.focused()).and_then(|id| {
                        let face = ctx.viewport(|v| v.this_pass.widgets.get(id).copied())?;
                        rect.contains_rect(face.rect).then_some(face)
                    });
                    if let Some(face) = focused
                        && menu_asked_of(&ctx, face.id, true)
                    {
                        keyed = Some(ToolbarMenu {
                            entry: entry_of(d),
                            at: face.rect.left_bottom(),
                            opener: Some(face.id),
                        });
                    }
                    items.push((entry_of(d), rect));
                    well_rect = Some(rect);
                }
                Drawn::Jobs => {
                    let Some(jobs) = self.jobs else {
                        continue;
                    };
                    ui.scope_builder(right("jobs"), |ui| {
                        if let Some(spinner) = jobs.spinner(ui) {
                            if menu_asked(&ctx, &spinner) {
                                keyed = Some(keyed_menu(entry_of(d), &spinner));
                            }
                            focusable.push((entry_of(d), spinner.id));
                            items.push((entry_of(d), spinner.rect));
                            if pressed(spinner) {
                                action = Some(ToolbarAction::Jobs);
                            }
                        }
                    });
                }
                Drawn::Save => {
                    let save = ui.scope_builder(right("save state"), |ui| {
                        saving.show(ui, self.saved, fold.status, true)
                    });
                    if let Some(button) = save.inner {
                        items.push((entry_of(d), button.rect));
                        if pressed(button) {
                            action = Some(ToolbarAction::NotSaved);
                        }
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
            let save = ui.scope_builder(
                UiBuilder::new()
                    .id_salt("save state")
                    .max_rect(room(inner.max.x - width, width))
                    .layout(Layout::right_to_left(Align::Center)),
                |ui| saving.show(ui, self.saved, fold.status, false),
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
        // The focus asked for, on the first of those entries that can take it.
        let wanted = (self.focus.iter()).find_map(|at| {
            let found = focusable.iter().find(|(entry, _)| *entry == Some(*at));
            found.map(|(_, id)| *id)
        });
        if let Some(id) = wanted {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        // The toolbar menu, asked for on whatever is under the pointer or from the keys,
        // in place of a press its Control-click on the Mac made; never on the caption
        // buttons, which are the platform's, nor a Control-click on » or the well, whose
        // own menus that click opens.
        // Where the panel is the title bar, a right-click on the menu button is the window's
        // menu, as on a title bar's icon; its keys still ask for the toolbar menu.
        let menu_button = (self.entries.iter()).position(|e| *e == ToolbarEntry::MenuButton);
        let mut window_menu = false;
        let menu = keyed.or_else(|| {
            let (at, control) = menu_click(ui, bar)?;
            let owns = |rect: Option<Rect>| rect.is_some_and(|r| r.contains(at));
            if owns(caption) || (control && (owns(more_rect) || owns(well_rect))) {
                return None;
            }
            let entry = items.iter().find(|(_, rect)| rect.contains(at));
            let entry = entry.and_then(|(entry, _)| *entry);
            if self.title_bar && !control && entry.is_some() && entry == menu_button {
                window_menu = true;
                return None;
            }
            Some(ToolbarMenu {
                entry,
                at,
                opener: None,
            })
        });
        if let Some(menu) = menu {
            action = Some(ToolbarAction::ContextMenu(menu));
        } else if window_menu {
            action = Some(ToolbarAction::WindowMenu);
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

/// The toolbar menu asked for from the keys on the focused `response`, the entry at `entry`:
/// under it, the focus going back to it as it closes.
fn keyed_menu(entry: Option<usize>, response: &egui::Response) -> ToolbarMenu {
    ToolbarMenu {
        entry,
        at: response.rect.left_bottom(),
        opener: Some(response.id),
    }
}

/// Where a right-click on the panel, `bar`, was let go this frame, as egui's context menus
/// open, or on the Mac a click with Control held (decision 91), and whether it was that
/// click: on the panel, under no popup over it, whatever is there.
fn menu_click(ui: &Ui, bar: Rect) -> Option<(Pos2, bool)> {
    let mac = ui.ctx().os() == egui::os::OperatingSystem::Mac;
    let at = ui.input(|i| {
        let pointer = &i.pointer;
        // Control read from the button's coming up, as `context_click` reads it.
        let control = || {
            i.events.iter().any(|event| {
                matches!(event, egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                    ..
                } if modifiers.ctrl)
            })
        };
        let control = mac && pointer.button_clicked(egui::PointerButton::Primary) && control();
        let right = pointer.button_clicked(egui::PointerButton::Secondary);
        let at = (right || control)
            .then(|| pointer.interact_pos())
            .flatten()?;
        Some((at, control && !right))
    })?;
    let on_bar = bar.contains(at.0) && ui.ctx().layer_id_at(at.0) == Some(ui.layer_id());
    on_bar.then_some(at)
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
/// double-click, or a press there that has turned into a drag, reported
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
    let (pressed, down, dragging, double, at, origin) = ui.input(|i| {
        let pointer = &i.pointer;
        let down = pointer.primary_down();
        (
            pointer.primary_pressed(),
            down,
            down && pointer.is_decidedly_dragging(),
            pointer.button_double_clicked(egui::PointerButton::Primary),
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

/// A button with its word under its icon (decision 92, Labels), as the Tools panel's tab
/// names are set: in the panel, 6 points below its top, the icon in `text_secondary` in a
/// 16-point square, so every word stands on one line, 3 points, then the word, centred, at
/// the caption size, Regular, in `text`. Otherwise an
/// icon button (decision 81): no plate, the icon `text_strong` under the pointer and
/// `accent_text` held down, high contrast's `button_edge` round it under the pointer, the
/// focus ring after a key.
struct Labelled<'b> {
    button: &'b ToolbarButton,
    word: std::sync::Arc<egui::Galley>,
    /// As wide as its word or its icon, the wider, with 8 points at each side.
    width: f32,
}

impl<'b> Labelled<'b> {
    fn of(ui: &Ui, button: &'b ToolbarButton) -> Self {
        let t = theme(ui.ctx());
        let font = FontId::proportional(t.text.caption);
        let text = button.word().to_owned();
        // Its colour given as it is drawn.
        let word = (ui.painter()).layout_no_wrap(text, font, egui::Color32::PLACEHOLDER);
        let width = word.size().x.max(LABEL_ICON) + 2.0 * t.spacing.m;
        Labelled {
            button,
            word,
            width,
        }
    }

    /// Draws it at the start of `ui`, the panel being `bar`, with `keys` in its tooltip.
    fn show(self, ui: &mut Ui, bar: Rect, keys: Option<&str>) -> egui::Response {
        let t = theme(ui.ctx());
        let button = self.button;
        let enabled = button.enabled && ui.is_enabled();
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        // The panel's height but for a margin above and below, so the focus ring shows.
        let at = ui.available_rect_before_wrap().left();
        let edge = t.spacing.xxs;
        let rect = Rect::from_x_y_ranges(
            at..=at + self.width,
            bar.top() + edge..=bar.bottom() - t.shape.separator_width - edge,
        );
        let response = ui.allocate_rect(rect, sense);
        if ui.is_rect_visible(rect) {
            let painter = &undimmed_painter(ui);
            let touch = Touch::of(ui.ctx(), &response, enabled, None);
            if touch.enabled && (touch.hovered || touch.pressed) && t.colors.button_edge.a() > 0 {
                painter.rect_stroke(
                    rect,
                    t.shape.radius,
                    Stroke::new(t.shape.hairline, t.colors.button_edge),
                    StrokeKind::Inside,
                );
            }
            let square = Rect::from_min_size(
                pos2(rect.center().x - LABEL_ICON / 2.0, bar.top() + LABEL_TOP),
                Vec2::splat(LABEL_ICON),
            );
            let icon = Rect::from_center_size(square.center(), Vec2::splat(button.icon.size()));
            button.icon.paint(painter, icon, colour(&t, touch, false));
            let word = pos2(
                rect.center().x - self.word.size().x / 2.0,
                square.bottom() + LABEL_GAP,
            );
            let text = if enabled {
                t.colors.text
            } else {
                t.colors.text_disabled
            };
            painter.galley(word, self.word, text);
            paint_focus(painter, rect, t.shape.radius, touch, &t);
        }
        let name = button.name.as_str();
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, name));
        let tip = match keys {
            Some(keys) => {
                report_shortcut(ui, response.id, keys);
                format!("{name} ({keys})")
            }
            None => name.to_owned(),
        };
        response.on_hover_text(tip)
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
