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

//! Edits saved beside the photo (`docs/library.md`, Sidecars): `IMG.ARW.apxmp`,
//! an XMP file. Version 2 (decisions 47 and 78), which the engine writes, holds the edit's
//! history ([`crate::history`]): its virtual copies, the first the one a photo opens, each
//! with its clock vector, its log of steps and its end, each step one module's whole state
//! after it written as version 1 writes a module. The edit is the log's fold. Version 1
//! (decision 28) held the [`Edit`] alone: each module it touches, in the modules' order,
//! with its version, the controls set and its marks, then the operation parameters no
//! module owns, only of operations an unedited photo runs, each operation with its schema
//! version as version 0 had them (decision 48). It reads as one copy, a step of each of
//! its entries sharing a batch, and is written as version 2 at the next change.
//!
//! The file's text layer, its XMP, structure, versions and build record, is
//! `apertrust_library::sidecar`'s (decision 54), which knows no module or operation: this
//! module is the typed reading and writing on top of it. It turns the entries of the
//! first copy's steps ([`Edits`]), which hold their values as text, into the steps of a
//! [`Log`] with the module definitions, and a log into entries. See that crate for the
//! file's layout.
//!
//! Numbers are written in their shortest exact form, choices by option id, points as
//! `x,y` pairs separated by spaces, and the marks (`apertrust:kept`, `apertrust:auto`)
//! as lists of control ids. An untouched control is left out. A switched module that is on
//! has `apertrust:enabled="true"`; one off writes no values, and only its marks, where it
//! has any (decision 32).
//!
//! A saved edit always opens (decision 21): a module, control, operation or parameter
//! the engine does not know, or a value it cannot read, is left out, a number outside
//! its range is clamped, an older module version is migrated, and a newer module or
//! another schema version is read by name, each with a warning, in every entry of every
//! step. A parameter a module owns, or one of an operation an unedited photo does not
//! run, is left out with a warning too, and a step left with no entry is left out. Version
//! 0, which held only operation parameters, still opens that way.
//!
//! The file records the build that wrote it, in `x:xmptk` (decision 44's version, which
//! the binaries hand the engine). An edit a newer Apertrust wrote is told by what it holds
//! ([`Newer`], decision 43): a file version above [`COPIES_VERSION`], read as this version
//! lays it out, a module version above its definition's or no whole number, or a module
//! this build neither has nor has retired (`apertrust_modules::RETIRED`), whose values a
//! save would lose, or a module turned on that this build does not switch (decision 32).
//! So is one of whose history the text layer cannot read part ([`Problem`]). Such an edit
//! opens for viewing only ([`ViewOnly`]); a retired module's values are left out with a
//! warning, as an older build wrote them. A switched module off that holds a value off its
//! defaults, which no build writes, is read as on, with a warning.

use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use apertrust_library::clock::{Clock, MachineId};
use apertrust_modules::{ControlValue, Definition, Mark, Module, ModuleValues, check};
use apertrust_plugin::{Operation, ParamValues};

use apertrust_library::sidecar::{self as text, Document, Entry, FIRST_COPY, VirtualCopy};
pub use apertrust_library::sidecar::{
    CONTROL_NAMESPACE, COPIES_VERSION, Edits, NAMESPACE, Newer, PARAM_NAMESPACE, Problem, Reason,
    SidecarError, ViewOnly, path_for, read_fingerprint, write_xml,
};
pub(crate) use apertrust_library::sidecar::{file_name, writer};

use crate::edit::Edit;
use crate::history::{self, BatchId, Change, Log, Note, Step, StepId, StepSource};

/// An edit read back: its log and the edit it folds to, the version it was saved as, the
/// build that wrote it, what could not be used, and the file as text, which a write keeps.
pub struct Loaded {
    /// The log's fold, each module's last entry up to the end.
    pub edit: Edit,
    /// The first virtual copy's history: a version 1 file's a step of each entry.
    pub log: Log,
    pub version: u32,
    /// The build that wrote it, as its `x:xmptk` records it; `None` for a file that
    /// records no Apertrust build.
    pub build: Option<String>,
    /// The first thing it holds that tells a newer Apertrust wrote it, or `None` where a
    /// save would keep everything it holds.
    pub newer: Option<Newer>,
    /// What of its history the text layer could not read as written ([`Edits::problems`]).
    pub problems: Vec<Problem>,
    pub warnings: Vec<String>,
    /// The file as text: the photo's name and fingerprint, and its copies, the first the
    /// log's, with its id, number, name and clock vector, which [`log_xml`] writes the log
    /// into, the others kept as read.
    pub file: Edits,
}

impl Loaded {
    /// The edit as one this build opens for viewing only, a newer Apertrust's first, then
    /// one with a problem; or `None` where this build may save it.
    pub fn view_only(&self) -> Option<ViewOnly> {
        let reason = match (&self.newer, self.problems.first()) {
            (Some(newer), _) => Reason::Newer(newer.clone()),
            (None, Some(problem)) => Reason::Unreadable(problem.clone()),
            (None, None) => return None,
        };
        Some(ViewOnly {
            build: self.build.clone(),
            reason,
        })
    }

    /// The log's copy's clock vector: the latest clock of each machine that changed it.
    pub fn clocks(&self) -> &[Clock] {
        self.file.copies.first().map_or(&[], |copy| &copy.clocks)
    }
}

/// Writes `edit` to `path` as `build` writes version 1 ([`to_xml`], [`write_xml`]): a file
/// every build reads, as tests and fixtures make one; the engine saves with [`write_log`].
pub fn write(
    path: &Path,
    edit: &Edit,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    build: &str,
) -> io::Result<()> {
    write_xml(path, &to_xml(edit, operations, modules, build))
}

/// Writes `log` to `path` as `build` writes it ([`log_xml`], [`write_xml`]), through the
/// safe write, so a write cut off midway leaves the file as it was.
pub fn write_log(
    path: &Path,
    file: &Edits,
    log: &Log,
    clocks: &[Clock],
    (operations, modules): (&[&dyn Operation], &[&dyn Module]),
    build: &str,
) -> io::Result<()> {
    write_xml(
        path,
        &log_xml(file, log, clocks, (operations, modules), build),
    )
}

/// Reads the edit at `path` for `operations` and `modules`, with `retired` the ids of the
/// modules Apertrust has retired ([`from_xml_at`]), its modified time naming a version 1
/// file's steps, logging what is left out.
pub fn read(
    path: &Path,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Result<Loaded, SidecarError> {
    let loaded = from_edits(text::read_edits(path)?, operations, modules, retired);
    for warning in &loaded.warnings {
        tracing::warn!(target: "pipe", "{}: {warning}", path.display());
    }
    Ok(loaded)
}

/// The XMP of an edit as `build` writes version 1, the build recorded in `x:xmptk`: every
/// module it touches in the order of `modules`, then every operation it sets a parameter
/// of in the order of `operations`. Every build reads it; the engine writes version 2
/// ([`log_xml`]).
pub fn to_xml(
    edit: &Edit,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    build: &str,
) -> String {
    let mut document = Document::new(build);
    for module in modules {
        let definition = module.definition();
        if let Some(values) = edit.modules.get(definition.id).filter(|v| !v.is_empty()) {
            document.entries.push(module_entry(definition, values));
        }
    }
    for operation in operations {
        let manifest = operation.manifest();
        if let Some(params) = (edit.params.get(manifest.id)).filter(|p| p.iter().next().is_some()) {
            document.entries.push(operation_entry(*operation, params));
        }
    }
    document.to_xml()
}

/// The XMP of `log` as `build` writes it, version 2, the build recorded in `x:xmptk`:
/// `file`'s photo, fingerprint and copies as read, but the first copy's log, end and clock
/// vector, which are `log`'s and `clocks`; for a file with no copy, the first copy,
/// [`FIRST_COPY`]. Each step's entries in its order, each written as version 1 writes it.
pub fn log_xml(
    file: &Edits,
    log: &Log,
    clocks: &[Clock],
    (operations, modules): (&[&dyn Operation], &[&dyn Module]),
    build: &str,
) -> String {
    let first = file.copies.first();
    let copy = VirtualCopy {
        id: first.map_or_else(|| FIRST_COPY.to_owned(), |copy| copy.id.clone()),
        number: first.map_or_else(|| "0".to_owned(), |copy| copy.number.clone()),
        name: first.map(|copy| copy.name.clone()).unwrap_or_default(),
        end: (log.steps()[..log.end()].last()).map_or_else(String::new, |step| step.id.to_string()),
        clocks: clocks.to_vec(),
        history: (log.steps().iter())
            .map(|step| step_text(step, operations, modules))
            .collect(),
    };
    let others = file.copies.iter().skip(1).cloned();
    Edits {
        version: COPIES_VERSION,
        build: Some(build.to_owned()),
        photo: file.photo.clone(),
        fingerprint: file.fingerprint.clone(),
        copies: std::iter::once(copy).chain(others).collect(),
        other: file.other.clone(),
        problems: Vec::new(),
    }
    .to_xml()
}

/// Reads an edit's XMP for `operations` and `modules` ([`from_xml_at`]), as a file with no
/// modified time.
pub fn from_xml(
    xml: &str,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Result<Loaded, SidecarError> {
    from_xml_at(xml, None, operations, modules, retired)
}

/// Reads an edit's XMP for `operations` and `modules`, its file last modified at
/// `modified`, which a version 1 file's steps take as their time. A module none of them
/// is, but one of `retired`, the ids of the modules Apertrust has retired, is an older
/// build's: left out with a warning, as any unknown module is, but not [`Newer`].
pub fn from_xml_at(
    xml: &str,
    modified: Option<SystemTime>,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Result<Loaded, SidecarError> {
    let modified = (modified.and_then(|modified| modified.duration_since(UNIX_EPOCH).ok()))
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        });
    let file = Edits::parse(xml, modified)?;
    Ok(from_edits(file, operations, modules, retired))
}

/// The log the first copy of `file` holds, and the edit it folds to, for `operations` and
/// `modules`: every entry of every step read as version 1's entry is, a step none of whose
/// entries this build keeps left out, its place in the end with it.
fn from_edits(
    file: Edits,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Loaded {
    let mut newer = file.newer();
    let mut warnings = Vec::new();
    if newer.is_some() {
        let version = file.version;
        warnings.push(format!(
            "file version {version}, newer than this build's {COPIES_VERSION}; read as version {COPIES_VERSION}"
        ));
    }
    let (mut steps, mut end) = (Vec::new(), 0);
    if let Some(copy) = file.copies.first() {
        let applied = copy.applied();
        for (at, step) in copy.history.iter().enumerate() {
            let mut entries = Vec::new();
            for entry in &step.entries {
                let (read, found) =
                    read_entry(entry.clone(), &mut warnings, (operations, modules), retired);
                newer = newer.or(found);
                entries.extend(read);
            }
            if entries.is_empty() && !step.entries.is_empty() {
                continue;
            }
            end += usize::from(at < applied);
            steps.push(typed_step(step, entries));
        }
    }
    // A warning once, however many steps hold the same entry.
    let mut seen = HashSet::new();
    warnings.retain(|warning| seen.insert(warning.clone()));
    let log = Log::from_steps(steps, end);
    Loaded {
        edit: log.fold(),
        log,
        version: file.version,
        build: file.build.clone(),
        newer,
        problems: file.problems.clone(),
        warnings,
        file,
    }
}

/// A step of the text layer's with `entries`, its entries read: Reset's where it is noted
/// `history-reset` or holds other than one entry, else its one module's. What cannot be
/// read, which the text layer told as a [`Problem`], is read as nothing: an id or a batch
/// as 0, a source as You, a clock as the unknown machine's at 1970.
fn typed_step(step: &text::Step, mut entries: Vec<history::Entry>) -> Step {
    let hex = |text: &str| u64::from_str_radix(text, 16).unwrap_or(0);
    let note = step.note.clone().map(|note| match note {
        text::Note::Message { id, args } => Note::Message { id, args },
        text::Note::Text(words) => Note::Text(words),
    });
    let reset = matches!(&note, Some(Note::Message { id, .. }) if id == Note::RESET);
    let change = match entries.len() {
        1 if !reset => Change::Entry(entries.remove(0)),
        _ => Change::Reset(entries),
    };
    Step {
        id: StepId(hex(&step.id)),
        clock: (step.clock).unwrap_or_else(|| Clock::at(0, None, MachineId::UNKNOWN)),
        source: StepSource::from_name(&step.source).unwrap_or(StepSource::You),
        note,
        batch: step.batch.as_deref().map(|batch| BatchId(hex(batch))),
        change,
    }
}

/// One entry of a step read for `operations` and `modules`: a module's whole state, or an
/// operation's unowned parameters; a newer build's module, which this build does not have,
/// by its stored name with no value, so the History tab lists it in an edit open for
/// viewing only, never written; or nothing where this build keeps none of it, a retired
/// module or an operation it does not run unedited. And what in it tells a newer Apertrust
/// wrote it, if anything does.
fn read_entry(
    entry: Entry,
    warnings: &mut Vec<String>,
    (operations, modules): (&[&dyn Operation], &[&dyn Module]),
    retired: &[&str],
) -> (Option<history::Entry>, Option<Newer>) {
    let mut read = Edit::default();
    match entry {
        Entry::Module {
            module,
            version,
            kept,
            auto,
            enabled,
            controls,
        } => {
            let newer = read_module(
                &mut read,
                warnings,
                (modules, retired),
                (&module, version.as_deref()),
                [(kept, Mark::Kept), (auto, Mark::Auto)],
                (enabled, controls),
            );
            let known = modules.iter().any(|m| m.definition().id == module);
            let entry = (known || newer.is_some()).then(|| history::Entry::Module {
                values: read.modules.remove(&module).unwrap_or_default(),
                module,
            });
            (entry, newer)
        }
        Entry::Operation {
            operation,
            schema,
            params,
        } => {
            let kept = operations
                .iter()
                .any(|op| op.manifest().id == operation && op.manifest().default_enabled);
            read_operation(
                &mut read,
                warnings,
                operations,
                modules,
                (&operation, schema),
                params,
            );
            let entry = kept.then(|| history::Entry::Operation {
                params: read.params.remove(&operation).unwrap_or_default(),
                operation,
            });
            (entry, None)
        }
    }
}

/// A step as the text layer holds it: its id, clock, source, note and batch as text, and
/// its entries each as version 1 writes one.
fn step_text(step: &Step, operations: &[&dyn Operation], modules: &[&dyn Module]) -> text::Step {
    text::Step {
        id: step.id.to_string(),
        clock: Some(step.clock),
        source: step.source.name().to_owned(),
        note: step.note.clone().map(|note| match note {
            Note::Message { id, args } => text::Note::Message { id, args },
            Note::Text(words) => text::Note::Text(words),
        }),
        batch: step.batch.map(|batch| batch.to_string()),
        entries: (step.change.entries().iter())
            .map(|entry| entry_text(entry, operations, modules))
            .collect(),
    }
}

/// An entry as version 1 writes it; one of a module or an operation this build does not
/// have, which only an edit open for viewing only holds and which is never written, by its
/// id alone.
fn entry_text(
    entry: &history::Entry,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
) -> Entry {
    match entry {
        history::Entry::Module { module, values } => {
            match modules.iter().find(|m| m.definition().id == module) {
                Some(found) => module_entry(found.definition(), values),
                None => Entry::Module {
                    module: module.clone(),
                    version: None,
                    kept: Vec::new(),
                    auto: Vec::new(),
                    enabled: None,
                    controls: Vec::new(),
                },
            }
        }
        history::Entry::Operation { operation, params } => {
            match operations.iter().find(|op| op.manifest().id == operation) {
                Some(found) => operation_entry(*found, params),
                None => Entry::Operation {
                    operation: operation.clone(),
                    schema: None,
                    params: Vec::new(),
                },
            }
        }
    }
}

/// A module's state as its entry: its version, its marks, `enabled` where a switched module
/// is on, and each control set, in the definition's order.
fn module_entry(definition: &Definition, values: &ModuleValues) -> Entry {
    let marked = |mark| {
        definition
            .controls
            .iter()
            .map(|control| control.id)
            .filter(|id| values.mark(id) == Some(mark))
            .map(str::to_owned)
            .collect()
    };
    Entry::Module {
        module: definition.id.to_owned(),
        version: Some(definition.version.to_string()),
        kept: marked(Mark::Kept),
        auto: marked(Mark::Auto),
        enabled: values.enabled().then(|| "true".to_owned()),
        controls: definition
            .controls
            .iter()
            .filter_map(|control| {
                let value = values.get(control.id)?;
                Some((control.id.to_owned(), control.text(value).to_owned()))
            })
            .collect(),
    }
}

/// An operation's unowned parameters as its entry, with its schema's version.
fn operation_entry(operation: &dyn Operation, params: &ParamValues) -> Entry {
    let schema = operation.schema();
    Entry::Operation {
        operation: operation.manifest().id.to_owned(),
        schema: Some(schema.version),
        params: params
            .iter()
            .map(|(name, value)| {
                let text = schema
                    .param(name)
                    .map_or_else(|| value.to_string(), |param| param.text(value));
                (name.to_owned(), text)
            })
            .collect(),
    }
}

/// Reads one module's entry into `edit`, `enabled` its `apertrust:enabled` as written;
/// what in it tells a newer Apertrust wrote it, if anything does.
fn read_module(
    edit: &mut Edit,
    warnings: &mut Vec<String>,
    (modules, retired): (&[&dyn Module], &[&str]),
    (id, saved): (&str, Option<&str>),
    marks: [(Vec<String>, Mark); 2],
    (enabled, controls): (Option<String>, Vec<(String, String)>),
) -> Option<Newer> {
    let Some(module) = modules.iter().find(|m| m.definition().id == id) else {
        if retired.contains(&id) {
            warnings.push(format!("module {id} is retired; left out"));
            return None;
        }
        warnings.push(format!("no module {id}, a newer Apertrust's; left out"));
        return Some(Newer::Module(id.to_owned()));
    };
    let definition = module.definition();
    let has = definition.version;
    // The saved version as written, and as a number; one that is no whole number is no
    // version this build knows.
    let saved = saved.map(|text| (text.trim(), text.trim().parse::<u32>()));
    // A control an older version had and this one lacks goes to the migration, read by
    // the shape of its text.
    let older = matches!(saved, Some((_, Ok(saved))) if saved < has);
    // `true` is the one value this build writes; any other is a newer build's, which a
    // save would lose, so the module reads as if it had none and the photo is view-only.
    let (enabled, unknown) = match enabled.as_deref().map(str::trim) {
        None => (false, None),
        Some("true") => (true, None),
        Some(other) => (false, Some(other.to_owned())),
    };
    let mut values = ModuleValues::new();
    values.set_enabled(enabled);
    for (name, text) in controls {
        match definition.control(&name) {
            Some(control) => match control.parse(&text) {
                Ok(value) => values.set(control.id, value),
                Err(error) => warnings.push(format!("{id}: {error}; left out")),
            },
            None => match older.then(|| loose(&text)).flatten() {
                Some(value) => values.set(&name, value),
                None => warnings.push(format!("{id} has no control {name}; left out")),
            },
        }
    }
    for (names, mark) in marks {
        for name in names {
            if older || definition.control(&name).is_some() {
                values.set_mark(&name, Some(mark));
            } else {
                warnings.push(format!("{id} has no control {name} to mark; left out"));
            }
        }
    }
    let newer = match saved {
        Some((_, Ok(saved))) if saved < has => {
            values = module.migrate(saved, values);
            None
        }
        Some((_, Ok(saved))) if saved == has => None,
        Some((saved, _)) => {
            warnings.push(format!(
                "{id}: module version {saved}, newer than this build's {has}; controls read by name"
            ));
            Some(Newer::ModuleVersion {
                module: id.to_owned(),
                version: saved.to_owned(),
                has,
            })
        }
        None => {
            warnings.push(format!(
                "{id}: no module version, this build has {has}; controls read by name"
            ));
            None
        }
    };
    // What a migration gave back, and what was read, held to this version's controls.
    let mut checked = ModuleValues::new();
    for (name, value) in values.values() {
        let Some(control) = definition.control(name) else {
            warnings.push(format!("{id} has no control {name}; left out"));
            continue;
        };
        match control.clamp(value.clone()) {
            Ok(clamped) => {
                if &clamped != value {
                    warnings.push(format!(
                        "{id}.{name} is {}, out of range; set to {}",
                        control.text(value),
                        control.text(&clamped)
                    ));
                }
                checked.set(control.id, clamped);
            }
            Err(error) => warnings.push(format!("{id}: {error}; left out")),
        }
    }
    for (name, mark) in values.marks() {
        match definition.control(name) {
            Some(control) => checked.set_mark(control.id, Some(mark)),
            None => warnings.push(format!("{id} has no control {name} to mark; left out")),
        }
    }
    let mut enabled = switch(definition, &values, &mut checked, warnings);
    if let Some(unknown) = unknown {
        warnings.push(format!("{id}: enabled is {unknown:?}, a newer Apertrust's"));
        enabled = enabled.or(Some(Newer::Enabled(id.to_owned())));
    }
    let newer = newer.or(enabled);
    if !checked.is_empty() {
        edit.modules.insert(definition.id.to_owned(), checked);
    }
    newer
}

/// A module's on or off as read into `checked` from `values`, after its migration
/// (decision 32): a switched module on as `values` says, or as a value off its
/// definition's defaults says, which no build writes for one off, with a warning, so it
/// renders what it holds. A module this build does not switch is always on, and one
/// turned on is a newer build's.
///
/// The defaults compared are the definition's, not the module's for the photo, as no
/// image context is known while the file is read: a switched module whose defaults come
/// from the photo, as Lens correction's will, would take a value at its per-photo default
/// for one off it here, and read as on.
fn switch(
    definition: &Definition,
    values: &ModuleValues,
    checked: &mut ModuleValues,
    warnings: &mut Vec<String>,
) -> Option<Newer> {
    let id = definition.id;
    if !definition.switched {
        if values.enabled() {
            warnings.push(format!(
                "{id} is turned on, and this build does not switch it"
            ));
            return Some(Newer::Enabled(id.to_owned()));
        }
        return None;
    }
    let mut on = values.enabled();
    if !on
        && checked
            .off_defaults(&definition.defaults())
            .values()
            .next()
            .is_some()
    {
        warnings.push(format!("{id} is off but holds values; read as on"));
        on = true;
    }
    checked.set_enabled(on);
    None
}

/// A control's value read without its control, by the shape of its text: a number,
/// `true` or `false`, or points. An option's id cannot be told from other text, so a
/// migration that renames an option needs more than this.
fn loose(text: &str) -> Option<ControlValue> {
    let text = text.trim();
    if let Ok(number) = text.parse() {
        return Some(ControlValue::Number(number));
    }
    if let Ok(flag) = text.parse() {
        return Some(ControlValue::Bool(flag));
    }
    text.split_whitespace()
        .map(|pair| {
            let (x, y) = pair.split_once(',')?;
            Some([x.parse().ok()?, y.parse().ok()?])
        })
        .collect::<Option<Vec<_>>>()
        .filter(|points| !points.is_empty())
        .map(ControlValue::Points)
}

fn read_operation(
    edit: &mut Edit,
    warnings: &mut Vec<String>,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    (id, saved): (&str, Option<u32>),
    params: Vec<(String, String)>,
) {
    let Some(operation) = operations.iter().find(|op| op.manifest().id == id) else {
        warnings.push(format!("no operation {id}; left out"));
        return;
    };
    let manifest = operation.manifest();
    if !manifest.default_enabled {
        warnings.push(format!("{id}: runs only while a module sets it; left out"));
        return;
    }
    let schema = operation.schema();
    if saved != Some(schema.version) {
        warnings.push(format!(
            "{id}: schema version {}, this build has {}; read by name",
            saved.map_or("none".to_owned(), |v| v.to_string()),
            schema.version
        ));
    }
    for (name, text) in params {
        let Some(param) = schema.param(&name) else {
            warnings.push(format!("{id} has no parameter {name}; left out"));
            continue;
        };
        if let Some(owner) = check::owner(modules, manifest.id, param.name) {
            warnings.push(format!(
                "{id}.{name}: the module {} owns it now; left out",
                owner.definition().id
            ));
            continue;
        }
        match param.parse(&text).and_then(|value| {
            let clamped = param.clamp(value)?;
            Ok((value, clamped))
        }) {
            Ok((value, clamped)) => {
                if clamped != value {
                    warnings.push(format!(
                        "{id}.{name} is {text}, out of range; set to {}",
                        param.text(clamped)
                    ));
                }
                edit.params
                    .entry(manifest.id.to_owned())
                    .or_default()
                    .set(param.name, clamped);
            }
            Err(error) => warnings.push(format!("{id}: {error}; left out")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Duration;

    use apertrust_library::sidecar::RDF_NAMESPACE as RDF;
    use apertrust_modules::{
        ChoiceStyle, Control, ControlKind, ControlValue, Definition, Owned, Row, Setting, Slider,
        Tab, Widget,
    };
    use apertrust_plugin::{
        Domain, ImageContext, Kernel, Kind, Manifest, NotPorted, Param, ParamValues, Prepared,
        Schema, Slot, Stage, Value,
    };

    use super::*;

    /// An operation with one parameter of each kind, which an unedited photo runs.
    struct Fake;

    static MANIFEST: Manifest = Manifest {
        id: "fake",
        version: 1,
        slot: Slot {
            stage: Stage::CameraRgb,
            order: 21.0,
        },
        domain: Domain::Scene,
        layers: true,
        differentiable: false,
        radius: 0,
        default_enabled: true,
        provenance: None,
    };

    static SCHEMA: Schema = Schema {
        version: 2,
        params: &[
            Param {
                name: "amount",
                kind: Kind::Float {
                    default: 0.0,
                    min: -4.0,
                    max: 4.0,
                },
            },
            Param {
                name: "level",
                kind: Kind::Int {
                    default: 0,
                    min: 0,
                    max: 100,
                },
            },
            Param {
                name: "mode",
                kind: Kind::Choice {
                    default: 0,
                    choices: &["soft", "hard"],
                },
            },
            Param {
                name: "on",
                kind: Kind::Bool { default: true },
            },
            Param {
                name: "owned",
                kind: Kind::Float {
                    default: 0.0,
                    min: 0.0,
                    max: 1.0,
                },
            },
        ],
    };

    static KERNEL: Kernel = Kernel {
        name: "fake",
        wgsl: "",
        derivatives: false,
    };

    impl Operation for Fake {
        fn manifest(&self) -> &'static Manifest {
            &MANIFEST
        }

        fn schema(&self) -> &'static Schema {
            &SCHEMA
        }

        fn kernel(&self) -> &'static Kernel {
            &KERNEL
        }

        fn prepare(
            &self,
            _: &ParamValues,
            _: &apertrust_plugin::ImageContext,
            input: &apertrust_plugin::ImageDesc,
        ) -> Result<Prepared, NotPorted> {
            Ok(Prepared::per_pixel(input, Vec::new(), Vec::new()))
        }
    }

    /// The same, but run only while a module sets it.
    struct Optional;

    static OPTIONAL: Manifest = Manifest {
        id: "optional",
        version: 1,
        slot: Slot {
            stage: Stage::CameraRgb,
            order: 22.0,
        },
        domain: Domain::Scene,
        layers: true,
        differentiable: false,
        radius: 0,
        default_enabled: false,
        provenance: None,
    };

    impl Operation for Optional {
        fn manifest(&self) -> &'static Manifest {
            &OPTIONAL
        }

        fn schema(&self) -> &'static Schema {
            &SCHEMA
        }

        fn kernel(&self) -> &'static Kernel {
            &KERNEL
        }

        fn prepare(
            &self,
            params: &ParamValues,
            image: &apertrust_plugin::ImageContext,
            input: &apertrust_plugin::ImageDesc,
        ) -> Result<Prepared, NotPorted> {
            Fake.prepare(params, image, input)
        }
    }

    /// A module with a control of each kind, owning `fake.owned`, at version 2; version 1
    /// called `strength` `amount`.
    struct Look;

    static LOOK: Definition = Definition {
        id: "look",
        version: 2,
        layers: true,
        auto: true,
        switched: false,
        controls: &[
            Control {
                id: "strength",
                kind: ControlKind::Number {
                    min: 0.0,
                    max: 1.0,
                    step: 0.01,
                    default: 0.0,
                },
                unit: None,
                unported: false,
            },
            Control {
                id: "style",
                kind: ControlKind::Choice {
                    options: &["soft", "hard"],
                    default: 0,
                    unported: &[],
                },
                unit: None,
                unported: false,
            },
            Control {
                id: "preserve",
                kind: ControlKind::Bool { default: true },
                unit: None,
                unported: false,
            },
            Control {
                id: "curve",
                kind: ControlKind::Points { default: &[] },
                unit: None,
                unported: false,
            },
        ],
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Slider(Slider::new("strength")),
                Row::Choice {
                    control: "style",
                    style: ChoiceStyle::List,
                },
                Row::Toggle {
                    control: "preserve",
                },
                Row::Custom {
                    widget: Widget::Curve,
                    controls: &["curve"],
                },
            ],
        }],
        before: &[],
        after: &[],
        presets: &[],
        views: &[],
        pickers: &[],
    };

    impl Module for Look {
        fn definition(&self) -> &'static Definition {
            &LOOK
        }

        fn owns(&self) -> &'static [Owned] {
            &[Owned {
                operation: "fake",
                param: "owned",
            }]
        }

        fn bind(
            &self,
            values: &ModuleValues,
            _: &ImageContext,
            _: &apertrust_modules::Base,
        ) -> Vec<Setting> {
            values
                .number("strength")
                .map(|v| Setting::new("fake", "owned", Value::Float(v)))
                .into_iter()
                .collect()
        }

        fn migrate(&self, from: u32, mut values: ModuleValues) -> ModuleValues {
            if from < 2
                && let Some(amount) = values.remove("amount")
            {
                values.set("strength", amount);
            }
            values
        }
    }

    const OPERATIONS: &[&dyn Operation] = &[&Fake, &Optional];
    const MODULES: &[&dyn Module] = &[&Look];
    /// The build the tests' edits are written by, a development build's version.
    const BUILD: &str = "0.4.0+12~g1a2b3c4";

    fn edited() -> Edit {
        let mut edit = Edit::default();
        edit.set_control(
            &Look,
            "strength",
            ControlValue::Number(0.1 + 0.2),
            &apertrust_modules::testing::image(),
        )
        .unwrap();
        edit.set_control(
            &Look,
            "style",
            ControlValue::Choice(1),
            &apertrust_modules::testing::image(),
        )
        .unwrap();
        edit.set_control(
            &Look,
            "preserve",
            ControlValue::Bool(false),
            &apertrust_modules::testing::image(),
        )
        .unwrap();
        edit.set_control(
            &Look,
            "curve",
            ControlValue::Points(vec![[0.25, 0.2], [0.75, 0.8]]),
            &apertrust_modules::testing::image(),
        )
        .unwrap();
        // Released, keeping its value, so not every set control is kept.
        edit.keep(&Look, "preserve", false).unwrap();
        for (name, value) in [
            ("amount", Value::Float(0.1 + 0.2)),
            ("level", Value::Int(42)),
            ("mode", Value::Choice(1)),
            ("on", Value::Bool(false)),
        ] {
            edit.set_param(OPERATIONS, MODULES, "fake", name, value)
                .unwrap();
        }
        edit
    }

    /// A version 0 file with the fake's entry, with these attributes.
    fn version_0(attributes: &str) -> String {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="{RDF}">
            <rdf:Description rdf:about="" xmlns:apertrust="{NAMESPACE}" xmlns:p="{PARAM_NAMESPACE}" apertrust:version="0">
            <apertrust:operations><rdf:Seq><rdf:li>
            <rdf:Description apertrust:operation="fake" apertrust:schema="2" {attributes}/>
            </rdf:li></rdf:Seq></apertrust:operations></rdf:Description></rdf:RDF></x:xmpmeta>"#
        )
    }

    /// A version 1 file with one module entry, with these attributes.
    fn version_1(attributes: &str) -> String {
        format!(
            r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="{RDF}">
            <rdf:Description rdf:about="" xmlns:apertrust="{NAMESPACE}" xmlns:c="{CONTROL_NAMESPACE}" apertrust:version="1">
            <apertrust:modules><rdf:Seq><rdf:li>
            <rdf:Description {attributes}/>
            </rdf:li></rdf:Seq></apertrust:modules></rdf:Description></rdf:RDF></x:xmpmeta>"#
        )
    }

    #[test]
    fn an_edit_reads_back_exactly() {
        let xml = to_xml(&edited(), OPERATIONS, MODULES, BUILD);
        for written in [
            r#"apertrust:version="1""#,
            r#"apertrust:module="look""#,
            r#"apertrust:moduleVersion="2""#,
            r#"apertrust:kept="strength style curve""#,
            r#"control:style="hard""#,
            r#"control:curve="0.25,0.2 0.75,0.8""#,
            r#"param:mode="hard""#,
        ] {
            assert!(xml.contains(written), "{written} in {xml}");
        }
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.edit, edited());
        assert_eq!(loaded.newer, None);
    }

    #[test]
    fn an_edit_records_the_build_that_wrote_it() {
        let xml = to_xml(&edited(), OPERATIONS, MODULES, BUILD);
        assert!(
            xml.contains(r#"x:xmptk="Apertrust 0.4.0+12~g1a2b3c4""#),
            "{xml}"
        );
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.build.as_deref(), Some(BUILD));
        // A packager's version is written as given, escaped.
        let xml = to_xml(&edited(), OPERATIONS, MODULES, "1.0 \"R&D\"");
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.build.as_deref(), Some("1.0 \"R&D\""));
        // A file that names no Apertrust build, as one another tool rewrote, or an empty
        // one.
        let loaded = from_xml(&version_1(""), OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.build, None);
        let xml = to_xml(&edited(), OPERATIONS, MODULES, " ");
        assert!(xml.contains(r#"x:xmptk="Apertrust  ""#), "{xml}");
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.build, None);
    }

    #[test]
    fn a_curve_with_no_points_between_its_ends_reads_back() {
        let mut edit = Edit::default();
        edit.set_control(
            &Look,
            "curve",
            ControlValue::Points(Vec::new()),
            &apertrust_modules::testing::image(),
        )
        .unwrap();
        let xml = to_xml(&edit, OPERATIONS, MODULES, BUILD);
        assert!(xml.contains(r#"control:curve="""#), "{xml}");
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.edit, edit);
    }

    #[test]
    fn an_empty_edit_is_written_and_reads_back_empty() {
        let xml = to_xml(&Edit::default(), OPERATIONS, MODULES, BUILD);
        assert!(!xml.contains("apertrust:operation="), "{xml}");
        assert!(!xml.contains("apertrust:module="), "{xml}");
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.edit.is_empty());
        assert!(loaded.warnings.is_empty());
    }

    #[test]
    fn a_kept_control_without_a_value_reads_back() {
        let mut edit = Edit::default();
        edit.keep(&Look, "strength", true).unwrap();
        let xml = to_xml(&edit, OPERATIONS, MODULES, BUILD);
        assert!(!xml.contains("control:strength"), "{xml}");
        assert_eq!(from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap().edit, edit);
    }

    #[test]
    fn what_cannot_be_used_is_left_out_or_clamped_with_a_warning() {
        let xml =
            version_0(r#"p:amount="9" p:level="x" p:mode="medium" p:unknown="1" p:on="true""#);
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        let params = &loaded.edit.params["fake"];
        assert_eq!(params.get("amount"), Some(Value::Float(4.0)));
        assert_eq!(params.get("on"), Some(Value::Bool(true)));
        assert_eq!(params.get("level"), None);
        assert_eq!(params.get("mode"), None);
        assert_eq!(loaded.warnings.len(), 4, "{:?}", loaded.warnings);

        let xml = version_1(
            r#"apertrust:module="look" apertrust:moduleVersion="2" apertrust:kept="strength grain"
               c:strength="1.5" c:style="medium" c:grain="1" c:curve="0,0 0.5""#,
        );
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        let values = &loaded.edit.modules["look"];
        assert_eq!(values.number("strength"), Some(1.0));
        assert_eq!(values.mark("strength"), Some(Mark::Kept));
        assert_eq!(values.get("style"), None);
        assert_eq!(values.get("curve"), None);
        // strength clamped; style, grain, curve and the mark on grain left out.
        assert_eq!(loaded.warnings.len(), 5, "{:?}", loaded.warnings);
    }

    #[test]
    fn an_option_not_ported_is_left_out_with_a_warning() {
        use apertrust_modules::testing::Sample;
        let xml = version_1(
            r#"apertrust:module="sample" apertrust:moduleVersion="1" c:method="film" c:size="fine""#,
        );
        let loaded = from_xml(&xml, OPERATIONS, &[&Sample], &[]).unwrap();
        let values = &loaded.edit.modules["sample"];
        assert_eq!(values.get("method"), None);
        assert_eq!(values.choice("size"), Some(0));
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        assert!(
            loaded.warnings[0].contains("film is not ported"),
            "{:?}",
            loaded.warnings
        );
    }

    #[test]
    fn version_0_loses_owned_and_optional_parameters_with_a_warning() {
        let xml = version_0(r#"p:amount="1" p:owned="0.5""#).replace(
            "</rdf:li></rdf:Seq>",
            r#"</rdf:li><rdf:li><rdf:Description apertrust:operation="optional" apertrust:schema="2" p:amount="1"/></rdf:li></rdf:Seq>"#,
        );
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.version, 0);
        assert_eq!(loaded.edit.params.len(), 1);
        let params = &loaded.edit.params["fake"];
        assert_eq!(params.get("amount"), Some(Value::Float(1.0)));
        assert_eq!(params.get("owned"), None);
        assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
        assert!(
            loaded.warnings[0].contains("module look"),
            "{:?}",
            loaded.warnings
        );
        assert!(
            loaded.warnings[1].contains("optional"),
            "{:?}",
            loaded.warnings
        );
        // Saved again, it is version 1.
        let saved = to_xml(&loaded.edit, OPERATIONS, MODULES, BUILD);
        assert!(saved.contains(r#"apertrust:version="1""#));
        assert_eq!(
            from_xml(&saved, OPERATIONS, MODULES, &[]).unwrap().edit,
            loaded.edit
        );
    }

    #[test]
    fn an_older_module_version_is_migrated_and_an_unknown_one_reported() {
        let older = version_1(
            r#"apertrust:module="look" apertrust:moduleVersion="1" apertrust:kept="strength" c:amount="0.25""#,
        );
        let loaded = from_xml(&older, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.edit.modules["look"].number("strength"), Some(0.25));
        assert_eq!(loaded.newer, None);
        let newer = older.replace(r#"moduleVersion="1""#, r#"moduleVersion="3""#);
        let loaded = from_xml(&newer, OPERATIONS, MODULES, &[]).unwrap();
        // Read by name: amount is no control of version 2.
        assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
        assert_eq!(
            loaded.newer,
            Some(Newer::ModuleVersion {
                module: "look".to_owned(),
                version: "3".to_owned(),
                has: 2
            })
        );
        // A version that is no whole number is none this build knows, and is kept as
        // written.
        let unreadable = older.replace(r#"moduleVersion="1""#, r#"moduleVersion="2.0""#);
        let loaded = from_xml(&unreadable, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(
            loaded.newer,
            Some(Newer::ModuleVersion {
                module: "look".to_owned(),
                version: "2.0".to_owned(),
                has: 2
            })
        );
        assert_eq!(loaded.warnings.len(), 2, "{:?}", loaded.warnings);
        let unknown = older.replace(r#"module="look""#, r#"module="lens""#);
        let loaded = from_xml(&unknown, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.edit.is_empty());
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        assert_eq!(loaded.newer, Some(Newer::Module("lens".to_owned())));
    }

    /// A module Apertrust has retired, as the Tone curve's `tone_curve` once RGB curve
    /// replaces it, was written by an older build: left out with a warning, never newer.
    #[test]
    fn a_retired_module_is_left_out_with_a_warning_and_is_not_newer() {
        let xml = version_1(r#"apertrust:module="lens" apertrust:moduleVersion="4" c:amount="1""#)
            .replace(
                "</rdf:Seq></apertrust:modules>",
                r#"<rdf:li><rdf:Description apertrust:module="look" apertrust:moduleVersion="2" c:strength="0.5"/></rdf:li></rdf:Seq></apertrust:modules>"#,
            );
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &["lens"]).unwrap();
        assert_eq!(loaded.newer, None);
        assert!(
            matches!(&loaded.warnings[..], [w] if w.contains("lens is retired")),
            "{:?}",
            loaded.warnings
        );
        assert_eq!(loaded.edit.modules.len(), 1);
        assert_eq!(loaded.edit.modules["look"].number("strength"), Some(0.5));
        // A module the build has wins over a retired id it took again.
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &["look", "lens"]).unwrap();
        assert_eq!(loaded.edit.modules["look"].number("strength"), Some(0.5));
    }

    /// An edit Exposure's version 1 saved, with the Contrast and Pivot it once had: its
    /// Exposure, in exact stops, is left out with a warning, its mark with it, and so are
    /// Contrast and Pivot, a warning each, and the rest reads. Colour balance RGB's
    /// Contrast is not set from them (decision 2: no released edit holds them).
    #[test]
    fn exposure_s_version_1_exposure_contrast_and_pivot_are_left_out_and_the_rest_reads() {
        let (operations, modules) = (apertrust_operations::OPERATIONS, apertrust_modules::MODULES);
        let xml = version_1(
            r#"apertrust:module="exposure" apertrust:moduleVersion="1" apertrust:kept="exposure contrast black_level" c:exposure="0.5" c:contrast="12" c:pivot="25" c:black_level="0.01""#,
        );
        let loaded = from_xml(&xml, operations, modules, &[]).unwrap();
        assert_eq!(loaded.warnings.len(), 4, "{:?}", loaded.warnings);
        let exposure = &loaded.edit.modules["exposure"];
        assert_eq!(exposure.get("exposure"), None);
        assert_eq!(exposure.mark("exposure"), None);
        assert_eq!(exposure.number("black_level"), Some(0.01));
        assert_eq!(exposure.mark("black_level"), Some(Mark::Kept));
        assert_eq!(exposure.get("contrast"), None);
        assert_eq!(loaded.edit.modules.len(), 1);
    }

    #[test]
    fn an_unknown_operation_or_schema_version_is_reported() {
        let other = version_0("").replace(
            r#"apertrust:operation="fake""#,
            r#"apertrust:operation="lens""#,
        );
        let loaded = from_xml(&other, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        let older = version_0(r#"p:amount="1""#)
            .replace(r#"apertrust:schema="2""#, r#"apertrust:schema="1""#);
        let loaded = from_xml(&older, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(
            loaded.edit.params["fake"].get("amount"),
            Some(Value::Float(1.0))
        );
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    }

    /// The file round trip through the text layer, which `apertrust_library::sidecar` tests alone.
    #[test]
    fn an_edit_is_written_to_a_file_and_read_back() {
        let directory =
            std::env::temp_dir().join(format!("apertrust-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = path_for(&directory.join("IMG.ARW"));
        write(&path, &edited(), OPERATIONS, MODULES, BUILD).unwrap();
        let loaded = read(&path, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.edit, edited());
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        // A foreign or unreadable file is refused as the text layer says.
        std::fs::write(&path, "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"/>").unwrap();
        assert!(matches!(
            read(&path, OPERATIONS, MODULES, &[]),
            Err(SidecarError::NotAnEdit)
        ));
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// The fixture `apertrust_library::sidecar` holds byte for byte reads with no warning and the
    /// edit writes back as the same bytes.
    #[test]
    fn the_version_1_fixture_reads_with_no_warning_and_writes_back_byte_for_byte() {
        let fixture = include_str!("../../library/tests/fixtures/version-1.apxmp");
        let loaded = from_xml(fixture, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.edit, edited());
        assert_eq!(loaded.newer, None);
        assert_eq!(to_xml(&loaded.edit, OPERATIONS, MODULES, BUILD), fixture);
    }

    /// Grain, a switched module (decision 32), with the fake's modules.
    const WITH_GRAIN: &[&dyn Module] = &[&Look, &apertrust_modules::grain::Grain];

    /// A switched module on writes `enabled`, with only its values off darktable's
    /// defaults, none at them; one off, only its marks, and nothing at all untouched.
    #[test]
    fn a_switched_module_on_writes_enabled_and_one_off_no_values() {
        use apertrust_modules::grain::Grain;
        let image = apertrust_modules::testing::image();
        let mut edit = Edit::default();
        edit.set_control(&Grain, "strength", ControlValue::Number(25.0), &image)
            .unwrap();
        edit.reset_control(&Grain, "strength").unwrap();
        let xml = to_xml(&edit, OPERATIONS, WITH_GRAIN, BUILD);
        for written in [
            r#"apertrust:module="grain""#,
            r#"apertrust:moduleVersion="2""#,
            r#"apertrust:enabled="true"/>"#,
        ] {
            assert!(xml.contains(written), "{written} in {xml}");
        }
        let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.edit, edit);
        assert!(loaded.edit.is_on(&Grain));

        let mut off = Edit::default();
        off.keep(&Grain, "scale", true).unwrap();
        let xml = to_xml(&off, OPERATIONS, WITH_GRAIN, BUILD);
        assert!(!xml.contains("apertrust:enabled"), "{xml}");
        assert!(!xml.contains("control:"), "{xml}");
        let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
        assert_eq!(loaded.edit, off);
        assert!(!loaded.edit.is_on(&Grain));
        off.reset_module(&Grain);
        let xml = to_xml(&off, OPERATIONS, WITH_GRAIN, BUILD);
        assert!(!xml.contains("grain"), "{xml}");
    }

    /// A version 1 entry of a module that has since become switched migrates on where it
    /// rendered, with its values, and off, its values dropped, where it did not; it reads
    /// with no warning, and writes back as version 2.
    #[test]
    fn a_version_1_entry_of_a_switched_module_reads_on_where_it_rendered() {
        use apertrust_modules::grain::Grain;
        let xml = version_1(
            r#"apertrust:module="grain" apertrust:moduleVersion="1" apertrust:kept="strength" c:strength="37.3" c:scale="800""#,
        );
        let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert!(loaded.edit.is_on(&Grain));
        let values = loaded.edit.module("grain").unwrap();
        assert_eq!(values.number("strength"), Some(37.3));
        assert_eq!(values.number("scale"), Some(800.0));
        let written = to_xml(&loaded.edit, OPERATIONS, WITH_GRAIN, BUILD);
        assert!(
            written.contains(r#"apertrust:moduleVersion="2""#),
            "{written}"
        );
        assert!(written.contains(r#"apertrust:enabled="true""#), "{written}");

        let xml =
            version_1(r#"apertrust:module="grain" apertrust:moduleVersion="1" c:scale="3200""#);
        let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert!(loaded.edit.is_empty());
    }

    /// A switched module off that holds a value off its defaults, which no build writes,
    /// reads as on with a warning; `enabled` on a module this build does not switch is a
    /// newer build's, so the photo opens for viewing only.
    #[test]
    fn enabled_is_repaired_or_read_as_a_newer_build_s() {
        use apertrust_modules::grain::Grain;
        let xml =
            version_1(r#"apertrust:module="grain" apertrust:moduleVersion="2" c:strength="40""#);
        let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
        assert!(loaded.edit.is_on(&Grain));
        assert_eq!(
            loaded.warnings,
            ["grain is off but holds values; read as on"]
        );
        assert_eq!(loaded.newer, None);

        let xml = version_1(
            r#"apertrust:module="look" apertrust:moduleVersion="2" apertrust:enabled="true" c:strength="0.5""#,
        );
        let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
        assert_eq!(loaded.newer, Some(Newer::Enabled("look".to_owned())));
        assert!(loaded.view_only().is_some());
        assert!(!loaded.edit.module("look").unwrap().enabled());
    }

    /// `enabled` holding anything but `true` is a newer build's, which a save would lose:
    /// the photo opens for viewing only, the module read as if it had none.
    #[test]
    fn enabled_other_than_true_is_a_newer_build_s() {
        use apertrust_modules::grain::Grain;
        for text in ["false", "1"] {
            let xml = version_1(&format!(
                r#"apertrust:module="grain" apertrust:moduleVersion="2" apertrust:enabled="{text}""#
            ));
            let loaded = from_xml(&xml, OPERATIONS, WITH_GRAIN, &[]).unwrap();
            assert_eq!(
                loaded.newer,
                Some(Newer::Enabled("grain".to_owned())),
                "{text}"
            );
            assert!(loaded.view_only().is_some(), "{text}");
            assert!(!loaded.edit.is_on(&Grain), "{text}");
            assert_eq!(loaded.warnings.len(), 1, "{text}: {:?}", loaded.warnings);
        }
    }

    // -- Version 2: the log -------------------------------------------------------------------

    /// A clock of the machine `machine`, 8 hex digits, at `when`, with `counter`.
    fn clock(when: u64, counter: u32, machine: &str) -> Clock {
        Clock {
            when,
            counter,
            machine: machine.parse().unwrap(),
        }
    }

    /// 2026-03-14T14:02:11.120Z.
    const PI_DAY: u64 = 1_773_496_931_120;

    /// Look's values in `edit`.
    fn look(edit: &Edit) -> history::Entry {
        history::Entry::Module {
            module: "look".to_owned(),
            values: edit.modules["look"].clone(),
        }
    }

    /// A log of five steps, the end on the fourth: the person's Look; an assistant's batch
    /// of the fake's parameters and Look, in words; a style's Look, a message with an
    /// argument; and a Reset after the end. Each step's clock has a counter of its own, the
    /// vector's another.
    fn logged() -> (Log, Vec<Clock>) {
        let edit = edited();
        let mut softer = edit.clone();
        (softer.set_control(
            &Look,
            "strength",
            ControlValue::Number(0.75),
            &apertrust_modules::testing::image(),
        ))
        .unwrap();
        let step = |id: u64, counter: u32, change| Step {
            id: StepId(0xa0 + id),
            clock: clock(PI_DAY + id * 1_000, counter, "5e1f22c9"),
            source: StepSource::You,
            note: None,
            batch: None,
            change,
        };
        let words = Some(Note::Text("warmer".to_owned()));
        let batch = Some(BatchId(0x0f1e_2d3c_4b5a_6978));
        let fake = history::Entry::Operation {
            operation: "fake".to_owned(),
            params: edit.params["fake"].clone(),
        };
        let steps = vec![
            step(1, 7, Change::Entry(look(&edit))),
            Step {
                source: StepSource::Assistant,
                note: words.clone(),
                batch,
                ..step(2, 2, Change::Entry(fake))
            },
            Step {
                source: StepSource::Assistant,
                note: words,
                batch,
                ..step(3, 3, Change::Entry(look(&softer)))
            },
            Step {
                source: StepSource::Style,
                note: Some(Note::Message {
                    id: "history-style".to_owned(),
                    args: vec![("style".to_owned(), "Film".to_owned())],
                }),
                ..step(4, 4, Change::Entry(look(&edit)))
            },
            Step {
                note: Some(Note::reset()),
                ..step(
                    5,
                    5,
                    Change::Reset(vec![
                        history::Entry::Module {
                            module: "look".to_owned(),
                            values: ModuleValues::new(),
                        },
                        history::Entry::Operation {
                            operation: "fake".to_owned(),
                            params: ParamValues::new(),
                        },
                    ]),
                )
            },
        ];
        let clocks = vec![clock(PI_DAY + 9_000, 9, "5e1f22c9")];
        (Log::from_steps(steps, 4), clocks)
    }

    /// The file of [`logged`]'s log, with the photo's name.
    fn logged_xml() -> String {
        let (log, clocks) = logged();
        let mut file = Edits::new("");
        file.photo = Some("IMG.ARW".to_owned());
        log_xml(&file, &log, &clocks, (OPERATIONS, MODULES), BUILD)
    }

    /// A log written as version 2 reads back as it was, its steps, their ids, clocks,
    /// sources, notes, batch and entries, the end and the clock vector, and writes back as
    /// the same bytes; the edit is its fold up to the end.
    #[test]
    fn a_log_is_written_as_version_2_and_reads_back_exactly() {
        let (log, clocks) = logged();
        let xml = logged_xml();
        for written in [
            r#"apertrust:version="2""#,
            r#"apertrust:photo="IMG.ARW""#,
            r#"apertrust:id="00000000""#,
            r#"apertrust:end="00000000000000a4""#,
            r#"apertrust:source="assistant""#,
            r#"apertrust:batch="0f1e2d3c4b5a6978""#,
            r#"apertrust:noteMessage="history-reset""#,
            r#"arg:style="Film""#,
            r#"control:strength="0.75""#,
            r#"param:mode="hard""#,
        ] {
            assert!(xml.contains(written), "{written} in {xml}");
        }
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
        assert_eq!(loaded.view_only(), None);
        assert_eq!(loaded.version, COPIES_VERSION);
        assert_eq!(loaded.build.as_deref(), Some(BUILD));
        assert_eq!(loaded.log, log);
        assert_eq!(loaded.clocks(), clocks);
        assert_eq!(loaded.edit, edited());
        assert_eq!(loaded.edit, log.fold());
        let again = log_xml(
            &loaded.file,
            &loaded.log,
            loaded.clocks(),
            (OPERATIONS, MODULES),
            BUILD,
        );
        assert_eq!(again, xml);
        // At Original, the end empty.
        let mut original = log.clone();
        original.set_end(0);
        let xml = log_xml(
            &Edits::new(""),
            &original,
            &[],
            (OPERATIONS, MODULES),
            BUILD,
        );
        assert!(xml.contains(r#"apertrust:end="""#), "{xml}");
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!((loaded.log.end(), loaded.edit.is_empty()), (0, true));
    }

    /// A copy the file holds beside the first, and what other tools wrote beside the edit,
    /// as ExifTool's `xmp:Rating`, are written back as they were read: a save never drops
    /// what the file holds.
    #[test]
    fn another_virtual_copy_and_other_tools_properties_are_kept_as_read() {
        use apertrust_library::xmp::{Name, Property};
        let mut file = Edits::parse(&logged_xml(), 0).unwrap();
        let mut other = file.copies[0].clone();
        other.id = "7c0e9a14".to_owned();
        other.number = "1".to_owned();
        other.name = "Warm".to_owned();
        file.copies.push(other.clone());
        let rating = Name::new("xmp", "http://ns.adobe.com/xap/1.0/", "Rating");
        file.other.push(Property::simple(rating, "3"));
        let xml = file.to_xml();
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        let (mut log, clocks) = logged();
        log.set_end(1);
        let written = log_xml(&loaded.file, &log, &clocks, (OPERATIONS, MODULES), BUILD);
        let read = Edits::parse(&written, 0).unwrap();
        assert_eq!(read.copies.len(), 2);
        assert_eq!(read.copies[0].end, "00000000000000a1");
        assert_eq!(read.copies[1], other);
        assert_eq!(read.other, file.other);
        assert!(read.problems.is_empty(), "{:?}", read.problems);
    }

    /// A newer file version is read as this version lays it out, for viewing only.
    #[test]
    fn a_newer_file_version_reads_as_this_one_and_is_newer() {
        let xml = logged_xml().replace(r#"apertrust:version="2""#, r#"apertrust:version="3""#);
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.version, 3);
        assert_eq!(loaded.edit, edited());
        assert_eq!(loaded.newer, Some(Newer::Version(3)));
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        let view_only = loaded.view_only().expect("viewing only");
        assert_eq!(view_only.build.as_deref(), Some(BUILD));
        assert_eq!(
            view_only.to_string(),
            format!(
                "the edit Apertrust {BUILD} wrote is version 3, newer than this build's 2: open for viewing only"
            )
        );
    }

    /// What each [`Problem`] is, by name: a new kind of problem fails to compile here, so it
    /// is added to [`every_problem_opens_the_edit_for_viewing_only`].
    pub(crate) fn kind(problem: &Problem) -> &'static str {
        match problem {
            Problem::StepClock { .. } => "step clock",
            Problem::VectorClock { .. } => "vector clock",
            Problem::TwoNotes { .. } => "two notes",
            Problem::ArgumentsWithoutMessage { .. } => "arguments without a message",
            Problem::NoteArguments { .. } => "note arguments",
            Problem::UnreadableNote { .. } => "unreadable note",
            Problem::StepId { .. } => "step id",
            Problem::Batch { .. } => "batch",
            Problem::Source { .. } => "source",
            Problem::End { .. } => "end",
            Problem::UnknownField { .. } => "unknown field",
            Problem::DuplicateStep { .. } => "duplicate step",
            Problem::ClockPastVector { .. } => "clock past vector",
        }
    }

    /// [`logged_xml`] with each kind of [`Problem`], by its [`kind`].
    fn damaged() -> Vec<(&'static str, String)> {
        let xml = logged_xml();
        let replaced = |from: &str, to: &str| {
            let damaged = xml.replacen(from, to, 1);
            assert_ne!(damaged, xml, "{from}");
            damaged
        };
        // Step a2's note in words as a text with a language, an element of its own.
        let words = r#" apertrust:note="warmer""#;
        let without = xml.replacen(words, "", 1);
        let at = without.find(r#"apertrust:id="00000000000000a2""#).unwrap();
        let entries = at + without[at..].find("<apertrust:").unwrap();
        let qualified = format!(
            r#"{}<apertrust:note xml:lang="en">warmer</apertrust:note>{}"#,
            &without[..entries],
            &without[entries..]
        );
        let message = r#"apertrust:noteMessage="history-style""#;
        vec![
            (
                "step clock",
                replaced(r#"apertrust:counter="7""#, r#"apertrust:counter="x""#),
            ),
            (
                "vector clock",
                replaced(r#"apertrust:counter="9""#, r#"apertrust:counter="x""#),
            ),
            (
                "two notes",
                replaced(message, &format!(r#"apertrust:note="Words" {message}"#)),
            ),
            ("arguments without a message", replaced(message, "")),
            ("note arguments", replaced("arg:style=", "param:style=")),
            ("unreadable note", qualified),
            (
                "step id",
                replaced(
                    r#"apertrust:id="00000000000000a1""#,
                    r#"apertrust:id="00000000000000A1""#,
                ),
            ),
            (
                "batch",
                replaced(
                    r#"apertrust:batch="0f1e2d3c4b5a6978""#,
                    r#"apertrust:batch="0f1e""#,
                ),
            ),
            (
                "source",
                replaced(r#"apertrust:source="style""#, r#"apertrust:source="robot""#),
            ),
            (
                "end",
                replaced(
                    r#"apertrust:end="00000000000000a4""#,
                    r#"apertrust:end="00000000000000ff""#,
                ),
            ),
            (
                "unknown field",
                replaced(
                    r#"apertrust:id="00000000000000a1""#,
                    r#"apertrust:id="00000000000000a1" apertrust:rating="2""#,
                ),
            ),
            (
                "duplicate step",
                replaced(
                    r#"apertrust:id="00000000000000a2""#,
                    r#"apertrust:id="00000000000000a1""#,
                ),
            ),
            (
                "clock past vector",
                replaced(
                    r#"apertrust:when="2026-03-14T14:02:20.120Z""#,
                    r#"apertrust:when="2026-03-14T14:00:00.000Z""#,
                ),
            ),
        ]
    }

    /// Every [`Problem`] the text layer tells opens the edit for viewing only, its reason
    /// that problem, not a newer build's; the rest of the log reads.
    #[test]
    fn every_problem_opens_the_edit_for_viewing_only() {
        let cases = damaged();
        assert_eq!(cases.len(), 13, "one case of each kind of problem");
        for (name, xml) in cases {
            let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
            assert_eq!(loaded.newer, None, "{name}");
            let view_only = loaded.view_only().unwrap_or_else(|| panic!("{name}"));
            let Reason::Unreadable(problem) = &view_only.reason else {
                panic!("{name}: {view_only:?}");
            };
            assert_eq!(kind(problem), name, "{name}: {problem}");
            // A field a save would drop is history this build cannot keep, the rest history
            // it cannot read.
            let said = if name == "unknown field" {
                "cannot keep"
            } else {
                "cannot read"
            };
            assert!(view_only.to_string().contains(said), "{name}");
            assert_eq!(loaded.log.steps().len(), 5, "{name}");
        }
    }

    /// Every entry of every step follows decision 48: a newer module version in an old step
    /// makes the edit view-only, an older one is migrated entry by entry, and a step whose
    /// only entry this build leaves out goes with its place in the end.
    #[test]
    fn every_entry_of_every_step_is_read_as_version_1_s() {
        let xml = logged_xml();
        let first = r#"apertrust:moduleVersion="2""#;
        let newer = xml.replacen(first, r#"apertrust:moduleVersion="3""#, 1);
        let loaded = from_xml(&newer, OPERATIONS, MODULES, &[]).unwrap();
        assert!(
            matches!(&loaded.newer, Some(Newer::ModuleVersion { version, .. }) if version == "3"),
            "{:?}",
            loaded.newer
        );
        assert!(loaded.view_only().is_some());
        // Look's version 1 called strength amount: the first step's entry migrates.
        let older = xml
            .replacen(first, r#"apertrust:moduleVersion="1""#, 1)
            .replacen("control:strength=", "control:amount=", 1);
        let loaded = from_xml(&older, OPERATIONS, MODULES, &[]).unwrap();
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        assert_eq!(loaded.view_only(), None);
        assert_eq!(loaded.log, logged().0);
        // The second step's only entry, an operation this build does not run: the step is
        // left out with a warning, and the end, on the fourth, is now on the third.
        let unknown = xml.replacen(
            r#"apertrust:operation="fake""#,
            r#"apertrust:operation="lens""#,
            1,
        );
        let loaded = from_xml(&unknown, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.view_only(), None);
        assert_eq!(loaded.warnings, ["no operation lens; left out"]);
        let ids: Vec<u64> = loaded.log.steps().iter().map(|step| step.id.0).collect();
        assert_eq!(ids, [0xa1, 0xa3, 0xa4, 0xa5]);
        assert_eq!(loaded.log.end(), 3);
    }

    /// A version 1 file reads as a step of each entry and is written as version 2, which
    /// reads back as the same log.
    #[test]
    fn a_version_1_file_is_written_back_as_version_2() {
        let xml = to_xml(&edited(), OPERATIONS, MODULES, BUILD);
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.log.steps().len(), 2);
        let written = log_xml(
            &loaded.file,
            &loaded.log,
            loaded.clocks(),
            (OPERATIONS, MODULES),
            BUILD,
        );
        assert!(written.contains(r#"apertrust:version="2""#), "{written}");
        let again = from_xml(&written, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(again.log, loaded.log);
        assert_eq!(again.edit, edited());
        assert_eq!(again.clocks(), loaded.clocks());
    }

    /// The environment variable that makes [`a_write_cut_off_midway_never_loses_the_previous_file`]
    /// the writer, killed midway, rather than the test.
    const CRASH_WRITER: &str = "APERTRUST_SIDECAR_CRASH_WRITER";

    /// Two logs of `steps` steps, Look's strength at a different value in each, and the
    /// edit each folds to.
    fn two_logs(steps: usize) -> [(String, Edit); 2] {
        [0.25, 0.75].map(|last| {
            let image = apertrust_modules::testing::image();
            let mut log = Log::default();
            let mut edit = Edit::default();
            for step in 0..steps {
                let before = edit.clone();
                let value = if step + 1 == steps {
                    last
                } else {
                    (step % 100) as f32 / 100.0
                };
                let value = ControlValue::Number(value);
                edit.set_control(&Look, "strength", value, &image).unwrap();
                let changes = Change::between(&before, &edit, &[]);
                let clock = clock(PI_DAY + step as u64, 0, "5e1f22c9");
                log.add_change(changes, StepSource::You, None, clock);
            }
            // Ids of their own, so the child's logs are the parent's, byte for byte.
            let steps = (log.steps().iter().enumerate())
                .map(|(at, step)| Step {
                    id: StepId(at as u64 + 1),
                    ..step.clone()
                })
                .collect();
            let log = Log::from_steps(steps, usize::MAX);
            let xml = log_xml(&Edits::new(""), &log, &[], (OPERATIONS, MODULES), BUILD);
            (xml, edit)
        })
    }

    /// A crash test (`docs/history.md`, Tests): a child process writes two 1,000-step logs
    /// over the sidecar in turn through the safe write and is killed at once, after a
    /// while, many times; after each kill, the sidecar reads whole as one of the two, never
    /// half of one, so a write cut off midway never loses the previous file.
    #[test]
    fn a_write_cut_off_midway_never_loses_the_previous_file() {
        let logs = two_logs(1000);
        // The file beside the sidecar the child makes once it has written a log over it.
        let ready = |path: &Path| path.with_extension("ready");
        if let Some(path) = std::env::var_os(CRASH_WRITER) {
            let path = std::path::PathBuf::from(path);
            // The second log first, as the file holds the first; then each in turn, until
            // it is killed.
            for (turn, (xml, _)) in logs.iter().cycle().skip(1).enumerate() {
                write_xml(&path, xml).unwrap();
                if turn == 0 {
                    std::fs::write(ready(&path), b"").unwrap();
                }
            }
        }
        let folder = std::env::temp_dir().join(format!("apertrust-crash-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = path_for(&folder.join("IMG.ARW"));
        write_xml(&path, &logs[0].0).unwrap();
        let test = "sidecar::tests::a_write_cut_off_midway_never_loses_the_previous_file";
        let mut seen = [false; 2];
        for round in 0..24u64 {
            let _ = std::fs::remove_file(ready(&path));
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([test, "--exact", "--test-threads=1", "--nocapture"])
                .env(CRASH_WRITER, &path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            // Once it has written, which in a debug build takes the longest, then after a
            // while longer each round, so the kills land at different points of a write.
            let began = std::time::Instant::now();
            while !ready(&path).exists() {
                assert!(began.elapsed() < Duration::from_secs(120), "round {round}");
                std::thread::sleep(Duration::from_millis(1));
            }
            std::thread::sleep(Duration::from_millis(round * 3));
            child.kill().unwrap();
            child.wait().unwrap();
            let xml = std::fs::read_to_string(&path).unwrap();
            let loaded = from_xml(&xml, OPERATIONS, MODULES, &[])
                .unwrap_or_else(|error| panic!("round {round}: {error}"));
            let which = (logs.iter())
                .position(|(written, _)| *written == xml)
                .unwrap_or_else(|| panic!("round {round}: the file is neither log"));
            assert_eq!(loaded.edit, logs[which].1, "round {round}");
            seen[which] = true;
        }
        assert_eq!(seen, [true, true], "the kills fell on both logs");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    /// The median of `runs` timings of `work`, in milliseconds.
    fn median_ms(runs: usize, mut work: impl FnMut()) -> f64 {
        let mut times: Vec<f64> = (0..runs)
            .map(|_| {
                let start = std::time::Instant::now();
                work();
                start.elapsed().as_secs_f64() * 1e3
            })
            .collect();
        times.sort_by(f64::total_cmp);
        times[runs / 2]
    }

    /// The 1,000-step measurements (`docs/history.md`, In the sidecar): a log of 1,000
    /// steps of Exposure, each two controls set and kept, read from a file on this machine's
    /// disk and written to it, within 20 ms and 50 ms. A measurement, run on its own:
    /// `cargo test --release -p apertrust-engine a_1000_step_log -- --ignored --nocapture`.
    #[test]
    #[ignore = "a measurement, run on its own in release"]
    fn a_1000_step_log_reads_within_20_ms_and_writes_within_50_ms() {
        use apertrust_modules::exposure::Exposure;
        let (operations, modules) = (apertrust_operations::OPERATIONS, apertrust_modules::MODULES);
        let image = apertrust_modules::testing::image();
        let mut log = Log::default();
        let mut edit = Edit::default();
        let mut clocks = Vec::new();
        let machine = MachineId::current();
        let mut last = None;
        for step in 0..1000 {
            let before = edit.clone();
            let ev = -2.0 + (step % 400) as f32 * 0.01;
            let black = (step % 50) as f32 * 0.001;
            (edit.set_control(&Exposure, "exposure", ControlValue::Number(ev), &image)).unwrap();
            let black = ControlValue::Number(black);
            (edit.set_control(&Exposure, "black_level", black, &image)).unwrap();
            let changes = Change::between(&before, &edit, &[]);
            let clock = Clock::now(last.as_ref(), machine);
            last = Some(clock);
            log.add_change(changes, StepSource::You, None, clock);
            history::advance(&mut clocks, clock);
        }
        let mut file = Edits::new("");
        file.photo = Some("DSCF0001.RAF".to_owned());
        file.fingerprint = Some(apertrust_library::sidecar::fingerprint(1, b"raw", b"raw"));
        let folder = std::env::temp_dir().join(format!("apertrust-1000-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = path_for(&folder.join("DSCF0001.RAF"));
        let xml = log_xml(&file, &log, &clocks, (operations, modules), BUILD);
        let serialise = median_ms(9, || {
            log_xml(&file, &log, &clocks, (operations, modules), BUILD);
        });
        // A write of new bytes each time, as each change makes, through the safe write.
        let mut turn = 0;
        let write = median_ms(9, || {
            turn += 1;
            let build = format!("{BUILD}.{turn}");
            write_log(&path, &file, &log, &clocks, (operations, modules), &build).unwrap();
        });
        let mut read_back = None;
        let read = median_ms(9, || {
            read_back = Some(super::read(&path, operations, modules, &[]).unwrap());
        });
        let loaded = read_back.unwrap();
        assert_eq!(loaded.log.steps().len(), 1000);
        assert_eq!(loaded.edit, edit);
        println!(
            "1,000 steps, {} kB: written in {write:.2} ms ({serialise:.2} ms of it the XML), read in {read:.2} ms",
            xml.len() / 1000
        );
        std::fs::remove_dir_all(&folder).unwrap();
        assert!(read < 20.0, "read in {read:.2} ms, over 20 ms");
        assert!(write < 50.0, "written in {write:.2} ms, over 50 ms");
    }
}
