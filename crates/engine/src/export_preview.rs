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

//! The Export dialog's feedback from the session (decision 56; `docs/engine.md`, Export
//! jobs; `docs/gui.md`, The Export dialog, Feedback): [`crate::session::Command::PreviewExport`]
//! and its answer, [`ExportPreview`], sent as [`crate::session::Event::ExportPreview`].
//!
//! A `UserForeground` job with no progress, on a job worker and never on the engine
//! thread, since a NAS can take seconds to say which files exist. The session cancels the
//! preview before it when a new one comes, so a pattern typed letter by letter queues no
//! stale work, and a preview that was cancelled sends nothing. It reports, for the photos
//! and settings given:
//!
//! - **each photo's name** as the export would write it, with the sequence rule
//!   (`_$(SEQUENCE)` added where several photos have a path with no variable) and what On
//!   conflict would do with a file already there ([`Disk::foresee`]), counting the files
//!   the photos before it take, as the export writes them one after the other;
//! - **the pattern's errors** (D8) and **the variables with no value here** (D7);
//! - **the active photo's output size**, the open photo's, which is the one whose extent
//!   is in memory;
//! - **the folders' preflight**, whether each can be made or written, without making it.
//!
//! It writes nothing and makes nothing. It decodes no image: a name that uses the shooting
//! data reads it with `apertrust_raw::read_exif_file`, the metadata alone, and only when
//! the pattern uses any (a probe expands it with and without such values to see). What
//! only a decode gives, the camera's name as the decoder reads it, the ISO, the exposure
//! bias and the image's sizes, is known for the open photo and empty for the others, whose
//! names then say they are not exact ([`PhotoName::exact`]) if the pattern uses them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use apertrust_export::disk::{self, Claimed, Disk, Foreseen};
pub use apertrust_export::disk::{Action, Blocked, Preflight, Why};
use apertrust_export::variables::{DateTime, Pattern, PatternError, Size, Unavailable, Values};
use apertrust_export::{ExportSettings, Places};
use apertrust_modules::Module;
use apertrust_plugin::{ImageContext, ImageDesc, Operation};

use crate::edit::{self, Edit};
use crate::export_job::{self, Live, SizeError};
use crate::jobs::Job;
use crate::names::{self, Shot, Sizes};
use crate::pipeline::Limits;
use crate::session::{Event, Outbox};
use crate::sidecar;

/// What the Export dialog shows under its rows, for the latest settings and photos.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPreview {
    /// What stops the path (D8), shown at the field; no names then.
    pub problem: Option<PathProblem>,
    /// Several photos and no variable in the path: darktable's rule added `_$(SEQUENCE)`.
    pub sequence_added: bool,
    /// The variables the path holds that have no value here, and why (D7).
    pub not_here: Vec<Unavailable>,
    /// One for each photo, in the order given.
    pub names: Vec<PhotoName>,
    /// The open photo's size, if it is among the photos; none otherwise, since another
    /// photo's needs a decode.
    pub size: Option<ActiveSize>,
    /// Each folder the files go to, once, in the order they come.
    pub folders: Vec<FolderCheck>,
}

/// What stops the path before any photo (D8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathProblem {
    /// The path is empty.
    Empty,
    /// An unknown variable, one not closed, a character that is no operation, or variables
    /// nested too deep, each with its place in the path as written.
    Pattern(PatternError),
}

/// One photo's file as the export would write it.
#[derive(Debug, Clone, PartialEq)]
pub struct PhotoName {
    /// The raw.
    pub source: PathBuf,
    /// Whether the name is the export's own. It is not where the path uses a value only a
    /// decode gives (the camera's name as the decoder reads it, the ISO, the exposure bias,
    /// the image's sizes) and this is not the open photo: those variables then show as
    /// empty.
    pub exact: bool,
    pub fate: Fate,
}

/// What the export does with a photo's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fate {
    /// It writes `file`.
    File {
        /// Where the file goes, a number added for Create unique filename included.
        file: PathBuf,
        /// Whether a file is already under the name the path gives, or an earlier photo of
        /// the export takes it.
        exists: bool,
        /// What On conflict does.
        action: Action,
    },
    /// The path expands to no file name for this photo (D8): the export fails it.
    NoName,
}

/// The open photo's output size.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveSize {
    pub source: PathBuf,
    pub size: Result<OutputSize, SizeProblem>,
}

/// An export's size, as `apertrust_export::size` computes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputSize {
    /// The photo at full resolution after its geometry.
    pub extent: [u32; 2],
    /// The file's size.
    pub width: u32,
    pub height: u32,
    /// The scale from the photo to the file: 1 at full size, below 1 where it shrinks,
    /// above 1 where it upscales.
    pub scale: f32,
}

/// Why the size stops the export (D8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeProblem {
    /// A side larger than the format's largest.
    TooLarge {
        format: &'static str,
        largest: u32,
        width: u32,
        height: u32,
    },
    /// The size leaves less than a pixel.
    Empty { width: u32, height: u32 },
    /// The photo's edit does not plan or bind, in plain words.
    Failed(String),
}

/// A folder the files go to, and whether it can be made or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderCheck {
    pub folder: PathBuf,
    pub preflight: Preflight,
}

/// What a preview counted, for the line under On conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Photos whose name is already taken.
    pub exist: usize,
    /// Of those, the ones saved with a number added.
    pub numbered: usize,
    /// Of those, the ones that replace the file there.
    pub overwritten: usize,
    /// Of those, the ones left out.
    pub skipped: usize,
    /// Photos the path gives no file name.
    pub unnamed: usize,
}

impl ExportPreview {
    /// How many photos On conflict touches.
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for name in &self.names {
            let Fate::File { exists, action, .. } = name.fate else {
                counts.unnamed += 1;
                continue;
            };
            counts.exist += usize::from(exists);
            match action {
                Action::Write => {}
                Action::Number => counts.numbered += 1,
                Action::Overwrite => counts.overwritten += 1,
                Action::Skip(_) => counts.skipped += 1,
            }
        }
        counts
    }
}

/// The open photo as the preview needs it: the parts of [`Live`] that plan its size and
/// fill its variables.
pub(crate) struct Active {
    sidecar: PathBuf,
    edit: Edit,
    image: ImageContext,
    sensor: ImageDesc,
    shot: Shot,
    unsaved: bool,
}

impl Active {
    pub(crate) fn of(live: &Live) -> Self {
        Self {
            sidecar: live.sidecar.clone(),
            edit: live.edit.clone(),
            image: live.image.clone(),
            sensor: live.input.desc(),
            shot: live.shot.clone(),
            unsaved: live.unsaved,
        }
    }
}

/// What a preview job runs with, owned: its body borrows nothing of the engine thread's.
pub(crate) struct Run {
    pub operations: &'static [&'static dyn Operation],
    pub modules: Vec<&'static dyn Module>,
    pub limits: Limits,
    pub photos: Vec<PathBuf>,
    pub settings: ExportSettings,
    pub places: Places,
    /// For the variables, as the export takes it when it starts.
    pub now: DateTime,
    pub build: String,
    pub active: Option<Active>,
    /// The number of the command, which the answer names.
    pub through: u64,
    pub outbox: Outbox,
    /// Holds the job before it looks, for a test.
    #[cfg(test)]
    pub hold: Option<crate::export_job::testing::Hold>,
}

/// The job that previews `run`'s export: no progress, no GPU.
pub(crate) fn job(run: Run) -> Job {
    let description = format!("preview of an export of {} images", run.photos.len());
    Job::new("export preview", description, move |context| {
        #[cfg(test)]
        if let Some(hold) = &run.hold {
            hold.wait();
        }
        // A cancelled preview, or one a newer replaced, sends nothing.
        let Some(preview) = run.preview(|| context.cancelled()) else {
            return Ok(());
        };
        if context.cancelled() {
            return Ok(());
        }
        let event = Event::ExportPreview {
            through: run.through,
            preview: Box::new(preview),
        };
        run.outbox.send(event);
        Ok(())
    })
}

impl Run {
    /// The preview, or `None` where `cancelled` says so, which it asks between photos.
    pub(crate) fn preview(&self, cancelled: impl Fn() -> bool) -> Option<ExportPreview> {
        let settings = &self.settings;
        let total = self.photos.len();
        let extension = settings.format.extension();
        let mut preview = ExportPreview {
            problem: None,
            sequence_added: false,
            not_here: Vec::new(),
            names: Vec::new(),
            size: None,
            folders: Vec::new(),
        };

        // The path as written, so that an error's place is in the person's text and the
        // variables with no value are those it names.
        let typed = match Pattern::parse(&settings.path) {
            Ok(typed) => typed,
            Err(error) => {
                preview.problem = Some(PathProblem::Pattern(error));
                return Some(preview);
            }
        };
        preview.not_here = typed.not_here();
        let names = names::Export::new(
            self.now,
            export_job::largest_size(settings),
            settings.size.upscale,
            &self.build,
            self.places.clone(),
        );
        let disk = match Disk::new(&settings.path, settings.on_conflict, total, names.home()) {
            Ok(disk) => disk,
            Err(disk::Error::Empty) => {
                preview.problem = Some(PathProblem::Empty);
                return Some(preview);
            }
            Err(disk::Error::Pattern(error)) => {
                preview.problem = Some(PathProblem::Pattern(error));
                return Some(preview);
            }
            Err(other) => unreachable!("a path is refused for its pattern alone: {other}"),
        };
        preview.sequence_added = disk.sequence_added();

        // The open photo, whose extent is in memory.
        let active = self.active.as_ref().and_then(|active| {
            let position = self
                .photos
                .iter()
                .position(|p| sidecar::path_for(p) == active.sidecar)?;
            Some((position, active))
        });
        let sized = active.map(|(position, active)| self.size_of(&self.photos[position], active));
        preview.size = sized.as_ref().map(|(size, _)| size.clone());

        // What the path asks of each photo, found by expanding it with and without values.
        let uses = Uses::of(&disk, extension);
        let mut claimed = Claimed::default();
        let mut folders = BTreeSet::new();
        for (index, path) in self.photos.iter().enumerate() {
            if cancelled() {
                return None;
            }
            let open = active.filter(|(position, _)| *position == index);
            let exif = if uses.exif {
                read_exif(path)
            } else {
                apertrust_raw::Exif::default()
            };
            let (shot, sizes) = match (open, &sized) {
                (Some((_, active)), Some((_, sizes))) => (active.shot.clone(), *sizes),
                _ => (Shot::default(), UNKNOWN),
            };
            let values = names.values_with(path, &shot, index as u32 + 1, sizes, exif);
            let changed = changed(path, open.map(|(_, active)| active));
            let fate = match disk.foresee(&values, extension, changed, &claimed) {
                Ok(Foreseen {
                    path: file,
                    exists,
                    action,
                }) => {
                    if !matches!(action, Action::Skip(_)) {
                        claimed.insert(&file);
                    }
                    let folder = file.parent().filter(|p| !p.as_os_str().is_empty());
                    let folder = folder.unwrap_or(Path::new(".")).to_owned();
                    if folders.insert(folder.clone()) {
                        preview.folders.push(FolderCheck {
                            preflight: disk::preflight(&folder),
                            folder,
                        });
                    }
                    Fate::File {
                        file,
                        exists,
                        action,
                    }
                }
                Err(_) => Fate::NoName,
            };
            preview.names.push(PhotoName {
                source: path.clone(),
                exact: open.is_some() || !uses.decoded,
                fate,
            });
        }
        Some(preview)
    }

    /// The open photo's size, and the sizes its variables read: its image's, and its
    /// extent after every geometry.
    fn size_of(&self, source: &Path, active: &Active) -> (ActiveSize, Sizes) {
        let size = |width, height| Size { width, height };
        let mut sizes = Sizes {
            sensor: size(active.sensor.width, active.sensor.height),
            developed: size(active.image.width, active.image.height),
            crop: size(active.image.width, active.image.height),
        };
        let settings = &self.settings;
        let bound = edit::instances(&active.edit, self.operations, &self.modules, &active.image);
        let result = match bound.all() {
            Err(error) => Err(SizeProblem::Failed(error.to_string())),
            Ok(instances) => export_job::size(
                &settings.size,
                &settings.format,
                &instances,
                &active.image,
                active.sensor,
                &self.limits,
            )
            .map(|(resample, extent)| {
                sizes.crop = size(extent[0], extent[1]);
                let [width, height] = resample.map_or(extent, |r| r.output);
                OutputSize {
                    extent,
                    width,
                    height,
                    scale: resample.map_or(1.0, |r| r.scale),
                }
            })
            .map_err(size_problem),
        };
        let active_size = ActiveSize {
            source: source.to_owned(),
            size: result,
        };
        (active_size, sizes)
    }
}

/// Why the size stops the export, D8's message at the field.
fn size_problem(error: SizeError) -> SizeProblem {
    match error {
        SizeError::Size(apertrust_export::Error::TooLarge {
            format,
            largest,
            width,
            height,
        }) => SizeProblem::TooLarge {
            format,
            largest,
            width,
            height,
        },
        SizeError::Size(apertrust_export::Error::Empty { width, height }) => {
            SizeProblem::Empty { width, height }
        }
        other => SizeProblem::Failed(other.to_string()),
    }
}

/// The sizes of a photo that is not decoded: none.
const UNKNOWN: Sizes = Sizes {
    sensor: Size {
        width: 0,
        height: 0,
    },
    developed: Size {
        width: 0,
        height: 0,
    },
    crop: Size {
        width: 0,
        height: 0,
    },
};

/// The photo's change time for Overwrite if changed: its edit's file's time, or now for
/// the open photo's edit that is not written yet, none for a photo never edited, as the
/// export takes it (D12). Advisory for the open photo: the sidecar's time is read
/// directly, not in order with the edit writer's pending writes, so a write still queued
/// can make the answer differ from the export's, which decides.
fn changed(path: &Path, active: Option<&Active>) -> Option<SystemTime> {
    if active.is_some_and(|active| active.unsaved) {
        return Some(SystemTime::now());
    }
    std::fs::metadata(sidecar::path_for(path))
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// The photo's shooting data from its file, without decoding the image; none where it
/// cannot be read, as the export reads it.
fn read_exif(path: &Path) -> apertrust_raw::Exif {
    apertrust_raw::read_exif_file(path).unwrap_or_else(|error| {
        tracing::debug!(target: "raw", "{}: no shooting data: {error}", path.display());
        apertrust_raw::Exif::default()
    })
}

/// What the path asks of a photo beyond its file and its place in the export.
struct Uses {
    /// The shooting data its file records.
    exif: bool,
    /// What only a decode gives.
    decoded: bool,
}

impl Uses {
    /// Expands the path for a photo with none of it, and with each filled in with values
    /// no photo has: the path uses what makes a difference. The probe sees a use by a
    /// change in the name, so only an operation that removes exactly the probe's value
    /// (a remove-prefix of "Probe lens") can hide it; any other value still shows.
    fn of(disk: &Disk, extension: &str) -> Self {
        let plain = Values {
            path: "/probe/IMG_0001.ARW".to_owned(),
            sequence: 1,
            ..Values::default()
        };
        let name = |values: &Values| disk.name(values, extension).ok();
        let bare = name(&plain);
        let differs = |fill: fn(&mut Values)| {
            let mut values = plain.clone();
            fill(&mut values);
            name(&values) != bare
        };
        Self {
            exif: differs(|v| {
                let when = DateTime {
                    year: 1999,
                    month: 12,
                    day: 31,
                    hour: 23,
                    minute: 58,
                    second: 57,
                    microsecond: 123_000,
                };
                v.taken = Some(when);
                v.exposure = 1.5;
                v.aperture = 2.8;
                v.focal_length = 85.0;
                v.focus_distance = 3.5;
                v.exposure_program = Some(3);
                v.metering_mode = Some(5);
                v.white_balance = Some(1);
                v.flash = Some(9);
                v.latitude = Some(50.45);
                v.longitude = Some(30.52);
                v.elevation = Some(179.0);
                v.lens = "Probe lens".to_owned();
                v.creator = Some("Probe creator".to_owned());
                v.rights = Some("Probe rights".to_owned());
                v.description = Some("Probe description".to_owned());
            }),
            decoded: differs(|v| {
                v.maker = "Probe maker".to_owned();
                v.model = "Probe model".to_owned();
                v.iso = 6400.0;
                v.exposure_bias = Some(-1.5);
                v.sensor = Size {
                    width: 6048,
                    height: 4024,
                };
                v.raw = Size {
                    width: 6000,
                    height: 4000,
                };
                v.crop = Size {
                    width: 5000,
                    height: 3333,
                };
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Duration;

    use apertrust_export::Format;
    use apertrust_export::png;
    use apertrust_export::settings::OnConflict;
    use apertrust_export::size::Request;
    use apertrust_modules::testing::{self as modules, Sample};
    use apertrust_operations::OPERATIONS;
    use apertrust_plugin::Format as Pixels;

    use super::*;
    use crate::compute::GpuImage;
    use crate::export_job::testing::{Hold, path_in};
    use crate::gpu::{GpuContext, GpuOptions};
    use crate::pipeline;
    use crate::session::{Command, Decoded, Session, SessionError, TEST_WAIT};

    const MODULES: &[&dyn Module] = &[&Sample];

    /// A folder of the test's own, empty.
    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("apertrust-preview-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// The names in `folder`, sorted.
    fn listed(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn settings(path: &str, on_conflict: OnConflict) -> ExportSettings {
        ExportSettings {
            path: path.to_owned(),
            on_conflict,
            format: Format::Png(png::Options::default()),
            size: Request::default(),
        }
    }

    fn photos(folder: &Path, names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(|name| folder.join(name)).collect()
    }

    /// A job's run that nobody hears, for `photos` and `settings`.
    fn run(photos: Vec<PathBuf>, settings: ExportSettings, active: Option<Active>) -> Run {
        Run {
            operations: OPERATIONS,
            modules: MODULES.to_vec(),
            limits: Limits::NONE,
            photos,
            settings,
            places: Places::default(),
            now: names::now(),
            build: "0.4.0+12~g1a2b3c4".to_owned(),
            active,
            through: 1,
            outbox: Outbox::unheard(),
            hold: None,
        }
    }

    fn preview(run: &Run) -> ExportPreview {
        run.preview(|| false).expect("not cancelled")
    }

    /// The files of a preview's names, relative to `folder`.
    fn files(preview: &ExportPreview, folder: &Path) -> Vec<String> {
        let relative = |fate: &Fate| match fate {
            Fate::File { file, .. } => file
                .strip_prefix(folder)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/"),
            Fate::NoName => "(no name)".to_owned(),
        };
        preview.names.iter().map(|n| relative(&n.fate)).collect()
    }

    fn pattern_of(path: &str) -> PatternError {
        let settings = settings(path, OnConflict::CreateUniqueFilename);
        match preview(&run(Vec::new(), settings, None)).problem {
            Some(PathProblem::Pattern(error)) => error,
            other => panic!("no pattern error: {other:?}"),
        }
    }

    /// The open photo `photo` as the session holds it: the modules crate's synthetic one.
    fn active(photo: &Path, unsaved: bool) -> Active {
        Active {
            sidecar: sidecar::path_for(photo),
            edit: Edit::default(),
            image: modules::image(),
            sensor: ImageDesc::new(96, 72, Pixels::Mosaic16),
            shot: Shot {
                maker: "Maker".to_owned(),
                model: "Model".to_owned(),
                iso: Some(400.0),
                exposure_bias: None,
                ..Shot::default()
            },
            unsaved,
        }
    }

    #[test]
    fn a_photo_is_named_by_the_path_and_the_formats_extension() {
        let out = scratch("one");
        let path = path_in(&out, "$(FILE_NAME)");
        let settings = settings(&path, OnConflict::CreateUniqueFilename);
        let run = run(photos(&out, &["IMG_2041.ARW"]), settings, None);
        let preview = preview(&run);
        assert_eq!(preview.problem, None);
        assert!(!preview.sequence_added);
        assert_eq!(files(&preview, &out), ["IMG_2041.png"]);
        let name = &preview.names[0];
        assert_eq!(name.source, out.join("IMG_2041.ARW"));
        assert!(name.exact);
        assert_eq!(
            name.fate,
            Fate::File {
                file: out.join("IMG_2041.png"),
                exists: false,
                action: Action::Write
            }
        );
        assert_eq!(preview.counts(), Counts::default());
    }

    #[test]
    fn several_photos_and_no_variable_add_the_sequence_and_say_so() {
        let out = scratch("sequence");
        let raws = photos(&out, &["a.ARW", "b.ARW", "c.ARW"]);
        let path = path_in(&out, "final");
        let several = run(raws.clone(), settings(&path, OnConflict::Skip), None);
        let said = preview(&several);
        assert!(said.sequence_added);
        assert_eq!(
            files(&said, &out),
            ["final_0001.png", "final_0002.png", "final_0003.png"]
        );
        // One photo needs no number.
        let single = run(raws[..1].to_vec(), settings(&path, OnConflict::Skip), None);
        let alone = preview(&single);
        assert!(!alone.sequence_added);
        assert_eq!(files(&alone, &out), ["final.png"]);
        // A variable in the path is the person's own way to tell the names apart, and
        // `$(SEQUENCE)` is one: nothing is added to it.
        let path = path_in(&out, "$(SEQUENCE)_$(FILE_NAME)");
        let own = run(raws, settings(&path, OnConflict::Skip), None);
        let given = preview(&own);
        assert!(!given.sequence_added);
        assert_eq!(
            files(&given, &out),
            ["0001_a.png", "0002_b.png", "0003_c.png"]
        );
    }

    #[test]
    fn the_files_already_there_and_what_each_on_conflict_does_with_them() {
        let out = scratch("conflict");
        // a and c are exported before; c's twice.
        for name in ["a.png", "c.png", "c_01.png"] {
            fs::write(out.join(name), b"there").unwrap();
        }
        let raws = photos(&out, &["a.ARW", "b.ARW", "c.ARW"]);
        let path = path_in(&out, "$(FILE_NAME)");
        let on = |on_conflict| preview(&run(raws.clone(), settings(&path, on_conflict), None));
        let counts = |exist, numbered, overwritten, skipped| Counts {
            exist,
            numbered,
            overwritten,
            skipped,
            unnamed: 0,
        };

        let unique = on(OnConflict::CreateUniqueFilename);
        assert_eq!(files(&unique, &out), ["a_01.png", "b.png", "c_02.png"]);
        let fates: Vec<_> = unique
            .names
            .iter()
            .map(|n| match n.fate {
                Fate::File { exists, action, .. } => (exists, action),
                Fate::NoName => panic!("named"),
            })
            .collect();
        assert_eq!(
            fates,
            [
                (true, Action::Number),
                (false, Action::Write),
                (true, Action::Number)
            ]
        );
        assert_eq!(unique.counts(), counts(2, 2, 0, 0));

        let overwrite = on(OnConflict::Overwrite);
        assert_eq!(files(&overwrite, &out), ["a.png", "b.png", "c.png"]);
        assert_eq!(overwrite.counts(), counts(2, 0, 2, 0));

        let skip = on(OnConflict::Skip);
        assert_eq!(skip.counts(), counts(2, 0, 0, 2));
        let Fate::File { action, .. } = skip.names[0].fate else {
            panic!("named")
        };
        assert_eq!(action, Action::Skip(Why::Exists));

        // The raws have no edit, so they have never changed: the files are kept.
        let changed = on(OnConflict::OverwriteIfChanged);
        assert_eq!(changed.counts(), counts(2, 0, 0, 2));
        let Fate::File { action, .. } = changed.names[2].fate else {
            panic!("named")
        };
        assert_eq!(action, Action::Skip(Why::NotChanged));

        // Nothing was written or made.
        assert_eq!(listed(&out), ["a.png", "c.png", "c_01.png"]);
    }

    #[test]
    fn two_photos_with_one_name_take_it_in_turn() {
        let out = scratch("twins");
        let raws = vec![
            out.join("card1").join("IMG_0001.ARW"),
            out.join("card2").join("IMG_0001.ARW"),
        ];
        let path = path_in(&out, "all/$(FILE_NAME)");
        let on = |on_conflict| preview(&run(raws.clone(), settings(&path, on_conflict), None));
        assert_eq!(
            files(&on(OnConflict::CreateUniqueFilename), &out),
            ["all/IMG_0001.png", "all/IMG_0001_01.png"]
        );
        let skip = on(OnConflict::Skip);
        assert_eq!((skip.counts().exist, skip.counts().skipped), (1, 1));
        assert_eq!(on(OnConflict::Overwrite).counts().overwritten, 1);
        // The first of the two is not on disk: it is the export that takes the name.
        assert!(!out.join("all").exists());
    }

    #[test]
    fn two_photos_differing_only_in_case_are_one_name_where_the_system_folds_case() {
        let out = scratch("case");
        let raws = photos(&out, &["img.ARW", "IMG.ARW"]);
        let path = path_in(&out, "all/$(FILE_NAME)");
        let preview = preview(&run(
            raws,
            settings(&path, OnConflict::CreateUniqueFilename),
            None,
        ));
        let expected = if cfg!(any(target_os = "macos", target_os = "windows")) {
            ["all/img.png", "all/IMG_01.png"]
        } else {
            ["all/img.png", "all/IMG.png"]
        };
        assert_eq!(files(&preview, &out), expected);
    }

    #[test]
    fn a_folder_that_cannot_be_made_or_written_is_told_and_not_made() {
        let out = scratch("folders");
        let raws = photos(&out, &["a.ARW"]);
        let on = |path: &str| preview(&run(raws.clone(), settings(path, OnConflict::Skip), None));
        // Missing: made by the export, from the nearest folder there.
        let missing = on(&path_in(&out, "not/yet/$(FILE_NAME)"));
        assert_eq!(
            missing.folders,
            [FolderCheck {
                folder: out.join("not/yet"),
                preflight: Preflight::Creates { from: out.clone() }
            }]
        );
        // A file in the way.
        fs::write(out.join("file"), b"x").unwrap();
        let blocked = on(&path_in(&out, "file/$(FILE_NAME)"));
        assert_eq!(
            blocked.folders[0].preflight,
            Preflight::Blocked {
                at: out.join("file"),
                why: Blocked::NotAFolder
            }
        );
        // A disk that is not connected.
        let gone = on("/run/media/apertrust/NO-DISK-HERE/out/$(FILE_NAME)");
        assert!(matches!(
            gone.folders[0].preflight,
            Preflight::Blocked {
                why: Blocked::NotConnected,
                ..
            }
        ));
        assert_eq!(listed(&out), ["file"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_nobody_may_write_is_told() {
        use std::os::unix::fs::PermissionsExt;
        let out = scratch("locked");
        let locked = out.join("locked");
        fs::create_dir(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
        let path = path_in(&locked, "new/$(FILE_NAME)");
        let raws = photos(&out, &["a.ARW"]);
        let told = preview(&run(raws, settings(&path, OnConflict::Skip), None));
        let made = locked.join("new").exists();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            told.folders[0].preflight,
            Preflight::Blocked {
                at: locked.clone(),
                why: Blocked::ReadOnly
            }
        );
        assert!(!made);
    }

    #[test]
    fn each_folder_is_checked_once_in_the_order_they_come() {
        let out = scratch("many");
        let path = "$(FILE_FOLDER)/out/$(FILE_NAME)";
        let raws = vec![
            out.join("x").join("a.ARW"),
            out.join("y").join("b.ARW"),
            out.join("x").join("c.ARW"),
        ];
        let folders = preview(&run(raws, settings(path, OnConflict::Skip), None)).folders;
        let seen: Vec<_> = folders.iter().map(|f| f.folder.as_path()).collect();
        assert_eq!(seen, [out.join("x").join("out"), out.join("y").join("out")]);
    }

    #[test]
    fn a_path_that_stops_the_export_is_told_with_its_place_and_the_nearest_name() {
        // D8: an unknown variable, with the nearest name and its place in the text.
        let text = "out/$(FILE_NAM)";
        let PatternError::Unknown {
            name,
            at,
            suggestion,
        } = pattern_of(text)
        else {
            panic!("not an unknown variable");
        };
        assert_eq!((name.as_str(), suggestion), ("FILE_NAM", Some("FILE_NAME")));
        assert_eq!(&text[at], "FILE_NAM");

        // A leading `~` is the home folder to the export, but the place is in the text
        // the person typed.
        let typed = "~/out/$(FILE_NAM)";
        let mut tilde = run(Vec::new(), settings(typed, OnConflict::Skip), None);
        tilde.places.home = Some(PathBuf::from("/home/ana"));
        let Some(PathProblem::Pattern(PatternError::Unknown { at, .. })) = preview(&tilde).problem
        else {
            panic!("no unknown variable");
        };
        assert_eq!(&typed[at], "FILE_NAM");

        // Not closed.
        assert!(matches!(
            pattern_of("out/$(FILE_NAME"),
            PatternError::Unclosed { expected: ')', .. }
        ));
        // A character that is no operation.
        assert!(matches!(
            pattern_of("out/$(FILE_NAME=)"),
            PatternError::NoOperation { found: '=', .. }
        ));
        // Nested too deep.
        let depth = apertrust_export::variables::MAX_DEPTH + 6;
        let deep = format!("{}x{}", "$(FILE_NAME-".repeat(depth), ")".repeat(depth));
        assert!(matches!(pattern_of(&deep), PatternError::TooDeep { .. }));

        // An empty path, and no names with any of them.
        let raws = photos(Path::new("."), &["a.ARW"]);
        let empty = preview(&run(raws.clone(), settings("", OnConflict::Skip), None));
        assert_eq!(empty.problem, Some(PathProblem::Empty));
        let bad = run(raws, settings("out/$(FILE_NAM)", OnConflict::Skip), None);
        assert!(preview(&bad).names.is_empty());
    }

    #[test]
    fn a_path_that_names_no_file_fails_for_that_photo_alone() {
        let out = scratch("noname");
        // A variable with no value here leaves the path empty for every photo.
        let raws = photos(&out, &["a.ARW", "b.ARW"]);
        let bare = preview(&run(
            raws.clone(),
            settings("$(TITLE)", OnConflict::Skip),
            None,
        ));
        assert_eq!(files(&bare, &out), ["(no name)", "(no name)"]);
        assert_eq!(bare.counts().unnamed, 2);
        assert!(bare.folders.is_empty());
        // Ending in a folder, the file name follows it, as darktable's does.
        let path = path_in(&out, "$(TITLE)");
        let folder = preview(&run(raws, settings(&path, OnConflict::Skip), None));
        assert_eq!(files(&folder, &out), ["a.png", "b.png"]);
        assert_eq!(listed(&out), Vec::<String>::new());
    }

    #[test]
    fn the_variables_with_no_value_here_are_listed_once() {
        let out = scratch("nothere");
        let path = path_in(&out, "$(STARS)_$(OPENCL.ACTIVATED)_$(STARS)_$(FILE_NAME)");
        let raws = photos(&out, &["a.ARW"]);
        let preview = preview(&run(raws, settings(&path, OnConflict::Skip), None));
        let names: Vec<_> = preview.not_here.iter().map(|u| u.name).collect();
        assert_eq!(names, ["STARS", "OPENCL.ACTIVATED"]);
        // They expand to nothing, and the export goes on.
        assert_eq!(files(&preview, &out), ["___a.png"]);
    }

    #[test]
    fn a_name_that_needs_the_decode_is_exact_only_for_the_open_photo() {
        let out = scratch("exact");
        let raws = photos(&out, &["a.ARW", "b.ARW"]);
        let path = path_in(&out, "$(EXIF.MODEL)_$(FILE_NAME)");
        let open = active(&raws[0], false);
        let model = run(raws.clone(), settings(&path, OnConflict::Skip), Some(open));
        let preview_of_model = preview(&model);
        assert_eq!(files(&preview_of_model, &out), ["Model_a.png", "_b.png"]);
        let exact: Vec<_> = preview_of_model.names.iter().map(|n| n.exact).collect();
        assert_eq!(exact, [true, false]);
        // A path that asks nothing of the decode is exact for all.
        let path = path_in(&out, "$(FILE_NAME)");
        let plain = run(raws, settings(&path, OnConflict::Skip), None);
        assert!(preview(&plain).names.iter().all(|n| n.exact));
    }

    #[test]
    fn the_path_is_probed_for_what_it_asks_of_a_photo() {
        let uses = |path: &str| {
            let disk = Disk::new(path, OnConflict::Skip, 2, None).expect("a pattern");
            let Uses { exif, decoded } = Uses::of(&disk, "png");
            (exif, decoded)
        };
        assert_eq!(uses("/o/$(FILE_NAME)"), (false, false));
        assert_eq!(uses("/o/$(FILE_FOLDER)/$(SEQUENCE)"), (false, false));
        assert_eq!(uses("/o/$(EXIF.YEAR)/$(FILE_NAME)"), (true, false));
        assert_eq!(uses("/o/$(EXIF.EXPOSURE)"), (true, false));
        assert_eq!(uses("/o/$(EXIF.MODEL)"), (false, true));
        assert_eq!(uses("/o/$(EXIF.ISO)"), (false, true));
        assert_eq!(uses("/o/$(WIDTH.SENSOR)x$(EXIF.YEAR)"), (true, true));
        // A value the path does not let through does not count.
        assert_eq!(uses("/o/$(EXIF.MODEL:0:0)$(FILE_NAME)"), (false, false));
    }

    #[test]
    fn the_open_photos_size_is_reported_and_a_size_that_leaves_no_image_is_a_problem() {
        let out = scratch("size");
        let raws = photos(&out, &["a.ARW", "b.ARW"]);
        let path = path_in(&out, "$(FILE_NAME)");
        let size_of = |request: Request, active: Option<Active>| {
            let mut settings = settings(&path, OnConflict::Skip);
            settings.size = request;
            preview(&run(raws.clone(), settings, active)).size
        };
        let open = || Some(active(&raws[0], false));
        let within = |side| Request {
            width: side,
            height: side,
            ..Request::default()
        };
        // Full size.
        let full = size_of(Request::default(), open()).expect("a size");
        assert_eq!(full.source, raws[0]);
        let full = full.size.expect("full size");
        assert_eq!([full.width, full.height], full.extent);
        assert_eq!(full.scale, 1.0);
        // Within 48 pixels.
        let small = size_of(within(48), open()).unwrap().size.unwrap();
        assert_eq!(small.extent, full.extent);
        assert_eq!(small.width.max(small.height), 48);
        assert!(small.scale < 1.0);
        // Not allowed to upscale: 10 000 pixels is more than the photo has.
        let kept = size_of(within(10_000), open()).unwrap().size.unwrap();
        assert_eq!([kept.width, kept.height], full.extent);
        // A side that leaves no pixel.
        let none = size_of(within(1), open()).unwrap().size;
        assert!(matches!(none, Err(SizeProblem::Empty { .. })), "{none:?}");
        // No open photo among them: no size, since another photo's needs a decode.
        assert_eq!(size_of(Request::default(), None), None);
        let elsewhere = Some(active(&out.join("other.ARW"), false));
        assert_eq!(size_of(Request::default(), elsewhere), None);
    }

    #[test]
    fn a_size_larger_than_the_format_holds_is_told_as_such() {
        let error = SizeError::Size(apertrust_export::Error::TooLarge {
            format: "JPEG",
            largest: 65535,
            width: 70_000,
            height: 50_000,
        });
        assert_eq!(
            size_problem(error),
            SizeProblem::TooLarge {
                format: "JPEG",
                largest: 65535,
                width: 70_000,
                height: 50_000
            }
        );
        let other = SizeError::Size(apertrust_export::Error::Quality(3));
        assert!(matches!(size_problem(other), SizeProblem::Failed(_)));
    }

    #[test]
    fn overwrite_if_changed_follows_the_open_photos_unsaved_edit() {
        let out = scratch("changed");
        let raws = photos(&out, &["a.ARW"]);
        fs::write(out.join("a.png"), b"there").unwrap();
        let path = path_in(&out, "$(FILE_NAME)");
        let on = |active| {
            let settings = settings(&path, OnConflict::OverwriteIfChanged);
            preview(&run(raws.clone(), settings, active)).counts()
        };
        // Saved, with no edit file: never changed, so the file is kept.
        assert_eq!(on(Some(active(&raws[0], false))).skipped, 1);
        // An edit not written yet changed just now, after the file was written.
        assert_eq!(on(Some(active(&raws[0], true))).overwritten, 1);
    }

    #[test]
    fn a_cancel_asked_between_photos_ends_the_preview_with_no_answer() {
        let out = scratch("cancel");
        let path = path_in(&out, "$(FILE_NAME)");
        let raws = photos(&out, &["a.ARW", "b.ARW", "c.ARW"]);
        let run = run(raws, settings(&path, OnConflict::Skip), None);
        let asked = std::cell::Cell::new(0);
        let cancelled = || {
            asked.set(asked.get() + 1);
            asked.get() > 1
        };
        assert_eq!(run.preview(cancelled), None);
    }

    fn gpu() -> GpuContext {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    }

    fn synthetic(gpu: &GpuContext, _: &Path) -> Result<Decoded, SessionError> {
        Ok(Decoded {
            camera: "Synthetic".to_owned(),
            image: modules::image(),
            input: GpuImage::upload_mosaic(gpu, 96, 72, &modules::mosaic()).expect("it uploads"),
            shot: Shot::default(),
            decode_ms: 0.0,
            upload_ms: 0.0,
        })
    }

    /// A session with the wake its client waits on.
    fn session(gpu: &GpuContext) -> (Session, Receiver<()>) {
        let (woken, wakes) = mpsc::channel();
        let wake = move || {
            let _ = woken.send(());
        };
        let session = Session::spawn(
            gpu.clone(),
            OPERATIONS,
            MODULES,
            "0.4.0+12~g1a2b3c4",
            pipeline::BUDGET,
            Arc::new(wake),
            synthetic,
        );
        (session, wakes)
    }

    fn previews(events: &[Event]) -> Vec<(u64, &ExportPreview)> {
        events
            .iter()
            .filter_map(|event| match event {
                Event::ExportPreview { through, preview } => Some((*through, &**preview)),
                _ => None,
            })
            .collect()
    }

    fn command(photos: &[PathBuf], path: &str) -> Command {
        Command::PreviewExport {
            photos: photos.to_vec(),
            settings: settings(path, OnConflict::CreateUniqueFilename),
            places: Places::default(),
        }
    }

    /// Reads events until the session has answered with a preview.
    fn until_answered(session: &Session, wakes: &Receiver<()>, events: &mut Vec<Event>) {
        while previews(events).is_empty() {
            wakes.recv_timeout(TEST_WAIT).expect("a preview answers");
            events.extend(session.events());
        }
    }

    #[test]
    fn an_older_preview_is_cancelled_when_a_newer_one_comes() {
        let gpu = gpu();
        let out = scratch("supersede");
        let raws = photos(&out, &["a.ARW", "b.ARW"]);
        let (session, wakes) = session(&gpu);
        let mut events = Vec::new();

        // The first preview is under way, held.
        let (hold, began, permit) = Hold::new();
        session.send(Command::HoldPreview(hold));
        let old = path_in(&out, "old/$(FILE_NAME)");
        let first = session.send(command(&raws, &old));
        began
            .recv_timeout(TEST_WAIT)
            .expect("the first preview begins");
        // A newer one cancels it, and is the one answered.
        let new = path_in(&out, "new/$(FILE_NAME)");
        let second = session.send(command(&raws, &new));
        assert!(second > first);
        until_answered(&session, &wakes, &mut events);
        // Let the first go on: it sees the cancel and answers nothing.
        permit.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        events.extend(session.events());
        let answers = previews(&events);
        assert_eq!(answers.len(), 1, "the cancelled preview sent an answer");
        let (through, preview) = answers[0];
        assert_eq!(through, second);
        assert!(files(preview, &out)[0].starts_with("new/"));
        // Nothing was written or made.
        assert_eq!(listed(&out), Vec::<String>::new());
    }

    #[test]
    fn dropping_the_session_while_a_preview_is_held_waits_for_it_and_ends() {
        let gpu = gpu();
        let out = scratch("dropped");
        let raws = photos(&out, &["a.ARW", "b.ARW"]);
        let (session, _wakes) = session(&gpu);
        let (hold, began, permit) = Hold::new();
        session.send(Command::HoldPreview(hold));
        let path = path_in(&out, "export/$(FILE_NAME)");
        session.send(command(&raws, &path));
        began.recv_timeout(TEST_WAIT).expect("the preview begins");
        // The session ends on a thread of its own, since it waits for the preview (J9).
        let (ended, ends) = mpsc::channel();
        let dropping = std::thread::spawn(move || {
            drop(session);
            let _ = ended.send(());
        });
        assert!(
            ends.recv_timeout(Duration::from_millis(300)).is_err(),
            "the session ended with a preview running"
        );
        // Let the preview go on: the session ends, with no hang.
        permit.send(()).unwrap();
        ends.recv_timeout(TEST_WAIT)
            .expect("the session ends once the preview has");
        dropping.join().expect("the drop does not panic");
        // Nothing was written or made.
        assert_eq!(listed(&out), Vec::<String>::new());
    }

    #[test]
    fn the_session_answers_for_the_open_photo_and_writes_nothing() {
        let gpu = gpu();
        let out = scratch("session");
        let raw = out.join("a.ARW");
        let (session, wakes) = session(&gpu);
        session.send(Command::Resize {
            width: 64,
            height: 48,
        });
        session.send(Command::Open(raw.clone()));
        let path = path_in(&out, "export/$(FILE_NAME)");
        let sent = session.send(command(&[raw.clone(), out.join("b.ARW")], &path));
        let mut events = Vec::new();
        until_answered(&session, &wakes, &mut events);
        let answers = previews(&events);
        let (through, preview) = answers[0];
        assert_eq!(through, sent);
        assert_eq!(files(preview, &out), ["export/a.png", "export/b.png"]);
        assert_eq!(preview.size.as_ref().map(|s| &s.source), Some(&raw));
        assert!(matches!(
            preview.folders[0].preflight,
            Preflight::Creates { .. }
        ));
        // The folder is not made, and no edit or file is written.
        assert!(!out.join("export").exists());
        assert!(listed(&out).iter().all(|name| !name.ends_with(".png")));
    }
}
