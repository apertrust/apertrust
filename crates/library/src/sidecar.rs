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

//! The `.apxmp` sidecar's text layer (decisions 28, 43, 47, 48, 54 and 78, layer 1 of the
//! library's API): `IMG.ARW.apxmp`, an XMP file beside the photo, as bytes in and values
//! out, and back. It knows the file's structure and nothing of what the values mean, so it
//! depends on no engine, GPU, module or operation crate.
//!
//! A [`Document`] is the file as text: its version, the build that wrote it (`x:xmptk`) and
//! its entries, each a module's or an operation's attributes as strings, in the order the
//! file has them. [`Document::parse`] reads one and [`Document::to_xml`] writes one, in a
//! layout that never varies, so a file read and written back is the same bytes. The engine
//! turns the entries into an edit's values, and an edit into entries, with the module
//! definitions; this module never sees a number, a choice or a parameter's range.
//!
//! ```xml
//! <rdf:Description rdf:about="" apertrust:version="1">
//!  <apertrust:modules>
//!   <rdf:Seq>
//!    <rdf:li>
//!     <rdf:Description apertrust:module="exposure" apertrust:moduleVersion="1"
//!       apertrust:kept="exposure black_level"
//!       control:exposure="0.5" control:black_level="0.01"/>
//!    </rdf:li>
//!    <rdf:li>
//!     <rdf:Description apertrust:module="grain" apertrust:moduleVersion="2"
//!       apertrust:enabled="true"/>
//!    </rdf:li>
//!   </rdf:Seq>
//!  </apertrust:modules>
//!  <apertrust:operations>
//!   <rdf:Seq>
//!    <rdf:li>
//!     <rdf:Description apertrust:operation="highlights" apertrust:schema="1"
//!       param:clip="0.9"/>
//!    </rdf:li>
//! ```
//!
//! A switched module that is on (decision 32) has `apertrust:enabled="true"`; absent, it
//! is off. The text layer keeps whatever the attribute holds, so the engine can tell a
//! value it does not know.
//!
//! Version 2 (decisions 47 and 78) is [`Edits`]: the photo's name and [`fingerprint`], and
//! its virtual copies, each with its clock vector, its history of steps, each step's
//! entries written as version 1 writes them, and its end. [`Edits::parse`] reads version 1
//! and 0 as one virtual copy whose history is one step, or none. The engine still writes
//! version 1 through [`Document`].
//!
//! What the text layer tells of an edit a newer Apertrust wrote (decision 43), from the
//! file alone, is a file version above [`VERSION`] ([`Document::newer`]). [`Newer`] also
//! names the three things only the module definitions can tell, a module version above its
//! definition's, a module the build neither has nor has retired, and a module turned on
//! that the build does not switch, which the engine finds.
//! [`ViewOnly`] is the error an edit opened for viewing only carries.
//!
//! The namespaces are on `apertrust.org` (decision 48).

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::xmp::{self, ArrayKind, Name, Namespace, Node, Property, Xmp};

/// The version the engine writes, whose edit [`Document`] reads; a file above it opens
/// for viewing only. Version 2's layout, [`COPIES_VERSION`], is [`Edits`]'.
pub const VERSION: u32 = 1;
/// The namespace of Apertrust's own properties, on the project's domain (decision 48).
pub const NAMESPACE: &str = "https://apertrust.org/ns/xmp/";
/// The namespace of module controls, by their ids.
pub const CONTROL_NAMESPACE: &str = "https://apertrust.org/ns/xmp/control/";
/// The namespace of operation parameters, by their schema names.
pub const PARAM_NAMESPACE: &str = "https://apertrust.org/ns/xmp/param/";
/// The namespace of RDF, whose `Description` holds the entries.
pub const RDF_NAMESPACE: &str = xmp::RDF;

mod copies;
#[cfg(test)]
mod tests;

pub use copies::{
    ARG_NAMESPACE, COPIES_VERSION, Edits, FIRST_COPY, Note, Problem, Step, VirtualCopy, read_edits,
};

/// What `x:xmptk` says before the build.
const TOOLKIT: &str = "Apertrust ";

#[derive(Debug, Error)]
pub enum SidecarError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("not XMP: {0}")]
    Xml(#[from] xmp::Error),
    #[error("not an Apertrust edit: no apertrust:version")]
    NotAnEdit,
    /// A version that is no whole number, so no layout this build knows.
    #[error("an edit of version {0}, which this Apertrust cannot read")]
    Version(String),
}

/// What tells an edit a newer Apertrust wrote (decision 43): something it holds that
/// this build would lose by saving it. Never a comparison of builds' versions, which
/// development builds on different branches do not order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Newer {
    /// A file version above [`VERSION`], read as [`VERSION`] lays it out.
    Version(u32),
    /// A module version above its definition's, or one that is no whole number, as
    /// written; its controls read by name.
    ModuleVersion {
        module: String,
        version: String,
        has: u32,
    },
    /// A module this build neither has nor has retired, left out.
    Module(String),
    /// A module whose `apertrust:enabled` this build cannot keep (decision 32): one it
    /// does not switch, or a value other than `true`, whose state a save would lose.
    Enabled(String),
}

impl fmt::Display for Newer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Newer::Version(version) => {
                write!(f, "is version {version}, newer than this build's {VERSION}")
            }
            Newer::ModuleVersion {
                module,
                version,
                has,
            } => write!(
                f,
                "holds {module} version {version}, newer than this build's {has}"
            ),
            Newer::Module(module) => {
                write!(
                    f,
                    "holds the module {module}, which this build does not have"
                )
            }
            Newer::Enabled(module) => {
                write!(
                    f,
                    "turns the module {module} on, which this build does not switch"
                )
            }
        }
    }
}

/// An edit a newer Apertrust wrote, which this build opens for viewing only (decision
/// 43): it renders what it reads, refuses every change and never writes the file, so
/// nothing the newer build wrote is lost.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("the edit {} wrote {newer}: open for viewing only", writer(build))]
pub struct ViewOnly {
    /// The build that wrote it, as the file records it; `None` where it records none.
    pub build: Option<String>,
    pub newer: Newer,
}

/// Who wrote an edit: `Apertrust` and its build, or a newer Apertrust where it names none.
pub fn writer(build: &Option<String>) -> String {
    build.as_ref().map_or_else(
        || "a newer Apertrust".to_owned(),
        |b| format!("Apertrust {b}"),
    )
}

/// What a sidecar's name adds to its photo's, in lower case as Apertrust writes it: a file
/// ending in `.APXMP` is another file.
pub const SUFFIX: &str = ".apxmp";

/// The sidecar beside `photo`: the photo's whole name, then `.apxmp`.
pub fn path_for(photo: &Path) -> PathBuf {
    let mut name = photo.as_os_str().to_owned();
    name.push(SUFFIX);
    PathBuf::from(name)
}

/// A file's name, as a debug line names it, or the whole path where it has none.
pub fn file_name(path: &Path) -> std::borrow::Cow<'_, str> {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
}

/// Writes an edit's XMP to `path` as `apertrust_base::fs::write` does (decision 53): through a
/// hidden temporary flushed and renamed into place, so a failed write never leaves half an
/// edit nor a stray file, and not at all, the file untouched, when it already holds `xml`.
pub fn write_xml(path: &Path, xml: &str) -> io::Result<()> {
    apertrust_base::fs::write(path, xml.as_bytes()).map(|_| ())
}

/// Reads the sidecar at `path` ([`Document::parse`]).
pub fn read(path: &Path) -> Result<Document, SidecarError> {
    Document::parse(&std::fs::read_to_string(path)?)
}

/// How much of a photo's start, and of its end, its fingerprint hashes: 64 KiB.
pub const FINGERPRINT_SPAN: u64 = 64 * 1024;

/// A photo's fingerprint as `apertrust:fingerprint` holds it (decision 53): its size in
/// bytes, a colon and 64 hex digits, the BLAKE3 hash of `head`, its first
/// [`FINGERPRINT_SPAN`] bytes, then `tail`, its last, each the whole file where it is
/// shorter.
pub fn fingerprint(size: u64, head: &[u8], tail: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(head);
    hasher.update(tail);
    format!("{size}:{}", hasher.finalize().to_hex())
}

/// The fingerprint of the photo at `path` ([`fingerprint`]), reading its first and last
/// [`FINGERPRINT_SPAN`] bytes alone.
pub fn read_fingerprint(path: &Path) -> io::Result<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    let span = size.min(FINGERPRINT_SPAN);
    let mut read = |from: u64| -> io::Result<Vec<u8>> {
        let mut bytes = vec![0; usize::try_from(span).unwrap_or(usize::MAX)];
        file.seek(SeekFrom::Start(from))?;
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    };
    let head = read(0)?;
    let tail = read(size - span)?;
    Ok(fingerprint(size, &head, &tail))
}

/// One `rdf:Description` as the file has it: a module's or an operation's, every value
/// as the text the file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Module {
        module: String,
        /// Its `apertrust:moduleVersion` as written.
        version: Option<String>,
        /// The ids in `apertrust:kept`.
        kept: Vec<String>,
        /// The ids in `apertrust:auto`.
        auto: Vec<String>,
        /// Its `apertrust:enabled` as written, `None` where absent: `true` for a switched
        /// module that is on.
        enabled: Option<String>,
        /// The `control:` attributes, by control id and text, in the file's order.
        controls: Vec<(String, String)>,
    },
    Operation {
        operation: String,
        /// Its `apertrust:schema`, `None` where absent or no whole number.
        schema: Option<u32>,
        /// The `param:` attributes, by parameter name and text, in the file's order.
        params: Vec<(String, String)>,
    },
}

/// An `.apxmp` file as text: its version, the build that wrote it and its entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The file's `apertrust:version`; [`VERSION`] for a document made to be written.
    pub version: u32,
    /// The build that wrote it, as its `x:xmptk` records it; `None` for a file that
    /// records no Apertrust build.
    pub build: Option<String>,
    /// Every entry in the file's order. [`Document::to_xml`] writes the modules' first and
    /// then the operations', each group in this order.
    pub entries: Vec<Entry>,
}

impl Document {
    /// An empty document of version [`VERSION`] for `build` to write.
    pub fn new(build: &str) -> Self {
        Document {
            version: VERSION,
            build: Some(build.to_owned()),
            entries: Vec::new(),
        }
    }

    /// The first thing in the file itself that tells a newer Apertrust wrote it: a version
    /// above [`VERSION`]. Such a file is read as this version lays it out, for viewing
    /// only.
    pub fn newer(&self) -> Option<Newer> {
        (self.version > VERSION).then_some(Newer::Version(self.version))
    }

    /// Reads an `.apxmp` file's XMP through the XMP layer ([`Xmp::parse`]): the version, the
    /// build and every item of `apertrust:modules` and `apertrust:operations`, a module with
    /// its version, marks and `control:` fields, an operation with its schema and its
    /// `param:` fields. An item that is neither is no entry.
    pub fn parse(xml: &str) -> Result<Document, SidecarError> {
        let xmp = Xmp::parse(xml)?;
        Ok(Document::from_xmp(&xmp, version(&xmp)?))
    }

    /// The document an `.apxmp` file's XMP holds, read as version 1 lays it out.
    fn from_xmp(xmp: &Xmp, version: u32) -> Document {
        let mut entries = Vec::new();
        for property in &xmp.properties {
            let listed = property.name.namespace == NAMESPACE
                && matches!(property.name.local.as_str(), "modules" | "operations");
            if listed {
                entries.extend(read_entries(&property.node));
            }
        }
        Document {
            version,
            build: build(xmp),
            entries,
        }
    }

    /// The document as XMP, the build recorded in `x:xmptk`: the module entries in their
    /// order, then the operation entries in theirs.
    pub fn to_xml(&self) -> String {
        let (modules, operations): (Vec<&Entry>, Vec<&Entry>) = self
            .entries
            .iter()
            .partition(|entry| matches!(entry, Entry::Module { .. }));
        let list = |local: &str, entries: Vec<&Entry>| {
            Property::new(
                Name::new("apertrust", NAMESPACE, local),
                entry_list(entries),
            )
        };
        xmp(
            self.build.as_deref(),
            Vec::new(),
            vec![
                own("version", &self.version.to_string()),
                list("modules", modules),
                list("operations", operations),
            ],
        )
        .to_xml()
    }
}

/// The file's `apertrust:version`, which every layout holds: [`SidecarError::NotAnEdit`]
/// where there is none, [`SidecarError::Version`] where it is no whole number.
fn version(xmp: &Xmp) -> Result<u32, SidecarError> {
    let text = xmp
        .properties
        .iter()
        .find(|p| p.name.local == "version" && p.name.namespace == NAMESPACE)
        .and_then(|p| p.node.as_str())
        .ok_or(SidecarError::NotAnEdit)?;
    text.trim()
        .parse()
        .map_err(|_| SidecarError::Version(text.to_owned()))
}

/// The build `x:xmptk` records, `None` where it records no Apertrust build.
fn build(xmp: &Xmp) -> Option<String> {
    xmp.toolkit
        .as_deref()
        .and_then(|toolkit| toolkit.strip_prefix(TOOLKIT))
        .map(str::trim)
        .filter(|build| !build.is_empty())
        .map(str::to_owned)
}

/// An `.apxmp` file's XMP: `build` recorded in `x:xmptk`, the namespaces every version
/// declares, then `more`, and `properties`.
fn xmp(build: Option<&str>, more: Vec<Namespace>, properties: Vec<Property>) -> Xmp {
    let mut namespaces = vec![
        Namespace::new("apertrust", NAMESPACE),
        Namespace::new("control", CONTROL_NAMESPACE),
        Namespace::new("param", PARAM_NAMESPACE),
    ];
    namespaces.extend(more);
    Xmp {
        toolkit: Some(format!("{TOOLKIT}{}", build.unwrap_or_default())),
        about: String::new(),
        namespaces,
        properties,
    }
}

/// One of Apertrust's own properties, simple, with its text.
fn own(local: &str, text: &str) -> Property {
    Property::simple(Name::new("apertrust", NAMESPACE, local), text)
}

/// The text of the simple field `local` of Apertrust's namespace among `fields`.
fn own_field<'a>(fields: &'a [Property], local: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|field| field.name.is(NAMESPACE, local))
        .and_then(|field| field.node.as_str())
}

/// The entries an `rdf:Seq` holds; an item that is neither a module's nor an operation's
/// is no entry.
fn read_entries(node: &Node) -> Vec<Entry> {
    node.items()
        .unwrap_or_default()
        .iter()
        .filter_map(|item| item.fields().and_then(entry))
        .collect()
}

/// `entries` as an `rdf:Seq` of structs, in their order.
fn entry_list<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> Node {
    Node::array(
        ArrayKind::Seq,
        entries.into_iter().map(entry_node).collect(),
    )
}

/// An entry as a struct: its id, version and marks in Apertrust's namespace, then its
/// `control:` or `param:` fields in its order.
fn entry_node(entry: &Entry) -> Node {
    let field = |prefix: &str, namespace: &str, local: &str, text: &str| {
        Property::simple(Name::new(prefix, namespace, local), text)
    };
    match entry {
        Entry::Module {
            module,
            version,
            kept,
            auto,
            enabled,
            controls,
        } => {
            let mut fields = vec![own("module", module)];
            if let Some(version) = version {
                fields.push(own("moduleVersion", version));
            }
            if let Some(enabled) = enabled {
                fields.push(own("enabled", enabled));
            }
            for (id, ids) in [("kept", kept), ("auto", auto)] {
                if !ids.is_empty() {
                    fields.push(own(id, &ids.join(" ")));
                }
            }
            for (id, text) in controls {
                fields.push(field("control", CONTROL_NAMESPACE, id, text));
            }
            Node::structure(fields)
        }
        Entry::Operation {
            operation,
            schema,
            params,
        } => {
            let mut fields = vec![own("operation", operation)];
            if let Some(schema) = schema {
                fields.push(own("schema", &schema.to_string()));
            }
            for (id, text) in params {
                fields.push(field("param", PARAM_NAMESPACE, id, text));
            }
            Node::structure(fields)
        }
    }
}

/// An entry from an item's fields: a module's, an operation's, or none where it names
/// neither.
fn entry(fields: &[Property]) -> Option<Entry> {
    let (mut module, mut operation) = (String::new(), String::new());
    let (mut version, mut schema) = (None, None);
    let (mut kept, mut auto) = (Vec::new(), Vec::new());
    let mut enabled = None;
    let (mut controls, mut params) = (Vec::new(), Vec::new());
    for field in fields {
        let Some(value) = field.node.as_str().map(str::to_owned) else {
            continue;
        };
        let name = &field.name;
        let list = |value: &str| value.split_whitespace().map(str::to_owned).collect();
        let namespace = name.namespace.as_str();
        if namespace == NAMESPACE {
            match name.local.as_str() {
                "module" => module = value,
                "moduleVersion" => version = Some(value),
                "kept" => kept = list(&value),
                "auto" => auto = list(&value),
                "enabled" => enabled = Some(value),
                "operation" => operation = value,
                "schema" => schema = value.trim().parse().ok(),
                _ => {}
            }
        } else if namespace == CONTROL_NAMESPACE {
            controls.push((name.local.clone(), value));
        } else if namespace == PARAM_NAMESPACE {
            params.push((name.local.clone(), value));
        }
    }
    if !module.is_empty() {
        Some(Entry::Module {
            module,
            version,
            kept,
            auto,
            enabled,
            controls,
        })
    } else if !operation.is_empty() {
        Some(Entry::Operation {
            operation,
            schema,
            params,
        })
    } else {
        None
    }
}
