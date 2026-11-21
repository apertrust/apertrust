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

//! Settings › Storage's hand-built part (decision 76; `docs/cache.md`, Settings ›
//! Storage), beside the cache's rows the schema gives:
//!
//! - **Folders,** above them: the user data folder and the cache folder, each with how it
//!   was chosen, read-only, since a folder is chosen before the app starts, and Show in
//!   Finder (Explorer, Files);
//! - **In the cache,** below them: each limited kind's entries and size with its Clear,
//!   the libraries' caches as one read-only line until their Remove comes with the
//!   library's step 27, and the limited kinds' total against the limit, or unlimited.
//!
//! The page asks for the sizes each time it is shown ([`Asked::Sizes`]); the app sends
//! `CacheCommand::Sizes`, and again after it sends a limit the page's rows changed, so the
//! total and the limit agree. A Clear's button is disabled from its press until its job
//! has ended ([`Mirror`]); its row is the jobs window's, as every job's. A refusal is
//! worded under the lines.

use std::path::PathBuf;

use apertrust_base::cache::{self, CacheEvent, Kind, LIBRARIES, Limit, PIPELINES, THUMBNAILS};
use apertrust_base::folders;
use apertrust_engine::jobs::{Argument, JobEvent, JobId, Message};
use apertrust_engine::session::{CacheCommand, Command, Event, SessionError, size_in_unit};
use apertrust_ui::components::FileManager;
use apertrust_ui::components::settings::{Button, Note, Readout, SettingRow};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Theme, Translator, translator};
use eframe::egui::{FontId, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2};

/// The limited kinds the page lists, each with its Clear, in the cache's order.
const CLEARED: [Kind; 2] = [THUMBNAILS, PIPELINES];

/// The kind of the session's job that clears a cache kind.
const CLEAR_JOB: &str = "cache-clear";

/// How a folder was chosen, as the page words it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// By a flag on the command line, the one given.
    Flag(&'static str),
    /// By an environment variable, the one named.
    Variable(&'static str),
    /// Inside the user data folder, which was given.
    UserData,
    /// The system's own.
    Platform,
}

/// A folder the app started with, and how it was chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    pub path: PathBuf,
    pub chosen: Chosen,
}

/// The folders the app started with, none where the system gave it no user data folder,
/// the person's home, which the page shows as `~`, and the platform's file manager, which
/// Show names.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Folders {
    pub user_data: Option<Folder>,
    pub cache: Option<Folder>,
    pub home: Option<PathBuf>,
    pub file_manager: FileManager,
}

impl Folders {
    /// The folders `folders` chose as the app started.
    pub fn of(folders: Option<&folders::Folders>, home: Option<PathBuf>) -> Folders {
        let Some(folders) = folders else {
            return Folders {
                home,
                file_manager: crate::jobs::FILE_MANAGER,
                ..Folders::default()
            };
        };
        Folders {
            user_data: Some(Folder {
                path: folders.user_data().to_owned(),
                chosen: user_data_chosen(folders.chosen()),
            }),
            cache: Some(Folder {
                path: folders.cache().to_owned(),
                chosen: cache_chosen(folders.cache_chosen()),
            }),
            home,
            file_manager: crate::jobs::FILE_MANAGER,
        }
    }
}

/// How the user data folder was chosen, as the page words it.
fn user_data_chosen(chosen: folders::Chosen) -> Chosen {
    match chosen {
        folders::Chosen::Flag(flag) => Chosen::Flag(flag),
        folders::Chosen::Variable => Chosen::Variable(folders::VARIABLE),
        folders::Chosen::Platform => Chosen::Platform,
    }
}

/// How the cache folder was chosen, as the page words it.
fn cache_chosen(chosen: folders::CacheChosen) -> Chosen {
    match chosen {
        folders::CacheChosen::Flag(flag) => Chosen::Flag(flag),
        folders::CacheChosen::Variable => Chosen::Variable(folders::CACHE_VARIABLE),
        folders::CacheChosen::UserData => Chosen::UserData,
        folders::CacheChosen::Platform => Chosen::Platform,
    }
}

/// Does what the page asked with `send`, the session's, which numbers each command: the
/// sizes, or a Clear, whose number `mirror` keeps so its button stays disabled until its job
/// ends; or a folder shown in the file manager.
pub fn act(asked: Asked, send: &mut dyn FnMut(Command) -> u64, mirror: &mut Mirror) {
    match asked {
        Asked::Sizes => {
            send(Command::Cache(CacheCommand::Sizes));
        }
        Asked::Clear(kind) => {
            let number = send(Command::Cache(CacheCommand::Clear { kind }));
            mirror.sent(kind, number);
        }
        Asked::Reveal(folder) => crate::jobs::reveal(&crate::jobs::Reveal::Folder(folder)),
    }
}

/// What the page shows, which the app hands Settings as it changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub folders: Folders,
    /// The session's latest sizes, none before its first answer.
    pub cache: Option<CacheEvent>,
    /// The kinds a Clear is clearing, from its press to its job's end.
    pub clearing: Vec<Kind>,
    /// The words of the last Clear the session refused, by message id.
    pub refused: Option<&'static str>,
}

/// What the page asks of the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    /// The cache's sizes: `CacheCommand::Sizes`.
    Sizes,
    /// A limited kind cleared: `CacheCommand::Clear`.
    Clear(Kind),
    /// A folder shown in the platform's file manager.
    Reveal(PathBuf),
}

/// A Clear, from the app's command, or its job's `Added`, to its job's end.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Clearing {
    kind: Kind,
    /// The command the app sent, until its job is added; none for a job the app did not
    /// start from the page, as a Try again.
    command: Option<u64>,
    job: Option<JobId>,
}

/// The mirror's part of the cache: the latest sizes fact, and each Clear from its press to
/// its job's end.
#[derive(Debug, Default)]
pub struct Mirror {
    latest: Option<CacheEvent>,
    clears: Vec<Clearing>,
    refused: Option<&'static str>,
}

impl Mirror {
    /// The Clear of `kind` the app sent as the command numbered `command`: its button is
    /// disabled until the job it starts has ended, or the session refuses it.
    pub fn sent(&mut self, kind: Kind, command: u64) {
        self.refused = None;
        self.clears.push(Clearing {
            kind,
            command: Some(command),
            job: None,
        });
    }

    /// Takes in `event`: the sizes, a Clear's job added or ended, or a Clear refused. A
    /// refusal's words go with the next sizes or the end of a Clear.
    pub fn apply(&mut self, event: &Event) {
        match event {
            Event::Cache(cache) => {
                self.latest = Some(*cache);
                self.refused = None;
            }
            Event::Job {
                event: JobEvent::Added {
                    job, kind, title, ..
                },
                command,
            } => self.added(*job, kind, title, *command),
            Event::Job {
                event: JobEvent::Ended { job, .. },
                ..
            } => {
                let before = self.clears.len();
                self.clears.retain(|clear| clear.job != Some(*job));
                if self.clears.len() < before {
                    self.refused = None;
                }
            }
            Event::Failed(SessionError::Refused { command, error }) => {
                let before = self.clears.len();
                self.clears.retain(|clear| clear.command != Some(*command));
                if self.clears.len() < before {
                    self.refused = Some(match **error {
                        SessionError::CacheClearing { .. } => "settings-storage-refused-clearing",
                        SessionError::NoCache => "settings-storage-refused-no-cache",
                        _ => "settings-storage-refused",
                    });
                }
            }
            // The engine thread has ended: no Clear will end.
            Event::Failed(SessionError::Stopped | SessionError::Panicked(_)) => {
                self.clears.clear();
            }
            _ => {}
        }
    }

    /// A job added: the Clear the app sent as `command`, or any other job that clears a
    /// cache kind, its title naming it, as the jobs window's Try again or another client's,
    /// so its kind's Clear stays disabled until it ends.
    fn added(&mut self, job: JobId, kind: &str, title: &Message, command: Option<u64>) {
        let sent = self
            .clears
            .iter_mut()
            .find(|clear| command.is_some() && clear.command == command);
        if let Some(clear) = sent {
            clear.job = Some(job);
            return;
        }
        if kind != CLEAR_JOB {
            return;
        }
        let cleared = match title.argument("kind") {
            Some(Argument::Text(id)) => cache::kind(id),
            _ => None,
        };
        if let Some(cleared) = cleared {
            self.clears.push(Clearing {
                kind: cleared,
                command: None,
                job: Some(job),
            });
        }
    }

    /// The page's view over `folders`.
    pub fn view(&self, folders: &Folders) -> View {
        let mut clearing: Vec<Kind> = Vec::new();
        for clear in &self.clears {
            if !clearing.contains(&clear.kind) {
                clearing.push(clear.kind);
            }
        }
        View {
            folders: folders.clone(),
            cache: self.latest,
            clearing,
            refused: self.refused,
        }
    }
}

/// The Folders group's lines: the user data folder and the cache folder. Returns what was
/// asked.
pub fn folders(ui: &mut Ui, t: &Theme, view: &View) -> Option<Asked> {
    let words = translator(ui.ctx());
    let mut asked = None;
    let lines = [
        ("settings-storage-user-data", &view.folders.user_data),
        ("settings-storage-cache", &view.folders.cache),
    ];
    for (label, folder) in lines {
        let label = words.text(label);
        let shown = folder.as_ref().map_or_else(
            || words.text("settings-storage-none"),
            |folder| crate::export::shown(&folder.path, view.folders.home.as_deref()),
        );
        let note = folder.as_ref().map(|folder| chosen(&words, folder.chosen));
        let show = words.text(show_words(view.folders.file_manager));
        let tip = words.text("settings-storage-show-tip");
        let button = folder.as_ref().map(|_| Press {
            text: &show,
            name: {
                let mut args = Args::new();
                args.set("show", show.as_str());
                args.set("folder", label.as_str());
                words.text_with("settings-storage-show-name", Some(&args))
            },
            tip: &tip,
            enabled: true,
        });
        let line = Line {
            label: &label,
            value: &shown,
            cut: Cut::Middle,
            note: note.as_deref(),
            warning: false,
            button,
        };
        if line.draw(ui, t)
            && let Some(folder) = folder
        {
            asked = Some(Asked::Reveal(folder.path.clone()));
        }
    }
    asked
}

/// The In the cache group's lines: each limited kind with its Clear, disabled while the
/// view says it is clearing or since it was `pressed`, the libraries' caches, the total
/// against the limit; with no cache, a line that says so, and no Clear. Returns what was
/// asked.
pub fn contents(ui: &mut Ui, t: &Theme, view: &View, pressed: &[Kind]) -> Option<Asked> {
    let words = translator(ui.ctx());
    if view.cache == Some(CacheEvent::NoCache) {
        said(ui, t, &words.text("settings-storage-no-cache"));
        return None;
    }
    let sizes = match view.cache {
        Some(CacheEvent::Sizes {
            sizes,
            limited,
            limit,
        }) => Some((sizes, limited, limit)),
        _ => None,
    };
    let counting = words.text("settings-storage-counting");
    let mut asked = None;
    let clear = words.text("settings-storage-clear");
    let tip = words.text("settings-storage-clear-tip");
    for kind in CLEARED {
        let label = words.text(&format!("settings-storage-kind-{}", kind.id()));
        let value = sizes.map_or_else(
            || counting.clone(),
            |(sizes, ..)| usage(&words, kind, sizes.of(kind).entries, sizes.of(kind).bytes),
        );
        let busy = view.clearing.contains(&kind) || pressed.contains(&kind);
        let enabled = sizes.is_some() && !busy;
        let line = Line {
            label: &label,
            value: &value,
            cut: Cut::End,
            note: None,
            warning: false,
            button: Some(Press {
                text: &clear,
                name: words.text(&format!("settings-storage-clear-{}", kind.id())),
                tip: &tip,
                enabled,
            }),
        };
        if line.draw(ui, t) {
            asked = Some(Asked::Clear(kind));
        }
    }
    let label = words.text("settings-storage-kind-libraries");
    let value = sizes.map_or_else(
        || counting.clone(),
        |(sizes, ..)| {
            let usage_ = sizes.of(LIBRARIES);
            usage(&words, LIBRARIES, usage_.entries, usage_.bytes)
        },
    );
    Line {
        label: &label,
        value: &value,
        cut: Cut::End,
        note: None,
        warning: false,
        button: None,
    }
    .draw(ui, t);
    let label = words.text("settings-storage-total");
    let value = sizes.map_or_else(
        || counting.clone(),
        |(_, limited, limit)| total(&words, limited.bytes, limit),
    );
    Line {
        label: &label,
        value: &value,
        cut: Cut::End,
        note: None,
        warning: false,
        button: None,
    }
    .draw(ui, t);
    if let Some(refused) = view.refused {
        said(ui, t, &words.text(refused));
    }
    asked
}

/// `file_manager`'s words for Show, as the jobs window's.
fn show_words(file_manager: FileManager) -> &'static str {
    match file_manager {
        FileManager::Finder => "activity-show-finder",
        FileManager::Explorer => "activity-show-explorer",
        FileManager::Files => "activity-show-files",
    }
}

/// How a folder was chosen, in words.
fn chosen(words: &Translator, chosen: Chosen) -> String {
    let mut args = Args::new();
    let id = match chosen {
        Chosen::Flag(flag) => {
            args.set("flag", flag);
            "settings-storage-chosen-flag"
        }
        Chosen::Variable(variable) => {
            args.set("variable", variable);
            "settings-storage-chosen-variable"
        }
        Chosen::UserData => "settings-storage-chosen-user-data",
        Chosen::Platform => "settings-storage-chosen-platform",
    };
    words.text_with(id, Some(&args))
}

/// Arguments of `bytes` as the Clear's messages take them: `$size`, with one decimal below
/// 10, in `$unit`.
fn sized(args: &mut Args, bytes: u64) {
    let (size, at_least, unit) = size_in_unit(bytes);
    args.set("size", apertrust_ui::i18n::decimals(size, at_least));
    args.set("unit", unit);
}

/// `kind`'s entries and their size, in the language's numbers.
fn usage(words: &Translator, kind: Kind, entries: u64, bytes: u64) -> String {
    let mut args = Args::new();
    args.set("kind", kind.id());
    args.set("entries", entries as f64);
    sized(&mut args, bytes);
    words.text_with("settings-storage-usage", Some(&args))
}

/// The limited kinds' `bytes` against `limit`, or unlimited.
fn total(words: &Translator, bytes: u64, limit: Limit) -> String {
    let mut args = Args::new();
    sized(&mut args, bytes);
    match limit {
        Limit::Unlimited => words.text_with("settings-storage-total-unlimited", Some(&args)),
        Limit::Bytes(limit) => {
            // In the setting's own steps of 0.5 GB, so a float's last digits never show, and
            // with its decimal, as its row writes it: "4.0 GB".
            let gigabytes = (limit as f64 / 1e8).round() / 10.0;
            args.set("limit", apertrust_ui::i18n::decimals(gigabytes, 1));
            words.text_with("settings-storage-total-limit", Some(&args))
        }
    }
}

/// Words on a line of their own across the content, wrapped, in the secondary text, where
/// the rows' labels start.
fn said(ui: &mut Ui, t: &Theme, text: &str) {
    let content = ui.available_width();
    let inset = t.spacing.s;
    let galley = ui.painter().layout(
        text.to_owned(),
        FontId::proportional(t.text.body),
        t.colors.text_secondary,
        content - 2.0 * inset,
    );
    let top = ui.cursor().min;
    let height = galley.size().y.max(t.sizes.setting_row);
    let rect = Rect::from_min_size(top, vec2(content, height));
    let at = pos2(top.x + inset, top.y + (height - galley.size().y) / 2.0);
    ui.painter().galley(at, galley, t.colors.text_secondary);
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
}

/// Where a value too long for its room is cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cut {
    /// In its middle, as a path, whose start and end both say where it is.
    Middle,
    /// At its end.
    End,
}

/// A line's button: its words, its full name for screen readers, its tooltip.
pub(crate) struct Press<'a> {
    text: &'a str,
    name: String,
    tip: &'a str,
    enabled: bool,
}

/// One of the page's lines, a settings row: its label at the start; in its column a
/// value it shows, a path cut in its middle, with its button after it, or the button with
/// the value before it as its fact; a note under it. Processing's In use line is one too
/// (`hardware`).
pub(crate) struct Line<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub cut: Cut,
    pub note: Option<&'a str>,
    /// Whether the note is a warning, in the warning colour and the value's description.
    pub warning: bool,
    pub button: Option<Press<'a>>,
}

impl Line<'_> {
    /// Draws it; whether its button was pressed.
    pub(crate) fn draw(self, ui: &mut Ui, _t: &Theme) -> bool {
        let mut row = SettingRow::new(("storage line", self.label), self.label).read_only(true);
        if let Some(note) = self.note {
            row = row.note(Note {
                text: note,
                warning: self.warning,
                live: false,
            });
        }
        let description = self.note.filter(|_| self.warning);
        row.show(ui, |ui| {
            fn button<'a>(press: &'a Press<'_>) -> Button<'a> {
                Button::new(press.text)
                    .name(&press.name)
                    .tip(press.tip)
                    .enabled(press.enabled)
            }
            match (&self.button, self.cut) {
                // A kind's Clear, its size before it.
                (Some(press), Cut::End) => button(press).fact(self.value).show(ui).clicked(),
                (press, cut) => {
                    let mut readout = Readout::new(self.value).path(cut == Cut::Middle);
                    if let Some(press) = press {
                        readout = readout.button(button(press));
                    }
                    if let Some(said) = description {
                        readout = readout.description(said);
                    }
                    readout.show(ui)
                }
            }
        })
        .inner
    }
}

#[cfg(test)]
pub(crate) mod tests;
