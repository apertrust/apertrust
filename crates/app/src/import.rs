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

//! The Import sheet (decisions 85 and 86; `docs/gui.md`, The Import sheet): its content on
//! the design system's sheet frame, in Import's first slice, 20a1. The Import tool in its
//! column and the ways in are the next slice's (#754); a test opens the sheet here.
//!
//! **The listing's mirror** ([`Listing`]), plain data a test feeds events and reads back:
//! the sheet sends `ListSource` for its source with Folder's and the default folder's
//! patterns and the person's places, and keeps the events of that listing's number alone,
//! as the Browser keeps its latest open's: the files as their `SourceListed` chunks come,
//! their capture times and cameras with `SourceTaken`, and whether each was imported before
//! with `SourceChecked`, the latest check's alone. As Folder or the default folder changes
//! it sends `CheckSource` for the listing shown, whose check the session numbers one past
//! the last, the listing's own the first. Closing the sheet sends `CancelListing`.
//!
//! **The grid's order** is darktable's: by capture time ascending, a photo with none after
//! those with one, then by file name as the Browser orders names. The checks are the
//! design system's [`Checks`], by the photos' places in that order, carried over as it
//! changes.
//!
//! **At opening,** once the listing and its check end, the photos not imported before are
//! checked, the first of them the sheet's active photo, which the grid scrolls to; with
//! none checked, the first photo listed is active. Screen readers hear it politely.
//!
//! **The cells** are the sheet's: the camera's embedded picture, asked for with
//! `SourceThumbnails` only for the cells in sight, once each while they stay in sight, the
//! file name in a box until it comes and where the raw holds none, a cloud file not on the
//! disk never asked for (decision 53); the check box and the name; the tooltip with the
//! name, the capture time, the file's modified time, which may differ from it (darktable's
//! column), the size and the camera, and where a photo imported before is. A photo imported
//! before carries Lucide's `circle-check`, its picture at half strength while unchecked.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use apertrust_engine::session::library::{
    DateTime, ImportPhotos, ImportPreview, ImportProblem, ImportSettings, OnSource, Problem, Row,
    SourceFile, SourceShot, SourceState, Thumbnail, Trees, Unwritable, browser_order,
};
use apertrust_engine::session::{Command, LibraryCommand, LibraryEvent};
use apertrust_export::variables::{MAX_DEPTH, PatternError};
use apertrust_ui::components::{
    CellMark, CellPicture, CheckGesture, Checks, GridMessage, SelectButton, Sheet, SheetCell,
    Summary, ThumbnailSize,
};
use apertrust_ui::i18n::{Args, Translator};
use apertrust_ui::{Icon, Locale, translator};
use eframe::egui;

use crate::SendCommand;
use crate::metadata::format::{naive, size};
use crate::sheet::{self, Place, Shown};

#[cfg(test)]
mod tests;

/// Where the sheet imports from, as Source chose it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The folder chosen: a card's root or any folder on it, or a folder.
    pub folder: PathBuf,
    /// Its name as the sheet says it: a card's volume name, a folder's last part.
    pub name: String,
    /// Whether it is on a camera card.
    pub card: bool,
}

/// The trees the check walks, as the settings hold them: `import.base_folder`,
/// `import.default_folder` and the person's places they expand from.
fn trees(settings: &ImportSettings) -> Trees {
    Trees {
        base_folder: settings.base_folder.clone(),
        default_folder: settings.default_folder.clone(),
        places: settings.places.clone(),
    }
}

/// One photo of the source, as the sheet mirrors it.
#[derive(Clone, Debug, PartialEq)]
pub struct Photo {
    pub file: SourceFile,
    /// Its file name, the last part of its path.
    pub name: String,
    /// When it was taken and by which camera, once `SourceTaken` has come.
    pub shot: Option<SourceShot>,
    /// Whether it was imported before, as the latest check has told it so far.
    pub state: Option<SourceState>,
    /// Its embedded picture, once its thumbnail has come: `None` inside where the raw holds
    /// none that can be read.
    pub thumbnail: Option<Option<Thumbnail>>,
}

impl Photo {
    /// Whether the latest check found it imported before, or probably so.
    pub fn imported_before(&self) -> bool {
        matches!(
            self.state,
            Some(SourceState::ImportedBefore { .. } | SourceState::ProbablyImported { .. })
        )
    }

    /// Whether the latest check found it new: Select new's and the opening's photos.
    pub fn is_new(&self) -> bool {
        self.state == Some(SourceState::New)
    }
}

/// The mirror of the source's listing and its check, with the checks.
#[derive(Debug, Default)]
pub struct Listing {
    /// The `ListSource`'s number, once sent.
    number: Option<u64>,
    /// The folder listed, the card's `DCIM` or the folder, as the first chunk names it.
    folder: Option<PathBuf>,
    /// The photos, in the listing's order.
    photos: Vec<Photo>,
    /// Each photo's place in `photos` by its path as text, which its thumbnail names.
    by_path: HashMap<String, usize>,
    /// The photo at each of the grid's places.
    order: Vec<usize>,
    /// The listing's last chunk has come, and its capture times.
    listed: bool,
    taken: bool,
    /// The folder listed, as Source chose it.
    source: PathBuf,
    /// Why the source could not be listed, in the library's plain words.
    failed: Option<String>,
    /// The source folder was gone as its listing failed: removed, as a card pulled out.
    gone: bool,
    /// The check whose `SourceChecked` are kept: the listing's own, 1, then one more for
    /// each `CheckSource`, as the session numbers them.
    check: u64,
    /// The check the states held are of.
    states_of: u64,
    /// How many photos the latest check has told, of how many.
    progress: (usize, usize),
    /// The latest check is done.
    checked: bool,
    /// The trees the latest check walked.
    folders: Vec<PathBuf>,
    checks: Checks,
    /// The opening's checks are set.
    opened: bool,
    /// The checks were changed by hand before the opening, which then leaves them.
    touched: bool,
    /// The photos whose thumbnails were asked for since they came into sight.
    asked: HashSet<usize>,
    /// Bumped as a cell's words change: the files listed, their times, their states.
    generation: u64,
    /// Bumped as a thumbnail comes.
    came: u64,
    /// Bumped as the photos checked change, which the preview follows.
    changed: u64,
}

/// A Select button of the sheet bar, acting on the cells shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Select {
    /// Exactly the photos not imported before, darktable's "select new".
    New,
    All,
    None,
}

impl Select {
    pub const ALL: [Select; 3] = [Select::New, Select::All, Select::None];

    /// Its id among the bar's buttons, and its words' message.
    pub fn id(self) -> &'static str {
        match self {
            Select::New => "new",
            Select::All => "all",
            Select::None => "none",
        }
    }

    fn words(self) -> &'static str {
        match self {
            Select::New => "import-select-new",
            Select::All => "import-select-all",
            Select::None => "import-select-none",
        }
    }
}

impl Listing {
    /// Lists `source`, in place of any listing before, checking it against `settings`'
    /// trees: `ListSource`, whose number it keeps.
    pub fn list(&mut self, source: &Path, settings: &ImportSettings, send: &mut SendCommand<'_>) {
        let Trees {
            base_folder,
            default_folder,
            places,
        } = trees(settings);
        let number = send(Command::Library(LibraryCommand::ListSource {
            source: source.to_owned(),
            include_subfolders: false,
            base_folder,
            default_folder,
            places,
        }));
        *self = Listing {
            number: Some(number),
            source: source.to_owned(),
            check: 1,
            generation: self.generation + 1,
            changed: self.changed + 1,
            ..Listing::default()
        };
    }

    /// Checks the listing again against `settings`' trees, as Folder changes:
    /// `CheckSource`, its check the next, whose events alone are kept from now on.
    pub fn recheck(&mut self, settings: &ImportSettings, send: &mut SendCommand<'_>) {
        let Some(listing) = self.number else {
            return;
        };
        let Trees {
            base_folder,
            default_folder,
            places,
        } = trees(settings);
        send(Command::Library(LibraryCommand::CheckSource {
            listing,
            base_folder,
            default_folder,
            places,
        }));
        self.check += 1;
        self.checked = false;
        self.progress = (0, self.photos.len());
    }

    /// What closing the sheet sends: `CancelListing` for the listing, if one was sent.
    pub fn cancel(&self) -> Option<Command> {
        let listing = self.number?;
        Some(Command::Library(LibraryCommand::CancelListing { listing }))
    }

    /// Takes in `event`, if it is the listing's. Returns whether it was.
    pub fn receive(&mut self, event: &LibraryEvent) -> bool {
        if self.number != Some(event.open()) {
            return false;
        }
        match event {
            LibraryEvent::SourceListed {
                folder,
                files,
                done,
                ..
            } => {
                self.folder.get_or_insert_with(|| folder.clone());
                for file in files {
                    let at = self.photos.len();
                    self.by_path
                        .insert(file.path.to_string_lossy().into_owned(), at);
                    self.photos.push(Photo {
                        name: file_name(&file.path),
                        file: file.clone(),
                        shot: None,
                        state: None,
                        thumbnail: None,
                    });
                }
                self.listed |= *done;
                if *done {
                    self.progress = (0, self.photos.len());
                }
                self.reorder();
            }
            LibraryEvent::SourceTaken { shots, .. } => {
                for (photo, shot) in self.photos.iter_mut().zip(shots) {
                    photo.shot = Some(shot.clone());
                }
                self.taken = true;
                self.reorder();
            }
            // Whether the source is still there, asked once: a card pulled out is removed,
            // one left in failed for another reason.
            LibraryEvent::SourceFailed { reason, .. } => {
                self.failed = Some(reason.clone());
                self.gone = !self.source.exists();
            }
            LibraryEvent::SourceChecked {
                check,
                folders,
                states,
                checked,
                of,
                done,
                ..
            } => {
                if *check != self.check {
                    return true;
                }
                // A newer check's first chunk replaces the states an older one told.
                if self.states_of != *check {
                    self.states_of = *check;
                    for photo in &mut self.photos {
                        photo.state = None;
                    }
                }
                let from = checked.saturating_sub(states.len());
                for (photo, state) in self.photos.iter_mut().skip(from).zip(states) {
                    photo.state = Some(state.clone());
                }
                self.folders.clone_from(folders);
                self.progress = (*checked, *of);
                self.checked = *done;
                if self.checked && self.listed && !self.opened {
                    self.open();
                }
            }
            LibraryEvent::Thumbnail { photo, image, .. } => {
                let Some(&at) = self.by_path.get(photo) else {
                    return true;
                };
                self.photos[at].thumbnail = Some(image.clone());
                self.asked.remove(&at);
                // A picture changes no cell's words.
                self.came += 1;
                return true;
            }
            _ => return false,
        }
        self.generation += 1;
        true
    }

    /// The opening's checks: the photos not imported before checked, unless the checks were
    /// changed by hand while the check ran; the first checked the sheet's active photo, else
    /// the first photo listed.
    fn open(&mut self) {
        self.opened = true;
        if !self.touched {
            for place in 0..self.order.len() {
                let new = self.at(place).is_new();
                self.checks.set(place, new);
            }
        }
        let first = self.checks.checked().next();
        self.checks
            .set_active(first.or((!self.order.is_empty()).then_some(0)));
        self.changed += 1;
    }

    /// Puts the photos in the grid's order, carrying the checks and the active photo over
    /// to their places in it.
    fn reorder(&mut self) {
        let photos = &self.photos;
        let mut order: Vec<usize> = (0..photos.len()).collect();
        order.sort_by(|&a, &b| {
            let (a, b) = (&photos[a], &photos[b]);
            let time = |photo: &Photo| photo.shot.as_ref().and_then(|shot| shot.taken).map(key);
            match (time(a), time(b)) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then_with(|| browser_order(&a.name, &b.name))
            .then_with(|| a.file.path.cmp(&b.file.path))
        });
        let mut place_of = vec![0; photos.len()];
        for (place, &photo) in order.iter().enumerate() {
            place_of[photo] = place;
        }
        let mut checks = Checks::new(order.len());
        for place in self.checks.checked() {
            checks.set(place_of[self.order[place]], true);
        }
        checks.set_active(
            self.checks
                .active()
                .map(|place| place_of[self.order[place]]),
        );
        self.checks = checks;
        self.order = order;
    }

    /// Applies what the person did to the checks.
    pub fn apply(&mut self, gesture: CheckGesture) {
        let was = self.checks.clone();
        self.checks.apply(gesture);
        self.touched |= !self.opened;
        self.changed_since(&was);
    }

    /// Applies a Select button to every cell: those not imported before alone for Select
    /// new, as darktable's "select new", every one or none; Select all and Select none, as
    /// Ctrl+A and Ctrl+Shift+A, forget the last touched, and so does Select new.
    pub fn select(&mut self, select: Select) {
        let was = self.checks.clone();
        match select {
            Select::All => self.checks.apply(CheckGesture::All),
            Select::None => self.checks.apply(CheckGesture::None),
            Select::New => {
                self.checks.apply(CheckGesture::None);
                for place in 0..self.order.len() {
                    let new = self.at(place).is_new();
                    self.checks.set(place, new);
                }
            }
        }
        self.touched |= !self.opened;
        self.changed_since(&was);
    }

    /// Bumps the checks' change where the photos checked differ from `was`'s.
    fn changed_since(&mut self, was: &Checks) {
        if !was.checked().eq(self.checks.checked()) {
            self.changed += 1;
        }
    }

    /// Bumped as the photos checked change.
    pub fn changed(&self) -> u64 {
        self.changed
    }

    /// The photos to import: the folder listed and the checked files' paths relative to
    /// it, once the folder is known.
    pub fn photos(&self) -> Option<ImportPhotos> {
        let folder = self.folder.clone()?;
        let files = (self.checks.checked())
            .map(|place| self.at(place).file.path.clone())
            .collect();
        Some(ImportPhotos { folder, files })
    }

    /// Where the listing stands, for the bar's counts.
    pub fn stage(&self) -> Stage {
        if self.number.is_none() || self.failed.is_some() {
            Stage::Nothing
        } else if !self.listed || !self.taken {
            Stage::Listing(self.photos.len())
        } else if !self.checked {
            let (checked, of) = self.progress;
            Stage::Checking {
                folder: self.folders.first().cloned(),
                checked,
                of,
            }
        } else {
            let before = (self.photos.iter()).filter(|p| p.imported_before()).count();
            Stage::Done {
                count: self.photos.len(),
                before,
            }
        }
    }

    /// `SourceThumbnails` for the photos at the grid's places `shown`, the cells in sight,
    /// at `size`: those whose thumbnails have not come and were not asked for since they
    /// came into sight, a cloud file not on the disk never; none where there are none.
    pub fn thumbnails(&mut self, shown: Range<usize>, size: u32) -> Option<Command> {
        let listing = self.number?;
        let shown: Vec<usize> =
            (shown.filter_map(|place| self.order.get(place).copied())).collect();
        let photos = &self.photos;
        // Out of sight or answered: asked for again should it be wanted.
        self.asked
            .retain(|at| shown.contains(at) && photos[*at].thumbnail.is_none());
        let wanted: Vec<usize> = (shown.into_iter())
            .filter(|at| {
                let photo = &photos[*at];
                photo.thumbnail.is_none() && photo.file.local && !self.asked.contains(at)
            })
            .collect();
        if wanted.is_empty() {
            return None;
        }
        self.asked.extend(&wanted);
        let photos = wanted
            .iter()
            .map(|at| photos[*at].file.path.clone())
            .collect();
        Some(Command::Library(LibraryCommand::SourceThumbnails {
            listing,
            photos,
            size,
        }))
    }

    /// How many photos are listed so far.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// The photo at the grid's place `place`.
    pub fn at(&self, place: usize) -> &Photo {
        &self.photos[self.order[place]]
    }

    /// The photo at `place`'s place in the listing's order, which its texture is kept by.
    pub fn index(&self, place: usize) -> usize {
        self.order[place]
    }

    pub fn checks(&self) -> &Checks {
        &self.checks
    }

    /// Whether the listing or its latest check still runs.
    pub fn busy(&self) -> bool {
        self.number.is_some() && self.failed.is_none() && !(self.listed && self.checked)
    }

    /// Why the source could not be listed, in the library's plain words.
    pub fn failed(&self) -> Option<&str> {
        self.failed.as_deref()
    }

    /// Whether the source folder was gone as its listing failed.
    pub fn gone(&self) -> bool {
        self.gone
    }

    /// Whether the opening's checks are set: the listing and its first check ended.
    pub fn opened(&self) -> bool {
        self.opened
    }

    /// Bumped as a cell's words change: the files listed, their times, their states.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Bumped as a thumbnail comes.
    pub fn came(&self) -> u64 {
        self.came
    }

    /// The trees the latest check walked.
    pub fn folders(&self) -> &[PathBuf] {
        &self.folders
    }

    /// The photos with a thumbnail come with a picture, by their places in the listing's
    /// order.
    fn pictures(&self) -> impl Iterator<Item = (usize, &Thumbnail)> {
        (self.photos.iter().enumerate())
            .filter_map(|(at, photo)| Some((at, photo.thumbnail.as_ref()?.as_ref()?)))
    }

    /// How many of the photos the latest check found new.
    pub fn new_count(&self) -> usize {
        self.photos.iter().filter(|photo| photo.is_new()).count()
    }
}

/// Where the listing stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stage {
    /// No source listed, or its listing failed.
    Nothing,
    /// Listing, this many found so far.
    Listing(usize),
    /// Checked against the trees, the first of them `folder` once a chunk has named it,
    /// `checked` photos of `of` so far.
    Checking {
        folder: Option<PathBuf>,
        checked: usize,
        of: usize,
    },
    /// Listed and checked: `count` photos, `before` of them imported before.
    Done { count: usize, before: usize },
}

/// A clock's time, to sort by: the camera's as it showed it, its offset aside, as darktable
/// sorts by its EXIF date and time.
fn key(at: DateTime) -> (u16, u8, u8, u8, u8, u8, u16) {
    (
        at.year,
        at.month,
        at.day,
        at.hour,
        at.minute,
        at.second,
        at.millisecond,
    )
}

/// The last part of `path`, as text.
fn file_name(path: &Path) -> String {
    (path.file_name()).map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// What a cell says, by its photo: built again as the listing or the language changes.
#[derive(Clone, Debug, Default, PartialEq)]
struct CellWords {
    check_name: String,
    /// Its check box's description: the capture time and whether it was imported before.
    said: String,
    tip: String,
    /// The mark's words, where it was imported before.
    mark: Option<String>,
}

/// The Import sheet's content.
pub struct ImportSheet {
    source: Option<Source>,
    settings: ImportSettings,
    listing: Listing,
    /// The settings whose trees the listing was last checked against.
    checked_with: Option<ImportSettings>,
    /// The cells' words, by the photos' places in the listing's order, as of the
    /// listing's generation in the language.
    words: Vec<CellWords>,
    words_of: Option<(u64, Locale)>,
    /// The pictures' textures, by the photos' places in the listing's order.
    textures: HashMap<usize, egui::TextureHandle>,
    textures_of: Option<u64>,
    /// What screen readers heard as the listing ended, said politely.
    said: Option<String>,
    /// The latest `PreviewImport`'s number, 0 where none was sent for no photos checked,
    /// with the checks' change and the settings it was asked for.
    asked: Option<(u64, u64, ImportSettings)>,
    /// The number of the `PreviewImport` the preview held answers.
    previewed: Option<u64>,
    /// Its answer, once it has come; an earlier one's stays until then.
    preview: Option<ImportPreview>,
}

impl ImportSheet {
    /// The sheet for `source`, or none yet, with the import settings `settings`: it lists
    /// the source as it is first drawn.
    pub fn new(source: Option<Source>, settings: ImportSettings) -> Self {
        ImportSheet {
            source,
            settings,
            listing: Listing::default(),
            checked_with: None,
            words: Vec::new(),
            words_of: None,
            textures: HashMap::new(),
            textures_of: None,
            said: None,
            asked: None,
            previewed: None,
            preview: None,
        }
    }

    /// The settings the sheet lists and checks with, which the Import tool changes.
    pub fn settings_mut(&mut self) -> &mut ImportSettings {
        &mut self.settings
    }

    /// The listing as the sheet mirrors it.
    #[cfg(test)]
    pub fn listing(&self) -> &Listing {
        &self.listing
    }

    /// Lists the source where it has not been, and checks it again where Folder or the
    /// default folder has changed since.
    fn sync(&mut self, send: &mut SendCommand<'_>) {
        let Some(source) = &self.source else {
            return;
        };
        let wanted = trees(&self.settings);
        match &self.checked_with {
            None => self.listing.list(&source.folder, &self.settings, send),
            Some(with) if trees(with) != wanted => self.listing.recheck(&self.settings, send),
            Some(_) => return,
        }
        self.checked_with = Some(self.settings.clone());
    }

    /// Asks for the preview of the photos checked with the settings, once the listing has
    /// opened, as either changes: `PreviewImport`, the latest answer alone shown.
    fn ask_preview(&mut self, send: &mut SendCommand<'_>) {
        if !self.listing.opened() || self.listing.failed().is_some() {
            return;
        }
        let changed = self.listing.changed();
        let asked = (self.asked.as_ref())
            .is_some_and(|(_, at, settings)| *at == changed && *settings == self.settings);
        if asked {
            return;
        }
        let Some(photos) = self.listing.photos() else {
            return;
        };
        let number = if photos.files.is_empty() {
            self.preview = None;
            0
        } else {
            send(Command::Library(LibraryCommand::PreviewImport {
                photos,
                settings: self.settings.clone(),
            }))
        };
        self.asked = Some((number, changed, self.settings.clone()));
    }

    /// Brings the cells' words and textures in line with the listing, in the language now.
    fn refresh(&mut self, ctx: &egui::Context, words: &Translator) {
        let generation = self.listing.generation();
        let now = (generation, words.locale());
        if self.words_of != Some(now) {
            self.words_of = Some(now);
            let home = self.settings.places.home.as_deref();
            let folders = self.listing.folders();
            self.words = (self.listing.photos.iter())
                .map(|photo| cell_words(photo, words, folders, home))
                .collect();
        }
        let came = self.listing.came();
        if self.textures_of != Some(came) {
            self.textures_of = Some(came);
            for (at, image) in self.listing.pictures() {
                self.textures.entry(at).or_insert_with(|| {
                    let pixels = [image.width as usize, image.height as usize];
                    let pixels = egui::ColorImage::from_rgba_unmultiplied(pixels, &image.pixels);
                    let name = format!("import thumbnail {at}");
                    ctx.load_texture(name, pixels, egui::TextureOptions::LINEAR)
                });
            }
        }
    }

    /// Why Import cannot press now, in words, or none where it can.
    fn why_not(&self, words: &Translator) -> Option<String> {
        let Some(source) = &self.source else {
            return Some(words.text("import-why-no-source"));
        };
        let mut args = Args::new();
        args.set("source", source.name.clone());
        if let Some(reason) = self.listing.failed() {
            args.set("reason", reason.to_owned());
            let id = if self.listing.gone() {
                "import-why-removed"
            } else {
                "import-why-failed"
            };
            return Some(words.text_with(id, Some(&args)));
        }
        if self.listing.busy() || !self.listing.opened() {
            return Some(words.text_with("import-why-listing", Some(&args)));
        }
        if self.listing.len() == 0 {
            let id = if source.card {
                "import-why-empty-card"
            } else {
                "import-why-empty-folder"
            };
            return Some(words.text_with(id, Some(&args)));
        }
        if self.listing.checks().checked_count() == 0 {
            return Some(words.text("import-why-none-checked"));
        }
        // The checks or the settings changed since the preview held: Import waits for its
        // answer, so it never starts what the footer has not shown.
        let previewing = match &self.asked {
            Some((number, changed, settings)) => {
                *changed != self.listing.changed()
                    || *settings != self.settings
                    || Some(*number) != self.previewed
            }
            None => true,
        };
        if previewing {
            return Some(words.text("import-why-previewing"));
        }
        let problem = self.preview.as_ref().and_then(|p| p.problems.first());
        problem.map(|problem| problem_words(problem, words, self.settings.places.home.as_deref()))
    }

    /// The bar's counts, as the listing stands.
    fn counts(&self, words: &Translator) -> String {
        let source = self.source.as_ref().map_or("", |s| s.name.as_str());
        let home = self.settings.places.home.as_deref();
        let mut args = Args::new();
        args.set("source", source.to_owned());
        let id = match self.listing.stage() {
            Stage::Nothing => return String::new(),
            Stage::Listing(count) => {
                args.set("count", count as f64);
                "import-counts-listing"
            }
            Stage::Checking {
                folder,
                checked,
                of,
            } => {
                args.set("checked", checked as f64);
                args.set("of", of as f64);
                match folder {
                    Some(folder) => {
                        args.set("folder", crate::export::shown(&folder, home));
                        "import-counts-checking"
                    }
                    None => "import-counts-checking-start",
                }
            }
            Stage::Done { count, before } => {
                args.set("count", count as f64);
                args.set("before", before as f64);
                if before > 0 {
                    "import-counts-before"
                } else {
                    "import-counts"
                }
            }
        };
        words.text_with(id, Some(&args))
    }

    /// What the grid says, as the listing stands: with no source, a source that failed,
    /// none listed or every photo imported before.
    fn message(&self, words: &Translator) -> Option<(String, String)> {
        let Some(source) = &self.source else {
            let note = words.text("import-grid-no-source-note");
            return Some((words.text("import-grid-no-source"), note));
        };
        if self.listing.failed().is_some() {
            return self.why_not(words).map(|why| (why, String::new()));
        }
        let Stage::Done { count, before } = self.listing.stage() else {
            return None;
        };
        let mut args = Args::new();
        args.set("source", source.name.clone());
        args.set("count", count as f64);
        let id = match (count, source.card) {
            (0, true) => "import-why-empty-card",
            (0, false) => "import-why-empty-folder",
            _ if before < count => return None,
            (_, true) => "import-grid-all-on",
            (_, false) => "import-grid-all-in",
        };
        Some((words.text_with(id, Some(&args)), String::new()))
    }
}

/// The footer's summary in words, owned, for `Summary`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Footer {
    pub lead: String,
    pub path: String,
    pub tail: String,
    pub tip: Option<String>,
    pub note: String,
    pub warning: bool,
}

/// The footer's summary of `checked` of the `count` photos of `source` with `preview`, the
/// latest answer, home as `~`: the first line the count, the size and the free space, the
/// destination and whether its folder is new, several or there already, its tooltip the
/// destination whole and the disk, empty with nothing checked; the second the photos imported
/// before that will be copied again, in `warning`, else that nothing on the source changes.
/// The primary button's reason, where it cannot press, takes the second line's place.
pub fn footer(
    words: &Translator,
    source: &str,
    (count, checked): (usize, usize),
    preview: Option<&ImportPreview>,
    home: Option<&Path>,
) -> Footer {
    let mut args = Args::new();
    args.set("source", source.to_owned());
    args.set("count", count as f64);
    args.set("checked", checked as f64);
    let mut out = Footer {
        note: words.text_with("import-note-copies", Some(&args)),
        ..Footer::default()
    };
    // With no photos, nothing to sum up; with none checked, Import's reason says so.
    if checked == 0 {
        return out;
    }
    // Before the first answer, and where a problem leaves no destination, the count alone.
    let Some(preview) = preview.filter(|preview| !preview.folders.is_empty()) else {
        out.lead = words.text_with("import-summary-lead-waiting", Some(&args));
        return out;
    };
    let size = crate::metadata::format::size(preview.size, words);
    args.set("size", size.clone());
    let id = match &preview.free {
        Some(free) => {
            args.set("free", crate::metadata::format::size(free.available, words));
            "import-summary-lead"
        }
        None => "import-summary-lead-unknown",
    };
    out.lead = words.text_with(id, Some(&args));
    let folders = &preview.folders;
    let destination = match folders.as_slice() {
        [] => None,
        [one] => Some(one.folder.clone()),
        several => common(several.iter().map(|f| f.folder.as_path())),
    };
    if let Some(destination) = &destination {
        out.path = crate::export::shown(destination, home);
    }
    let new = folders.iter().filter(|f| f.there.is_none()).count();
    let taken = preview.folder_uses.taken || preview.subfolder_uses.taken;
    let mut tail = Args::new();
    let id = match folders.as_slice() {
        [] => None,
        [one] => Some(match one.there {
            None => "import-summary-new-folder",
            Some(there) => {
                tail.set("count", there as f64);
                "import-summary-there"
            }
        }),
        several => {
            tail.set("count", (if new > 0 { new } else { several.len() }) as f64);
            Some(match (new > 0, taken) {
                (true, true) => "import-summary-new-folders-by-day",
                (true, false) => "import-summary-new-folders",
                (false, _) => "import-summary-folders-there",
            })
        }
    };
    if let Some(id) = id {
        out.tail = words.text_with(id, Some(&tail));
    }
    // The destination whole, then the disk where its free space was read.
    let disk = preview.free.as_ref().map(|free| {
        let mut on = Args::new();
        on.set("size", size.clone());
        on.set("free", crate::metadata::format::size(free.available, words));
        on.set("volume", free.volume.clone());
        words.text_with("import-summary-tip", Some(&on))
    });
    let whole = destination.as_ref().map(|d| crate::export::shown(d, home));
    let tip: Vec<String> = whole.into_iter().chain(disk).collect();
    out.tip = (!tip.is_empty()).then(|| tip.join("\n"));
    if preview.imported_before > 0 {
        let mut again = Args::new();
        again.set("count", preview.imported_before as f64);
        out.note = words.text_with("import-note-again", Some(&again));
        out.warning = true;
    }
    out
}

/// The deepest folder the `folders` share, none where they share none.
fn common<'a>(mut folders: impl Iterator<Item = &'a Path>) -> Option<PathBuf> {
    let mut shared = folders.next()?.to_owned();
    for folder in folders {
        while !folder.starts_with(&shared) {
            if !shared.pop() {
                return None;
            }
        }
    }
    Some(shared)
}

/// `problem` in words, naming its row: "Subfolder: unknown variable $(MONT).".
pub fn problem_words(problem: &ImportProblem, words: &Translator, home: Option<&Path>) -> String {
    let mut args = Args::new();
    let id = match &problem.problem {
        Problem::Pattern(PatternError::Unknown { name, .. }) if name.is_empty() => {
            "import-problem-nameless"
        }
        Problem::Pattern(PatternError::Unknown { name, .. }) => {
            args.set("name", name.clone());
            "import-problem-unknown"
        }
        Problem::Pattern(PatternError::Unclosed {
            variable, expected, ..
        }) => {
            args.set("variable", variable.clone());
            args.set("expected", expected.to_string());
            "import-problem-unclosed"
        }
        Problem::Pattern(PatternError::NoOperation {
            variable, found, ..
        }) => {
            args.set("variable", variable.clone());
            args.set("found", found.to_string());
            "import-problem-no-operation"
        }
        Problem::Pattern(PatternError::TooDeep { .. }) => {
            args.set("depth", MAX_DEPTH.to_string());
            "import-problem-too-deep"
        }
        Problem::OutsideFolder { subfolder } => {
            args.set("subfolder", subfolder.clone());
            "import-problem-outside-folder"
        }
        Problem::Unwritable { at, why } => {
            args.set("folder", crate::export::shown(at, home));
            match why {
                Unwritable::NotAbsolute => "import-problem-not-absolute",
                Unwritable::NotConnected => "import-problem-not-connected",
                Unwritable::NotAFolder => "import-problem-not-a-folder",
                Unwritable::ReadOnly => "import-problem-read-only",
                Unwritable::Unreadable(kind) => {
                    args.set("why", kind.to_string());
                    "import-problem-unreadable"
                }
            }
        }
        Problem::OnSource { why, .. } => match why {
            OnSource::IsSource => "import-problem-is-source",
            OnSource::InsideSource => "import-problem-inside-source",
            OnSource::OnCard { .. } => "import-problem-on-card",
        },
        Problem::Space {
            needed,
            available,
            volume,
        } => {
            args.set("needed", crate::metadata::format::size(*needed, words));
            args.set(
                "available",
                crate::metadata::format::size(*available, words),
            );
            args.set("volume", volume.clone());
            "import-problem-space"
        }
    };
    let row = match problem.row {
        Row::Folder => "import-row-folder",
        Row::Subfolder => "import-row-subfolder",
    };
    let mut named = Args::new();
    named.set("row", words.text(row));
    named.set("problem", words.text_with(id, Some(&args)));
    words.text_with("import-problem", Some(&named))
}

impl sheet::Content for ImportSheet {
    fn show(&mut self, ui: &mut egui::Ui, place: Place, send: &mut SendCommand<'_>) -> Shown {
        self.sync(send);
        let ctx = ui.ctx().clone();
        let words = translator(&ctx);
        self.refresh(&ctx, &words);
        let source = (self.source.as_ref()).map_or_else(String::new, |s| s.name.clone());
        let with = |id: &str, pairs: &[(&str, f64)]| {
            let mut args = Args::new();
            args.set("source", source.clone());
            for (name, value) in pairs {
                args.set(name.to_string(), *value);
            }
            words.text_with(id, Some(&args))
        };
        let count = self.listing.len();
        let checked = self.listing.checks().checked_count();
        // With no source yet, the sheet's name alone.
        let (heading, name) = match &self.source {
            Some(_) => (with("import-heading", &[]), with("import-sheet-name", &[])),
            None => (
                words.text("import-heading-none"),
                words.text("import-heading-none"),
            ),
        };
        let counts = [("count", count as f64), ("checked", checked as f64)];
        let grid_name = with("import-grid-name", &counts);
        let checked_said = with("import-grid-checked", &counts);
        let primary = with("import-primary", &[("count", checked as f64)]);
        let why_not = self.why_not(&words);
        let bar_counts = self.counts(&words);
        let bar_name = words.text("import-bar-name");
        let selects: Vec<(Select, String)> = (Select::ALL.into_iter())
            .map(|select| (select, words.text(select.words())))
            .collect();
        let buttons: Vec<SelectButton> = (selects.iter())
            .map(|(select, text)| SelectButton {
                id: select.id(),
                text,
            })
            .collect();
        let message = self.message(&words);
        let home = self.settings.places.home.as_deref();
        let summary = footer(
            &words,
            &source,
            (count, checked),
            self.preview.as_ref(),
            home,
        );
        let listing = &self.listing;
        let cells = &self.words;
        let textures = &self.textures;
        let out = Sheet::new(&heading, &name, count, listing.checks().active(), |place| {
            let photo = listing.at(place);
            let at = listing.index(place);
            let said = &cells[at];
            let checked = listing.checks().is_checked(place);
            SheetCell {
                name: &photo.name,
                picture: textures.get(&at).map(|texture| CellPicture {
                    texture: texture.id(),
                    size: texture.size_vec2(),
                }),
                no_preview: false,
                checked,
                check_name: &said.check_name,
                check_description: &said.said,
                dimmed: photo.imported_before() && !checked,
                mark: said.mark.as_deref().map(|words| CellMark {
                    icon: Icon::ImportedBefore,
                    words,
                }),
                tip: Some(&said.tip),
            }
        })
        .counts(&bar_counts)
        .bar_name(&bar_name)
        .select(&buttons)
        .grid_name(&grid_name, &checked_said)
        .message(
            message
                .as_ref()
                .map(|(text, note)| GridMessage { text, note }),
        )
        .summary(Summary {
            lead: &summary.lead,
            path: &summary.path,
            tail: &summary.tail,
            tip: summary.tip.as_deref(),
            note: &summary.note,
            warning: summary.warning,
        })
        .size(ThumbnailSize::Medium)
        .column(place.column, place.at_start)
        .primary(&primary, why_not.as_deref())
        .show(ui, |_, _| {});
        let opened = self.listing.opened();
        if let Some(gesture) = out.gesture {
            self.listing.apply(gesture);
        }
        let select = out.select.as_deref();
        if let Some(select) = Select::ALL.into_iter().find(|s| Some(s.id()) == select) {
            self.listing.select(select);
        }
        self.ask_preview(send);
        let side = ThumbnailSize::Medium.side();
        let size = crate::thumbnails::size_for(side, ctx.pixels_per_point());
        if let Some(command) = self.listing.thumbnails(out.shown.clone(), size) {
            send(command);
        }
        // The listing's end, said once: how many were listed, and how many are new and
        // checked.
        if opened && self.said.is_none() {
            let new = self.listing.new_count() as f64;
            self.said = Some(with(
                "import-listed-said",
                &[("count", count as f64), ("new", new)],
            ));
        }
        if let Some(said) = &self.said {
            announce(ui, said);
        }
        Shown {
            closed: out.close,
            column: out.column,
        }
    }

    fn receive(&mut self, event: &LibraryEvent) {
        if let LibraryEvent::ImportPreview { open, preview } = event {
            if self.asked.as_ref().is_some_and(|(asked, ..)| asked == open) {
                self.preview = Some(preview.as_ref().clone());
                self.previewed = Some(*open);
            }
            return;
        }
        self.listing.receive(event);
    }

    fn closing(&mut self) -> Vec<Command> {
        self.listing.cancel().into_iter().collect()
    }
}

/// `said` in a polite live region of `ui`'s, which screen readers say as it comes.
fn announce(ui: &egui::Ui, said: &str) {
    let rect = egui::Rect::from_min_size(ui.max_rect().min, egui::Vec2::ZERO);
    let status = ui.interact(
        rect,
        egui::Id::unique("import listed said"),
        egui::Sense::hover(),
    );
    status.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Status, true, said));
    (ui.ctx()).accesskit_node_builder(status.id, |node| {
        node.set_live(egui::accesskit::Live::Polite);
    });
}

/// What `photo`'s cell says in `words`: its check box's name and description, its tooltip
/// and its mark's words, where a copy of it is shown relative to the tree of `folders` it
/// lies in, else with `home` as `~`.
fn cell_words(
    photo: &Photo,
    words: &Translator,
    folders: &[PathBuf],
    home: Option<&Path>,
) -> CellWords {
    let say = |id: &str, name: &str, value: String| {
        let mut args = Args::new();
        args.set(name.to_owned(), value);
        words.text_with(id, Some(&args))
    };
    let taken = (photo.shot.as_ref())
        .and_then(|shot| shot.taken.as_ref())
        .and_then(naive)
        .map(|at| words.format_date_time_to_second(at))
        .filter(|at| !at.is_empty());
    let modified = (photo.file.modified)
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map(|since| words.format_date_time(since.as_millis() as u64))
        .filter(|at| !at.is_empty());
    let camera = photo.shot.as_ref().and_then(camera);
    let mark = match &photo.state {
        Some(SourceState::ImportedBefore { copy }) => Some(say(
            "import-cell-imported-before",
            "path",
            shown_in(copy, folders, home),
        )),
        Some(SourceState::ProbablyImported { candidate }) => Some(say(
            "import-cell-probably",
            "path",
            shown_in(candidate, folders, home),
        )),
        Some(SourceState::New) | None => None,
    };
    let tip = (Some(photo.name.clone()).into_iter())
        .chain(taken.clone().map(|at| say("import-cell-taken", "time", at)))
        .chain(modified.map(|at| say("import-cell-modified", "time", at)))
        .chain(Some(size(photo.file.size, words)))
        .chain(camera)
        .chain(mark.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let before = photo.imported_before();
    let said = match (taken, before) {
        (Some(at), false) => say("import-cell-said", "time", at),
        (Some(at), true) => say("import-cell-said-imported", "time", at),
        (None, true) => words.text("import-cell-said-imported-only"),
        (None, false) => String::new(),
    };
    CellWords {
        check_name: say("import-cell-check", "name", photo.name.clone()),
        said,
        tip,
        mark,
    }
}

/// The camera's make and model as one name, the make left out where the model starts with
/// it, as most cameras write their models: "Canon EOS R5", "FUJIFILM X-T5".
fn camera(shot: &SourceShot) -> Option<String> {
    let make = shot
        .make
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    let model = shot
        .model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty());
    match (make, model) {
        (Some(make), Some(model)) => {
            let first = make.split_whitespace().next().unwrap_or(make);
            let named = (model.to_lowercase()).starts_with(&first.to_lowercase());
            Some(if named {
                model.to_owned()
            } else {
                format!("{make} {model}")
            })
        }
        (Some(one), None) | (None, Some(one)) => Some(one.to_owned()),
        (None, None) => None,
    }
}

/// `path` relative to the first of `folders` it lies in, as a copy is named in the tree the
/// check walked, "20261108_Studio/20261108_0042.CR3"; else whole, with `home` as `~`.
fn shown_in(path: &Path, folders: &[PathBuf], home: Option<&Path>) -> String {
    (folders.iter())
        .find_map(|folder| path.strip_prefix(folder).ok())
        .filter(|rest| !rest.as_os_str().is_empty())
        .map_or_else(
            || crate::export::shown(path, home),
            |rest| rest.display().to_string(),
        )
}
