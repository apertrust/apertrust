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

//! Every kernel must parse and validate in naga, the WGSL front end wgpu uses, so a
//! broken kernel fails here rather than at pipeline creation. A kernel that declares
//! derivatives must export `main` and both derivative entry points, and an
//! operation's manifest must agree with its kernel about them (decision 21).

use apertrust_operations::{KERNELS, OPERATIONS};
use naga::valid::{Capabilities, ValidationFlags, Validator};

#[test]
fn every_operation_kernel_validates() {
    for kernel in KERNELS {
        let (name, wgsl) = (kernel.name, kernel.wgsl);
        let module = naga::front::wgsl::parse_str(wgsl)
            .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(wgsl)));
        Validator::new(ValidationFlags::all(), Capabilities::default())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(wgsl)));

        let has_entry = |wanted: &str| module.entry_points.iter().any(|e| e.name == wanted);
        assert!(!module.entry_points.is_empty(), "{name}: no entry point");
        // A differentiable kernel is per pixel, with `main` and both derivatives; a
        // multi-pass kernel, such as demosaic's, names an entry point per pass.
        if kernel.derivatives {
            for entry in [
                "main".to_owned(),
                format!("{name}_backward"),
                format!("{name}_forward_diff"),
            ] {
                assert!(has_entry(&entry), "{name}: no `{entry}` entry point");
            }
        }
    }
}

#[test]
fn manifests_agree_with_their_kernels_about_derivatives() {
    for operation in OPERATIONS {
        let manifest = operation.manifest();
        assert_eq!(
            manifest.differentiable,
            operation.kernel().derivatives,
            "{}",
            manifest.id
        );
    }
}
