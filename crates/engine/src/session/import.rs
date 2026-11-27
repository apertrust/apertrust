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

//! Import's source in the session (decision 85; `docs/library.md`, Import in folder mode):
//! the latest listing, its jobs, and its checks against the destination.
//!
//! A listing's job lists the source, then checks it with the latest trees asked for: its
//! own, or a `CheckSource`'s that came while it listed. A `CheckSource` after the listing
//! is a job of its own. Each check is numbered within its listing, the listing's own 1,
//! and asks whether it is still the latest before each event, so one a newer check
//! replaced sends nothing more.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use apertrust_library::ops::Trees;
use apertrust_library::source::Listed;

use crate::jobs::JobId;

/// The latest listing of Import's source, which the next `ListSource` or a
/// `CancelListing` naming it cancels.
pub(super) struct SourceListing {
    /// Its `ListSource`'s number.
    pub listing: u64,
    /// The job listing it, then checking it.
    pub job: JobId,
    /// The job of the latest `CheckSource` after the listing, which the next cancels.
    pub check: Option<JobId>,
    /// The thumbnail jobs asked for of its photos, which a new listing or a
    /// `CancelListing` cancels.
    pub thumbnails: Vec<JobId>,
    pub checks: Arc<Checks>,
}

/// A listing's checks: the listing, once done, and the latest check asked for with its
/// trees; and the folder listed, once its first chunk names it, which its photos'
/// thumbnails are read from. Shared by its jobs and the engine thread.
pub(super) struct Checks(Mutex<State>);

struct State {
    folder: Option<PathBuf>,
    listed: Option<Arc<Listed>>,
    latest: u64,
    trees: Trees,
}

impl Checks {
    /// The checks of a listing asked to check against `trees` as it ends.
    pub fn new(trees: Trees) -> Self {
        Checks(Mutex::new(State {
            folder: None,
            listed: None,
            latest: 1,
            trees,
        }))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The listing, done: kept for the checks after it. The check its job runs now, the
    /// latest asked for, with its number and its trees.
    pub fn listed(&self, listed: Arc<Listed>) -> (u64, Trees) {
        let mut state = self.lock();
        state.listed = Some(listed);
        (state.latest, state.trees.clone())
    }

    /// A new check against `trees`: its number, one past the last, and the listing to check
    /// where it is done; none while it is listed, its job then running this check as it
    /// ends.
    pub fn ask(&self, trees: Trees) -> (u64, Option<Arc<Listed>>) {
        let mut state = self.lock();
        state.latest += 1;
        state.trees = trees;
        (state.latest, state.listed.clone())
    }

    /// Whether the check numbered `check` is still the latest.
    pub fn latest(&self, check: u64) -> bool {
        self.lock().latest == check
    }

    /// The folder listed, the card's `DCIM` or the folder, as its first chunk named it.
    pub fn found(&self, folder: &Path) {
        self.lock().folder.get_or_insert_with(|| folder.to_owned());
    }

    /// The folder listed, once its first chunk has named it.
    pub fn folder(&self) -> Option<PathBuf> {
        self.lock().folder.clone()
    }
}
