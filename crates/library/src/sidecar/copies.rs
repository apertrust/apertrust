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

//! Version 2 of the `.apxmp` as text (decisions 47 and 78, `docs/history.md`, In the
//! sidecar): the photo's identity and its virtual copies, each with its clock vector, its
//! history as a log of steps and its end. Every value is the text the file holds, but the
//! clocks, which [`Clock`] reads; the entries are version 1's, so this module, like the
//! rest of the text layer, never sees a module's meaning. What cannot be read as written,
//! a clock or a note, is a [`Problem`] the reader is told of, the rest of the file read.
//!
//! ```xml
//! <rdf:Description rdf:about="" apertrust:version="2"
//!   apertrust:photo="DSCF0001.RAF" apertrust:fingerprint="30482117:9f2c…">
//!  <apertrust:copies>
//!   <rdf:Seq>
//!    <rdf:li>
//!     <rdf:Description apertrust:id="7c0e9a14" apertrust:number="0" apertrust:name=""
//!       apertrust:end="a1b2c3d4e5f60718">
//!      <apertrust:clocks>
//!       <rdf:Bag>
//!        <rdf:li>
//!         <rdf:Description apertrust:when="2026-03-14T14:02:11.120Z"
//!           apertrust:counter="0" apertrust:machine="5e1f22c9"/>
//!        </rdf:li>
//!       </rdf:Bag>
//!      </apertrust:clocks>
//!      <apertrust:history>
//!       <rdf:Seq>
//!        <rdf:li>
//!         <rdf:Description apertrust:id="a1b2c3d4e5f60718"
//!           apertrust:when="2026-03-14T14:02:11.120Z" apertrust:counter="0"
//!           apertrust:machine="5e1f22c9" apertrust:source="style"
//!           apertrust:noteMessage="history-style" apertrust:batch="0f1e2d3c4b5a6978">
//!          <apertrust:noteArgs>
//!           <rdf:Description arg:style="Film grain"/>
//!          </apertrust:noteArgs>
//!          <apertrust:entries>
//!           <rdf:Seq>
//!            <rdf:li>
//!             <rdf:Description apertrust:module="exposure" … control:exposure="1.2"/>
//! ```
//!
//! A step's note is `apertrust:note`, an assistant's or a script's own words, or
//! `apertrust:noteMessage`, a message id, with its arguments in `apertrust:noteArgs`, a
//! struct of fields in [`ARG_NAMESPACE`] by their names: nested, so an argument named as a
//! step's field, `id` or `when`, is never read as one by tools that name a struct's fields
//! by their local names alone, as ExifTool does. Its entries, modules' and operations'
//! alike, are one `rdf:Seq` in the step's order, each written as version 1 writes it. The
//! clock vector is written in the order of its machine ids, so two machines writing the
//! same copy write the same bytes (decision 52).
//!
//! A version 1 or 0 file reads as one virtual copy ([`Edits::parse`]): its history one
//! step holding the file's entries, or none, end empty, for a file holding nothing.

use std::path::Path;
use std::time::UNIX_EPOCH;

use thiserror::Error;

use crate::clock::{self, Clock, ClockError, MachineId};
use crate::xmp::{ArrayKind, Name, Namespace, Node, Property, Value, Xmp};

use super::{
    Document, Entry, NAMESPACE, Newer, SidecarError, build, entry_list, own, own_field,
    read_entries, version, xmp,
};

/// The version whose layout holds the photo's identity and its virtual copies, which
/// [`Edits`] reads and writes.
pub const COPIES_VERSION: u32 = 2;
/// The namespace of a step's note's arguments, by their names.
pub const ARG_NAMESPACE: &str = "https://apertrust.org/ns/xmp/arg/";
/// The id of the virtual copy a version 1 file reads as, the same on every machine, so two
/// machines that read the same photo's version 1 file and change it compare the one copy
/// (decision 52). A new copy's random id is never this one (decision 47).
pub const FIRST_COPY: &str = "00000000";
/// What a version 1 file's step's source is: the person's.
const YOU: &str = "you";

/// A photo's `.apxmp` of version 2 as text: the photo it belongs to and its virtual copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edits {
    /// The file's `apertrust:version`: [`COPIES_VERSION`] for one made to be written, or
    /// above it for a newer build's, read as this version lays it out; below it for a
    /// version 1 or 0 file read as one virtual copy, which [`Edits::to_xml`] writes as
    /// [`COPIES_VERSION`].
    pub version: u32,
    /// The build that wrote it, as its `x:xmptk` records it.
    pub build: Option<String>,
    /// `apertrust:photo`, the photo's file name.
    pub photo: Option<String>,
    /// `apertrust:fingerprint`, as [`super::fingerprint`] writes it.
    pub fingerprint: Option<String>,
    /// `apertrust:copies`, in the file's order.
    pub copies: Vec<VirtualCopy>,
    /// What [`Edits::parse`] could not read as written, in the file's order; none for a
    /// file made to be written. An edit with any is opened for viewing only, so nothing is
    /// written over what was not read (`docs/history.md`).
    pub problems: Vec<Problem>,
}

/// Something of a version 2 file that could not be read as written: the rest is read,
/// and the step it is in is kept.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Problem {
    /// A step's clock that cannot be read: the step is kept with no clock.
    #[error("the clock of step {step} of copy {copy} cannot be read: {error}")]
    StepClock {
        copy: String,
        step: String,
        error: ClockError,
    },
    /// A clock of a copy's vector that cannot be read: it is left out.
    #[error("a clock of copy {copy}'s vector cannot be read and is left out: {error}")]
    VectorClock { copy: String, error: ClockError },
    /// A step with both `apertrust:note` and `apertrust:noteMessage`: the message is read.
    #[error("step {step} of copy {copy} has both a note and a message: the message is read")]
    TwoNotes { copy: String, step: String },
    /// A step with `apertrust:noteArgs` but no `apertrust:noteMessage`: the arguments are
    /// left out.
    #[error("step {step} of copy {copy} has a message's arguments but no message: left out")]
    ArgumentsWithoutMessage { copy: String, step: String },
    /// `apertrust:noteArgs` holding what is not a text argument: a field of another
    /// namespace, an argument that is no plain text, or `noteArgs` that is no struct. The
    /// text arguments are read; the rest is left out.
    #[error("step {step} of copy {copy} has note arguments that cannot be read: left out")]
    NoteArguments { copy: String, step: String },
    /// `apertrust:note` or `apertrust:noteMessage`, by its local name `field`, that is no
    /// plain text: a struct, an array, a resource or a text with qualifiers. It is left
    /// out.
    #[error("the {field} of step {step} of copy {copy} is not plain text: left out")]
    UnreadableNote {
        copy: String,
        step: String,
        field: String,
    },
}

/// One virtual copy (decision 47): its id, place and name, its clock vector, its history
/// and its end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualCopy {
    /// `apertrust:id`, 8 hex digits.
    pub id: String,
    /// `apertrust:number`, its place, darktable's version number: `0` for the first.
    pub number: String,
    /// `apertrust:name`, darktable's version name; empty for none.
    pub name: String,
    /// `apertrust:end`, the id of the last step applied; empty for Original.
    pub end: String,
    /// `apertrust:clocks`, an `rdf:Bag` of the latest clock of each machine that changed
    /// it: read in the file's order, written in the order of their machine ids.
    pub clocks: Vec<Clock>,
    /// `apertrust:history`, its steps oldest first, those after the end among them.
    pub history: Vec<Step>,
}

/// One step of a virtual copy's history (decision 78).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// `apertrust:id`, 16 hex digits.
    pub id: String,
    /// `apertrust:when`, `apertrust:counter` and `apertrust:machine`; `None` where they
    /// cannot be read ([`Problem::StepClock`]), and then none is written.
    pub clock: Option<Clock>,
    /// `apertrust:source` by name: `you`, `auto`, `style`, `assistant` or `default`.
    pub source: String,
    pub note: Option<Note>,
    /// `apertrust:batch`, 16 hex digits.
    pub batch: Option<String>,
    /// `apertrust:entries`: each module and operation it touched, with its whole state
    /// after it, in the step's order.
    pub entries: Vec<Entry>,
}

/// What a step says of itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// `apertrust:noteMessage`, a message id, with its arguments by name and text, the
    /// fields of `apertrust:noteArgs` in [`ARG_NAMESPACE`], in the file's order.
    Message {
        id: String,
        args: Vec<(String, String)>,
    },
    /// `apertrust:note`, an assistant's or a script's own words.
    Text(String),
}

impl Edits {
    /// An empty file of version [`COPIES_VERSION`] for `build` to write.
    pub fn new(build: &str) -> Self {
        Edits {
            version: COPIES_VERSION,
            build: Some(build.to_owned()),
            photo: None,
            fingerprint: None,
            copies: Vec::new(),
            problems: Vec::new(),
        }
    }

    /// A version above [`COPIES_VERSION`], which a newer Apertrust wrote: read as version 2
    /// lays it out, for viewing only (decision 43).
    pub fn newer(&self) -> Option<Newer> {
        (self.version > COPIES_VERSION).then_some(Newer::Version(self.version))
    }

    /// Reads an `.apxmp` file's XMP through the XMP layer, `modified` its modified time in
    /// milliseconds since 1970. A file of version 2 or above is read as version 2 lays it
    /// out. A file of version 1 or 0 is read as one virtual copy, [`FIRST_COPY`], number
    /// `0` with no name, its history one step holding the file's entries in its order: the
    /// step's id the first 16 hex digits of the BLAKE3 hash of `xml`'s bytes, so every
    /// machine reading the same file gives the same step; its clock `modified` with counter
    /// 0 and machine [`MachineId::UNKNOWN`], also the copy's clock vector; source `you`,
    /// no note, no batch; the end that step. A version 1 or 0 file holding no entry reads
    /// as a copy with no step, its end empty.
    ///
    /// What cannot be read as written, a clock or a note, is read as far as it can be and
    /// told in [`Edits::problems`] (decision 48); a value missing elsewhere reads as empty
    /// text.
    pub fn parse(xml: &str, modified: u64) -> Result<Edits, SidecarError> {
        let xmp = Xmp::parse(xml)?;
        let version = version(&xmp)?;
        if version < COPIES_VERSION {
            let document = Document::from_xmp(&xmp, version);
            return Ok(Edits::from_version_1(document, xml.as_bytes(), modified));
        }
        let mut copies = Vec::new();
        let mut problems = Vec::new();
        if let Some(listed) = xmp.get(NAMESPACE, "copies") {
            for item in listed.items().unwrap_or_default() {
                copies.push(read_copy(item.fields().unwrap_or_default(), &mut problems));
            }
        }
        let text = |local: &str| {
            xmp.get(NAMESPACE, local)
                .and_then(Node::as_str)
                .map(str::to_owned)
        };
        Ok(Edits {
            version,
            build: build(&xmp),
            photo: text("photo"),
            fingerprint: text("fingerprint"),
            copies,
            problems,
        })
    }

    /// A version 1 or 0 document, of the file `bytes` modified at `modified`, as one
    /// virtual copy ([`Edits::parse`]).
    fn from_version_1(document: Document, bytes: &[u8], modified: u64) -> Edits {
        let mut copy = VirtualCopy {
            id: FIRST_COPY.to_owned(),
            number: "0".to_owned(),
            name: String::new(),
            end: String::new(),
            clocks: Vec::new(),
            history: Vec::new(),
        };
        if !document.entries.is_empty() {
            // BLAKE3 itself, not the index's (layer 3), as the text layer calls nothing above
            // it (decision 54); the same hash as `index::hash`, which the engine's reading uses.
            let hash = *blake3::hash(bytes).as_bytes();
            let id: String = hash[..8].iter().map(|byte| format!("{byte:02x}")).collect();
            let clock = Clock::at(modified, None, MachineId::UNKNOWN);
            copy.end.clone_from(&id);
            copy.clocks.push(clock);
            copy.history.push(Step {
                id,
                clock: Some(clock),
                source: YOU.to_owned(),
                note: None,
                batch: None,
                entries: document.entries,
            });
        }
        Edits {
            version: document.version,
            build: document.build,
            photo: None,
            fingerprint: None,
            copies: vec![copy],
            problems: Vec::new(),
        }
    }

    /// The file as XMP, in a layout that never varies, so a file read and written back is
    /// the same bytes: its version, at least [`COPIES_VERSION`], the photo's name and
    /// fingerprint where it has them, and its copies in their order, each step's fields in
    /// the order the module's documentation shows, each clock vector in the order of its
    /// machine ids. `x:xmptk` records [`Edits::build`], which a file read keeps as the
    /// build that wrote it: the caller sets its own before writing.
    pub fn to_xml(&self) -> String {
        let mut properties = vec![own(
            "version",
            &self.version.max(COPIES_VERSION).to_string(),
        )];
        for (local, text) in [("photo", &self.photo), ("fingerprint", &self.fingerprint)] {
            if let Some(text) = text {
                properties.push(own(local, text));
            }
        }
        properties.push(Property::new(
            Name::new("apertrust", NAMESPACE, "copies"),
            Node::array(ArrayKind::Seq, self.copies.iter().map(copy_node).collect()),
        ));
        xmp(
            self.build.as_deref(),
            vec![Namespace::new("arg", ARG_NAMESPACE)],
            properties,
        )
        .to_xml()
    }
}

/// Reads the `.apxmp` at `path` as [`Edits`] ([`Edits::parse`]), with its modified time,
/// 0 where the system gives none.
pub fn read_edits(path: &Path) -> Result<Edits, SidecarError> {
    let xml = std::fs::read_to_string(path)?;
    let modified = std::fs::metadata(path)?
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        });
    Edits::parse(&xml, modified)
}

/// The node of the field `local` of Apertrust's namespace among `fields`.
fn own_node<'a>(fields: &'a [Property], local: &str) -> Option<&'a Node> {
    fields
        .iter()
        .find(|field| field.name.is(NAMESPACE, local))
        .map(|field| &field.node)
}

/// A virtual copy from its struct's fields, what cannot be read added to `problems`.
fn read_copy(fields: &[Property], problems: &mut Vec<Problem>) -> VirtualCopy {
    let text = |local: &str| own_field(fields, local).unwrap_or_default().to_owned();
    let items = |local: &str| {
        own_node(fields, local)
            .and_then(Node::items)
            .unwrap_or_default()
    };
    let id = text("id");
    let mut clocks = Vec::new();
    for item in items("clocks") {
        match read_clock(item.fields().unwrap_or_default()) {
            Ok(clock) => clocks.push(clock),
            Err(error) => problems.push(Problem::VectorClock {
                copy: id.clone(),
                error,
            }),
        }
    }
    let history = items("history")
        .iter()
        .map(|item| read_step(item.fields().unwrap_or_default(), &id, problems))
        .collect();
    VirtualCopy {
        number: text("number"),
        name: text("name"),
        end: text("end"),
        id,
        clocks,
        history,
    }
}

/// A clock from the three fields of a struct ([`Clock::from_fields`]).
fn read_clock(fields: &[Property]) -> Result<Clock, ClockError> {
    let text = |local: &str| own_field(fields, local).unwrap_or_default();
    Clock::from_fields(
        text(clock::WHEN),
        text(clock::COUNTER),
        text(clock::MACHINE),
    )
}

/// A step of the copy `copy` from its struct's fields, what cannot be read added to
/// `problems`.
fn read_step(fields: &[Property], copy: &str, problems: &mut Vec<Problem>) -> Step {
    let text = |local: &str| own_field(fields, local).map(str::to_owned);
    let id = text("id").unwrap_or_default();
    let (copy, step) = (copy.to_owned(), id.clone());
    // The text fields of `arg:`; anything else, which a save would drop, is a problem.
    let args = own_node(fields, "noteArgs").map(|node| {
        let mut args = Vec::new();
        let mut odd = node.fields().is_none() || !node.qualifiers.is_empty();
        for field in node.fields().unwrap_or_default() {
            match &field.node.value {
                Value::Simple(text)
                    if field.name.namespace == ARG_NAMESPACE
                        && field.node.qualifiers.is_empty() =>
                {
                    args.push((field.name.local.clone(), text.clone()));
                }
                _ => odd = true,
            }
        }
        if odd {
            problems.push(Problem::NoteArguments {
                copy: copy.clone(),
                step: step.clone(),
            });
        }
        args
    });
    let clock = match read_clock(fields) {
        Ok(clock) => Some(clock),
        Err(error) => {
            problems.push(Problem::StepClock {
                copy: copy.clone(),
                step: step.clone(),
                error,
            });
            None
        }
    };
    // A note's field as plain text; one that is not, a struct, an array, a resource or a
    // qualified text, which a save would change or drop, is a problem and read as absent.
    let mut note_text = |local: &str| {
        let node = own_node(fields, local)?;
        match &node.value {
            Value::Simple(text) if node.qualifiers.is_empty() => Some(text.clone()),
            _ => {
                problems.push(Problem::UnreadableNote {
                    copy: copy.clone(),
                    step: step.clone(),
                    field: local.to_owned(),
                });
                None
            }
        }
    };
    let (message, words) = (note_text("noteMessage"), note_text("note"));
    let note = match (message, words) {
        (Some(message), words) => {
            if words.is_some() {
                problems.push(Problem::TwoNotes { copy, step });
            }
            Some(Note::Message {
                id: message,
                args: args.unwrap_or_default(),
            })
        }
        (None, words) => {
            if args.is_some() {
                problems.push(Problem::ArgumentsWithoutMessage { copy, step });
            }
            words.map(Note::Text)
        }
    };
    Step {
        id,
        clock,
        source: text("source").unwrap_or_default(),
        note,
        batch: text("batch"),
        entries: own_node(fields, "entries")
            .map(read_entries)
            .unwrap_or_default(),
    }
}

/// A clock's three fields, [`Clock::fields`], as properties of Apertrust's namespace.
fn clock_fields(clock: &Clock) -> impl Iterator<Item = Property> {
    clock
        .fields()
        .into_iter()
        .map(|(local, text)| own(local, &text))
}

/// A virtual copy as a struct: its id, number, name and end, its clock vector and its
/// history.
fn copy_node(copy: &VirtualCopy) -> Node {
    let list = |local: &str, kind, items| {
        Property::new(
            Name::new("apertrust", NAMESPACE, local),
            Node::array(kind, items),
        )
    };
    let mut sorted: Vec<&Clock> = copy.clocks.iter().collect();
    sorted.sort_by_key(|clock| clock.machine);
    let clocks = sorted
        .into_iter()
        .map(|clock| Node::structure(clock_fields(clock).collect()))
        .collect();
    Node::structure(vec![
        own("id", &copy.id),
        own("number", &copy.number),
        own("name", &copy.name),
        own("end", &copy.end),
        list("clocks", ArrayKind::Bag, clocks),
        list(
            "history",
            ArrayKind::Seq,
            copy.history.iter().map(step_node).collect(),
        ),
    ])
}

/// A step as a struct: its id, clock, source, note and batch, then its note's arguments
/// where it has any, then its entries.
fn step_node(step: &Step) -> Node {
    let mut fields = vec![own("id", &step.id)];
    fields.extend(step.clock.iter().flat_map(clock_fields));
    fields.push(own("source", &step.source));
    let mut args = None;
    match &step.note {
        Some(Note::Text(text)) => fields.push(own("note", text)),
        Some(Note::Message { id, args: given }) => {
            fields.push(own("noteMessage", id));
            args = (!given.is_empty()).then_some(given);
        }
        None => {}
    }
    if let Some(batch) = &step.batch {
        fields.push(own("batch", batch));
    }
    if let Some(args) = args {
        let args = args
            .iter()
            .map(|(name, text)| Property::simple(Name::new("arg", ARG_NAMESPACE, name), text))
            .collect();
        fields.push(Property::new(
            Name::new("apertrust", NAMESPACE, "noteArgs"),
            Node::structure(args),
        ));
    }
    fields.push(Property::new(
        Name::new("apertrust", NAMESPACE, "entries"),
        entry_list(&step.entries),
    ));
    Node::structure(fields)
}
