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

//! Where a source's photos may have been imported before, told by the destination alone
//! (decision 85; `docs/library.md`, Import in folder mode): no record is kept, and nothing
//! is written anywhere.
//!
//! - **The trees** ([`trees`]): Folder's pattern, `import.base_folder`, expanded up to its
//!   first variable that is not the person's own folder or name ([`tree_root`]), and the
//!   default folder's, `import.default_folder`, wherever Folder's tree does not already
//!   hold it; a tree inside the other is walked once, as the outer one.
//! - **Candidates** ([`candidates`]): each tree walked for its raws' names and sizes, as
//!   the tree's walk reads entries ([`crate::walk`]), with a size added; a raw of a source
//!   photo's size whose name is the photo's own, or whose `.apxmp` keeps the photo's name in
//!   `xmpMM:PreservedFileName`, read only for a raw whose size matches. The source's own
//!   folders are left out, so a source inside a tree never matches itself. A tree that is
//!   not there holds nothing.
//!
//! The fingerprint that confirms a candidate is the operations' (`ops::check_source`).
//! The trees are walked on a thread of their own, so a share that hangs holds that thread
//! alone, never the caller, which returns once cancelled.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use apertrust_base::variables::{Pattern, Places, Values};

use crate::listing::{Kind, is_raw, kind};
use crate::sidecar::{Edits, SUFFIX};

/// How often the caller asks its cancel while it waits on the walking thread.
const CANCEL_POLL: Duration = Duration::from_millis(20);

/// The folder a folder pattern's tree starts at: `pattern` expanded from `places` up to
/// its first variable that is not the person's own folder or name, cut back to the last
/// whole folder before it, a leading `~` the home folder. `None` where that is no absolute
/// folder, where the pattern cannot be read, and where it is the file system's root, which
/// would walk the whole system: `/` on macOS and Linux, the system drive on Windows, the
/// drive that holds Windows (`%SystemDrive%`, `C:` where it is not set). Any other drive's
/// root or volume, a card's or a backup disk's, is a tree as any folder is.
pub fn tree_root(pattern: &str, places: &Places) -> Option<PathBuf> {
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_owned());
    tree_root_as(pattern, places, cfg!(windows), &system_drive)
}

/// [`tree_root`] as on Windows where `windows`, its system drive `system_drive`, else as on
/// macOS and Linux, its folders told by their text alone, so each system's are tested on
/// every system.
fn tree_root_as(
    pattern: &str,
    places: &Places,
    windows: bool,
    system_drive: &str,
) -> Option<PathBuf> {
    let separator = |c: char| c == '/' || (windows && c == '\\');
    let lossy =
        |path: &Option<PathBuf>| (path.as_ref()).map(|path| path.to_string_lossy().into_owned());
    // darktable's own fallback (`_init_expansion`): the home's Pictures where the system
    // names no Pictures folder.
    let pictures = (places.pictures.clone())
        .or_else(|| places.home.as_ref().map(|home| home.join("Pictures")));
    let values = Values {
        home: lossy(&places.home).unwrap_or_default(),
        pictures: lossy(&pictures).unwrap_or_default(),
        desktop: lossy(&places.desktop),
        user_name: places.user_name.clone(),
        ..Values::default()
    };
    let (mut text, whole) = Pattern::parse(pattern).ok()?.expand_fixed(&values);
    if let (Some(rest), Some(home)) = (text.strip_prefix('~'), &places.home)
        && rest.chars().next().is_none_or(separator)
    {
        text = format!("{}{rest}", home.to_string_lossy());
    }
    // A drive's letter, `D:`, and nothing else.
    let drive = |text: &str| {
        let bytes = text.as_bytes();
        windows && bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    };
    if !whole {
        let end = text.rfind(separator)?;
        // Cut back to a root, `/` or `D:\`, its separator is kept.
        let kept = &text[..end];
        let end = if kept.is_empty() || drive(kept) {
            end + 1
        } else {
            end
        };
        text.truncate(end);
    }
    let absolute = if windows {
        let bytes = text.as_bytes();
        let on_drive = text.get(..2).is_some_and(drive)
            && bytes
                .get(2)
                .is_some_and(|&byte| separator(char::from(byte)));
        let share = text.starts_with("//") || text.starts_with("\\\\");
        on_drive || share
    } else {
        text.starts_with('/')
    };
    let bare = text.trim_end_matches(separator);
    let system_root = if windows {
        bare.eq_ignore_ascii_case(system_drive.trim_end_matches(separator))
    } else {
        bare.is_empty()
    };
    (absolute && !system_root).then(|| PathBuf::from(text))
}

/// The trees the check walks, Folder's first: the trees of `base_folder` and
/// `default_folder` ([`tree_root`]), the default folder's left out where Folder's holds it,
/// and Folder's where it lies inside the default folder's.
pub fn trees(base_folder: &str, default_folder: &str, places: &Places) -> Vec<PathBuf> {
    let base = tree_root(base_folder, places);
    let default = tree_root(default_folder, places);
    match (base, default) {
        (Some(base), Some(default)) if default.starts_with(&base) => vec![base],
        (Some(base), Some(default)) if base.starts_with(&default) => vec![default],
        (base, default) => base.into_iter().chain(default).collect(),
    }
}

/// A file of the trees that may be a source photo's copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: PathBuf,
    /// Whether its data is on the disk: a cloud file not on the disk is never
    /// fingerprinted (decision 53).
    pub local: bool,
}

/// What the walk looks for: each source photo's file name and size, by its place in the
/// listing.
#[derive(Debug, Clone, Default)]
pub struct Sought {
    count: usize,
    sizes: HashSet<u64>,
    /// The names alone, so a raw of no name sought, with no sidecar to keep one, is not
    /// measured.
    named: HashSet<String>,
    names: HashMap<(String, u64), Vec<usize>>,
}

impl Sought {
    /// The photos named and measured `photos`, in the listing's order.
    pub fn new<'a>(photos: impl IntoIterator<Item = (&'a str, u64)>) -> Self {
        let mut sought = Sought::default();
        for (at, (name, size)) in photos.into_iter().enumerate() {
            sought.count = at + 1;
            sought.sizes.insert(size);
            sought.named.insert(name.to_owned());
            let key = (name.to_owned(), size);
            sought.names.entry(key).or_default().push(at);
        }
        sought
    }
}

/// Each sought photo's candidates in `trees`, in the listing's order, in the order the walk
/// found them, the raws of its own name first in each folder: the trees walked breadth
/// first on a thread of their own, hidden folders and links left out, on Unix no folder on
/// another device walked into, as the tree's walk; `source` the folder listed and
/// `listed` the folders whose files the source's listing took, whose files are left out.
/// [`Walked::Cancelled`] once `cancel` answers true, returning without waiting for a call
/// into a tree that has not returned; [`Walked::Failed`] where the walk could not start or
/// ended without an answer, a panic. With no photo sought, no tree is walked.
pub fn candidates(
    trees: &[PathBuf],
    source: &Path,
    listed: &[PathBuf],
    sought: Sought,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Walked {
    if cancel() {
        return Walked::Cancelled;
    }
    // No photo sought, nothing to look for: no tree is walked.
    if sought.count == 0 {
        return Walked::Found(Vec::new());
    }
    let stop = Arc::new(AtomicBool::new(false));
    let (sent, answer) = mpsc::sync_channel(1);
    let reading = {
        let (trees, stop) = (trees.to_vec(), Arc::clone(&stop));
        let (source, listed) = (source.to_owned(), listed.to_vec());
        thread::Builder::new()
            .name("import check".to_owned())
            .spawn(move || {
                let stopped = || stop.load(Ordering::Relaxed);
                // Compared as the system resolves them, so a source named through a link to
                // a tree still never matches itself: resolved here, so a source on a share
                // that hangs holds this thread alone.
                let canonical =
                    |root: &Path| fs::canonicalize(root).unwrap_or_else(|_| root.to_owned());
                let source_root = canonical(&source);
                let skipped: HashSet<PathBuf> = (listed.iter())
                    .map(|folder| source_root.join(folder.strip_prefix(&source).unwrap_or(folder)))
                    .collect();
                let mut found = vec![Vec::new(); sought.count];
                for tree in &trees {
                    let canonical_tree = canonical(tree);
                    let walked = Tree {
                        root: tree,
                        canonical: &canonical_tree,
                        skipped: &skipped,
                        sought: &sought,
                    }
                    .walk(&mut found, &stopped);
                    if !walked {
                        return;
                    }
                }
                let _ = sent.send(found);
            })
    };
    if let Err(error) = reading {
        tracing::warn!("the check against the destination cannot start: {error}");
        return Walked::Failed;
    }
    // Dropped, as the call returns whichever way, it tells the thread to stop.
    let _stop = Stop(stop);
    loop {
        if cancel() {
            return Walked::Cancelled;
        }
        match answer.recv_timeout(CANCEL_POLL) {
            Ok(found) => return Walked::Found(found),
            Err(RecvTimeoutError::Timeout) => {}
            // The thread ended without an answer, though it was not told to stop: it
            // panicked.
            Err(RecvTimeoutError::Disconnected) => return Walked::Failed,
        }
    }
}

/// How [`candidates`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Walked {
    /// Each sought photo's candidates, in the listing's order.
    Found(Vec<Vec<Candidate>>),
    /// Cancelled: nothing more is to be sent.
    Cancelled,
    /// The walk could not start, or ended without an answer.
    Failed,
}

/// Tells the walking thread to stop as it is dropped.
struct Stop(Arc<AtomicBool>);

impl Drop for Stop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// One tree to walk for candidates.
struct Tree<'a> {
    root: &'a Path,
    /// Its root as the system resolves it, which the folders `skipped` are compared under.
    canonical: &'a Path,
    skipped: &'a HashSet<PathBuf>,
    sought: &'a Sought,
}

impl Tree<'_> {
    /// Walks the tree breadth first, adding each candidate to its photo's in `found`;
    /// false once `stopped` answers true.
    fn walk(&self, found: &mut [Vec<Candidate>], stopped: &dyn Fn() -> bool) -> bool {
        let device = crate::walk::device(self.root);
        let mut queue = VecDeque::from([self.root.to_owned()]);
        while let Some(folder) = queue.pop_front() {
            if stopped() {
                return false;
            }
            #[cfg(test)]
            crash::at(&folder);
            let relative = folder.strip_prefix(self.root).unwrap_or(&folder);
            let own = self.skipped.contains(&self.canonical.join(relative));
            let entries = match fs::read_dir(&folder) {
                Ok(entries) => entries,
                // A tree not there yet holds nothing; a folder that cannot be read, nothing
                // known.
                Err(error) => {
                    tracing::debug!("{}: not checked: {error}", folder.display());
                    continue;
                }
            };
            let mut raws = Vec::new();
            let mut sidecars = HashSet::new();
            for entry in entries {
                if stopped() {
                    return false;
                }
                let Ok(entry) = entry else {
                    break;
                };
                let Ok(name) = entry.file_name().into_string() else {
                    continue;
                };
                match kind(&entry, &name) {
                    Kind::Folder { link: false } if crate::walk::same_device(&entry, device) => {
                        queue.push_back(folder.join(&name));
                    }
                    // The source's own files are never its copies.
                    Kind::File if own => {}
                    Kind::File if is_raw(&name) => raws.push((name, entry)),
                    Kind::File if name.ends_with(SUFFIX) => {
                        sidecars.insert(name);
                    }
                    Kind::Folder { .. } | Kind::File | Kind::Hidden | Kind::Unreadable => {}
                }
            }
            for (name, entry) in raws {
                self.raw(&folder, &name, &entry, &sidecars, found);
            }
        }
        true
    }

    /// Adds the raw `name` of `folder` to the candidates of each sought photo it may be a
    /// copy of: of its size, under the photo's name or the one its `.apxmp`, among
    /// `sidecars`, keeps. A raw of no name sought with no sidecar is not measured, and its
    /// sidecar is read only where its size is sought.
    fn raw(
        &self,
        folder: &Path,
        name: &str,
        entry: &fs::DirEntry,
        sidecars: &HashSet<String>,
        found: &mut [Vec<Candidate>],
    ) {
        let sidecar = format!("{name}{SUFFIX}");
        let with_sidecar = sidecars.contains(&sidecar);
        if !with_sidecar && !self.sought.named.contains(name) {
            return;
        }
        let path = folder.join(name);
        let meta = match entry.metadata() {
            Ok(meta) if meta.file_type().is_symlink() => fs::metadata(&path),
            meta => meta,
        };
        let Ok(meta) = meta else {
            return;
        };
        let size = meta.len();
        if !meta.is_file() || !self.sought.sizes.contains(&size) {
            return;
        }
        let local = crate::listing::is_local(&meta);
        #[cfg(any(test, feature = "testing"))]
        let local = local && !crate::testing::evicted(&path);
        let mut names = vec![name.to_owned()];
        if with_sidecar
            && let Some(preserved) = preserved_name(&folder.join(sidecar))
            && preserved != name
        {
            names.push(preserved);
        }
        for name in names {
            let photos = self.sought.names.get(&(name, size));
            for &at in photos.into_iter().flatten() {
                if !found[at].iter().any(|candidate| candidate.path == path) {
                    found[at].push(Candidate {
                        path: path.clone(),
                        local,
                    });
                }
            }
        }
    }
}

/// The camera name the `.apxmp` at `path` keeps, its `xmpMM:PreservedFileName`; none where
/// it holds none or cannot be read.
fn preserved_name(path: &Path) -> Option<String> {
    #[cfg(any(test, feature = "testing"))]
    crate::testing::read(path);
    let xml = fs::read_to_string(path).ok()?;
    // The modified time dates a version 1 file's steps alone, which the name never reads.
    Edits::parse(&xml, 0).ok()?.preserved_file_name()
}

/// A test's hook: the walk panics as it comes to a folder marked here, as a bug would.
#[cfg(test)]
pub(crate) mod crash {
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, PoisonError};

    static MARKED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

    /// Makes the walk panic at `folder`.
    pub(crate) fn mark(folder: &Path) {
        (MARKED.lock().unwrap_or_else(PoisonError::into_inner)).push(folder.to_owned());
    }

    /// Panics where `folder` is marked.
    pub(crate) fn at(folder: &Path) {
        let marked = MARKED.lock().unwrap_or_else(PoisonError::into_inner);
        let hit = marked.iter().any(|marked| marked == folder);
        drop(marked);
        assert!(!hit, "a test's crash at {}", folder.display());
    }
}

#[cfg(test)]
pub(crate) mod tests;
