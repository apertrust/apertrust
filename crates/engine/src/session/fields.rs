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
//! Behaviour): its rating, reject and colour labels, set by [`Command::Rate`], the Metadata
//! tool's fields, set by [`Command::SetMetadata`], and the copy's name, set by
//! [`Command::RenameCopy`], each the photo's current value with its clock in its first
//! copy, the one open, never a step of its history.
//!
//! A change is an undo record of the session's ([`FieldRecord`]), which Undo takes back and
//! Redo puts back in the order changes were made, the edit's and the fields' alike, a quick
//! repeat of one key, or a commit of the same field again, merged into it; Reset, Compress,
//! Truncate and Discard History leave the fields as they are. The write leaves the log as
//! the file holds it, so a photo only rated or titled is given none of its default steps and
//! stays unedited; the rendered thumbnail's version hashes the edit alone
//! ([`apertrust_library::ops::edit_hash`]). darktable's values, read from its `.xmp` as a
//! photo with no `.apxmp` opens, its version name the copy's, are copied into the file at
//! its first write, with that write's clock. A newer file taken in has its fields and name
//! joined with the session's, the later clock winning each ([`library_fields::join`]).
//! [`Event::Metadata`] tells the open copy's fields, its name and place as a fact.
//!
//! A rating or a change of fields naming other photos is a batch on several photos
//! (decision 109, `batch`), which reaches the active photo here, its record carrying the
//! batch, and the others by a job, which sets each one's fields as [`wanted`] and
//! [`set_wanted`] set the active photo's.
//!
//! [`Command::Rate`]: super::Command::Rate
//! [`Command::SetMetadata`]: super::Command::SetMetadata
//! [`Command::RenameCopy`]: super::Command::RenameCopy

use std::path::PathBuf;
use std::time::Instant;

use apertrust_library::clock::Clock;
use apertrust_library::fields::{
    self as library_fields, COPY_NAME, FieldChange, FieldEntry, FieldValue, Rating, TypedError,
};
use apertrust_library::sidecar::{Edits, FIRST_COPY, VirtualCopy};

use super::styles::Batch;
use super::{
    CopyState, Engine, Event, MetadataState, Open, RatingChange, RatingState, SessionError,
};
use crate::history::{BatchId, Move};

/// The namespace of `xmpMM:PreservedFileName`, which an import that renames a photo keeps
/// its camera name in (decision 85).
const MEDIA_MANAGEMENT: &str = "http://ns.adobe.com/xap/1.0/mm/";

/// What a [`FieldRecord`] changed, which tells which records merge and how Undo names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Changed {
    /// The rating, as the change last given.
    Rating(RatingChange),
    /// The Metadata tool's fields, by their properties, as the command named them.
    Metadata(Vec<&'static str>),
    /// The copy's name.
    Name,
}

impl Changed {
    /// Whether a change of `later` right after `self` merges into it: a rating of the same
    /// kind, the same fields, or the name again.
    fn merges(&self, later: &Changed) -> bool {
        match (self, later) {
            (Changed::Rating(a), Changed::Rating(b)) => a.same_kind(b),
            (Changed::Metadata(a), Changed::Metadata(b)) => a == b,
            (Changed::Name, Changed::Name) => true,
            _ => false,
        }
    }

    /// What Undo and Redo name it, on `photos` photos.
    pub(super) fn moved(&self, photos: usize) -> Move {
        match self {
            Changed::Rating(change) => Move::Field {
                change: *change,
                photos,
            },
            Changed::Metadata(fields) => Move::Metadata {
                field: match fields.as_slice() {
                    [field] => Some(*field),
                    _ => None,
                },
                photos,
            },
            Changed::Name => Move::CopyName,
        }
    }
}

/// One change of a photo's fields, an undo record of the session's: what it changed, the
/// last where quick repeats merged, each field it set with the clock that set it, and the
/// batch on several photos it is part of, if any (decision 109), which a batch keeps for
/// each of its photos.
#[derive(Debug, Clone)]
pub(super) struct FieldRecord {
    pub(super) change: Changed,
    /// Each field set, its value before and after, and the clock the field holds as the
    /// record last left it, which tells a field changed since.
    changes: Vec<(FieldChange, Clock)>,
    pub(super) batch: Option<BatchId>,
}

impl FieldRecord {
    pub(super) fn new(
        change: Changed,
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

    /// Sets each field it changed in `copy` back as it was before it, or with `back` false
    /// as it left it, at `clock`, where the field still holds the value the record left it
    /// at, and with `clocked` the clock too: one changed since, as a newer file taken in
    /// changed it, keeps its later value, and with `clocked` though a later change set it
    /// back to the same value, as a batch's move on its photos reads them (decision 110);
    /// the open photo's own Undo, whose records move the field in turn, reads the value
    /// alone. The copy's name is one of its fields here ([`COPY_NAME`]). Whether it set any.
    pub(super) fn put(
        &mut self,
        copy: &mut VirtualCopy,
        (back, clocked): (bool, bool),
        clock: Clock,
    ) -> bool {
        let mut put = false;
        for (change, set) in &mut self.changes {
            let (left, value) = match back {
                true => (&change.after, change.before.clone()),
                false => (&change.before, change.after.clone()),
            };
            let held = library_fields::value_in(copy, &change.field);
            let moved = clocked && clock_in(copy, &change.field) != Some(*set);
            if held.as_ref() != left.as_ref() || moved {
                continue;
            }
            library_fields::set_in(copy, &change.field, value, clock);
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

    /// Whether `later`, a change right after it, merges into it, neither of them part of a
    /// batch on several photos, which is a record of its own.
    pub(super) fn merges(&self, later: &FieldRecord) -> bool {
        self.batch.is_none() && later.batch.is_none() && self.change.merges(&later.change)
    }
}

/// The clock of the last change of `field` in `copy`, or with [`COPY_NAME`] its name's;
/// `None` for one never set, or not written yet.
fn clock_in(copy: &VirtualCopy, field: &str) -> Option<Clock> {
    match field {
        COPY_NAME => copy.name_clock,
        _ => (copy.fields.iter())
            .find(|entry| entry.field == field)
            .and_then(|entry| entry.clock),
    }
}

/// A first copy, [`FIRST_COPY`], with nothing in it, as [`crate::sidecar::log_xml`] makes
/// one for a file with none.
pub(super) fn new_copy() -> VirtualCopy {
    VirtualCopy {
        id: FIRST_COPY.to_owned(),
        number: "0".to_owned(),
        name: String::new(),
        name_clock: None,
        end: String::new(),
        clocks: Vec::new(),
        fields: Vec::new(),
        history: Vec::new(),
    }
}

/// `file`'s first copy, the one the session opens, made where the file has none, as
/// [`crate::sidecar::log_xml`] would make it.
pub(super) fn copy_mut(file: &mut Edits) -> &mut VirtualCopy {
    if file.copies.is_empty() {
        file.copies.push(new_copy());
    }
    &mut file.copies[0]
}

/// The fields of `file`'s first copy ([`copy_mut`]).
pub(super) fn first_mut(file: &mut Edits) -> &mut Vec<FieldEntry> {
    &mut copy_mut(file).fields
}

/// What `copy` holds beside its edit, which a turn compares before and after it: its
/// fields and its name.
pub(super) fn beside(copy: Option<&VirtualCopy>) -> (Vec<FieldEntry>, String) {
    copy.map(|copy| (copy.fields.clone(), copy.name.clone()))
        .unwrap_or_default()
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
    /// session holds them, and the fields and the copy's name joined with the session's, the
    /// later clock winning each, so a change not yet written is kept.
    pub(super) fn take_file(&mut self, file: Edits) {
        let ours = self.file.copies.first().cloned();
        let (photo, fingerprint) = (self.file.photo.take(), self.file.fingerprint.take());
        self.file = file;
        self.written = false;
        self.file.photo = photo;
        self.file.fingerprint = fingerprint.or(self.file.fingerprint.take());
        let Some(ours) = ours else {
            return;
        };
        let joined = library_fields::join(&ours.fields, self.fields());
        let theirs = self.file.copies.first().cloned().unwrap_or_else(new_copy);
        let (name, name_clock) = library_fields::join_name(&ours, &theirs);
        if joined != theirs.fields || (&name, name_clock) != (&theirs.name, theirs.name_clock) {
            let copy = copy_mut(&mut self.file);
            copy.fields = joined;
            (copy.name, copy.name_clock) = (name, name_clock);
        }
    }

    /// The open copy's fields, its name and place, and the photo's preserved file name, as
    /// [`Event::Metadata`] tells them, through the commands `through`.
    pub(super) fn metadata(&self, through: u64) -> MetadataState {
        let copy = self.file.copies.first();
        let fields = (self.fields().iter())
            .filter(|entry| !library_fields::is_rating(&entry.field))
            .map(|entry| (entry.field.clone(), entry.value.clone()))
            .collect();
        let preserved = (self.file.other.iter())
            .find(|property| property.name.is(MEDIA_MANAGEMENT, "PreservedFileName"))
            .and_then(|property| property.node.as_str())
            .map(str::to_owned);
        // Values with no clock are darktable's, not yet written.
        let darktable = copy.is_some_and(|copy| {
            let name = !copy.name.is_empty() && copy.name_clock.is_none();
            name || copy.fields.iter().any(|entry| entry.clock.is_none())
        });
        MetadataState {
            through,
            photo: self.photo.path.clone(),
            fields,
            copy: CopyState {
                name: copy.map(|copy| copy.name.clone()).unwrap_or_default(),
                number: 1,
                count: self.file.copies.len().max(1),
            },
            preserved_file_name: preserved,
            darktable,
        }
    }
}

/// Gives `clock` to each field of `file`'s first copy read from darktable's `.xmp`, which
/// has none yet, and to its name where darktable's version name gave it one, as the first
/// write copies them in (decision 51); whether there was any.
pub(super) fn stamp_darktable(file: &mut Edits, clock: impl FnOnce() -> Clock) -> bool {
    let Some(copy) = file.copies.first_mut() else {
        return false;
    };
    let name = !copy.name.is_empty() && copy.name_clock.is_none();
    if !name && copy.fields.iter().all(|entry| entry.clock.is_some()) {
        return false;
    }
    let clock = clock();
    for entry in copy.fields.iter_mut().filter(|entry| entry.clock.is_none()) {
        entry.clock = Some(clock);
    }
    if name {
        copy.name_clock = Some(clock);
    }
    true
}

/// What a change of fields wants: each field by its property, its value now and after.
pub(super) type Wanted = Vec<(&'static str, Option<FieldValue>, Option<FieldValue>)>;

/// What a [`super::Command::SetMetadata`] sets: each field by its property with what was
/// typed, and each with a value as it is, as a list's chosen value, never typed again.
#[derive(Debug, Clone, Default)]
pub(super) struct MetadataSet {
    pub(super) fields: Vec<(String, Option<String>)>,
    pub(super) values: Vec<(String, FieldValue)>,
}

impl MetadataSet {
    /// The properties it names, in order, each once.
    pub(super) fn properties(&self) -> Vec<String> {
        let mut properties: Vec<String> = Vec::new();
        let named = (self.fields.iter().map(|(field, _)| field))
            .chain(self.values.iter().map(|(field, _)| field));
        for field in named {
            if !properties.contains(field) {
                properties.push(field.clone());
            }
        }
        properties
    }
}

/// What setting `set` on `copy` wants (a [`super::Command::SetMetadata`]): each field, by
/// its property, its value in `copy` now and what was typed as its kind takes it
/// ([`FieldValue::from_typed`]), `None` clearing it, or the value given as it is, a field
/// named twice taking the later. Refused whole for a field none of
/// [`library_fields::FIELDS`] is, or the rating's ([`SessionError::UnknownField`]), and for
/// a value its field cannot hold ([`SessionError::FieldValue`]), whatever `copy` holds.
pub(super) fn wanted(
    copy: Option<&VirtualCopy>,
    set: &MetadataSet,
) -> Result<Wanted, SessionError> {
    let def = |property: &String| {
        (library_fields::field(property))
            .filter(|def| !library_fields::is_rating(def.property))
            .ok_or_else(|| SessionError::UnknownField {
                field: property.clone(),
            })
    };
    let refused = |property: &String, reason| SessionError::FieldValue {
        field: property.clone(),
        reason,
    };
    let mut wanted: Wanted = Vec::new();
    for (property, typed) in &set.fields {
        let def = def(property)?;
        let now = copy.and_then(|copy| library_fields::value_in(copy, def.property));
        let after = match typed {
            Some(typed) => FieldValue::from_typed(def, typed, now.as_ref())
                .map_err(|reason| refused(property, reason))?,
            None => None,
        };
        wanted.retain(|(kept, _, _)| *kept != def.property);
        wanted.push((def.property, now, after));
    }
    for (property, value) in &set.values {
        let def = def(property)?;
        if !value.is(def.kind) {
            return Err(refused(property, TypedError::WrongKind));
        }
        let now = copy.and_then(|copy| library_fields::value_in(copy, def.property));
        wanted.retain(|(kept, _, _)| *kept != def.property);
        wanted.push((def.property, now, Some(value.clone())));
    }
    Ok(wanted)
}

/// The properties `wanted` sets, in order.
pub(super) fn properties(wanted: &Wanted) -> Vec<&'static str> {
    wanted.iter().map(|(property, ..)| *property).collect()
}

/// Sets `wanted` on `copy` at `clock`, each field whose value changes: what it changed, in
/// order, none for a change that changes nothing. The caller advances the copy's clock
/// vector with `clock`.
pub(super) fn set_wanted(copy: &mut VirtualCopy, wanted: Wanted, clock: Clock) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    for (field, before, after) in wanted {
        if before == after {
            continue;
        }
        library_fields::set_in(copy, field, after.clone(), clock);
        changes.push(FieldChange {
            field: field.to_owned(),
            before,
            after,
        });
    }
    changes
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
        let batch = self.batching;
        let open = self.fields_open()?;
        let clock = open.history.next_clock(batch.map(|batch| batch.wall));
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
        let batch = batch.map(|batch| batch.id);
        let record = FieldRecord::new(Changed::Rating(change), changes, clock, batch);
        open.history.rated(record, at);
        open.fields_unsaved = true;
        open.unsaved = true;
        Ok(true)
    }

    /// [`super::Command::SetMetadata`] on the active photo, sent at `at`: each of `fields`,
    /// by its property, set to what was typed as its kind takes it
    /// ([`FieldValue::from_typed`]), `None` clearing it, one undo record for all of them,
    /// then saved, the edit left as it is. Refused whole for a field none of
    /// [`library_fields::FIELDS`] is, or the rating's ([`SessionError::UnknownField`]), and
    /// for a value its field cannot hold ([`SessionError::FieldValue`]); refused as
    /// [`Engine::rate`] is with no photo or one open for viewing only. Whether it changed any
    /// field.
    pub(super) fn set_metadata(
        &mut self,
        set: &MetadataSet,
        at: Instant,
    ) -> Result<bool, SessionError> {
        let batch = self.batching;
        let open = self.fields_open()?;
        let wanted = wanted(open.file.copies.first(), set)?;
        let change = Changed::Metadata(properties(&wanted));
        Ok(change_fields(open, change, wanted, at, batch))
    }

    /// [`super::Command::RenameCopy`]: the open copy's name set to `name`, on one line and
    /// trimmed, an empty one clearing it, one undo record, as [`Engine::set_metadata`] sets
    /// a field.
    pub(super) fn rename_copy(&mut self, name: &str, at: Instant) -> Result<bool, SessionError> {
        let open = self.fields_open()?;
        let now =
            (open.file.copies.first()).and_then(|copy| library_fields::value_in(copy, COPY_NAME));
        // On one line, a pasted line break a space, as a field that takes none.
        let name = name.lines().collect::<Vec<_>>().join(" ");
        let name = name.trim();
        let after = (!name.is_empty()).then(|| FieldValue::Text(name.to_owned()));
        Ok(change_fields(
            open,
            Changed::Name,
            vec![(COPY_NAME, now, after)],
            at,
            None,
        ))
    }

    /// The open photo, whose fields a command sets: refused with none open
    /// ([`SessionError::NotLive`] while one opens, else [`SessionError::NoPhoto`]) and for
    /// one open for viewing only.
    fn fields_open(&mut self) -> Result<&mut Open, SessionError> {
        let Some(open) = &mut self.open else {
            return Err(match self.opening {
                Some(_) => SessionError::NotLive,
                None => SessionError::NoPhoto,
            });
        };
        if let Some(view_only) = &open.view_only {
            return Err(SessionError::ViewOnly(view_only.clone()));
        }
        Ok(open)
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

    /// Everything the open photo's `Metadata` fact tells but its `through`, which a turn
    /// compares before and after it.
    pub(super) fn open_beside(&self) -> Option<MetadataState> {
        self.open.as_ref().map(|open| open.metadata(0))
    }

    /// The open photo's `Metadata` where anything it tells differs from `before`, as a turn
    /// began: its photo, fields, copy, preserved file name or darktable's mark.
    pub(super) fn metadata_events(&self, before: Option<MetadataState>) -> Option<Event> {
        let now = self.open_beside()?;
        if before.is_some_and(|before| before == now) {
            return None;
        }
        self.metadata_fact()
    }

    /// The open photo's fields as a fact, through the commands applied.
    pub(super) fn metadata_fact(&self) -> Option<Event> {
        let open = self.open.as_ref()?;
        Some(Event::Metadata(open.metadata(self.applied)))
    }
}

/// Sets `wanted` on `open`'s copy as one undo record of `change`, sent at `at`, in the
/// batch on several photos `batch` where it is part of one, each field whose value changes
/// with one clock, the copy's clock vector advanced with it; nothing for a change that
/// changes nothing. Whether it changed any.
fn change_fields(
    open: &mut Open,
    change: Changed,
    wanted: Wanted,
    at: Instant,
    batch: Option<Batch>,
) -> bool {
    let clock = open.history.next_clock(batch.map(|batch| batch.wall));
    let batch = batch.map(|batch| batch.id);
    let changes = set_wanted(copy_mut(&mut open.file), wanted, clock);
    if changes.is_empty() {
        return false;
    }
    let name = crate::sidecar::file_name(&open.photo.path);
    let set: Vec<&str> = changes.iter().map(|change| change.field.as_str()).collect();
    tracing::debug!(target: "pipe", "set {} of {name}", set.join(", "));
    open.history
        .rated(FieldRecord::new(change, changes, clock, batch), at);
    open.fields_unsaved = true;
    open.unsaved = true;
    true
}
