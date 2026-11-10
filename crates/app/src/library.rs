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

//! What the open folder looks like (`docs/gui.md`, The side panel, The central panel's
//! states): the Photos tab's Browse section in folder mode's form (decision 73), its top
//! line with the recents and ＋, its Folders group the open folder's tree, and the empty
//! states of the central panel and the grid panel.
//! Both draw from the mirror's [`Folder`] and say what the person asked for as [`Action`]s,
//! which the app carries out.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use apertrust_ui::components::{
    Button, ChoiceBox, CollapsibleGroup, IconButton, Section, SectionAction, context_menu,
    focus_ring_centred, group_label, middle_cut, pull_down,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::popup::{self, Entry, Item, Submenu};
use apertrust_ui::{Icon, theme, tr, translator};
use eframe::egui::{self, Align, Layout, Rect, Sense, UiBuilder, pos2, vec2};

use crate::folder::Folder;
use crate::{export, jobs};

/// Browse's id (decision 73), by which settings keep whether its section is open, and
/// under which they keep its groups.
pub const ID: &str = "browse";

/// Whether Browse is open when settings have not kept it otherwise.
pub const OPEN: bool = true;

/// The Folders group's id inside Browse.
pub const FOLDERS: &str = "folders";

/// Whether Folders is open when settings have not kept it otherwise: in folder mode it is
/// Browse's one group.
pub const FOLDERS_OPEN: bool = true;

/// Whether Browse and its Folders are open, as settings keep them.
#[derive(Clone, Copy, Debug)]
pub struct Open {
    pub browse: bool,
    pub folders: bool,
}

/// What the person asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// File › Open Folder…'s dialog.
    OpenFolder,
    /// File › Open…'s dialog.
    OpenFiles,
    /// A folder, a subfolder of the open one, its parent or a recent one, opened at once.
    Open(PathBuf),
    /// Browse's header: open or close it.
    Toggle,
    /// The Folders group's row: open or close it.
    ToggleFolders,
    /// A folder's row in the tree: show its subfolders under it, or hide them.
    ToggleTree(PathBuf),
    /// A folder open in the tree whose subfolders are not listed: ask the library, once an
    /// open (`FolderState::ask_inside`).
    Ask(PathBuf),
    /// Try Again under a folder whose subfolders could not be listed.
    Retry(PathBuf),
    /// Show in Finder, Browse's option: the folder opened in the platform's file manager.
    Reveal(PathBuf),
    /// Show in Folder, a folder row's: the folder shown in its parent.
    ShowInFolder(PathBuf),
    /// Remove from the List: a missing recent taken out of the recents.
    Remove(PathBuf),
    /// Locate…: the platform's folder dialog, asking where a missing recent went.
    Locate(PathBuf),
}

/// A recent folder, with why it cannot be opened, if it cannot.
#[derive(Clone, Copy, Debug)]
pub struct Recent<'a> {
    pub path: &'a Path,
    pub missing: Option<Missing<'a>>,
}

/// Why a recent folder is drawn disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing<'a> {
    /// Its opening failed this run, for this reason.
    Failed(&'a str),
    /// A check of the recents found it gone ([`crate::recents`]).
    Gone,
}

/// The keys of the two ways to open, as the platform writes them, where the keymap has any.
#[derive(Clone, Debug, Default)]
pub struct Keys {
    pub folder: Option<String>,
    pub files: Option<String>,
}

/// What the central panel says while no photo is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Empty<'a> {
    /// Nothing has been opened: where to start.
    Start,
    /// The folder is being read.
    Reading(&'a Path),
    /// It could not be read, or its reading was cancelled (`None`).
    Failed(&'a Path, Option<&'a str>),
    /// It has no raws.
    NoRaws(&'a str),
    /// It has photos, and none is open.
    NoneChosen,
}

impl<'a> Empty<'a> {
    /// What `folder` makes of an empty central panel.
    pub fn of(folder: &'a Folder) -> Empty<'a> {
        match folder {
            Folder::None => Empty::Start,
            Folder::Opening { path } => Empty::Reading(path),
            Folder::Failed { path, reason } => Empty::Failed(path, reason.as_deref()),
            Folder::Open(listing) if !listing.done => Empty::Reading(&listing.path),
            Folder::Open(listing) if listing.photos.is_empty() => Empty::NoRaws(&listing.name),
            Folder::Open(_) | Folder::Alone { .. } => Empty::NoneChosen,
        }
    }

    /// What the grid panel says with it: "No raw photos in …" where the folder has none, and
    /// nothing else, since it is empty in every other state, as gui.md has it.
    pub fn grid_says(&self, ctx: &egui::Context) -> Option<String> {
        match self {
            Empty::NoRaws(name) => Some(with_folder(ctx, "folder-empty", name)),
            _ => None,
        }
    }
}

fn with_folder(ctx: &egui::Context, id: &str, folder: &str) -> String {
    let mut args = Args::new();
    args.set("folder", folder);
    translator(ctx).text_with(id, Some(&args))
}

/// A folder's last part, or the whole path where it has none.
fn name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// One folder in a list: a line, its name cut with … where it is too long, the rows' `hover`
/// band under the pointer and the rows' focus ring, both 4 points beyond it at each end; the
/// whole path in its tooltip, disabled with why in its tooltip when it cannot be opened.
fn row(ui: &mut egui::Ui, label: &str, tip: &str, missing: Option<Missing<'_>>) -> bool {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let response = ui
        .add_enabled_ui(missing.is_none(), |ui| {
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let band = rect.expand2(vec2(t.spacing.xs, 0.0));
                if response.hovered() && ui.is_enabled() {
                    painter.rect_filled(band, t.shape.radius, t.colors.hover);
                }
                let font = egui::FontId::proportional(t.text.body);
                let mut job =
                    egui::text::LayoutJob::simple_singleline(label.to_owned(), font, t.colors.text);
                job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width());
                let galley = painter.layout_job(job);
                let at = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
                painter.galley(at, galley, t.colors.text);
                if response.has_focus() {
                    focus_ring_centred(painter, band, &t);
                }
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label)
            });
            response
        })
        .inner;
    match missing {
        Some(why) => {
            let words = translator(ui.ctx());
            let text = match why {
                Missing::Failed(reason) => {
                    let mut args = Args::new();
                    args.set("reason", reason);
                    words.text_with("folders-recent-missing", Some(&args))
                }
                Missing::Gone => words.text("folders-recent-gone"),
            };
            response.on_disabled_hover_text(format!("{tip}\n{text}"));
            false
        }
        None => response.on_hover_text(tip).clicked(),
    }
}

/// What Browse draws from: the folder open, the recents, which of its parts are open and
/// the keys of the two ways to open.
pub struct Browse<'a> {
    pub folder: &'a Folder,
    /// The recents' folders, newest first.
    pub recent: &'a [Recent<'a>],
    pub open: Open,
    /// The folders the person opened (true) or closed (false) in the tree this run; the open
    /// folder starts open and the others closed.
    pub tree: &'a BTreeMap<PathBuf, bool>,
    /// The subfolders of folders inside the open one as the library listed them, by path, or
    /// why it could not.
    pub inside: &'a HashMap<PathBuf, Result<Vec<String>, String>>,
    pub keys: &'a Keys,
    /// The person's home folder, which tooltips write as `~`.
    pub home: Option<&'a Path>,
}

/// How far each level of the tree is set in (`docs/gui.md`, Names).
const LEVEL_INDENT: f32 = 12.0;

/// The side of a tree row's folder icon.
const ROW_ICON: f32 = 16.0;

/// The folder the tree is about: its path, its photo count where the listing has given
/// every photo, and its subfolders' names.
fn shown(folder: &Folder) -> Option<(&Path, Option<usize>, &[String])> {
    match folder {
        Folder::Open(listing) => Some((
            &listing.path,
            listing.done.then_some(listing.photos.len()),
            &listing.subfolders,
        )),
        Folder::Opening { path } => Some((path, None, &[])),
        Folder::None | Folder::Failed { .. } | Folder::Alone { .. } => None,
    }
}

/// The Photos tab's Browse section (decision 73), in folder mode's form: its top line,
/// what is open, a box listing the recents to choose from, with ＋ beside it; then its
/// Folders group, the open folder's tree with the way up to its parent; ⋯ holds Show in
/// Finder. Each part is open or closed as `browse.open` says. Albums and Smart albums join
/// Folders with the library (Phase 5); folder mode never shows them.
///
/// For a photo alone, as the wireframe's one-photo column draws it, the top line names the
/// photo, Folders holds Open This Folder alone, and ⋯ holds Open This Folder, then Show in
/// Finder, which shows the photo in its folder.
pub fn browse(ui: &mut egui::Ui, browse: &Browse<'_>) -> Vec<Action> {
    let t = theme(ui.ctx());
    let words = translator(ui.ctx());
    let mut actions = Vec::new();
    let title = words.text("section-browse");
    let open = shown(browse.folder).map(|(path, ..)| path.to_owned());
    let alone = alone(browse.folder);
    let reveal = Item::new("reveal", words.text(jobs::FILE_MANAGER.show_message()))
        .enabled(open.is_some() || alone.is_some());
    let mut options = vec![Entry::Item(reveal)];
    if alone.is_some() {
        let this = Item::new("open-this-folder", words.text("browse-open-this-folder"));
        options.insert(0, Entry::Item(this));
    }
    let out = Section::new(&title, browse.open.browse)
        .options_menu(options)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(t.spacing.xs);
            let margin = t.spacing.l;
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(margin as i8, 0))
                .show(ui, |ui| {
                    top_line(ui, browse, &mut actions);
                    let group = words.text("browse-folders");
                    let toggled = CollapsibleGroup::new(&group, browse.open.folders)
                        .show(ui, |ui| tree(ui, browse, &mut actions));
                    if toggled {
                        actions.push(Action::ToggleFolders);
                    }
                    ui.add_space(t.spacing.m);
                });
        });
    if out.actions.contains(&SectionAction::Toggle) {
        actions.push(Action::Toggle);
    }
    match (out.chosen.as_deref(), open, alone) {
        (Some("reveal"), Some(path), _) => actions.push(Action::Reveal(path)),
        (Some("reveal"), None, Some((photo, _))) => {
            actions.push(Action::ShowInFolder(photo.to_owned()));
        }
        (Some("open-this-folder"), _, Some((_, folder))) => {
            actions.push(Action::Open(folder.to_owned()));
        }
        _ => {}
    }
    actions
}

/// The photo open alone and its folder, which Open This Folder opens.
fn alone(folder: &Folder) -> Option<(&Path, &Path)> {
    match folder {
        Folder::Alone { path, .. } => Some((path, path.parent()?)),
        _ => None,
    }
}

/// What the top line's box says, as the wireframe draws it: the open folder, "Folder:
/// 2026-10 Lisbon", its name cut in its middle to fit `width` in `font`, its count on its
/// row in the tree; that it is being read; the name of a photo alone; or, with none open
/// or the last open failed, that none is. Returns it cut, and whole.
fn open_line(
    ctx: &egui::Context,
    painter: &egui::Painter,
    folder: &Folder,
    font: &egui::FontId,
    width: f32,
) -> (String, String) {
    let words = translator(ctx);
    let line = |name: &str| -> String {
        let mut args = Args::new();
        args.set("name", name);
        match folder {
            Folder::Open(listing) if listing.done => {
                words.text_with("browse-open-folder", Some(&args))
            }
            Folder::Open(_) | Folder::Opening { .. } => {
                words.text_with("folders-reading", Some(&args))
            }
            Folder::Alone { .. } => name.to_owned(),
            Folder::None | Folder::Failed { .. } => words.text("browse-no-folder"),
        }
    };
    let whole_name = match folder {
        Folder::Open(listing) => listing.name.clone(),
        Folder::Opening { path } => name(path),
        Folder::Alone { photo, .. } => photo.name.clone(),
        Folder::None | Folder::Failed { .. } => String::new(),
    };
    let whole = line(&whole_name);
    // The name takes what the rest of the line leaves.
    let measure = |text: &str| {
        let galley = painter.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE);
        galley.size().x
    };
    let room = width - measure(&line(""));
    let cut = middle_cut(painter, &whole_name, "", font, room.max(0.0));
    (line(&cut), whole)
}

/// Browse's top line: a box saying what is open, a Boxed choice's look, which lists the
/// recents, newest first, to open one, a missing one marked with Locate… and Remove from
/// the List; then ＋, which makes or opens one, after it in the focus order too.
fn top_line(ui: &mut egui::Ui, browse: &Browse<'_>, actions: &mut Vec<Action>) {
    let t = theme(ui.ctx());
    let words = translator(ui.ctx());
    let line = vec2(ui.available_width(), t.sizes.panel_line);
    ui.allocate_ui_with_layout(line, Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = t.spacing.xs;
        let plus_name = words.text("browse-plus");
        let plus = IconButton::new(Icon::Plus, &plus_name);
        let plus_side = plus.side(ui);
        let width = (ui.available_width() - plus_side - t.spacing.xs).max(0.0);
        let ctx = ui.ctx().clone();
        let (_, whole) = open_line(
            &ctx,
            ui.painter(),
            browse.folder,
            &egui::FontId::default(),
            f32::INFINITY,
        );
        let name = words.text("browse-open-name");
        let icon = match browse.folder {
            Folder::Alone { .. } => Icon::Photo,
            _ => Icon::Folder,
        };
        let response =
            ChoiceBox::new(&name, &whole)
                .icon(icon)
                .show(ui, width, |painter, font, room| {
                    open_line(&ctx, painter, browse.folder, font, room).0
                });
        // Its count, which the tree's row shows, said with it.
        if let Folder::Open(listing) = browse.folder
            && listing.done
        {
            let mut args = Args::new();
            args.set("count", listing.photos.len() as f64);
            let count = words.text_with("photo-count", Some(&args));
            ctx.accesskit_node_builder(response.id, |node| node.set_description(count));
        }
        // The paths the list was built from, kept under its id, so that a list that changes
        // while it is open still opens the folder chosen.
        let paths = popup::button_menu(response.id).with("paths");
        let chosen = pull_down(ui, &response, &name, || {
            let (entries, listed) = recents_menu(&ctx, browse);
            ctx.data_mut(|d| d.insert_temp(paths, listed));
            entries
        });
        if let Some(chosen) = chosen {
            let listed: Vec<PathBuf> = ctx.data(|d| d.get_temp(paths)).unwrap_or_default();
            actions.extend(recent_action(&chosen, &listed));
        }
        let plus = plus.show(ui);
        let chosen = pull_down(ui, &plus, &plus_name, || plus_menu(&ctx, browse.keys));
        match chosen.as_deref() {
            Some("open-folder") => actions.push(Action::OpenFolder),
            Some("open") => actions.push(Action::OpenFiles),
            _ => {}
        }
    });
}

/// ＋'s menu, in the wireframe's order: the library and the project, which come later,
/// drawn disabled, then the ways to open, with their keys.
fn plus_menu(ctx: &egui::Context, keys: &Keys) -> Vec<Entry> {
    let words = translator(ctx);
    let later = words.text("section-tab-later");
    let with_keys = |item: Item, keys: &Option<String>| match keys {
        Some(keys) => item.shortcut(keys.clone()),
        None => item,
    };
    vec![
        Entry::Item(
            Item::new("new-library", words.text("action-new-library"))
                .enabled(false)
                .tip(later.clone()),
        ),
        Entry::Item(
            Item::new("new-project", words.text("action-new-project"))
                .enabled(false)
                .tip(later),
        ),
        Entry::Separator,
        Entry::Item(with_keys(
            Item::new("open-folder", words.text("action-open-folder")),
            &keys.folder,
        )),
        Entry::Item(with_keys(
            Item::new("open", words.text("action-open")),
            &keys.files,
        )),
    ]
}

/// The recents' list under the top line's box, with the paths its items stand for: each
/// folder by its name, the open one checked, its path in its tooltip; a missing one marked,
/// as a submenu whose first line says why, disabled, then Try Again where its opening
/// failed this run, Locate… and Remove from the List.
fn recents_menu(ctx: &egui::Context, browse: &Browse<'_>) -> (Vec<Entry>, Vec<PathBuf>) {
    let words = translator(ctx);
    if browse.recent.is_empty() {
        let none = Item::new("none", words.text("browse-recents-none")).enabled(false);
        return (vec![Entry::Item(none)], Vec::new());
    }
    let open = shown(browse.folder).map(|(path, ..)| path);
    let entries = browse
        .recent
        .iter()
        .enumerate()
        .map(|(i, recent)| {
            let label = name(recent.path);
            let tip = export::shown(recent.path, browse.home);
            let Some(why) = recent.missing else {
                let item = Item::new(format!("open:{i}"), label).tip(tip);
                return Entry::Item(item.checked(open == Some(recent.path)));
            };
            let mut args = Args::new();
            args.set("name", label.as_str());
            let text = words.text_with("browse-recent-missing", Some(&args));
            let reason = Item::new(format!("why:{i}"), missing_words(ctx, why))
                .enabled(false)
                .tip(tip);
            let mut entries = vec![Entry::Item(reason), Entry::Separator];
            if matches!(why, Missing::Failed(_)) {
                let again = Item::new(format!("open:{i}"), words.text("browse-try-again"));
                entries.push(Entry::Item(again));
            }
            entries.push(Entry::Item(Item::new(
                format!("locate:{i}"),
                words.text("browse-locate"),
            )));
            entries.push(Entry::Item(Item::new(
                format!("remove:{i}"),
                words.text("browse-remove"),
            )));
            Entry::Submenu(Submenu::new(format!("missing:{i}"), text, entries))
        })
        .collect();
    let paths = browse.recent.iter().map(|r| r.path.to_owned()).collect();
    (entries, paths)
}

/// What the item `id` of the recents' list asks for, of the path it stood for in
/// `listed`, the list as it was built.
fn recent_action(id: &str, listed: &[PathBuf]) -> Option<Action> {
    let (what, i) = id.split_once(':')?;
    let path = listed.get(i.parse::<usize>().ok()?)?.clone();
    match what {
        "open" => Some(Action::Open(path)),
        "locate" => Some(Action::Locate(path)),
        "remove" => Some(Action::Remove(path)),
        _ => None,
    }
}

/// Why a recent folder cannot be opened, in words.
fn missing_words(ctx: &egui::Context, why: Missing<'_>) -> String {
    let words = translator(ctx);
    match why {
        Missing::Failed(reason) => {
            let mut args = Args::new();
            args.set("reason", reason);
            words.text_with("folders-recent-missing", Some(&args))
        }
        Missing::Gone => words.text("folders-recent-gone"),
    }
}

/// One row of the tree: set in by its `level`, from 1; a chevron where it has subfolders
/// shown or hidden; the folder's icon, its name cut in its middle and its photo count at
/// the end where known. The open folder's is chosen, on `list_chosen`. To screen readers a
/// tree item with its level, chosen and open state.
struct TreeRow<'a> {
    level: usize,
    name: &'a str,
    count: Option<usize>,
    /// Some for a row whose subfolders show (true) or hide (false) under it.
    expanded: Option<bool>,
    chosen: bool,
}

impl TreeRow<'_> {
    /// Draws it; returns its response and whether its chevron was clicked.
    fn show(self, ui: &mut egui::Ui) -> (egui::Response, bool) {
        let t = theme(ui.ctx());
        let size = vec2(ui.available_width(), t.sizes.panel_line);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let indent = (self.level.saturating_sub(1)) as f32 * LEVEL_INDENT;
        let chevron_side = Icon::ChevronEnd.size();
        let chevron = Rect::from_min_size(
            pos2(rect.left() + indent, rect.center().y - chevron_side / 2.0),
            vec2(chevron_side, chevron_side),
        );
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let band = rect.expand2(vec2(t.spacing.xs, 0.0));
            if self.chosen {
                painter.rect_filled(band, t.shape.radius, t.colors.list_chosen);
            } else if response.hovered() {
                painter.rect_filled(band, t.shape.radius, t.colors.hover);
            }
            let ink = if self.chosen {
                t.colors.text_strong
            } else {
                t.colors.text
            };
            if let Some(expanded) = self.expanded {
                let icon = if expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronEnd
                };
                icon.paint(painter, chevron, t.colors.text_secondary);
            }
            let mut x = chevron.right() + t.spacing.xs;
            let icon = Rect::from_min_size(
                pos2(x, rect.center().y - ROW_ICON / 2.0),
                vec2(ROW_ICON, ROW_ICON),
            );
            Icon::Folder.paint(painter, icon, t.colors.text_secondary);
            x = icon.right() + t.spacing.s;
            let font = egui::FontId::proportional(t.text.body);
            let mut end = rect.right();
            if let Some(count) = self.count {
                let small = egui::FontId::proportional(t.text.caption);
                let count = translator(ui.ctx()).format_number(count as f64, 0, false);
                let galley = painter.layout_no_wrap(count, small, t.colors.text_secondary);
                end -= galley.size().x;
                let at = pos2(end, rect.center().y - galley.size().y / 2.0);
                painter.galley(at, galley, t.colors.text_secondary);
                end -= t.spacing.s;
            }
            let shown = middle_cut(painter, self.name, "", &font, (end - x).max(0.0));
            let galley = painter.layout_no_wrap(shown, font, ink);
            painter.galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
            if response.has_focus() {
                focus_ring_centred(painter, band, &t);
            }
        }
        let name = self.name;
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TreeItem, true, name));
        let (level, expanded, chosen, count) = (self.level, self.expanded, self.chosen, self.count);
        let words = translator(ui.ctx());
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_level(level.saturating_sub(1));
            node.set_selected(chosen);
            if let Some(expanded) = expanded {
                node.set_expanded(expanded);
            }
            if let Some(count) = count {
                let mut args = Args::new();
                args.set("count", count as f64);
                node.set_description(words.text_with("photo-count", Some(&args)));
            }
        });
        let on_chevron = self.expanded.is_some()
            && response.clicked()
            && response
                .interact_pointer_pos()
                .is_some_and(|at| chevron.expand(t.spacing.xxs).contains(at));
        (response, on_chevron)
    }
}

/// A folder row's right-click menu: Show in Folder, named as the platform's file manager
/// is (Show in Finder on the Mac), and Open.
fn row_menu(ui: &egui::Ui, response: &egui::Response, name: &str, path: &Path) -> Option<Action> {
    let words = translator(ui.ctx());
    let mut args = Args::new();
    args.set("name", name);
    let menu = words.text_with("browse-folder-menu", Some(&args));
    let chosen = context_menu(ui, response, &menu, || {
        vec![
            Entry::Item(Item::new(
                "show",
                words.text(jobs::FILE_MANAGER.show_message()),
            )),
            Entry::Item(Item::new("open", words.text("browse-open-row"))),
        ]
    })?;
    match chosen.as_str() {
        "show" => Some(Action::ShowInFolder(path.to_owned())),
        "open" => Some(Action::Open(path.to_owned())),
        _ => None,
    }
}

/// The Folders group's rows: the way up to the open folder's parent, a row naming it after
/// ‹; then the tree, the open folder chosen at its top with its photo count and, while it
/// is open, its subfolders a level in, each with its own while open. A click opens a
/// folder, the chevron or ← and → show or hide a folder's subfolders, and a right-click
/// holds Show in Folder and Open.
fn tree(ui: &mut egui::Ui, browse: &Browse<'_>, actions: &mut Vec<Action>) {
    let words = translator(ui.ctx());
    // A photo alone has no tree: its folder is offered instead.
    if let Some((_, folder)) = alone(browse.folder) {
        let this = words.text("browse-open-this-folder");
        let response = open_this_row(ui, &this).on_hover_text(export::shown(folder, browse.home));
        if response.clicked() {
            actions.push(Action::Open(folder.to_owned()));
        }
        return;
    }
    // With none open, the top line says so, and the tree shows nothing.
    let Some((path, count, subfolders)) = shown(browse.folder) else {
        return;
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let parent_name = name(parent);
        let mut args = Args::new();
        args.set("name", parent_name.as_str());
        let up = words.text_with("browse-parent", Some(&args));
        let response =
            up_row(ui, &parent_name, &up).on_hover_text(export::shown(parent, browse.home));
        if response.clicked() {
            actions.push(Action::Open(parent.to_owned()));
        }
        actions.extend(row_menu(ui, &response, &parent_name, parent));
    }
    let folders = words.text("browse-folders");
    let open = Branch {
        path,
        name: &name(path),
        level: 1,
        count,
        listed: Some(Inside::Listed(subfolders)),
        chosen: true,
    };
    ui.scope_builder(
        UiBuilder::new()
            .id_salt("browse tree")
            .accessibility_role(egui::Role::Tree)
            .accessibility_label(&folders),
        |ui| open.show(ui, browse, actions),
    );
}

/// A folder in the tree, with what is known of its subfolders.
/// What the library said of a folder's subfolders.
#[derive(Clone, Copy)]
enum Inside<'a> {
    Listed(&'a [String]),
    /// They could not be listed, for this reason.
    Unreadable(&'a str),
}

impl<'a> Inside<'a> {
    fn of(listed: &'a Result<Vec<String>, String>) -> Self {
        match listed {
            Ok(names) => Inside::Listed(names),
            Err(why) => Inside::Unreadable(why),
        }
    }
}

struct Branch<'a> {
    path: &'a Path,
    name: &'a str,
    level: usize,
    count: Option<usize>,
    /// Its subfolders as listed, or why they could not be, or None until the library has
    /// listed them.
    listed: Option<Inside<'a>>,
    /// Whether it is the open folder, chosen at the tree's top.
    chosen: bool,
}

impl Branch<'_> {
    /// Its row and, while it is open, its subfolders under it a level in. Until the library
    /// has listed them, a quiet "Reading…", and their ask, which the app sends once an open
    /// ([`Action::Ask`]), so a folder left open is asked again after the folder is opened
    /// again. Where they could not be listed, the reason and Try Again, its chevron kept: a
    /// share down a moment is not a folder with none. Open or closed as the person left it
    /// this run, else open for the open folder and closed for the others; a folder listed
    /// with no subfolders has no chevron.
    fn show(self, ui: &mut egui::Ui, browse: &Browse<'_>, actions: &mut Vec<Action>) {
        let open = browse.tree.get(self.path).copied().unwrap_or(self.chosen);
        let chevron = match self.listed {
            Some(Inside::Listed([])) => None,
            _ => Some(open),
        };
        let (response, on_chevron) = TreeRow {
            level: self.level,
            name: self.name,
            count: self.count,
            expanded: chevron,
            chosen: self.chosen,
        }
        .show(ui);
        let response = response.on_hover_text(export::shown(self.path, browse.home));
        // ← closes the folder, → opens it, as a tree's keys do.
        let key = |key: egui::Key| {
            response.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))
        };
        let (left, right) = (key(egui::Key::ArrowLeft), key(egui::Key::ArrowRight));
        if left || right {
            ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        let toggle = on_chevron || (chevron.is_some() && ((left && open) || (right && !open)));
        if toggle {
            actions.push(Action::ToggleTree(self.path.to_owned()));
        } else if response.clicked() && !self.chosen {
            actions.push(Action::Open(self.path.to_owned()));
        }
        actions.extend(row_menu(ui, &response, self.name, self.path));
        if chevron != Some(true) {
            return;
        }
        let names = match self.listed {
            Some(Inside::Listed(names)) => names,
            Some(Inside::Unreadable(why)) => {
                if retry_row(ui, self.level + 1, why) {
                    actions.push(Action::Retry(self.path.to_owned()));
                }
                return;
            }
            None => {
                reading_row(ui, self.level + 1);
                actions.push(Action::Ask(self.path.to_owned()));
                return;
            }
        };
        for sub in names {
            let full = self.path.join(sub);
            let listed = browse.inside.get(&full).map(Inside::of);
            Branch {
                path: &full,
                name: sub,
                level: self.level + 1,
                count: None,
                listed,
                chosen: false,
            }
            .show(ui, browse, actions);
        }
    }
}

/// The line under a folder whose subfolders could not be listed, set in at `level`: "Could
/// not be read. Try Again", a button, the reason in its tooltip. Returns whether it was
/// pressed.
fn retry_row(ui: &mut egui::Ui, level: usize, why: &str) -> bool {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let words = translator(ui.ctx()).text("browse-unreadable");
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let band = rect.expand2(vec2(t.spacing.xs, 0.0));
        if response.hovered() {
            painter.rect_filled(band, t.shape.radius, t.colors.hover);
        }
        let indent = (level.saturating_sub(1)) as f32 * LEVEL_INDENT;
        let x = rect.left() + indent + Icon::ChevronEnd.size() + t.spacing.xs;
        let font = egui::FontId::proportional(t.text.small);
        let galley = painter.layout_no_wrap(words.clone(), font, t.colors.text_secondary);
        let at = pos2(x, rect.center().y - galley.size().y / 2.0);
        painter.galley(at, galley, t.colors.text_secondary);
        if response.has_focus() {
            focus_ring_centred(painter, band, &t);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &words));
    response.on_hover_text(why).clicked()
}

/// The quiet line under a folder whose subfolders the library is listing, set in at
/// `level`.
fn reading_row(ui: &mut egui::Ui, level: usize) {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let words = translator(ui.ctx()).text("browse-reading");
    if ui.is_rect_visible(rect) {
        let indent = (level.saturating_sub(1)) as f32 * LEVEL_INDENT;
        let x = rect.left() + indent + Icon::ChevronEnd.size() + t.spacing.xs;
        let font = egui::FontId::proportional(t.text.small);
        let painter = ui.painter();
        let galley = painter.layout_no_wrap(words.clone(), font, t.colors.text_secondary);
        let at = pos2(x, rect.center().y - galley.size().y / 2.0);
        painter.galley(at, galley, t.colors.text_secondary);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &words));
}

/// The way up: ‹ and the parent's name in `text_secondary`, a button named `up` ("Up to
/// 2026") for screen readers.
fn up_row(ui: &mut egui::Ui, parent: &str, up: &str) -> egui::Response {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let band = rect.expand2(vec2(t.spacing.xs, 0.0));
        if response.hovered() {
            painter.rect_filled(band, t.shape.radius, t.colors.hover);
        }
        let side = Icon::Back.size();
        let at = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - side / 2.0),
            vec2(side, side),
        );
        Icon::Back.paint(painter, at, t.colors.text_secondary);
        let x = at.right() + t.spacing.xs;
        let font = egui::FontId::proportional(t.text.body);
        let shown = middle_cut(painter, parent, "", &font, (rect.right() - x).max(0.0));
        let galley = painter.layout_no_wrap(shown, font, t.colors.text_secondary);
        painter.galley(
            pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            t.colors.text_secondary,
        );
        if response.has_focus() {
            focus_ring_centred(painter, band, &t);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, up));
    response
}

/// Open This Folder, a photo alone's one row in Folders, as the wireframe draws it: the
/// folder icon in `text_secondary` and the words in the text colour, underlined, a button
/// for screen readers.
fn open_this_row(ui: &mut egui::Ui, words: &str) -> egui::Response {
    let t = theme(ui.ctx());
    let size = vec2(ui.available_width(), t.sizes.panel_line);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let band = rect.expand2(vec2(t.spacing.xs, 0.0));
        if response.hovered() {
            painter.rect_filled(band, t.shape.radius, t.colors.hover);
        }
        let icon = Rect::from_min_size(
            pos2(rect.left(), rect.center().y - ROW_ICON / 2.0),
            vec2(ROW_ICON, ROW_ICON),
        );
        Icon::Folder.paint(painter, icon, t.colors.text_secondary);
        let x = icon.right() + t.spacing.s;
        let font = egui::FontId::proportional(t.text.body);
        let shown = middle_cut(painter, words, "", &font, (rect.right() - x).max(0.0));
        let format = egui::TextFormat {
            font_id: font,
            color: t.colors.text,
            underline: egui::Stroke::new(1.0, t.colors.text),
            ..Default::default()
        };
        let galley = painter.layout_job(egui::text::LayoutJob::single_section(shown, format));
        painter.galley(
            pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            t.colors.text,
        );
        if response.has_focus() {
            focus_ring_centred(painter, band, &t);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, words));
    response
}

/// The central panel's empty state in the middle of `slot`: "No photo selected" alone on the
/// surround, in its text colour; else a card with the heading, and under the start's the two
/// ways to open with their keys and the recent folders, under a folder with no raws Open
/// Folder… again.
pub fn empty_state(
    ui: &mut egui::Ui,
    slot: Rect,
    what: Empty<'_>,
    recent: &[Recent<'_>],
    keys: &Keys,
    home: Option<&Path>,
) -> Vec<Action> {
    let t = theme(ui.ctx());
    let words = translator(ui.ctx());
    let ctx = ui.ctx().clone();
    let heading = match what {
        Empty::Start => words.text("start-title"),
        Empty::Reading(path) => with_folder(&ctx, "folder-reading", &name(path)),
        Empty::Failed(path, reason) => {
            let shown = path.display().to_string();
            match reason {
                Some(reason) => {
                    let mut args = Args::new();
                    args.set("folder", shown.as_str());
                    args.set("reason", reason);
                    words.text_with("folder-failed", Some(&args))
                }
                None => with_folder(&ctx, "folder-cancelled", &shown),
            }
        }
        Empty::NoRaws(name) => with_folder(&ctx, "folder-empty", name),
        Empty::NoneChosen => words.text("folder-no-photo"),
    };
    let buttons = matches!(what, Empty::Start | Empty::NoRaws(_) | Empty::Failed(..));
    let recents = matches!(what, Empty::Start) && !recent.is_empty();
    // A line alone where nothing is chosen yet; the card where there is more to say or do.
    let boxed = !matches!(what, Empty::NoneChosen);

    let width = (slot.width() - 2.0 * t.spacing.xl).clamp(0.0, 420.0);
    let margin = t.spacing.l;
    // The card's height from its parts, so it can be centred without a layout pass.
    let line = t.sizes.panel_line;
    let mut height = 2.0 * margin + t.text.heading * 1.6;
    if buttons {
        height += t.spacing.l + 2.0 * (t.sizes.control + t.spacing.s);
    }
    if recents {
        height += t.spacing.l + t.text.caption * 1.6 + recent.len() as f32 * line;
    }
    let height = height.min(slot.height() - 2.0 * t.spacing.m).max(0.0);
    let card = Rect::from_center_size(slot.center(), vec2(width, height));
    let mut actions = Vec::new();
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(card)
            .layout(Layout::top_down(Align::Center)),
        |ui| {
            if boxed {
                ui.painter()
                    .rect_filled(card, t.shape.radius, t.colors.raised);
            }
            ui.add_space(margin);
            ui.spacing_mut().item_spacing.y = t.spacing.s;
            ui.set_width(width);
            ui.add(
                egui::Label::new(egui::RichText::new(&heading).size(t.text.heading).color(
                    if boxed {
                        t.colors.text
                    } else {
                        t.colors.text_on_surround()
                    },
                ))
                .wrap(),
            );
            if buttons {
                ui.add_space(t.spacing.s);
                for (id, keys, action) in [
                    ("action-open-folder", &keys.folder, Action::OpenFolder),
                    ("action-open", &keys.files, Action::OpenFiles),
                ] {
                    let words = tr(ui.ctx(), id);
                    let mut button = Button::new(&words).width(width - 2.0 * margin);
                    if let Some(keys) = keys {
                        button = button.keys(keys);
                    }
                    if button.show(ui).clicked() {
                        actions.push(action);
                    }
                }
            }
            if recents {
                let inset = egui::Margin::symmetric(margin as i8, 0);
                egui::Frame::new().inner_margin(inset).show(ui, |ui| {
                    ui.set_width(width - 2.0 * margin);
                    group_label(ui, &tr(ui.ctx(), "start-recent"));
                    for entry in recent {
                        let tip = export::shown(entry.path, home);
                        if row(ui, &name(entry.path), &tip, entry.missing) {
                            actions.push(Action::Open(entry.path.to_owned()));
                        }
                    }
                });
            }
        },
    );
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::folder::Listing;

    fn listing(photos: usize, done: bool) -> Folder {
        let photo = |i: usize| apertrust_engine::session::library::Photo {
            name: format!("IMG_{i}.ARW"),
            size: 1,
            modified: None,
            edit: apertrust_engine::session::library::EditState::None,
        };
        Folder::Open(Listing {
            name: "Wedding".to_owned(),
            path: PathBuf::from("/photos/Wedding"),
            subfolders: Vec::new(),
            photos: (0..photos).map(photo).collect(),
            done,
            taken: None,
            thumbnails: Default::default(),
        })
    }

    #[test]
    fn the_empty_state_follows_the_folder() {
        assert_eq!(Empty::of(&Folder::None), Empty::Start);
        let reading = Folder::Opening {
            path: PathBuf::from("/photos/a"),
        };
        assert_eq!(Empty::of(&reading), Empty::Reading(Path::new("/photos/a")));
        let failed = Folder::Failed {
            path: PathBuf::from("/photos/a"),
            reason: Some("gone".to_owned()),
        };
        assert_eq!(
            Empty::of(&failed),
            Empty::Failed(Path::new("/photos/a"), Some("gone"))
        );
        // Listed but for a chunk: still reading; then none, or some.
        assert!(matches!(Empty::of(&listing(0, false)), Empty::Reading(_)));
        assert_eq!(Empty::of(&listing(0, true)), Empty::NoRaws("Wedding"));
        assert_eq!(Empty::of(&listing(3, true)), Empty::NoneChosen);
    }

    use apertrust_ui::components::PanelBody;
    use apertrust_ui::{Builtin, Locale, Theme};
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    /// An open folder with subfolders and every photo listed.
    fn wedding() -> Folder {
        let Folder::Open(mut listing) = listing(120, true) else {
            unreachable!()
        };
        listing.subfolders = vec!["Edits".to_owned(), "Selects".to_owned()];
        Folder::Open(listing)
    }

    /// Two recent folders, the second one that could not be opened.
    fn recent() -> Vec<Recent<'static>> {
        vec![
            Recent {
                path: Path::new("/photos/Portraits"),
                missing: None,
            },
            Recent {
                path: Path::new("/photos/Gone"),
                missing: Some(Missing::Failed("the folder is missing")),
            },
        ]
    }

    fn sized(
        size: egui::Vec2,
        draw: impl Fn(&mut egui::Ui, &mut Vec<Action>) + 'static,
    ) -> Harness<'static, Vec<Action>> {
        themed(Theme::default(), size, draw)
    }

    fn themed(
        theme: Theme,
        size: egui::Vec2,
        draw: impl Fn(&mut egui::Ui, &mut Vec<Action>) + 'static,
    ) -> Harness<'static, Vec<Action>> {
        Harness::builder()
            .with_size(size)
            .with_context_setup(move |ctx| {
                apertrust_ui::install(ctx, theme.clone());
                apertrust_ui::set_locale(ctx, Locale::EnGb);
            })
            .build_ui_state(
                move |ui, actions: &mut Vec<Action>| draw(ui, actions),
                Vec::new(),
            )
    }

    /// Browse and its Folders, both open.
    const OPEN_ALL: Open = Open {
        browse: true,
        folders: true,
    };

    /// The keys of the two ways to open, as a Linux keymap writes them.
    fn keys() -> Keys {
        Keys {
            folder: Some("Ctrl+Shift+O".to_owned()),
            files: Some("Ctrl+O".to_owned()),
        }
    }

    /// Browse drawn from `folder` and `recent`, its parts open as `open` says, the open
    /// folder's subfolders shown while `tree`.
    fn browsed(
        size: egui::Vec2,
        folder: impl Fn() -> Folder + 'static,
        recent: impl Fn() -> Vec<Recent<'static>> + 'static,
        open: Open,
        tree: bool,
    ) -> Harness<'static, Vec<Action>> {
        sized(size, move |ui, actions| {
            let (folder, recent, keys) = (folder(), recent(), keys());
            // The open folder closed where `tree` is false.
            let mut kept = BTreeMap::new();
            if let Some((path, ..)) = shown(&folder).filter(|_| !tree) {
                kept.insert(path.to_owned(), false);
            }
            let browse = Browse {
                folder: &folder,
                recent: &recent,
                open,
                tree: &kept,
                inside: &HashMap::new(),
                keys: &keys,
                home: None,
            };
            PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
        })
    }

    fn wedding_browsed() -> Harness<'static, Vec<Action>> {
        browsed(egui::vec2(320.0, 700.0), wedding, recent, OPEN_ALL, true)
    }

    /// The tree: the open folder chosen at its top, its subfolders a level in, each with its
    /// level, the open folder's open state and its count, for screen readers.
    #[test]
    fn the_tree_holds_the_open_folder_and_its_subfolders_a_level_in() {
        let mut harness = wedding_browsed();
        harness.run();
        harness.get_by_role_and_label(Role::Tree, "Folders");
        let open = harness.get_by_role_and_label(Role::TreeItem, "Wedding");
        let node = open.accesskit_node();
        assert_eq!(node.data().level(), Some(0));
        assert_eq!(node.data().is_expanded(), Some(true));
        assert_eq!(node.data().is_selected(), Some(true));
        assert_eq!(node.data().description(), Some("120 photos"));
        for sub in ["Edits", "Selects"] {
            let row = harness.get_by_role_and_label(Role::TreeItem, sub);
            assert_eq!(row.accesskit_node().data().level(), Some(1), "{sub}");
        }
        harness
            .get_by_role_and_label(Role::TreeItem, "Edits")
            .click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [Action::Open(PathBuf::from("/photos/Wedding/Edits"))]
        );
        // → and ← on the open folder show and hide its subfolders.
        harness
            .get_by_role_and_label(Role::TreeItem, "Wedding")
            .focus();
        harness.run();
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        assert_eq!(
            harness.state().last(),
            Some(&Action::ToggleTree(PathBuf::from("/photos/Wedding")))
        );
    }

    /// A subfolder opens a level of its own: its subfolders once listed, a quiet line
    /// while they are read, and no chevron where it has none.
    #[test]
    fn a_subfolder_opens_a_level_of_its_own() {
        let listed = || {
            let Folder::Open(mut listing) = wedding() else {
                unreachable!()
            };
            listing.subfolders.push("Raw".to_owned());
            Folder::Open(listing)
        };
        let listed_inside = || {
            let inside = [("Edits", vec!["Picks".to_owned()]), ("Raw", Vec::new())];
            let wedding = Path::new("/photos/Wedding");
            HashMap::from(inside.map(|(folder, names)| (wedding.join(folder), Ok(names))))
        };
        let opened = || {
            let tree = [("Edits", true), ("Selects", true)]
                .map(|(folder, open)| (PathBuf::from("/photos/Wedding").join(folder), open));
            BTreeMap::from(tree)
        };
        let mut harness = sized(egui::vec2(320.0, 700.0), move |ui, actions| {
            let (folder, keys, tree, inside) = (listed(), keys(), opened(), listed_inside());
            let browse = Browse {
                folder: &folder,
                recent: &[],
                open: OPEN_ALL,
                tree: &tree,
                inside: &inside,
                keys: &keys,
                home: None,
            };
            PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
        });
        harness.run();
        let picks = harness.get_by_role_and_label(Role::TreeItem, "Picks");
        assert_eq!(picks.accesskit_node().data().level(), Some(2));
        let edits = harness.get_by_role_and_label(Role::TreeItem, "Edits");
        assert_eq!(edits.accesskit_node().data().is_expanded(), Some(true));
        // Selects is open but not listed yet; Raw was listed with none.
        harness.get_by_label("Reading…");
        let raw = harness.get_by_role_and_label(Role::TreeItem, "Raw");
        assert_eq!(raw.accesskit_node().data().is_expanded(), None);
        // → on a closed subfolder opens it, ← on an open one closes it.
        harness
            .get_by_role_and_label(Role::TreeItem, "Edits")
            .focus();
        harness.run();
        harness.key_press(egui::Key::ArrowLeft);
        harness.run();
        // Selects, open and not listed, asks for its subfolders each frame it is drawn.
        let selects = PathBuf::from("/photos/Wedding/Selects");
        let (asks, rest): (Vec<_>, Vec<_>) = harness
            .state()
            .iter()
            .cloned()
            .partition(|action| matches!(action, Action::Ask(_)));
        assert!(
            asks.iter().all(|ask| *ask == Action::Ask(selects.clone())),
            "{asks:?}"
        );
        assert!(!asks.is_empty());
        assert_eq!(
            rest,
            [Action::ToggleTree(PathBuf::from("/photos/Wedding/Edits"))]
        );
    }

    /// A folder left open asks for its subfolders while they are not listed, as after the
    /// folder is opened again; its own open subfolder asks once it is listed itself.
    #[test]
    fn an_open_folder_not_listed_asks_and_its_open_subfolder_asks_after_it() {
        let edits = PathBuf::from("/photos/Wedding/Edits");
        let picks = edits.join("Picks");
        let tree = BTreeMap::from([(edits.clone(), true), (picks.clone(), true)]);
        let asked = |inside: HashMap<PathBuf, Result<Vec<String>, String>>| {
            let tree = tree.clone();
            let mut harness = sized(egui::vec2(320.0, 700.0), move |ui, actions| {
                let (folder, keys) = (wedding(), keys());
                let browse = Browse {
                    folder: &folder,
                    recent: &[],
                    open: OPEN_ALL,
                    tree: &tree,
                    inside: &inside,
                    keys: &keys,
                    home: None,
                };
                PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
            });
            harness.run();
            let mut asks: Vec<Action> = harness.state().clone();
            asks.dedup();
            asks
        };
        // Opened again, nothing is listed: Edits asks, and Picks waits for it.
        assert_eq!(asked(HashMap::new()), [Action::Ask(edits.clone())]);
        let listed = HashMap::from([(edits.clone(), Ok(vec!["Picks".to_owned()]))]);
        assert_eq!(asked(listed), [Action::Ask(picks)]);
    }

    /// A folder whose subfolders could not be listed keeps its chevron and offers to try
    /// again, its reason in the tooltip.
    #[test]
    fn a_folder_that_could_not_be_listed_offers_to_try_again() {
        let edits = PathBuf::from("/photos/Wedding/Edits");
        let tree = BTreeMap::from([(edits.clone(), true)]);
        let inside = HashMap::from([(edits.clone(), Err("the share is down".to_owned()))]);
        let mut harness = sized(egui::vec2(320.0, 700.0), move |ui, actions| {
            let (folder, keys) = (wedding(), keys());
            let browse = Browse {
                folder: &folder,
                recent: &[],
                open: OPEN_ALL,
                tree: &tree,
                inside: &inside,
                keys: &keys,
                home: None,
            };
            PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
        });
        harness.run();
        let row = harness.get_by_role_and_label(Role::TreeItem, "Edits");
        assert_eq!(row.accesskit_node().data().is_expanded(), Some(true));
        harness
            .get_by_role_and_label(Role::Button, "Could not be read. Try Again")
            .click();
        harness.run();
        assert_eq!(*harness.state(), [Action::Retry(edits)]);
    }

    /// Hidden, the subfolders leave the tree, and the open folder says it is closed.
    #[test]
    fn the_open_folder_s_subfolders_hide() {
        let mut harness = browsed(egui::vec2(320.0, 700.0), wedding, recent, OPEN_ALL, false);
        harness.run();
        let open = harness.get_by_role_and_label(Role::TreeItem, "Wedding");
        assert_eq!(open.accesskit_node().data().is_expanded(), Some(false));
        assert!(harness.query_by_label("Edits").is_none());
    }

    /// Above the tree, the way up: a row naming the parent, which opens it.
    #[test]
    fn the_parent_s_row_opens_the_parent() {
        let mut harness = wedding_browsed();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Up to photos")
            .click();
        harness.run();
        assert_eq!(*harness.state(), [Action::Open(PathBuf::from("/photos"))]);
        // A folder at the top of its disk has none.
        let root = || {
            let Folder::Open(mut listing) = listing(1, true) else {
                unreachable!()
            };
            listing.path = PathBuf::from("/");
            Folder::Open(listing)
        };
        let mut top = browsed(egui::vec2(320.0, 700.0), root, Vec::new, OPEN_ALL, true);
        top.run();
        assert!(top.query_by_label_contains("Up to").is_none());
    }

    /// The top line says what is open, and its list holds the recents: choosing one opens
    /// it.
    #[test]
    fn the_top_line_says_what_is_open_and_lists_the_recents() {
        let mut harness = wedding_browsed();
        harness.run();
        let what = harness.get_by_role_and_label(Role::ComboBox, "What is open");
        assert_eq!(
            what.accesskit_node().value().as_deref(),
            Some("Folder: Wedding")
        );
        assert_eq!(
            what.accesskit_node().data().description(),
            Some("120 photos")
        );
        what.click();
        harness.run();
        // A check item, which the open folder would have checked.
        harness.get_by_label("Portraits").click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [Action::Open(PathBuf::from("/photos/Portraits"))]
        );
    }

    /// The box's list opens the folder it was built for, though the recents change while
    /// it is open.
    #[test]
    fn a_recent_chosen_after_the_list_changed_is_the_one_shown() {
        let moved = std::rc::Rc::new(std::cell::Cell::new(false));
        let during = moved.clone();
        let mut harness = sized(egui::vec2(320.0, 700.0), move |ui, actions| {
            let (folder, keys) = (wedding(), keys());
            // A folder opened elsewhere goes to the top while the list is open.
            let mut recent = recent();
            if during.get() {
                recent.insert(
                    0,
                    Recent {
                        path: Path::new("/photos/Newer"),
                        missing: None,
                    },
                );
            }
            let browse = Browse {
                folder: &folder,
                recent: &recent,
                open: OPEN_ALL,
                tree: &BTreeMap::new(),
                inside: &HashMap::new(),
                keys: &keys,
                home: None,
            };
            PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
        });
        harness.run();
        harness
            .get_by_role_and_label(Role::ComboBox, "What is open")
            .click();
        harness.run();
        moved.set(true);
        harness.run();
        harness.get_by_label("Portraits").click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [Action::Open(PathBuf::from("/photos/Portraits"))]
        );
    }

    /// The open folder is checked in the box's list.
    #[test]
    fn the_open_folder_is_checked_among_the_recents() {
        let open = || {
            let Folder::Open(mut listing) = listing(3, true) else {
                unreachable!()
            };
            listing.name = "Portraits".to_owned();
            listing.path = PathBuf::from("/photos/Portraits");
            Folder::Open(listing)
        };
        let mut harness = browsed(egui::vec2(320.0, 700.0), open, recent, OPEN_ALL, true);
        harness.run();
        harness
            .get_by_role_and_label(Role::ComboBox, "What is open")
            .click();
        harness.run();
        // The list's item, not the tree's row of the same name.
        let toggled: Vec<_> = harness
            .get_all_by_label("Portraits")
            .filter_map(|node| node.accesskit_node().data().toggled())
            .collect();
        assert_eq!(toggled, [egui::accesskit::Toggled::True]);
    }

    /// The top line while a folder is read, and after one could not be.
    #[test]
    fn the_top_line_says_a_folder_is_read_or_that_none_is_open() {
        let reading = || Folder::Opening {
            path: PathBuf::from("/photos/Wedding"),
        };
        let failed = || Folder::Failed {
            path: PathBuf::from("/photos/Gone"),
            reason: Some("the folder is missing".to_owned()),
        };
        for (folder, says) in [
            (
                Box::new(reading) as Box<dyn Fn() -> Folder>,
                "Reading Wedding…",
            ),
            (Box::new(failed), "No folder open"),
        ] {
            let mut harness = browsed(egui::vec2(320.0, 700.0), folder, Vec::new, OPEN_ALL, true);
            harness.run();
            let what = harness.get_by_role_and_label(Role::ComboBox, "What is open");
            assert_eq!(what.accesskit_node().value().as_deref(), Some(says));
        }
    }

    /// A recent whose opening failed this run can be tried again from its submenu.
    #[test]
    fn a_recent_that_failed_can_be_tried_again() {
        let mut harness = wedding_browsed();
        harness.run();
        harness
            .get_by_role_and_label(Role::ComboBox, "What is open")
            .click();
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItem, "Gone, not found")
            .click();
        harness.run();
        harness
            .get_by_role_and_label(Role::MenuItem, "Try Again")
            .click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [Action::Open(PathBuf::from("/photos/Gone"))]
        );
    }

    /// A folder row's menu opens from the keyboard too: the menu key and Shift+F10.
    #[test]
    fn a_folder_row_s_menu_opens_from_the_keyboard() {
        for (modifiers, key) in [
            (egui::Modifiers::NONE, egui::Key::Menu),
            (egui::Modifiers::SHIFT, egui::Key::F10),
        ] {
            let mut harness = wedding_browsed();
            harness.run();
            harness
                .get_by_role_and_label(Role::TreeItem, "Edits")
                .focus();
            harness.run();
            harness.key_press_modifiers(modifiers, key);
            harness.run();
            harness
                .get_by_role_and_label(Role::MenuItem, "Open")
                .click();
            harness.run();
            assert_eq!(
                *harness.state(),
                [Action::Open(PathBuf::from("/photos/Wedding/Edits"))],
                "{key:?}"
            );
        }
    }

    /// The open folder's chevron, and →, show its subfolders; ← hides them.
    #[test]
    fn the_open_folder_s_chevron_and_arrows_show_and_hide_its_subfolders() {
        let toggle = Action::ToggleTree(PathBuf::from("/photos/Wedding"));
        let mut hidden = browsed(egui::vec2(320.0, 700.0), wedding, recent, OPEN_ALL, false);
        hidden.run();
        hidden
            .get_by_role_and_label(Role::TreeItem, "Wedding")
            .focus();
        hidden.run();
        // ← on a closed row, and → on an open one, change nothing.
        hidden.key_press(egui::Key::ArrowLeft);
        hidden.run();
        assert!(hidden.state().is_empty());
        hidden.key_press(egui::Key::ArrowRight);
        hidden.run();
        assert_eq!(*hidden.state(), std::slice::from_ref(&toggle));
        // A click on the chevron, at the row's start.
        let mut shown = wedding_browsed();
        shown.run();
        let row = shown.get_by_role_and_label(Role::TreeItem, "Wedding");
        let rect = row.rect();
        shown.event(egui::Event::PointerMoved(egui::pos2(
            rect.left() + 4.0,
            rect.center().y,
        )));
        shown.run();
        let at = egui::pos2(rect.left() + 4.0, rect.center().y);
        shown.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        shown.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        shown.run();
        assert_eq!(*shown.state(), [toggle]);
    }

    /// A missing recent is marked in the list, with why, Locate… and Remove from the List.
    #[test]
    fn a_missing_recent_offers_locate_and_remove() {
        for (item, action) in [
            ("Locate…", Action::Locate(PathBuf::from("/photos/Gone"))),
            (
                "Remove from the List",
                Action::Remove(PathBuf::from("/photos/Gone")),
            ),
        ] {
            let mut harness = wedding_browsed();
            harness.run();
            harness
                .get_by_role_and_label(Role::ComboBox, "What is open")
                .click();
            harness.run();
            harness
                .get_by_role_and_label(Role::MenuItem, "Gone, not found")
                .click();
            harness.run();
            let why =
                harness.get_by_label("This folder could not be opened. the folder is missing");
            assert!(why.accesskit_node().is_disabled());
            harness.get_by_role_and_label(Role::MenuItem, item).click();
            harness.run();
            assert_eq!(*harness.state(), [action], "{item}");
        }
    }

    /// ＋ holds the two ways to open with their keys, and the library and the project to
    /// come, drawn disabled.
    #[test]
    fn plus_opens_a_folder_or_a_photo_and_offers_what_comes_later_disabled() {
        let mut harness = wedding_browsed();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "New or open")
            .click();
        harness.run();
        for later in ["New Library…", "New Project…"] {
            let item = harness.get_by_role_and_label(Role::MenuItem, later);
            assert!(item.accesskit_node().is_disabled(), "{later}");
        }
        let folder = harness.get_by_role_and_label(Role::MenuItem, "Open Folder…");
        assert!(!folder.accesskit_node().is_disabled());
        harness.get_by_role_and_label(Role::MenuItem, "Open…");
        folder.click();
        harness.run();
        assert_eq!(*harness.state(), [Action::OpenFolder]);
        // Its Open… is File › Open…'s, with its key.
        harness
            .get_by_role_and_label(Role::Button, "New or open")
            .click();
        harness.run();
        let open = harness.get_by_role_and_label(Role::MenuItem, "Open…");
        let keys = open
            .accesskit_node()
            .data()
            .keyboard_shortcut()
            .map(str::to_owned);
        open.click();
        harness.run();
        assert_eq!(keys.as_deref(), Some("Ctrl+O"));
        assert_eq!(*harness.state(), [Action::OpenFolder, Action::OpenFiles]);
    }

    /// ⋯ shows the open folder in the platform's file manager.
    #[test]
    fn browse_s_options_show_the_open_folder_in_the_file_manager() {
        let mut harness = wedding_browsed();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Browse options")
            .click();
        harness.run();
        let words = translator(&harness.ctx);
        let show = words.text(jobs::FILE_MANAGER.show_message());
        harness.get_by_role_and_label(Role::MenuItem, &show).click();
        harness.run();
        assert_eq!(
            *harness.state(),
            [Action::Reveal(PathBuf::from("/photos/Wedding"))]
        );
    }

    /// A photo opened alone, as the command line opens one: the wireframe's IMG_2041.ARW.
    fn alone() -> Folder {
        Folder::Alone {
            path: PathBuf::from("/Users/me/Pictures/2026-10 Lisbon/IMG_2041.ARW"),
            photo: apertrust_engine::session::library::Photo {
                name: "IMG_2041.ARW".to_owned(),
                size: 0,
                modified: None,
                edit: apertrust_engine::session::library::EditState::None,
            },
        }
    }

    /// Browse for a photo alone: its top line names the photo, and Folders holds Open This
    /// Folder alone, which opens the photo's folder, as ⋯'s Open This Folder does; ⋯'s
    /// Show in Finder shows the photo in it.
    #[test]
    fn a_photo_alone_offers_its_folder_as_open_this_folder() {
        let lisbon = PathBuf::from("/Users/me/Pictures/2026-10 Lisbon");
        let mut harness = browsed(egui::vec2(320.0, 400.0), alone, recent, OPEN_ALL, true);
        harness.run();
        let what = harness.get_by_role_and_label(Role::ComboBox, "What is open");
        assert_eq!(
            what.accesskit_node().value().as_deref(),
            Some("IMG_2041.ARW")
        );
        assert!(harness.query_by_role(Role::Tree).is_none());
        assert!(harness.query_by_label_contains("Up to").is_none());
        harness
            .get_by_role_and_label(Role::Button, "Open This Folder")
            .click();
        harness.run();
        assert_eq!(*harness.state(), [Action::Open(lisbon.clone())]);
        let words = translator(&harness.ctx);
        let show = words.text(jobs::FILE_MANAGER.show_message());
        for (item, action) in [
            ("Open This Folder", Action::Open(lisbon.clone())),
            (
                show.as_str(),
                Action::ShowInFolder(lisbon.join("IMG_2041.ARW")),
            ),
        ] {
            harness.state_mut().clear();
            harness
                .get_by_role_and_label(Role::Button, "Browse options")
                .click();
            harness.run();
            harness.get_by_role_and_label(Role::MenuItem, item).click();
            harness.run();
            assert_eq!(*harness.state(), [action], "{item}");
        }
    }

    /// A folder row's right-click holds Show in Folder and Open.
    #[test]
    fn a_folder_row_s_right_click_shows_it_in_its_folder_or_opens_it() {
        let show = translator(&egui::Context::default()).text(jobs::FILE_MANAGER.show_message());
        for (item, action) in [
            (
                show.as_str(),
                Action::ShowInFolder(PathBuf::from("/photos/Wedding/Selects")),
            ),
            (
                "Open",
                Action::Open(PathBuf::from("/photos/Wedding/Selects")),
            ),
        ] {
            let mut harness = wedding_browsed();
            harness.run();
            harness
                .get_by_role_and_label(Role::TreeItem, "Selects")
                .click_secondary();
            harness.run();
            harness.get_by_role_and_label(Role::MenuItem, item).click();
            harness.run();
            assert_eq!(*harness.state(), [action], "{item}");
        }
    }

    /// Folder mode shows Folders alone in Browse: Albums and Smart albums come with the
    /// library. Browse's header and the Folders row each open or close their own.
    #[test]
    fn browse_holds_folders_alone_and_its_header_and_the_folders_row_toggle() {
        let mut harness = browsed(
            egui::vec2(320.0, 700.0),
            || Folder::None,
            Vec::new,
            OPEN_ALL,
            true,
        );
        harness.run();
        for title in ["Albums", "Smart albums"] {
            assert!(harness.query_by_label(title).is_none(), "{title}");
        }
        // The top line says none is open; the tree shows nothing.
        let what = harness.get_by_role_and_label(Role::ComboBox, "What is open");
        assert_eq!(
            what.accesskit_node().value().as_deref(),
            Some("No folder open")
        );
        assert!(harness.query_by_role(Role::Tree).is_none());
        harness
            .get_by_role_and_label(Role::Button, "Browse")
            .click();
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "Folders")
            .click();
        harness.run();
        assert_eq!(*harness.state(), [Action::Toggle, Action::ToggleFolders]);
    }

    /// With Folders closed, Browse keeps its top line and hides the tree.
    #[test]
    fn a_closed_folders_group_hides_the_tree_but_not_browse_s_top_line() {
        let open = Open {
            browse: true,
            folders: false,
        };
        let mut harness = browsed(egui::vec2(320.0, 700.0), wedding, recent, open, true);
        harness.run();
        harness.get_by_role_and_label(Role::ComboBox, "What is open");
        for hidden in ["Wedding", "Edits", "Up to photos"] {
            assert!(harness.query_by_label(hidden).is_none(), "{hidden}");
        }
    }

    /// A folder whose name is too long for the panel is cut in its middle, in the top line
    /// and in the tree, where its count stays whole.
    #[test]
    fn a_long_name_is_cut_in_its_middle() {
        let long = || {
            let Folder::Open(mut listing) = listing(212, true) else {
                unreachable!()
            };
            listing.name = "2026-10 Lisbon, Sintra and the Atlantic coast, every day".to_owned();
            listing.path = PathBuf::from("/photos").join(&listing.name);
            Folder::Open(listing)
        };
        let mut harness = browsed(egui::vec2(240.0, 400.0), long, Vec::new, OPEN_ALL, true);
        harness.run();
        let texts: Vec<String> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        let top = texts
            .iter()
            .find(|text| text.starts_with("Folder: "))
            .expect("the top line");
        assert!(top.contains('…') && top.ends_with('y'), "{top}");
        assert!(texts.iter().any(|text| text == "212"), "{texts:?}");
        let row = texts
            .iter()
            .find(|text| text.starts_with("2026-10") && text.ends_with("day"))
            .expect("the tree's row");
        assert!(row.contains('…'), "{row}");
    }

    /// The central panel's card over a slot, with what it asked for.
    fn card(what: Folder, recent: Vec<(PathBuf, Option<String>)>) -> Harness<'static, Vec<Action>> {
        card_sized(egui::vec2(320.0, 700.0), what, recent)
    }

    fn card_sized(
        size: egui::Vec2,
        what: Folder,
        recent: Vec<(PathBuf, Option<String>)>,
    ) -> Harness<'static, Vec<Action>> {
        sized(size, move |ui, actions| {
            let slot = ui.ctx().content_rect();
            let surround = theme(ui.ctx()).colors.photo_surround;
            ui.painter().rect_filled(slot, 0.0, surround);
            let recent: Vec<Recent<'_>> = recent
                .iter()
                .map(|(path, missing)| Recent {
                    path,
                    missing: missing.as_deref().map(Missing::Failed),
                })
                .collect();
            let keys = Keys {
                folder: Some("Ctrl+Shift+O".to_owned()),
                files: Some("Ctrl+O".to_owned()),
            };
            actions.extend(empty_state(
                ui,
                slot,
                Empty::of(&what),
                &recent,
                &keys,
                None,
            ));
        })
    }

    #[test]
    fn with_nothing_open_the_card_offers_both_ways_to_open_and_the_recent_folders() {
        let recent = vec![
            (PathBuf::from("/photos/Portraits"), None),
            (PathBuf::from("/photos/Gone"), Some("missing".to_owned())),
        ];
        let mut harness = card(Folder::None, recent);
        harness.run();
        harness.get_by_label("Open a folder to start");
        let folder = harness.get_by_label("Open Folder…");
        // The keys are the button's shortcut, not part of its name.
        assert_eq!(
            folder.accesskit_node().data().keyboard_shortcut(),
            Some("Ctrl+Shift+O")
        );
        folder.click();
        harness.run();
        harness.get_by_label("Open…").click();
        harness.run();
        harness.get_by_label("Portraits").click();
        harness.run();
        assert!(harness.get_by_label("Gone").accesskit_node().is_disabled());
        assert_eq!(
            *harness.state(),
            [
                Action::OpenFolder,
                Action::OpenFiles,
                Action::Open(PathBuf::from("/photos/Portraits"))
            ]
        );
    }

    #[test]
    fn a_folder_with_no_raws_a_failed_one_and_one_with_no_photo_chosen_each_say_so() {
        let mut empty = card(listing(0, true), Vec::new());
        empty.run();
        empty.get_by_label("No raw photos in Wedding");
        empty.get_by_label("Open Folder…");

        let failed = Folder::Failed {
            path: PathBuf::from("/photos/gone"),
            reason: Some("the folder is missing".to_owned()),
        };
        let mut failed = card(failed, Vec::new());
        failed.run();
        failed.get_by_label("Could not open /photos/gone: the folder is missing");

        let cancelled = Folder::Failed {
            path: PathBuf::from("/photos/slow"),
            reason: None,
        };
        let mut cancelled = card(cancelled, Vec::new());
        cancelled.run();
        cancelled.get_by_label("Reading /photos/slow was cancelled");

        let mut chosen = card(wedding(), Vec::new());
        chosen.run();
        // A line alone: nothing more to read, and no card drawn under it.
        chosen.get_by_label("No photo selected");
        assert!(
            chosen
                .query_by_label("Choose one in the grid panel")
                .is_none()
        );
        let raised = theme(&chosen.ctx).colors.raised;
        let filled = |shape: &egui::epaint::ClippedShape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == raised);
        assert!(!chosen.output().shapes.iter().any(filled));
        // On the surround, so in the surround's text colour, which reads on every background.
        let colour = chosen
            .output()
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "No photo selected" => {
                    text.galley.job.sections.first().map(|s| s.format.color)
                }
                _ => None,
            });
        let on_surround = theme(&chosen.ctx).colors.text_on_surround();
        assert_eq!(colour, Some(on_surround));
        let mut start = card(Folder::None, Vec::new());
        start.run();
        assert!(
            start.output().shapes.iter().any(filled),
            "the start keeps its card"
        );
    }

    #[test]
    fn the_grid_panel_says_a_folder_has_no_raws_and_nothing_in_the_other_states() {
        let ctx = egui::Context::default();
        apertrust_ui::install(&ctx, Theme::default());
        assert_eq!(
            Empty::of(&listing(0, true)).grid_says(&ctx).as_deref(),
            Some("No raw photos in Wedding")
        );
        for folder in [Folder::None, wedding(), listing(0, false)] {
            assert_eq!(Empty::of(&folder).grid_says(&ctx), None);
        }
    }

    /// The Photos tab's Browse in the dark and light themes, and the start card,
    /// photographed as the app draws them.
    #[test]
    fn the_photos_tab_and_the_start_card_are_photographed() {
        let mut results = egui_kittest::SnapshotResults::new();
        // The wireframe's folder: a long trip, its days and its panoramas.
        let lisbon = || {
            let Folder::Open(mut listing) = listing(212, true) else {
                unreachable!()
            };
            listing.name = "2026-10 Lisbon".to_owned();
            listing.path = PathBuf::from("/Users/me/Pictures/2026-10 Lisbon");
            listing.subfolders = ["Day 1 Alfama", "Day 2 Sintra", "Panoramas"]
                .map(str::to_owned)
                .to_vec();
            Folder::Open(listing)
        };
        for (which, name) in [
            (Builtin::Dark, "photos-tab"),
            (Builtin::Light, "photos-tab-light"),
        ] {
            let mut tab = themed(
                Theme::builtin(which),
                egui::vec2(320.0, 360.0),
                move |ui, actions| {
                    let panel = theme(ui.ctx()).colors.panel;
                    ui.painter().rect_filled(ui.max_rect(), 0.0, panel);
                    let (folder, recent, keys) = (lisbon(), recent(), keys());
                    // Day 1 opened and listed, Panoramas listed with none, Day 2 not yet.
                    let trip = Path::new("/Users/me/Pictures/2026-10 Lisbon");
                    let day = trip.join("Day 1 Alfama");
                    let parts = ["Evening", "Morning"].map(str::to_owned).to_vec();
                    let inside = HashMap::from([
                        (day.clone(), Ok(parts)),
                        (trip.join("Panoramas"), Ok(Vec::new())),
                    ]);
                    let tree = BTreeMap::from([(day, true)]);
                    let browse = Browse {
                        folder: &folder,
                        recent: &recent,
                        open: OPEN_ALL,
                        tree: &tree,
                        inside: &inside,
                        keys: &keys,
                        home: None,
                    };
                    PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
                },
            );
            tab.run();
            tab.snapshot(name);
            results.extend(tab.take_snapshot_results());
        }

        // A photo alone, as the wireframe's one-photo column draws it.
        let mut tab = themed(
            Theme::builtin(Builtin::Dark),
            egui::vec2(320.0, 160.0),
            move |ui, actions| {
                let panel = theme(ui.ctx()).colors.panel;
                ui.painter().rect_filled(ui.max_rect(), 0.0, panel);
                let (folder, recent, keys) = (alone(), recent(), keys());
                let browse = Browse {
                    folder: &folder,
                    recent: &recent,
                    open: OPEN_ALL,
                    tree: &BTreeMap::new(),
                    inside: &HashMap::new(),
                    keys: &keys,
                    home: None,
                };
                PanelBody::new().show(ui, |ui| actions.extend(super::browse(ui, &browse)));
            },
        );
        tab.run();
        tab.snapshot("photos-tab-alone");
        results.extend(tab.take_snapshot_results());

        let recent = vec![
            (PathBuf::from("/photos/Portraits"), None),
            (PathBuf::from("/photos/Gone"), Some("missing".to_owned())),
        ];
        let mut card = card_sized(egui::vec2(640.0, 360.0), Folder::None, recent);
        card.run();
        card.snapshot("start-card");
        results.extend(card.take_snapshot_results());
    }
}
