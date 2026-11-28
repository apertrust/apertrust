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

//! The Import tool's Source group (decision 85; `docs/gui.md`, The Import tool): where to
//! import from, a card or drive mounted as the sheet opened, or a folder of Pictures', Home's
//! or a volume's tree.
//!
//! - **Places** ([`Places`]): the volumes `ListVolumes` listed as the sheet opened, cards
//!   first, each by its name, then Pictures and Home. darktable's ＋, − and ↺ by the list's
//!   label are drawn disabled until the person's own places come (20a2).
//! - **Folders:** Collections' tree of the place chosen, at most 8 rows tall, each level
//!   asked for with `SourceSubfolders` as it opens, a level that cannot be read saying why
//!   in its row; a card's shows its `DCIM` chosen.
//! - **Choosing** a place or a folder makes it the source, which the sheet lists again
//!   with a new `ListSource`, cancelling the one before, and keeps as `import.place` and
//!   `import.folder`, written at once.
//! - **At opening** the first card among the volumes is chosen, where one is in; else the
//!   place and folder last used stay.
//! - ↑ and ↓ move along a list, Return or Space chooses.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use apertrust_engine::session::library::{Places as Folders, Volume};
use apertrust_engine::session::{Command, LibraryCommand, LibraryEvent};
use apertrust_ui::components::IconButton;
use apertrust_ui::i18n::{Args, Translator};
use apertrust_ui::{Icon, theme};
use eframe::egui::{self, UiBuilder};

use super::Source;
use crate::tree_row::TreeRow;

/// How many rows the Folders tree shows at most before it scrolls.
const MOST_ROWS: f32 = 8.0;

/// What a place is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A camera card, a volume holding a `DCIM` folder at its top.
    Card,
    /// Another volume; `removable` where the system takes it for one.
    Drive {
        removable: bool,
    },
    Pictures,
    Home,
}

/// One of Places' rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// Its folder: a volume's mount point, or Pictures or Home.
    pub root: PathBuf,
    /// A volume's name; Pictures and Home take their words.
    pub name: String,
    pub kind: Kind,
}

impl Place {
    /// Its name in `words`.
    fn shown(&self, words: &Translator) -> String {
        match self.kind {
            Kind::Pictures => words.text("import-place-pictures"),
            Kind::Home => words.text("import-place-home"),
            Kind::Card | Kind::Drive { .. } => self.name.clone(),
        }
    }

    fn icon(&self) -> Icon {
        match self.kind {
            Kind::Card => Icon::Camera,
            Kind::Drive { .. } => Icon::HardDrive,
            Kind::Pictures | Kind::Home => Icon::Folder,
        }
    }

    /// Whether its photos are on this computer already: a place on no removable volume.
    pub fn local(&self) -> bool {
        matches!(
            self.kind,
            Kind::Pictures | Kind::Home | Kind::Drive { removable: false }
        )
    }
}

/// The Source group's state: the volumes, the place and the folder chosen, and its tree.
#[derive(Debug, Default)]
pub struct Places {
    /// The volumes listed as the sheet opened, once `Volumes` has come.
    volumes: Option<Vec<Volume>>,
    /// The person's folders, Pictures and Home among them.
    folders: Folders,
    /// The place chosen, by its root, and the folder in its tree.
    place: Option<PathBuf>,
    folder: Option<PathBuf>,
    /// Each folder's subfolders as listed, or why it cannot be read.
    listed: HashMap<PathBuf, Result<Vec<String>, String>>,
    /// The folders asked for and not answered yet.
    asking: BTreeSet<PathBuf>,
    /// The tree's folders open.
    expanded: BTreeSet<PathBuf>,
    /// Whether the first volumes have come: the card among them is chosen then.
    opened: bool,
    /// What to send the session, sent as the sheet draws.
    commands: Vec<Command>,
}

impl Places {
    /// Source with the person's `folders`, on `place` and `folder` as last used, by their
    /// paths, empty for none: the first frame asks for the volumes.
    pub fn new(folders: Folders, place: &str, folder: &str) -> Self {
        let path = |text: &str| (!text.is_empty()).then(|| PathBuf::from(text));
        let mut places = Places {
            folders,
            place: path(place),
            folder: path(folder),
            commands: vec![Command::Library(LibraryCommand::ListVolumes)],
            ..Places::default()
        };
        // The tree opens from the place down to the folder's parent, the folder's row in
        // sight; the place itself as the folder, or one outside it, opens the place alone.
        if let Some(place) = places.place.clone() {
            let parent = (places.folder.as_deref())
                .and_then(Path::parent)
                .filter(|parent| parent.starts_with(&place));
            let open: Vec<PathBuf> = parent.map_or_else(
                || vec![place.clone()],
                |parent| {
                    (parent.ancestors())
                        .take_while(|folder| folder.starts_with(&place))
                        .map(Path::to_owned)
                        .collect()
                },
            );
            for folder in open.iter().rev() {
                places.open(folder);
            }
        }
        places
    }

    /// What to send the session now.
    pub fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }

    /// Every place, cards first, then the other volumes, then Pictures and Home.
    pub fn places(&self) -> Vec<Place> {
        let volumes = self.volumes.iter().flatten();
        let (cards, drives): (Vec<&Volume>, Vec<&Volume>) = volumes.partition(|v| v.card);
        let mut places: Vec<Place> = (cards.into_iter().chain(drives))
            .map(|volume| Place {
                root: volume.mount_point.clone(),
                name: volume.name.clone(),
                kind: if volume.card {
                    Kind::Card
                } else {
                    Kind::Drive {
                        removable: volume.removable,
                    }
                },
            })
            .collect();
        let folders = [
            (&self.folders.pictures, Kind::Pictures),
            (&self.folders.home, Kind::Home),
        ];
        for (root, kind) in folders {
            if let Some(root) = root {
                places.push(Place {
                    root: root.clone(),
                    name: String::new(),
                    kind,
                });
            }
        }
        places
    }

    /// The place chosen, where it is one of the places.
    pub fn chosen(&self) -> Option<Place> {
        let root = self.place.as_ref()?;
        self.places().into_iter().find(|place| place.root == *root)
    }

    /// The source the place and the folder chosen make, where there is one.
    pub fn source(&self) -> Option<Source> {
        let folder = self.folder.clone().or_else(|| self.place.clone())?;
        let place = self.chosen();
        let card = place.as_ref().is_some_and(|p| p.kind == Kind::Card);
        let name = match &place {
            Some(place) if card => place.name.clone(),
            _ => file_name(&folder),
        };
        Some(Source { folder, name, card })
    }

    /// The place and folder chosen, as `import.place` and `import.folder` keep them.
    pub fn kept(&self) -> [(&'static str, String); 2] {
        let text = |path: &Option<PathBuf>| {
            (path.as_ref()).map_or_else(String::new, |p| p.display().to_string())
        };
        [
            ("import.place", text(&self.place)),
            ("import.folder", text(&self.folder)),
        ]
    }

    /// Takes in one of the library's events, its own: the volumes, and a folder's
    /// subfolders it asked for. Returns the source as the first volumes come: a card among
    /// them chosen, else the place kept, now known for a card or not.
    pub fn receive(&mut self, event: &LibraryEvent) -> Option<Source> {
        match event {
            LibraryEvent::Volumes { volumes } => {
                self.volumes = Some(volumes.clone());
                if std::mem::replace(&mut self.opened, true) {
                    return None;
                }
                // The first card is chosen, unless it is the place kept; then, as every
                // place kept, it is told now for what it is, a card by its name.
                let card = volumes.iter().find(|volume| volume.card);
                match card.map(|card| card.mount_point.clone()) {
                    Some(root) if self.place.as_ref() != Some(&root) => self.choose(&root),
                    _ => self.source(),
                }
            }
            LibraryEvent::SubfoldersOf {
                folder,
                names,
                unreadable,
                ..
            } if self.asking.remove(folder) => {
                let listed = match unreadable {
                    Some(why) => Err(why.clone()),
                    None => Ok(names.clone()),
                };
                self.listed.insert(folder.clone(), listed);
                None
            }
            _ => None,
        }
    }

    /// Chooses the place at `root`, a card's `DCIM` its folder, else the place itself.
    /// Returns the source it makes.
    pub fn choose(&mut self, root: &Path) -> Option<Source> {
        self.place = Some(root.to_owned());
        let card = self.chosen().is_some_and(|place| place.kind == Kind::Card);
        let folder = if card {
            root.join("DCIM")
        } else {
            root.to_owned()
        };
        self.expanded.clear();
        self.open(root);
        self.folder = Some(folder);
        self.source()
    }

    /// Chooses the folder `folder` of the place's tree. Returns the source it makes.
    pub fn choose_folder(&mut self, folder: &Path) -> Option<Source> {
        self.folder = Some(folder.to_owned());
        self.source()
    }

    /// Opens `folder` in the tree, asking for its subfolders where they have not come.
    fn open(&mut self, folder: &Path) {
        self.expanded.insert(folder.to_owned());
        if !self.listed.contains_key(folder) && self.asking.insert(folder.to_owned()) {
            let folder = folder.to_owned();
            (self.commands).push(Command::Library(LibraryCommand::SourceSubfolders {
                folder,
            }));
        }
    }

    /// What the group says after its title while closed: "Card: EOS_DIGITAL", or the
    /// folder's name.
    pub fn summary(&self, words: &Translator) -> Option<String> {
        let source = self.source()?;
        if !source.card {
            return Some(source.name);
        }
        let mut args = Args::new();
        args.set("name", source.name);
        Some(words.text_with("import-source-card", Some(&args)))
    }

    /// Places: its label with ＋, − and ↺, drawn disabled, then a row for each place.
    /// Returns the source a place chosen makes.
    pub fn show_places(&mut self, ui: &mut egui::Ui, words: &Translator) -> Option<Source> {
        let t = theme(ui.ctx());
        let later = words.text("section-tab-later");
        let label = words.text("import-places");
        ui.horizontal(|ui| {
            ui.set_height(t.sizes.panel_line);
            ui.label(egui::RichText::new(&label).color(t.colors.text_secondary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (icon, name) in [
                    (Icon::Reset, "import-places-reset"),
                    (Icon::Minus, "import-places-remove"),
                    (Icon::Plus, "import-places-add"),
                ] {
                    let name = words.text(name);
                    IconButton::new(icon, &name)
                        .size(t.sizes.field)
                        .tip(&later)
                        .enabled(false)
                        .show(ui);
                }
            });
        });
        let places = self.places();
        let mut chosen = None;
        let ids: Vec<egui::Id> = (0..places.len())
            .map(|at| egui::Id::unique(("import place", at)))
            .collect();
        // A list to screen readers, named by its label.
        let list = UiBuilder::new()
            .id_salt("import places")
            .accessibility_role(egui::Role::List)
            .accessibility_label(&label);
        ui.scope_builder(list, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (at, place) in places.iter().enumerate() {
                let name = place.shown(words);
                let (response, _) = TreeRow {
                    id: Some(ids[at]),
                    level: 1,
                    name: &name,
                    expanded: None,
                    mark: None,
                    icon: Some(place.icon()),
                    count: None,
                    chosen: self.place.as_ref() == Some(&place.root),
                    enabled: true,
                    role: egui::Role::ListItem,
                    label: None,
                }
                .show(ui);
                if chose(ui, &response, &ids, at) {
                    chosen = Some(place.root.clone());
                }
            }
        });
        chosen.and_then(|root| self.choose(&root))
    }

    /// Folders: the tree of the place chosen, at most 8 rows tall, each folder's subfolders
    /// asked for as it opens. Returns the source a folder chosen makes.
    pub fn show_folders(&mut self, ui: &mut egui::Ui, words: &Translator) -> Option<Source> {
        let t = theme(ui.ctx());
        let place = self.chosen()?;
        let local = place.local().then(|| words.text("import-folders-local"));
        let mut rows = Vec::new();
        self.rows(&place.root, 1, &mut rows);
        let mut toggled = None;
        let mut chosen = None;
        let title = words.text("import-folders");
        label_line(ui, &title);
        {
            let most = MOST_ROWS * t.sizes.panel_line;
            // A tree to screen readers, as Collections' Folders is, named by its label.
            let tree = UiBuilder::new()
                .id_salt("import folders tree")
                .accessibility_role(egui::Role::Tree)
                .accessibility_label(&title);
            egui::ScrollArea::vertical()
                .id_salt("import folders")
                .max_height(most)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.scope_builder(tree, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let ids: Vec<egui::Id> = (rows.iter())
                            .map(|row| egui::Id::unique(("import folder", &row.path)))
                            .collect();
                        for (at, row) in rows.iter().enumerate() {
                            let name = match (&row.unreadable, at) {
                                (Some(why), _) => why.clone(),
                                (None, 0) => place.shown(words),
                                (None, _) => file_name(&row.path),
                            };
                            let (response, chevron) = TreeRow {
                                id: Some(ids[at]),
                                level: row.level,
                                name: &name,
                                expanded: row.expanded,
                                mark: None,
                                icon: Some(Icon::Folder),
                                count: None,
                                chosen: self.folder.as_ref() == Some(&row.path),
                                enabled: row.unreadable.is_none(),
                                role: egui::Role::TreeItem,
                                label: None,
                            }
                            .show(ui);
                            let response = match &local {
                                Some(local) => response.on_hover_text(local),
                                None => response,
                            };
                            let on_chevron = response.clicked()
                                && (response.interact_pointer_pos())
                                    .is_some_and(|at| chevron.contains(at));
                            // ← closes a folder, → opens it, as Collections' tree's keys do.
                            let key = |key: egui::Key| {
                                response.has_focus()
                                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))
                            };
                            let (left, right) =
                                (key(egui::Key::ArrowLeft), key(egui::Key::ArrowRight));
                            let keyed = match row.expanded {
                                Some(open) => (left && open) || (right && !open),
                                None => false,
                            };
                            if row.expanded.is_some() && (on_chevron || keyed) {
                                toggled = Some(row.path.clone());
                            } else if row.unreadable.is_none() && chose(ui, &response, &ids, at) {
                                chosen = Some(row.path.clone());
                            }
                        }
                    });
                });
        }
        if let Some(folder) = toggled
            && !self.expanded.remove(&folder)
        {
            self.open(&folder);
        }
        chosen.and_then(|folder| self.choose_folder(&folder))
    }

    /// The tree's rows from `folder` at `level` down, its open folders' subfolders under
    /// them, a level that cannot be read a row saying why.
    fn rows(&self, folder: &Path, level: usize, into: &mut Vec<Row>) {
        let listed = self.listed.get(folder);
        let open = self.expanded.contains(folder);
        let expanded = match listed {
            Some(Ok(names)) if names.is_empty() => None,
            _ => Some(open),
        };
        into.push(Row {
            path: folder.to_owned(),
            level,
            expanded,
            unreadable: None,
        });
        if !open {
            return;
        }
        match listed {
            Some(Ok(names)) => {
                for name in names {
                    self.rows(&folder.join(name), level + 1, into);
                }
            }
            Some(Err(why)) => into.push(Row {
                path: folder.join(".unreadable"),
                level: level + 1,
                expanded: None,
                unreadable: Some(why.clone()),
            }),
            None => {}
        }
    }
}

/// A row of the Folders tree.
struct Row {
    path: PathBuf,
    level: usize,
    expanded: Option<bool>,
    unreadable: Option<String>,
}

/// Whether the row `at` of a list whose rows are `ids` was chosen: clicked, or Return or
/// Space pressed on it; ↑ and ↓ on it move the focus along the list.
fn chose(ui: &egui::Ui, response: &egui::Response, ids: &[egui::Id], at: usize) -> bool {
    if !response.has_focus() {
        return response.clicked();
    }
    let pressed = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
    if pressed(egui::Key::ArrowDown)
        && let Some(next) = ids.get(at + 1)
    {
        ui.memory_mut(|m| m.request_focus(*next));
    }
    if pressed(egui::Key::ArrowUp)
        && let Some(previous) = at.checked_sub(1).and_then(|at| ids.get(at))
    {
        ui.memory_mut(|m| m.request_focus(*previous));
    }
    response.clicked() || pressed(egui::Key::Enter) || pressed(egui::Key::Space)
}

/// The last part of `path`, as text, or the whole where it has none.
fn file_name(path: &Path) -> String {
    (path.file_name()).map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// A list's label on a line of its own, in `text_secondary`, as Places' is.
fn label_line(ui: &mut egui::Ui, label: &str) {
    let t = theme(ui.ctx());
    ui.horizontal(|ui| {
        ui.set_height(t.sizes.panel_line);
        ui.label(egui::RichText::new(label).color(t.colors.text_secondary));
    });
}
