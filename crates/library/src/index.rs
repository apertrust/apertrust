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

//! The open folder's index, layer 3 of decision 54 in memory (decision 47): the listing
//! ([`crate::listing`]) plus each sidecar read through [`crate::sidecar`], so the folder
//! says which photos have an edit, each sidecar's version and hash, and which open for
//! viewing only (decision 43).
//!
//! The index is plain data, always rebuilt from the files below it. [`Index::build`] is a
//! blocking call a job's body can make: it lists the folder on the listing's worker,
//! then reads sidecars on up to [`READERS`] threads at once, as decision 53 says, so a
//! share that answers slowly is asked many times at once and the interface is never
//! waiting. Cancelling is a closure the build asks between calls into the folder, and
//! while it waits for the listing or sidecar results: a cancelled build returns at
//! once, never waiting for a filesystem call, and marks sidecars whose results have
//! not arrived. Owned readers stop before their next read.
//!
//! A sidecar that cannot be read or parsed is that photo's alone to report
//! ([`Sidecar::Unreadable`]); the rest are indexed. Each photo's rating and fields are read
//! with its sidecar, the first copy's, its name and the count of copies ([`Edit::rating`],
//! [`Edit::fields`]), or, for a photo with no `.apxmp`, from darktable's `.xmp` beside it
//! on the same readers ([`Entry::darktable`], decision 110). Changes made outside are
//! planned.

use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use crate::fields::{Fields, Rating};
use crate::sidecar::{Edits, Newer};

use crate::listing::{Error, Event, FileStat, Folder, Listing, Options, Orphan, Photo, Summary};

/// How many sidecars are read at once (decision 53).
pub const READERS: usize = 16;

/// How often the build looks at the cancel closure while it waits for the listing.
const CANCEL_POLL: Duration = Duration::from_millis(20);

/// What a photo's sidecar turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sidecar {
    /// The photo has no sidecar.
    None,
    /// The sidecar was read.
    Read(Edit),
    /// The sidecar could not be read or is no `.apxmp`; the reason, for the user.
    Unreadable(String),
    /// The build was cancelled before this sidecar was read.
    NotRead,
}

/// What a readable sidecar says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The file's `apertrust:version`.
    pub version: u32,
    /// The BLAKE3 hash of the file's bytes: it changes whenever the file does, so it tells
    /// that the sidecar changed, and means the same on every machine and build. It is not
    /// the key of a thumbnail (planned), which hashes the edit the file holds, so that
    /// another build rewriting the same edit differently does not invalidate them.
    pub file_hash: [u8; 32],
    /// Whether the photo has an edit: some tool's last entry up to a copy's end is not
    /// empty, a fold of its log as text (`docs/history.md`, In the sidecar), or the file is a
    /// newer Apertrust's, whose layout this build may not read and so may not call empty.
    pub edited: bool,
    /// Why the photo opens for viewing only, where it does (decision 43).
    pub view_only: Option<Newer>,
    /// The first virtual copy's rating, stars, reject and colour labels (decision 110).
    pub rating: Rating,
    /// The first virtual copy's fields and name, and how many copies the photo has
    /// (decision 110).
    pub fields: Arc<Fields>,
}

/// One photo of the folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The raw's file name.
    pub name: String,
    pub raw: FileStat,
    /// The sidecar's size and modified time, where it has one.
    pub file: Option<FileStat>,
    pub sidecar: Sidecar,
    /// The rating and fields darktable's `.xmp` gives a photo with no `.apxmp`, its version
    /// name the copy's, where it has one that reads (decision 110); `None` for the rest.
    pub darktable: Option<(Rating, Arc<Fields>)>,
}

impl Entry {
    /// The photo's rating: its sidecar's, else darktable's, else none.
    pub fn rating(&self) -> Rating {
        match (&self.sidecar, &self.darktable) {
            (Sidecar::Read(edit), _) => edit.rating,
            (_, Some((rating, _))) => *rating,
            _ => Rating::default(),
        }
    }

    /// The photo's fields: its sidecar's, else darktable's, else none.
    pub fn fields(&self) -> Arc<Fields> {
        match (&self.sidecar, &self.darktable) {
            (Sidecar::Read(edit), _) => Arc::clone(&edit.fields),
            (_, Some((_, fields))) => Arc::clone(fields),
            _ => Arc::default(),
        }
    }
}

/// What one read of the build gives: a sidecar's, or darktable's rating and fields.
enum Read {
    Sidecar(Sidecar),
    Darktable(Option<(Rating, Arc<Fields>)>),
}

/// What an index counted of the photos' sidecars.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Photos with an edit.
    pub edited: usize,
    /// Photos whose edit opens for viewing only.
    pub view_only: usize,
    /// Photos whose sidecar could not be read.
    pub unreadable: usize,
}

/// The open folder: its photos with their sidecars, in name order.
#[derive(Debug, Clone)]
pub struct Index {
    pub photos: Vec<Entry>,
    pub orphans: Vec<Orphan>,
    /// The subfolders' names, as the system's file browser sorts them
    /// ([`crate::listing::browser_order`]).
    pub subfolders: Vec<String>,
    /// The listing's counts.
    pub summary: Summary,
    /// Whether the build was cancelled, in the listing or among the sidecars: the index
    /// holds what it had reached, and sidecars it had not are [`Sidecar::NotRead`].
    pub cancelled: bool,
    /// How long listing and reading took.
    pub elapsed: Duration,
}

impl Index {
    /// Lists `folder` and reads its sidecars, and waits for both. `cancel` is asked
    /// on the calling thread while it waits for listing and sidecar results. A cancel
    /// returns without joining a blocked read.
    pub fn build(
        folder: &Path,
        options: Options,
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Result<Index, Error> {
        build(folder, options, cancel, read_file)
    }

    /// What the sidecars say, counted.
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for photo in &self.photos {
            match &photo.sidecar {
                Sidecar::Read(edit) => {
                    counts.edited += usize::from(edit.edited);
                    counts.view_only += usize::from(edit.view_only.is_some());
                }
                Sidecar::Unreadable(_) => counts.unreadable += 1,
                Sidecar::None | Sidecar::NotRead => {}
            }
        }
        counts
    }
}

fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// [`Index::build`] with the bytes of a file read by `reader`, replaceable in tests.
fn build(
    folder: &Path,
    options: Options,
    cancel: &(dyn Fn() -> bool + Sync),
    reader: impl Fn(&Path) -> io::Result<Vec<u8>> + Send + Sync + 'static,
) -> Result<Index, Error> {
    let started = Instant::now();
    let listed = list(folder, options, cancel)?;
    let mut cancelled = listed.summary.cancelled;

    // Only photos with a sidecar, or with darktable's, are read; the rest are settled
    // already.
    let mut to_read = Vec::new();
    let mut photos: Vec<Entry> = Vec::with_capacity(listed.photos.len());
    for Photo {
        name,
        raw,
        sidecar,
        darktable,
    } in listed.photos
    {
        let path = match (sidecar, darktable) {
            (Some(_), _) => Some(sidecar_name(&name)),
            (None, Some(_)) => Some(format!("{name}{}", crate::darktable::SUFFIX)),
            (None, None) => None,
        };
        if let Some(path) = path {
            to_read.push((photos.len(), folder.join(path)));
        }
        photos.push(Entry {
            sidecar: if sidecar.is_some() {
                Sidecar::NotRead
            } else {
                Sidecar::None
            },
            name,
            raw,
            file: sidecar,
            darktable: None,
        });
    }

    if !cancelled && !to_read.is_empty() {
        let paths = to_read.iter().map(|(_, path)| path.clone()).collect();
        let (results, stopped) = crate::reads::read(
            paths,
            READERS,
            move |path| {
                panic::catch_unwind(AssertUnwindSafe(|| read_one_of(path, &reader))).unwrap_or_else(
                    |_| Read::Sidecar(Sidecar::Unreadable("reading it panicked".to_owned())),
                )
            },
            cancel,
        );
        for (at, read) in results {
            let photo = &mut photos[to_read[at].0];
            match read {
                Read::Sidecar(sidecar) => photo.sidecar = sidecar,
                Read::Darktable(read) => photo.darktable = read,
            }
        }
        cancelled = stopped;
    }

    let index = Index {
        photos,
        orphans: listed.orphans,
        subfolders: listed.subfolders,
        summary: listed.summary,
        cancelled,
        elapsed: started.elapsed(),
    };
    let counts = index.counts();
    tracing::debug!(
        target: "perf",
        "indexed {}: {} photos, {} edited, {} view-only, {} unreadable sidecars in {:.1} ms{}",
        folder.display(),
        index.photos.len(),
        counts.edited,
        counts.view_only,
        counts.unreadable,
        index.elapsed.as_secs_f64() * 1e3,
        if cancelled { " (cancelled)" } else { "" },
    );
    Ok(index)
}

/// The listing, collected while `cancel` is watched. Once `cancel` answers true it
/// returns at once, what was listed so far with its summary saying cancelled, and drops
/// the listing, which tells its worker to stop without waiting for it: a worker inside a
/// call into a share that does not return holds neither the caller nor a job's worker.
fn list(
    folder: &Path,
    options: Options,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Folder, Error> {
    let listing = Listing::start(folder, options);
    let mut listed = Folder {
        photos: Vec::new(),
        orphans: Vec::new(),
        subfolders: Vec::new(),
        summary: Summary::default(),
        deadline_passed: false,
    };
    loop {
        if cancel() {
            listed.summary.cancelled = true;
            return Ok(listed);
        }
        match listing.events().recv_timeout(CANCEL_POLL) {
            Ok(Event::Chunk(mut photos)) => listed.photos.append(&mut photos),
            Ok(Event::Orphans(mut orphans)) => listed.orphans.append(&mut orphans),
            Ok(Event::Subfolders(mut names)) => listed.subfolders.append(&mut names),
            Ok(Event::DeadlinePassed { .. }) => listed.deadline_passed = true,
            Ok(Event::Done(summary)) => {
                listed.summary = summary;
                return Ok(listed);
            }
            Ok(Event::Failed(error)) => return Err(error),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(Error::Stopped),
        }
    }
}

fn sidecar_name(raw: &str) -> String {
    format!("{raw}{}", crate::sidecar::SUFFIX)
}

/// Reads the sidecar at `path` alone, once, as a listing reads each: none where there is
/// no file, unreadable for any other failure.
pub fn read_one(path: &Path) -> Sidecar {
    match std::fs::read(path) {
        Ok(bytes) => parse_sidecar(&bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Sidecar::None,
        Err(error) => Sidecar::Unreadable(error.to_string()),
    }
}

/// Reads one file of the build: an `.apxmp`, as [`read_sidecar`] does, or darktable's
/// `.xmp`, for its rating and fields; one that cannot be read or is no XMP gives none.
fn read_one_of(path: &Path, reader: &impl Fn(&Path) -> io::Result<Vec<u8>>) -> Read {
    if path
        .as_os_str()
        .to_string_lossy()
        .ends_with(crate::sidecar::SUFFIX)
    {
        return Read::Sidecar(read_sidecar(path, reader));
    }
    let read = reader(path).ok();
    let fields = read.and_then(|bytes| crate::darktable::parse_bytes(path, &bytes));
    Read::Darktable(fields.map(|read| {
        let fields = Fields::held(&read.fields, &read.name, 1);
        (Rating::of(&read.fields), Arc::new(fields))
    }))
}

/// Reads one sidecar: its bytes, their hash and what [`crate::sidecar`] makes of them.
fn read_sidecar(path: &Path, reader: &impl Fn(&Path) -> io::Result<Vec<u8>>) -> Sidecar {
    match reader(path) {
        Ok(bytes) => parse_sidecar(&bytes),
        Err(error) => Sidecar::Unreadable(error.to_string()),
    }
}

/// What a sidecar's `bytes` say: their hash and what [`crate::sidecar`] makes of them.
fn parse_sidecar(bytes: &[u8]) -> Sidecar {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Sidecar::Unreadable("not text".to_owned());
    };
    // The modified time names a version 1 file's steps alone, which the index never shows.
    match Edits::parse(text, 0) {
        Ok(edits) => {
            let view_only = edits.newer();
            Sidecar::Read(Edit {
                version: edits.version,
                file_hash: hash(bytes),
                edited: edits.edited() || view_only.is_some(),
                view_only,
                rating: (edits.copies.first())
                    .map(|copy| Rating::of(&copy.fields))
                    .unwrap_or_default(),
                fields: Arc::new(Fields::of(&edits.copies)),
            })
        }
        Err(error) => Sidecar::Unreadable(error.to_string()),
    }
}

/// The BLAKE3 hash of `bytes`.
pub fn hash(bytes: &[u8]) -> [u8; 32] {
    *blake3::hash(bytes).as_bytes()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;

    use crate::sidecar;
    use tempfile::TempDir;

    use super::*;

    /// One sidecar read alone: none where there is no file, read where there is one, and
    /// unreadable where it is no `.apxmp`.
    #[test]
    fn one_sidecar_reads_as_a_listing_reads_it() {
        let dir = folder(1);
        let raw = dir.path().join("DSCF0000.RAF");
        let path = sidecar::path_for(&raw);
        assert!(matches!(
            read_one(&path),
            Sidecar::Read(Edit { edited: true, .. })
        ));
        assert_eq!(read_one(&dir.path().join("none.apxmp")), Sidecar::None);
        std::fs::write(&path, "not an apxmp").unwrap();
        assert!(matches!(read_one(&path), Sidecar::Unreadable(_)));
    }

    /// A folder of `count` photos, each with a sidecar holding one entry.
    fn folder(count: usize) -> TempDir {
        let dir = TempDir::new().unwrap();
        for i in 0..count {
            let raw = dir.path().join(format!("DSCF{i:04}.RAF"));
            std::fs::write(&raw, [0u8; 8]).unwrap();
            let mut document = sidecar::Document::new("test");
            document.entries.push(sidecar::Entry::Operation {
                operation: "exposure".to_owned(),
                schema: Some(1),
                params: vec![("exposure".to_owned(), format!("{i}"))],
            });
            sidecar::write_xml(&sidecar::path_for(&raw), &document.to_xml()).unwrap();
        }
        dir
    }

    /// A reader that counts how many calls are in flight and how many there have been.
    #[derive(Default)]
    struct Probe {
        in_flight: AtomicUsize,
        most: AtomicUsize,
        reads: AtomicUsize,
        /// The first reads wait until this many have arrived, as a slow share's answers
        /// make them; a timeout keeps a shortfall from hanging.
        wait_for: usize,
    }

    impl Probe {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.most.fetch_max(now, Ordering::SeqCst);
            let nth = self.reads.fetch_add(1, Ordering::SeqCst);
            if nth < self.wait_for {
                let limit = Instant::now() + Duration::from_secs(10);
                // The reads so far count arrivals, and none of the first has left yet.
                while self.reads.load(Ordering::SeqCst) < self.wait_for && Instant::now() < limit {
                    thread::yield_now();
                }
            }
            let bytes = std::fs::read(path);
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            bytes
        }
    }

    #[test]
    fn at_most_sixteen_sidecars_are_read_at_once() {
        let dir = folder(100);
        let probe = std::sync::Arc::new(Probe {
            // Other opens may hold admission slots; this test checks the upper bound.
            wait_for: 1,
            ..Probe::default()
        });

        let reader = std::sync::Arc::clone(&probe);
        let index = build(dir.path(), Options::default(), &|| false, move |path| {
            reader.read(path)
        })
        .unwrap();

        assert_eq!(index.photos.len(), 100);
        assert_eq!(index.counts().edited, 100);
        assert_eq!(probe.reads.load(Ordering::SeqCst), 100);
        let most = probe.most.load(Ordering::SeqCst);
        assert!((1..=READERS).contains(&most), "{most} reads at once");
    }

    #[test]
    fn a_reader_that_panics_makes_its_photo_unreadable_not_the_build_cancelled() {
        let dir = folder(40);

        let index = build(dir.path(), Options::default(), &|| false, |path| {
            if path.ends_with("DSCF0007.RAF.apxmp") {
                panic!("a reader failed");
            }
            std::fs::read(path)
        })
        .unwrap();

        assert!(!index.cancelled);
        let counts = index.counts();
        assert_eq!((counts.edited, counts.unreadable), (39, 1));
        assert!(
            matches!(&index.photos[7].sidecar, Sidecar::Unreadable(why) if why.contains("panic"))
        );
    }

    #[test]
    fn a_cancel_among_the_reads_stops_them_and_marks_the_rest() {
        let dir = folder(100);
        let probe = std::sync::Arc::new(Probe::default());
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let reader = std::sync::Arc::clone(&probe);
        let reader_stop = std::sync::Arc::clone(&stop);

        let index = build(
            dir.path(),
            Options::default(),
            &|| stop.load(Ordering::SeqCst),
            move |path| {
                if reader.reads.load(Ordering::SeqCst) >= 5 {
                    reader_stop.store(true, Ordering::SeqCst);
                }
                reader.read(path)
            },
        )
        .unwrap();

        assert!(index.cancelled);
        let reads = probe.reads.load(Ordering::SeqCst);
        assert!((5..=100).contains(&reads), "{reads} reads");
        let unread = index
            .photos
            .iter()
            .filter(|p| p.sidecar == Sidecar::NotRead)
            .count();
        assert!(unread >= 100 - reads);
    }
}
