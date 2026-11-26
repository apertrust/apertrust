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

//! The open photo's fields, its values beside the edit (decision 110, `docs/history.md`,
//! Behaviour): its rating, reject and colour labels, set by [`Command::Rate`], each the
//! photo's current value with its clock in its first copy's `apertrust:fields`, never a
//! step of its history.
//!
//! A change is an undo record of the session's ([`FieldRecord`]), which Undo takes back and
//! Redo puts back in the order changes were made, the edit's and the fields' alike, a quick
//! repeat of one key merged into it; Reset, Compress, Truncate and Discard History leave the
//! fields as they are. The write leaves the log as the file holds it, so a photo only rated
//! is given none of its default steps and stays unedited; the rendered thumbnail's version
//! hashes the edit alone ([`apertrust_library::ops::edit_hash`]). darktable's values, read
//! from its `.xmp` as a photo with no `.apxmp` opens, are copied into the file at its first
//! write, with that write's clock. A newer file taken in has its fields joined with the
//! session's, the later clock winning each ([`library_fields::join`]).
//!
//! A command naming other photos is a batch on several photos (decision 109, `batch`),
//! which reaches the active photo here, its record carrying the batch.
//!
//! [`Command::Rate`]: super::Command::Rate

use std::path::PathBuf;
use std::time::Instant;

use apertrust_library::clock::Clock;
use apertrust_library::fields::{self as library_fields, FieldChange, FieldEntry, Rating};
use apertrust_library::sidecar::{Edits, FIRST_COPY, VirtualCopy};

use super::{Engine, Event, Open, RatingChange, RatingState, SessionError};
use crate::history::BatchId;

/// One change of a photo's fields, an undo record of the session's: the change, the last
/// where quick repeats merged, each field it set with the clock that set it, and the batch
/// on several photos it is part of, if any (decision 109), which a batch keeps for each of
/// its photos.
#[derive(Debug, Clone)]
pub(super) struct FieldRecord {
    pub(super) change: RatingChange,
    /// Each field set, its value before and after, and the clock the field holds as the
    /// record last left it, which tells a field changed since.
    changes: Vec<(FieldChange, Clock)>,
    pub(super) batch: Option<BatchId>,
}

impl FieldRecord {
    pub(super) fn new(
        change: RatingChange,
        changes: Vec<FieldChange>,
        clock: Clock,
        batch: Option<BatchId>,
    ) -> Self {
        FieldRecord {
            change,
            changes: changes.into_iter().map(|change| (change, clock)).collect(),
            batch,
        }
    }

    /// The latest clock it set.
    pub(super) fn clock(&self) -> Clock {
        (self.changes.iter().map(|(_, clock)| *clock))
            .max()
            .expect("a record sets a field")
    }

    /// Sets each field it changed among `entries` back as it was before it, or with `back`
    /// false as it left it, at `clock`, where the field still holds the value the record left
    /// it at: one changed since, as a newer file taken in changed it, keeps its later value.
    /// Whether it set any.
    pub(super) fn put(&mut self, entries: &mut Vec<FieldEntry>, back: bool, clock: Clock) -> bool {
        let mut put = false;
        for (change, set) in &mut self.changes {
            let (left, value) = match back {
                true => (&change.after, change.before.clone()),
                false => (&change.before, change.after.clone()),
            };
            if library_fields::value(entries, &change.field).as_ref() != left.as_ref() {
                continue;
            }
            library_fields::set(entries, &change.field, value, clock);
            *set = clock;
            put = true;
        }
        put
    }

    /// `later`, a quick repeat of the same change, taken into it: each field's value after
    /// it and its clock, the value before kept from the first.
    pub(super) fn merge(&mut self, later: FieldRecord) {
        for (change, clock) in later.changes {
            match (self.changes.iter_mut()).find(|(kept, _)| kept.field == change.field) {
                Some((kept, set)) => {
                    kept.after = change.after;
                    *set = clock;
                }
                None => self.changes.push((change, clock)),
            }
        }
        self.change = later.change;
    }
}

/// The fields of `file`'s first copy, the one the session opens, made where the file has
/// none, as [`crate::sidecar::log_xml`] would make it.
pub(super) fn first_mut(file: &mut Edits) -> &mut Vec<FieldEntry> {
    if file.copies.is_empty() {
        file.copies.push(VirtualCopy {
            id: FIRST_COPY.to_owned(),
            number: "0".to_owned(),
            name: String::new(),
            name_clock: None,
            end: String::new(),
            clocks: Vec::new(),
            fields: Vec::new(),
            history: Vec::new(),
        });
    }
    &mut file.copies[0].fields
}

impl Open {
    /// The fields of the copy open.
    pub(super) fn fields(&self) -> &[FieldEntry] {
        self.file.copies.first().map_or(&[], |copy| &copy.fields)
    }

    /// Its rating, as its fields hold it.
    pub(super) fn rating(&self) -> Rating {
        Rating::of(self.fields())
    }

    /// The file a newer one taken in leaves, `file`: the photo's name and fingerprint as the
    /// session holds them, and the fields joined with the session's, the later clock winning
    /// each, so a rating not yet written is kept.
    pub(super) fn take_file(&mut self, file: Edits) {
        let ours = self.fields().to_vec();
        let (photo, fingerprint) = (self.file.photo.take(), self.file.fingerprint.take());
        self.file = file;
        self.written = false;
        self.file.photo = photo;
        self.file.fingerprint = fingerprint.or(self.file.fingerprint.take());
        let joined = library_fields::join(&ours, self.fields());
        if joined != self.fields() {
            *first_mut(&mut self.file) = joined;
        }
    }
}

/// Gives `clock` to each field of `file`'s first copy read from darktable's `.xmp`, which
/// has none yet, as the first write copies it in (decision 51); whether there was any.
pub(super) fn stamp_darktable(file: &mut Edits, clock: impl FnOnce() -> Clock) -> bool {
    let Some(copy) = file.copies.first_mut() else {
        return false;
    };
    if copy.fields.iter().all(|entry| entry.clock.is_some()) {
        return false;
    }
    let clock = clock();
    for entry in copy.fields.iter_mut().filter(|entry| entry.clock.is_none()) {
        entry.clock = Some(clock);
    }
    true
}

impl Engine {
    /// [`super::Command::Rate`] on the active photo, sent at `at`: `change` set on its
    /// fields, an undo record, in the batch being reached where a batch on several photos
    /// reaches it ([`Engine::batching`]), the edit left as it is, then saved. Refused with no
    /// photo open ([`SessionError::NotLive`] while one opens, else [`SessionError::NoPhoto`])
    /// and on a photo open for viewing only. Allowed before the photo is live, since it
    /// needs no decode. A change that changes nothing records and writes nothing; whether it
    /// changed any field.
    pub(super) fn rate(&mut self, change: RatingChange, at: Instant) -> Result<bool, SessionError> {
        let batch = self.batching.map(|batch| batch.id);
        let Some(open) = &mut self.open else {
            return Err(match self.opening {
                Some(_) => SessionError::NotLive,
                None => SessionError::NoPhoto,
            });
        };
        if let Some(view_only) = &open.view_only {
            return Err(SessionError::ViewOnly(view_only.clone()));
        }
        let clock = open.history.next_clock();
        let changes = change.apply(first_mut(&mut open.file), clock);
        if changes.is_empty() {
            return Ok(false);
        }
        let Rating {
            stars,
            rejected,
            labels,
        } = open.rating();
        let name = crate::sidecar::file_name(&open.photo.path);
        tracing::debug!(target: "pipe", "rated {name}: {stars} stars, rejected: {rejected}, labels: {labels:05b}");
        open.history
            .rated(FieldRecord::new(change, changes, clock, batch), at);
        open.fields_unsaved = true;
        open.unsaved = true;
        Ok(true)
    }

    /// The open photo's path and rating, which a turn compares before and after it.
    pub(super) fn open_rating(&self) -> Option<(PathBuf, Rating)> {
        (self.open.as_ref()).map(|open| (open.photo.path.clone(), open.rating()))
    }

    /// The open photo's rating where it differs from `before`, the same photo's as a turn
    /// began: `Rating`, the fact, then `Rated`, the notice the Browser's cell follows.
    pub(super) fn rating_events(&self, before: Option<(PathBuf, Rating)>) -> Vec<Event> {
        let Some((photo, rating)) = self.open_rating() else {
            return Vec::new();
        };
        if before.is_some_and(|(was, then)| was == photo && then == rating) {
            return Vec::new();
        }
        let mut events: Vec<Event> = self.rating_fact().into_iter().collect();
        events.push(Event::Rated { photo, rating });
        events
    }

    /// The open photo's rating as a fact, through the commands applied.
    pub(super) fn rating_fact(&self) -> Option<Event> {
        let (photo, rating) = self.open_rating()?;
        Some(Event::Rating(RatingState {
            through: self.applied,
            photo,
            rating,
        }))
    }
}
