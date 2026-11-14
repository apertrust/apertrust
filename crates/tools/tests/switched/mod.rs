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

//! What the switched tools' rows share (decision 32; `docs/tools/README.md`, Rows
//! every table has): the tool off, kept or untouched, on at darktable's defaults, and
//! an edit its version 1 saved, read through the engine's sidecar reader as a photo opens
//! it. Each switched tool's test file includes it with `mod switched;`.

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::sidecar;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, ParamValues};
use apertrust_tools::{TOOLS, Tool, check};

/// Edits that leave the switched `tool` off: none, and every control kept at its
/// default, which never turns it on.
pub fn off(tool: &dyn Tool) -> Vec<Edit> {
    let mut kept = Edit::default();
    for control in tool.definition().controls {
        kept.keep(tool, control.id, true).unwrap();
    }
    assert!(!kept.is_on(tool));
    vec![Edit::default(), kept]
}

/// The tool on at darktable's defaults: its first control written at its default, as
/// a click that leaves a value where it was turns darktable's module on.
pub fn on_at_defaults(tool: &dyn Tool, image: &ImageContext) -> Edit {
    let control = &tool.definition().controls[0];
    let mut edit = Edit::default();
    edit.set_control(tool, control.id, control.default(), image)
        .unwrap();
    assert!(edit.is_on(tool));
    edit
}

/// The parameters the edit binds to `operation` on `image`, `None` where it is not
/// planned.
pub fn bound(edit: &Edit, operation: &str, image: &ImageContext) -> Option<ParamValues> {
    edit::instances(edit, OPERATIONS, TOOLS, image)
        .all()
        .unwrap()
        .into_iter()
        .find(|instance| instance.operation.manifest().id == operation)
        .map(|instance| instance.params)
}

/// The row on at darktable's defaults, on the CPU: the tool's defaults, each made
/// explicit, bind its operations' defaults for each photo (`check::switched_defaults`),
/// and on at its defaults it plans `operation` with no parameter set, at the operation's
/// own defaults, darktable's new instance.
pub fn on_at_defaults_holds(tool: &dyn Tool, operation: &str) {
    for image in [crate::common::photo(), ImageContext::full(8, 8)] {
        let problems = check::switched_defaults(tool, OPERATIONS, &image);
        assert!(problems.is_empty(), "{problems:?}");
        let on = on_at_defaults(tool, &image);
        let params = bound(&on, operation, &image).expect("planned while on");
        assert!(params.iter().next().is_none(), "{params:?}");
        for edit in off(tool) {
            assert!(bound(&edit, operation, &image).is_none());
        }
    }
}

/// An edit version 1 saved with `tool`'s entry holding `controls` as their text, read
/// as a photo opens it, with no warning.
pub fn version_1(tool: &str, controls: &[(&str, &str)]) -> Edit {
    let attributes: String = controls
        .iter()
        .map(|(control, text)| format!(r#" c:{control}="{text}""#))
        .collect();
    let xml = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description rdf:about="" xmlns:apertrust="{}" xmlns:c="{}" apertrust:version="1">
        <apertrust:modules><rdf:Seq><rdf:li>
        <rdf:Description apertrust:module="{tool}" apertrust:moduleVersion="1"{attributes}/>
        </rdf:li></rdf:Seq></apertrust:modules></rdf:Description></rdf:RDF></x:xmpmeta>"#,
        sidecar::NAMESPACE,
        sidecar::CONTROL_NAMESPACE,
    );
    let loaded = sidecar::from_xml(&xml, OPERATIONS, TOOLS, &[]).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.newer, None);
    loaded.edit
}
