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
//! **Its rows** are File on disk with its path, On conflict, JPEG and PNG, Set size's four
//! modes, Allow upscaling, High quality resampling, and Profile with Image settings and
//! sRGB. Every other row is drawn, disabled, saying it is to come: the other storages and
//! formats, Store masks, Intent, Style, Metadata and Multi-preset export. The rows are the
//! side panel's controls laid out as a tool's: Blended choices, toggles and sliders,
//! one-line rows stacked with no spacing between them, a row whose label and fields do not
//! fit the panel's width putting its label above them. The header's ↺ applies Default
//! settings.
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
use apertrust_export::disk::{Action, Blocked, Preflight};
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
use apertrust_ui::components::{
    Button, Choice, ChoiceAction, ChoiceStyle, CollapsibleGroup, PathField, Section, SectionAction,
    SliderAction, SliderRow, TextField, ToggleRow, ValueField, Variable, field_inset, group_label,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Theme, theme, tr, translator};
use eframe::egui::{self, Color32, Label, RichText, Ui, vec2};

/// The tool's id, as the shell keeps its section's state and Export… reveals it.
pub const ID: &str = "export";
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

/// What the tool's section did this frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// Its header was pressed: open it, or close it.
    pub toggle: bool,
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
    /// Whether a photo is open to export, as the last frame was told.
    photo: bool,
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
            photo: true,
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
    #[cfg(test)]
    pub fn values(&self) -> &Values {
        &self.values
    }

    /// Draws the section in `ui`, open or closed as `open` says, for a photo that is open
    /// or not (`photo`). Returns what it asked of the app: the header pressed, the settings
    /// changed, and Export.
    pub fn show(&mut self, ui: &mut Ui, open: bool, photo: bool) -> Drawn {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let before = self.values.clone();
        self.photo = photo;
        let title = tr(&ctx, "section-export");
        let reset = tr(&ctx, "export-reset");
        let mut pressed = false;
        // A utility: its reset applies Default settings, before its options, which hold
        // nothing yet.
        let out = Section::new(&title, open)
            .utility_reset(&reset)
            .reset(self.values != Values::default())
            .options(false)
            .show(ui, |ui| pressed = self.rows(ui, &t));
        if out.actions.contains(&SectionAction::Reset) {
            self.values = Values::default();
            ctx.request_repaint();
        }
        // Overwriting asks first, as darktable's storage does whether or not a file is
        // there: the app sends the export on a yes, or at once where the setting says not
        // to ask.
        let confirm = pressed && asks_before(self.values.on_conflict);
        let values = (self.values != before).then(|| self.values.clone());
        Drawn {
            toggle: out.actions.contains(&SectionAction::Toggle),
            asked: Asked {
                values,
                start: pressed && !confirm,
                confirm,
            },
        }
    }

    /// The section's rows, then the Export button. Returns whether Export was pressed.
    fn rows(&mut self, ui: &mut Ui, t: &Theme) -> bool {
        let ctx = ui.ctx().clone();
        let later = tr(&ctx, "export-later");
        // Rows stack as a tool's do, each carrying its own gaps (docs/gui.md, How the rows
        // are laid out).
        ui.spacing_mut().item_spacing = vec2(t.spacing.s, 0.0);

        // Storage options.
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
        self.conflict_lines(ui, t);

        // Format options.
        group_label(ui, &tr(&ctx, "export-format-options"));
        self.format_rows(ui);

        // Global options.
        group_label(ui, &tr(&ctx, "export-global-options"));
        self.size_rows(ui, t);
        // Toggles, as a tool draws a boolean, where darktable's are No and Yes (D15).
        let label = tr(&ctx, "export-upscale");
        if let Some(on) = ToggleRow::new(&label, self.values.upscale).show(ui) {
            self.values.upscale = on;
        }
        let label = tr(&ctx, "export-high-quality");
        let promise = tr(&ctx, "export-high-quality-tip");
        if let Some(on) = ToggleRow::new(&label, self.values.high_quality)
            .promise(&promise)
            .show(ui)
        {
            self.values.high_quality = on;
        }
        let label = tr(&ctx, "export-masks");
        let _ = ToggleRow::new(&label, false)
            .enabled(false)
            .promise(&tr(&ctx, "export-masks-later"))
            .show(ui);
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

        // Groups to come, each 4 points under the row before it, as in a tool.
        for id in ["export-metadata", "export-multi-preset"] {
            let title = tr(&ctx, id);
            CollapsibleGroup::new(&title, false)
                .disabled(&later)
                .show(ui, |_| {});
        }
        ui.add_space(t.spacing.m);
        self.button(ui, t)
    }

    /// The path and, under it, what the session says of it: the file's name and where it
    /// goes, the variables with no value here, and the problems that stop Export (D8).
    fn path_rows(&mut self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let label = tr(&ctx, "export-path");
        // The label on a line of its own, a row high, and the field on the next, as no row
        // of a tool's holds a text field.
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            ui.label(RichText::new(&label).size(t.text.body));
        });
        let listed: Vec<Variable> = variables::list()
            .iter()
            .map(|v| Variable {
                insert: v.insert(),
                description: v.description(),
            })
            .collect();
        let problem = self.path_problem(&ctx);
        let choose = tr(&ctx, "export-choose-folder");
        let width = ui.available_width();
        let mut output = None;
        ui.horizontal(|ui| {
            ui.set_min_height(t.sizes.panel_line);
            let button = ui.fonts_mut(|f| {
                f.layout_no_wrap(
                    choose.clone(),
                    egui::FontId::proportional(t.text.body),
                    t.colors.text,
                )
                .size()
                .x
            }) + 2.0 * t.spacing.m;
            let field = (width - button - t.spacing.s).max(80.0);
            output = Some(
                PathField::new(
                    egui::Id::unique("export path"),
                    &label,
                    &self.values.path,
                    &listed,
                )
                .width(field)
                .error(problem.is_some())
                .show(ui),
            );
            let later = tr(&ctx, "export-later");
            Button::new(&choose).enabled(false).tip(&later).show(ui);
        });
        let output = output.expect("the path field draws");
        if let Some(text) = output.text {
            self.values.path = text;
        }
        let Some((_, preview)) = &self.preview else {
            return;
        };
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
            return;
        }
        // The first photo's name and where its file goes, the others summed up.
        if let Some(first) = preview.names.first() {
            let source = first
                .source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let line = match &first.fate {
                Fate::File { file, .. } => {
                    let mut args = Args::new();
                    args.set("source", source);
                    args.set("file", shown(file, self.home.as_deref()));
                    translator(&ctx).text_with("export-name", Some(&args))
                }
                Fate::NoName => tr(&ctx, "export-no-name"),
            };
            note(ui, t, line, t.colors.text_secondary);
            if preview.names.len() > 1 {
                let mut args = Args::new();
                args.set("count", (preview.names.len() - 1) as f64);
                let more = translator(&ctx).text_with("export-names-more", Some(&args));
                note(ui, t, more, t.colors.text_secondary);
            }
        }
        if preview.sequence_added {
            let mut args = Args::new();
            args.set("count", preview.names.len() as f64);
            let line = translator(&ctx).text_with("export-sequence-added", Some(&args));
            note(ui, t, line, t.colors.text_secondary);
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
            let line = translator(&ctx).text_with("export-not-here", Some(&args));
            note(ui, t, line, t.colors.text_secondary);
        }
    }

    /// What stops the path or its folder, in words, with the fix the session suggests
    /// (D8): none while all is well, and none until the session has answered.
    fn path_problem(&self, ctx: &egui::Context) -> Option<Problem> {
        let (_, preview) = self.preview.as_ref()?;
        problem_of(ctx, preview, &self.values.path, self.home.as_deref())
    }

    /// The line under On conflict, saying what happens to the files already there.
    fn conflict_lines(&self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let Some((_, preview)) = &self.preview else {
            return;
        };
        let counts = preview.counts();
        if counts.exist == 0 || self.path_problem(&ctx).is_some() {
            return;
        }
        let total = preview.names.len();
        let mut args = Args::new();
        args.set("total", total as f64);
        args.set("exist", counts.exist as f64);
        if total == 1 {
            let Some(first) = preview.names.first() else {
                return;
            };
            let Fate::File { file, action, .. } = &first.fate else {
                return;
            };
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            args.set("file", name);
            let (id, colour) = match action {
                Action::Write => return,
                Action::Number => ("export-conflict-one-number", t.colors.text_secondary),
                Action::Overwrite => ("export-conflict-one-overwrite", t.colors.warning),
                Action::Skip(apertrust_export::disk::Why::Exists) => {
                    ("export-conflict-one-skip", t.colors.text_secondary)
                }
                Action::Skip(apertrust_export::disk::Why::NotChanged) => {
                    ("export-conflict-one-unchanged", t.colors.text_secondary)
                }
            };
            note(ui, t, translator(&ctx).text_with(id, Some(&args)), colour);
            return;
        }
        for (count, id, colour) in [
            (
                counts.numbered,
                "export-conflict-many-number",
                t.colors.text_secondary,
            ),
            (
                counts.overwritten,
                "export-conflict-many-overwrite",
                t.colors.warning,
            ),
            (
                counts.skipped,
                "export-conflict-many-skip",
                t.colors.text_secondary,
            ),
        ] {
            if count > 0 {
                args.set("count", count as f64);
                note(ui, t, translator(&ctx).text_with(id, Some(&args)), colour);
            }
        }
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
        if let Some(ChoiceAction::Set(i)) = choice(&label, &names, current)
            .disabled_options(&unbuilt, &later)
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
                if let Some(value) = whole_slider(ui, &label, quality, 5.0..=100.0, 95.0) {
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
                if let Some(value) = whole_slider(ui, &label, compression, 0.0..=9.0, 5.0) {
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
        if let Some(ChoiceAction::Set(i)) = choice(&label, &modes, current).show(ui) {
            self.values.size_mode = SizeMode::OPTIONS[i].0;
        }
        let v = &mut self.values;
        match v.size_mode {
            SizeMode::Pixels => {
                let unit = tr(&ctx, "unit-pixels");
                let name = tr(&ctx, "export-size-pixels");
                let shown = [v.width, v.height].map(f64::from);
                let [w, h] = size_fields(ui, t, &name, shown, 0, 100_000.0, &unit);
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
                let typed = size_fields(ui, t, &name, lengths, 2, 10_000.0, &unit);
                let mut pixels = [v.width, v.height];
                for (i, typed) in typed.into_iter().enumerate() {
                    if let Some(length) = typed {
                        lengths[i] = length;
                        pixels[i] = pixels_of(length, cm, dpi);
                    }
                }
                // The resolution.
                let name = tr(&ctx, "export-dpi");
                let unit = tr(&ctx, "export-dpi-unit");
                let mut new = None;
                end_row(ui, t, &name, field_inset(t), |ui| {
                    let field = ValueField::new(
                        egui::Id::unique("export dpi"),
                        &name,
                        f64::from(dpi),
                        72.0..=9600.0,
                    )
                    .unit(&unit)
                    .number_only();
                    let room = field.width(ui);
                    ui.allocate_ui(vec2(room, t.sizes.field), |ui| {
                        if let (_, Some(value)) = field.show(ui) {
                            new = Some(value.round() as u32);
                        }
                    });
                    ui.label(
                        RichText::new(unit.clone())
                            .size(t.text.small)
                            .color(t.colors.text_secondary),
                    );
                });
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
                let mut args = Args::new();
                args.set("width", v.width.to_string());
                args.set("height", v.height.to_string());
                let line = translator(&ctx).text_with("export-equal-to", Some(&args));
                note(ui, t, line, t.colors.text_secondary);
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
                note(ui, t, tr(&ctx, "export-scale-tip"), t.colors.text_secondary);
            }
        }
        self.size_line(ui, t);
    }

    /// What the active photo will be: its size and whether that is its full size, or the
    /// problem that stops Export.
    fn size_line(&self, ui: &mut Ui, t: &Theme) {
        let ctx = ui.ctx().clone();
        let Some((_, preview)) = &self.preview else {
            return;
        };
        let Some(active) = &preview.size else {
            return;
        };
        let file = active
            .source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut args = Args::new();
        args.set("file", file);
        match &active.size {
            Ok(size) => {
                args.set("width", size.width.to_string());
                args.set("height", size.height.to_string());
                let v = &self.values;
                let full = size.scale == 1.0;
                let limited = v.size_mode != SizeMode::Scale && (v.width > 0 || v.height > 0);
                let bigger = (v.width > size.extent[0]) || (v.height > size.extent[1]);
                let id = if !full {
                    "export-size"
                } else if !v.upscale
                    && (bigger
                        || v.size_mode == SizeMode::Scale && size::scale_factor(&v.scale) > 1.0)
                {
                    "export-size-capped"
                } else if v.size_mode != SizeMode::Scale && !limited {
                    "export-size-no-limit"
                } else {
                    "export-size-full"
                };
                let line = translator(&ctx).text_with(id, Some(&args));
                note(ui, t, line, t.colors.text_secondary);
            }
            Err(problem) => note(ui, t, size_problem(&ctx, problem), t.colors.error),
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

    /// The Export button, primary, the section's full width, and what the tool says of
    /// exporting under it. Disabled with its reason as its tooltip where Export cannot run.
    /// Returns whether it was pressed.
    fn button(&mut self, ui: &mut Ui, t: &Theme) -> bool {
        let ctx = ui.ctx().clone();
        let reason = self.cannot_export(&ctx);
        let mut args = Args::new();
        args.set("count", PHOTOS as f64);
        let words = translator(&ctx).text_with("export-button", Some(&args));
        let mut button = Button::new(&words)
            .primary(true)
            .enabled(reason.is_none())
            .width(ui.available_width());
        if let Some(why) = &reason {
            button = button.tip(why);
        }
        let pressed = button.show(ui).clicked();
        ui.add_space(t.spacing.s);
        note(ui, t, tr(&ctx, "export-hint"), t.colors.text_secondary);
        pressed
    }

    /// Why Export cannot run, in words, if it cannot: the session has found a problem with
    /// the path, its folder or the size (D8). Until the session answers there is none to find;
    /// it checks again as the job starts.
    fn cannot_export(&self, ctx: &egui::Context) -> Option<String> {
        let say = |id: &str, problem: &str| {
            let mut args = Args::new();
            args.set("problem", problem);
            translator(ctx).text_with(id, Some(&args))
        };
        if !self.photo {
            return Some(tr(ctx, "export-why-no-photo"));
        }
        if let Some(problem) = self.path_problem(ctx) {
            return Some(say("export-why-path", &problem.text));
        }
        // Overwrite kept from a platform that asks: no question here, so no export.
        if !self.alerts && asks_before(self.values.on_conflict) {
            return Some(tr(ctx, "export-conflict-overwrite-no-alert"));
        }
        let (_, preview) = self.preview.as_ref()?;
        match &preview.size.as_ref()?.size {
            Ok(_) => None,
            Err(problem) => Some(say("export-why-size", &size_problem(ctx, problem))),
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

/// A note under a row, in the small text.
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

/// The width and height fields of a size row, `name` its label, `unit` after them. They
/// show `width` and `height` with `decimals`. Returns what the user typed in each.
fn size_fields(
    ui: &mut Ui,
    t: &Theme,
    name: &str,
    [width, height]: [f64; 2],
    decimals: usize,
    reach: f64,
    unit: &str,
) -> [Option<f64>; 2] {
    let ctx = ui.ctx().clone();
    let mut changed = [None; 2];
    end_row(ui, t, name, field_inset(t), |ui| {
        for (i, (value, side)) in [(width, "export-width"), (height, "export-height")]
            .into_iter()
            .enumerate()
        {
            if i == 1 {
                ui.label(
                    RichText::new("×")
                        .size(t.text.small)
                        .color(t.colors.text_secondary),
                );
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
            let room = field.width(ui);
            ui.allocate_ui(vec2(room, t.sizes.field), |ui| {
                if let (_, Some(typed)) = field.show(ui) {
                    changed[i] = Some(typed);
                }
            });
        }
        ui.label(
            RichText::new(unit)
                .size(t.text.small)
                .color(t.colors.text_secondary),
        );
    });
    changed
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
