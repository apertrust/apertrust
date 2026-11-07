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
//! an XMP file. Version 1 (decision 28) holds the [`Edit`]: each module it touches, in
//! the modules' order, with its version, the controls set and its marks, then the
//! operation parameters no module owns, only of operations an unedited photo runs, each
//! operation with its schema version as version 0 had them (decision 48).
//!
//! The file's text layer, its XMP, structure, versions and build record, is
//! `apertrust_library::sidecar`'s (decision 54), which knows no module or operation: this module
//! is the typed reading and writing on top of it. It turns a [`Document`]'s entries, which
//! hold their values as text, into an [`Edit`] with the module definitions, and an
//! [`Edit`] into entries. See that crate for the file's layout.
//!
//! Numbers are written in their shortest exact form, choices by option id, points as
//! `x,y` pairs separated by spaces, and the marks (`apertrust:kept`, `apertrust:auto`)
//! as lists of control ids. An untouched control or module is left out.
//!
//! A saved edit always opens (decision 21): a module, control, operation or parameter
//! the engine does not know, or a value it cannot read, is left out, a number outside
//! its range is clamped, an older module version is migrated, and a newer module or
//! another schema version is read by name, each with a warning. A parameter a module
//! owns, or one of an operation an unedited photo does not run, is left out with a
//! warning too. Version 0, which held only operation parameters, still opens that way,
//! and is saved as version 1.
//!
//! The file records the build that wrote it, in `x:xmptk` (decision 44's version, which
//! the binaries hand the engine). An edit a newer Apertrust wrote is told by what it holds
//! ([`Newer`], decision 43): a file version above [`VERSION`], read as this version lays
//! it out, a module version above its definition's or no whole number, or a module this
//! build neither has nor has retired (`apertrust_modules::RETIRED`), whose values a save
//! would lose. Such an edit opens for viewing only ([`ViewOnly`]); a retired module's
//! values are left out with a warning, as an older build wrote them.

use std::io;
use std::path::Path;

use apertrust_modules::{ControlValue, Mark, Module, ModuleValues, check};
use apertrust_plugin::Operation;

pub use apertrust_library::sidecar::{
    CONTROL_NAMESPACE, NAMESPACE, Newer, PARAM_NAMESPACE, SidecarError, VERSION, ViewOnly,
    path_for, write_xml,
};
use apertrust_library::sidecar::{Document, Entry};
pub(crate) use apertrust_library::sidecar::{file_name, writer};

use crate::edit::Edit;

/// An edit read back, the version it was saved as, the build that wrote it, and what
/// could not be used.
pub struct Loaded {
    pub edit: Edit,
    pub version: u32,
    /// The build that wrote it, as its `x:xmptk` records it; `None` for a file that
    /// records no Apertrust build.
    pub build: Option<String>,
    /// The first thing it holds that tells a newer Apertrust wrote it, or `None` where a
    /// save would keep everything it holds.
    pub newer: Option<Newer>,
    pub warnings: Vec<String>,
}

impl Loaded {
    /// The edit as a newer Apertrust's, open for viewing only, or `None` where this build
    /// may save it.
    pub fn view_only(&self) -> Option<ViewOnly> {
        let newer = self.newer.clone()?;
        Some(ViewOnly {
            build: self.build.clone(),
            newer,
        })
    }
}

/// Writes `edit` to `path` as `build` writes it ([`to_xml`], [`write_xml`]).
pub fn write(
    path: &Path,
    edit: &Edit,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    build: &str,
) -> io::Result<()> {
    write_xml(path, &to_xml(edit, operations, modules, build))
}

/// Reads the edit at `path` for `operations` and `modules`, with `retired` the ids of the
/// modules Apertrust has retired ([`from_xml`]), logging what is left out.
pub fn read(
    path: &Path,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Result<Loaded, SidecarError> {
    let loaded = from_document(
        apertrust_library::sidecar::read(path)?,
        operations,
        modules,
        retired,
    );
    for warning in &loaded.warnings {
        tracing::warn!(target: "pipe", "{}: {warning}", path.display());
    }
    Ok(loaded)
}

/// The XMP of an edit as `build` writes it, the build recorded in `x:xmptk`: every module
/// it touches in the order of `modules`, then every operation it sets a parameter of in
/// the order of `operations`.
pub fn to_xml(
    edit: &Edit,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    build: &str,
) -> String {
    let mut document = Document::new(build);
    for module in modules {
        let definition = module.definition();
        let Some(values) = edit.modules.get(definition.id).filter(|v| !v.is_empty()) else {
            continue;
        };
        let marked = |mark| {
            definition
                .controls
                .iter()
                .map(|control| control.id)
                .filter(|id| values.mark(id) == Some(mark))
                .map(str::to_owned)
                .collect()
        };
        document.entries.push(Entry::Module {
            module: definition.id.to_owned(),
            version: Some(definition.version.to_string()),
            kept: marked(Mark::Kept),
            auto: marked(Mark::Auto),
            controls: definition
                .controls
                .iter()
                .filter_map(|control| {
                    let value = values.get(control.id)?;
                    Some((control.id.to_owned(), control.text(value).to_owned()))
                })
                .collect(),
        });
    }
    for operation in operations {
        let manifest = operation.manifest();
        let Some(params) = edit
            .params
            .get(manifest.id)
            .filter(|p| p.iter().next().is_some())
        else {
            continue;
        };
        let schema = operation.schema();
        document.entries.push(Entry::Operation {
            operation: manifest.id.to_owned(),
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
        });
    }
    document.to_xml()
}

/// Reads an edit's XMP for `operations` and `modules`. A module none of them is, but one
/// of `retired`, the ids of the modules Apertrust has retired, is an older build's: left
/// out with a warning, as any unknown module is, but not [`Newer`].
pub fn from_xml(
    xml: &str,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Result<Loaded, SidecarError> {
    Ok(from_document(
        Document::parse(xml)?,
        operations,
        modules,
        retired,
    ))
}

/// The edit a [`Document`]'s entries hold, for `operations` and `modules`.
fn from_document(
    document: Document,
    operations: &[&dyn Operation],
    modules: &[&dyn Module],
    retired: &[&str],
) -> Loaded {
    let mut newer = document.newer();
    let Document {
        version,
        build,
        entries,
    } = document;
    let mut edit = Edit::default();
    let mut warnings = Vec::new();
    if newer.is_some() {
        warnings.push(format!(
            "file version {version}, newer than this build's {VERSION}; read as version {VERSION}"
        ));
    }
    for entry in entries {
        match entry {
            Entry::Module {
                module,
                version,
                kept,
                auto,
                controls,
            } => {
                let found = read_module(
                    &mut edit,
                    &mut warnings,
                    (modules, retired),
                    (&module, version.as_deref()),
                    [(kept, Mark::Kept), (auto, Mark::Auto)],
                    controls,
                );
                newer = newer.or(found);
            }
            Entry::Operation {
                operation,
                schema,
                params,
            } => read_operation(
                &mut edit,
                &mut warnings,
                operations,
                modules,
                (&operation, schema),
                params,
            ),
        }
    }
    Loaded {
        edit,
        version,
        build,
        newer,
        warnings,
    }
}

/// Reads one module's entry into `edit`; what in it tells a newer Apertrust wrote it, if
/// anything does.
fn read_module(
    edit: &mut Edit,
    warnings: &mut Vec<String>,
    (modules, retired): (&[&dyn Module], &[&str]),
    (id, saved): (&str, Option<&str>),
    marks: [(Vec<String>, Mark); 2],
    controls: Vec<(String, String)>,
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
    let mut values = ModuleValues::new();
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
    if !checked.is_empty() {
        edit.modules.insert(definition.id.to_owned(), checked);
    }
    newer
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
mod tests {
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

    /// A newer file version is read as this version lays it out, for viewing only.
    #[test]
    fn a_newer_file_version_reads_as_this_one_and_is_newer() {
        let xml = to_xml(&edited(), OPERATIONS, MODULES, "0.9.0")
            .replace(r#"apertrust:version="1""#, r#"apertrust:version="2""#);
        let loaded = from_xml(&xml, OPERATIONS, MODULES, &[]).unwrap();
        assert_eq!(loaded.version, 2);
        assert_eq!(loaded.edit, edited());
        assert_eq!(loaded.newer, Some(Newer::Version(2)));
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
        let view_only = loaded.view_only().expect("viewing only");
        assert_eq!(view_only.build.as_deref(), Some("0.9.0"));
        assert_eq!(
            view_only.to_string(),
            "the edit Apertrust 0.9.0 wrote is version 2, newer than this build's 1: open for viewing only"
        );
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
}
