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

//! The Export tool (`docs/gui.md`, The Export tool; decision 56): darktable's export module
//! row for row, a utility section of the Photos tab's Tools panel, after Collections. It is
//! a function of `&mut Ui`, so the app draws it in the panel and the interaction tests draw
//! it in a harness.
//!
//! **Its page is a utility's declaration** (decision 93; `apertrust_tools::utility::export`),
//! drawn by the one page drawer: Storage options and Format options, plain titled groups
//! always shown; Global options, collapsible and open at first; and Metadata and
//! Multi-preset export, collapsible, closed and drawn disabled. Its settings are setting
//! rows naming the `export.*` settings by their keys in full, which this module binds to
//! [`Values`] ([`Values::get`], [`Values::set`]): On conflict, File format with JPEG's or
//! PNG's rows, Set size, Allow upscaling, High quality resampling and Profile; Store
//! masks, Intent and Style, with no setting yet, are drawn disabled. Its custom rows are
//! drawn here through its `UtilityTool`: Target storage, File on disk the one that can be
//! chosen; the path on one line that scrolls, the folder button at its end and no visible
//! label; the size line; the groups to come; and its end. What the session says, the
//! tooltips of the path and Set size and the problems under the path and the size, this
//! module supplies to the drawer, and so does it the options not built yet, disabled,
//! saying why. The rows are one-line rows stacked with no spacing between them, as a
//! tool's. The header is every tool's (decision 93), drawn from its declaration: ↺, which
//! applies Default settings, the presets button, drawn disabled until presets come, and ⋯.
//!
//! **Its end** (`end.rs`) is the summary, a collapsible group of its own, the warnings, the
//! Export button and the line under it with its job's progress.
//!
//! **The settings** are the `export.*` settings, one [`Values`], read from the app's store
//! as the app starts and offered back to it as they change (`Asked::values`), as darktable
//! keeps its conf. The engine reads no settings (decision 43): [`Values::settings`] turns
//! them into the plain [`ExportSettings`] the session's `Export` and `PreviewExport` take.
//!
//! **The feedback under the rows** comes from the session (`PreviewExport`,
//! `Event::ExportPreview`): the app asks for a preview whenever the settings or the photo
//! change while the tool is open and shown, and hands the answer back with
//! [`Export::receive`], which shows one only where it answers the latest request or a later
//! one ([`Export::sent`]), so a slow answer to an earlier request never replaces it. The
//! request names the photo open alone, the first of the answer's names and the one its size
//! is for, so it needs no `focus` photo until it covers the whole selection. The tool exports the
//! active photo, since nothing selects photos yet; exporting closes nothing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use apertrust_base::settings::{Choice as Stored, Store};
use apertrust_engine::export_preview::{Fate, PathProblem, SizeProblem};
use apertrust_engine::session::{Command, ControlValue, ExportPreview, ExportSettings, Places};
use apertrust_export::Format;
use apertrust_export::disk::{Blocked, Preflight};
use apertrust_export::jpeg::{self, Subsample};
use apertrust_export::png::{self, BitDepth};
use apertrust_export::settings::registry::{
    EXPORT_DISK_ON_CONFLICT, EXPORT_DISK_PATH, EXPORT_FORMAT, EXPORT_JPEG_QUALITY,
    EXPORT_JPEG_SUBSAMPLE, EXPORT_PNG_BIT_DEPTH, EXPORT_PNG_COMPRESSION, EXPORT_PROFILE,
    EXPORT_SIZE_DPI, EXPORT_SIZE_HEIGHT, EXPORT_SIZE_HIGH_QUALITY, EXPORT_SIZE_MODE,
    EXPORT_SIZE_SCALE, EXPORT_SIZE_UPSCALE, EXPORT_SIZE_WIDTH,
};
use apertrust_export::settings::{FileFormat, OnConflict, Profile, SizeMode};
use apertrust_export::size::{self, Request};
use apertrust_export::variables::{self, PatternError};
use apertrust_tools::utility::UtilityDefinition;
use apertrust_tools::{ControlKind, Row, SettingRow, Widget};
use apertrust_ui::components::{
    Button, Choice, ChoiceStyle, CollapsibleGroup, IconButton, PathField, TextField, ValueField,
    Variable, Wheel, field_inset,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Icon, Theme, Translator, theme, tr, translator};
use eframe::egui::{self, RichText, Ui, vec2};

use crate::tools::{Change, Head, Note, Pressed};
use crate::utility::{Context, UtilityTool};

mod end;
pub mod folder;
pub use end::{Jobs, Others, Photo, Shown, Why};

/// The tool's id, as the shell keeps its section's state and Export… reveals it.
pub const ID: &str = "export";
/// The Summary's id, as the shell keeps its state under the tool's. Global options' is the
/// declaration's, open at first (decision 90).
pub const SUMMARY: &str = "summary";
/// Whether the Summary is open until the person closes it: it starts open, as Global
/// options does.
pub const SUMMARY_OPEN: bool = true;

/// The export settings as the tool edits them: one field for each `export.*` setting the
/// tool's rows show.
#[derive(Clone, Debug, PartialEq)]
pub struct Values {
    /// File on disk's path, a pattern of darktable's variables without the extension.
    pub path: String,
    pub on_conflict: OnConflict,
    pub format: FileFormat,
    pub jpeg_quality: u8,
    pub jpeg_subsample: Subsample,
    pub png_bit_depth: BitDepth,
    pub png_compression: u8,
    pub size_mode: SizeMode,
    /// The largest width and height in pixels, 0 for no limit, whatever the mode shows
    /// them in, as darktable holds them.
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    /// By scale, darktable's text for it.
    pub scale: String,
    pub upscale: bool,
    pub high_quality: bool,
    pub profile: Profile,
}

impl Default for Values {
    /// The schema's defaults: darktable's.
    fn default() -> Self {
        Values {
            path: EXPORT_DISK_PATH.default().to_owned(),
            on_conflict: EXPORT_DISK_ON_CONFLICT.default(),
            format: EXPORT_FORMAT.default(),
            jpeg_quality: EXPORT_JPEG_QUALITY.default() as u8,
            jpeg_subsample: EXPORT_JPEG_SUBSAMPLE.default(),
            png_bit_depth: EXPORT_PNG_BIT_DEPTH.default(),
            png_compression: EXPORT_PNG_COMPRESSION.default() as u8,
            size_mode: EXPORT_SIZE_MODE.default(),
            width: EXPORT_SIZE_WIDTH.default() as u32,
            height: EXPORT_SIZE_HEIGHT.default() as u32,
            dpi: EXPORT_SIZE_DPI.default() as u32,
            scale: EXPORT_SIZE_SCALE.default().to_owned(),
            upscale: EXPORT_SIZE_UPSCALE.default(),
            high_quality: EXPORT_SIZE_HIGH_QUALITY.default(),
            profile: EXPORT_PROFILE.default(),
        }
    }
}

impl Values {
    /// The settings as `store` holds them.
    pub fn read(store: &Store) -> Values {
        Values {
            path: store.get(&EXPORT_DISK_PATH),
            on_conflict: store.get(&EXPORT_DISK_ON_CONFLICT),
            format: store.get(&EXPORT_FORMAT),
            jpeg_quality: store.get(&EXPORT_JPEG_QUALITY).round() as u8,
            jpeg_subsample: store.get(&EXPORT_JPEG_SUBSAMPLE),
            png_bit_depth: store.get(&EXPORT_PNG_BIT_DEPTH),
            png_compression: store.get(&EXPORT_PNG_COMPRESSION).round() as u8,
            size_mode: store.get(&EXPORT_SIZE_MODE),
            width: store.get(&EXPORT_SIZE_WIDTH).round() as u32,
            height: store.get(&EXPORT_SIZE_HEIGHT).round() as u32,
            dpi: store.get(&EXPORT_SIZE_DPI).round() as u32,
            scale: store.get(&EXPORT_SIZE_SCALE),
            upscale: store.get(&EXPORT_SIZE_UPSCALE),
            high_quality: store.get(&EXPORT_SIZE_HIGH_QUALITY),
            profile: store.get(&EXPORT_PROFILE),
        }
    }

    /// Sets in `store` what differs from the settings as it holds them, as the app's
    /// other settings are offered each frame.
    pub fn write(&self, store: &mut Store) {
        let was = Values::read(store);
        if self.path != was.path {
            store.set(&EXPORT_DISK_PATH, self.path.clone());
        }
        if self.on_conflict != was.on_conflict {
            store.set(&EXPORT_DISK_ON_CONFLICT, self.on_conflict);
        }
        if self.format != was.format {
            store.set(&EXPORT_FORMAT, self.format);
        }
        if self.jpeg_quality != was.jpeg_quality {
            store.set(&EXPORT_JPEG_QUALITY, f32::from(self.jpeg_quality));
        }
        if self.jpeg_subsample != was.jpeg_subsample {
            store.set(&EXPORT_JPEG_SUBSAMPLE, self.jpeg_subsample);
        }
        if self.png_bit_depth != was.png_bit_depth {
            store.set(&EXPORT_PNG_BIT_DEPTH, self.png_bit_depth);
        }
        if self.png_compression != was.png_compression {
            store.set(&EXPORT_PNG_COMPRESSION, f32::from(self.png_compression));
        }
        if self.size_mode != was.size_mode {
            store.set(&EXPORT_SIZE_MODE, self.size_mode);
        }
        if self.width != was.width {
            store.set(&EXPORT_SIZE_WIDTH, self.width as f32);
        }
        if self.height != was.height {
            store.set(&EXPORT_SIZE_HEIGHT, self.height as f32);
        }
        if self.dpi != was.dpi {
            store.set(&EXPORT_SIZE_DPI, self.dpi as f32);
        }
        if self.scale != was.scale {
            store.set(&EXPORT_SIZE_SCALE, self.scale.clone());
        }
        if self.upscale != was.upscale {
            store.set(&EXPORT_SIZE_UPSCALE, self.upscale);
        }
        if self.high_quality != was.high_quality {
            store.set(&EXPORT_SIZE_HIGH_QUALITY, self.high_quality);
        }
        if self.profile != was.profile {
            store.set(&EXPORT_PROFILE, self.profile);
        }
    }

    /// How many of the settings the size line holds are off their defaults, which Global
    /// options counts in its "N set" with its setting rows.
    pub fn set_in_size(&self) -> usize {
        let d = Values::default();
        [
            self.width != d.width,
            self.height != d.height,
            self.dpi != d.dpi,
            self.scale != d.scale,
        ]
        .into_iter()
        .filter(|set| *set)
        .count()
    }

    /// The setting `key`, named in full, as the settings keep it (decision 93): a choice by
    /// its id there; none for a key that is none of the tool's.
    pub fn get(&self, key: &str) -> Option<Kept> {
        Some(match key {
            "export.disk.path" => Kept::Text(self.path.clone()),
            "export.disk.on_conflict" => Kept::Choice(id_of(self.on_conflict)),
            "export.format" => Kept::Choice(id_of(self.format)),
            "export.jpeg.quality" => Kept::Number(f32::from(self.jpeg_quality)),
            "export.jpeg.subsample" => Kept::Choice(id_of(self.jpeg_subsample)),
            "export.png.bit_depth" => Kept::Choice(id_of(self.png_bit_depth)),
            "export.png.compression" => Kept::Number(f32::from(self.png_compression)),
            "export.size.mode" => Kept::Choice(id_of(self.size_mode)),
            "export.size.width" => Kept::Number(self.width as f32),
            "export.size.height" => Kept::Number(self.height as f32),
            "export.size.dpi" => Kept::Number(self.dpi as f32),
            "export.size.scale" => Kept::Text(self.scale.clone()),
            "export.size.upscale" => Kept::Bool(self.upscale),
            "export.size.high_quality" => Kept::Bool(self.high_quality),
            "export.profile" => Kept::Choice(id_of(self.profile)),
            _ => return None,
        })
    }

    /// Sets the setting `key`, named in full, to `value` as the settings keep it
    /// ([`Self::get`]); an option the settings do not hold, as a format not built, a value
    /// of another kind or a key that is none of the tool's changes nothing.
    pub fn set(&mut self, key: &str, value: &Kept) {
        fn choose<C: Stored>(value: &Kept, into: &mut C) {
            if let Kept::Choice(id) = value
                && let Some(chosen) = by_id(id)
            {
                *into = chosen;
            }
        }
        fn whole(value: &Kept, into: &mut u32) {
            if let Kept::Number(n) = value {
                *into = n.round() as u32;
            }
        }
        fn text(value: &Kept, into: &mut String) {
            if let Kept::Text(text) = value {
                into.clone_from(text);
            }
        }
        let number = match value {
            Kept::Number(n) => Some(n.round() as u8),
            _ => None,
        };
        let on = match value {
            Kept::Bool(on) => Some(*on),
            _ => None,
        };
        match key {
            "export.disk.path" => text(value, &mut self.path),
            "export.size.width" => whole(value, &mut self.width),
            "export.size.height" => whole(value, &mut self.height),
            "export.size.dpi" => whole(value, &mut self.dpi),
            "export.size.scale" => text(value, &mut self.scale),
            "export.disk.on_conflict" => choose(value, &mut self.on_conflict),
            "export.format" => choose(value, &mut self.format),
            "export.jpeg.quality" => self.jpeg_quality = number.unwrap_or(self.jpeg_quality),
            "export.jpeg.subsample" => choose(value, &mut self.jpeg_subsample),
            "export.png.bit_depth" => choose(value, &mut self.png_bit_depth),
            "export.png.compression" => {
                self.png_compression = number.unwrap_or(self.png_compression);
            }
            "export.size.mode" => choose(value, &mut self.size_mode),
            "export.size.upscale" => self.upscale = on.unwrap_or(self.upscale),
            "export.size.high_quality" => self.high_quality = on.unwrap_or(self.high_quality),
            "export.profile" => choose(value, &mut self.profile),
            _ => {}
        }
    }

    /// The ids the choice `key` takes, the options built; none for another key.
    pub fn options(key: &str) -> Vec<&'static str> {
        fn ids<C: Stored>() -> Vec<&'static str> {
            C::OPTIONS.iter().map(|(_, id)| *id).collect()
        }
        match key {
            "export.disk.on_conflict" => ids::<OnConflict>(),
            "export.format" => ids::<FileFormat>(),
            "export.jpeg.subsample" => ids::<Subsample>(),
            "export.png.bit_depth" => ids::<BitDepth>(),
            "export.size.mode" => ids::<SizeMode>(),
            "export.profile" => ids::<Profile>(),
            _ => Vec::new(),
        }
    }
}

/// An export setting's value as the settings keep it: a choice by its id, a number, a
/// boolean or a text.
#[derive(Clone, Debug, PartialEq)]
pub enum Kept {
    Choice(&'static str),
    Number(f32),
    Bool(bool),
    Text(String),
}

/// `value`'s id in the settings.
fn id_of<C: Stored + PartialEq>(value: C) -> &'static str {
    let found = C::OPTIONS.iter().find(|(option, _)| *option == value);
    found.map_or("", |(_, id)| id)
}

/// The option of `C` the settings call `id`, if one is.
fn by_id<C: Stored>(id: &str) -> Option<C> {
    C::OPTIONS
        .iter()
        .find(|(_, option)| *option == id)
        .map(|(option, _)| *option)
}

/// The settings' id of the option `option` of the setting row naming `key`: its own, but
/// for JPEG's subsampling and PNG's bit depth, whose ids in the settings, `444` and `8`,
/// are no row option's.
pub fn stored_id(key: &str, option: &'static str) -> &'static str {
    const RENAMED: &[(&str, &str, &str)] = &[
        ("export.jpeg.subsample", "yuv444", "444"),
        ("export.jpeg.subsample", "yuv440", "440"),
        ("export.jpeg.subsample", "yuv422", "422"),
        ("export.jpeg.subsample", "yuv420", "420"),
        ("export.png.bit_depth", "bits_8", "8"),
        ("export.png.bit_depth", "bits_16", "16"),
    ];
    let renamed = RENAMED
        .iter()
        .find(|(k, row, _)| *k == key && *row == option);
    renamed.map_or(option, |(_, _, id)| id)
}

/// The setting row of Export's page naming `key` in full.
fn row_of(key: &str) -> Option<&'static SettingRow> {
    fn find(rows: &'static [Row], key: &str) -> Option<&'static SettingRow> {
        rows.iter().find_map(|row| match row {
            Row::Setting(setting) if setting.key == Some(key) => Some(setting),
            row => find(row.rows(), key),
        })
    }
    let tool = &definition().tool;
    let tabs = tool.tabs.iter().map(|tab| tab.rows);
    let all = std::iter::once(tool.before).chain(tabs).chain([tool.after]);
    all.into_iter().find_map(|rows| find(rows, key))
}

impl Values {
    /// What the session's `Export` and `PreviewExport` take: the path, On conflict, the
    /// format with its options and the size as the tool asks for it
    /// (`Request::dialog`, whose rule for a side left at 0 applies). A JPEG's density is
    /// the print dpi in the cm and inch modes, as darktable's `metadata/resolution` is.
    ///
    /// The profile is not among them yet: both of the tool's choices export the Output
    /// colour profile's sRGB, the only profile an edit can have (decision 26).
    pub fn settings(&self) -> ExportSettings {
        let format = match self.format {
            FileFormat::Jpeg => Format::Jpeg(jpeg::Options {
                quality: self.jpeg_quality,
                subsample: self.jpeg_subsample,
                dpi: match self.size_mode {
                    SizeMode::Cm | SizeMode::Inch => self.dpi as u16,
                    SizeMode::Pixels | SizeMode::Scale => jpeg::Options::default().dpi,
                },
            }),
            FileFormat::Png => Format::Png(png::Options {
                bit_depth: self.png_bit_depth,
                compression: self.png_compression,
            }),
        };
        let by_scale = self.size_mode == SizeMode::Scale;
        ExportSettings {
            path: self.path.clone(),
            on_conflict: self.on_conflict,
            format,
            size: Request {
                width: if by_scale { 0 } else { self.width },
                height: if by_scale { 0 } else { self.height },
                scale: by_scale.then(|| self.scale.clone()),
                upscale: self.upscale,
                high_quality: self.high_quality,
                dialog: true,
            },
        }
    }
}

#[cfg(test)]
mod tests;

/// Whether an export with `on_conflict` asks the overwrite question first, by the tool's
/// Export and by Export Again: darktable's rule (`disk.c`, `ask_user_confirmation`),
/// On conflict Overwrite whether or not a file is there, and never Overwrite if changed.
/// Whether it asks then is the setting's (`alerts::Question`).
pub fn asks_before(on_conflict: OnConflict) -> bool {
    on_conflict == OnConflict::Overwrite
}

/// The person's folders and name on this machine, for darktable's variables, which the
/// engine reads from no platform folder itself (decision 43).
pub fn places() -> Places {
    let dirs = directories::UserDirs::new();
    let folder = |pick: fn(&directories::UserDirs) -> Option<&Path>| {
        dirs.as_ref().and_then(pick).map(Path::to_owned)
    };
    Places {
        home: folder(|dirs| Some(dirs.home_dir())),
        pictures: folder(directories::UserDirs::picture_dir),
        desktop: folder(directories::UserDirs::desktop_dir),
        user_name: ["USER", "USERNAME", "LOGNAME"]
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|v| !v.is_empty()))
            .unwrap_or_default(),
    }
}

/// The session's preview of what `values` would export for the photo open at `photo`: what
/// the app asks for as the tool is shown, as the settings change and as the photo does.
/// The export itself is [`crate::export_again::export`], whose settings are the ones the
/// app holds; the photo's edit is the session's to take as it is handled (D12).
pub fn preview_command(photo: &Path, values: &Values, places: &Places) -> Command {
    Command::PreviewExport {
        photos: vec![photo.to_owned()],
        settings: values.settings(),
        places: places.clone(),
    }
}

/// When the app asks the session for a preview: while the tool is open and the Photos tab
/// shown, for the photo open, on the first frame it is, and again as the photo, the settings
/// or the generation change, the generation being what else makes an answer stale: the
/// photo's edit changing, an export ending. Nothing is asked while the tool is closed or
/// out of view, so showing it again asks afresh.
#[derive(Debug, Default)]
pub struct Previews {
    /// What the session was last asked about, none while the tool was not shown.
    sent: Option<Sent>,
}

/// A preview asked for.
#[derive(Debug)]
struct Sent {
    photo: PathBuf,
    values: Values,
    generation: u64,
}

/// What [`Previews::next`] says to ask.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    /// Nothing: nothing changed, or the tool is not shown.
    Nothing,
    /// The settings, or the photo's edit or an export's files, changed for the photo the
    /// tool already previews.
    Changed(PathBuf, Values),
    /// A first preview, for a photo or a showing: answers to earlier commands, and the
    /// answer held, are for another photo or showing.
    Fresh(PathBuf, Values),
}

impl Previews {
    /// Whether the last preview asked for was for `photo`, the one open or none, while the
    /// tool was shown: an answer held is for that photo and showing alone.
    pub fn asked_about(&self, photo: Option<&Path>) -> bool {
        self.sent.as_ref().map(|sent| sent.photo.as_path()) == photo
    }

    /// What to ask now, with `photo` open or none, the tool `shown` or not, the settings as
    /// `values` have them and the `generation` of what else the answer depends on. Called
    /// every frame: it compares by reference and clones only when it asks.
    pub fn next(
        &mut self,
        photo: Option<&Path>,
        shown: bool,
        values: &Values,
        generation: u64,
    ) -> Ask {
        let wanted = photo.filter(|_| shown);
        let same = match (&self.sent, wanted) {
            (None, None) => true,
            (Some(sent), Some(photo)) => {
                sent.photo == photo && sent.values == *values && sent.generation == generation
            }
            _ => false,
        };
        if same {
            return Ask::Nothing;
        }
        let fresh = self.sent.as_ref().map(|sent| sent.photo.as_path()) != wanted;
        let Some(photo) = wanted else {
            self.sent = None;
            return Ask::Nothing;
        };
        self.sent = Some(Sent {
            photo: photo.to_owned(),
            values: values.clone(),
            generation,
        });
        let (photo, values) = (photo.to_owned(), values.clone());
        if fresh {
            Ask::Fresh(photo, values)
        } else {
            Ask::Changed(photo, values)
        }
    }
}

/// What a frame of the tool asked of the app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// Whether its custom rows, its ↺ or a fix changed the settings this frame: the app
    /// keeps the tool's settings as they stand once the frame's setting rows' changes are
    /// routed to it ([`Export::values`]), and asks the session for a new preview.
    pub changed: bool,
    /// Export was pressed, past the guard against a second press: the app decides what
    /// that does from the settings the export takes, once the frame's changes are in
    /// ([`press`]).
    pub pressed: bool,
    /// The folder button was pressed: the app asks the platform's folder dialog where the
    /// files go, opening it where the path is ([`folder::look`]), and hands the folder
    /// chosen back ([`Export::choose_folder`]).
    pub choose_folder: bool,
}

/// What a press of Export does with `values`, the settings the export takes, the frame's
/// changes in: it asks first with On conflict Overwrite ([`asks_before`]), sending the
/// export only on a yes, or at once where the setting says not to ask; otherwise it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Start,
    Confirm,
}

/// What a press of Export does with `values` ([`Press`]).
pub fn press(values: &Values) -> Press {
    if asks_before(values.on_conflict) {
        Press::Confirm
    } else {
        Press::Start
    }
}

/// What the page's custom rows asked for this frame (decision 93).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The Summary's row: open it, or close it.
    Summary,
    /// Export was pressed.
    Pressed,
    /// Show jobs was pressed.
    ShowJobs,
    /// The folder button at the path's end was pressed.
    ChooseFolder,
}

/// What the tool's section did this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// Its header was pressed: open it, or close it.
    pub toggle: bool,
    /// The summary's row was pressed: open it, or close it.
    pub toggle_summary: bool,
    /// Show jobs was pressed.
    pub show_jobs: bool,
    pub asked: Asked,
    /// What the page drawer's rows asked for, for the app to carry out: a setting changed,
    /// by its key in full, routed back to the tool, and Global options opened or closed.
    pub changes: Vec<Change>,
}

/// The tool's state: the settings it edits and what the session said of them.
#[derive(Debug)]
pub struct Export {
    values: Values,
    /// The newest answer of the session, with the number of the command it answers.
    preview: Option<(u64, ExportPreview)>,
    /// The number of the latest preview the app asked for: an answer to an earlier one is
    /// stale ([`Export::receive`]).
    sent: u64,
    /// The person's home, which paths are shown relative to as `~`.
    home: Option<PathBuf>,
    /// The print lengths last entered, which a change of resolution keeps ([`Print`]).
    print: Option<Print>,
    /// Whether the platform can ask the overwrite question here
    /// (`dialog_host::platform::shows_alerts`): where it cannot, On conflict's Overwrite
    /// is listed disabled, and Export rests while it is chosen.
    alerts: bool,
    /// The photo it acts on and how the user's exports stand, as the last frame was told.
    photo: Photo,
    others: Others,
    jobs: Jobs,
    /// When Export was last pressed to some effect, for the guard against a second press.
    pressed_at: Option<f64>,
    /// Whether the Summary is open, as the last frame was told.
    summary: bool,
    /// Whether its ↺ applied Default settings this frame, which the app shows at once.
    reset: bool,
    /// The keys of its settings `--conf` gives for this run.
    given: BTreeSet<String>,
    /// Whether the tool had the focus as its last frame ended, and in a field typed in,
    /// which keeps Esc: what takes its keys ([`Export::keys`]). egui lets go of the focus
    /// as Esc's frame starts, so Esc is the tool's by where the focus stood before it.
    holds: Option<Holds>,
    /// The Export button's id the last frame, while it could take the focus.
    button: Option<egui::Id>,
    /// Whether the button takes the focus as it is next drawn: Export… revealed the tool.
    focus_button: bool,
    /// What had the focus as Export… revealed the tool, which Esc gives it back to; none
    /// for the Viewer, as for nothing focused. Kept while the focus stays in the tool.
    before: Option<egui::Id>,
    /// Whether Ctrl+Return pressed Export this frame, as the button would be.
    keyed: bool,
}

/// Export's declaration, in `UTILITIES` (decision 93).
pub fn definition() -> &'static UtilityDefinition {
    apertrust_tools::utility::utility(ID).expect("Export is built in")
}

/// Export as a utility tool (decision 93): its settings, its page's setting rows, bound to
/// [`Values`] by their keys, those not built yet drawn disabled; its custom rows, Target
/// storage, the path line, the size line, the groups to come and the Summary with the
/// tool's end; what the session says, as the tooltips and problems of the path and the
/// size; and its ↺, Default settings.
impl UtilityTool for Export {
    type Action = Page;

    fn definition(&self) -> &'static UtilityDefinition {
        definition()
    }

    fn body(&mut self, widget: Widget, ui: &mut Ui, _cx: &mut Context<'_>) -> Vec<Page> {
        // Its page's spacing is set as the page drawer starts ([`UtilityTool::spacing`]).
        let t = theme(ui.ctx());
        match widget {
            Widget::ExportStorage => Export::target_storage(ui),
            Widget::ExportPath if self.path_line(ui, &t) => return vec![Page::ChooseFolder],
            Widget::ExportPath => {}
            Widget::ExportSize => self.size_line(ui, &t),
            Widget::ExportLater => Export::later_groups(ui),
            Widget::ExportEnd => return self.summary_and_end(ui, &t),
            _ => {}
        }
        Vec::new()
    }

    /// Default settings: every export setting at darktable's default, but those `--conf`
    /// gives for this run, which keep their values, as the store keeps them.
    fn reset(&mut self, _cx: &mut Context<'_>) {
        self.values = self.defaults();
        self.reset = true;
    }

    fn given(&self, key: &str) -> bool {
        self.given.contains(key)
    }

    /// Its setting row `row`'s value: a choice the option whose id is the one the settings
    /// hold ([`stored_id`]).
    fn setting(&self, row: &SettingRow) -> Option<ControlValue> {
        let key = row.key?;
        Some(match self.values.get(key)? {
            Kept::Choice(id) => {
                let ControlKind::Choice { options, .. } = row.kind else {
                    return None;
                };
                let at = options
                    .iter()
                    .position(|option| stored_id(key, option) == id)?;
                ControlValue::Choice(at as u32)
            }
            Kept::Number(number) => ControlValue::Number(number),
            Kept::Bool(on) => ControlValue::Bool(on),
            Kept::Text(_) => return None,
        })
    }

    /// The setting `key` as its row reports it, a choice by the id of the option there.
    fn set(&mut self, key: &str, value: &ControlValue) {
        let kept = match value {
            ControlValue::Choice(at) => {
                let options = match row_of(key).map(|row| row.kind) {
                    Some(ControlKind::Choice { options, .. }) => options,
                    _ => return,
                };
                let Some(option) = options.get(*at as usize) else {
                    return;
                };
                Kept::Choice(stored_id(key, option))
            }
            ControlValue::Number(number) => Kept::Number(*number),
            ControlValue::Bool(on) => Kept::Bool(*on),
            _ => return,
        };
        self.values.set(key, &kept);
    }

    /// The options the settings do not take, not built yet, as the formats but JPEG and
    /// PNG and the profiles but Image settings and sRGB; and On conflict's Overwrite where
    /// the platform cannot ask first.
    fn disabled(&self, row: &SettingRow, words: &Translator) -> Option<(Vec<usize>, String)> {
        let ControlKind::Choice { options, .. } = row.kind else {
            return None;
        };
        let key = row.key?;
        if key == "export.disk.on_conflict" && !self.alerts {
            let overwrite = id_of(OnConflict::Overwrite);
            let at = options.iter().position(|option| *option == overwrite);
            let why = words.text("export-conflict-overwrite-no-alert");
            return Some((at.into_iter().collect(), why));
        }
        let built = Values::options(key);
        let unbuilt: Vec<usize> = (options.iter().enumerate())
            .filter(|(_, option)| !built.contains(&stored_id(key, option)))
            .map(|(at, _)| at)
            .collect();
        (!unbuilt.is_empty()).then(|| (unbuilt, words.text("export-later")))
    }

    /// Set size's: what the photo will be, from the session's answer, and what the fields
    /// come to. File format's: that this version writes no metadata into exported files.
    fn tip(&self, row: &Row, words: &Translator) -> Option<String> {
        let Row::Setting(setting) = row else {
            return None;
        };
        match setting.key? {
            "export.size.mode" => self.size_tip(words),
            "export.format" => Some(words.text("export-metadata-note")),
            _ => None,
        }
    }

    /// What stops the path or its folder, with its fix, under the path line; what stops
    /// the size under the size line. A path `--conf` gives keeps its note with no fix,
    /// since nothing here may change it.
    fn note(&self, row: &Row, words: &Translator) -> Option<Note> {
        match row {
            Row::Custom {
                widget: Widget::ExportPath,
                ..
            } => {
                let problem = self.path_problem(words)?;
                let fix = problem.fix.filter(|_| !self.path_given());
                let fix = fix.map(|(name, _)| {
                    let mut args = Args::new();
                    args.set("name", name.as_str());
                    words.text_with("export-fix", Some(&args))
                });
                Some(Note {
                    text: problem.text,
                    fix,
                })
            }
            Row::Custom {
                widget: Widget::ExportSize,
                ..
            } => {
                let (_, preview) = self.preview.as_ref()?;
                let problem = preview.size.as_ref()?.size.as_ref().err()?;
                Some(Note {
                    text: size_problem(words, problem),
                    fix: None,
                })
            }
            _ => None,
        }
    }

    /// The path's fix: the variable the session suggests in place of the unknown one; none
    /// for a path `--conf` gives.
    fn fix(&mut self, row: &Row, words: &Translator) {
        if let Row::Custom {
            widget: Widget::ExportPath,
            ..
        } = row
            && !self.path_given()
            && let Some((_, fixed)) = self.path_problem(words).and_then(|problem| problem.fix)
        {
            self.values.path = fixed;
        }
    }

    fn set_in(&self, widget: Widget) -> usize {
        match widget {
            Widget::ExportSize => self.values.set_in_size(),
            _ => 0,
        }
    }

    /// Its rows stack with no gap between them, as a tool's do (docs/gui.md, How the rows
    /// are laid out).
    fn spacing(&self, ui: &Ui) -> Option<egui::Vec2> {
        Some(vec2(theme(ui.ctx()).spacing.s, 0.0))
    }

    /// Only the focused row takes the wheel, never the one under the pointer, so a scroll
    /// through its settings never changes where or what it writes (decision 90).
    fn wheel(&self) -> Wheel {
        Wheel::Focused
    }
}

/// The focus in the tool as a frame of it ends ([`Export::keys`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Holds {
    /// Whether a field is being typed in, which keeps Esc for its own.
    typing: bool,
}

impl Holds {
    /// Where the focus stands, `ctx`'s, with the tool drawn in `area`: on a widget there,
    /// or none.
    fn now(ctx: &egui::Context, area: egui::Rect) -> Option<Holds> {
        let focused = ctx.memory(|m| m.focused())?;
        let at = ctx.read_response(focused)?;
        area.contains_rect(at.rect).then(|| Holds {
            typing: ctx.text_edit_focused(),
        })
    }
}

/// What the cm and inch fields hold, as darktable's print entries do, with the pixels they
/// gave at the resolution `dpi`: while the pixels and the resolution are still those, the
/// lengths are the physical size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Print {
    mode: SizeMode,
    lengths: [f64; 2],
    pixels: [u32; 2],
    dpi: u32,
}

impl Export {
    /// The tool over `values`, with paths shown relative to the person's home.
    pub fn new(values: Values) -> Export {
        Export {
            values,
            preview: None,
            sent: 0,
            home: places().home,
            print: None,
            alerts: true,
            photo: Photo::None,
            others: Others::default(),
            jobs: Jobs::default(),
            pressed_at: None,
            summary: SUMMARY_OPEN,
            reset: false,
            given: BTreeSet::new(),
            holds: None,
            button: None,
            focus_button: false,
            before: None,
            keyed: false,
        }
    }

    /// Whether `--conf` gives the path for this run.
    fn path_given(&self) -> bool {
        self.given.contains("export.disk.path")
    }

    /// What Default settings gives: every export setting at darktable's default, but those
    /// `--conf` gives for this run, at their values.
    fn defaults(&self) -> Values {
        let mut values = Values::default();
        for key in &self.given {
            if let Some(kept) = self.values.get(key) {
                values.set(key, &kept);
            }
        }
        values
    }

    /// The tool where the platform can, or cannot, ask an alert.
    pub fn alerts(mut self, alerts: bool) -> Export {
        self.alerts = alerts;
        self
    }

    /// The tool with the keys of its settings `--conf` gives for this run (decision 43):
    /// their rows drawn disabled at those values, saying so, their changes refused, and
    /// Default settings keeping them.
    pub fn given(mut self, keys: impl IntoIterator<Item = String>) -> Export {
        self.given = keys.into_iter().collect();
        self
    }

    /// The tool with a home of its own, for tests.
    #[cfg(test)]
    pub fn with_home(values: Values, home: Option<PathBuf>) -> Export {
        Export {
            home,
            ..Export::new(values)
        }
    }

    /// Notes that the app asked the session for the preview numbered `number`: an answer to
    /// an earlier request that arrives after it is for settings no longer in place.
    pub fn sent(&mut self, number: u64) {
        self.sent = self.sent.max(number);
    }

    /// Takes the session's answer for the command numbered `through` where it answers the
    /// latest request or a later one ([`Export::sent`]), and is not older than the one held:
    /// a preview cancelled in time sends nothing, but one that had finished sends its
    /// answer, which may come after a newer request and is dropped.
    pub fn receive(&mut self, through: u64, preview: ExportPreview) {
        let older = (self.preview.as_ref()).is_some_and(|(held, _)| through < *held);
        if through >= self.sent && !older {
            self.preview = Some((through, preview));
        }
    }

    /// Takes the folder chosen in the folder button's dialog: the path becomes `folder`
    /// followed by `file`, the file part kept from the path as the button was pressed
    /// ([`folder::destination`]), the person's home written `~`. Nothing where `--conf`
    /// gives the path. Returns whether the path changed, for the app to keep it and ask
    /// for a new preview, as a path typed is.
    pub fn choose_folder(&mut self, folder: &Path, file: &str) -> bool {
        if self.path_given() {
            return false;
        }
        let path = folder::destination(folder, file, self.home.as_deref(), folder::WINDOWS);
        let changed = path != self.values.path;
        self.values.path = path;
        changed
    }

    /// The person's home, which paths are shown relative to and the folder button opens
    /// at where the path names no folder.
    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    /// Drops the answer held, which was for another photo or an earlier showing.
    pub fn forget(&mut self) {
        self.preview = None;
    }

    /// Drops the answer held unless `previews` last asked about `photo`, the one open, which
    /// is the photo the answer was made for. Called before the tool is drawn, so a photo
    /// opened since the last frame never shows the earlier one's feedback, nor lets it
    /// decide whether Export rests; the first preview for the new photo is asked after the
    /// draw ([`Previews::next`]).
    pub fn retire(&mut self, previews: &Previews, photo: Option<&Path>) {
        if !previews.asked_about(photo) {
            self.forget();
        }
    }

    /// The settings as they stand.
    pub fn values(&self) -> &Values {
        &self.values
    }

    /// Draws the section in `ui`, its header as `head` has it, the shell's as every
    /// utility's is, and the rest as `shown` says, through its `UtilityTool` with every
    /// tool's header and the one page drawer (decision 93): Global options open as `cx`'s
    /// settings keep it, the Summary, the photo it acts on and the jobs. Returns what it
    /// asked of the app: the header pressed, the Summary's, the settings changed, those of
    /// its setting rows as changes for the app to route back to it, Global options', Export
    /// and Show jobs.
    pub fn show(&mut self, ui: &mut Ui, head: Head, shown: &Shown, cx: &mut Context<'_>) -> Drawn {
        let ctx = ui.ctx().clone();
        let before = self.values.clone();
        self.photo = shown.photo.clone();
        self.others = shown.others;
        self.jobs = shown.jobs.clone();
        self.summary = shown.summary;
        // Its ↺ applies Default settings, drawn disabled while they hold; then the presets
        // button, drawn disabled until presets come, then its options, whose Reset is ↺.
        let resettable = self.values != self.defaults();
        self.keys(&ctx);
        let top = ui.cursor().top();
        let mut drawn = crate::utility::section(ui, self, cx, head, resettable);
        let area = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), top..=ui.cursor().top());
        self.holds = Holds::now(&ctx, area);
        // The focus gone elsewhere: Esc has nothing to give back to.
        if self.holds.is_none() {
            self.before = None;
        }
        // Asked of the button as it was drawn this frame, or nowhere where it was not.
        self.keyed = false;
        self.focus_button = false;
        // Its ↺ pressed in this frame: Default settings is what was asked for last, so an
        // edit its rows commit in the same frame, a number typed and left by the click,
        // goes, a setting row's or a custom row's; groups opened or closed stay.
        if std::mem::take(&mut self.reset) {
            self.values = self.defaults();
            (drawn.changes).retain(|change| !matches!(change, Change::Setting { .. }));
            ctx.request_repaint();
        }
        let has = |action: Page| drawn.actions.contains(&action);
        // A second press just after the first is ignored.
        let now = ctx.input(|i| i.time);
        // Whether it asks first is the app's to say, from the settings once the frame's
        // changes are in: a change to Overwrite in this frame asks too.
        let pressed = has(Page::Pressed) && !self.too_soon(now);
        Drawn {
            toggle: drawn.pressed.contains(&Pressed::Toggle),
            toggle_summary: has(Page::Summary),
            show_jobs: has(Page::ShowJobs),
            asked: Asked {
                changed: self.values != before,
                pressed,
                choose_folder: has(Page::ChooseFolder) && !self.path_given(),
            },
            changes: drawn.changes,
        }
    }

    /// Export… revealed the tool: the Export button takes the focus as it is drawn, so
    /// Return presses it, and Esc gives the focus back to what had it, `ctx`'s now. Called
    /// before [`Export::show`] in the frame that scrolls to the tool.
    pub fn reveal(&mut self, ctx: &egui::Context) {
        self.before = ctx.memory(|m| m.focused());
        self.focus_button = true;
    }

    /// The tool's keys while the focus is in it, but in an open list, whose keys are its
    /// own, before its rows see them: Ctrl+Return (Cmd+Return) presses Export as the button
    /// would, the focus moving to the button as a click there takes it, so a value being
    /// typed is given first; Esc, but in a field typed in, gives the focus back to what had
    /// it before Export… revealed the tool, or to the Viewer, letting it go.
    fn keys(&mut self, ctx: &egui::Context) {
        let Some(holds) = self.holds else {
            return;
        };
        if apertrust_ui::popup::any_open(ctx) {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)) {
            self.keyed = true;
            if let Some(button) = self.button {
                ctx.memory_mut(|m| m.request_focus(button));
            }
        }
        let escape = !holds.typing
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        if escape {
            // egui has let go of it already; where it was before goes back there.
            let focused = ctx.memory(|m| m.focused());
            match (self.before.take(), focused) {
                (Some(before), _) => ctx.memory_mut(|m| m.request_focus(before)),
                (None, Some(focused)) => ctx.memory_mut(|m| m.surrender_focus(focused)),
                (None, None) => {}
            }
        }
    }

    /// Target storage: File on disk, the one that can be chosen; the others darktable lists
    /// drawn disabled, saying they come later. No setting holds it yet.
    fn target_storage(ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let later = tr(&ctx, "export-later");
        let storages = [
            tr(&ctx, "export-storage-latex"),
            tr(&ctx, "export-storage-disk"),
            tr(&ctx, "export-storage-email"),
            tr(&ctx, "export-storage-gallery"),
        ];
        let storages: Vec<&str> = storages.iter().map(String::as_str).collect();
        let label = tr(&ctx, "export-target-storage");
        let _ = choice(&label, &storages, 1)
            .disabled_options(&[0, 2, 3], &later)
            .show(ui);
    }

    /// The groups to come, Metadata and Multi-preset export, each 4 points under the row
    /// before it, as in a tool, drawn disabled.
    fn later_groups(ui: &mut Ui) {
        let later = tr(ui.ctx(), "export-later");
        for id in ["export-metadata", "export-multi-preset"] {
            let title = tr(ui.ctx(), id);
            CollapsibleGroup::new(&title, false)
                .disabled(&later)
                .show(ui, |_| {});
        }
    }

    /// The Summary, open at first, of what an export would write, none with no photo; then
    /// the tool's end. What was pressed.
    fn summary_and_end(&mut self, ui: &mut Ui, t: &Theme) -> Vec<Page> {
        let mut pressed = Vec::new();
        if matches!(self.photo, Photo::Ready { .. }) {
            let title = tr(ui.ctx(), "export-summary-title");
            let summary =
                CollapsibleGroup::new(&title, self.summary).show(ui, |ui| self.summary_list(ui, t));
            if summary {
                pressed.push(Page::Summary);
            }
        }
        let end = self.end(ui, t);
        if end.pressed {
            pressed.push(Page::Pressed);
        }
        if end.show_jobs {
            pressed.push(Page::ShowJobs);
        }
        pressed
    }

    /// The path on one line that scrolls, with the folder button at its end and no visible
    /// label, as darktable's; to screen readers it is named "Path", and the file name the
    /// session resolved is its tooltip and its description, with the variables with no value
    /// here and the sequence rule. A problem that stops Export (D8), with its fix, is the
    /// note under it ([`UtilityTool::note`]), outlining the field in `error`. The folder
    /// button, "Choose folder", is drawn disabled as the field is while `--conf` gives the
    /// path. Returns whether it was pressed, by a click or by Return or Space while focused.
    fn path_line(&mut self, ui: &mut Ui, t: &Theme) -> bool {
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        let name = tr(&ctx, "export-path");
        let listed: Vec<Variable> = variables::list()
            .iter()
            .map(|v| Variable {
                insert: v.insert(),
                description: v.description(),
            })
            .collect();
        let problem = self.path_problem(&words);
        let choose = tr(&ctx, "export-choose-folder");
        let tip = self.path_tip(&words);
        let given = self.path_given();
        // As high as the field and centred on its line, a square beside it.
        let folder = IconButton::new(Icon::ChooseFolder, &choose)
            .size(t.sizes.field)
            .enabled(!given);
        let mut chosen = false;
        // The field's width is the row's less the button and the gap, never the text's. The
        // field and the button share one line; the list of variables comes under it.
        let field = (ui.available_width() - folder.side(ui) - t.spacing.s).max(80.0);
        let output = PathField::new(
            egui::Id::unique("export path"),
            &name,
            &self.values.path,
            &listed,
        )
        .width(field)
        .error(problem.is_some())
        .enabled(!given)
        .beside(|ui| {
            chosen = folder.show(ui).clicked();
        })
        .show(ui);
        if let Some(text) = output.text {
            self.values.path = text;
        }
        // A path `--conf` gives says so, after the preview where there is one, its field
        // disabled.
        let tip = if given {
            let said = crate::utility::overridden(&words);
            Some(tip.map_or_else(|| said.clone(), |tip| format!("{tip}\n{said}")))
        } else {
            tip
        };
        if let Some(tip) = tip {
            ctx.accesskit_node_builder(output.response.id, |node| {
                node.set_description(tip.clone());
            });
            drop(
                output
                    .response
                    .on_hover_text(tip.clone())
                    .on_disabled_hover_text(tip),
            );
        }
        chosen
    }

    /// What the path field's tooltip says, from the session's answer: the first photo's
    /// name and where its file goes, "IMG_2041.ARW → ~/…/IMG_2041.jpg", the others summed
    /// up, the sequence rule and the variables with no value here. A name only a decode can
    /// give, for a photo that is not the open one, is never shown as if exact: the pattern
    /// stays as written, filled in as the photo is exported. None while there is no answer,
    /// or a problem stops the path.
    fn path_tip(&self, words: &Translator) -> Option<String> {
        let (_, preview) = self.preview.as_ref()?;
        if self.path_problem(words).is_some() {
            return None;
        }
        let mut lines = Vec::new();
        if let Some(first) = preview.names.first() {
            let source = first
                .source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut args = Args::new();
            args.set("source", source);
            lines.push(match &first.fate {
                Fate::File { .. } if !first.exact => {
                    args.set("pattern", self.values.path.as_str());
                    words.text_with("export-path-tip-unexact", Some(&args))
                }
                Fate::File { file, .. } => {
                    args.set("file", shown(file, self.home.as_deref()));
                    words.text_with("export-name", Some(&args))
                }
                Fate::NoName => words.text("export-no-name"),
            });
            if preview.names.len() > 1 {
                let mut args = Args::new();
                args.set("count", (preview.names.len() - 1) as f64);
                lines.push(words.text_with("export-names-more", Some(&args)));
            }
        }
        if preview.sequence_added {
            let mut args = Args::new();
            args.set("count", preview.names.len() as f64);
            lines.push(words.text_with("export-sequence-added", Some(&args)));
        }
        if !preview.not_here.is_empty() {
            let names: Vec<String> = preview
                .not_here
                .iter()
                .map(|u| format!("$({})", u.name))
                .collect();
            let mut args = Args::new();
            args.set("names", names.join(", "));
            args.set("count", names.len() as f64);
            lines.push(words.text_with("export-not-here", Some(&args)));
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    /// What stops the path or its folder, in words, with the fix the session suggests
    /// (D8): none while all is well, and none until the session has answered.
    fn path_problem(&self, words: &Translator) -> Option<Problem> {
        let (_, preview) = self.preview.as_ref()?;
        problem_of(
            words,
            preview,
            &self.values.path,
            self.home.as_deref(),
            true,
        )
    }

    /// What stops the path itself, every photo's, leaving out its folder's checks, which
    /// are the photo open's folder's ([`Self::path_problem`]).
    fn pattern_problem(&self, words: &Translator) -> Option<Problem> {
        let (_, preview) = self.preview.as_ref()?;
        problem_of(
            words,
            preview,
            &self.values.path,
            self.home.as_deref(),
            false,
        )
    }

    /// The size line for Set size's mode: its fields on one line, or the scale's. Set size
    /// itself is a setting row, its tooltip what the photo will be ([`Self::size_tip`]),
    /// and a problem with the size the note under this line ([`UtilityTool::note`]).
    fn size_line(&mut self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let overridden = crate::utility::overridden(&translator(&ctx));
        let given = ["export.size.width", "export.size.height", "export.size.dpi"]
            .map(|key| self.given.contains(key));
        let scale_given = self.given.contains("export.size.scale");
        let v = &mut self.values;
        match v.size_mode {
            SizeMode::Pixels => {
                let unit = tr(&ctx, "unit-pixels");
                let name = tr(&ctx, "export-size-pixels");
                let shown = [v.width, v.height].map(f64::from);
                let (typed, _) = size_line(
                    ui,
                    t,
                    &name,
                    shown,
                    0,
                    100_000.0,
                    &unit,
                    None,
                    given,
                    &overridden,
                );
                let [w, h] = typed;
                v.width = w.map_or(v.width, |typed| typed.round() as u32);
                v.height = h.map_or(v.height, |typed| typed.round() as u32);
            }
            SizeMode::Cm | SizeMode::Inch => {
                let cm = v.size_mode == SizeMode::Cm;
                let (name, unit) = if cm {
                    (tr(&ctx, "export-size-cm"), tr(&ctx, "export-unit-cm"))
                } else {
                    (tr(&ctx, "export-size-inch"), tr(&ctx, "export-unit-inch"))
                };
                let dpi = v.dpi.clamp(72, 9600);
                // darktable keeps what the print fields hold, written to two decimals, and
                // derives the pixels from it, so a change of resolution recomputes them from
                // the lengths entered and changing it back returns them (`_print_dpi_changed`,
                // `_resync_pixel_dimensions`). The lengths are held while the pixels are
                // still the ones they gave at this resolution; otherwise they are read off
                // the pixels.
                let shown = |pixels: u32, dpi: u32| {
                    let length = if cm {
                        size::pixels_to_cm(pixels, dpi)
                    } else {
                        size::pixels_to_inches(pixels, dpi)
                    };
                    (f64::from(length) * 100.0).round() / 100.0
                };
                let held = self.print.filter(|p| {
                    p.mode == v.size_mode && p.pixels == [v.width, v.height] && p.dpi == dpi
                });
                let mut lengths = held.map_or_else(
                    || [shown(v.width, dpi), shown(v.height, dpi)],
                    |p| p.lengths,
                );
                // One line: the lengths, the unit, @ and the resolution.
                let (typed, new) = size_line(
                    ui,
                    t,
                    &name,
                    lengths,
                    2,
                    10_000.0,
                    &unit,
                    Some(dpi),
                    given,
                    &overridden,
                );
                let mut pixels = [v.width, v.height];
                for (i, typed) in typed.into_iter().enumerate() {
                    if let Some(length) = typed {
                        lengths[i] = length;
                        pixels[i] = pixels_of(length, cm, dpi);
                    }
                }
                // A side `--conf` gives keeps its pixels, its length following the new
                // resolution; the others keep their lengths.
                if let Some(new) = new {
                    for i in 0..2 {
                        if given[i] {
                            lengths[i] = shown(pixels[i], new);
                        } else {
                            pixels[i] = pixels_of(lengths[i], cm, new);
                        }
                    }
                    v.dpi = new;
                }
                [v.width, v.height] = pixels;
                self.print = Some(Print {
                    mode: v.size_mode,
                    lengths,
                    pixels,
                    dpi: v.dpi.clamp(72, 9600),
                });
            }
            SizeMode::Scale => {
                let name = tr(&ctx, "export-scale");
                let mut new = None;
                end_row(ui, t, &name, 0.0, |ui| {
                    let (response, text) =
                        TextField::new(egui::Id::unique("export scale"), &name, &v.scale)
                            .width(80.0)
                            .enabled(!scale_given)
                            .show(ui);
                    if scale_given {
                        given_field(response, &overridden);
                    }
                    new = text;
                });
                if let Some(text) = new {
                    v.scale = text;
                }
            }
        }
    }

    /// What the Set size row's tooltip says: what the active photo will be, its size and
    /// whether that is its full size; in the print modes what the lengths come to in pixels,
    /// and for a scale how it is written. None where there is nothing to say.
    fn size_tip(&self, words: &Translator) -> Option<String> {
        let v = &self.values;
        let mut lines = Vec::new();
        if let Some((_, preview)) = &self.preview
            && let Some(active) = &preview.size
            && let Ok(size) = &active.size
        {
            let mut args = Args::new();
            args.set("file", file_name_of(&active.source));
            args.set("width", size.width.to_string());
            args.set("height", size.height.to_string());
            let full = size.scale == 1.0;
            let limited = v.size_mode != SizeMode::Scale && (v.width > 0 || v.height > 0);
            let bigger = (v.width > size.extent[0]) || (v.height > size.extent[1]);
            let id = if !full {
                "export-size"
            } else if !v.upscale
                && (bigger || v.size_mode == SizeMode::Scale && size::scale_factor(&v.scale) > 1.0)
            {
                "export-size-capped"
            } else if v.size_mode != SizeMode::Scale && !limited {
                "export-size-no-limit"
            } else {
                "export-size-full"
            };
            lines.push(words.text_with(id, Some(&args)));
        }
        match v.size_mode {
            SizeMode::Cm | SizeMode::Inch => {
                let mut args = Args::new();
                args.set("width", v.width.to_string());
                args.set("height", v.height.to_string());
                lines.push(words.text_with("export-equal-to", Some(&args)));
            }
            SizeMode::Scale => lines.push(words.text("export-scale-tip")),
            SizeMode::Pixels => {}
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}

/// A problem with the path or its folder, in words, and the fix to offer.
struct Problem {
    text: String,
    /// The suggested variable's name and the whole path with it in place of the unknown.
    fix: Option<(String, String)>,
}

/// What stops the path, or its folder, for `preview`, which answered for a path that may
/// since have changed: `path` is the one now, and a fix is offered only where it still has
/// the unknown name the preview found at the place it found it.
fn problem_of(
    words: &Translator,
    preview: &ExportPreview,
    path: &str,
    home: Option<&Path>,
    folders: bool,
) -> Option<Problem> {
    let say = |id: &str, pairs: &[(&str, String)]| {
        let mut args = Args::new();
        for (name, value) in pairs {
            args.set(name.to_string(), value.clone());
        }
        words.text_with(id, Some(&args))
    };
    if let Some(problem) = &preview.problem {
        let plain = |text: String| Problem { text, fix: None };
        return Some(match problem {
            PathProblem::Empty => plain(words.text("export-problem-empty")),
            PathProblem::Pattern(PatternError::Unknown {
                name,
                at,
                suggestion,
            }) => {
                if name.is_empty() {
                    plain(words.text("export-problem-nameless"))
                } else if let Some(suggestion) = suggestion {
                    let text = say(
                        "export-problem-unknown-near",
                        &[
                            ("name", name.clone()),
                            ("suggestion", (*suggestion).to_owned()),
                        ],
                    );
                    let fix = path.get(at.clone()).filter(|found| found == name).map(|_| {
                        let mut fixed = path.to_owned();
                        fixed.replace_range(at.clone(), suggestion);
                        ((*suggestion).to_owned(), fixed)
                    });
                    Problem { text, fix }
                } else {
                    plain(say("export-problem-unknown", &[("name", name.clone())]))
                }
            }
            PathProblem::Pattern(PatternError::Unclosed {
                variable, expected, ..
            }) => plain(say(
                "export-problem-unclosed",
                &[
                    ("variable", variable.clone()),
                    ("expected", expected.to_string()),
                ],
            )),
            PathProblem::Pattern(PatternError::NoOperation {
                variable, found, ..
            }) => plain(say(
                "export-problem-no-operation",
                &[("variable", variable.clone()), ("found", found.to_string())],
            )),
            PathProblem::Pattern(PatternError::TooDeep { .. }) => plain(say(
                "export-problem-too-deep",
                &[("depth", variables::MAX_DEPTH.to_string())],
            )),
        });
    }
    if !folders {
        return None;
    }
    preview.folders.iter().find_map(|check| {
        let Preflight::Blocked { why, .. } = &check.preflight else {
            return None;
        };
        let folder = shown(&check.folder, home);
        let id = match why {
            Blocked::NotConnected => "export-problem-not-connected",
            Blocked::NotAFolder => "export-problem-not-a-folder",
            Blocked::ReadOnly => "export-problem-read-only",
            Blocked::Unreadable(_) => "export-problem-unreadable",
        };
        let kind = match why {
            Blocked::Unreadable(kind) => kind.to_string(),
            _ => String::new(),
        };
        Some(Problem {
            text: say(id, &[("folder", folder), ("why", kind)]),
            fix: None,
        })
    })
}

/// The size problem in words.
fn size_problem(words: &Translator, problem: &SizeProblem) -> String {
    let mut args = Args::new();
    match problem {
        SizeProblem::TooLarge {
            format, largest, ..
        } => {
            args.set("format", *format);
            args.set("largest", largest.to_string());
            words.text_with("export-problem-too-large", Some(&args))
        }
        SizeProblem::Empty { width, height } => {
            args.set("width", width.to_string());
            args.set("height", height.to_string());
            words.text_with("export-problem-no-pixels", Some(&args))
        }
        SizeProblem::Failed(why) => {
            args.set("why", why.as_str());
            words.text_with("export-problem-failed", Some(&args))
        }
    }
}

/// A path as shown: under the person's home as `~/…`.
pub(crate) fn shown(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The name of the file at `path`, as shown.
fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A row of the tool laid out as a tool's one-line row: `panel_line` high, `label` at its
/// start, the fields `add` draws at its end, `inset` short of it, so a field's box ends with
/// the row and a unit after the fields where slider values end ([`field_inset`]). Where the
/// label and the fields do not fit the panel's width, the label has a line of its own and
/// the fields the next, at the row's end as before. The fields are added from the row's
/// start, in reading order for readers and Tab, after a space as wide as the last pass found
/// them, so a pass that finds them changed is drawn again.
fn end_row(ui: &mut Ui, t: &Theme, label: &str, inset: f32, add: impl FnOnce(&mut Ui)) {
    let id = ui.make_persistent_id(("export row", label));
    let width: Option<f32> = ui.data(|d| d.get_temp(id));
    let named = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(t.text.body),
            t.colors.text,
        )
        .size()
        .x
    });
    let fits = width.is_none_or(|w| named + t.spacing.s + w + inset <= ui.available_width());
    if !fits {
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            ui.label(RichText::new(label).size(t.text.body));
        });
    }
    ui.horizontal(|ui| {
        ui.set_min_height(t.sizes.panel_line);
        if fits {
            ui.label(RichText::new(label).size(t.text.body));
        }
        ui.add_space((ui.available_width() - width.unwrap_or(0.0) - inset).max(0.0));
        let start = ui.cursor().min.x;
        add(ui);
        let measured = ui.min_rect().max.x - start;
        if width != Some(measured) {
            ui.data_mut(|d| d.insert_temp(id, measured));
            ui.ctx().request_discard("an export row measured");
        }
    });
}

/// A dropdown row as a tool panel draws one: Blended, darktable's combobox, the option at
/// the row's end with no box (decision 42).
fn choice<'a>(label: &'a str, options: &'a [&'a str], selected: usize) -> Choice<'a> {
    Choice::new(label, options, selected).style(ChoiceStyle::Blended)
}

/// The pixels of a print length in centimetres (`cm`) or inches at `dpi`, as darktable's
/// `print2pixels` rounds them up.
fn pixels_of(length: f64, cm: bool, dpi: u32) -> u32 {
    if cm {
        size::cm_to_pixels(length as f32, dpi)
    } else {
        size::inches_to_pixels(length as f32, dpi)
    }
}

/// A field drawn disabled since `--conf` gives its setting: `overridden`, saying so, is its
/// tooltip and what screen readers hear of it, as the path's.
fn given_field(response: egui::Response, overridden: &str) {
    (response.ctx).accesskit_node_builder(response.id, |node| node.set_description(overridden));
    drop(response.on_disabled_hover_text(overridden));
}

/// The size row, on one line with no label: the width and height fields sharing the row's
/// width, `×` between them and `unit` after, and with a `dpi` the resolution, "@" then its
/// field then dpi. `name` is the row's name to screen readers, a group of the fields. They
/// show `width` and `height` with `decimals`. A field whose setting `--conf` gives, as
/// `given` says for the width, the height and the resolution, is drawn disabled with
/// `overridden` as its tooltip. Returns what the user typed in each, and the resolution
/// typed.
#[allow(clippy::too_many_arguments)]
fn size_line(
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    [width, height]: [f64; 2],
    decimals: usize,
    reach: f64,
    unit: &str,
    dpi: Option<u32>,
    given: [bool; 3],
    overridden: &str,
) -> ([Option<f64>; 2], Option<u32>) {
    // A field `--conf` gives, disabled and saying so; the others as they are.
    let show = |ui: &mut Ui, field: ValueField<'_>, given: bool| {
        if !given {
            return field.show(ui).1;
        }
        ui.add_enabled_ui(false, |ui| {
            let (response, _) = field.show(ui);
            given_field(response, overridden);
        });
        None
    };
    let ctx = ui.ctx().clone();
    let mut changed = [None; 2];
    let mut new_dpi = None;
    let small = egui::FontId::proportional(t.text.small);
    let wide = |ui: &Ui, text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), small.clone(), t.colors.text_secondary)
            .size()
            .x
    };
    let dpi_unit = tr(&ctx, "export-dpi-unit");
    let dpi_name = tr(&ctx, "export-dpi");
    let scope = ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = t.spacing.s;
        ui.set_min_height(t.sizes.panel_line);
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            let gap = t.spacing.s;
            let dpi_field = dpi.map(|dpi| {
                ValueField::new(
                    egui::Id::unique("export dpi"),
                    &dpi_name,
                    f64::from(dpi),
                    72.0..=9600.0,
                )
                .unit(&dpi_unit)
                .number_only()
            });
            // The labels and the gaps take what they need; the fields share the rest, the
            // resolution's by its number's width where there is room, and all shrink alike
            // where there is not, so nothing runs past the row.
            let mut fixed = wide(ui, "×") + wide(ui, unit) + 3.0 * gap;
            if dpi_field.is_some() {
                fixed += wide(ui, "@") + wide(ui, &dpi_unit) + 3.0 * gap;
            }
            let room = ui.available_width() - field_inset(t) - fixed;
            let dpi_width = dpi_field
                .as_ref()
                .map(|field| field.width(ui).min(room * 0.8 / 2.8).floor());
            let each = ((room - dpi_width.unwrap_or(0.0)) / 2.0).floor().max(8.0);
            let label = |ui: &mut Ui, text: &str| {
                ui.label(
                    RichText::new(text)
                        .size(t.text.small)
                        .color(t.colors.text_secondary),
                );
            };
            for (i, (value, side)) in [(width, "export-width"), (height, "export-height")]
                .into_iter()
                .enumerate()
            {
                if i == 1 {
                    label(ui, "×");
                }
                let side = tr(&ctx, side);
                let field = ValueField::new(
                    egui::Id::unique(("export side", i)),
                    &side,
                    value,
                    0.0..=reach,
                )
                .decimals(decimals)
                .unit(unit)
                .number_only();
                ui.allocate_ui(vec2(each, t.sizes.field), |ui| {
                    if let Some(typed) = show(ui, field, given[i]) {
                        changed[i] = Some(typed);
                    }
                });
            }
            label(ui, unit);
            if let Some(field) = dpi_field {
                label(ui, "@");
                let room = dpi_width.unwrap_or(0.0);
                ui.allocate_ui(vec2(room, t.sizes.field), |ui| {
                    if let Some(value) = show(ui, field, given[2]) {
                        new_dpi = Some(value.round() as u32);
                    }
                });
                label(ui, &dpi_unit);
            }
        });
        // The row is a group named by what it holds, the fields inside it.
        let at = ui.min_rect();
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::Group);
            node.set_label(name.to_owned());
            node.set_bounds(egui::accesskit::Rect {
                x0: f64::from(at.left()),
                y0: f64::from(at.top()),
                x1: f64::from(at.right()),
                y1: f64::from(at.bottom()),
            });
        });
    });
    drop(scope);
    (changed, new_dpi)
}

/// darktable's twelve export profiles in its order, by the Output colour profile tool's
/// messages for them.
const PROFILES: [&str; 12] = [
    "tool-colorout-control-type-option-srgb",
    "tool-colorout-control-type-option-adobe_rgb",
    "tool-colorout-control-type-option-lin_rec709",
    "tool-colorout-control-type-option-rec709",
    "tool-colorout-control-type-option-lin_rec2020",
    "tool-colorout-control-type-option-pq_rec2020",
    "tool-colorout-control-type-option-hlg_rec2020",
    "tool-colorout-control-type-option-pq_p3",
    "tool-colorout-control-type-option-hlg_p3",
    "tool-colorout-control-type-option-display_p3",
    "tool-colorout-control-type-option-prophoto_rgb",
    "tool-colorout-control-type-option-brg",
];
