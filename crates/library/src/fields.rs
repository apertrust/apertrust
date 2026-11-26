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

//! A virtual copy's fields, its values beside the edit (decision 110, `docs/library.md`,
//! Metadata beside the edit): the photo's current values, each with the clock of its last
//! change, never steps of its history. Built: the rating's stars, the reject and the colour
//! labels ([`Rating`]), and the Metadata tool's fields, IPTC Core's, Dublin Core's publisher
//! and Apertrust's notes, typed as text ([`FieldValue::from_typed`]).
//!
//! Every field is data, [`FIELDS`]: its id, its property and its kind. A copy holds an
//! entry for each field ever set ([`FieldEntry`]), its clock and its value, a cleared one
//! kept with no value, so the clear wins over an older value on another computer. In the
//! `.apxmp` they are the copy's `apertrust:fields`, after its clock vector, in the order of
//! their properties, so two machines write the same bytes, each value in the standard's
//! form: a language alternative with its `x-default` item first, an `rdf:Seq`, an
//! `rdf:Bag`, `True` or `False`, or text:
//!
//! ```xml
//! <apertrust:fields>
//!  <rdf:Bag>
//!   <rdf:li>
//!    <rdf:Description apertrust:field="dc:title"
//!      apertrust:when="2026-11-08T10:42:01.120Z" apertrust:counter="0"
//!      apertrust:machine="5e1f22c9">
//!     <apertrust:value>
//!      <rdf:Alt><rdf:li xml:lang="x-default">Morning market, Alfama</rdf:li></rdf:Alt>
//!     </apertrust:value>
//!    </rdf:Description>
//!   </rdf:li>
//!   <rdf:li>
//!    <rdf:Description apertrust:field="xmp:Rating"
//!      apertrust:when="2026-11-08T10:43:12.004Z" apertrust:counter="0"
//!      apertrust:machine="5e1f22c9" apertrust:value="3"/>
//!   </rdf:li>
//! ```
//!
//! The copy's name is no entry: it stays the copy's `apertrust:name`, with the clock of its
//! last change in `apertrust:nameClock` where it was set ([`COPY_NAME`], [`set_in`]).
//!
//! [`set`] changes one; [`join`] combines two copies' entries field by field, the later
//! clock winning each (decision 52), and [`join_name`] their names. What cannot be read as
//! written is a [`Problem::Field`], so the photo opens for viewing only and nothing is
//! written over it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::clock::{self, Clock};
use crate::sidecar::{NAMESPACE, Problem, VirtualCopy};
use crate::xmp::{ArrayKind, DEFAULT_LANGUAGE, Name, Node, Property, Value, XML};

/// The namespace of XMP's basic properties, `xmp:Rating`'s.
pub const XMP_NAMESPACE: &str = "http://ns.adobe.com/xap/1.0/";
/// darktable's namespace, `darktable:colorlabels`'s.
pub const DARKTABLE_NAMESPACE: &str = "http://darktable.sf.net/";
/// Dublin Core's, `dc:title`'s.
pub const DC_NAMESPACE: &str = "http://purl.org/dc/elements/1.1/";
/// The namespace IPTC Core keeps its older fields in, `photoshop:City`'s.
pub const PHOTOSHOP_NAMESPACE: &str = "http://ns.adobe.com/photoshop/1.0/";
/// IPTC Core's own, `Iptc4xmpCore:Location`'s.
pub const IPTC_CORE_NAMESPACE: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";
/// XMP's rights namespace, `xmpRights:Marked`'s.
pub const RIGHTS_NAMESPACE: &str = "http://ns.adobe.com/xap/1.0/rights/";

/// What a field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Stars, 0 to 5, as `xmp:Rating` writes them.
    Stars,
    /// `True` or `False`, as XMP writes a boolean.
    Marked,
    /// A set of darktable's colour labels, 0 red to 4 purple, an `rdf:Seq` of their numbers
    /// as darktable writes `darktable:colorlabels`.
    Labels,
    Text,
    /// A language alternative, an `rdf:Alt`, its `x-default` item the one typed.
    LangAlt,
    /// An ordered list, an `rdf:Seq`, typed with "; " between its items.
    Seq,
    /// An unordered list, an `rdf:Bag`, typed with "; " between its items.
    Bag,
    /// An XMP date, `2026-07-04`, or with its time ([`is_date`]).
    Date,
}

/// One field: its id, the property it is stored under and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldDef {
    /// Its id, as clients name it.
    pub id: &'static str,
    /// Its property's namespace.
    pub namespace: &'static str,
    /// Its property's qualified name, `prefix:name`, as `apertrust:field` holds it.
    pub property: &'static str,
    pub kind: FieldKind,
    /// The struct the standard keeps it in, as `Iptc4xmpCore:CreatorContactInfo` keeps the
    /// creator's contact details; none for a property of its own.
    pub within: Option<&'static str>,
    /// Whether it takes line breaks: Description and Notes.
    pub lines: bool,
    /// Whether it is private, left out of an exported file unless Export's Metadata group
    /// includes it: Notes.
    pub private: bool,
}

/// The stars, kept while the photo is rejected ("a reject that keeps the stars").
pub const RATING: &str = "xmp:Rating";
/// The reject, a field of its own: darktable's −1 would lose the stars beneath it.
pub const REJECTED: &str = "apertrust:rejected";
/// The colour labels.
pub const COLOUR_LABELS: &str = "darktable:colorlabels";
/// The copy's name: no entry of its fields but its `apertrust:name`, which a change, its
/// undo and a client name by this as they name a field ([`value_in`], [`set_in`]).
pub const COPY_NAME: &str = "apertrust:name";
/// The struct IPTC Core keeps the creator's contact details in.
const CONTACT: Option<&str> = Some("Iptc4xmpCore:CreatorContactInfo");

/// A field `id` of `namespace` under `property`, of `kind`: a property of its own, one
/// logical line, not private.
const fn def(
    id: &'static str,
    namespace: &'static str,
    property: &'static str,
    kind: FieldKind,
) -> FieldDef {
    FieldDef {
        id,
        namespace,
        property,
        kind,
        within: None,
        lines: false,
        private: false,
    }
}

/// A member `id` of the creator's contact details, text.
const fn contact(id: &'static str, property: &'static str) -> FieldDef {
    FieldDef {
        within: CONTACT,
        ..def(id, IPTC_CORE_NAMESPACE, property, FieldKind::Text)
    }
}

/// An IPTC Core field `id` of the Photoshop namespace, text.
const fn photoshop(id: &'static str, property: &'static str) -> FieldDef {
    def(id, PHOTOSHOP_NAMESPACE, property, FieldKind::Text)
}

/// Every field, in the order of their properties: the Metadata tool's (`docs/gui.md`, The
/// Metadata tool) and the rating's.
pub const FIELDS: &[FieldDef] = &[
    contact("contact_city", "Iptc4xmpCore:CiAdrCity"),
    contact("contact_country", "Iptc4xmpCore:CiAdrCtry"),
    contact("address", "Iptc4xmpCore:CiAdrExtadr"),
    contact("postal_code", "Iptc4xmpCore:CiAdrPcode"),
    contact("contact_region", "Iptc4xmpCore:CiAdrRegion"),
    contact("email", "Iptc4xmpCore:CiEmailWork"),
    contact("phone", "Iptc4xmpCore:CiTelWork"),
    contact("website", "Iptc4xmpCore:CiUrlWork"),
    def(
        "country_code",
        IPTC_CORE_NAMESPACE,
        "Iptc4xmpCore:CountryCode",
        FieldKind::Text,
    ),
    def(
        "genre",
        IPTC_CORE_NAMESPACE,
        "Iptc4xmpCore:IntellectualGenre",
        FieldKind::Text,
    ),
    def(
        "sublocation",
        IPTC_CORE_NAMESPACE,
        "Iptc4xmpCore:Location",
        FieldKind::Text,
    ),
    def(
        "scene_code",
        IPTC_CORE_NAMESPACE,
        "Iptc4xmpCore:Scene",
        FieldKind::Bag,
    ),
    def(
        "subject_code",
        IPTC_CORE_NAMESPACE,
        "Iptc4xmpCore:SubjectCode",
        FieldKind::Bag,
    ),
    FieldDef {
        lines: true,
        private: true,
        ..def("notes", NAMESPACE, "apertrust:notes", FieldKind::Text)
    },
    def("rejected", NAMESPACE, REJECTED, FieldKind::Marked),
    def(
        "colour_labels",
        DARKTABLE_NAMESPACE,
        COLOUR_LABELS,
        FieldKind::Labels,
    ),
    def("creator", DC_NAMESPACE, "dc:creator", FieldKind::Seq),
    FieldDef {
        lines: true,
        ..def(
            "description",
            DC_NAMESPACE,
            "dc:description",
            FieldKind::LangAlt,
        )
    },
    def("publisher", DC_NAMESPACE, "dc:publisher", FieldKind::Bag),
    def(
        "copyright_notice",
        DC_NAMESPACE,
        "dc:rights",
        FieldKind::LangAlt,
    ),
    def("title", DC_NAMESPACE, "dc:title", FieldKind::LangAlt),
    photoshop("job_title", "photoshop:AuthorsPosition"),
    photoshop("description_writer", "photoshop:CaptionWriter"),
    photoshop("city", "photoshop:City"),
    photoshop("country", "photoshop:Country"),
    photoshop("credit_line", "photoshop:Credit"),
    def(
        "date_created",
        PHOTOSHOP_NAMESPACE,
        "photoshop:DateCreated",
        FieldKind::Date,
    ),
    photoshop("headline", "photoshop:Headline"),
    photoshop("instructions", "photoshop:Instructions"),
    photoshop("source", "photoshop:Source"),
    photoshop("state", "photoshop:State"),
    photoshop("job_id", "photoshop:TransmissionReference"),
    def("rating", XMP_NAMESPACE, RATING, FieldKind::Stars),
    def(
        "copyright_status",
        RIGHTS_NAMESPACE,
        "xmpRights:Marked",
        FieldKind::Marked,
    ),
    def(
        "usage_terms",
        RIGHTS_NAMESPACE,
        "xmpRights:UsageTerms",
        FieldKind::LangAlt,
    ),
    def(
        "web_statement",
        RIGHTS_NAMESPACE,
        "xmpRights:WebStatement",
        FieldKind::Text,
    ),
];

/// The field stored under `property`, a qualified name.
pub fn field(property: &str) -> Option<&'static FieldDef> {
    FIELDS.iter().find(|def| def.property == property)
}

/// Whether `property` is the rating's, its stars, reject or labels, which
/// [`RatingChange`] sets, never typing.
pub fn is_rating(property: &str) -> bool {
    [RATING, REJECTED, COLOUR_LABELS].contains(&property)
}

/// A field's value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FieldValue {
    /// Stars, 0 to 5.
    Stars(u8),
    Marked(bool),
    /// The colour labels on, a bit each in darktable's order, bit 0 red to bit 4 purple.
    Labels(u8),
    Text(String),
    /// A language alternative's items, each its language and text, `x-default` first
    /// where it was typed.
    LangAlt(Vec<(String, String)>),
    /// A `Seq`'s or a `Bag`'s items, in order.
    List(Vec<String>),
    /// An XMP date as written ([`is_date`]).
    Date(String),
}

/// What [`FieldValue::from_typed`] cannot take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
pub enum TypedError {
    /// A date field's value that is no XMP date.
    #[error("not a date")]
    NotADate,
    /// A mark's value that is neither `True` nor `False`.
    #[error("neither True nor False")]
    NotMarked,
    /// The rating's stars or labels, which [`RatingChange`] sets.
    #[error("set by a rating, not typed")]
    NotTyped,
}

/// What a list's items are typed and shown with between them.
pub const LIST_SEPARATOR: &str = "; ";

impl FieldValue {
    /// Whether it is a value of `kind`.
    pub fn is(&self, kind: FieldKind) -> bool {
        matches!(
            (self, kind),
            (FieldValue::Stars(_), FieldKind::Stars)
                | (FieldValue::Marked(_), FieldKind::Marked)
                | (FieldValue::Labels(_), FieldKind::Labels)
                | (FieldValue::Text(_), FieldKind::Text)
                | (FieldValue::LangAlt(_), FieldKind::LangAlt)
                | (FieldValue::List(_), FieldKind::Seq | FieldKind::Bag)
                | (FieldValue::Date(_), FieldKind::Date)
        )
    }

    /// As a person reads and types it: a language alternative's `x-default` item, else its
    /// first; a list's items with "; " between them; `True` or `False`; text, a date and
    /// stars as written; the labels' numbers with "; " between them.
    pub fn shown(&self) -> String {
        match self {
            FieldValue::Text(text) | FieldValue::Date(text) => text.clone(),
            FieldValue::LangAlt(items) => (items.iter())
                .find(|(lang, _)| lang == DEFAULT_LANGUAGE)
                .or(items.first())
                .map(|(_, text)| text.clone())
                .unwrap_or_default(),
            FieldValue::List(items) => items.join(LIST_SEPARATOR),
            FieldValue::Marked(on) => (if *on { "True" } else { "False" }).to_owned(),
            FieldValue::Stars(stars) => stars.to_string(),
            FieldValue::Labels(bits) => (0..LABELS)
                .filter(|label| bits & (1 << label) != 0)
                .map(|label| label.to_string())
                .collect::<Vec<_>>()
                .join(LIST_SEPARATOR),
        }
    }

    /// What a person typed into the field `def`, as the field then holds it, `kept` what it
    /// holds now: on one line, its line breaks spaces, where the field takes none; trimmed
    /// of spaces at both ends, as darktable trims its metadata
    /// (`src/common/metadata.c:402-417`), `None` for an empty value, which clears it; for a
    /// language alternative its `x-default` item, first, its other languages kept from
    /// `kept`; for a list its items split at `;`, each trimmed, the empty left out; `True`
    /// or `False`; an XMP date ([`is_date`]). The rating's stars and labels are set
    /// otherwise ([`TypedError::NotTyped`]).
    pub fn from_typed(
        def: &FieldDef,
        typed: &str,
        kept: Option<&FieldValue>,
    ) -> Result<Option<FieldValue>, TypedError> {
        let joined = typed.lines().collect::<Vec<_>>().join(" ");
        let typed = if def.lines { typed } else { &joined };
        let typed = typed.trim();
        if typed.is_empty() {
            return Ok(None);
        }
        let value = match def.kind {
            FieldKind::Text => FieldValue::Text(typed.to_owned()),
            FieldKind::LangAlt => {
                let mut items = vec![(DEFAULT_LANGUAGE.to_owned(), typed.to_owned())];
                if let Some(FieldValue::LangAlt(others)) = kept {
                    let others = others.iter().filter(|(lang, _)| lang != DEFAULT_LANGUAGE);
                    items.extend(others.cloned());
                }
                FieldValue::LangAlt(items)
            }
            FieldKind::Seq | FieldKind::Bag => {
                let items: Vec<String> = (typed.split(';'))
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect();
                if items.is_empty() {
                    return Ok(None);
                }
                FieldValue::List(items)
            }
            FieldKind::Marked => match typed {
                "True" => FieldValue::Marked(true),
                "False" => FieldValue::Marked(false),
                _ => return Err(TypedError::NotMarked),
            },
            FieldKind::Date if is_date(typed) => FieldValue::Date(typed.to_owned()),
            FieldKind::Date => return Err(TypedError::NotADate),
            FieldKind::Stars | FieldKind::Labels => return Err(TypedError::NotTyped),
        };
        Ok(Some(value))
    }

    /// As the file holds it, for a field of `kind`: stars as their number, a mark as `True`
    /// or `False`, labels as an `rdf:Seq` of their numbers, lowest first; text and a date
    /// as text; a language alternative as an `rdf:Alt` of its items with their languages;
    /// a list as an `rdf:Seq`, or an `rdf:Bag` for a bag.
    fn node(&self, kind: FieldKind) -> Node {
        match self {
            FieldValue::Stars(stars) => Node::simple(&stars.to_string()),
            FieldValue::Marked(on) => Node::simple(if *on { "True" } else { "False" }),
            FieldValue::Labels(bits) => Node::array(
                ArrayKind::Seq,
                (0..LABELS)
                    .filter(|label| bits & (1 << label) != 0)
                    .map(|label| Node::simple(&label.to_string()))
                    .collect(),
            ),
            FieldValue::Text(text) | FieldValue::Date(text) => Node::simple(text),
            FieldValue::LangAlt(items) => Node::array(
                ArrayKind::Alt,
                (items.iter())
                    .map(|(lang, text)| Node::with_lang(text, lang))
                    .collect(),
            ),
            FieldValue::List(items) => {
                let array = match kind {
                    FieldKind::Bag => ArrayKind::Bag,
                    _ => ArrayKind::Seq,
                };
                Node::array(array, items.iter().map(|item| Node::simple(item)).collect())
            }
        }
    }

    /// A value of `kind` as the file holds it, [`Self::node`]'s inverse; `None` for one of
    /// another form: stars past 5, a label past 4, an alternative's item with no language
    /// or another qualifier, a list's item with any or that is no text, and a date that is
    /// none, among them.
    fn read(node: &Node, kind: FieldKind) -> Option<FieldValue> {
        if !node.qualifiers.is_empty() {
            return None;
        }
        match (kind, &node.value) {
            (FieldKind::Stars, Value::Simple(text)) => (text.parse().ok())
                .filter(|stars| *stars <= MAX_STARS)
                .map(FieldValue::Stars),
            (FieldKind::Marked, Value::Simple(text)) => match text.as_str() {
                "True" => Some(FieldValue::Marked(true)),
                "False" => Some(FieldValue::Marked(false)),
                _ => None,
            },
            (FieldKind::Labels, Value::Array(ArrayKind::Seq, items)) => {
                let mut bits = 0;
                for item in items {
                    let label: u8 = (item.as_str()?.parse().ok())
                        .filter(|label| *label < LABELS)
                        .filter(|_| item.qualifiers.is_empty())?;
                    bits |= 1 << label;
                }
                Some(FieldValue::Labels(bits))
            }
            (FieldKind::Text, Value::Simple(text)) => Some(FieldValue::Text(text.clone())),
            (FieldKind::Date, Value::Simple(text)) if is_date(text) => {
                Some(FieldValue::Date(text.clone()))
            }
            (FieldKind::LangAlt, Value::Array(ArrayKind::Alt, items)) => (items.iter())
                .map(|item| match (&item.value, item.qualifiers.as_slice()) {
                    (Value::Simple(text), [lang]) if lang.name.is(XML, "lang") => {
                        Some((lang.value.clone(), text.clone()))
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(FieldValue::LangAlt),
            (FieldKind::Seq, Value::Array(ArrayKind::Seq, items))
            | (FieldKind::Bag, Value::Array(ArrayKind::Bag, items)) => (items.iter())
                .map(|item| match &item.value {
                    Value::Simple(text) if item.qualifiers.is_empty() => Some(text.clone()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(FieldValue::List),
            _ => None,
        }
    }
}

/// Whether `text` is an XMP date, ISO 8601 as XMP takes it: a year, a year and its month, a
/// date, or a date and its hours and minutes, then its seconds and their fraction where it
/// has them, then `Z` or an offset where it has one (`2026`, `2026-07`, `2026-07-04`,
/// `2026-07-04T08:10:01.5+01:00`).
pub fn is_date(text: &str) -> bool {
    let digits =
        |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
    let number = |part: &str| part.parse::<u32>().unwrap_or(u32::MAX);
    let (date, time) = match text.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let parts: Vec<&str> = date.split('-').collect();
    let whole_date = match parts[..] {
        [year] => digits(year, 4),
        [year, month] => digits(year, 4) && digits(month, 2) && (1..=12).contains(&number(month)),
        [year, month, day] => {
            let year_n = number(year);
            let leap = year_n % 4 == 0 && (year_n % 100 != 0 || year_n % 400 == 0);
            let days = match number(month) {
                2 if leap => 29,
                2 => 28,
                4 | 6 | 9 | 11 => 30,
                1..=12 => 31,
                _ => 0,
            };
            digits(year, 4)
                && digits(month, 2)
                && digits(day, 2)
                && (1..=days).contains(&number(day))
        }
        _ => false,
    };
    let Some(time) = time else {
        return whole_date;
    };
    // A time needs the whole date before it.
    if !whole_date || parts.len() != 3 {
        return false;
    }
    let (clock, zone) = time.split_at(time.find(['Z', '+', '-']).unwrap_or(time.len()));
    let zone_reads = match zone {
        "" | "Z" => true,
        _ if !zone.starts_with(['+', '-']) => false,
        _ => zone[1..].split_once(':').is_some_and(|(hours, minutes)| {
            digits(hours, 2) && digits(minutes, 2) && number(hours) <= 23 && number(minutes) <= 59
        }),
    };
    let (clock, fraction) = match clock.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (clock, None),
    };
    let fraction_reads =
        fraction.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()));
    let two = |part: &str, most: u32| digits(part, 2) && number(part) <= most;
    let clock_reads = match clock.split(':').collect::<Vec<_>>()[..] {
        [hours, minutes] => fraction.is_none() && two(hours, 23) && two(minutes, 59),
        [hours, minutes, seconds] => {
            two(hours, 23) && two(minutes, 59) && two(seconds, 59) && fraction_reads
        }
        _ => false,
    };
    clock_reads && zone_reads
}

/// The most stars a photo has.
pub const MAX_STARS: u8 = 5;
/// How many colour labels there are, darktable's red, yellow, green, blue and purple.
pub const LABELS: u8 = 5;

/// One field a copy holds: its property, the clock of its last change and its value, none
/// where it was cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEntry {
    /// The property's qualified name, as [`FieldDef::property`] gives it.
    pub field: String,
    /// The clock of its last change; `None` for a value not yet written, as darktable's
    /// read from its `.xmp`, which takes the clock of the write that copies it in.
    pub clock: Option<Clock>,
    pub value: Option<FieldValue>,
}

impl FieldEntry {
    /// A value read from elsewhere, darktable's `.xmp`, with no clock yet.
    pub fn unclocked(field: &str, value: FieldValue) -> FieldEntry {
        FieldEntry {
            field: field.to_owned(),
            clock: None,
            value: Some(value),
        }
    }
}

/// Sets `field` among `entries` to `value`, `None` clearing it, at `clock`: its entry
/// replaced, or added in its property's place. The caller advances the copy's clock vector
/// with `clock`, as a move of the end does.
pub fn set(entries: &mut Vec<FieldEntry>, field: &str, value: Option<FieldValue>, clock: Clock) {
    let entry = FieldEntry {
        field: field.to_owned(),
        clock: Some(clock),
        value,
    };
    match entries.iter_mut().find(|kept| kept.field == field) {
        Some(kept) => *kept = entry,
        None => {
            entries.push(entry);
            entries.sort_by(|a, b| a.field.cmp(&b.field));
        }
    }
}

/// The value `field` holds among `entries`, `None` where it is unset or cleared.
pub fn value(entries: &[FieldEntry], field: &str) -> Option<FieldValue> {
    (entries.iter())
        .find(|entry| entry.field == field)
        .and_then(|entry| entry.value.clone())
}

/// The value `field` holds in `copy`: one of its fields, or with [`COPY_NAME`] its name as
/// text, `None` where it has none.
pub fn value_in(copy: &VirtualCopy, field: &str) -> Option<FieldValue> {
    match field {
        COPY_NAME => (!copy.name.is_empty()).then(|| FieldValue::Text(copy.name.clone())),
        _ => value(&copy.fields, field),
    }
}

/// Sets `field` in `copy` to `value` at `clock`, as [`set`] does, or with [`COPY_NAME`] its
/// name, empty for `None`, and the name's clock.
pub fn set_in(copy: &mut VirtualCopy, field: &str, value: Option<FieldValue>, clock: Clock) {
    match field {
        COPY_NAME => {
            copy.name = value.map(|value| value.shown()).unwrap_or_default();
            copy.name_clock = Some(clock);
        }
        _ => set(&mut copy.fields, field, value, clock),
    }
}

/// Two copies' entries combined field by field (decision 52): for each field, the entry
/// with the later clock, a cleared one included, one with no clock losing to one with a
/// clock; the same whichever side is `ours`, since clocks order totally.
pub fn join(ours: &[FieldEntry], theirs: &[FieldEntry]) -> Vec<FieldEntry> {
    let mut joined: BTreeMap<&str, &FieldEntry> = BTreeMap::new();
    for entry in ours.iter().chain(theirs) {
        let later = |kept: &&FieldEntry| entry.clock > kept.clock;
        if joined.get(entry.field.as_str()).is_none_or(later) {
            joined.insert(&entry.field, entry);
        }
    }
    joined.into_values().cloned().collect()
}

/// Two copies' names combined as [`join`] combines a field: the name with the later clock,
/// one with no clock losing to one with a clock, and `ours` where neither has one.
pub fn join_name(ours: &VirtualCopy, theirs: &VirtualCopy) -> (String, Option<Clock>) {
    if theirs.name_clock > ours.name_clock {
        (theirs.name.clone(), theirs.name_clock)
    } else {
        (ours.name.clone(), ours.name_clock)
    }
}

/// A photo's rating as the Browser and the caption show it: its stars, kept under a
/// reject, whether it is rejected, and its colour labels, a bit each, bit 0 red.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rating {
    pub stars: u8,
    pub rejected: bool,
    pub labels: u8,
}

impl Rating {
    /// The rating `entries` hold: no stars, not rejected and no label where unset.
    pub fn of(entries: &[FieldEntry]) -> Rating {
        let stars = match value(entries, RATING) {
            Some(FieldValue::Stars(stars)) => stars,
            _ => 0,
        };
        let rejected = value(entries, REJECTED) == Some(FieldValue::Marked(true));
        let labels = match value(entries, COLOUR_LABELS) {
            Some(FieldValue::Labels(bits)) => bits,
            _ => 0,
        };
        Rating {
            stars,
            rejected,
            labels,
        }
    }

    /// darktable's single rating, `xmp:Rating` as it writes it: −1 while rejected, else the
    /// stars (`src/common/image.c:600-605`).
    pub fn darktable(&self) -> i8 {
        if self.rejected { -1 } else { self.stars as i8 }
    }
}

/// A change of a photo's rating (decision 110), the value given, never a toggle, so a
/// script's command means one thing; a client works out which way a toggle goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RatingChange {
    /// So many stars, 0 to 5, and the reject taken back, as a star's click does.
    Stars(u8),
    /// The reject set or taken back, the stars kept beneath it.
    Rejected(bool),
    /// One label, 0 red to 4 purple, on or off.
    Label { label: u8, on: bool },
    /// Every label off.
    ClearLabels,
}

/// One field a change set: its value before and after it, `None` where unset or cleared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    pub field: String,
    pub before: Option<FieldValue>,
    pub after: Option<FieldValue>,
}

impl RatingChange {
    /// Whether `self` and `other` change the same thing, so quick repeats of one key are
    /// one undo: stars and stars, the reject and the reject, one label and the same label,
    /// a clear and a clear.
    pub fn same_kind(&self, other: &RatingChange) -> bool {
        match (self, other) {
            (RatingChange::Label { label: a, .. }, RatingChange::Label { label: b, .. }) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }

    /// Sets what it changes among `entries` at `clock`: each field whose value it changes,
    /// unset counting as no stars, no reject and no label, and none that it leaves as it
    /// is. What it changed, in the order of the properties.
    pub fn apply(&self, entries: &mut Vec<FieldEntry>, clock: Clock) -> Vec<FieldChange> {
        let rating = Rating::of(entries);
        let labels = |bits: u8| (COLOUR_LABELS, FieldValue::Labels(bits));
        let wanted = match *self {
            RatingChange::Stars(stars) => vec![
                (REJECTED, FieldValue::Marked(false)),
                (RATING, FieldValue::Stars(stars.min(MAX_STARS))),
            ],
            RatingChange::Rejected(on) => vec![(REJECTED, FieldValue::Marked(on))],
            RatingChange::Label { label, on } if label < LABELS => {
                let bit = 1 << label;
                let bits = if on {
                    rating.labels | bit
                } else {
                    rating.labels & !bit
                };
                vec![labels(bits)]
            }
            RatingChange::Label { .. } => Vec::new(),
            RatingChange::ClearLabels => vec![labels(0)],
        };
        let now = |field: &str| match field {
            RATING => FieldValue::Stars(rating.stars),
            REJECTED => FieldValue::Marked(rating.rejected),
            _ => FieldValue::Labels(rating.labels),
        };
        let mut changed = Vec::new();
        for (field, after) in wanted {
            if now(field) == after {
                continue;
            }
            changed.push(FieldChange {
                field: field.to_owned(),
                before: value(entries, field),
                after: Some(after.clone()),
            });
            set(entries, field, Some(after), clock);
        }
        changed
    }
}

/// `entries` as the copy's `apertrust:fields`, an `rdf:Bag` of structs in the order of
/// their properties: each its field, its clock and its value where it has one.
pub(crate) fn node(entries: &[FieldEntry]) -> Node {
    let mut sorted: Vec<&FieldEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.field.cmp(&b.field));
    let items = sorted.into_iter().map(|entry| {
        let own = |local: &str, text: &str| {
            Property::simple(Name::new("apertrust", NAMESPACE, local), text)
        };
        let mut fields = vec![own("field", &entry.field)];
        let clock = entry.clock.iter().flat_map(Clock::fields);
        fields.extend(clock.map(|(local, text)| own(local, &text)));
        if let Some(value) = &entry.value {
            let kind = field(&entry.field).map_or(FieldKind::Text, |def| def.kind);
            fields.push(Property::new(
                Name::new("apertrust", NAMESPACE, "value"),
                value.node(kind),
            ));
        }
        Node::structure(fields)
    });
    Node::array(ArrayKind::Bag, items.collect())
}

/// The entries a copy's `apertrust:fields` holds, `copy` its id, what cannot be read as
/// written added to `problems` as [`Problem::Field`] and the entry left out: a list that is
/// no `rdf:Bag`, an item that is no struct, an entry naming no field or one this build
/// does not know, a field twice, a field of an entry that is not its own, a clock that
/// cannot be read, and a value of the wrong form.
pub(crate) fn read(node: &Node, copy: &str, problems: &mut Vec<Problem>) -> Vec<FieldEntry> {
    let mut problem = |field: &str, what: &'static str| {
        problems.push(Problem::Field {
            copy: copy.to_owned(),
            field: field.to_owned(),
            what,
        });
    };
    if !matches!(node.value, Value::Array(ArrayKind::Bag, _)) || !node.qualifiers.is_empty() {
        problem("", "a list that is no bag");
    }
    let mut entries: Vec<FieldEntry> = Vec::new();
    for item in node.items().unwrap_or_default() {
        let Some(fields) = item.fields().filter(|_| item.qualifiers.is_empty()) else {
            problem("", "an item that is no struct");
            continue;
        };
        let text = |local: &str| {
            (fields.iter())
                .find(|field| field.name.is(NAMESPACE, local))
                .and_then(|field| field.node.as_str())
                .unwrap_or_default()
        };
        let name = text("field");
        let Some(def) = field(name) else {
            problem(name, "a field this build does not know");
            continue;
        };
        const KNOWN: [&str; 5] = [
            "field",
            clock::WHEN,
            clock::COUNTER,
            clock::MACHINE,
            "value",
        ];
        let odd = fields.iter().enumerate().any(|(at, field)| {
            let local = field.name.local.as_str();
            let simple = local == "value"
                || (matches!(field.node.value, Value::Simple(_))
                    && field.node.qualifiers.is_empty());
            // By its namespace and local name, whatever prefix each is written with.
            let twice = (fields[..at].iter())
                .any(|before| before.name.is(&field.name.namespace, &field.name.local));
            !field.name.is(NAMESPACE, local) || !KNOWN.contains(&local) || !simple || twice
        });
        if odd {
            problem(name, "what is not an entry's own");
            continue;
        }
        let Ok(clock) = Clock::from_fields(
            text(clock::WHEN),
            text(clock::COUNTER),
            text(clock::MACHINE),
        ) else {
            problem(name, "a clock that cannot be read");
            continue;
        };
        let value = (fields.iter()).find(|field| field.name.is(NAMESPACE, "value"));
        let value = match value.map(|field| FieldValue::read(&field.node, def.kind)) {
            Some(Some(value)) => Some(value),
            Some(None) => {
                problem(name, "a value of the wrong form");
                continue;
            }
            None => None,
        };
        if entries.iter().any(|kept| kept.field == name) {
            problem(name, "a field twice");
            continue;
        }
        entries.push(FieldEntry {
            field: name.to_owned(),
            clock: Some(clock),
            value,
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::MachineId;

    fn clock(when: u64) -> Clock {
        Clock {
            when,
            counter: 0,
            machine: MachineId::UNKNOWN,
        }
    }

    fn entry(field: &str, when: Option<u64>, value: Option<FieldValue>) -> FieldEntry {
        FieldEntry {
            field: field.to_owned(),
            clock: when.map(clock),
            value,
        }
    }

    fn alt(text: &str) -> FieldValue {
        FieldValue::LangAlt(vec![(DEFAULT_LANGUAGE.to_owned(), text.to_owned())])
    }

    #[test]
    fn every_field_is_known_by_its_property_in_order() {
        let properties: Vec<&str> = FIELDS.iter().map(|def| def.property).collect();
        let mut sorted = properties.clone();
        sorted.sort_unstable();
        assert_eq!(properties, sorted);
        for def in FIELDS {
            assert_eq!(field(def.property), Some(def));
            let prefix = def.property.split(':').next().unwrap();
            assert!(!prefix.is_empty() && def.property.len() > prefix.len() + 1);
        }
        let ids: std::collections::BTreeSet<&str> = FIELDS.iter().map(|def| def.id).collect();
        assert_eq!(ids.len(), FIELDS.len());
        assert_eq!(
            field("dc:title").map(|def| def.kind),
            Some(FieldKind::LangAlt)
        );
        assert_eq!(field("dc:subject"), None);
        let lines: Vec<&str> = (FIELDS.iter().filter(|def| def.lines))
            .map(|def| def.id)
            .collect();
        assert_eq!(lines, ["notes", "description"]);
        assert!(field("apertrust:notes").unwrap().private);
        // The Metadata tool's 33 and the rating's three.
        let typed = FIELDS.iter().filter(|def| !is_rating(def.property));
        assert_eq!((typed.count(), FIELDS.len()), (33, 36));
    }

    #[test]
    fn each_value_reads_back_as_written() {
        let values = [
            (FieldKind::Stars, FieldValue::Stars(0)),
            (FieldKind::Stars, FieldValue::Stars(5)),
            (FieldKind::Marked, FieldValue::Marked(true)),
            (FieldKind::Marked, FieldValue::Marked(false)),
            (FieldKind::Labels, FieldValue::Labels(0)),
            (FieldKind::Labels, FieldValue::Labels(0b1_0101)),
            (FieldKind::Text, FieldValue::Text("Alfama".to_owned())),
            (FieldKind::LangAlt, alt("Morning market")),
            (
                FieldKind::LangAlt,
                FieldValue::LangAlt(vec![
                    ("x-default".to_owned(), "Harbour".to_owned()),
                    ("de".to_owned(), "Hafen".to_owned()),
                ]),
            ),
            (FieldKind::Seq, FieldValue::List(vec!["Ana".to_owned()])),
            (
                FieldKind::Bag,
                FieldValue::List(vec!["a".to_owned(), "b".to_owned()]),
            ),
            (FieldKind::Date, FieldValue::Date("2026-07-04".to_owned())),
        ];
        for (kind, value) in values {
            assert!(value.is(kind));
            assert_eq!(FieldValue::read(&value.node(kind), kind), Some(value));
        }
        let wrong = [
            (FieldKind::Stars, Node::simple("6")),
            (FieldKind::Stars, Node::simple("-1")),
            (FieldKind::Marked, Node::simple("true")),
            (
                FieldKind::Labels,
                Node::array(ArrayKind::Seq, vec![Node::simple("5")]),
            ),
            (FieldKind::Labels, Node::array(ArrayKind::Bag, vec![])),
            (FieldKind::Labels, Node::simple("1")),
            (FieldKind::LangAlt, Node::simple("bare")),
            (
                FieldKind::LangAlt,
                Node::array(ArrayKind::Alt, vec![Node::simple("no language")]),
            ),
            (
                FieldKind::Seq,
                Node::array(ArrayKind::Bag, vec![Node::simple("a")]),
            ),
            (
                FieldKind::Bag,
                Node::array(ArrayKind::Bag, vec![Node::with_lang("a", "de")]),
            ),
            (FieldKind::Date, Node::simple("4 July")),
            (
                FieldKind::Text,
                Node::array(ArrayKind::Seq, vec![Node::simple("a")]),
            ),
        ];
        for (kind, node) in wrong {
            assert_eq!(FieldValue::read(&node, kind), None, "{node:?}");
        }
    }

    /// What a person types, trimmed, lists split and joined with "; ", a language
    /// alternative's other languages kept, and shown back as typed.
    #[test]
    fn each_kind_typed_and_shown_back() {
        let field = |kind| def("t", NAMESPACE, "t:t", kind);
        let typed = |kind, text: &str| FieldValue::from_typed(&field(kind), text, None);
        assert_eq!(
            typed(FieldKind::Text, "  Alfama \n"),
            Ok(Some(FieldValue::Text("Alfama".to_owned())))
        );
        assert_eq!(typed(FieldKind::Text, "   "), Ok(None));
        // A pasted line break is a space where the field takes none, and kept where it does.
        let pasted = FieldValue::Text("Morning market Alfama".to_owned());
        assert_eq!(
            typed(FieldKind::Text, "Morning market\nAlfama"),
            Ok(Some(pasted))
        );
        let notes = FieldDef {
            lines: true,
            ..field(FieldKind::Text)
        };
        let kept = FieldValue::from_typed(&notes, "a\nb", None);
        assert_eq!(kept, Ok(Some(FieldValue::Text("a\nb".to_owned()))));
        let creators = typed(FieldKind::Seq, " Ana Sousa ;; Rui ; ")
            .unwrap()
            .unwrap();
        assert_eq!(
            creators,
            FieldValue::List(vec!["Ana Sousa".to_owned(), "Rui".to_owned()])
        );
        assert_eq!(creators.shown(), "Ana Sousa; Rui");
        assert_eq!(typed(FieldKind::Bag, " ; ; "), Ok(None));
        let german = FieldValue::LangAlt(vec![
            ("x-default".to_owned(), "Harbour".to_owned()),
            ("de".to_owned(), "Hafen".to_owned()),
        ]);
        let retyped = FieldValue::from_typed(&field(FieldKind::LangAlt), " Port ", Some(&german));
        assert_eq!(
            retyped,
            Ok(Some(FieldValue::LangAlt(vec![
                ("x-default".to_owned(), "Port".to_owned()),
                ("de".to_owned(), "Hafen".to_owned()),
            ])))
        );
        assert_eq!(german.shown(), "Harbour");
        assert_eq!(
            typed(FieldKind::Marked, "True"),
            Ok(Some(FieldValue::Marked(true)))
        );
        assert_eq!(typed(FieldKind::Marked, "yes"), Err(TypedError::NotMarked));
        assert_eq!(
            typed(FieldKind::Date, " 2026-07-04 "),
            Ok(Some(FieldValue::Date("2026-07-04".to_owned())))
        );
        assert_eq!(
            typed(FieldKind::Date, "4 July 2026"),
            Err(TypedError::NotADate)
        );
        assert_eq!(typed(FieldKind::Stars, "3"), Err(TypedError::NotTyped));
    }

    #[test]
    fn xmp_dates_and_what_is_none() {
        for date in [
            "2026",
            "2026-07",
            "2026-07-04",
            "2024-02-29",
            "2026-07-04T08:10",
            "2026-07-04T08:10:01",
            "2026-07-04T08:10:01.25",
            "2026-07-04T08:10Z",
            "2026-07-04T08:10:01+01:00",
            "2026-07-04T08:10:01.5-05:30",
        ] {
            assert!(is_date(date), "{date}");
        }
        for not in [
            "",
            "26",
            "2026-7-4",
            "2026-13",
            "2026-02-29",
            "2026-04-31",
            "2026-07T08:10",
            "2026-07-04T8:10",
            "2026-07-04T24:00",
            "2026-07-04T08:10.5",
            "2026-07-04T08:10:01.",
            "2026-07-04T08:10+1",
            "2026-07-04T08:10Z01:00",
            "04/07/2026",
            "2026-07-04 08:10",
        ] {
            assert!(!is_date(not), "{not}");
        }
    }

    #[test]
    fn a_reject_keeps_the_stars() {
        let mut entries = Vec::new();
        set(&mut entries, RATING, Some(FieldValue::Stars(3)), clock(1));
        set(
            &mut entries,
            REJECTED,
            Some(FieldValue::Marked(true)),
            clock(2),
        );
        set(
            &mut entries,
            COLOUR_LABELS,
            Some(FieldValue::Labels(0b1001)),
            clock(3),
        );
        let rating = Rating::of(&entries);
        assert_eq!(
            rating,
            Rating {
                stars: 3,
                rejected: true,
                labels: 0b1001
            }
        );
        assert_eq!(rating.darktable(), -1);
        set(
            &mut entries,
            REJECTED,
            Some(FieldValue::Marked(false)),
            clock(4),
        );
        assert_eq!(Rating::of(&entries).darktable(), 3);
        // In the order of their properties, whatever the order set.
        let order: Vec<&str> = entries.iter().map(|e| e.field.as_str()).collect();
        assert_eq!(order, [REJECTED, COLOUR_LABELS, RATING]);
    }

    #[test]
    fn a_change_sets_only_what_it_changes() {
        let mut entries = Vec::new();
        let changed = |entries: &mut Vec<FieldEntry>, change: RatingChange, when| {
            let changes = change.apply(entries, clock(when));
            changes.into_iter().map(|c| c.field).collect::<Vec<_>>()
        };
        // No stars on a photo with none changes nothing.
        assert!(changed(&mut entries, RatingChange::Stars(0), 1).is_empty());
        assert_eq!(changed(&mut entries, RatingChange::Stars(3), 2), [RATING]);
        assert_eq!(
            changed(&mut entries, RatingChange::Rejected(true), 3),
            [REJECTED]
        );
        // A star takes the reject back, the stars as they were.
        assert_eq!(changed(&mut entries, RatingChange::Stars(3), 4), [REJECTED]);
        let label = |label, on| RatingChange::Label { label, on };
        assert_eq!(changed(&mut entries, label(0, true), 5), [COLOUR_LABELS]);
        assert!(changed(&mut entries, label(0, true), 6).is_empty());
        assert!(changed(&mut entries, label(9, true), 6).is_empty());
        assert_eq!(changed(&mut entries, label(4, true), 7), [COLOUR_LABELS]);
        assert_eq!(Rating::of(&entries).labels, 0b1_0001);
        assert_eq!(
            changed(&mut entries, RatingChange::ClearLabels, 8),
            [COLOUR_LABELS]
        );
        assert_eq!(
            Rating::of(&entries),
            Rating {
                stars: 3,
                rejected: false,
                labels: 0
            }
        );
        // Each changed field takes the change's clock.
        assert_eq!(entries.iter().map(|e| e.clock).max(), Some(Some(clock(8))));
        let stars = RatingChange::Stars(1);
        assert!(stars.same_kind(&RatingChange::Stars(4)));
        assert!(label(1, true).same_kind(&label(1, false)));
        assert!(!label(1, true).same_kind(&label(2, true)));
        assert!(!stars.same_kind(&RatingChange::Rejected(true)));
    }

    #[test]
    fn joining_takes_the_later_clock_of_each_field_both_ways() {
        let ours = vec![
            entry(RATING, Some(5), Some(FieldValue::Stars(2))),
            entry(REJECTED, Some(1), Some(FieldValue::Marked(true))),
            entry("dc:title", Some(4), Some(alt("Ours"))),
        ];
        let theirs = vec![
            entry(RATING, Some(3), Some(FieldValue::Stars(4))),
            // A clear beats an older value.
            entry(REJECTED, Some(2), None),
            entry(COLOUR_LABELS, None, Some(FieldValue::Labels(1))),
            entry("dc:title", Some(6), Some(alt("Theirs"))),
            entry("photoshop:City", Some(1), None),
        ];
        let joined = join(&ours, &theirs);
        assert_eq!(joined, join(&theirs, &ours));
        assert_eq!(Rating::of(&joined).stars, 2);
        assert_eq!(value(&joined, REJECTED), None);
        assert_eq!(value(&joined, "dc:title"), Some(alt("Theirs")));
        // A cleared field is kept as an entry, so the clear goes on winning.
        assert!(joined.iter().any(|e| e.field == "photoshop:City"));
        // A value beats an older clear, and a value with a clock one with none.
        let later = [entry(REJECTED, Some(9), Some(FieldValue::Marked(true)))];
        assert_eq!(
            value(&join(&joined, &later), REJECTED),
            Some(FieldValue::Marked(true))
        );
        let clocked = [entry(COLOUR_LABELS, Some(1), Some(FieldValue::Labels(4)))];
        assert_eq!(
            value(&join(&clocked, &joined), COLOUR_LABELS),
            Some(FieldValue::Labels(4))
        );
    }

    #[test]
    fn the_copy_s_name_is_set_and_joined_as_a_field() {
        let copy = |name: &str, when: Option<u64>| VirtualCopy {
            id: "00000000".to_owned(),
            number: "0".to_owned(),
            name: name.to_owned(),
            name_clock: when.map(clock),
            end: String::new(),
            clocks: Vec::new(),
            fields: Vec::new(),
            history: Vec::new(),
        };
        let mut ours = copy("", None);
        assert_eq!(value_in(&ours, COPY_NAME), None);
        set_in(
            &mut ours,
            COPY_NAME,
            Some(FieldValue::Text("Black and white".to_owned())),
            clock(3),
        );
        assert_eq!(
            value_in(&ours, COPY_NAME),
            Some(FieldValue::Text("Black and white".to_owned()))
        );
        set_in(&mut ours, "dc:title", Some(alt("A")), clock(4));
        assert_eq!(value_in(&ours, "dc:title"), Some(alt("A")));
        let theirs = copy("Warm", Some(2));
        let joined = join_name(&ours, &theirs);
        assert_eq!(joined, join_name(&theirs, &ours));
        assert_eq!(joined, ("Black and white".to_owned(), Some(clock(3))));
        // darktable's name, with no clock, loses to any set.
        assert_eq!(join_name(&copy("dt", None), &theirs).0, "Warm");
    }

    #[test]
    fn entries_read_back_as_written_a_cleared_one_kept() {
        let entries = vec![
            entry("Iptc4xmpCore:CiEmailWork", Some(5), None),
            entry(REJECTED, Some(2), None),
            entry(COLOUR_LABELS, Some(3), Some(FieldValue::Labels(0b11))),
            entry(
                "dc:creator",
                Some(4),
                Some(FieldValue::List(vec!["Ana".to_owned()])),
            ),
            entry("dc:title", Some(4), Some(alt("Morning market"))),
            entry(RATING, Some(1), Some(FieldValue::Stars(4))),
            entry("xmpRights:Marked", Some(6), Some(FieldValue::Marked(false))),
        ];
        let mut problems = Vec::new();
        assert_eq!(read(&node(&entries), "c", &mut problems), entries);
        assert_eq!(problems, []);
    }

    #[test]
    fn what_cannot_be_read_is_a_problem_and_left_out() {
        let own = |local: &str, text: &str| {
            Property::simple(Name::new("apertrust", NAMESPACE, local), text)
        };
        let with_clock = |name: &str, more: Vec<Property>| {
            let mut fields = vec![own("field", name)];
            fields.extend(clock(1).fields().map(|(local, text)| own(local, &text)));
            fields.extend(more);
            Node::structure(fields)
        };
        let items = vec![
            with_clock("dc:subject", vec![]),
            Node::simple("loose"),
            Node::structure(vec![own("field", RATING), own("when", "never")]),
            with_clock(RATING, vec![own("value", "9")]),
            with_clock(RATING, vec![own("colour", "red")]),
            with_clock(REJECTED, vec![own("value", "True")]),
            with_clock(REJECTED, vec![own("value", "False")]),
            // A language alternative's value written as plain text.
            with_clock("dc:title", vec![own("value", "bare")]),
        ];
        let mut problems = Vec::new();
        let read = read(&Node::array(ArrayKind::Bag, items), "c", &mut problems);
        assert_eq!(
            read,
            [entry(REJECTED, Some(1), Some(FieldValue::Marked(true)))]
        );
        let what: Vec<&str> = (problems.iter())
            .map(|problem| match problem {
                Problem::Field { what, .. } => *what,
                _ => "",
            })
            .collect();
        assert_eq!(
            what,
            [
                "a field this build does not know",
                "an item that is no struct",
                "a clock that cannot be read",
                "a value of the wrong form",
                "what is not an entry's own",
                "a field twice",
                "a value of the wrong form",
            ]
        );
    }

    /// A member is the namespace's and its local name, whatever prefix it is written with:
    /// one under another prefix reads, and the same member twice under two prefixes is a
    /// problem, never one of them dropped by a save.
    #[test]
    fn a_member_twice_under_two_prefixes_is_a_problem() {
        let member = |prefix: &str, local: &str, text: &str| {
            Property::simple(Name::new(prefix, NAMESPACE, local), text)
        };
        let item = |more: Vec<Property>| {
            let mut fields = vec![member("apertrust", "field", RATING)];
            let clock = clock(1)
                .fields()
                .map(|(local, text)| member("apertrust", local, &text));
            fields.extend(clock);
            fields.extend(more);
            Node::structure(fields)
        };
        let mut problems = Vec::new();
        let other = item(vec![member("ap", "value", "3")]);
        let read_other = read(
            &Node::array(ArrayKind::Bag, vec![other]),
            "c",
            &mut problems,
        );
        assert_eq!(
            read_other,
            [entry(RATING, Some(1), Some(FieldValue::Stars(3)))]
        );
        assert_eq!(problems, []);
        let values = vec![
            member("apertrust", "value", "3"),
            member("ap", "value", "4"),
        ];
        let twice = item(values);
        let read_twice = read(
            &Node::array(ArrayKind::Bag, vec![twice]),
            "c",
            &mut problems,
        );
        assert_eq!(read_twice, []);
        assert!(
            matches!(
                &problems[..],
                [Problem::Field {
                    what: "what is not an entry's own",
                    ..
                }]
            ),
            "{problems:?}"
        );
    }
}
