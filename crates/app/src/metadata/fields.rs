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

//! The Metadata tool's fields for one photo (decision 110; `docs/gui.md`, The Metadata
//! tool): each a [`FieldWell`] showing the active photo's value as the session's
//! `Metadata` fact tells it, a commit sending `SetMetadata`, or `RenameCopy` for Basic's
//! Copy, with what was typed; the value shown at once and replaced by the session's next
//! `Metadata`.
//!
//! - **Disabled** with no photo, while several are selected (#644 edits them) and on a
//!   photo open for viewing only, the tooltip saying why.
//! - **A failed write** marks each field committed since the last write that succeeded: a
//!   warning button at its well's end, its menu the reason and Try Again, which writes again
//!   (`RetrySave`); the Viewer shows the failed-edit message from the same save state.
//! - **Date created** takes a date as the language writes one, day first or, in American
//!   English, month first, or as ISO 8601, sent as an XMP date; anything else stays in the
//!   well with the warning button "Not a date" and is not sent.
//! - **Copyright status** is a pull-down of Unknown, Copyrighted and Public domain.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

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

/// The fields' state between frames.
#[derive(Debug, Default)]
pub struct Fields {
    /// A value committed and not yet told, by property, with the value the session told
    /// as it was committed: shown while the session tells that one, so a fact of another
    /// change, as a rating's, never shows the value before for a moment, and one telling
    /// anything else, the commit's or an Undo's, ends it.
    shown: HashMap<&'static str, (String, String)>,
    /// The photo each field being edited began on, by its row's id: a draft never goes to
    /// another photo.
    editing: HashMap<&'static str, PathBuf>,
    /// The fields committed since the last write the session confirmed.
    unconfirmed: BTreeSet<&'static str>,
    /// The fields whose write failed, until a write succeeds.
    failed: BTreeSet<&'static str>,
    /// What was typed in Date created that is no date, kept in its well.
    not_a_date: Option<String>,
    /// The photo the state is of, which another photo opened clears.
    photo: Option<PathBuf>,
    /// The pass the save state was last followed in.
    pass: Option<u64>,
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

/// The well of the field row `id`.
fn well_id(id: &str) -> egui::Id {
    egui::Id::unique(("metadata-field", id))
}

/// Why the fields cannot be edited now, in `words`; none where they can.
fn disabled(mirror: &Mirror, words: &Translator) -> Option<String> {
    if mirror.selected_count() > 1 {
        return Some(words.text("metadata-several-later"));
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

impl Fields {
    /// Follows the save state once a pass: a failed write marks the fields committed since
    /// the last one that succeeded, and one that succeeds clears them; another photo clears
    /// everything.
    fn follow(&mut self, ctx: &egui::Context, pass: u64, mirror: &Mirror) {
        if self.pass == Some(pass) {
            return;
        }
        self.pass = Some(pass);
        let photo = active_photo(mirror).map(|photo| photo.path.clone());
        if photo != self.photo {
            // A draft of the photo left goes with it, never to the next.
            for id in self.editing.keys() {
                FieldWell::forget(ctx, well_id(id));
            }
            *self = Fields {
                photo,
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

    /// The value of `property` as the well shows it: one committed and not yet told, of
    /// the active photo alone, else the session's.
    fn value(&self, property: &str, mirror: &Mirror) -> String {
        let this_photo = active_photo(mirror).map(|photo| &photo.path) == self.photo.as_ref();
        if let Some((text, _)) = self.shown.get(property).filter(|_| this_photo) {
            return text.clone();
        }
        told(property, state(mirror))
    }

    /// Whether `property` holds a value, as a closed group's dot counts it.
    pub fn holds(&self, property: &str, mirror: &Mirror) -> bool {
        !self.text(property, mirror).is_empty()
    }

    /// The value of `property` as its well shows it, of the active photo.
    pub fn text(&self, property: &str, mirror: &Mirror) -> String {
        self.value(property, mirror)
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

impl Fields {
    /// Draws the field `row` in its labels' `column`, a commit going into `commands`.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        row: &FieldRow,
        column: f32,
        mirror: &Mirror,
        words: &Translator,
        commands: &mut Vec<Command>,
    ) {
        self.follow(ui.ctx(), ui.ctx().cumulative_pass_nr(), mirror);
        let [label, tag] = messages::field_words(ID, row.id).map(|id| words.text(&id));
        let state = state(mirror);
        let why = disabled(mirror, words);
        let def = library_fields::field(row.field);
        let kind = def.map_or(FieldKind::Text, |def| def.kind);
        let lines = def.is_some_and(|def| def.lines);
        let mut tip = vec![tag];
        match row.field {
            "apertrust:notes" => tip.insert(0, words.text("metadata-notes-private")),
            "photoshop:DateCreated" => tip.insert(0, words.text("metadata-date-created-hint")),
            _ => {}
        }
        if let Some(why) = &why {
            tip.insert(0, why.clone());
        }
        let tip = tip.join("\n");
        let property = def.map_or(COPY_NAME, |def| def.property);
        let not_a_date = (kind == FieldKind::Date)
            .then(|| self.not_a_date.clone())
            .flatten();
        let text = not_a_date
            .clone()
            .unwrap_or_else(|| self.value(property, mirror));
        let copy_of = state.map(|state| {
            let mut args = Args::new();
            args.set("number", state.copy.number as f64);
            args.set("count", state.copy.count as f64);
            words.text_with("metadata-copy-of", Some(&args))
        });
        let id = well_id(row.id);
        // A draft begun on another photo, which changed with no press to end it, as from
        // the menu bar or an open from the Finder, is forgotten, never sent to this one.
        let open = open_photo(mirror).map(|photo| &photo.path);
        if let Some(began) = self.editing.get(row.id)
            && Some(began) != open
        {
            FieldWell::forget(ui.ctx(), id);
            self.editing.remove(row.id);
        }
        let options = [
            ("", words.text("metadata-copyright-unknown")),
            ("True", words.text("metadata-copyright-copyrighted")),
            ("False", words.text("metadata-copyright-public-domain")),
        ];
        let options: Vec<(&str, &str)> = options.iter().map(|(id, w)| (*id, w.as_str())).collect();
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
        if out.began
            && let Some(open) = open
        {
            self.editing.insert(row.id, open.clone());
        }
        if !out.editing {
            self.editing.remove(row.id);
        }
        match out.chosen.as_deref() {
            Some(TRY_AGAIN) => commands.push(Command::RetrySave),
            Some(chosen) if kind == FieldKind::Marked => {
                let typed = (!chosen.is_empty()).then(|| chosen.to_owned());
                self.commit(def, property, typed.unwrap_or_default(), mirror, commands);
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
                    self.commit(def, property, date, mirror, commands);
                }
                None => self.not_a_date = Some(typed),
            }
            return;
        }
        self.commit(def, property, typed, mirror, commands);
    }

    /// Sends `typed` as the value of `def`'s field, `property`, an empty one clearing it,
    /// the copy's name by `RenameCopy`, and shows it until the session tells it; nothing
    /// where it holds the value already as the session would make it of what was typed
    /// (`FieldValue::from_typed`), so a space added or a list's spacing changes nothing and
    /// no write is waited for.
    fn commit(
        &mut self,
        def: Option<&library_fields::FieldDef>,
        property: &'static str,
        typed: String,
        mirror: &Mirror,
        commands: &mut Vec<Command>,
    ) {
        let now = self.value(property, mirror);
        let made = match def {
            Some(def) => {
                let kept = state(mirror).and_then(|state| {
                    (state.fields.iter())
                        .find(|(field, _)| field == property)
                        .and_then(|(_, value)| value.clone())
                });
                match FieldValue::from_typed(def, &typed, kept.as_ref()) {
                    Ok(value) => value.map(|value| value.shown()).unwrap_or_default(),
                    // The session refuses it; nothing is sent.
                    Err(_) => return,
                }
            }
            None => typed.trim().to_owned(),
        };
        if made == now {
            return;
        }
        let command = if property == COPY_NAME {
            Command::RenameCopy { name: made.clone() }
        } else {
            let value = (!made.is_empty()).then(|| typed.clone());
            Command::SetMetadata {
                fields: vec![(property.to_owned(), value)],
            }
        };
        commands.push(command);
        let before = told(property, state(mirror));
        self.shown.insert(property, (made, before));
        self.unconfirmed.insert(property);
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
