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

//! Preset and style files (decision 106): `<user data>/presets/<tool>/<Name>.appreset`,
//! one preset of one tool, and `<user data>/styles/<Name>.apstyle`, one style of one or
//! more tools, read, written, listed, renamed and deleted as text. They live in the user
//! data folder, not a photo folder, so this module is outside the library's four layers
//! (decision 54): no layer calls it, and the app and the CLI call it with the folder
//! `Folders::presets` or `Folders::styles` gives. It knows the files' structure and
//! nothing of what the values mean: the engine turns the entries into tool values with the
//! tool definitions, as it does an edit's.
//!
//! Both are XMP, their tool entries written exactly as an `.apxmp` of version 1 writes
//! them, by [`sidecar`]'s own code, so the same values give the same bytes:
//!
//! ```xml
//! <rdf:Description rdf:about=""
//!   apertrust:preset="1"
//!   apertrust:name="Bright interiors">
//!  <apertrust:tools>
//!   <rdf:Seq>
//!    <rdf:li>
//!     <rdf:Description
//!       apertrust:tool="exposure"
//!       apertrust:toolVersion="3"
//!       apertrust:kept="exposure"
//!       control:exposure="1.2"/>
//!    </rdf:li>
//!   </rdf:Seq>
//!  </apertrust:tools>
//! </rdf:Description>
//! ```
//!
//! A style has `apertrust:style` in place of `apertrust:preset`. A utility's preset,
//! Export's or the Metadata tool's, holds the utility's id, `apertrust:tool`, and its
//! settings as the `setting:` fields of the struct `apertrust:settings`
//! ([`SETTING_NAMESPACE`]), each as text: a list as its items joined by single spaces, as the
//! Metadata tool's rows and groups, whose ids are snake_case; a value the tool's settings
//! do not declare, as its `metadata.shown_only`, is the tool's own to read.
//!
//! The built-in presets, Export's For sharing and Full size JPEG and the Metadata tool's
//! Essentials, Everything and IPTC for agencies, are the repository's
//! `data/presets/<tool>/<id>.appreset`, and the built-in styles its
//! `data/styles/<id>.apstyle`, embedded here, the one crate that parses them (decision
//! 45), so the app and the CLI both have them ([`builtins`], [`builtin_styles`]); they are
//! never copied into the user data folder.
//!
//! A file's name is its name made safe for every system ([`file_name`]), numbered where
//! another file has it. A file a newer Apertrust wrote, of a newer format version or with
//! a field this build does not know, of any namespace, is read as this build lays it out
//! and says so ([`Stored::newer`]); a newer tool version or a tool this build lacks only
//! the engine can tell.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::listing::browser_order;
use crate::sidecar::{
    self, CONTROL_NAMESPACE, Entry, NAMESPACE, Newer, Problem, TOOLKIT, entry_list, entry_problems,
    own, read_entries,
};
use crate::xmp::{self, ArrayKind, Name, Namespace, Node, Property, Value, Xmp};

/// The format version this build reads and writes in `apertrust:preset` and
/// `apertrust:style`.
pub const FORMAT_VERSION: u32 = 1;
/// The namespace of a utility preset's settings, by their keys.
pub const SETTING_NAMESPACE: &str = "https://apertrust.org/ns/xmp/setting/";
/// The most bytes of a name a file name keeps, before its number and extension, so both
/// fit in the 255 bytes every system allows.
const MAX_NAME_BYTES: usize = 200;

/// A preset or a style built in (decision 106): its own id, by which the client names it
/// (the Fluent message `preset-<tool>-<id>` or `style-builtin-<id>`,
/// `apertrust-cli export --preset <id>`), and its file, whose `apertrust:name` is its
/// British English name.
struct Builtin {
    id: &'static str,
    text: &'static str,
}

/// Every preset built in, by its tool's id, each tool's in the order its list shows them:
/// `data/presets/<tool>/<id>.appreset`.
const PRESETS: &[(&str, Builtin)] = &[
    (
        "export",
        Builtin {
            id: "for-sharing",
            text: include_str!("../../../data/presets/export/for-sharing.appreset"),
        },
    ),
    (
        "export",
        Builtin {
            id: "full-size-jpeg",
            text: include_str!("../../../data/presets/export/full-size-jpeg.appreset"),
        },
    ),
    (
        "metadata",
        Builtin {
            id: "essentials",
            text: include_str!("../../../data/presets/metadata/essentials.appreset"),
        },
    ),
    (
        "metadata",
        Builtin {
            id: "everything",
            text: include_str!("../../../data/presets/metadata/everything.appreset"),
        },
    ),
    (
        "metadata",
        Builtin {
            id: "iptc-for-agencies",
            text: include_str!("../../../data/presets/metadata/iptc-for-agencies.appreset"),
        },
    ),
];

/// A style built in, `data/styles/<id>.apstyle`.
macro_rules! style {
    ($id:literal) => {
        Builtin {
            id: $id,
            text: include_str!(concat!("../../../data/styles/", $id, ".apstyle")),
        }
    };
}

/// Every style built in, each one tool's preset of darktable's at the reference release,
/// its values as darktable's `init_presets` sets them (`data/README.md`).
const STYLES: &[Builtin] = &[
    style!("bw-luminance"),
    style!("bw-hp5"),
    style!("bw-acros-100"),
    style!("natural-colour"),
    style!("vibrant-colour"),
    style!("punchy"),
    style!("open-shadows-highlights"),
    style!("lomo"),
];

/// The presets built in for `tool`, in the order its list shows them, each by its id with
/// its file read; none for a tool that has none. Each reads, as a test holds.
pub fn builtins(tool: &str) -> Vec<(&'static str, Stored)> {
    let of_tool = PRESETS.iter().filter(|(of, _)| *of == tool);
    read_builtins(of_tool.map(|(_, builtin)| builtin), Kind::Preset)
}

/// The styles built in, each by its id with its file read. Each reads, as a test holds.
pub fn builtin_styles() -> Vec<(&'static str, Stored)> {
    read_builtins(STYLES.iter(), Kind::Style)
}

/// `builtins` read as files of `kind`, each by its id.
fn read_builtins<'a>(
    builtins: impl Iterator<Item = &'a Builtin>,
    kind: Kind,
) -> Vec<(&'static str, Stored)> {
    builtins
        .filter_map(|builtin| match parse(builtin.text, kind) {
            Ok(stored) => Some((builtin.id, stored)),
            Err(error) => {
                debug_assert!(false, "the built-in {}: {error}", builtin.id);
                None
            }
        })
        .collect()
}

/// Which of the two files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One preset of one tool, `.appreset`.
    Preset,
    /// One style of one or more tools, `.apstyle`.
    Style,
}

impl Kind {
    /// The file's extension, without its dot, in lower case as Apertrust writes it.
    pub fn extension(self) -> &'static str {
        match self {
            Kind::Preset => "appreset",
            Kind::Style => "apstyle",
        }
    }

    /// The kind a file's extension names, matched as written: `None` for any other file.
    pub fn of(path: &Path) -> Option<Kind> {
        let extension = path.extension()?;
        [Kind::Preset, Kind::Style]
            .into_iter()
            .find(|kind| extension == kind.extension())
    }

    /// The root property holding the format version, `apertrust:preset` or
    /// `apertrust:style`.
    fn property(self) -> &'static str {
        match self {
            Kind::Preset => "preset",
            Kind::Style => "style",
        }
    }
}

#[derive(Debug, Error)]
pub enum PresetError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("not XMP: {0}")]
    Xml(#[from] xmp::Error),
    /// No `apertrust:preset` or `apertrust:style`, whichever the kind asks.
    #[error("not an Apertrust preset or style")]
    NotAPreset,
    /// A format version that is no whole number, so no layout this build knows.
    #[error("a preset or style of version {0}, which this Apertrust cannot read")]
    Version(String),
    /// A name another file in the folder holds.
    #[error("another file in the folder is named {0}")]
    NameTaken(String),
    /// No name: empty, or spaces alone.
    #[error("a preset or style needs a name")]
    EmptyName,
    /// A file a newer Apertrust wrote ([`Stored::newer`]), which is never rewritten.
    #[error("the file {0}: never rewritten")]
    Newer(Newer),
}

/// One preset or style file as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub kind: Kind,
    /// `apertrust:name`, the name as typed, its spaces at both ends trimmed as it is
    /// written.
    pub name: String,
    /// `apertrust:description`, where it has one.
    pub description: Option<String>,
    /// The `Entry::Tool`s of `apertrust:tools`, in the file's order: one for a tool's
    /// preset, none for a utility's, one or more for a style.
    pub entries: Vec<Entry>,
    /// A utility preset's utility, by its id: `apertrust:tool` of `apertrust:settings`.
    pub settings_tool: Option<String>,
    /// A utility preset's `setting:` fields, by key and text, in the file's order.
    pub settings: Vec<(String, String)>,
    /// Its format version, `apertrust:preset` or `apertrust:style`; [`FORMAT_VERSION`]
    /// for one made to be written.
    pub version: u32,
    /// What in the file this build does not know, or does not read as written, so a write
    /// would drop it: each a [`Problem::UnknownField`], as version 2 of the `.apxmp` tells
    /// them. Any one parks the file ([`Stored::newer`]).
    pub problems: Vec<Problem>,
    /// The file it was read from; empty for one not read from a file.
    pub path: PathBuf,
}

impl Stored {
    /// A file of `kind` named `name`, holding nothing yet, to be written.
    pub fn new(kind: Kind, name: &str) -> Self {
        Stored {
            kind,
            name: name.to_owned(),
            description: None,
            entries: Vec::new(),
            settings_tool: None,
            settings: Vec::new(),
            version: FORMAT_VERSION,
            problems: Vec::new(),
            path: PathBuf::new(),
        }
    }

    /// What tells from the file alone that a newer Apertrust wrote it: a format version
    /// above [`FORMAT_VERSION`], else the first of [`Stored::problems`], a field this
    /// build does not know. Such a file is parked, listed but not applied, and never
    /// rewritten: [`write()`] and [`rename`] refuse it, and [`write()`] refuses its name.
    pub fn newer(&self) -> Option<Newer> {
        if self.version > FORMAT_VERSION {
            return Some(Newer::FormatVersion {
                version: self.version,
                has: FORMAT_VERSION,
            });
        }
        self.problems.first().cloned().map(Newer::Field)
    }

    /// The file as XMP, `build` recorded in `x:xmptk`, in a layout that never varies: its
    /// version, name and description, then its tool entries as an `.apxmp` writes them,
    /// then its settings. A part it does not have is left out.
    pub fn to_xml(&self, build: &str) -> String {
        let name = |local: &str| Name::new("apertrust", NAMESPACE, local);
        let mut namespaces = vec![
            Namespace::new("apertrust", NAMESPACE),
            Namespace::new("control", CONTROL_NAMESPACE),
        ];
        let mut properties = vec![
            own(self.kind.property(), &self.version.to_string()),
            own("name", &self.name),
        ];
        if let Some(description) = &self.description {
            properties.push(own("description", description));
        }
        let tools: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|entry| matches!(entry, Entry::Tool { .. }))
            .collect();
        if !tools.is_empty() {
            properties.push(Property::new(name("tools"), entry_list(tools)));
        }
        if self.settings_tool.is_some() || !self.settings.is_empty() {
            namespaces.push(Namespace::new("setting", SETTING_NAMESPACE));
            let tool = self.settings_tool.iter().map(|tool| own("tool", tool));
            let settings = self.settings.iter().map(|(key, text)| {
                Property::simple(Name::new("setting", SETTING_NAMESPACE, key), text)
            });
            let fields = tool.chain(settings).collect();
            properties.push(Property::new(name("settings"), Node::structure(fields)));
        }
        Xmp {
            toolkit: Some(format!("{TOOLKIT}{build}")),
            about: String::new(),
            namespaces,
            properties,
        }
        .to_xml()
    }
}

/// Reads a preset or style file's XMP through the XMP layer: its format version, name,
/// description, tool entries and settings, every value as the text the file holds. An
/// item of `apertrust:tools` that is no tool's is left out, and told in
/// [`Stored::problems`] with every other field a write would drop.
pub fn parse(xml: &str, kind: Kind) -> Result<Stored, PresetError> {
    let xmp = Xmp::parse(xml)?;
    let text = |local: &str| xmp.get(NAMESPACE, local).and_then(Node::as_str);
    let version = text(kind.property()).ok_or(PresetError::NotAPreset)?;
    let version = version
        .trim()
        .parse()
        .map_err(|_| PresetError::Version(version.to_owned()))?;
    let name = text("name")
        .filter(|name| !name.trim().is_empty())
        .ok_or(PresetError::EmptyName)?;
    let entries = xmp
        .get(NAMESPACE, "tools")
        .map(read_entries)
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| matches!(entry, Entry::Tool { .. }))
        .collect();
    let fields = xmp
        .get(NAMESPACE, "settings")
        .and_then(Node::fields)
        .unwrap_or_default();
    let settings = fields
        .iter()
        .filter(|field| field.name.namespace == SETTING_NAMESPACE)
        .filter_map(|field| Some((field.name.local.clone(), field.node.as_str()?.to_owned())))
        .collect();
    let settings_tool = fields
        .iter()
        .find(|field| field.name.is(NAMESPACE, "tool"))
        .and_then(|field| field.node.as_str())
        .map(str::to_owned);
    Ok(Stored {
        kind,
        name: name.to_owned(),
        description: text("description").map(str::to_owned),
        entries,
        settings_tool,
        settings,
        version,
        problems: problems(&xmp, kind),
        path: PathBuf::new(),
    })
}

/// What in a file's XMP [`parse`] does not read as written, so a write would drop it, as
/// [`sidecar`]'s version 2 tells it of an edit (decision 106): a root property, of
/// Apertrust's namespace or another, other than the kind's version, `name`,
/// `description`, `tools` and `settings`, one of those twice or in another shape than this
/// build writes; an entry's field ([`entry_problems`]) or an operation's entry; and in
/// `settings` a field other than `apertrust:tool`, once, and the `setting:` ones.
fn problems(xmp: &Xmp, kind: Kind) -> Vec<Problem> {
    let unknown = |place: &str, field: &Property| Problem::UnknownField {
        place: place.to_owned(),
        field: format!("{}:{}", field.name.prefix, field.name.local),
    };
    let plain = |node: &Node| matches!(node.value, Value::Simple(_)) && node.qualifiers.is_empty();
    let mut problems = Vec::new();
    let mut seen = Vec::new();
    for property in &xmp.properties {
        let local = property.name.local.as_str();
        let node = &property.node;
        let own = property.name.namespace == NAMESPACE;
        let known = own
            && match local {
                "tools" => {
                    let place = "the file's tools";
                    entry_problems(node, place, &mut problems);
                    if read_entries(node)
                        .iter()
                        .any(|entry| matches!(entry, Entry::Operation { .. }))
                    {
                        problems.push(Problem::UnknownField {
                            place: place.to_owned(),
                            field: "an operation's entry".to_owned(),
                        });
                    }
                    matches!(node.value, Value::Array(ArrayKind::Seq, _))
                        && node.qualifiers.is_empty()
                }
                "settings" => {
                    let mut tool = false;
                    for field in node.fields().unwrap_or_default() {
                        let known = if field.name.is(NAMESPACE, "tool") {
                            !std::mem::replace(&mut tool, true)
                        } else {
                            field.name.namespace == SETTING_NAMESPACE
                        };
                        if !known || !plain(&field.node) {
                            problems.push(unknown("the file's settings", field));
                        }
                    }
                    node.fields().is_some() && node.qualifiers.is_empty()
                }
                "name" | "description" => plain(node),
                local => local == kind.property() && plain(node),
            };
        if !known || seen.contains(&local) {
            problems.push(unknown("the file", property));
        }
        if own {
            seen.push(local);
        }
    }
    problems
}

/// Reads the file at `path` ([`parse`]), with its path.
fn read(path: &Path, kind: Kind) -> Result<Stored, PresetError> {
    let mut stored = parse(&fs::read_to_string(path)?, kind)?;
    stored.path = path.to_owned();
    Ok(stored)
}

/// The file name a name gives, without its extension (decision 106): `/ \ : * ? " < > |`
/// and control characters replaced by `_`, leading and trailing dots and spaces trimmed,
/// and at most 200 bytes kept, cut at a character's boundary, so a number and the
/// extension still fit; then a `_` after a name Windows keeps for a device (CON, PRN, AUX,
/// NUL, COM1 to COM9, LPT1 to LPT9, in any case, alone or before a dot, as `CON.txt`).
/// `_` for a name that leaves nothing.
pub fn file_name(name: &str) -> String {
    let trim = |text: &str| text.trim_matches(['.', ' ']).to_owned();
    let safe: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let safe = trim(&safe);
    let end = (0..=safe.len().min(MAX_NAME_BYTES))
        .rev()
        .find(|&end| safe.is_char_boundary(end))
        .unwrap_or(0);
    let mut file = trim(&safe[..end]);
    if file.is_empty() {
        return "_".to_owned();
    }
    // Windows takes the part before the first dot, spaces at its end ignored, for the name.
    let device = file
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    if is_device(device) {
        file.insert(device.len(), '_');
    }
    file
}

/// Whether Windows keeps `name` for a device, whatever its case.
fn is_device(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    match name.as_bytes() {
        b"CON" | b"PRN" | b"AUX" | b"NUL" => true,
        [b'C', b'O', b'M', digit] | [b'L', b'P', b'T', digit] => (b'1'..=b'9').contains(digit),
        _ => false,
    }
}

/// What [`list`] found in a folder.
#[derive(Debug, Default)]
pub struct Listed {
    /// Every file that reads, by name in the file browser's order ([`browser_order`]).
    pub files: Vec<Stored>,
    /// Every file that does not, by path, with why; the folder itself where it cannot be
    /// read.
    pub failed: Vec<(PathBuf, PresetError)>,
}

/// Every file of `kind` directly in `folder`, read: those whose extension is the kind's,
/// as written, but hidden ones (a name starting with a dot). A folder that does not exist
/// holds none; a file that cannot be read is reported without failing the rest.
pub fn list(folder: &Path, kind: Kind) -> Listed {
    let mut listed = Listed::default();
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return listed,
        Err(error) => {
            listed.failed.push((folder.to_owned(), error.into()));
            return listed;
        }
    };
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                listed.failed.push((folder.to_owned(), error.into()));
                continue;
            }
        };
        let hidden = path
            .file_name()
            .is_none_or(|name| name.as_encoded_bytes().starts_with(b"."));
        if hidden || Kind::of(&path) != Some(kind) || !path.is_file() {
            continue;
        }
        match read(&path, kind) {
            Ok(stored) => listed.files.push(stored),
            Err(error) => listed.failed.push((path, error)),
        }
    }
    listed
        .files
        .sort_by(|a, b| browser_order(&a.name, &b.name).then_with(|| a.path.cmp(&b.path)));
    listed.failed.sort_by(|a, b| a.0.cmp(&b.0));
    listed
}

/// Writes `stored` into `folder`, made if it does not exist, through the safe write
/// ([`sidecar::write_xml`], decision 53), `build` recorded in `x:xmptk`, and returns its
/// path. The name is written with its spaces at both ends trimmed. The file already
/// holding that name is replaced, whatever its file name; otherwise the file is named by
/// [`file_name`], and where another file has that file name, ` 2`, ` 3` and on are tried:
/// compared ignoring case, and taken wherever the file system already has a file by it,
/// as macOS has `Café` under either of its Unicode forms. Refuses a parked `stored`, and a
/// name a parked file holds ([`PresetError::Newer`]).
pub fn write(folder: &Path, stored: &Stored, build: &str) -> Result<PathBuf, PresetError> {
    if let Some(newer) = stored.newer() {
        return Err(PresetError::Newer(newer));
    }
    let name = checked(&stored.name)?;
    fs::create_dir_all(folder)?;
    let mut same = None;
    for file in list(folder, stored.kind).files {
        if file.name.trim() != name {
            continue;
        }
        if let Some(newer) = file.newer() {
            return Err(PresetError::Newer(newer));
        }
        same.get_or_insert(file.path);
    }
    let path = match same {
        Some(path) => path,
        None => free(folder, &file_name(name), stored.kind, None)?,
    };
    let xml = if stored.name == name {
        stored.to_xml(build)
    } else {
        Stored {
            name: name.to_owned(),
            ..stored.clone()
        }
        .to_xml(build)
    };
    sidecar::write_xml(&path, &xml)?;
    Ok(path)
}

/// Renames the file at `path`: rewrites its `apertrust:name` to `new_name`, its spaces at
/// both ends trimmed, everything else in it kept, then renames the file to match
/// ([`file_name`]), numbered as [`write()`] numbers it. Refuses a name another file of its
/// kind in the folder holds ([`PresetError::NameTaken`]), and a parked file
/// ([`PresetError::Newer`]). Returns the file's new path.
///
/// The name is rewritten first: where the file's rename then fails, the file holds the new
/// name under its old file name, and is listed by it.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, PresetError> {
    let kind = Kind::of(path).ok_or(PresetError::NotAPreset)?;
    let name = checked(new_name)?;
    let text = fs::read_to_string(path)?;
    if let Some(newer) = parse(&text, kind)?.newer() {
        return Err(PresetError::Newer(newer));
    }
    // A bare file name's parent is empty: the current folder.
    let folder = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let own_file = path.file_name().unwrap_or_default();
    let other = list(folder, kind)
        .files
        .into_iter()
        .any(|stored| stored.name.trim() == name && stored.path.file_name() != Some(own_file));
    if other {
        return Err(PresetError::NameTaken(name.to_owned()));
    }
    let target = free(folder, &file_name(name), kind, Some(path))?;
    let mut xmp = Xmp::parse(&text)?;
    xmp.set(own("name", name));
    sidecar::write_xml(path, &xmp.to_xml())?;
    if target.file_name() != Some(own_file) {
        fs::rename(path, &target)?;
    }
    Ok(target)
}

/// Deletes the file at `path`.
pub fn delete(path: &Path) -> Result<(), PresetError> {
    Ok(fs::remove_file(path)?)
}

/// The name with its spaces at both ends trimmed, refusing one left empty.
fn checked(name: &str) -> Result<&str, PresetError> {
    let name = name.trim();
    if name.is_empty() {
        Err(PresetError::EmptyName)
    } else {
        Ok(name)
    }
}

/// The first free file name for `base` in `folder` ([`candidates`]): no entry has it,
/// compared ignoring case, and the file system finds no file by it but `own`, the file
/// being renamed, which counts as free.
fn free(folder: &Path, base: &str, kind: Kind, own: Option<&Path>) -> Result<PathBuf, PresetError> {
    let own_file = own.and_then(Path::file_name);
    let taken: Vec<String> = names(folder)?
        .into_iter()
        .filter(|name| Some(OsStr::new(name)) != own_file)
        .collect();
    candidates(base, kind, taken.len())
        .filter(|candidate| find(&taken, candidate).is_none())
        .map(|candidate| folder.join(candidate))
        .find(|path| match fs::symlink_metadata(path) {
            Err(_) => true,
            Ok(_) => own.is_some_and(|own| same_file(own, path)),
        })
        .ok_or_else(|| PresetError::NameTaken(base.to_owned()))
}

/// Whether two paths name one file, as the file system resolves them.
fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// The names of every entry of `folder`, one that is not text with its bytes made text.
fn names(folder: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(folder)? {
        names.push(entry?.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// The file names to try for `base`, in order: itself, then ` 2`, ` 3` and on, enough
/// that one is free of `taken` names.
fn candidates(base: &str, kind: Kind, taken: usize) -> impl Iterator<Item = String> {
    let extension = kind.extension();
    (1..=taken + 1).map(move |number| match number {
        1 => format!("{base}.{extension}"),
        n => format!("{base} {n}.{extension}"),
    })
}

/// The name in `names` that is `name`, ignoring case.
fn find<'a>(names: &'a [String], name: &str) -> Option<&'a str> {
    let name = name.to_lowercase();
    names
        .iter()
        .find(|other| other.to_lowercase() == name)
        .map(String::as_str)
}
