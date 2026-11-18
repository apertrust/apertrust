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
//! row for row, a utility section of the Photos tab's side panel, after Browse. It is a
//! function of `&mut Ui`, so the app draws it in the panel and the interaction tests draw it
//! in a harness.
//!
//! **Its groups** are Storage options and Format options, plain titled groups always
//! shown; Global options, collapsible and open at first; and Metadata and Multi-preset
//! export, collapsible, closed and drawn disabled. **Its rows** are File on disk with its
//! path on one line that scrolls, the folder button at its end and no visible label; On
//! conflict, JPEG and PNG, Set size's four modes, Allow upscaling, High quality resampling,
//! and Profile with Image settings and sRGB. Every other row is drawn, disabled, saying it
//! is to come: the other storages and formats, Store masks, Intent and Style. The rows are
//! the side panel's controls laid out as a tool's: Blended choices, toggles and sliders,
//! one-line rows stacked with no spacing between them, a row whose label and fields do not
//! fit the panel's width putting its label above them. The header is every tool's
//! (decision 93), drawn from its declaration in `UTILITIES`: ↺, which applies Default
//! settings, the presets button, drawn disabled until presets come, and ⋯. Its page is the
//! one page drawer's, five custom rows this module draws through its `UtilityTool`, until
//! its rows are setting rows (step 93.6).
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
//! [`Export::receive`], which keeps the one with the highest `through`. The tool exports the
//! active photo, since nothing selects photos yet; exporting closes nothing.

use std::path::{Path, PathBuf};

use apertrust_base::settings::{Choice as _, Store};
use apertrust_engine::export_preview::{Fate, PathProblem, SizeProblem};
use apertrust_engine::session::{Command, ExportPreview, ExportSettings, Places};
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
use apertrust_tools::Widget;
use apertrust_tools::utility::UtilityDefinition;
use apertrust_ui::Icon;
use apertrust_ui::components::{
    Button, Choice, ChoiceAction, ChoiceStyle, CollapsibleGroup, IconButton, PathField,
    SliderAction, SliderRow, TextField, ValueField, Variable, field_inset, group_label,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Theme, theme, tr, translator};
use eframe::egui::{self, Color32, Label, RichText, Ui, vec2};

use crate::tools::{Head, Pressed};
use crate::utility::{Context, UtilityTool};

mod end;
pub use end::{Jobs, Photo, Shown, Why};

/// The tool's id, as the shell keeps its section's state and Export… reveals it.
pub const ID: &str = "export";
/// The collapsible groups' ids, as the shell keeps their state under the tool's.
pub const GLOBAL: &str = "global";
pub const SUMMARY: &str = "summary";
/// Whether Global options is open until the person closes it: the one group that starts
/// open (decision 90).
pub const GLOBAL_OPEN: bool = true;
/// Whether the Summary is open until the person closes it: it starts open, as Global
/// options does.
pub const SUMMARY_OPEN: bool = true;
/// Whether the section is open until the person opens or closes it, as every tool is
/// closed at first (decision 29).
pub const OPEN: bool = false;
/// How many photos the tool exports: the active one, until a selection comes.
const PHOTOS: usize = 1;

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

    /// How many of the settings Global options holds are off their defaults, for the dot a
    /// closed group shows.
    pub fn set_in_global(&self) -> usize {
        let d = Values::default();
        [
            self.size_mode != d.size_mode,
            self.width != d.width,
            self.height != d.height,
            self.dpi != d.dpi,
            self.scale != d.scale,
            self.upscale != d.upscale,
            self.high_quality != d.high_quality,
            self.profile != d.profile,
        ]
        .into_iter()
        .filter(|set| *set)
        .count()
    }

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
    /// The settings, when they changed this frame: the app keeps them and asks the session
    /// for a new preview.
    pub values: Option<Values>,
    /// Export was pressed: the app sends the session the export of the settings it holds.
    pub start: bool,
    /// Export was pressed with On conflict Overwrite ([`asks_before`]): the app asks first
    /// (`alerts::overwrite_request`), sending the export only on a yes, or at once where
    /// the setting says not to ask. Never together with `start`.
    pub confirm: bool,
}

/// What the page's custom rows asked for this frame (decision 93).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// Global options' row: open it, or close it.
    Global,
    /// The Summary's row: open it, or close it.
    Summary,
    /// Export was pressed.
    Pressed,
    /// Show jobs was pressed.
    ShowJobs,
}

/// What the tool's section did this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// Its header was pressed: open it, or close it.
    pub toggle: bool,
    /// Global options' row, or the summary's, was pressed: open it, or close it.
    pub toggle_global: bool,
    pub toggle_summary: bool,
    /// Show jobs was pressed.
    pub show_jobs: bool,
    pub asked: Asked,
}

/// The tool's state: the settings it edits and what the session said of them.
#[derive(Debug)]
pub struct Export {
    values: Values,
    /// The newest answer of the session, with the number of the command it answers.
    preview: Option<(u64, ExportPreview)>,
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
    jobs: Jobs,
    /// When Export was last pressed to some effect, for the guard against a second press.
    pressed_at: Option<f64>,
    /// Whether Global options and the Summary are open, as the last frame was told.
    groups: (bool, bool),
    /// Whether its ↺ applied Default settings this frame, which the app shows at once.
    reset: bool,
}

/// Export's declaration, in `UTILITIES` (decision 93).
pub fn definition() -> &'static UtilityDefinition {
    apertrust_tools::utility::utility(ID).expect("Export is built in")
}

/// Export as a utility tool (decision 93): its page five custom rows, Storage options,
/// Format options, Global options, the groups to come and the Summary with the tool's end,
/// each drawn here as before, until its rows are setting rows (step 93.6); its ↺ its own,
/// Default settings.
impl UtilityTool for Export {
    type Action = Page;

    fn definition(&self) -> &'static UtilityDefinition {
        definition()
    }

    fn body(&mut self, widget: Widget, ui: &mut Ui, _cx: &mut Context<'_>) -> Vec<Page> {
        let t = theme(ui.ctx());
        // Rows stack as a tool's do, each carrying its own gaps (docs/gui.md, How the rows
        // are laid out).
        ui.spacing_mut().item_spacing = vec2(t.spacing.s, 0.0);
        match widget {
            Widget::ExportStorage => self.storage_rows(ui, &t),
            // Format options, a plain group.
            Widget::ExportFormat => {
                group_label(ui, &tr(ui.ctx(), "export-format-options"));
                self.format_rows(ui);
            }
            Widget::ExportGlobal => {
                if self.global_group(ui, &t) {
                    return vec![Page::Global];
                }
            }
            Widget::ExportLater => Export::later_groups(ui),
            Widget::ExportEnd => return self.summary_and_end(ui, &t),
            _ => {}
        }
        Vec::new()
    }

    /// Default settings: every export setting at darktable's default.
    fn reset(&mut self, _cx: &mut Context<'_>) {
        self.values = Values::default();
        self.reset = true;
    }
}

/// What the cm and inch fields hold, as darktable's print entries do, with the pixels they
/// gave: while the pixels are still those, the lengths are the physical size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Print {
    mode: SizeMode,
    lengths: [f64; 2],
    pixels: [u32; 2],
}

impl Export {
    /// The tool over `values`, with paths shown relative to the person's home.
    pub fn new(values: Values) -> Export {
        Export {
            values,
            preview: None,
            home: places().home,
            print: None,
            alerts: true,
            photo: Photo::None,
            jobs: Jobs::default(),
            pressed_at: None,
            groups: (GLOBAL_OPEN, SUMMARY_OPEN),
            reset: false,
        }
    }

    /// The tool where the platform can, or cannot, ask an alert.
    pub fn alerts(mut self, alerts: bool) -> Export {
        self.alerts = alerts;
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

    /// Takes the session's answer for the command numbered `through`, if it is newer than
    /// the one held: a preview cancelled in time sends nothing, but one that had finished
    /// sends its answer, so a client keeps the highest.
    pub fn receive(&mut self, through: u64, preview: ExportPreview) {
        if self
            .preview
            .as_ref()
            .is_none_or(|(held, _)| through >= *held)
        {
            self.preview = Some((through, preview));
        }
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
    /// tool's header and the one page drawer (decision 93): its groups, the photo it acts
    /// on and the jobs. Returns what it asked of the app: the header pressed, the groups',
    /// the settings changed, Export and Show jobs.
    pub fn show(&mut self, ui: &mut Ui, head: Head, shown: &Shown, cx: &mut Context<'_>) -> Drawn {
        let ctx = ui.ctx().clone();
        let before = self.values.clone();
        self.photo = shown.photo.clone();
        self.jobs = shown.jobs.clone();
        self.groups = (shown.global, shown.summary);
        // Its ↺ applies Default settings, drawn disabled while they hold; then the presets
        // button, drawn disabled until presets come, then its options, which hold nothing
        // yet.
        let resettable = self.values != Values::default();
        let drawn = crate::utility::section(ui, self, cx, head, resettable);
        if std::mem::take(&mut self.reset) {
            ctx.request_repaint();
        }
        let has = |action: Page| drawn.actions.contains(&action);
        // A second press just after the first is ignored.
        let now = ctx.input(|i| i.time);
        let pressed = has(Page::Pressed) && !self.too_soon(now);
        // Overwriting asks first, as darktable's storage does whether or not a file is
        // there: the app sends the export on a yes, or at once where the setting says not
        // to ask.
        let confirm = pressed && asks_before(self.values.on_conflict);
        let start = pressed && !confirm;
        if start {
            self.note_started(now);
        }
        let values = (self.values != before).then(|| self.values.clone());
        Drawn {
            toggle: drawn.pressed.contains(&Pressed::Toggle),
            toggle_global: has(Page::Global),
            toggle_summary: has(Page::Summary),
            show_jobs: has(Page::ShowJobs),
            asked: Asked {
                values,
                start,
                confirm,
            },
        }
    }

    /// Storage options, a plain group: always shown, so where files go is in view. Target
    /// storage, File on disk's path and On conflict.
    fn storage_rows(&mut self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let later = tr(&ctx, "export-later");
        group_label(ui, &tr(&ctx, "export-storage-options"));
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

        // File on disk.
        self.path_rows(ui, t);
        let conflicts = [
            "export-conflict-unique",
            "export-conflict-overwrite",
            "export-conflict-changed",
            "export-conflict-skip",
        ]
        .map(|id| tr(&ctx, id));
        let conflicts: Vec<&str> = conflicts.iter().map(String::as_str).collect();
        let label = tr(&ctx, "export-on-conflict");
        let current = OnConflict::OPTIONS
            .iter()
            .position(|(c, _)| *c == self.values.on_conflict)
            .unwrap_or(0);
        let no_alert = tr(&ctx, "export-conflict-overwrite-no-alert");
        let overwrite = OnConflict::OPTIONS
            .iter()
            .position(|(c, _)| *c == OnConflict::Overwrite);
        let unasked: Vec<usize> = overwrite.filter(|_| !self.alerts).into_iter().collect();
        if let Some(ChoiceAction::Set(i)) = choice(&label, &conflicts, current)
            .disabled_options(&unasked, &no_alert)
            .show(ui)
        {
            self.values.on_conflict = OnConflict::OPTIONS[i].0;
        }
    }

    /// Global options, collapsible and open at first: the size is the next thing an export
    /// depends on. Whether it was pressed, to open or close it.
    fn global_group(&mut self, ui: &mut Ui, t: &Theme) -> bool {
        let title = tr(ui.ctx(), "export-global-options");
        let set = self.values.set_in_global();
        CollapsibleGroup::new(&title, self.groups.0)
            .set(set)
            .show(ui, |ui| self.global_rows(ui, t))
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
            let summary = CollapsibleGroup::new(&title, self.groups.1)
                .show(ui, |ui| self.summary_list(ui, t));
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

    /// Global options' rows: the size, the toggles, the profile and what is to come.
    fn global_rows(&mut self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        self.size_rows(ui, t);
        // darktable's choices of No and Yes, the settings staying booleans.
        let label = tr(&ctx, "export-upscale");
        if let Some(on) = no_or_yes(ui, &label, self.values.upscale, true, None) {
            self.values.upscale = on;
        }
        let label = tr(&ctx, "export-high-quality");
        let promise = tr(&ctx, "export-high-quality-tip");
        if let Some(on) = no_or_yes(ui, &label, self.values.high_quality, true, Some(&promise)) {
            self.values.high_quality = on;
        }
        let label = tr(&ctx, "export-masks");
        let later = tr(&ctx, "export-masks-later");
        let _ = no_or_yes(ui, &label, false, false, Some(&later));
        self.profile_rows(ui);
        let label = tr(&ctx, "export-intent");
        let intents = [
            tr(&ctx, "export-profile-image"),
            tr(&ctx, "export-intent-perceptual"),
            tr(&ctx, "export-intent-relative"),
            tr(&ctx, "export-intent-saturation"),
            tr(&ctx, "export-intent-absolute"),
        ];
        let intents: Vec<&str> = intents.iter().map(String::as_str).collect();
        let _ = choice(&label, &intents, 0)
            .enabled(false)
            .promise(&tr(&ctx, "export-intent-tip"))
            .show(ui);
        let label = tr(&ctx, "export-style");
        let none = [tr(&ctx, "export-style-none")];
        let none: Vec<&str> = none.iter().map(String::as_str).collect();
        let _ = choice(&label, &none, 0)
            .enabled(false)
            .promise(&tr(&ctx, "export-style-later"))
            .show(ui);
    }

    /// The path on one line that scrolls, with the folder button at its end and no visible
    /// label, as darktable's; to screen readers it is named "Path", and the file name the
    /// session resolved is its tooltip and its description, with the variables with no value
    /// here and the sequence rule. Under it, a problem that stops Export (D8) and its fix.
    fn path_rows(&mut self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let name = tr(&ctx, "export-path");
        let listed: Vec<Variable> = variables::list()
            .iter()
            .map(|v| Variable {
                insert: v.insert(),
                description: v.description(),
            })
            .collect();
        let problem = self.path_problem(&ctx);
        let choose = tr(&ctx, "export-choose-folder");
        let later = tr(&ctx, "export-later");
        let tip = self.path_tip(&ctx);
        // As high as the field and centred on its line, a square beside it.
        let folder = IconButton::new(Icon::Folder, &choose)
            .size(t.sizes.field)
            .enabled(false)
            .tip(&later);
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
        .beside(|ui| {
            folder.show(ui);
        })
        .show(ui);
        if let Some(text) = output.text {
            self.values.path = text;
        }
        if let Some(tip) = tip {
            ctx.accesskit_node_builder(output.response.id, |node| {
                node.set_description(tip.clone());
            });
            drop(output.response.on_hover_text(tip));
        }
        if let Some(problem) = &problem {
            let mut fix = None;
            note(ui, t, problem.text.clone(), t.colors.error);
            if let Some((name, text)) = &problem.fix {
                let mut args = Args::new();
                args.set("name", name.as_str());
                let label = translator(&ctx).text_with("export-fix", Some(&args));
                if Button::new(&label).show(ui).clicked() {
                    fix = Some(text.clone());
                }
            }
            if let Some(text) = fix {
                self.values.path = text;
            }
        }
    }

    /// What the path field's tooltip says, from the session's answer: the first photo's
    /// name and where its file goes, "IMG_2041.ARW → ~/…/IMG_2041.jpg", the others summed
    /// up, the sequence rule and the variables with no value here. A name only a decode can
    /// give, for a photo that is not the open one, is never shown as if exact: the pattern
    /// stays as written, filled in as the photo is exported. None while there is no answer,
    /// or a problem stops the path.
    fn path_tip(&self, ctx: &egui::Context) -> Option<String> {
        let (_, preview) = self.preview.as_ref()?;
        if self.path_problem(ctx).is_some() {
            return None;
        }
        let words = translator(ctx);
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
                Fate::NoName => tr(ctx, "export-no-name"),
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
    fn path_problem(&self, ctx: &egui::Context) -> Option<Problem> {
        let (_, preview) = self.preview.as_ref()?;
        problem_of(ctx, preview, &self.values.path, self.home.as_deref())
    }

    /// File format and the rows of the format chosen: JPEG's quality and subsampling, PNG's
    /// bit depth and compression. The formats darktable lists that Apertrust cannot write
    /// yet are listed, disabled.
    fn format_rows(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let later = tr(&ctx, "export-later");
        let copy = tr(&ctx, "export-format-copy");
        let names: Vec<&str> = FORMATS
            .iter()
            .map(|f| match f {
                Listed::Name(name) => *name,
                Listed::Copy => copy.as_str(),
            })
            .collect();
        let unbuilt: Vec<usize> = (0..names.len())
            .filter(|i| !matches!(*i, JPEG | PNG))
            .collect();
        let label = tr(&ctx, "export-file-format");
        let current = match self.values.format {
            FileFormat::Jpeg => JPEG,
            FileFormat::Png => PNG,
        };
        // Until metadata is built an exported file carries none: the row's tooltip says so.
        let metadata = tr(&ctx, "export-metadata-note");
        if let Some(ChoiceAction::Set(i)) = choice(&label, &names, current)
            .disabled_options(&unbuilt, &later)
            .promise(&metadata)
            .description(&metadata)
            .show(ui)
        {
            self.values.format = if i == PNG {
                FileFormat::Png
            } else {
                FileFormat::Jpeg
            };
        }
        match self.values.format {
            FileFormat::Jpeg => {
                let label = tr(&ctx, "export-jpeg-quality");
                let quality = f64::from(self.values.jpeg_quality);
                if let Some(value) = whole_slider(
                    ui,
                    &label,
                    quality,
                    5.0..=100.0,
                    EXPORT_JPEG_QUALITY.default().into(),
                ) {
                    self.values.jpeg_quality = value;
                }
                let label = tr(&ctx, "export-jpeg-subsample");
                let options = [
                    tr(&ctx, "export-subsample-auto"),
                    "4:4:4".to_owned(),
                    "4:4:0".to_owned(),
                    "4:2:2".to_owned(),
                    "4:2:0".to_owned(),
                ];
                let options: Vec<&str> = options.iter().map(String::as_str).collect();
                let current = Subsample::OPTIONS
                    .iter()
                    .position(|(s, _)| *s == self.values.jpeg_subsample)
                    .unwrap_or(0);
                let promise = tr(&ctx, "export-jpeg-subsample-tip");
                if let Some(ChoiceAction::Set(i)) =
                    choice(&label, &options, current).promise(&promise).show(ui)
                {
                    self.values.jpeg_subsample = Subsample::OPTIONS[i].0;
                }
            }
            FileFormat::Png => {
                let label = tr(&ctx, "export-png-bit-depth");
                let options = [tr(&ctx, "export-bits-8"), tr(&ctx, "export-bits-16")];
                let options: Vec<&str> = options.iter().map(String::as_str).collect();
                let current = BitDepth::OPTIONS
                    .iter()
                    .position(|(b, _)| *b == self.values.png_bit_depth)
                    .unwrap_or(0);
                if let Some(ChoiceAction::Set(i)) = choice(&label, &options, current).show(ui) {
                    self.values.png_bit_depth = BitDepth::OPTIONS[i].0;
                }
                let label = tr(&ctx, "export-png-compression");
                let compression = f64::from(self.values.png_compression);
                if let Some(value) = whole_slider(
                    ui,
                    &label,
                    compression,
                    0.0..=9.0,
                    EXPORT_PNG_COMPRESSION.default().into(),
                ) {
                    self.values.png_compression = value;
                }
            }
        }
    }

    /// Set size, its fields for the mode chosen, and the line saying what size the active
    /// photo will be.
    fn size_rows(&mut self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let modes = [
            tr(&ctx, "export-mode-pixels"),
            tr(&ctx, "export-mode-cm"),
            tr(&ctx, "export-mode-inch"),
            tr(&ctx, "export-mode-scale"),
        ];
        let modes: Vec<&str> = modes.iter().map(String::as_str).collect();
        let label = tr(&ctx, "export-set-size");
        let current = SizeMode::OPTIONS
            .iter()
            .position(|(m, _)| *m == self.values.size_mode)
            .unwrap_or(0);
        // What the size comes to is the row's tooltip, not a line under it.
        let tip = self.size_tip(&ctx);
        let mut chooser = choice(&label, &modes, current);
        if let Some(tip) = &tip {
            chooser = chooser.promise(tip).description(tip);
        }
        if let Some(ChoiceAction::Set(i)) = chooser.show(ui) {
            self.values.size_mode = SizeMode::OPTIONS[i].0;
        }
        let v = &mut self.values;
        match v.size_mode {
            SizeMode::Pixels => {
                let unit = tr(&ctx, "unit-pixels");
                let name = tr(&ctx, "export-size-pixels");
                let shown = [v.width, v.height].map(f64::from);
                let (typed, _) = size_line(ui, t, &name, shown, 0, 100_000.0, &unit, None);
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
                // still the ones they gave; otherwise they are read off the pixels.
                let shown = |pixels: u32| {
                    let length = if cm {
                        size::pixels_to_cm(pixels, dpi)
                    } else {
                        size::pixels_to_inches(pixels, dpi)
                    };
                    (f64::from(length) * 100.0).round() / 100.0
                };
                let held = self
                    .print
                    .filter(|p| p.mode == v.size_mode && p.pixels == [v.width, v.height]);
                let mut lengths =
                    held.map_or_else(|| [shown(v.width), shown(v.height)], |p| p.lengths);
                // One line: the lengths, the unit, @ and the resolution.
                let (typed, new) = size_line(ui, t, &name, lengths, 2, 10_000.0, &unit, Some(dpi));
                let mut pixels = [v.width, v.height];
                for (i, typed) in typed.into_iter().enumerate() {
                    if let Some(length) = typed {
                        lengths[i] = length;
                        pixels[i] = pixels_of(length, cm, dpi);
                    }
                }
                if let Some(new) = new {
                    pixels = lengths.map(|length| pixels_of(length, cm, new));
                    v.dpi = new;
                }
                [v.width, v.height] = pixels;
                self.print = Some(Print {
                    mode: v.size_mode,
                    lengths,
                    pixels,
                });
            }
            SizeMode::Scale => {
                let name = tr(&ctx, "export-scale");
                let mut new = None;
                end_row(ui, t, &name, 0.0, |ui| {
                    let (_, text) =
                        TextField::new(egui::Id::unique("export scale"), &name, &v.scale)
                            .width(80.0)
                            .show(ui);
                    new = text;
                });
                if let Some(text) = new {
                    v.scale = text;
                }
            }
        }
        self.size_problem(ui, t);
    }

    /// What the Set size row's tooltip says: what the active photo will be, its size and
    /// whether that is its full size; in the print modes what the lengths come to in pixels,
    /// and for a scale how it is written. None where there is nothing to say.
    fn size_tip(&self, ctx: &egui::Context) -> Option<String> {
        let words = translator(ctx);
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
            SizeMode::Scale => lines.push(tr(ctx, "export-scale-tip")),
            SizeMode::Pixels => {}
        }
        (!lines.is_empty()).then(|| lines.join("\n"))
    }

    /// The problem with the size that stops Export, `error` under its rows: a size over the
    /// format's limit, or one that leaves no pixel.
    fn size_problem(&self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let Some((_, preview)) = &self.preview else {
            return;
        };
        if let Some(active) = &preview.size
            && let Err(problem) = &active.size
        {
            note(ui, t, size_problem(&ctx, problem), t.colors.error);
        }
    }

    /// Profile: Image settings and sRGB, the rest of darktable's twelve listed, disabled.
    fn profile_rows(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let later = tr(&ctx, "export-later");
        let mut options = vec![tr(&ctx, "export-profile-image")];
        options.extend(PROFILES.iter().map(|id| tr(&ctx, id)));
        let options: Vec<&str> = options.iter().map(String::as_str).collect();
        let unbuilt: Vec<usize> = (2..options.len()).collect();
        let label = tr(&ctx, "export-profile");
        let current = match self.values.profile {
            Profile::ImageSettings => 0,
            Profile::Srgb => 1,
        };
        let promise = tr(&ctx, "export-profile-tip");
        if let Some(ChoiceAction::Set(i)) = choice(&label, &options, current)
            .promise(&promise)
            .disabled_options(&unbuilt, &later)
            .show(ui)
        {
            self.values.profile = if i == 0 {
                Profile::ImageSettings
            } else {
                Profile::Srgb
            };
        }
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
    ctx: &egui::Context,
    preview: &ExportPreview,
    path: &str,
    home: Option<&Path>,
) -> Option<Problem> {
    let words = translator(ctx);
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
            PathProblem::Empty => plain(tr(ctx, "export-problem-empty")),
            PathProblem::Pattern(PatternError::Unknown {
                name,
                at,
                suggestion,
            }) => {
                if name.is_empty() {
                    plain(tr(ctx, "export-problem-nameless"))
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
fn size_problem(ctx: &egui::Context, problem: &SizeProblem) -> String {
    let words = translator(ctx);
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

/// A slider row of a whole number, `default` its reset: the new value when the user sets
/// or resets it.
fn whole_slider(
    ui: &mut Ui,
    label: &str,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    default: f64,
) -> Option<u8> {
    let action = SliderRow::new(label, value, range)
        .default_value(default)
        .step(1.0)
        .decimals(0)
        .show(ui)?;
    match action {
        SliderAction::Set { value, .. } => Some(value.round() as u8),
        SliderAction::Reset => Some(default as u8),
        SliderAction::ToggleKeep | SliderAction::Quad | SliderAction::Picker(_) => None,
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

/// A problem under a row, in the small text: what stops Export, the one line a row keeps.
fn note(ui: &mut Ui, t: &Theme, text: impl Into<String>, colour: Color32) {
    ui.add(Label::new(RichText::new(text.into()).size(t.text.small).color(colour)).wrap());
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

/// A row of No and Yes, darktable's choice for a setting that is on or off: the new value
/// where the person chose the other. `promise` is its tooltip; a disabled row gives none.
fn no_or_yes(
    ui: &mut Ui,
    label: &str,
    on: bool,
    enabled: bool,
    promise: Option<&str>,
) -> Option<bool> {
    let (no, yes) = (
        tr(ui.ctx(), "export-option-no"),
        tr(ui.ctx(), "export-option-yes"),
    );
    let options = [no.as_str(), yes.as_str()];
    let mut row = choice(label, &options, usize::from(on)).enabled(enabled);
    if let Some(promise) = promise {
        row = row.promise(promise);
    }
    match row.show(ui)? {
        ChoiceAction::Set(i) => Some(i == 1),
        _ => None,
    }
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

/// The size row, on one line with no label: the width and height fields sharing the row's
/// width, `×` between them and `unit` after, and with a `dpi` the resolution, "@" then its
/// field then dpi. `name` is the row's name to screen readers, a group of the fields. They
/// show `width` and `height` with `decimals`. Returns what the user typed in each, and the
/// resolution typed.
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
) -> ([Option<f64>; 2], Option<u32>) {
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
                    if let (_, Some(typed)) = field.show(ui) {
                        changed[i] = Some(typed);
                    }
                });
            }
            label(ui, unit);
            if let Some(field) = dpi_field {
                label(ui, "@");
                let room = dpi_width.unwrap_or(0.0);
                ui.allocate_ui(vec2(room, t.sizes.field), |ui| {
                    if let (_, Some(value)) = field.show(ui) {
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

/// What the format list holds at a place.
enum Listed {
    Name(&'static str),
    Copy,
}

/// darktable's formats in its order, Copy last.
const FORMATS: [Listed; 14] = [
    Listed::Name("AVIF"),
    Listed::Name("HEIF"),
    Listed::Name("JPEG (8-bit)"),
    Listed::Name("JPEG 2000 (12-bit)"),
    Listed::Name("JPEG XL"),
    Listed::Name("OpenEXR"),
    Listed::Name("PDF"),
    Listed::Name("PFM"),
    Listed::Name("PNG"),
    Listed::Name("PPM (16-bit)"),
    Listed::Name("TIFF"),
    Listed::Name("WebP"),
    Listed::Name("XCF"),
    Listed::Copy,
];
/// Where JPEG and PNG stand in [`FORMATS`].
const JPEG: usize = 2;
const PNG: usize = 8;

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
