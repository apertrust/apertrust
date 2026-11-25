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

//! The Metadata tool (decision 110; `docs/gui.md`, The Metadata tool), read-only for one
//! photo: darktable's image information in the Photos tab, its groups after EXIF and IPTC
//! Core declared in `apertrust_tools::utility::metadata`.
//!
//! - **Its facts** are the active photo's, from the session's `Photo`, its EXIF and its
//!   history, worded by [`mod@format`], each group's drawn as one [`FactList`]; Rating, Colour
//!   labels and Show on Map are drawn disabled until what they call is built.
//! - **Its view** ([`View`]), which rows show and which groups are open, is three settings,
//!   `metadata.shown`, `metadata.hidden` and `metadata.open`, kept against the
//!   declaration; ＋ shows or hides one row, and its presets, the built-in three and the
//!   person's, hold the view and nothing else.
//! - **Its ⋯** holds Copy Information and Copy Coordinates, its own items until decision
//!   98's generated actions.

pub mod format;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use apertrust_base::settings::Store;
use apertrust_engine::session::Photo;
use apertrust_tools::settings::registry::{METADATA_HIDDEN, METADATA_OPEN, METADATA_SHOWN};
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::utility::metadata::{ROWS, group_of, widget_row};
use apertrust_tools::{Cut, Fact, FactEnd as FactEndDef, Row, Widget, messages};
use apertrust_ui::components::{self as parts, FactCut, FactEnd, FactList, FactMarks, Marks};
use apertrust_ui::i18n::Args;
use apertrust_ui::popup::{Entry, Item, Submenu};
use apertrust_ui::{Icon, Translator, theme};
use eframe::egui;

use crate::library;
use crate::mirror::Mirror;
use crate::presets::{self, Item as PresetItem, Line, Menu, Settings};
use crate::utility::{Context, UtilityTool};

/// The tool's id.
pub const ID: &str = "metadata";
/// Its own ⋯ items, by their ids.
pub const COPY_INFORMATION: &str = "metadata-copy-information";
pub const COPY_COORDINATES: &str = "metadata-copy-coordinates";
/// A preset's setting that only a preset holds: its rows shown are the only ones shown.
const SHOWN_ONLY: &str = "metadata.shown_only";

/// The Metadata tool's declaration, in `UTILITIES`.
pub fn definition() -> &'static UtilityDefinition {
    apertrust_tools::utility::utility(ID).expect("Metadata is built in")
}

/// The tool's view, its three settings: the rows shown that the declaration hides, the
/// rows hidden that it shows, each by id in the declaration's order, and the groups open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub shown: Vec<String>,
    pub hidden: Vec<String>,
    pub open: Vec<String>,
}

impl Default for View {
    /// The schema's defaults: the declaration's rows, Basic, Exposure and Content open.
    fn default() -> View {
        View {
            shown: owned(METADATA_SHOWN.default()),
            hidden: owned(METADATA_HIDDEN.default()),
            open: owned(METADATA_OPEN.default()),
        }
    }
}

/// A list setting's default as the view keeps it.
fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

/// Whether the row `id` shows at first, as the tool declares it; none for a row this build
/// does not have.
fn declared(id: &str) -> Option<bool> {
    let rows = ROWS.iter().flat_map(|group| group.rows.iter());
    rows.into_iter()
        .find(|row| row.id == id)
        .map(|row| row.shown)
}

impl View {
    /// The view as `store` holds it.
    pub fn read(store: &Store) -> View {
        View {
            shown: store.get(&METADATA_SHOWN),
            hidden: store.get(&METADATA_HIDDEN),
            open: store.get(&METADATA_OPEN),
        }
    }

    /// Sets in `store` what differs from the view as it holds it.
    pub fn write(&self, store: &mut Store) {
        let was = View::read(store);
        if self.shown != was.shown {
            store.set(&METADATA_SHOWN, self.shown.clone());
        }
        if self.hidden != was.hidden {
            store.set(&METADATA_HIDDEN, self.hidden.clone());
        }
        if self.open != was.open {
            store.set(&METADATA_OPEN, self.open.clone());
        }
    }

    /// Whether the row `id` shows: as declared, unless the view says otherwise.
    pub fn shows(&self, id: &str) -> bool {
        let at_first = declared(id).unwrap_or(false);
        let listed = |list: &[String]| list.iter().any(|row| row == id);
        if at_first {
            !listed(&self.hidden)
        } else {
            listed(&self.shown)
        }
    }

    /// The row `id` shown where it is hidden, hidden where shown.
    pub fn toggle(&mut self, id: &str) {
        let Some(at_first) = declared(id) else {
            return;
        };
        let list = if at_first {
            &mut self.hidden
        } else {
            &mut self.shown
        };
        match list.iter().position(|row| row == id) {
            Some(at) => drop(list.remove(at)),
            None => list.push(id.to_owned()),
        }
        self.normalise();
    }

    /// Whether the group `id` is open.
    pub fn opens(&self, group: &str) -> bool {
        self.open.iter().any(|open| open == group)
    }

    /// The group `id` opened or closed.
    pub fn set_open(&mut self, group: &str, open: bool) {
        self.open.retain(|kept| kept != group);
        if open {
            self.open.push(group.to_owned());
        }
        self.normalise();
    }

    /// Each list in the declaration's order, each id once, so two views that show alike
    /// are equal.
    fn normalise(&mut self) {
        let rows: Vec<&str> = (ROWS.iter())
            .flat_map(|group| group.rows.iter().map(|row| row.id))
            .collect();
        let groups: Vec<&str> = ROWS.iter().map(|group| group.group).collect();
        let order = |list: &mut Vec<String>, by: &[&str]| {
            list.sort_by_key(|id| by.iter().position(|known| known == id));
            list.dedup();
        };
        order(&mut self.shown, &rows);
        order(&mut self.hidden, &rows);
        order(&mut self.open, &groups);
    }

    /// The view a preset's `settings` give over this build's defaults (decision 106): its
    /// lists, ids joined by spaces, a row this build lacks skipped, and with
    /// `metadata.shown_only` its rows shown the only ones shown.
    pub fn from_settings(settings: &Settings) -> View {
        let list = |key: &str| -> Option<Vec<String>> {
            let (_, value) = settings.iter().find(|(k, _)| k == key)?;
            Some(value.split_whitespace().map(str::to_owned).collect())
        };
        let only = settings
            .iter()
            .any(|(key, value)| key == SHOWN_ONLY && value == "true");
        let defaults = View::default();
        let mut view = View {
            shown: Vec::new(),
            hidden: Vec::new(),
            open: list(METADATA_OPEN.id).unwrap_or(defaults.open),
        };
        let named = list(METADATA_SHOWN.id).unwrap_or_default();
        let named = named.iter().filter(|id| declared(id).is_some());
        if only {
            let named: BTreeSet<&String> = named.collect();
            for row in ROWS.iter().flat_map(|group| group.rows.iter()) {
                let wanted = named.iter().any(|id| *id == row.id);
                if wanted && !row.shown {
                    view.shown.push(row.id.to_owned());
                } else if !wanted && row.shown {
                    view.hidden.push(row.id.to_owned());
                }
            }
        } else {
            view.shown = (named.filter(|id| declared(id) == Some(false)).cloned()).collect();
            let hidden = list(METADATA_HIDDEN.id).unwrap_or_default();
            view.hidden = (hidden.into_iter())
                .filter(|id| declared(id) == Some(true))
                .collect();
        }
        view.open
            .retain(|group| ROWS.iter().any(|listed| listed.group == group));
        view.normalise();
        view
    }

    /// The view as a preset holds it: each list as its ids joined by spaces.
    pub fn settings(&self) -> Settings {
        vec![
            (METADATA_SHOWN.id.to_owned(), self.shown.join(" ")),
            (METADATA_HIDDEN.id.to_owned(), self.hidden.join(" ")),
            (METADATA_OPEN.id.to_owned(), self.open.join(" ")),
        ]
    }

    /// Its setting `key`'s value as `self` holds it, where `key` is one of its three.
    fn get(&self, key: &str) -> Option<&Vec<String>> {
        match key {
            "metadata.shown" => Some(&self.shown),
            "metadata.hidden" => Some(&self.hidden),
            "metadata.open" => Some(&self.open),
            _ => None,
        }
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Vec<String>> {
        match key {
            "metadata.shown" => Some(&mut self.shown),
            "metadata.hidden" => Some(&mut self.hidden),
            "metadata.open" => Some(&mut self.open),
            _ => None,
        }
    }
}

/// The tool's state: its view, the keys `--conf` gives for this run, its built-in presets
/// and the person's home, which a folder is shown under.
#[derive(Debug)]
pub struct Metadata {
    view: View,
    given: BTreeSet<String>,
    builtins: Vec<(&'static str, View)>,
    home: Option<PathBuf>,
    /// The labels' column and the pass it was measured in, measured once a pass.
    column: Option<(u64, f32)>,
}

impl Metadata {
    /// The tool over `view`.
    pub fn new(view: View) -> Metadata {
        Metadata {
            view,
            given: BTreeSet::new(),
            builtins: (presets::builtins(ID).into_iter())
                .map(|(id, settings)| (id, View::from_settings(&settings)))
                .collect(),
            home: crate::export::places().home,
            column: None,
        }
    }

    /// The tool with the keys of its settings `--conf` gives for this run: a preset or ＋
    /// keeps them.
    pub fn given(mut self, keys: impl IntoIterator<Item = String>) -> Metadata {
        self.given = keys.into_iter().collect();
        self
    }

    /// The tool with a home of its own, for tests.
    #[cfg(test)]
    pub fn with_home(mut self, home: Option<PathBuf>) -> Metadata {
        self.home = home;
        self
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    /// `view` with each setting `--conf` gives at its value now.
    fn kept(&self, view: View) -> View {
        kept(view, &self.view, &self.given)
    }

    /// Applies a Metadata preset whose settings are `settings` (decision 106), with no
    /// history step: the view it gives over the defaults, but the settings `--conf` gives,
    /// which keep theirs. Default settings is the preset of none.
    pub fn apply(&mut self, settings: &Settings) {
        self.view = self.kept(View::from_settings(settings));
    }

    /// Default settings, a built-in preset or the person's in `folder`, `item`, applied
    /// ([`Self::apply`]); the name of the person's preset applied, which Update, Rename… and
    /// Delete… then act on. One that cannot be applied changes nothing, with a warning under
    /// `-d settings`.
    pub fn apply_item(&mut self, folder: &Path, item: &PresetItem) -> Option<String> {
        let (settings, own) = match item {
            PresetItem::Default => (Some(Vec::new()), None),
            PresetItem::Builtin(id) => {
                let builtins = presets::builtins(ID).into_iter();
                let found = builtins.into_iter().find(|(own, _)| own == id);
                (found.map(|(_, settings)| settings), None)
            }
            PresetItem::Apply(name) => {
                let listed = presets::read_settings(folder, ID).into_iter();
                let found = listed.into_iter().find(|preset| preset.name == *name);
                (
                    found.and_then(|preset| preset.values.ok()),
                    Some(name.clone()),
                )
            }
            _ => (None, None),
        };
        let Some(settings) = settings else {
            tracing::warn!(target: "settings", "metadata: preset {} cannot be applied", item.id());
            return None;
        };
        self.apply(&settings);
        own
    }

    /// The settings a preset saved now holds: the view, but what `--conf` gives.
    pub fn preset_settings(&self) -> Settings {
        let mut settings = self.view.settings();
        settings.retain(|(key, _)| !self.given.contains(key));
        settings
    }

    /// ⋯'s own items in `words`: Copy Information, and Copy Coordinates while `photo` has
    /// a location; neither without a photo.
    pub fn options(words: &Translator, photo: Option<&Photo>) -> Vec<Entry> {
        let located = photo.is_some_and(|photo| format::coordinates(&photo.exif).is_some());
        let information = Item::new(COPY_INFORMATION, words.text(COPY_INFORMATION));
        let coordinates = Item::new(COPY_COORDINATES, words.text(COPY_COORDINATES));
        vec![
            Entry::Item(information.enabled(photo.is_some())),
            Entry::Item(coordinates.enabled(located)),
        ]
    }

    /// What ⋯'s own item `id` copies of the photo the tool describes ([`described`]):
    /// every row shown, "Label: value" a line, or its coordinates.
    pub fn copied(&self, id: &str, cx: &Context<'_>) -> Option<String> {
        let photo = described(cx.mirror)?;
        match id {
            COPY_INFORMATION => Some(self.information(photo, cx)),
            COPY_COORDINATES => format::coordinates(&photo.exif),
            _ => None,
        }
    }

    /// Every row shown of `photo`, in the groups' order, "Label: value" a line, "—" for
    /// none.
    fn information(&self, photo: &Photo, cx: &Context<'_>) -> String {
        let source = self.source(photo, cx);
        let mut lines = Vec::new();
        for row in ROWS.iter().flat_map(|group| group.rows.iter()) {
            if !self.view.shows(row.id) {
                continue;
            }
            let label = cx.words.text(&label_id(row.id));
            let value = format::fact(row.id, &source).value;
            lines.push(format!("{label}: {}", value.as_deref().unwrap_or("—")));
        }
        lines.join("\n")
    }

    fn source<'a>(&'a self, photo: &'a Photo, cx: &'a Context<'_>) -> format::Source<'a> {
        format::Source {
            photo,
            history: cx.mirror.history.as_ref(),
            home: self.home.as_deref(),
            words: cx.words,
        }
    }

    /// The labels' column, as wide as the longest label shown at most 40% of the row,
    /// measured once a pass and shared by every list and row of marks drawn in it.
    fn column(&mut self, ui: &egui::Ui, words: &Translator) -> f32 {
        let pass = ui.ctx().cumulative_pass_nr();
        if let Some((at, width)) = self.column
            && at == pass
        {
            return width;
        }
        let labels: Vec<String> = (ROWS.iter())
            .flat_map(|group| group.rows.iter())
            .filter(|row| self.view.shows(row.id))
            .map(|row| words.text(&label_id(row.id)))
            .collect();
        let width = FactList::column(ui, labels.iter().map(String::as_str));
        self.column = Some((pass, width));
        width
    }

    /// The facts of `facts` as `cx` gives the photo, each a row of the list.
    fn rows(&self, facts: &[&Fact], cx: &Context<'_>) -> Vec<Drawn> {
        let words = cx.words;
        let subject = Subject::of(cx);
        facts
            .iter()
            .map(|fact| {
                let [label, tag, none] =
                    messages::fact_words(ID, fact.id).map(|id| words.text(&id));
                let mut row = Drawn {
                    label,
                    value: None,
                    tip: String::new(),
                    cut: match (fact.cut, fact.id) {
                        (Cut::Middle, "file_name") => FactCut::File,
                        (Cut::Middle, _) => FactCut::Middle,
                        (Cut::End, _) => FactCut::End,
                    },
                    end: None,
                    enabled: true,
                };
                match &subject {
                    Subject::None => {
                        row.enabled = false;
                        row.tip = words.text("metadata-select-a-photo");
                    }
                    Subject::Several { count, names } if fact.id == "file_name" => {
                        let mut args = Args::new();
                        args.set("count", *count as f64);
                        row.value = Some(words.text_with("photo-count", Some(&args)));
                        row.tip = names.join("\n");
                    }
                    Subject::Several { .. } => {
                        row.enabled = false;
                        row.tip = words.text("metadata-several-later");
                    }
                    Subject::One(photo) => {
                        let shown = format::fact(fact.id, &self.source(photo, cx));
                        let why = if photo.exif.unread.contains(&exif_field(fact.id)) {
                            let mut args = Args::new();
                            args.set("format", photo.format.as_str());
                            words.text_with("metadata-fact-unread", Some(&args))
                        } else {
                            none
                        };
                        let first = shown.value.clone().unwrap_or(why);
                        let lines = [Some(first), shown.note, Some(tag)];
                        row.tip = lines.into_iter().flatten().collect::<Vec<_>>().join("\n");
                        row.value = shown.value;
                        if fact.end == Some(FactEndDef::OpenFolder) {
                            row.end = Some(open_folder(photo, cx));
                        }
                    }
                }
                row
            })
            .collect()
    }
}

/// `view` with each setting of `given` at its value in `now`.
fn kept(mut view: View, now: &View, given: &BTreeSet<String>) -> View {
    for key in given {
        if let (Some(now), Some(into)) = (now.get(key), view.get_mut(key)) {
            into.clone_from(now);
        }
    }
    view
}

/// The active photo, where one is open.
fn open_photo<'a>(cx: &'a Context<'_>) -> Option<&'a Photo> {
    match &cx.mirror.photo {
        crate::mirror::PhotoState::Open(photo) => Some(photo),
        _ => None,
    }
}

/// The one photo the tool describes and ⋯ copies: the active photo, but none while several
/// are selected, whose facts the tool does not read yet.
pub fn described(mirror: &Mirror) -> Option<&Photo> {
    match &mirror.photo {
        crate::mirror::PhotoState::Open(photo) if mirror.selected_count() <= 1 => Some(photo),
        _ => None,
    }
}

/// The [`apertrust_engine::session::Exif`] field the fact `id` reads, by the name
/// `Exif::unread` lists it under: the fact's own id but where the two differ.
fn exif_field(id: &str) -> &str {
    match id {
        "make" => "make_written",
        "model" => "model_written",
        "date_taken" => "taken",
        "orientation" => "orientation_code",
        "subject_distance" => "focus_distance",
        "altitude" => "elevation",
        "direction" => "gps_direction",
        "speed" => "gps_speed",
        "accuracy" => "gps_accuracy",
        "satellites" => "gps_satellites",
        "map_datum" => "gps_map_datum",
        id => id,
    }
}

/// The message of the row `id`'s label: a fact's, or Rating's and Colour labels'.
fn label_id(id: &str) -> String {
    messages::fact_words(ID, id)[0].clone()
}

/// What the tool describes this frame (decision 110): no photo, the active photo alone, or
/// several selected, how many and the first five's names.
enum Subject<'a> {
    None,
    One(&'a Photo),
    Several { count: usize, names: Vec<String> },
}

impl<'a> Subject<'a> {
    fn of(cx: &'a Context<'_>) -> Subject<'a> {
        let count = cx.mirror.selected_count();
        if count > 1 {
            let names = (cx.mirror.selection.photos.iter())
                .take(5)
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .collect();
            return Subject::Several { count, names };
        }
        match open_photo(cx) {
            Some(photo) => Subject::One(photo),
            None => Subject::None,
        }
    }
}

/// A row as the fact list draws it, its words owned.
struct Drawn {
    label: String,
    value: Option<String>,
    tip: String,
    cut: FactCut,
    end: Option<(bool, String, Option<library::Action>)>,
    enabled: bool,
}

/// Open This Folder on `photo`'s folder: whether it can run, its tooltip, and what it does.
/// In folder mode it chooses the folder in Collections' tree, for photos opened alone it
/// opens it, as Collections' own does; it rests where the folder is the one open.
fn open_folder(photo: &Photo, cx: &Context<'_>) -> (bool, String, Option<library::Action>) {
    let words = cx.words;
    let Some(folder) = photo.path.parent().map(Path::to_owned) else {
        return (false, String::new(), None);
    };
    let open = match &cx.mirror.folder.folder {
        crate::folder::Folder::Open(listing) => Some(listing.path.as_path()),
        _ => None,
    };
    if open == Some(folder.as_path()) {
        return (false, words.text("metadata-folder-open"), None);
    }
    let root = cx.mirror.folder.root();
    let action = match root {
        Some(root) if open.is_some() && folder.starts_with(root) => library::Action::Choose(folder),
        _ => library::Action::Open(folder),
    };
    (true, String::new(), Some(action))
}

impl UtilityTool for Metadata {
    type Action = library::Action;

    fn definition(&self) -> &'static UtilityDefinition {
        definition()
    }

    /// Rating and Colour labels in Basic, and Show on Map after GPS, each drawn disabled
    /// until what it calls is built.
    fn body(
        &mut self,
        widget: Widget,
        ui: &mut egui::Ui,
        cx: &mut Context<'_>,
    ) -> Vec<library::Action> {
        let words = cx.words;
        match widget {
            Widget::MetadataRating | Widget::MetadataLabels => {
                let column = self.column(ui, words);
                let id = widget_row(widget).unwrap_or_default();
                let (later, marks) = match widget {
                    Widget::MetadataRating => ("metadata-rating-later", Marks::Stars),
                    _ => ("metadata-labels-later", Marks::Labels),
                };
                let (label, tip) = (words.text(&label_id(id)), words.text(later));
                FactMarks::new(&label, &tip, marks, column).show(ui);
            }
            // At the row's end, where a fact's end item sits.
            Widget::MetadataMap => {
                let tip = words.text("metadata-map-later");
                ui.add_space(theme(ui.ctx()).spacing.xs);
                let end = egui::Layout::right_to_left(egui::Align::Center);
                ui.with_layout(end, |ui| {
                    parts::Button::new(&words.text("metadata-map"))
                        .enabled(false)
                        .tip(&tip)
                        .show(ui);
                });
            }
            _ => {}
        }
        Vec::new()
    }

    fn facts(
        &mut self,
        ui: &mut egui::Ui,
        facts: &[&Fact],
        cx: &mut Context<'_>,
    ) -> Vec<library::Action> {
        let column = self.column(ui, cx.words);
        let drawn = self.rows(facts, cx);
        let ends: Vec<Option<FactEnd<'_>>> = drawn
            .iter()
            .map(|row| {
                row.end.as_ref().map(|(enabled, tip, _)| FactEnd {
                    icon: Icon::ChooseFolder,
                    name: "",
                    enabled: *enabled,
                    tip: (!tip.is_empty()).then_some(tip.as_str()),
                })
            })
            .collect();
        let open_name = cx.words.text("collections-open-this-folder");
        let list: Vec<parts::Fact<'_>> = (drawn.iter().zip(ends))
            .map(|(row, end)| parts::Fact {
                label: &row.label,
                value: row.value.as_deref(),
                tip: &row.tip,
                cut: row.cut,
                end: end.map(|end| FactEnd {
                    name: &open_name,
                    ..end
                }),
                enabled: row.enabled,
            })
            .collect();
        let group = facts
            .first()
            .and_then(|fact| group_of(fact.id))
            .unwrap_or(ID);
        let name = cx.words.text(&messages::group_title(
            ID,
            group,
            apertrust_tools::GroupKind::Collapsible,
        ));
        let out = FactList::new(&name, &list).column_width(column).show(ui);
        out.end
            .and_then(|at| drawn[at].end.as_ref())
            .and_then(|(_, _, action)| action.clone())
            .into_iter()
            .collect()
    }

    /// A row ＋ hid, and a group whose every row it hid.
    fn hidden(&self, row: &Row) -> bool {
        match row {
            Row::Fact(fact) => !self.view.shows(fact.id),
            Row::Custom { widget, .. } => {
                widget_row(*widget).is_some_and(|id| !self.view.shows(id))
            }
            Row::Group(group) => {
                let listed = ROWS.iter().find(|listed| listed.group == group.id);
                listed.is_some_and(|listed| {
                    !listed.rows.is_empty() && listed.rows.iter().all(|r| !self.view.shows(r.id))
                })
            }
            _ => false,
        }
    }

    fn open(&self, group: &str) -> Option<bool> {
        Some(self.view.opens(group))
    }

    fn set_open(&mut self, group: &str, open: bool) {
        if !self.given.contains(METADATA_OPEN.id) {
            self.view.set_open(group, open);
        }
    }

    /// The groups in order, each a submenu of its rows with a ✓ on each shown; a group
    /// with no rows yet listed disabled.
    fn rows_menu(&self, words: &Translator) -> Vec<Entry> {
        let later = words.text("section-tab-later");
        ROWS.iter()
            .map(|listed| {
                let title = words.text(&messages::group_title(
                    ID,
                    listed.group,
                    apertrust_tools::GroupKind::Collapsible,
                ));
                if listed.rows.is_empty() {
                    let item = Item::new(listed.group, title).enabled(false);
                    return Entry::Item(item.tip(later.clone()));
                }
                let rows = (listed.rows.iter())
                    .map(|row| {
                        let label = words.text(&label_id(row.id));
                        Entry::Item(Item::new(row.id, label).checked(self.view.shows(row.id)))
                    })
                    .collect();
                Entry::Submenu(Submenu::new(listed.group, title, rows))
            })
            .collect()
    }

    fn row_chosen(&mut self, id: &str) {
        let key = match declared(id) {
            Some(true) => METADATA_HIDDEN.id,
            _ => METADATA_SHOWN.id,
        };
        if !self.given.contains(key) {
            self.view.toggle(id);
        }
    }

    /// Its presets' pull-down ([`presets::pull_down`]): Default settings, then Essentials,
    /// Everything and IPTC for agencies, then the person's, ✓ on each whose view this is.
    fn presets(&self, menu: &Menu) -> Option<crate::utility::Presets> {
        let (menu, view, given) = (menu.clone(), self.view.clone(), self.given.clone());
        let builtins = self.builtins.clone();
        Some(Box::new(move |words: &Translator| {
            let holds = |preset: View| kept(preset, &view, &given) == view;
            let builtins = (builtins.into_iter())
                .map(|(id, preset)| Line {
                    item: PresetItem::Builtin(id.to_owned()),
                    name: presets::builtin_name(ID, id, words),
                    checked: Ok(holds(preset)),
                })
                .collect();
            let title = words.text(&format!("section-{ID}"));
            let own = (presets::read_settings(&menu.folder, ID).into_iter())
                .map(|preset| Line {
                    item: PresetItem::Apply(preset.name.clone()),
                    checked: match &preset.values {
                        Ok(settings) => Ok(holds(View::from_settings(settings))),
                        Err(why) => Err(presets::parked(words, why, &title)),
                    },
                    name: preset.name,
                })
                .collect();
            let at_defaults = holds(View::default());
            presets::pull_down(at_defaults, [builtins, own], &menu, words)
        }))
    }
}
