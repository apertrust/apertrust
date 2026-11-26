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
//! labels ([`Rating`]); the Metadata tool's text fields come with them.
//!
//! Every field is data, [`FIELDS`]: its id, its property and its kind. A copy holds an
//! entry for each field ever set ([`FieldEntry`]), its clock and its value, a cleared one
//! kept with no value, so the clear wins over an older value on another computer. In the
//! `.apxmp` they are the copy's `apertrust:fields`, after its clock vector, in the order of
//! their properties, so two machines write the same bytes:
//!
//! ```xml
//! <apertrust:fields>
//!  <rdf:Bag>
//!   <rdf:li>
//!    <rdf:Description apertrust:field="xmp:Rating"
//!      apertrust:when="2026-11-08T10:42:01.120Z" apertrust:counter="0"
//!      apertrust:machine="5e1f22c9" apertrust:value="3"/>
//!   </rdf:li>
//! ```
//!
//! [`set`] changes one; [`join`] combines two copies' entries field by field, the later
//! clock winning each (decision 52). What cannot be read as written is a
//! [`Problem::Field`], so the photo opens for viewing only and nothing is written over it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::clock::{self, Clock};
use crate::sidecar::{NAMESPACE, Problem};
use crate::xmp::{ArrayKind, Name, Node, Property, Value};

/// The namespace of XMP's basic properties, `xmp:Rating`'s.
pub const XMP_NAMESPACE: &str = "http://ns.adobe.com/xap/1.0/";
/// darktable's namespace, `darktable:colorlabels`'s.
pub const DARKTABLE_NAMESPACE: &str = "http://darktable.sf.net/";

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
}

/// The stars, kept while the photo is rejected ("a reject that keeps the stars").
pub const RATING: &str = "xmp:Rating";
/// The reject, a field of its own: darktable's −1 would lose the stars beneath it.
pub const REJECTED: &str = "apertrust:rejected";
/// The colour labels.
pub const COLOUR_LABELS: &str = "darktable:colorlabels";

/// Every field, in the order of their properties.
pub const FIELDS: &[FieldDef] = &[
    FieldDef {
        id: "rejected",
        namespace: NAMESPACE,
        property: REJECTED,
        kind: FieldKind::Marked,
    },
    FieldDef {
        id: "colour_labels",
        namespace: DARKTABLE_NAMESPACE,
        property: COLOUR_LABELS,
        kind: FieldKind::Labels,
    },
    FieldDef {
        id: "rating",
        namespace: XMP_NAMESPACE,
        property: RATING,
        kind: FieldKind::Stars,
    },
];

/// The field stored under `property`, a qualified name.
pub fn field(property: &str) -> Option<&'static FieldDef> {
    FIELDS.iter().find(|def| def.property == property)
}

/// A field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldValue {
    /// Stars, 0 to 5.
    Stars(u8),
    Marked(bool),
    /// The colour labels on, a bit each in darktable's order, bit 0 red to bit 4 purple.
    Labels(u8),
}

impl FieldValue {
    /// Whether it is a value of `kind`.
    pub fn is(&self, kind: FieldKind) -> bool {
        matches!(
            (self, kind),
            (FieldValue::Stars(_), FieldKind::Stars)
                | (FieldValue::Marked(_), FieldKind::Marked)
                | (FieldValue::Labels(_), FieldKind::Labels)
        )
    }

    /// As the file holds it: stars as their number, a mark as `True` or `False`, labels as
    /// an `rdf:Seq` of their numbers, lowest first.
    fn node(&self) -> Node {
        match *self {
            FieldValue::Stars(stars) => Node::simple(&stars.to_string()),
            FieldValue::Marked(on) => Node::simple(if on { "True" } else { "False" }),
            FieldValue::Labels(bits) => Node::array(
                ArrayKind::Seq,
                (0..LABELS)
                    .filter(|label| bits & (1 << label) != 0)
                    .map(|label| Node::simple(&label.to_string()))
                    .collect(),
            ),
        }
    }

    /// A value of `kind` as the file holds it, [`Self::node`]'s inverse; `None` for one of
    /// another form, stars past 5 or a label past 4 among them.
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
            _ => None,
        }
    }
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
        .and_then(|entry| entry.value)
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
                after: Some(after),
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
            fields.push(Property::new(
                Name::new("apertrust", NAMESPACE, "value"),
                value.node(),
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

    #[test]
    fn every_field_is_known_by_its_property_in_order() {
        let properties: Vec<&str> = FIELDS.iter().map(|def| def.property).collect();
        let mut sorted = properties.clone();
        sorted.sort_unstable();
        assert_eq!(properties, sorted);
        for def in FIELDS {
            assert_eq!(field(def.property), Some(def));
        }
        assert_eq!(field("dc:title"), None);
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
        ];
        for (kind, value) in values {
            assert!(value.is(kind));
            assert_eq!(FieldValue::read(&value.node(), kind), Some(value));
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
        ];
        for (kind, node) in wrong {
            assert_eq!(FieldValue::read(&node, kind), None, "{node:?}");
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
        ];
        let theirs = vec![
            entry(RATING, Some(3), Some(FieldValue::Stars(4))),
            // A clear beats an older value.
            entry(REJECTED, Some(2), None),
            entry(COLOUR_LABELS, None, Some(FieldValue::Labels(1))),
        ];
        let joined = join(&ours, &theirs);
        assert_eq!(joined, join(&theirs, &ours));
        assert_eq!(Rating::of(&joined).stars, 2);
        assert_eq!(value(&joined, REJECTED), None);
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
    fn entries_read_back_as_written_a_cleared_one_kept() {
        let entries = vec![
            entry(REJECTED, Some(2), None),
            entry(COLOUR_LABELS, Some(3), Some(FieldValue::Labels(0b11))),
            entry(RATING, Some(1), Some(FieldValue::Stars(4))),
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
            with_clock("dc:title", vec![]),
            Node::simple("loose"),
            Node::structure(vec![own("field", RATING), own("when", "never")]),
            with_clock(RATING, vec![own("value", "9")]),
            with_clock(RATING, vec![own("colour", "red")]),
            with_clock(REJECTED, vec![own("value", "True")]),
            with_clock(REJECTED, vec![own("value", "False")]),
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
