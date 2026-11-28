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

//! The Import tool (decisions 85, 86 and 93; `docs/gui.md`, The Import tool): the Import
//! sheet's one tool, drawn in its column by every tool's header and page drawer from its
//! declaration ([`apertrust_tools::utility::import::IMPORT`]).
//!
//! - **Its settings** are the sheet's ([`Tool::settings`]), each change of Folder,
//!   Subfolder or Import name written at once as an `import.*` setting, with no history
//!   step: the sheet hands each to the shell, which the settings store keeps
//!   ([`Tool::take_written`]). A key `--conf` gives for the run is drawn disabled at its
//!   value, saying so.
//! - **Its groups** open and close as a tool's do, kept in `edit.groups` under `import`
//!   ([`Tool::take_toggled`]); a closed group with a problem opens once as the problem
//!   appears, its row scrolled into view.
//! - **Its rows' tooltips** come from the latest preview: Folder's the sample path for the
//!   sheet's active photo, the folder new or there with its count, that nothing is
//!   replaced and the clashes; Subfolder's the date it uses. Each is the row's accessible
//!   description too. A problem is the row's warning button, its menu the fix.
//! - **Its ↺** puts back darktable's defaults, Folder the default folder, leaving Source.
//! - **20a1:** Import name disabled where neither Folder nor Subfolder uses the import
//!   name; Folder's ⌄ and the presets button drawn disabled; Keep original file name on
//!   and disabled, so File name is disabled; Metadata and Filters disabled and closed;
//!   Open the destination at When it ends and Verify the copies off, both disabled.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use apertrust_engine::session::library::{
    Fix, ImportFate, ImportPreview, ImportProblem, ImportSettings, Problem, Row as ProblemRow,
};
use apertrust_engine::session::{ControlValue, LibraryEvent};
use apertrust_export::variables::{self, PatternError};
use apertrust_library::destination::{self, Patterns, Shot};
use apertrust_library::settings::registry::{IMPORT_JOB_CODE, IMPORT_SUBFOLDER};
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::utility::import::IMPORT;
use apertrust_tools::{Row, SettingRow, Widget};
use apertrust_ui::components::{
    Fix as RowFix, PathChoice, PathRow, Problem as RowProblem, TextFieldRow, Variable,
};
use apertrust_ui::i18n::{Args, Translator};
use eframe::egui;

use super::source::Places;
use super::{Source, common, problem_words};
use crate::utility::{Context, UtilityTool};

/// What the tool asks the sheet for, beyond its settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// The system's folder chooser, for Folder: its folder button, or a problem's fix.
    ChooseFolder,
    /// Return in Import name: the sheet presses Import where it can.
    Returned,
    /// Source chose where to import from: the sheet lists it.
    Source(Source),
}

/// Subfolder's fixed choices in its ⌄, each by its id, its words and its pattern.
pub const SUBFOLDERS: [(&str, &str, &str); 4] = [
    (
        "each-import",
        "import-subfolder-each-import",
        "$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)",
    ),
    (
        "each-day",
        "import-subfolder-each-day",
        "$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)",
    ),
    ("named", "import-subfolder-named", "$(IMPORT_NAME)"),
    ("none", "import-subfolder-none", ""),
];

/// File name's pattern, darktable's default, shown while the original names are kept.
const FILE_NAME: &str = "$(YEAR)$(MONTH)$(DAY)_$(SEQUENCE).$(FILE_EXTENSION)";

/// The Import tool's state, kept while its sheet is open.
#[derive(Debug)]
pub struct Tool {
    /// The settings the sheet lists, checks, previews and imports with.
    pub settings: ImportSettings,
    /// The keys `--conf` gives for this run.
    given: BTreeSet<String>,
    /// Its collapsible groups the person opened or closed, by id.
    groups: BTreeMap<String, bool>,
    /// The import's day, which Subfolder's tooltip names.
    today: chrono::NaiveDate,
    /// The settings changed since the sheet last took them, by key in full.
    written: Vec<(&'static str, String)>,
    /// Its groups opened or closed since the sheet last took them.
    toggled: Vec<(&'static str, bool)>,
    /// The rows whose problems have opened their group, so each opens it once.
    opened_for: Vec<ProblemRow>,
    /// The row a problem has just opened its group for, scrolled into view as drawn.
    reveal: Option<ProblemRow>,
    /// Source: the places, the place and the folder chosen, and the tree.
    pub places: Places,
}

impl Tool {
    /// The tool over `settings`, its groups as the person left them, none given by
    /// `--conf`, for an import today.
    pub fn new(settings: ImportSettings) -> Self {
        Tool {
            places: Places::new(settings.places.clone(), "", ""),
            settings,
            given: BTreeSet::new(),
            groups: BTreeMap::new(),
            today: chrono::Local::now().date_naive(),
            written: Vec::new(),
            toggled: Vec::new(),
            opened_for: Vec::new(),
            reveal: None,
        }
    }

    /// Source on `place` and `folder` as last used, by their paths, empty for none.
    pub fn source(mut self, place: &str, folder: &str) -> Self {
        self.places = Places::new(self.settings.places.clone(), place, folder);
        self
    }

    /// Takes in one of the library's events, Source's: the volumes, and the subfolders it
    /// asked for. Returns the source chosen as the first volumes come, a card among them,
    /// kept as the place and folder last used.
    pub fn receive(&mut self, event: &LibraryEvent) -> Option<Source> {
        let source = self.places.receive(event)?;
        self.keep_source();
        Some(source)
    }

    /// Writes the place and the folder chosen, as `import.place` and `import.folder`.
    fn keep_source(&mut self) {
        for (key, value) in self.places.kept() {
            self.written.retain(|(written, _)| *written != key);
            self.written.push((key, value));
        }
    }

    /// Its groups opened or closed as `groups` keeps them, by id.
    pub fn groups(mut self, groups: BTreeMap<String, bool>) -> Self {
        self.groups = groups;
        self
    }

    /// The keys `--conf` gives for this run, each drawn disabled at its value.
    pub fn given(mut self, keys: impl IntoIterator<Item = String>) -> Self {
        self.given = keys.into_iter().collect();
        self
    }

    /// The import's day, which Subfolder's tooltip names.
    pub fn today(mut self, today: chrono::NaiveDate) -> Self {
        self.today = today;
        self
    }

    /// The settings changed since the last call, by key in full, for the shell to keep.
    pub fn take_written(&mut self) -> Vec<(&'static str, String)> {
        std::mem::take(&mut self.written)
    }

    /// Its groups opened or closed since the last call, for the shell to keep.
    pub fn take_toggled(&mut self) -> Vec<(&'static str, bool)> {
        std::mem::take(&mut self.toggled)
    }

    /// Whether the group `group` is open: as the person left it, else as it opens at first.
    pub fn is_open(&self, group: &str) -> bool {
        (self.groups.get(group).copied()).unwrap_or_else(|| IMPORT.opens(group))
    }

    /// Opens or closes `group`, telling the shell.
    fn set_group(&mut self, group: &str, open: bool) {
        let Some(id) = IMPORT.open.iter().copied().find(|id| *id == group) else {
            return;
        };
        self.groups.insert(id.to_owned(), open);
        self.toggled.push((id, open));
    }

    /// Whether `--conf` gives `key` for this run.
    fn is_given(&self, key: &str) -> bool {
        self.given.contains(key)
    }

    /// Writes the setting `key`, a text, as the person changed it, unless `--conf` gives it.
    pub fn write(&mut self, key: &'static str, value: String) {
        if self.is_given(key) {
            return;
        }
        let kept = match key {
            "import.base_folder" => &mut self.settings.base_folder,
            "import.subfolder" => &mut self.settings.subfolder,
            "import.job_code" => &mut self.settings.job_code,
            _ => return,
        };
        if *kept == value {
            return;
        }
        kept.clone_from(&value);
        self.written.retain(|(written, _)| *written != key);
        self.written.push((key, value));
    }

    /// The settings ↺ puts back, darktable's defaults, Folder the default folder; Source's
    /// place and folder, kept apart, stay.
    fn defaults(&self) -> [(&'static str, String); 3] {
        [
            ("import.base_folder", self.settings.default_folder.clone()),
            ("import.subfolder", IMPORT_SUBFOLDER.default().to_owned()),
            ("import.job_code", IMPORT_JOB_CODE.default().to_owned()),
        ]
    }

    /// Whether every setting ↺ puts back is at its default, ↺ then drawn at rest; one
    /// `--conf` gives counts as at its default, ↺ leaving it.
    pub fn at_defaults(&self) -> bool {
        (self.defaults().iter()).all(|(key, value)| self.is_given(key) || self.text(key) == value)
    }

    /// The text setting `key` as the tool holds it.
    fn text(&self, key: &str) -> &str {
        match key {
            "import.base_folder" => &self.settings.base_folder,
            "import.subfolder" => &self.settings.subfolder,
            "import.job_code" => &self.settings.job_code,
            _ => "",
        }
    }

    /// Whether a pattern in use names the import name: Folder, or Subfolder where it is not
    /// empty. File name's pattern is not in use while the original names are kept.
    pub fn uses_name(&self) -> bool {
        let uses =
            |pattern: &str| pattern.contains("$(IMPORT_NAME)") || pattern.contains("$(JOBCODE)");
        uses(&self.settings.base_folder) || uses(&self.settings.subfolder)
    }

    /// Takes in the latest preview: a closed group with a problem opens once, as the
    /// problem appears, its row then scrolled into view.
    pub fn problems(&mut self, preview: Option<&ImportPreview>) {
        let rows: Vec<ProblemRow> = preview
            .map(|preview| preview.problems.iter().map(|p| p.row).collect())
            .unwrap_or_default();
        // A problem gone may open its group again when it comes back.
        self.opened_for.retain(|row| rows.contains(row));
        for row in rows {
            if self.opened_for.contains(&row) {
                continue;
            }
            self.opened_for.push(row);
            if !self.is_open("destination") {
                self.set_group("destination", true);
                self.reveal = Some(row);
            }
        }
    }

    /// Folder with its variables filled in for an import now, where its pattern reads:
    /// where the system's folder chooser starts.
    pub fn folder_start(&self) -> Option<PathBuf> {
        let patterns = Patterns::parse(&self.settings).ok()?;
        let now = destination::local(std::time::SystemTime::now());
        Some(patterns.folder(now, &Shot::default()))
    }

    /// Folder chosen with the system's folder chooser, as its text.
    pub fn folder_chosen(&mut self, folder: &Path) {
        self.write("import.base_folder", folder.display().to_string());
    }
}

/// The tool as one frame draws it: its state, and what the sheet shows it, the latest
/// preview, the sheet's active photo by its path relative to the source, the source and
/// the person's home.
pub struct Drawn<'a> {
    pub tool: &'a mut Tool,
    pub preview: Option<&'a ImportPreview>,
    pub active: Option<&'a Path>,
    pub source: Option<&'a Source>,
}

impl Drawn<'_> {
    /// The person's home, which a path shows as `~`.
    fn home(&self) -> Option<&Path> {
        self.tool.settings.places.home.as_deref()
    }

    /// The problem of `row` in the latest preview.
    fn problem(&self, row: ProblemRow) -> Option<&ImportProblem> {
        (self.preview?.problems.iter()).find(|problem| problem.row == row)
    }

    /// `problem`'s words and its fix's.
    fn problem_text(
        &self,
        problem: &ImportProblem,
        words: &Translator,
    ) -> (String, Option<String>) {
        let mut text = problem_words(problem, words, self.home());
        let fix = problem.fix.as_ref().map(|fix| match fix {
            Fix::Use { variable } => {
                let mut args = Args::new();
                args.set("variable", variable.to_string());
                text = format!(
                    "{text} {}",
                    words.text_with("import-did-you-mean", Some(&args))
                );
                words.text_with("import-fix-use", Some(&args))
            }
            Fix::ChooseFolder => words.text("import-fix-choose-folder"),
        });
        (text, fix)
    }

    /// The fix of `row`'s problem chosen, by its id: a variable put in place of the name
    /// typed, or the folder chooser asked for.
    fn fixed(&mut self, row: ProblemRow, key: &'static str, chosen: &str, asked: &mut Vec<Asked>) {
        let Some(problem) = self.problem(row).cloned() else {
            return;
        };
        match (&problem.fix, &problem.problem, chosen) {
            (
                Some(Fix::Use { variable }),
                Problem::Pattern(PatternError::Unknown { at, .. }),
                "use",
            ) => {
                // The name typed, where the field still holds it there.
                let text = self.tool.text(key).to_owned();
                if text.get(at.clone()).is_some() {
                    let fixed = format!("{}{variable}{}", &text[..at.start], &text[at.end..]);
                    self.tool.write(key, fixed);
                }
            }
            (Some(Fix::ChooseFolder), _, "choose-folder") => asked.push(Asked::ChooseFolder),
            _ => {}
        }
    }

    /// Folder's tooltip: the sample path for the sheet's active photo, the folder new or
    /// there, that nothing is replaced, and the clashes with an edit of their own.
    fn folder_tip(&self, words: &Translator) -> String {
        let home = self.home();
        let mut lines = Vec::new();
        if let Some(preview) = self.preview {
            let active = (self.active)
                .and_then(|path| preview.photos.iter().find(|photo| photo.source == path));
            let to = active.and_then(|photo| match &photo.fate {
                ImportFate::Copy { to, .. } => Some(to),
                ImportFate::There { at } | ImportFate::Diverged { at, .. } => Some(at),
                ImportFate::Unreadable { .. } => None,
            });
            if let (Some(path), Some(to)) = (self.active, to) {
                let mut args = Args::new();
                args.set("photo", file_name(path));
                args.set("path", crate::export::shown(to, home));
                lines.push(words.text_with("import-folder-sample", Some(&args)));
            }
            let folders = &preview.folders;
            let mut args = Args::new();
            match folders.as_slice() {
                [] => {}
                [one] => {
                    let name = file_name(&one.folder);
                    let parent = one.folder.parent().map(file_name).unwrap_or_default();
                    match one.there {
                        None => {
                            args.set("parent", parent);
                            lines.push(words.text_with("import-folder-new", Some(&args)));
                        }
                        Some(there) => {
                            args.set("folder", name);
                            args.set("count", there as f64);
                            lines.push(words.text_with("import-folder-into", Some(&args)));
                        }
                    }
                }
                several => {
                    let parent = common(several.iter().map(|f| f.folder.as_path()));
                    let new = several.iter().filter(|f| f.there.is_none()).count();
                    args.set(
                        "parent",
                        parent.as_deref().map(file_name).unwrap_or_default(),
                    );
                    args.set("count", (if new > 0 { new } else { several.len() }) as f64);
                    let id = if new > 0 {
                        "import-folder-new-several"
                    } else {
                        "import-folder-there-several"
                    };
                    lines.push(words.text_with(id, Some(&args)));
                }
            }
        }
        lines.push(words.text("import-folder-never-replaces"));
        let diverged = self.preview.map_or(0, |preview| preview.counts().diverged);
        if diverged > 0 {
            let mut args = Args::new();
            args.set("count", diverged as f64);
            let source = self.source.map_or_else(String::new, |s| s.name.clone());
            args.set("source", source);
            lines.push(words.text_with("import-folder-diverged", Some(&args)));
        }
        lines.join("\n")
    }

    /// Subfolder's tooltip: which date it uses, today's, the day taken, or none.
    fn subfolder_tip(&self, words: &Translator) -> Option<String> {
        let uses = self.preview?.subfolder_uses;
        if uses.taken {
            Some(words.text("import-subfolder-by-day"))
        } else if uses.today {
            let mut args = Args::new();
            args.set("date", words.format_date(self.tool.today, false));
            Some(words.text_with("import-subfolder-today", Some(&args)))
        } else {
            None
        }
    }

    /// Why a row whose key `--conf` gives is disabled, else none.
    fn given(&self, key: &str, words: &Translator) -> Option<String> {
        self.tool
            .is_given(key)
            .then(|| crate::utility::overridden(words))
    }

    /// Import name's row.
    fn name_row(&mut self, ui: &mut egui::Ui, words: &Translator, asked: &mut Vec<Asked>) {
        let key = "import.job_code";
        let label = words.text("import-row-name");
        let unused = words.text("import-name-unused");
        let tip = words.text("import-name-tip");
        let given = self.given(key, words);
        let text = self.tool.settings.job_code.clone();
        let mut row = TextFieldRow::new(egui::Id::unique("import name"), &label, &text).tip(&tip);
        if let Some(why) = &given {
            row = row.disabled(why);
        } else if !self.tool.uses_name() {
            row = row.disabled(&unused);
        }
        let out = row.show(ui);
        if let Some(text) = out.text {
            self.tool.write(key, text);
        }
        if out.returned {
            asked.push(Asked::Returned);
        }
    }

    /// Folder's or Subfolder's row, `key`'s, with `row`'s problems.
    fn path_row(
        &mut self,
        ui: &mut egui::Ui,
        words: &Translator,
        row: ProblemRow,
        asked: &mut Vec<Asked>,
    ) {
        let folder = row == ProblemRow::Folder;
        let key = if folder {
            "import.base_folder"
        } else {
            "import.subfolder"
        };
        let listed: Vec<Variable> = (variables::list().iter())
            .map(|v| Variable {
                insert: v.insert(),
                description: v.description(),
            })
            .collect();
        let label = words.text(if folder {
            "import-row-folder"
        } else {
            "import-row-subfolder"
        });
        let problem = self.problem(row).map(|p| self.problem_text(p, words));
        let fix_id = self
            .problem(row)
            .and_then(|p| p.fix.as_ref())
            .map(|fix| match fix {
                Fix::Use { .. } => "use",
                Fix::ChooseFolder => "choose-folder",
            });
        let fixes: Vec<RowFix> = match (&problem, fix_id) {
            (Some((_, Some(text))), Some(id)) => vec![RowFix { id, text }],
            _ => Vec::new(),
        };
        let shown = problem
            .as_ref()
            .map(|(text, _)| RowProblem::new(text).stops(true).fixes(&fixes));
        let tip = if folder {
            Some(self.folder_tip(words))
        } else {
            self.subfolder_tip(words)
        };
        let given = self.given(key, words);
        let later = words.text("section-tab-later");
        let choices_name = words.text(if folder {
            "import-folder-choices"
        } else {
            "import-subfolder-choices"
        });
        let choose = words.text("import-choose-folder");
        let hint = words.text("import-subfolder-hint");
        let text = self.tool.text(key).to_owned();
        let names: Vec<String> = (SUBFOLDERS.iter())
            .map(|(_, id, _)| words.text(id))
            .collect();
        let choices: Vec<PathChoice> = (SUBFOLDERS.iter().zip(&names))
            .map(|((id, _, pattern), name)| PathChoice {
                id,
                name,
                pattern: (!pattern.is_empty()).then_some(*pattern),
                chosen: text == *pattern,
                disabled: None,
            })
            .collect();
        let id = egui::Id::unique(("import path", key));
        let mut path = PathRow::new(id, &label, &text, &listed).problem(shown);
        path = if folder {
            path.choices_later(&choices_name, &later).folder(&choose)
        } else {
            path.choices(&choices_name, &choices).hint(&hint)
        };
        if let Some(tip) = &tip {
            path = path.tip(tip);
        }
        if let Some(why) = &given {
            path = path.disabled(why);
        }
        let out = path.show(ui);
        if self.tool.reveal == Some(row) {
            self.tool.reveal = None;
            out.response.scroll_to_me(Some(egui::Align::Center));
        }
        if let Some(text) = out.text {
            self.tool.write(key, text);
        }
        if let Some(chosen) = out.fix {
            self.fixed(row, key, &chosen, asked);
        }
        if let Some(chosen) = out.choice
            && let Some((_, _, pattern)) = SUBFOLDERS.iter().find(|(id, ..)| *id == chosen)
        {
            self.tool.write(key, (*pattern).to_owned());
        }
        if out.folder {
            asked.push(Asked::ChooseFolder);
        }
    }

    /// File name's row, drawn disabled while the original names are kept.
    fn file_name_row(&self, ui: &mut egui::Ui, words: &Translator) {
        let label = words.text("import-file-name");
        let why = words.text("import-file-name-kept");
        PathRow::new(egui::Id::unique("import file name"), &label, FILE_NAME, &[])
            .disabled(&why)
            .show(ui);
    }
}

/// The last part of `path`, as text.
fn file_name(path: &Path) -> String {
    (path.file_name()).map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

impl UtilityTool for Drawn<'_> {
    type Action = Asked;

    fn definition(&self) -> &'static UtilityDefinition {
        &IMPORT
    }

    fn body(&mut self, widget: Widget, ui: &mut egui::Ui, cx: &mut Context<'_>) -> Vec<Asked> {
        let words = cx.words;
        let mut asked = Vec::new();
        let chose = match widget {
            Widget::ImportPlaces => self.tool.places.show_places(ui, words),
            Widget::ImportFolders => self.tool.places.show_folders(ui, words),
            _ => None,
        };
        if let Some(source) = chose {
            self.tool.keep_source();
            asked.push(Asked::Source(source));
        }
        cx.commands.extend(self.tool.places.take_commands());
        match widget {
            Widget::ImportName => self.name_row(ui, words, &mut asked),
            Widget::ImportFolder => self.path_row(ui, words, ProblemRow::Folder, &mut asked),
            Widget::ImportSubfolder => self.path_row(ui, words, ProblemRow::Subfolder, &mut asked),
            Widget::ImportFileName => self.file_name_row(ui, words),
            _ => {}
        }
        asked
    }

    /// Default settings, written at once.
    fn reset(&mut self, _cx: &mut Context<'_>) {
        for (key, value) in self.tool.defaults() {
            self.tool.write(key, value);
        }
    }

    /// The value 20a1 gives each row with no setting yet.
    fn setting(&self, row: &SettingRow) -> Option<ControlValue> {
        // A card's DCIM is listed whole; every other row is at its declared default.
        let card = self.source.is_some_and(|source| source.card);
        (row.id == "include_subfolders" && card).then_some(ControlValue::Bool(true))
    }

    fn tip(&self, row: &Row, words: &Translator) -> Option<String> {
        let Row::Setting(setting) = row else {
            return None;
        };
        Some(match setting.id {
            "keep_original_name" => words.text("import-keep-original-tip"),
            "open_destination" => words.text("import-open-destination-tip"),
            "include_subfolders" if self.source.is_some_and(|s| s.card) => {
                words.text("import-include-subfolders-card")
            }
            "include_subfolders" | "select_new" | "verify" => words.text("section-tab-later"),
            _ => return None,
        })
    }

    fn open(&self, group: &str) -> Option<bool> {
        Some(self.tool.is_open(group))
    }

    fn set_open(&mut self, group: &str, open: bool) {
        self.tool.set_group(group, open);
    }

    fn group_later(&self, group: &str, words: &Translator) -> Option<String> {
        matches!(group, "metadata" | "filters").then(|| words.text("section-tab-later"))
    }

    fn group_summary(&self, group: &str, words: &Translator) -> Option<String> {
        match group {
            "source" => self.tool.places.summary(words),
            "metadata" => Some(words.text("import-group-no-rating")),
            "filters" => Some(words.text("import-group-show-imported")),
            _ => None,
        }
    }

    fn presets_later(&self, words: &Translator) -> Option<String> {
        Some(words.text("import-presets-later"))
    }
}
