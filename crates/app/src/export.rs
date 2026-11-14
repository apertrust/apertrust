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

//! The Export dialog's content (`docs/gui.md`, The Export dialog; decision 56): darktable's
//! export module row for row, in a sheet on the Mac. It is a function of `&mut Ui`
//! (decision 41), so the dialog host draws it in the sheet's window and the interaction
//! tests draw it in a harness.
//!
//! **Its rows** are File on disk with its path, On conflict, JPEG and PNG, Set size's four
//! modes, Allow upscaling, High quality resampling, and Profile with Image settings and
//! sRGB. Every other row is drawn, disabled, saying it is to come: the other storages and
//! formats, Store masks, Intent, Style, Metadata and Multi-preset export. The Presets column holds Default settings alone.
//!
//! **The settings** are the `export.*` settings, one [`Values`], read from the app's store
//! when the dialog opens and offered back to it as they change (`Asked::values`), so Close
//! keeps them as left, as darktable keeps its conf. The engine reads no settings
//! (decision 43): [`Values::settings`] turns them into the plain [`ExportSettings`] the
//! session's `Export` and `PreviewExport` take.
//!
//! **The feedback under the rows** comes from the session (`PreviewExport`,
//! `Event::ExportPreview`): the app asks for a preview whenever the settings change, and
//! hands the answer back with [`Export::receive`], which keeps the one with the highest
//! `through`. The dialog exports the active photo, since nothing selects photos yet.

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
    Button, Choice, ChoiceAction, ChoiceStyle, CollapsibleGroup, PathField, SliderAction,
    SliderRow, TextField, ValueField, Variable, group_label, quiet_scroll,
};
use apertrust_ui::i18n::Args;
use apertrust_ui::{Theme, theme, tr, translator};
use eframe::egui::os::OperatingSystem;
use eframe::egui::{
    self, Color32, Key, Label, Modifiers, Rect, RichText, ScrollArea, Sense, Ui, UiBuilder, vec2,
};

use crate::dialog_host::{self, Asks, Dialog, Form, Keys, Look, Output};

/// The dialog's width, in points (`docs/gui.md`, The Export dialog, Its form).
pub const WIDTH: f32 = 640.0;
/// The Presets column's width.
const PRESETS: f32 = 200.0;
/// The header's height, with the hairline under it.
const HEADER: f32 = 44.0;
/// The two columns' height; the settings column scrolls inside it.
const BODY: f32 = 440.0;
/// The footer's height.
const FOOTER: f32 = 52.0;
/// How many photos the dialog exports: the active one, until a selection comes.
const PHOTOS: usize = 1;

/// The export settings as the dialog edits them: one field for each `export.*` setting the
/// dialog's rows show.
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
    /// format with its options and the size as the dialog asks for it
    /// (`Request::dialog`, whose rule for a side left at 0 applies). A JPEG's density is
    /// the print dpi in the cm and inch modes, as darktable's `metadata/resolution` is.
    ///
    /// The profile is not among them yet: both of the dialog's choices export the Output
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

/// Whether an export with `on_conflict` asks the overwrite question first, by the dialog's
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

/// What the app sends the session for what the dialog asked of it, for the photo open at
/// `photo`: the preview of the settings that changed, then, once Export is pressed, the
/// export of the settings the app holds, `held`, which are the dialog's. The photo's edit
/// is the session's to take as the export command is handled (D12).
pub fn commands(photo: &Path, asked: &Asked, held: &Values, places: &Places) -> Vec<Command> {
    let photos = vec![photo.to_owned()];
    let mut commands = Vec::new();
    if let Some(values) = &asked.values {
        commands.push(Command::PreviewExport {
            photos: photos.clone(),
            settings: values.settings(),
            places: places.clone(),
        });
    }
    if asked.start {
        commands.push(Command::Export {
            photos,
            settings: held.settings(),
            places: places.clone(),
        });
    }
    commands
}

/// What a frame of the dialog asked of the app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// The settings, whenever they changed and on the first frame: the app keeps them and
    /// asks the session for a new preview.
    pub values: Option<Values>,
    /// Export was pressed: the app sends the session the export of the settings it holds.
    pub start: bool,
    /// Export was pressed with On conflict Overwrite ([`asks_before`]): the dialog stays
    /// open and the app asks first (`alerts::overwrite_request`), sending the export, and
    /// closing the dialog, only on a yes, or at once where the setting says not to ask.
    /// Never together with `start`.
    pub confirm: bool,
    /// This is the dialog's first frame: the preview the app asks for is the first of its
    /// opening, and no answer to an earlier one is the dialog's.
    pub opened: bool,
}

/// The dialog's state while it is open.
#[derive(Debug)]
pub struct Export {
    values: Values,
    /// Whether the first frame has said what the settings are, for the first preview.
    started: bool,
    /// The newest answer of the session, with the number of the command it answers.
    preview: Option<(u64, ExportPreview)>,
    /// The person's home, which paths are shown relative to as `~`.
    home: Option<PathBuf>,
    /// The three regions F6 moves between, as the last frame drew them: the presets, the
    /// settings and the footer.
    regions: [Rect; 3],
    /// The print lengths last entered, which a change of resolution keeps ([`Print`]).
    print: Option<Print>,
    /// Whether the host can ask the overwrite question here
    /// (`dialog_host::platform::shows_alerts`): where it cannot, On conflict's Overwrite
    /// is listed disabled, and Export rests while it is chosen.
    alerts: bool,
}

/// What the cm and inch fields hold, as darktable's print entries do, with the pixels they
/// gave: while the pixels are still those, the lengths are the physical size.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Print {
    mode: SizeMode,
    lengths: [f64; 2],
    pixels: [u32; 2],
}

/// Export in the dialog host (decision 41; `docs/gui.md`, The Export dialog, Its form): a
/// sheet, AppKit's on the Mac and a modal window elsewhere, on the panel's colour and
/// drawing its own margins; Esc, Cmd+. and Cmd+W close it, as its Close.
impl Dialog for Export {
    fn id(&self) -> &'static str {
        "export"
    }

    /// "Export 1 photo", which its window's frame shows, and the Mac's sheet, which has
    /// none, as its own heading.
    fn title(&self, ctx: &egui::Context) -> String {
        let mut args = Args::new();
        args.set("count", PHOTOS as f64);
        translator(ctx).text_with("export-title", Some(&args))
    }

    fn form(&self, _os: OperatingSystem) -> Form {
        Form::Sheet
    }

    fn look(&self, t: &Theme) -> Look {
        Look {
            background: t.colors.panel,
            margin: 0.0,
        }
    }

    fn keys(&self, _os: OperatingSystem) -> Keys {
        Keys {
            escape: true,
            close_window: true,
            cancel: true,
        }
    }

    fn show(&mut self, ui: &mut Ui) -> Asks {
        let (asked, close, keeps_escape) = self.frame(ui);
        let outputs = (asked.values.is_some() || asked.start || asked.confirm)
            .then_some(Output::Export(asked))
            .into_iter()
            .collect();
        Asks {
            outputs,
            close,
            keeps_escape,
        }
    }
}

impl Export {
    /// The dialog over `values`, with paths shown relative to the person's home.
    pub fn new(values: Values) -> Export {
        Export {
            values,
            started: false,
            preview: None,
            home: places().home,
            regions: [Rect::NOTHING; 3],
            print: None,
            alerts: true,
        }
    }

    /// The dialog where the host can, or cannot, ask an alert.
    pub fn alerts(mut self, alerts: bool) -> Export {
        self.alerts = alerts;
        self
    }

    /// The dialog with a home of its own, for tests.
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

    /// The settings as they stand.
    #[cfg(test)]
    pub fn values(&self) -> &Values {
        &self.values
    }

    /// Draws the dialog in `ui`: its header in the Mac's sheet, the two columns, the footer,
    /// and its keys but the host's. Returns what it asked of the app, whether it is to
    /// close, and whether the path's variable list was open as the frame began, which
    /// Escape closes first.
    fn frame(&mut self, ui: &mut Ui) -> (Asked, bool, bool) {
        let ctx = ui.ctx().clone();
        let t = theme(&ctx);
        let before = self.values.clone();
        let typing = dialog_host::typing(&ctx);
        let focused = ctx.memory(|m| m.focused());
        ui.set_width(WIDTH);
        // The regions sit edge to edge; each column sets the spacing of its own rows.
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        // The title is its window's frame's, the system's or the host's on Linux; a sheet
        // has none, so it shows it here.
        let os = ctx.os();
        if dialog_host::platform::form(self.form(os), os) == Form::Sheet {
            self.header(ui, &t);
        }
        let mut regions = [Rect::NOTHING; 3];
        let mut list_was_open = false;
        ui.horizontal_top(|ui| {
            regions[0] = self.presets(ui, &t);
            let (line, _) = ui.allocate_exact_size(vec2(1.0, BODY), Sense::hover());
            ui.painter().rect_filled(line, 0.0, t.colors.separator);
            let width = WIDTH - PRESETS - 1.0;
            let area = ui.allocate_ui_with_layout(vec2(width, BODY), top_down(), |ui| {
                ui.set_min_size(vec2(width, BODY));
                quiet_scroll(
                    ui,
                    ScrollArea::vertical()
                        .id_salt("export settings")
                        .max_height(BODY)
                        .auto_shrink([false, false]),
                    |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(t.spacing.xl as i8, 0))
                            .show(ui, |ui| list_was_open = self.settings_column(ui, &t));
                    },
                );
            });
            regions[1] = area.response.rect;
        });
        let (close_pressed, export_pressed, footer) = self.footer(ui, &t);
        regions[2] = footer;
        self.regions = regions;

        let mut close = close_pressed;
        let mut start = export_pressed;
        // The keys (docs/gui.md, The Export dialog, Keyboard): Esc, Cmd+. and Cmd+W are the
        // host's ([`Dialog::keys`]), Esc leaving a list or a field being typed in to them;
        // Return exports unless a control has the focus, which takes it itself, and
        // Cmd+Return from anywhere; F6 moves between the three regions.
        let plain = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let command = ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
        start |= (command || plain && focused.is_none() && !typing)
            && self.cannot_export(&ctx).is_none();
        // Shift first: a key with no modifier asked for matches the shifted one too.
        let back = ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::F6));
        let forward = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F6));
        if forward || back {
            let widgets = crate::focusable(&ctx);
            let focused = ctx.memory(|m| m.focused());
            if let Some(id) = crate::actions::next_region(&self.regions, &widgets, focused, forward)
            {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        }
        // Overwriting asks first, as darktable's storage does whether or not a file is
        // there: the dialog stays, and the app sends the export on a yes, or at once where
        // the setting says not to ask.
        let confirm = start && asks_before(self.values.on_conflict);
        if confirm {
            start = false;
        }
        if start {
            close = true;
        }
        let first = !std::mem::replace(&mut self.started, true);
        let values = (first || self.values != before).then(|| self.values.clone());
        let asked = Asked {
            values,
            start,
            confirm,
            opened: first,
        };
        (asked, close, list_was_open)
    }

    /// The header: the title, 16 points Medium in a 44-point band with a hairline under
    /// it, drawn only in the Mac's sheet, which has no title bar to show it.
    fn header(&self, ui: &mut Ui, t: &Theme) {
        let (rect, _) = ui.allocate_exact_size(vec2(WIDTH, HEADER), Sense::hover());
        let title = self.title(ui.ctx());
        let mut band = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(t.spacing.xl, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        band.heading(title);
        let line = Rect::from_min_size(
            egui::pos2(rect.left(), rect.bottom() - 1.0),
            vec2(WIDTH, 1.0),
        );
        ui.painter().rect_filled(line, 0.0, t.colors.separator);
    }

    /// The Presets column: Default settings alone, built in, ticked while the settings are
    /// the defaults, else a line saying they match no preset. Choosing it resets them, as
    /// darktable's reset does. Returns the column's rectangle.
    fn presets(&mut self, ui: &mut Ui, t: &Theme) -> Rect {
        let ctx = ui.ctx().clone();
        let out = ui.allocate_ui_with_layout(vec2(PRESETS, BODY), top_down(), |ui| {
            ui.set_min_size(vec2(PRESETS, BODY));
            egui::Frame::NONE
                .inner_margin(egui::Margin::same(t.spacing.m as i8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(t.spacing.s, t.spacing.xs);
                    group_label(ui, &tr(&ctx, "export-presets"));
                    let matches = self.values == Values::default();
                    let name = tr(&ctx, "export-preset-default");
                    let tip = tr(&ctx, "export-preset-default-tip");
                    let words = if matches {
                        format!("✓  {name}")
                    } else {
                        format!("    {name}")
                    };
                    let row = ui.add(
                        egui::Button::new(RichText::new(words).size(t.text.body))
                            .frame(false)
                            .selected(matches)
                            .min_size(vec2(ui.available_width(), t.sizes.row)),
                    );
                    let row = row.on_hover_text(tip);
                    ui.ctx().accesskit_node_builder(row.id, |node| {
                        node.set_label(name);
                        node.set_selected(matches);
                    });
                    if row.clicked() {
                        self.values = Values::default();
                    }
                    note(
                        ui,
                        t,
                        tr(&ctx, "export-preset-built-in"),
                        t.colors.text_secondary,
                    );
                    if !matches {
                        note(
                            ui,
                            t,
                            tr(&ctx, "export-preset-matches-none"),
                            t.colors.text_secondary,
                        );
                    }
                    ui.add_space(t.spacing.m);
                    note(
                        ui,
                        t,
                        tr(&ctx, "export-presets-later"),
                        t.colors.text_secondary,
                    );
                });
        });
        out.response.rect
    }

    /// The settings column's rows. Returns whether the path's variable list was open as
    /// the frame began, for Escape to close it and nothing else.
    fn settings_column(&mut self, ui: &mut Ui, t: &Theme) -> bool {
        let ctx = ui.ctx().clone();
        let later = tr(&ctx, "export-later");
        ui.spacing_mut().item_spacing = vec2(t.spacing.s, t.spacing.xs);
        let mut list_was_open = false;

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
        let _ = Choice::new(&label, &storages, 1)
            .style(ChoiceStyle::Boxed)
            .disabled_options(&[0, 2, 3], &later)
            .show(ui);

        // File on disk.
        list_was_open |= self.path_rows(ui, t);
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
        if let Some(ChoiceAction::Set(i)) = Choice::new(&label, &conflicts, current)
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
        let no_yes = [tr(&ctx, "export-no"), tr(&ctx, "export-yes")];
        let no_yes: Vec<&str> = no_yes.iter().map(String::as_str).collect();
        let label = tr(&ctx, "export-upscale");
        if let Some(ChoiceAction::Set(i)) =
            Choice::new(&label, &no_yes, usize::from(self.values.upscale)).show(ui)
        {
            self.values.upscale = i == 1;
        }
        let label = tr(&ctx, "export-high-quality");
        let promise = tr(&ctx, "export-high-quality-tip");
        if let Some(ChoiceAction::Set(i)) =
            Choice::new(&label, &no_yes, usize::from(self.values.high_quality))
                .promise(&promise)
                .show(ui)
        {
            self.values.high_quality = i == 1;
        }
        let label = tr(&ctx, "export-masks");
        let _ = Choice::new(&label, &no_yes, 0)
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
        let _ = Choice::new(&label, &intents, 0)
            .enabled(false)
            .promise(&tr(&ctx, "export-intent-tip"))
            .show(ui);
        let label = tr(&ctx, "export-style");
        let none = [tr(&ctx, "export-style-none")];
        let none: Vec<&str> = none.iter().map(String::as_str).collect();
        let _ = Choice::new(&label, &none, 0)
            .enabled(false)
            .promise(&tr(&ctx, "export-style-later"))
            .show(ui);

        // Groups to come.
        ui.add_space(t.spacing.m);
        for id in ["export-metadata", "export-multi-preset"] {
            let title = tr(&ctx, id);
            CollapsibleGroup::new(&title, false)
                .disabled(&later)
                .show(ui, |_| {});
        }
        ui.add_space(t.spacing.m);
        list_was_open
    }

    /// The path and, under it, what the session says of it: the file's name and where it
    /// goes, the variables with no value here, and the problems that stop Export (D8).
    /// Returns whether the variable list was open as the frame began.
    fn path_rows(&mut self, ui: &mut Ui, t: &Theme) -> bool {
        let ctx = ui.ctx().clone();
        let label = tr(&ctx, "export-path");
        ui.label(RichText::new(&label).size(t.text.body));
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
            let button = ui.fonts_mut(|f| {
                f.layout_no_wrap(
                    choose.clone(),
                    egui::FontId::proportional(t.text.body),
                    t.colors.text,
                )
                .size()
                .x
            }) + 2.0 * t.spacing.m;
            let field = (width - button - t.spacing.s).max(120.0);
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
            return output.list_open;
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
            return output.list_open;
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
        output.list_open
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
        if let Some(ChoiceAction::Set(i)) = Choice::new(&label, &names, current)
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
                if let Some(ChoiceAction::Set(i)) = Choice::new(&label, &options, current)
                    .promise(&promise)
                    .show(ui)
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
                if let Some(ChoiceAction::Set(i)) = Choice::new(&label, &options, current).show(ui)
                {
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
        if let Some(ChoiceAction::Set(i)) = Choice::new(&label, &modes, current).show(ui) {
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
                end_row(ui, t, &name, |ui| {
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
                end_row(ui, t, &name, |ui| {
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
        if let Some(ChoiceAction::Set(i)) = Choice::new(&label, &options, current)
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

    /// The footer: what the dialog says of itself, Close, and Export, accented, last, as
    /// the Mac has the default button. Returns whether each was pressed and the footer's
    /// rectangle.
    fn footer(&mut self, ui: &mut Ui, t: &Theme) -> (bool, bool, Rect) {
        let ctx = ui.ctx().clone();
        let (rect, _) = ui.allocate_exact_size(vec2(WIDTH, FOOTER), Sense::hover());
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(WIDTH, 1.0)),
            0.0,
            t.colors.separator,
        );
        let mut close = false;
        let mut export = false;
        let reason = self.cannot_export(&ctx);
        let mut row = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink2(vec2(t.spacing.xl, 0.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let close_words = tr(&ctx, "export-close");
        let mut args = Args::new();
        args.set("count", PHOTOS as f64);
        let export_words = translator(&ctx).text_with("export-button", Some(&args));
        let id = egui::Id::unique("export footer buttons");
        let width: Option<f32> = row.data(|d| d.get_temp(id));
        // What the dialog says of itself takes the room the buttons leave, wrapping where
        // a language needs more; it is no stop for F6 or Tab, so the footer's first is Close.
        let room = (row.available_width() - width.unwrap_or(0.0) - t.spacing.l).max(0.0);
        row.allocate_ui_with_layout(
            vec2(room, FOOTER),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add(
                    Label::new(
                        RichText::new(tr(&ctx, "export-hint"))
                            .size(t.text.small)
                            .color(t.colors.text_secondary),
                    )
                    .selectable(false)
                    .wrap(),
                );
            },
        );
        row.add_space((row.available_width() - width.unwrap_or(0.0)).max(0.0));
        let start = row.cursor().min.x;
        close = Button::new(&close_words).show(&mut row).clicked() || close;
        let mut button = Button::new(&export_words)
            .primary(true)
            .enabled(reason.is_none());
        if let Some(why) = &reason {
            button = button.tip(why);
        }
        export |= button.show(&mut row).clicked();
        let measured = row.min_rect().max.x - start;
        if width != Some(measured) {
            row.data_mut(|d| d.insert_temp(id, measured));
            ctx.request_discard("the export footer measured");
        }
        (close, export, rect)
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

/// Rows top to bottom from the column's start, in a parent that lays out sideways.
fn top_down() -> egui::Layout {
    egui::Layout::top_down(egui::Align::Min)
}

/// A note under a row, in the small text.
fn note(ui: &mut Ui, t: &Theme, text: impl Into<String>, colour: Color32) {
    ui.add(Label::new(RichText::new(text.into()).size(t.text.small).color(colour)).wrap());
}

/// A row of the dialog: `label` at its start, the fields `add` draws at its end. The
/// fields are added from the row's start, in reading order for readers and Tab, after a
/// space as wide as the last pass found them, so a pass that finds them changed is drawn
/// again.
fn end_row(ui: &mut Ui, t: &Theme, label: &str, add: impl FnOnce(&mut Ui)) {
    let id = ui.make_persistent_id(("export row", label));
    let width: Option<f32> = ui.data(|d| d.get_temp(id));
    ui.horizontal(|ui| {
        ui.set_min_height(t.sizes.field);
        ui.label(RichText::new(label).size(t.text.body));
        ui.add_space((ui.available_width() - width.unwrap_or(0.0)).max(0.0));
        let start = ui.cursor().min.x;
        add(ui);
        let measured = ui.min_rect().max.x - start;
        if width != Some(measured) {
            ui.data_mut(|d| d.insert_temp(id, measured));
            ui.ctx().request_discard("an export row measured");
        }
    });
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
    end_row(ui, t, name, |ui| {
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

/// darktable's twelve export profiles in its order, by the Output colour profile module's
/// messages for them.
const PROFILES: [&str; 12] = [
    "module-colorout-control-type-option-srgb",
    "module-colorout-control-type-option-adobe_rgb",
    "module-colorout-control-type-option-lin_rec709",
    "module-colorout-control-type-option-rec709",
    "module-colorout-control-type-option-lin_rec2020",
    "module-colorout-control-type-option-pq_rec2020",
    "module-colorout-control-type-option-hlg_rec2020",
    "module-colorout-control-type-option-pq_p3",
    "module-colorout-control-type-option-hlg_p3",
    "module-colorout-control-type-option-display_p3",
    "module-colorout-control-type-option-prophoto_rgb",
    "module-colorout-control-type-option-brg",
];
