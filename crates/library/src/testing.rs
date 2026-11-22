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

//! A test's hooks, for the library's tests and, through the `testing` feature, its
//! clients': a raw marked as not on the disk ([`evict`]), as a cloud provider leaves a file
//! it has freed, so a test needs no cloud provider; and a count of the library's reads of
//! each raw's bytes ([`reads`]): its fingerprint, its embedded picture and its EXIF.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// The raws marked as not on the disk.
static EVICTED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Each read of a raw's bytes, by its path.
static READS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Marks the raw at `path` as not on the disk: [`crate::ops::raw_state`] says so until
/// [`restore`].
pub fn evict(path: &Path) {
    let mut evicted = EVICTED.lock().unwrap_or_else(PoisonError::into_inner);
    evicted.push(path.to_owned());
}

/// Marks the raw at `path` as on the disk again.
pub fn restore(path: &Path) {
    let mut evicted = EVICTED.lock().unwrap_or_else(PoisonError::into_inner);
    evicted.retain(|evicted| evicted != path);
}

/// Whether the raw at `path` is marked as not on the disk.
pub(crate) fn evicted(path: &Path) -> bool {
    let evicted = EVICTED.lock().unwrap_or_else(PoisonError::into_inner);
    evicted.iter().any(|evicted| evicted == path)
}

/// Counts a read of the raw at `path`, made as the library reads it.
pub(crate) fn read(path: &Path) {
    let mut reads = READS.lock().unwrap_or_else(PoisonError::into_inner);
    reads.push(path.to_owned());
}

/// How many times the library has read the raw at `path`, whether the read succeeded or
/// not.
pub fn reads(path: &Path) -> usize {
    let reads = READS.lock().unwrap_or_else(PoisonError::into_inner);
    reads.iter().filter(|read| *read == path).count()
}
