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
//! while it waits for the listing: a cancelled build returns at once, never waiting for a
//! listing stuck in a call, and the index says so and marks the sidecars it had not
//! reached.
//!
//! A sidecar that cannot be read or parsed is that photo's alone to report
//! ([`Sidecar::Unreadable`]); the rest are indexed. Ratings and other metadata, and changes
//! made outside, are planned.

use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::{Duration, Instant};

use crate::sidecar::{Document, Newer};

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
    /// Whether the photo has an edit: the sidecar holds entries, or is a newer
    /// Apertrust's, whose layout this build may not read and so may not call empty.
    pub edited: bool,
    /// Why the photo opens for viewing only, where it does (decision 43).
    pub view_only: Option<Newer>,
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
    /// between calls into the folder, from the calling thread and the readers', and
    /// stops the build when it answers true.
    pub fn build(
        folder: &Path,
        options: Options,
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Result<Index, Error> {
        build(folder, options, cancel, &read_file)
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
    reader: &(impl Fn(&Path) -> io::Result<Vec<u8>> + Sync),
) -> Result<Index, Error> {
    let started = Instant::now();
    let listed = list(folder, options, cancel)?;
    let mut cancelled = listed.summary.cancelled;

    // Only photos with a sidecar are read; the rest are settled already.
    let mut photos: Vec<Entry> = listed
        .photos
        .into_iter()
        .map(|Photo { name, raw, sidecar }| Entry {
            sidecar: if sidecar.is_some() {
                Sidecar::NotRead
            } else {
                Sidecar::None
            },
            name,
            raw,
            file: sidecar,
        })
        .collect();
    let to_read: Vec<usize> = (0..photos.len())
        .filter(|&i| photos[i].file.is_some())
        .collect();

    if !cancelled && !to_read.is_empty() {
        let next = AtomicUsize::new(0);
        // Set by a reader that found `cancel` true with a sidecar still to read.
        let stopped = AtomicBool::new(false);
        let results: Vec<Vec<(usize, Sidecar)>> = thread::scope(|scope| {
            let workers: Vec<_> = (0..READERS.min(to_read.len()))
                .map(|_| {
                    scope.spawn(|| {
                        let mut done = Vec::new();
                        loop {
                            let at = next.fetch_add(1, Ordering::Relaxed);
                            let Some(&photo) = to_read.get(at) else {
                                break;
                            };
                            if cancel() {
                                stopped.store(true, Ordering::Relaxed);
                                break;
                            }
                            let path = folder.join(sidecar_name(&photos[photo].name));
                            // A reader that panics is that photo's unreadable sidecar,
                            // and the thread goes on with its share.
                            let sidecar = panic::catch_unwind(AssertUnwindSafe(|| {
                                read_sidecar(&path, reader)
                            }))
                            .unwrap_or_else(|_| {
                                Sidecar::Unreadable("reading it panicked".to_owned())
                            });
                            done.push((photo, sidecar));
                        }
                        done
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap_or_else(|p| panic::resume_unwind(p)))
                .collect()
        });
        for (photo, sidecar) in results.into_iter().flatten() {
            photos[photo].sidecar = sidecar;
        }
        cancelled = stopped.load(Ordering::Relaxed);
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

/// Reads one sidecar: its bytes, their hash and what [`crate::sidecar`] makes of them.
fn read_sidecar(path: &Path, reader: &impl Fn(&Path) -> io::Result<Vec<u8>>) -> Sidecar {
    let bytes = match reader(path) {
        Ok(bytes) => bytes,
        Err(error) => return Sidecar::Unreadable(error.to_string()),
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Sidecar::Unreadable("not text".to_owned());
    };
    match Document::parse(text) {
        Ok(document) => {
            let view_only = document.newer();
            Sidecar::Read(Edit {
                version: document.version,
                file_hash: hash(&bytes),
                edited: !document.entries.is_empty() || view_only.is_some(),
                view_only,
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
    use std::sync::atomic::AtomicBool;

    use crate::sidecar;
    use tempfile::TempDir;

    use super::*;

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
        let probe = Probe {
            wait_for: READERS,
            ..Probe::default()
        };

        let index = build(dir.path(), Options::default(), &|| false, &|path| {
            probe.read(path)
        })
        .unwrap();

        assert_eq!(index.photos.len(), 100);
        assert_eq!(index.counts().edited, 100);
        assert_eq!(probe.reads.load(Ordering::SeqCst), 100);
        let most = probe.most.load(Ordering::SeqCst);
        assert_eq!(most, READERS, "reads at once");
    }

    #[test]
    fn a_reader_that_panics_makes_its_photo_unreadable_not_the_build_cancelled() {
        let dir = folder(40);

        let index = build(dir.path(), Options::default(), &|| false, &|path| {
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
        let probe = Probe::default();
        let stop = AtomicBool::new(false);

        let index = build(
            dir.path(),
            Options::default(),
            &|| stop.load(Ordering::SeqCst),
            &|path| {
                if probe.reads.load(Ordering::SeqCst) >= 5 {
                    stop.store(true, Ordering::SeqCst);
                }
                probe.read(path)
            },
        )
        .unwrap();

        assert!(index.cancelled);
        let reads = probe.reads.load(Ordering::SeqCst);
        assert!((5..5 + READERS).contains(&reads), "{reads} reads");
        let unread = index
            .photos
            .iter()
            .filter(|p| p.sidecar == Sidecar::NotRead)
            .count();
        assert_eq!(unread, 100 - reads);
    }
}
