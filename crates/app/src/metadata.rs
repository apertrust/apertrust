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
pub mod several;
#[cfg(test)]
pub(crate) mod tests;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use apertrust_base::settings::Store;
use apertrust_engine::session::Photo;
use apertrust_tools::settings::registry::{METADATA_HIDDEN, METADATA_OPEN, METADATA_SHOWN};
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::utility::metadata::{ROWS, group_of, widget_row};
use apertrust_tools::{Cut, Fact, FactEnd as FactEndDef, Row, Widget, messages};
use apertrust_ui::components::{
    self as parts, ColourLabel, FactCut, FactEnd, FactList, FactMarks, Marks, RatingNotes,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::popup::{Entry, Item, Submenu};
use apertrust_ui::{Icon, Translator, theme};
use eframe::egui;

use crate::actions::ActionId;
use crate::library;
use crate::mirror::Mirror;
use crate::presets::{self, Item as PresetItem, Line, Menu, Settings};
use crate::utility::{Context, UtilityTool};
use apertrust_engine::session::facts::{FactSummary, FactValue, Key, Summary};
use apertrust_engine::session::{Command, Rating};
use std::sync::Arc;

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
    /// The described photos' ratings and the pass they were gathered in, once a pass for
    /// Rating's and Colour labels' rows.
    rated: Option<(u64, Rated)>,
    /// The summary of several photos asked for, and the one held.
    asking: several::Asking,
    /// The rating's actions a click on Rating's stars or a label's dot asked for, which the
    /// app runs as the keys run them ([`Metadata::take_ran`]).
    ran: Vec<ActionId>,
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
            rated: None,
            asking: several::Asking::default(),
            ran: Vec::new(),
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

    /// ⋯'s own items in `words` for what the tool describes of `mirror`: Copy Information,
    /// and Copy Coordinates while the photo described has a location, or every photo
    /// selected the same one, disabled where they were taken in different places; neither
    /// without a photo, nor before several photos' summary comes.
    pub fn options(&self, words: &Translator, mirror: &Mirror) -> Vec<Entry> {
        let information = Item::new(COPY_INFORMATION, words.text(COPY_INFORMATION));
        let coordinates = Item::new(COPY_COORDINATES, words.text(COPY_COORDINATES));
        let (described, located) = match self.coordinates(mirror) {
            Located::Here(_) => (true, true),
            Located::Nowhere(described) => (described, false),
            Located::Apart => {
                let apart = words.text("metadata-different-places");
                return vec![
                    Entry::Item(information),
                    Entry::Item(coordinates.enabled(false).tip(apart)),
                ];
            }
        };
        vec![
            Entry::Item(information.enabled(described)),
            Entry::Item(coordinates.enabled(located)),
        ]
    }

    /// Where the photos the tool describes were taken, as Copy Coordinates copies it.
    fn coordinates(&self, mirror: &Mirror) -> Located {
        if mirror.selected_count() <= 1 {
            let photo = described(mirror);
            return match photo.and_then(|photo| format::coordinates(&photo.exif)) {
                Some(here) => Located::Here(here),
                None => Located::Nowhere(photo.is_some()),
            };
        }
        let Some(summary) = self.held(mirror) else {
            return Located::Nowhere(false);
        };
        let same = |id| match summary.get(id) {
            Some(FactSummary::Same(value)) => value.shot.clone(),
            _ => None,
        };
        let (latitude, longitude) = (summary.get("latitude"), summary.get("longitude"));
        match (same("latitude"), same("longitude")) {
            (Some(shot), Some(_)) => match format::coordinates(&shot.exif) {
                Some(here) => Located::Here(here),
                None => Located::Nowhere(true),
            },
            // None recorded, or still being read: disabled, saying nothing of places.
            _ if [latitude, longitude]
                .iter()
                .any(|fact| matches!(fact, Some(FactSummary::Waiting { .. }))) =>
            {
                Located::Nowhere(true)
            }
            _ if matches!(latitude, Some(FactSummary::None))
                && matches!(longitude, Some(FactSummary::None)) =>
            {
                Located::Nowhere(true)
            }
            // Two places or more on either axis; one, with photos that have none, says
            // nothing of places.
            _ if [latitude, longitude].iter().any(|fact| {
                matches!(fact, Some(FactSummary::Different { values, more, .. })
                    if values.len() + more > 1)
            }) =>
            {
                Located::Apart
            }
            _ => Located::Nowhere(true),
        }
    }

    /// What ⋯'s own item `id` copies of what the tool describes: every row shown as it is
    /// drawn, "Label: value" a line, or the coordinates.
    pub fn copied(&self, id: &str, cx: &Context<'_>) -> Option<String> {
        match id {
            COPY_INFORMATION => Some(self.information(cx)),
            COPY_COORDINATES => match self.coordinates(cx.mirror) {
                Located::Here(here) => Some(here),
                _ => None,
            },
            _ => None,
        }
    }

    /// Every row shown, in the groups' order, as it is drawn, "Label: value" a line, "—" for
    /// none.
    fn information(&self, cx: &Context<'_>) -> String {
        let subject = Subject::of(cx);
        let mut lines = Vec::new();
        for row in ROWS.iter().flat_map(|group| group.rows.iter()) {
            if !self.view.shows(row.id) {
                continue;
            }
            let label = cx.words.text(&label_id(row.id));
            let marks = [Widget::MetadataRating, Widget::MetadataLabels]
                .into_iter()
                .find(|widget| widget_row(*widget) == Some(row.id));
            let value = match marks {
                Some(widget) => Rated::of(cx).text(widget, cx.words, &RatingNotes::default()),
                None => self.worded(row.id, &subject, cx).value,
            };
            let value = value.filter(|value| !value.is_empty());
            lines.push(format!("{label}: {}", value.as_deref().unwrap_or("—")));
        }
        lines.join("\n")
    }

    /// What to send the session while the tool is `shown` and open: a `Summarise` of the
    /// selection when two or more photos are, once for each selection ([`several::Asking`]).
    pub fn ask(&mut self, shown: bool, mirror: &Mirror) -> Option<Command> {
        let selection = &mirror.selection;
        // Again as the open photo's edit turns, or a batch ends: neither tells a new
        // selection.
        let edited = (mirror.values.edited, mirror.batches_ended);
        (self.asking).ask(shown, selection.through, edited, &selection.photos)
    }

    /// The described photos' ratings, gathered once a pass for both rows.
    fn rated(&mut self, pass: u64, cx: &Context<'_>) -> Rated {
        if let Some((at, rated)) = &self.rated
            && *at == pass
        {
            return rated.clone();
        }
        let rated = Rated::of(cx);
        self.rated = Some((pass, rated.clone()));
        rated
    }

    /// The rating's actions Rating's and Colour labels' marks asked for since, to run.
    pub fn take_ran(&mut self) -> Vec<ActionId> {
        std::mem::take(&mut self.ran)
    }

    /// The `Summarise` went out as the command numbered `number`.
    pub fn sent(&mut self, number: u64) {
        self.asking.sent(number);
    }

    /// The session's summary for the command numbered `through`.
    pub fn receive(&mut self, through: u64, summary: Arc<Summary>) {
        self.asking.receive(through, summary);
    }

    /// The summary of the selection `mirror` shows, where that is the session's and the
    /// summary is of it: none while a selection command is on its way, or the summary held is
    /// an earlier selection's, so nothing of another selection is ever drawn.
    fn held(&self, mirror: &Mirror) -> Option<&Summary> {
        if !mirror.selection_settled() {
            return None;
        }
        self.asking.held(mirror.selection.through)
    }

    /// How long until the rows waiting for a summary show "…"; none where nothing waits.
    pub fn until_dotted(&self) -> Option<std::time::Duration> {
        self.asking.until_dotted()
    }

    fn source<'a>(&'a self, photo: &'a Photo, cx: &'a Context<'_>) -> format::Source<'a> {
        format::Source {
            of: format::Of::photo(photo),
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
                let [label, tag, _] = messages::fact_words(ID, fact.id).map(|id| words.text(&id));
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
                let worded = self.worded(fact.id, &subject, cx);
                row.enabled = worded.enabled;
                row.value = worded.value;
                row.tip = [Some(worded.tip), Some(tag)]
                    .into_iter()
                    .flatten()
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                if fact.end == Some(FactEndDef::OpenFolder) {
                    row.end = match &subject {
                        Subject::One(photo) => {
                            photo.path.parent().map(|folder| open_folder(folder, cx))
                        }
                        Subject::Several { .. } => Some(self.shared_folder(cx)),
                        Subject::None => None,
                    };
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

/// The ratings of the photos the tool describes, for Rating's and Colour labels' rows
/// (decision 110): the open photo's, as the caption's, or several photos' from their listing,
/// each rating with how many photos have it; enabled while the rating's actions can run.
#[derive(Clone, Debug)]
struct Rated {
    ratings: Vec<(Rating, usize)>,
    /// How many photos are described.
    photos: usize,
    enabled: bool,
    /// The rating a click on its stars toggles against, as the caption's: the open photo's.
    open: Option<Rating>,
}

/// A row of marks as [`Rated`] words it.
struct MarksRow {
    marks: Marks,
    words: Option<String>,
    tip: String,
}

impl Rated {
    fn of(cx: &Context<'_>) -> Rated {
        let mirror = cx.mirror;
        let open = mirror.rateable();
        let shown = mirror.rating.as_ref().map(|state| state.rating);
        let mut ratings: Vec<(Rating, usize)> = Vec::new();
        let mut photos = 0;
        let mut count = |rating: Rating| match ratings.iter_mut().find(|(kept, _)| *kept == rating)
        {
            Some((_, n)) => *n += 1,
            None => ratings.push((rating, 1)),
        };
        if mirror.selected_count() > 1 && mirror.selection_settled() {
            let lookup = Lookup::of(mirror);
            for path in &mirror.selection.photos {
                photos += 1;
                count(lookup.rating(path).unwrap_or_default());
            }
        } else if let Some(rating) = (open.or(shown))
            // While a selection command is untold, no photo, as the other rows.
            .filter(|_| active_photo(cx.mirror).is_some() && mirror.selection_settled())
        {
            photos = 1;
            count(rating);
        }
        ratings.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        Rated {
            enabled: open.is_some() && photos > 0,
            ratings,
            photos,
            open,
        }
    }

    /// Rating's or Colour labels' row as text: the words the row shows, else its stars as ★
    /// and ☆, or the labels every photo has by name and those some have marked so; none with
    /// no photo. With the person's `notes`, as screen readers hear it, each rating or label
    /// the words stand for ends in its note, as the tooltip says it: "Rejected · Out", or
    /// Different then each rating's line; Copy Information passes none.
    fn text(&self, widget: Widget, words: &Translator, notes: &RatingNotes) -> Option<String> {
        if self.photos == 0 {
            return None;
        }
        let row = self.row(widget, words, notes);
        if let Some(said) = row.words {
            let plain = self.row(widget, words, &RatingNotes::default()).tip;
            return Some(if row.tip == plain {
                said
            } else if plain.is_empty() {
                // One rating, Rejected, its note the tooltip's whole line.
                row.tip
            } else {
                let lines: Vec<&str> = row.tip.lines().collect();
                format!("{said}: {}", lines.join(", "))
            });
        }
        Some(match row.marks {
            Marks::Stars(stars) => (0..5u8)
                .map(|at| if at < stars { '★' } else { '☆' })
                .collect(),
            Marks::Labels { on, some } => {
                let name = |at: usize| {
                    let colour = words.text(&format!("colour-label-{}", LABELS[at]));
                    RatingNotes::noted(colour, &notes.labels[at], words)
                };
                let mut names: Vec<String> =
                    (0..5).filter(|at| on & (1 << at) != 0).map(name).collect();
                names.extend((0..5).filter(|at| some & (1 << at) != 0).map(|at| {
                    let mut args = Args::new();
                    args.set("label", name(at));
                    words.text_with("metadata-label-some", Some(&args))
                }));
                if names.is_empty() {
                    "—".to_owned()
                } else {
                    names.join(", ")
                }
            }
        })
    }

    /// Rating's or Colour labels' row: the stars of the rating every photo has, "Rejected",
    /// or "Different" with each rating and its count in the tooltip; the labels every photo
    /// has, filled, and those some have, ringed. Its tooltip says, from `notes`, the rating
    /// shown, each rating Different counts and each label shown, where the person noted it:
    /// "3 stars · Print candidates", a line each.
    fn row(&self, widget: Widget, words: &Translator, notes: &RatingNotes) -> MarksRow {
        let tip = match self.photos {
            0 => words.text("metadata-select-a-photo"),
            _ => String::new(),
        };
        if widget == Widget::MetadataLabels {
            let all = (self.ratings.iter()).fold(0x1f, |all, (rating, _)| all & rating.labels);
            let any = (self.ratings.iter()).fold(0, |any, (rating, _)| any | rating.labels);
            let noted: Vec<String> = (ColourLabel::ALL.into_iter().enumerate())
                .filter(|(at, label)| {
                    any & (1 << at) != 0 && !notes.label(*label).trim().is_empty()
                })
                .map(|(_, label)| {
                    let colour = words.text(&format!("colour-label-{}", label.id()));
                    RatingNotes::noted(colour, notes.label(label), words)
                })
                .collect();
            let tip = if self.photos > 0 && !noted.is_empty() {
                noted.join("\n")
            } else {
                tip
            };
            return MarksRow {
                marks: Marks::Labels {
                    on: if self.photos > 0 { all } else { 0 },
                    some: any & !all,
                },
                words: None,
                tip,
            };
        }
        let stars = |rating: &Rating| {
            if rating.rejected {
                RatingNotes::noted(words.text("metadata-rejected"), &notes.reject, words)
            } else {
                let mut args = Args::new();
                args.set("count", f64::from(rating.stars));
                let said = words.text_with("metadata-stars", Some(&args));
                RatingNotes::noted(said, notes.star(rating.stars), words)
            }
        };
        // The rating's note, where the person gave it one.
        let note = |rating: &Rating| {
            let note = if rating.rejected {
                notes.reject.as_str()
            } else {
                notes.star(rating.stars)
            };
            (!note.trim().is_empty()).then(|| stars(rating))
        };
        // As shown: a rejected photo reads Rejected whatever its stars.
        let shared =
            |a: &Rating, b: &Rating| a.rejected == b.rejected && (a.rejected || a.stars == b.stars);
        match self.ratings.as_slice() {
            [] => MarksRow {
                marks: Marks::Stars(0),
                words: None,
                tip,
            },
            [(first, _), rest @ ..] if rest.iter().all(|(other, _)| shared(first, other)) => {
                MarksRow {
                    marks: Marks::Stars(first.stars),
                    words: first.rejected.then(|| words.text("metadata-rejected")),
                    tip: note(first).unwrap_or(tip),
                }
            }
            several => {
                let mut counted: Vec<(String, usize)> = Vec::new();
                for (rating, n) in several {
                    let said = stars(rating);
                    match counted.iter_mut().find(|(kept, _)| *kept == said) {
                        Some((_, total)) => *total += n,
                        None => counted.push((said, *n)),
                    }
                }
                counted.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                let lines: Vec<String> = (counted.into_iter())
                    .map(|(said, n)| {
                        let mut args = Args::new();
                        args.set("value", said);
                        args.set("count", n as f64);
                        words.text_with("metadata-value-count", Some(&args))
                    })
                    .collect();
                MarksRow {
                    marks: Marks::Stars(0),
                    words: Some(words.text("metadata-different")),
                    tip: lines.join("\n"),
                }
            }
        }
    }

    /// The rating's action a click on the mark at `at` runs, as the keys and the caption
    /// run it: so many stars, the open photo's own star clearing it; a label.
    fn action(&self, widget: Widget, at: usize) -> ActionId {
        if widget == Widget::MetadataLabels {
            return ActionId::LABEL[at.min(4)];
        }
        let stars = (at + 1).min(5);
        let own = self.photos == 1
            && self
                .open
                .is_some_and(|open| usize::from(open.stars) == stars);
        ActionId::RATING[if own { 0 } else { stars }]
    }
}

/// The colour labels' ids in darktable's order, as their words name them.
const LABELS: [&str; 5] = ["red", "yellow", "green", "blue", "purple"];

/// The ratings the open folder's listing, or the photos alone, hold, by photo: built once,
/// then read for each selected photo.
enum Lookup<'a> {
    Folder {
        folder: Option<PathBuf>,
        names: HashMap<&'a std::ffi::OsStr, Rating>,
    },
    Alone(HashMap<&'a Path, Rating>),
}

impl<'a> Lookup<'a> {
    fn of(mirror: &'a Mirror) -> Lookup<'a> {
        let folder = &mirror.folder;
        let photos = folder.photos();
        match &folder.folder {
            crate::folder::Folder::Open(listing) => Lookup::Folder {
                folder: std::path::absolute(&listing.path).ok(),
                names: (photos.iter())
                    .map(|photo| (std::ffi::OsStr::new(photo.name.as_str()), photo.rating))
                    .collect(),
            },
            _ => Lookup::Alone(
                (folder.alone_paths().iter().zip(photos))
                    .map(|(path, photo)| (path.as_path(), photo.rating))
                    .collect(),
            ),
        }
    }

    /// The rating of the photo at `path`, where the listing holds it.
    fn rating(&self, path: &Path) -> Option<Rating> {
        match self {
            Lookup::Folder { folder, names } => {
                let parent = std::path::absolute(path.parent()?).ok();
                (parent.is_some() && parent == *folder)
                    .then(|| names.get(path.file_name()?).copied())
                    .flatten()
            }
            Lookup::Alone(paths) => paths.get(path).copied(),
        }
    }
}

/// Where the photos described were taken, as Copy Coordinates copies it.
enum Located {
    /// One place: its coordinates.
    Here(String),
    /// None recorded; whether there is a photo described at all.
    Nowhere(bool),
    /// Several places.
    Apart,
}

/// A row's words: its value, `None` for "—", the first line of its tooltip, and whether it
/// is enabled.
struct Worded {
    value: Option<String>,
    tip: String,
    enabled: bool,
}

impl Metadata {
    /// The row `id` of `subject`: one photo's fact, several photos' summary, or none.
    fn worded(&self, id: &str, subject: &Subject<'_>, cx: &Context<'_>) -> Worded {
        let words = cx.words;
        let none = words.text(&messages::fact_words(ID, id)[2]);
        match subject {
            Subject::None => Worded {
                value: None,
                tip: words.text("metadata-select-a-photo"),
                enabled: false,
            },
            Subject::Several { count, names } if id == "file_name" => {
                let mut tip = names.join("\n");
                if *count > names.len() {
                    let more = (*count - names.len()) as f64;
                    let mut args = Args::new();
                    args.set("count", more);
                    tip = format!(
                        "{tip}\n{}",
                        words.text_with("metadata-and-more", Some(&args))
                    );
                }
                let mut args = Args::new();
                args.set("count", *count as f64);
                Worded {
                    value: Some(words.text_with("photo-count", Some(&args))),
                    tip,
                    enabled: true,
                }
            }
            Subject::Several { .. } => {
                let active = active_photo(cx.mirror).map(format::Of::photo);
                let several = several::Words {
                    words,
                    home: self.home.as_deref(),
                    active,
                };
                let held = self.held(cx.mirror);
                let row = several::row(id, held, &none, &several);
                let row = several::waiting(row, held.is_some(), &self.asking);
                Worded {
                    value: row.value,
                    tip: row.tip,
                    enabled: true,
                }
            }
            Subject::One(photo) => {
                let (value, tip) = one_fact(id, photo, &self.source(photo, cx));
                Worded {
                    value,
                    tip,
                    enabled: true,
                }
            }
        }
    }

    /// Open This Folder over several photos: on the folder they share, disabled where they
    /// are in several, and with no word while their summary has not come.
    fn shared_folder(&self, cx: &Context<'_>) -> (bool, String, Option<library::Action>) {
        let Some(summary) = self.held(cx.mirror) else {
            return (false, String::new(), None);
        };
        match summary.get("folder") {
            Some(FactSummary::Same(FactValue {
                key: Key::Text(folder),
                ..
            })) => open_folder(Path::new(folder), cx),
            _ => (false, cx.words.text("metadata-several-folders"), None),
        }
    }
}

/// One photo's fact `id` as `source` words it: its value, `None` for "—", and its tooltip
/// before its tag, the value or why it has none (the camera recorded none, or Apertrust
/// does not read it from the file's format), then what it adds.
pub(crate) fn one_fact(
    id: &str,
    photo: &Photo,
    source: &format::Source<'_>,
) -> (Option<String>, String) {
    let words = source.words;
    let shown = format::fact(id, source);
    let why = if photo.exif.unread.contains(&exif_field(id)) {
        let mut args = Args::new();
        args.set("format", photo.format.as_str());
        words.text_with("metadata-fact-unread", Some(&args))
    } else {
        words.text(&messages::fact_words(ID, id)[2])
    };
    let first = shown.value.clone().unwrap_or(why);
    let tip = [Some(first), shown.note].into_iter().flatten();
    (shown.value, tip.collect::<Vec<_>>().join("\n"))
}

/// The active photo, where one is open, whatever else is selected.
pub(crate) fn active_photo(mirror: &Mirror) -> Option<&Photo> {
    match &mirror.photo {
        crate::mirror::PhotoState::Open(photo) => Some(photo),
        _ => None,
    }
}

/// The one photo the tool describes and ⋯ copies: the active photo, but none while several
/// are selected, whose facts the tool does not read yet.
pub fn described(mirror: &Mirror) -> Option<&Photo> {
    match &mirror.photo {
        crate::mirror::PhotoState::Open(photo)
            if mirror.selected_count() <= 1 && mirror.selection_settled() =>
        {
            Some(photo)
        }
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
            // The session's names, once its selection is the one shown.
            let settled = cx.mirror.selection_settled();
            let names = (cx.mirror.selection.photos.iter())
                .filter(|_| settled)
                .take(5)
                .filter_map(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .collect();
            return Subject::Several { count, names };
        }
        // The selection on its way may leave the active photo out: nothing until it is told.
        match active_photo(cx.mirror).filter(|_| cx.mirror.selection_settled()) {
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

/// Open This Folder on `folder`: whether it can run, its tooltip, and what it does. In
/// folder mode it chooses the folder in Collections' tree, for photos opened alone it opens
/// it, as Collections' own does; it rests where the folder is the one open.
fn open_folder(folder: &Path, cx: &Context<'_>) -> (bool, String, Option<library::Action>) {
    let words = cx.words;
    let folder = folder.to_owned();
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

    /// Rating and Colour labels in Basic, enabled while the rating's actions can run, a click
    /// on a mark running its action; and Show on Map after GPS, drawn disabled until the map
    /// is built.
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
                let label = words.text(&label_id(id));
                let rated = self.rated(ui.ctx().cumulative_pass_nr(), cx);
                let notes = parts::rating_notes(ui.ctx());
                let row = rated.row(widget, words, &notes);
                let said = rated.text(widget, words, &notes);
                let marks = FactMarks::new(&label, &row.tip, row.marks, column)
                    .enabled(rated.enabled)
                    .words(row.words.as_deref())
                    .said(said.as_deref());
                if let Some(at) = marks.show(ui) {
                    self.ran.push(rated.action(widget, at));
                }
            }
            // At the row's end, where a fact's end item sits.
            Widget::MetadataMap => {
                let tip = words.text("metadata-map-later");
                let t = theme(ui.ctx());
                ui.add_space(t.spacing.xs);
                // One button high: a layout of its own would take every point left below.
                let line = egui::vec2(ui.available_width(), t.sizes.control);
                let end = egui::Layout::right_to_left(egui::Align::Center);
                ui.allocate_ui_with_layout(line, end, |ui| {
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
