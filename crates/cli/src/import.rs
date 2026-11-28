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

//! `import`: Import's copy from a card or a folder, run in this process with no session and
//! no GPU, as `library scan` is (decision 85; `docs/library.md`, The CLI): the listing, the
//! check against the destination, the preview and the copy are the bodies the session's
//! jobs run. Only new photos are copied, under their camera names, `_01` where a name is
//! taken; the problems the preview finds refuse the run before anything is copied.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Result, anyhow, bail};
use apertrust_base::settings::Store;
use apertrust_library::ops::{
    self as library, Fingerprints, Fix, ImportCause, ImportFate, ImportPhotos, ImportPreview,
    ImportProblem, ImportResult, ImportSettings, ImportStep, LibraryEvent, OnSource, Places,
    Problem, Row, SourceState, Trees, Unwritable,
};
use apertrust_library::source::Listed;

use crate::cache::size;

/// `import SOURCE` and its flags.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Where to import from, a folder: a card, its root or any folder inside it, whose
    /// `DCIM` is listed whole, or any other folder, its own raws only.
    #[arg(value_name = "SOURCE")]
    pub source: PathBuf,
    /// Folder: the folder the photos go under, a pattern of darktable's variables such as
    /// `$(PICTURES_FOLDER)/Apertrust`, a leading ~ the home folder; import.base_folder for
    /// this run.
    #[arg(long, value_name = "DIR")]
    pub folder: Option<String>,
    /// Subfolder: worked out for each photo, such as `$(YEAR)$(MONTH)$(DAY)_$(IMPORT_NAME)`
    /// or `$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(IMPORT_NAME)` for one folder each day
    /// taken; empty puts the photos straight into the folder. import.subfolder for this run.
    #[arg(long, value_name = "PATTERN")]
    pub subfolder: Option<String>,
    /// Import name, which `$(IMPORT_NAME)` and `$(JOBCODE)` expand to; import.job_code for
    /// this run.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// Print where each photo would go, what would be skipped, the folders and the size,
    /// and copy nothing.
    #[arg(long)]
    pub dry_run: bool,
}

/// Runs `import` with the settings `store` holds, the flags over them, and the person's
/// `places`. Fails, exiting with status 1, where the source cannot be listed, a problem
/// refuses the run, a photo fails or the copy stops, and, once the photos read are copied,
/// where a folder of the source could not be read.
pub fn run(args: Args, store: &Store, places: Places) -> Result<()> {
    let mut settings = ImportSettings::read(store, places);
    if let Some(folder) = args.folder {
        settings.base_folder = folder_pattern(&folder, cfg!(windows));
    }
    if let Some(subfolder) = args.subfolder {
        settings.subfolder = subfolder;
    }
    if let Some(name) = args.name {
        settings.job_code = name;
    }
    let source = std::path::absolute(&args.source)
        .map_err(|error| anyhow!("{}: {error}", args.source.display()))?;
    let shown = Shown {
        source: source.clone(),
        home: settings.places.home.clone(),
    };
    let fingerprints = Arc::new(Fingerprints::default());
    let start = Instant::now();
    let listed = list(&source)?;
    let listed_ms = start.elapsed().as_secs_f64() * 1e3;
    let start = Instant::now();
    let states = check(&listed, &settings, &fingerprints);
    let checked_ms = start.elapsed().as_secs_f64() * 1e3;
    let count = listed.files.len();
    tracing::debug!(target: "perf", "import: listed {count} raws in {listed_ms:.0} ms, checked them in {checked_ms:.0} ms");
    let new: Vec<PathBuf> = (listed.files.iter().zip(&states))
        .filter(|(_, state)| matches!(state, SourceState::New))
        .map(|(file, _)| file.path.clone())
        .collect();
    let before = count - new.len();
    let photos = ImportPhotos {
        folder: listed.folder.clone(),
        files: new,
    };
    let now = apertrust_library::destination::local(std::time::SystemTime::now());
    let preview = preview(&photos, &settings, now, &fingerprints);
    if !preview.problems.is_empty() {
        for problem in &preview.problems {
            eprintln!("{}", problem_line(problem, &shown));
        }
        bail!("nothing imported");
    }
    let unread = listed.left_out.unreadable.len();
    let all_read = || match unread {
        0 => Ok(()),
        _ => Err(anyhow!("{unread} folders of the source not read")),
    };
    if args.dry_run {
        print!("{}", dry_run(&preview, &photos, before, &shown));
        return all_read();
    }
    if photos.files.is_empty() {
        println!("Nothing new to import; {before} imported before left out");
        return all_read();
    }
    let start = Instant::now();
    let end = copy(&photos, &settings, now, &shown);
    let copied_ms = start.elapsed().as_secs_f64() * 1e3;
    let copied = end.copied;
    tracing::debug!(target: "perf", "import: copied {copied} photos in {copied_ms:.0} ms");
    println!("{}", copied_line(&end, before, &shown));
    if let Some(refused) = &end.refused {
        bail!("refused: {}", refused.reason);
    }
    if let Some(failure) = &end.failure {
        let left = end.left.len();
        bail!("{} photos not imported: {}", left, failure.reason);
    }
    all_read()
}

/// `--folder`'s text as a pattern: on Windows, where a folder is written with `\`, which a
/// pattern reads as an escape, each `\` a `/`, as `export`'s `-o` takes a path.
fn folder_pattern(folder: &str, windows: bool) -> String {
    if windows {
        folder.replace('\\', "/")
    } else {
        folder.to_owned()
    }
}

/// How paths are shown: a photo by its path under the source asked for, where it lies
/// there, a destination with the home folder as `~`.
struct Shown {
    source: PathBuf,
    home: Option<PathBuf>,
}

impl Shown {
    fn photo(&self, folder: &Path, file: &Path) -> String {
        let path = folder.join(file);
        let shown = path.strip_prefix(&self.source).unwrap_or(file);
        forward(shown)
    }

    fn destination(&self, path: &Path) -> String {
        match self
            .home
            .as_deref()
            .and_then(|home| path.strip_prefix(home).ok())
        {
            Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
            Some(rest) => format!("~/{}", forward(rest)),
            None => path.display().to_string(),
        }
    }
}

/// A relative path with `/` between its parts, as the issue's lines show them.
fn forward(path: &Path) -> String {
    let parts: Vec<_> = path.iter().map(|part| part.to_string_lossy()).collect();
    parts.join("/")
}

/// The source listed; the folders that could not be read named on stderr. An error where
/// it cannot be listed.
fn list(source: &Path) -> Result<Listed> {
    let failed = Mutex::new(None);
    let tell = |event| {
        if let LibraryEvent::SourceFailed { reason, .. } = event {
            *failed.lock().unwrap() = Some(reason);
        }
    };
    let listed = library::list_source(0, source, false, &tell, &|| false);
    if let Some(reason) = failed.into_inner().unwrap() {
        bail!("{reason}");
    }
    let listed = listed.ok_or_else(|| anyhow!("{}: the listing stopped", source.display()))?;
    for unreadable in &listed.left_out.unreadable {
        let folder = listed.folder.join(&unreadable.folder);
        eprintln!("{}: not read: {}", folder.display(), unreadable.reason);
    }
    Ok(listed)
}

/// Each photo of `listed` told new or imported before, against Folder's and the default
/// folder's trees.
fn check(
    listed: &Listed,
    settings: &ImportSettings,
    fingerprints: &Arc<Fingerprints>,
) -> Vec<SourceState> {
    let trees = Trees {
        base_folder: settings.base_folder.clone(),
        default_folder: settings.default_folder.clone(),
        places: settings.places.clone(),
    };
    let states = Mutex::new(Vec::new());
    let tell = |event| {
        if let LibraryEvent::SourceChecked { states: chunk, .. } = event {
            states.lock().unwrap().extend(chunk);
        }
    };
    library::check_source(0, 1, listed, &trees, fingerprints, &tell, &|| false);
    states.into_inner().unwrap()
}

/// The preview of `photos`.
fn preview(
    photos: &ImportPhotos,
    settings: &ImportSettings,
    now: apertrust_base::variables::DateTime,
    fingerprints: &Arc<Fingerprints>,
) -> ImportPreview {
    let found = Mutex::new(None);
    let tell = |event| {
        if let LibraryEvent::ImportPreview { preview, .. } = event {
            *found.lock().unwrap() = Some(*preview);
        }
    };
    library::preview_import(0, photos, settings, now, fingerprints, &tell, &|| false);
    found.into_inner().unwrap().unwrap_or_default()
}

/// A problem's line: its row, what is wrong and its fix.
fn problem_line(problem: &ImportProblem, shown: &Shown) -> String {
    let row = match problem.row {
        Row::Folder => "Folder",
        Row::Subfolder => "Subfolder",
    };
    let what = match &problem.problem {
        Problem::Pattern(apertrust_base::variables::PatternError::Unknown {
            name,
            suggestion,
            ..
        }) => {
            let near =
                suggestion.map_or_else(String::new, |near| format!(". Did you mean $({near})?"));
            format!("unknown variable $({name}){near}")
        }
        Problem::Pattern(error) => error.to_string(),
        Problem::OutsideFolder { subfolder } => format!("{subfolder} leads out of Folder"),
        Problem::Unwritable { at, why } => {
            let why = match why {
                Unwritable::NotAbsolute => "is no absolute folder".to_owned(),
                Unwritable::NotConnected => "is on a disk not connected".to_owned(),
                Unwritable::NotAFolder => "is no folder".to_owned(),
                Unwritable::ReadOnly => "cannot be written".to_owned(),
                Unwritable::Unreadable(kind) => format!("cannot be read: {kind}"),
            };
            format!("{} {why}", shown.destination(at))
        }
        Problem::OnSource { folder, why } => {
            let why = match why {
                OnSource::IsSource => "is the source".to_owned(),
                OnSource::InsideSource => "lies inside the source".to_owned(),
                OnSource::OnCard { card } => format!("is on the card {}", card.display()),
            };
            format!("{} {why}", shown.destination(folder))
        }
        Problem::Space {
            needed,
            available,
            volume,
        } => format!(
            "needs {}; {} is free on {volume}",
            size(*needed),
            size(*available)
        ),
    };
    // "Did you mean $(MONTH)?" is the fix to use already.
    if what.ends_with('?') {
        return format!("{row}: {what}");
    }
    let fix = match &problem.fix {
        Some(Fix::Use { variable }) => format!(" Use $({variable})."),
        Some(Fix::ChooseFolder) => " Choose another folder.".to_owned(),
        None => String::new(),
    };
    format!("{row}: {what}.{fix}")
}

/// `--dry-run`'s lines: each photo and where it goes or why it is skipped, the folders new
/// or there with their counts, then the summary.
fn dry_run(preview: &ImportPreview, photos: &ImportPhotos, before: usize, shown: &Shown) -> String {
    let mut text = String::new();
    for photo in &preview.photos {
        let name = shown.photo(&photos.folder, &photo.source);
        let line = match &photo.fate {
            ImportFate::Copy { to, .. } => format!("{name} → {}", shown.destination(to)),
            ImportFate::There { at } => {
                format!(
                    "{name}: skipped if identical, there already as {}",
                    shown.destination(at)
                )
            }
            ImportFate::Diverged { at, .. } => format!(
                "{name}: skipped if identical, there already with an edit of its own as {}",
                shown.destination(at)
            ),
            ImportFate::Unreadable { reason } => format!("{name}: skipped, {reason}"),
        };
        text.push_str(&line);
        text.push('\n');
    }
    for folder in &preview.folders {
        let name = shown.destination(&folder.folder);
        let photos = folder.photos;
        let line = match folder.there {
            None => format!("new folder: {name}, {photos} to come"),
            Some(there) => format!("folder there: {name}, {there} raws there, {photos} to come"),
        };
        text.push_str(&line);
        text.push('\n');
    }
    let counts = preview.counts();
    let mut summary = format!("{} to copy, {}", counts.copied, size(preview.size));
    if let Some(free) = &preview.free {
        summary.push_str(&format!(
            "; {} free on {}",
            size(free.available),
            free.volume
        ));
    }
    summary.push_str(&format!("; {before} imported before left out"));
    text.push_str(&summary);
    text.push('\n');
    text
}

/// Copies `photos`, a line on stdout for each as it lands and on stderr for each that
/// fails.
fn copy(
    photos: &ImportPhotos,
    settings: &ImportSettings,
    now: apertrust_base::variables::DateTime,
    shown: &Shown,
) -> library::ImportEnd {
    let tell = |event| {
        let LibraryEvent::Imported { photos: done, .. } = event else {
            return;
        };
        for photo in done {
            let name = shown.photo(&photos.folder, &photo.source);
            match photo.result {
                ImportResult::Copied { copy } => {
                    println!("{name} → {}", shown.destination(&copy));
                }
                ImportResult::There { copy } => {
                    println!(
                        "{name}: skipped, already there as {}",
                        shown.destination(&copy)
                    );
                }
                ImportResult::Diverged { copy, .. } => println!(
                    "{name}: skipped, already there with an edit of its own as {}",
                    shown.destination(&copy)
                ),
                ImportResult::Failed { reason, cause } => {
                    eprintln!("{name}: not imported ({}): {reason}", cause_words(cause));
                }
            }
        }
    };
    let build = apertrust_base::program!().version;
    let step = |_: ImportStep<'_>| {};
    library::run_import(0, photos, settings, now, build, &tell, &step, &|| false)
}

/// A cause in plain words.
fn cause_words(cause: ImportCause) -> &'static str {
    match cause {
        ImportCause::Unreadable => "a file cannot be read",
        ImportCause::Exists => "a file of that name appeared at the destination",
        ImportCause::DiskFull => "the disk is full",
        ImportCause::Gone => "the source or the destination is no longer there",
        ImportCause::Write => "a file could not be written",
        ImportCause::Pattern => "a folder pattern is not valid",
        ImportCause::Folder => "the destination is no absolute folder",
        ImportCause::OnSource => "the destination is on the source",
    }
}

/// The summary once copied: how many landed and where, those imported before left out, and
/// those skipped.
fn copied_line(end: &library::ImportEnd, before: usize, shown: &Shown) -> String {
    let photos = if end.copied == 1 { "photo" } else { "photos" };
    let mut line = format!("Imported {} {photos}", end.copied);
    if let Some(destination) = &end.destination {
        line.push_str(&format!(" into {}", shown.destination(destination)));
    }
    line.push_str(&format!("; {before} imported before left out"));
    if end.there > 0 {
        line.push_str(&format!("; {} skipped: already there", end.there));
    }
    if end.diverged > 0 {
        line.push_str(&format!(
            "; {} skipped: already there with an edit of their own",
            end.diverged
        ));
    }
    if end.failed > 0 {
        line.push_str(&format!("; {} failed", end.failed));
    }
    if end.not_reached > 0 {
        line.push_str(&format!("; {} not reached", end.not_reached));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    /// On Windows a folder's `\` is a `/`, so `D:\Photos\$(YEAR)` keeps its separators and
    /// its variable; elsewhere `\` stays an escape, as patterns read it.
    #[test]
    fn a_windows_folder_s_backslashes_are_separators() {
        assert_eq!(
            folder_pattern(r"D:\Photos\$(YEAR)", true),
            "D:/Photos/$(YEAR)"
        );
        assert_eq!(
            folder_pattern(r"D:/Photos\Wedding", true),
            "D:/Photos/Wedding"
        );
        assert_eq!(
            folder_pattern(r"/photos/\$(YEAR)", false),
            r"/photos/\$(YEAR)"
        );
        let pattern =
            apertrust_base::variables::Pattern::parse(&folder_pattern(r"D:\Photos\$(YEAR)", true));
        assert!(pattern.is_ok());
    }

    #[test]
    fn a_problem_is_said_with_its_row_and_its_fix() {
        let shown = Shown {
            source: PathBuf::from("/Volumes/EOS_DIGITAL"),
            home: Some(PathBuf::from("/home/ana")),
        };
        let unknown = ImportProblem {
            row: Row::Subfolder,
            problem: Problem::Pattern(apertrust_base::variables::PatternError::Unknown {
                name: "MONT".to_owned(),
                at: 9..13,
                suggestion: Some("MONTH"),
            }),
            fix: Some(Fix::Use { variable: "MONTH" }),
        };
        assert_eq!(
            problem_line(&unknown, &shown),
            "Subfolder: unknown variable $(MONT). Did you mean $(MONTH)?"
        );
        let space = ImportProblem {
            row: Row::Folder,
            problem: Problem::Space {
                needed: 6_200_000_000,
                available: 4_100_000_000,
                volume: "Macintosh HD".to_owned(),
            },
            fix: Some(Fix::ChooseFolder),
        };
        assert_eq!(
            problem_line(&space, &shown),
            "Folder: needs 6.2 GB; 4.1 GB is free on Macintosh HD. Choose another folder."
        );
        assert_eq!(
            shown.destination(Path::new("/home/ana/Pictures/Apertrust")),
            "~/Pictures/Apertrust"
        );
        assert_eq!(
            shown.photo(
                Path::new("/Volumes/EOS_DIGITAL/DCIM"),
                Path::new("100CANON/IMG_0001.CR3")
            ),
            "DCIM/100CANON/IMG_0001.CR3"
        );
    }
}
