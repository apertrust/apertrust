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

//! The Metadata tool's fields (decision 110; `docs/gui.md`, The Metadata tool): each a
//! [`FieldWell`] showing the value of what the tool describes, the active photo's as the
//! session's `Metadata` fact tells it, or several photos' from the session's `Summary`; a
//! commit sends `SetMetadata`, or `RenameCopy` for Basic's Copy, with what was typed and
//! the photos the field opened on; the value shown at once and replaced by the session's
//! next `Metadata`, or the summary asked for once the batch ends.
//!
//! - **Several photos:** a value they share edits as one photo's; one they differ in reads
//!   Different, its ⌄ listing each value with its count, a value chosen written to all at
//!   once and Clear on N Photos clearing it on all; Return with nothing typed writes
//!   nothing. Copy is read-only. Each write is one batch, its notice and its Undo the
//!   batch's (`styles::tool::batch_notice`).
//! - **The photos a field opened on** are those a commit names: a draft that ends with no
//!   press, as the active photo or the selection changes from the menu bar or an open from
//!   the Finder, is sent to them before the command that leaves them ([`Fields::drafts`]),
//!   while they are still the session's.
//! - **Disabled** with no photo, every selected photo open for viewing only, and on a photo
//!   open for viewing only, the tooltip saying why.
//! - **A failed write** marks each field committed since the last write that succeeded: a
//!   warning button at its well's end, its menu the reason and Try Again, which writes again
//!   (`RetrySave`); the Viewer shows the failed-edit message from the same save state.
//! - **Date created** takes a date as the language writes one, day first or, in American
//!   English, month first, or as ISO 8601, sent as an XMP date; anything else stays in the
//!   well with the warning button "Not a date" and is not sent.
//! - **Copyright status** is a pull-down of Unknown, Copyrighted and Public domain.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use apertrust_engine::session::facts::{FieldSummary, Summary};
use apertrust_engine::session::{Command, FieldValue, MetadataState, Photo};
use apertrust_library::fields::{self as library_fields, COPY_NAME, FieldKind};
use apertrust_tools::{FieldRow, messages};
use apertrust_ui::Translator;
use apertrust_ui::components::FieldWell;
use apertrust_ui::i18n::{Args, Locale};
use apertrust_ui::popup::{Entry, Item};
use eframe::egui;

use super::{ID, active_photo};
use crate::mirror::{Mirror, PhotoState};

/// Try Again's id in a field's warning menu.
const TRY_AGAIN: &str = "metadata-try-again";
/// The reason's id there, an item that cannot be chosen.
const REASON: &str = "metadata-field-reason";
/// A value's id in a Different field's values list, before its place.
const VALUE: &str = "metadata-value-";
/// Clear on N Photos' id there.
const CLEAR_ON: &str = "metadata-clear-on";
/// The values list's "and N more values", an item that cannot be chosen.
const MORE: &str = "metadata-more";

/// The fields' state between frames.
#[derive(Debug, Default)]
pub struct Fields {
    /// A value committed and not yet told, by property, with the value the session told
    /// as it was committed: shown while the session tells that one, so a fact of another
    /// change, as a rating's, never shows the value before for a moment, and one telling
    /// anything else, the commit's or an Undo's, ends it.
    shown: HashMap<&'static str, (String, String)>,
    /// A value committed on several photos, by property: shown on those photos until a
    /// summary asked for after its batch ended comes.
    written: HashMap<&'static str, Written>,
    /// Each field being edited, by its row's id: its property, the photos it began on, a
    /// draft going to those alone, and what it showed as editing began, its text and
    /// whether it read Different, which a commit is compared with.
    editing: HashMap<&'static str, Editing>,
    /// The fields committed since the last write the session confirmed.
    unconfirmed: BTreeSet<&'static str>,
    /// The fields whose write failed, until a write succeeds.
    failed: BTreeSet<&'static str>,
    /// What was typed in Date created that is no date, kept in its well.
    not_a_date: Option<String>,
    /// The photos the state is of, which others described clear.
    photos: Vec<PathBuf>,
    /// The number of the summary of several photos asked for last.
    asked: u64,
    /// The pass the save state was last followed in.
    pass: Option<u64>,
}

/// A field being edited: its property, the photos it began on and what it showed then,
/// its text and whether it read Different.
type Editing = (&'static str, Vec<PathBuf>, (String, bool));

/// A value written on several photos, shown on them until their summary tells it.
#[derive(Debug)]
struct Written {
    /// The value as the well shows it.
    text: String,
    /// The photos it was written to.
    photos: Vec<PathBuf>,
    /// The number of its `SetMetadata`, once sent.
    command: Option<u64>,
    /// The number of the summary asked for last as its batch was seen to end.
    ended: Option<u64>,
}

/// The active photo's `Metadata` fact, where the session told it of that photo.
pub fn state(mirror: &Mirror) -> Option<&MetadataState> {
    let photo = active_photo(mirror)?;
    (mirror.metadata.as_ref()).filter(|state| state.photo == photo.path)
}

/// The photo open, never a photo left's held while the next opens.
fn open_photo(mirror: &Mirror) -> Option<&Photo> {
    match &mirror.photo {
        PhotoState::Open(photo) => Some(photo),
        _ => None,
    }
}

/// The photos the fields describe and a commit names: the selection, as the session told
/// it, where several are selected, else the photo open; none while either is on its way.
pub fn described(mirror: &Mirror) -> Vec<PathBuf> {
    if mirror.selected_count() > 1 {
        return match mirror.selection_settled() {
            true => mirror.selection.photos.clone(),
            false => Vec::new(),
        };
    }
    open_photo(mirror)
        .filter(|_| mirror.selection_settled())
        .map(|photo| vec![photo.path.clone()])
        .unwrap_or_default()
}

/// The well of the field row `id`.
fn well_id(id: &str) -> egui::Id {
    egui::Id::unique(("metadata-field", id))
}

/// Why the fields cannot be edited now, in `words`; none where they can. Copy, `property`
/// [`COPY_NAME`], is read-only with several photos selected.
fn disabled(mirror: &Mirror, property: &str, words: &Translator) -> Option<String> {
    if mirror.selected_count() > 1 {
        if !mirror.selection_settled() {
            return Some(words.text("metadata-select-a-photo"));
        }
        if property == COPY_NAME {
            return Some(words.text("metadata-rename-one"));
        }
        // The session refuses a batch while the active photo opens.
        if let PhotoState::Opening { .. } = mirror.photo {
            let opening = crate::controls_opening(mirror, words);
            return Some(opening.unwrap_or_else(|| words.text("metadata-select-a-photo")));
        }
        let photos = &mirror.selection.photos;
        let view_only = super::view_only(mirror, photos);
        return (view_only.len() == photos.len()).then(|| words.text("metadata-all-view-only"));
    }
    // A photo left's values while the next opens are read-only.
    if let PhotoState::Opening { .. } = mirror.photo {
        let opening = crate::controls_opening(mirror, words);
        return Some(opening.unwrap_or_else(|| words.text("metadata-select-a-photo")));
    }
    let Some(photo) = open_photo(mirror).filter(|_| mirror.selection_settled()) else {
        return Some(words.text("metadata-select-a-photo"));
    };
    if let Some(view_only) = &photo.view_only {
        return Some(crate::view_only_notice(words, view_only));
    }
    state(mirror)
        .is_none()
        .then(|| words.text("metadata-select-a-photo"))
}

/// What several photos hold of `property`, as their summary `held` says: the value they
/// share, empty for none or while no summary is held, and how they differ where they do.
fn several(property: &str, held: Option<&Summary>) -> (String, Option<FieldSummary>) {
    match held.and_then(|summary| summary.field(property)) {
        Some(FieldSummary::Same(value)) => (value.shown(), None),
        Some(different @ FieldSummary::Different { .. }) => {
            (String::new(), Some(different.clone()))
        }
        _ => (String::new(), None),
    }
}

impl Fields {
    /// Follows what the tool describes and the save state once a pass: a failed write marks
    /// the fields committed since the last one that succeeded, and one that succeeds clears
    /// them; other photos described commit each draft to the photos it began on, into
    /// `commands`, and clear everything else.
    fn follow(
        &mut self,
        ctx: &egui::Context,
        (pass, month_first): (u64, bool),
        mirror: &Mirror,
        commands: &mut Vec<Command>,
    ) {
        if self.pass == Some(pass) {
            return;
        }
        self.pass = Some(pass);
        let photos = described(mirror);
        if photos != self.photos {
            // A draft of the photos left goes to them, never to the next (decision 110).
            commands.extend(self.drafts(ctx, month_first));
            *self = Fields {
                photos,
                pass: Some(pass),
                ..Fields::default()
            };
            return;
        }
        if mirror.saving.failure.is_some() {
            self.failed.append(&mut self.unconfirmed);
        } else if !mirror.saving.writing() {
            self.unconfirmed.clear();
            self.failed.clear();
        }
        let state = state(mirror);
        self.shown
            .retain(|property, (_, before)| told(property, state) == *before);
    }

    /// Every field's draft ended with no press, as the photos it began on leave the tool
    /// (decision 110): what was typed, a date as Return makes one, sent to those photos;
    /// none for Copy, whose `RenameCopy` names no photo, nor for nothing typed.
    /// `month_first` reads a date as American English writes it.
    pub fn drafts(&mut self, ctx: &egui::Context, month_first: bool) -> Vec<Command> {
        let mut commands = Vec::new();
        for (id, (property, began, start)) in self.editing.drain() {
            let typed = FieldWell::take(ctx, well_id(id));
            let Some(mut typed) = typed.filter(|_| property != COPY_NAME) else {
                continue;
            };
            let def = library_fields::field(property);
            if def.is_some_and(|def| def.kind == FieldKind::Date) {
                match typed_date(&typed, month_first) {
                    Some(date) => typed = date,
                    None => continue,
                }
            }
            // As Return commits it: nothing where it holds what editing began on.
            let Some(made) = made(def, &typed, None) else {
                continue;
            };
            if unchanged(&made, &start) {
                continue;
            }
            let typed = (!made.is_empty()).then_some(typed);
            commands.push(Command::SetMetadata {
                photos: began,
                fields: vec![(property.to_owned(), typed)],
                values: Vec::new(),
            });
        }
        commands
    }

    /// `command` went out numbered `number`: a value written on several photos by it shows
    /// until its batch ends.
    pub fn sent(&mut self, command: &Command, number: u64) {
        let Command::SetMetadata {
            photos,
            fields,
            values,
        } = command
        else {
            return;
        };
        let named =
            (fields.iter().map(|(field, _)| field)).chain(values.iter().map(|(field, _)| field));
        for field in named {
            let written = (self.written.iter_mut()).find(|(property, written)| {
                **property == field.as_str() && written.photos == *photos
            });
            if let Some((_, written)) = written.filter(|(_, written)| written.command.is_none()) {
                written.command = Some(number);
            }
        }
    }

    /// Follows the summary of several photos asked for last, numbered `asked`, and the one
    /// held, numbered `held`: a value written on them is dropped once its own batch has
    /// ended and a summary asked for after that comes, or as its command is refused.
    pub fn follow_summary(&mut self, mirror: &Mirror, asked: u64, held: Option<u64>) {
        // The end seen against the summary asked for before this one: one asked for since,
        // as the groups closed meanwhile open again, may be after the end.
        let before = std::mem::replace(&mut self.asked, asked);
        let through = mirror.batch_ended_through;
        for written in self.written.values_mut() {
            if written.ended.is_none() && written.command.is_some_and(|n| n <= through) {
                written.ended = Some(before);
            }
        }
        // A command refused before its batch began wrote nothing.
        let refused = |n: &u64| mirror.refused.contains(n);
        self.written.retain(|_, written| {
            let shown = match (written.ended, held) {
                (Some(ended), Some(held)) => held <= ended,
                _ => true,
            };
            shown && !written.command.as_ref().is_some_and(refused)
        });
    }

    /// The value of `property` as the well shows it, with how several photos differ in it
    /// where they do: one committed and not yet told, else the session's, the active
    /// photo's, or the photos' summary `held` with several selected.
    pub fn value(
        &self,
        property: &str,
        mirror: &Mirror,
        held: Option<&Summary>,
    ) -> (String, Option<FieldSummary>) {
        if mirror.selected_count() > 1 {
            let photos = described(mirror);
            let written = self.written.get(property);
            if let Some(written) = written.filter(|written| written.photos == photos) {
                return (written.text.clone(), None);
            }
            return several(property, held);
        }
        let this_photo = described(mirror) == self.photos;
        if let Some((text, _)) = self.shown.get(property).filter(|_| this_photo) {
            return (text.clone(), None);
        }
        (told(property, state(mirror)), None)
    }

    /// Whether `property` holds a value, as a closed group's dot counts it: on one photo, or
    /// on any of several.
    pub fn holds(&self, property: &str, mirror: &Mirror, held: Option<&Summary>) -> bool {
        let (text, different) = self.value(property, mirror, held);
        !text.is_empty() || different.is_some()
    }
}

/// The value of `property` as the session's fact `state` tells it, as a well shows it.
fn told(property: &str, state: Option<&MetadataState>) -> String {
    let Some(state) = state else {
        return String::new();
    };
    if property == COPY_NAME {
        return state.copy.name.clone();
    }
    (state.fields.iter())
        .find(|(field, _)| field == property)
        .and_then(|(_, value)| value.as_ref())
        .map(|value| value.shown())
        .unwrap_or_default()
}

/// The message `id` in `words` with `args`, each a name and a number or text.
fn said(words: &Translator, id: &str, args: &[(&str, Arg<'_>)]) -> String {
    let mut set = Args::new();
    for (name, value) in args {
        match value {
            Arg::Number(number) => set.set(*name, *number as f64),
            Arg::Text(text) => set.set(*name, *text),
        }
    }
    words.text_with(id, Some(&set))
}

/// An argument of [`said`].
enum Arg<'a> {
    Number(usize),
    Text(&'a str),
}

/// A Different field's tooltip lines, in `words`: up to five values with their counts,
/// "and 3 more", and how many photos hold none.
fn different_lines(
    values: &[(FieldValue, usize)],
    more: usize,
    empty: usize,
    words: &Translator,
) -> Vec<String> {
    let shown = values.len().min(5);
    let mut lines: Vec<String> = (values[..shown].iter())
        .map(|(value, count)| {
            let value = value.shown();
            said(
                words,
                "metadata-value-count",
                &[("value", Arg::Text(&value)), ("count", Arg::Number(*count))],
            )
        })
        .collect();
    let left = values.len() - shown + more;
    if left > 0 {
        lines.push(said(
            words,
            "metadata-and-more",
            &[("count", Arg::Number(left))],
        ));
    }
    if empty > 0 {
        lines.push(said(
            words,
            "metadata-photos-none",
            &[("count", Arg::Number(empty))],
        ));
    }
    lines
}

/// A Different field's values list, in `words`: each value with its count, most common
/// first, at most ten, then "and 24 more values", disabled; a separator; Clear on N Photos.
fn values_menu(
    values: &[(FieldValue, usize)],
    more: usize,
    count: usize,
    words: &Translator,
) -> Vec<Entry> {
    let mut menu: Vec<Entry> = (values.iter().enumerate())
        .map(|(at, (value, times))| {
            let item = Item::new(format!("{VALUE}{at}"), value.shown());
            Entry::Item(item.shortcut(times.to_string()))
        })
        .collect();
    if more > 0 {
        let text = said(
            words,
            "metadata-more-values",
            &[("count", Arg::Number(more))],
        );
        menu.push(Entry::Item(Item::new(MORE, text).enabled(false)));
    }
    menu.push(Entry::Separator);
    let clear = said(words, CLEAR_ON, &[("count", Arg::Number(count))]);
    menu.push(Entry::Item(Item::new(CLEAR_ON, clear)));
    menu
}

impl Fields {
    /// Draws the field `row` in its labels' `column`, of the photos `held` summarises where
    /// several are selected, a commit going into `commands`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        row: &FieldRow,
        column: f32,
        mirror: &Mirror,
        held: Option<&Summary>,
        words: &Translator,
        commands: &mut Vec<Command>,
    ) {
        let month_first = words.locale() == Locale::EnUs;
        let pass = ui.ctx().cumulative_pass_nr();
        self.follow(ui.ctx(), (pass, month_first), mirror, commands);
        let [label, tag] = messages::field_words(ID, row.id).map(|id| words.text(&id));
        let state = state(mirror);
        let def = library_fields::field(row.field);
        let property = def.map_or(COPY_NAME, |def| def.property);
        let why = disabled(mirror, property, words);
        let kind = def.map_or(FieldKind::Text, |def| def.kind);
        let lines = def.is_some_and(|def| def.lines);
        let count = mirror.selected_count();
        let (value, different) = self.value(property, mirror, held);
        // What the well shows as editing begins, which a commit is compared with.
        let shown = (value.clone(), different.is_some());
        let mut tip = vec![tag];
        match row.field {
            "apertrust:notes" => tip.insert(0, words.text("metadata-notes-private")),
            "photoshop:DateCreated" => tip.insert(0, words.text("metadata-date-created-hint")),
            _ => {}
        }
        // "Title of 12 photos", then how they differ.
        if count > 1 {
            let mut first = vec![said(
                words,
                "metadata-field-of",
                &[("field", Arg::Text(&label)), ("count", Arg::Number(count))],
            )];
            if let Some(FieldSummary::Different {
                values,
                more,
                empty,
            }) = &different
            {
                first.extend(different_lines(values, *more, *empty, words));
            }
            tip.splice(0..0, first);
        }
        if let Some(why) = &why {
            tip.insert(0, why.clone());
        }
        let tip = tip.join("\n");
        let not_a_date = (kind == FieldKind::Date)
            .then(|| self.not_a_date.clone())
            .flatten();
        let text = not_a_date.clone().unwrap_or(value);
        let copy_of = state.filter(|_| count < 2).map(|state| {
            let mut args = Args::new();
            args.set("number", state.copy.number as f64);
            args.set("count", state.copy.count as f64);
            words.text_with("metadata-copy-of", Some(&args))
        });
        let id = well_id(row.id);
        let options = [
            ("", words.text("metadata-copyright-unknown")),
            ("True", words.text("metadata-copyright-copyrighted")),
            ("False", words.text("metadata-copyright-public-domain")),
        ];
        let options: Vec<(&str, &str)> = options.iter().map(|(id, w)| (*id, w.as_str())).collect();
        let word = words.text("metadata-different");
        let description = match &different {
            Some(FieldSummary::Different {
                values,
                more,
                empty,
            }) => said(
                words,
                "metadata-different-description",
                &[
                    ("count", Arg::Number(count)),
                    (
                        "values",
                        Arg::Number(values.len() + more + usize::from(*empty > 0)),
                    ),
                ],
            ),
            _ => String::new(),
        };
        let values_name = said(
            words,
            "metadata-values-of",
            &[("field", Arg::Text(&label)), ("count", Arg::Number(count))],
        );
        let mut well = FieldWell::new(id, &label, &text, column)
            .lines(lines)
            .enabled(why.is_none())
            .tip(&tip);
        if row.field == COPY_NAME
            && let Some(copy_of) = &copy_of
        {
            well = well.after(copy_of);
        }
        if kind == FieldKind::Marked {
            well = well.choice(&options);
        }
        if let Some(FieldSummary::Different { values, more, .. }) = &different {
            well = well.different(&word, &description);
            if why.is_none() && kind != FieldKind::Marked {
                well = well.values(&values_name, values_menu(values, *more, count, words));
            }
        }
        let not_saved = {
            let mut args = Args::new();
            args.set("field", label.as_str());
            words.text_with("metadata-not-saved", Some(&args))
        };
        let warning = if not_a_date.is_some() {
            let example = date_example(words.locale());
            let mut args = Args::new();
            args.set("example", example);
            let why = words.text_with("metadata-not-a-date-why", Some(&args));
            Some((words.text("metadata-not-a-date"), vec![reason(why)]))
        } else if self.failed.contains(property) {
            let why = mirror.saving.failure.clone().unwrap_or_default();
            let try_again = Item::new(TRY_AGAIN, words.text(TRY_AGAIN));
            Some((not_saved, vec![reason(why), Entry::Item(try_again)]))
        } else {
            None
        };
        let name = warning.as_ref().map(|(name, _)| name.clone());
        if let (Some(name), Some((_, menu))) = (&name, warning) {
            well = well.warning(name, menu);
        }
        let out = well.show(ui);
        if out.began {
            let start = (property, self.photos.clone(), shown.clone());
            self.editing.insert(row.id, start);
        }
        // The photos it opened on, which a commit names, and what it showed as editing
        // began, which a commit is compared with, not a summary come since.
        let began = self.editing.get(row.id).cloned();
        let (photos, shown) = match began {
            Some((_, photos, start)) => (photos, start),
            None => (self.photos.clone(), shown),
        };
        if !out.editing {
            self.editing.remove(row.id);
        }
        match out.chosen.as_deref() {
            Some(TRY_AGAIN) => commands.push(Command::RetrySave),
            Some(CLEAR_ON) => return self.write(property, None, photos, mirror, commands),
            // The value as the photos hold it, never typed again, so a list's item holding
            // "; " stays whole.
            Some(chosen) if chosen.starts_with(VALUE) => {
                let at = chosen[VALUE.len()..].parse::<usize>().ok();
                let value = match &different {
                    Some(FieldSummary::Different { values, .. }) => at
                        .and_then(|at| values.get(at))
                        .map(|(value, _)| value.clone()),
                    _ => None,
                };
                if let Some(value) = value {
                    self.write_value(property, value, photos, commands);
                }
                return;
            }
            // A choice over Different is no "nothing typed": Unknown clears it on all.
            Some(chosen) if kind == FieldKind::Marked && different.is_some() => {
                let typed = (!chosen.is_empty()).then(|| chosen.to_owned());
                return self.write(property, typed, photos, mirror, commands);
            }
            Some(chosen) if kind == FieldKind::Marked => {
                let typed = (!chosen.is_empty()).then(|| chosen.to_owned());
                let typed = typed.unwrap_or_default();
                return self.commit(def, property, typed, (photos, &shown), mirror, commands);
            }
            _ => {}
        }
        if out.cancelled && kind == FieldKind::Date {
            self.not_a_date = None;
        }
        let Some(typed) = out.committed else {
            return;
        };
        if kind == FieldKind::Date {
            let month_first = words.locale() == Locale::EnUs;
            match typed_date(&typed, month_first) {
                Some(date) => {
                    self.not_a_date = None;
                    self.commit(def, property, date, (photos, &shown), mirror, commands);
                }
                None => self.not_a_date = Some(typed),
            }
            return;
        }
        self.commit(def, property, typed, (photos, &shown), mirror, commands);
    }

    /// Sends `typed` as the value of `def`'s field, `property`, an empty one clearing it,
    /// to `photos`, the copy's name by `RenameCopy`, and shows it until the session tells
    /// it; nothing where it holds the value already, `now`, the well's text and whether it
    /// read Different, as the session would make it of what was typed
    /// (`FieldValue::from_typed`), so a space added or a list's spacing changes nothing and
    /// no write is waited for, and nothing for an empty one over Different.
    fn commit(
        &mut self,
        def: Option<&library_fields::FieldDef>,
        property: &'static str,
        typed: String,
        (photos, now): (Vec<PathBuf>, &(String, bool)),
        mirror: &Mirror,
        commands: &mut Vec<Command>,
    ) {
        let kept = state(mirror)
            .filter(|_| photos.len() < 2)
            .and_then(|state| {
                (state.fields.iter())
                    .find(|(field, _)| field == property)
                    .and_then(|(_, value)| value.clone())
            });
        // The session refuses it; nothing is sent.
        let Some(made) = made(def, &typed, kept.as_ref()) else {
            return;
        };
        if unchanged(&made, now) {
            return;
        }
        if photos.len() > 1 {
            let typed = (!made.is_empty()).then_some(typed);
            self.write(property, typed, photos, mirror, commands);
            return;
        }
        let command = if property == COPY_NAME {
            Command::RenameCopy { name: made.clone() }
        } else {
            let value = (!made.is_empty()).then(|| typed.clone());
            Command::SetMetadata {
                photos,
                fields: vec![(property.to_owned(), value)],
                values: Vec::new(),
            }
        };
        commands.push(command);
        let before = told(property, state(mirror));
        self.shown.insert(property, (made, before));
        self.unconfirmed.insert(property);
    }

    /// Sends `typed`, or `None` to clear it, as the value of `property` on several
    /// `photos`, one batch, and shows it on them until their summary tells it.
    /// Sends `value`, as it is, as the value of `property` on several `photos`, one batch,
    /// and shows it on them until their summary tells it.
    fn write_value(
        &mut self,
        property: &'static str,
        value: FieldValue,
        photos: Vec<PathBuf>,
        commands: &mut Vec<Command>,
    ) {
        let text = value.shown();
        commands.push(Command::SetMetadata {
            photos: photos.clone(),
            fields: Vec::new(),
            values: vec![(property.to_owned(), value)],
        });
        let written = Written {
            text,
            photos,
            command: None,
            ended: None,
        };
        self.written.insert(property, written);
    }

    fn write(
        &mut self,
        property: &'static str,
        typed: Option<String>,
        photos: Vec<PathBuf>,
        mirror: &Mirror,
        commands: &mut Vec<Command>,
    ) {
        if photos.len() < 2 {
            let def = library_fields::field(property);
            let now = (told(property, state(mirror)), false);
            let typed = typed.unwrap_or_default();
            self.commit(def, property, typed, (photos, &now), mirror, commands);
            return;
        }
        let shown = match library_fields::field(property) {
            Some(def) => match &typed {
                Some(typed) => match FieldValue::from_typed(def, typed, None) {
                    Ok(value) => value.map(|value| value.shown()).unwrap_or_default(),
                    Err(_) => return,
                },
                None => String::new(),
            },
            None => return,
        };
        commands.push(Command::SetMetadata {
            photos: photos.clone(),
            values: Vec::new(),
            fields: vec![(property.to_owned(), typed)],
        });
        let written = Written {
            text: shown,
            photos,
            command: None,
            ended: None,
        };
        self.written.insert(property, written);
    }
}

/// What was typed into the field `def`, the copy's name with none, as the session makes it
/// of it and a well shows it, `kept` the value it holds now; `None` for what the session
/// refuses.
fn made(
    def: Option<&library_fields::FieldDef>,
    typed: &str,
    kept: Option<&FieldValue>,
) -> Option<String> {
    match def {
        Some(def) => (FieldValue::from_typed(def, typed, kept).ok())
            .map(|value| value.map(|value| value.shown()).unwrap_or_default()),
        None => Some(typed.trim().to_owned()),
    }
}

/// Whether `made` changes nothing of `start`, what a field showed as editing began, its
/// text and whether it read Different: the same text, or over Different nothing, which
/// writes nothing (darktable's "leave unchanged"); several photos holding it apart take
/// whatever else was typed.
fn unchanged(made: &str, (text, different): &(String, bool)) -> bool {
    made == text && (!different || made.is_empty())
}

/// The label of the field `property` in `words`, "Title", as its row names it; "Metadata"
/// for a property no field of the tool's is.
pub fn label(property: &str, words: &Translator) -> String {
    match library_fields::field(property) {
        Some(def) => words.text(&messages::field_words(ID, def.id)[0]),
        None => words.text("history-metadata"),
    }
}

/// An item of a warning menu saying why, which cannot be chosen.
fn reason(why: String) -> Entry {
    Entry::Item(Item::new(REASON, why).enabled(false))
}

/// A date as the language writes it in an example: day first, by its separator, or month
/// first in American English.
fn date_example(locale: Locale) -> &'static str {
    match locale {
        Locale::EnUs => "07/04/2026",
        Locale::Uk | Locale::Pl => "04.07.2026",
        _ => "04/07/2026",
    }
}

/// What was typed into Date created as an XMP date: empty for empty, which clears it, an
/// XMP date as typed, or a day, a month and a year of four digits, by `.`, `/` or `-`,
/// the month first where `month_first`; `None` for what is no date.
pub fn typed_date(typed: &str, month_first: bool) -> Option<String> {
    let typed = typed.trim();
    if typed.is_empty() || library_fields::is_date(typed) {
        return Some(typed.to_owned());
    }
    let parts: Vec<&str> = typed.split(['.', '/', '-']).map(str::trim).collect();
    let [first, second, year] = parts[..] else {
        return None;
    };
    let (day, month) = if month_first {
        (second, first)
    } else {
        (first, second)
    };
    let number = |part: &str| -> Option<u32> {
        (part.len() <= 2 && !part.is_empty())
            .then(|| part.parse().ok())
            .flatten()
    };
    let (day, month) = (number(day)?, number(month)?);
    (year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit())).then_some(())?;
    let date = format!("{year}-{month:02}-{day:02}");
    library_fields::is_date(&date).then_some(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_typed_as_each_language_writes_it_is_an_xmp_date() {
        let cases = [
            ("04/07/2026", false, Some("2026-07-04")),
            ("4.7.2026", false, Some("2026-07-04")),
            ("07/04/2026", true, Some("2026-07-04")),
            ("2026-07-04", true, Some("2026-07-04")),
            ("2026-07", false, Some("2026-07")),
            ("  ", false, Some("")),
            ("31/02/2026", false, None),
            ("4 July 2026", false, None),
            ("04/07/26", false, None),
        ];
        for (typed, month_first, expected) in cases {
            assert_eq!(
                typed_date(typed, month_first).as_deref(),
                expected,
                "{typed}"
            );
        }
    }
}
