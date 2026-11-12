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

//! The engine's own kernels must parse and validate in naga, the WGSL front end wgpu
//! uses, so a broken kernel fails here rather than at pipeline creation. No kernel, the
//! engine's or an operation's, may write a local at an index known only at run time,
//! which Direct3D 12's compiler fails or may fail to compile (decision 10).

use apertrust_engine::pipeline;
use apertrust_plugin::Kernel;
use naga::valid::{Capabilities, FunctionInfo, ModuleInfo, ValidationFlags, Validator};
use naga::{AddressSpace, Block, Expression, Function, Handle, Module, Statement, TypeInner};

#[test]
fn every_engine_kernel_validates() {
    for kernel in engine_kernels() {
        let (module, _) = validate(kernel);
        // `main`, but for the pickers' reduction, whose two passes are named for what
        // they do (docs/pickers.md, The kernel).
        let entry_points: &[&str] = match kernel.name {
            "picker" => &["picker_rgba", "picker_combine"],
            _ => &["main"],
        };
        for entry_point in entry_points {
            assert!(
                module.entry_points.iter().any(|e| e.name == *entry_point),
                "{}: no `{entry_point}` entry point",
                kernel.name
            );
        }
    }
}

/// FXC, the Direct3D 12 shader compiler the engine pins, cannot write a function-local
/// vector component at an index known only at run time: it tries to unroll the loop
/// around the write, and naga's loop bounding stops it. Local array elements, and vector
/// components in workgroup memory, are refused too until tested on Windows. Only CI's Windows job runs FXC, so this finds the
/// construct in naga's IR on every machine. `add_at` and `set_at` in
/// `crates/plugin/slang/components.slang` write a component without indexing.
#[test]
fn no_kernel_writes_a_local_at_a_run_time_index() {
    let kernels = apertrust_operations::KERNELS.iter().chain(engine_kernels());
    let mut found = Vec::new();
    for kernel in kernels {
        let (module, info) = validate(kernel);
        found.extend(
            writes_at_run_time_index(&module, &info)
                .into_iter()
                .map(|function| format!("{}: {function}", kernel.name)),
        );
    }
    assert!(
        found.is_empty(),
        "these write a local at a run-time index, which FXC fails or may fail to compile: {found:#?}"
    );
}

#[test]
fn the_check_finds_a_run_time_index_and_allows_the_rest() {
    let flagged = |wgsl: &'static str| {
        let (module, info) = validate(&Kernel {
            name: "case",
            wgsl,
            derivatives: false,
        });
        writes_at_run_time_index(&module, &info)
    };
    // A local vector written at a loop's index, what FXC fails on.
    assert_eq!(
        flagged(
            "fn f(c: u32) -> f32 { var v = vec3<f32>(0.0); for (var i = 0u; i < 3u; i++) { v[i] = 1.0; } return v[c]; }"
        ),
        ["f"]
    );
    // The same inside an if, its else, a switch's case and a loop's continuing block.
    assert_eq!(
        flagged(
            "fn f(c: u32) -> f32 { var v = vec3<f32>(0.0); if (c < 3u) { v[c] = 1.0; } return v.x; }"
        ),
        ["f"]
    );
    assert_eq!(
        flagged(
            "fn f(c: u32) -> f32 { var v = vec3<f32>(0.0); if (c < 3u) {} else { v[c] = 1.0; } return v.x; }"
        ),
        ["f"]
    );
    assert_eq!(
        flagged(
            "fn f(c: u32) -> f32 { var v = vec3<f32>(0.0); var i = 0u; loop { if (i >= 3u) { break; } continuing { v[i] = 1.0; i += 1u; } } return v[c]; }"
        ),
        ["f"]
    );
    assert_eq!(
        flagged(
            "fn f(c: u32) -> f32 { var v = vec3<f32>(0.0); switch (c) { case 1u: { v[c] = 1.0; } default: {} } return v.x; }"
        ),
        ["f"]
    );
    // The same through a pointer argument; passing the pointer is no write.
    assert_eq!(
        flagged(
            "fn f(v: ptr<function, vec3<f32>>, c: u32) { (*v)[c] = 1.0; }
             fn g(c: u32) -> f32 { var v = vec3<f32>(0.0); f(&v, c); return v.x; }"
        ),
        ["f"]
    );
    // A pointer to a local array's element at a run-time index, passed to a function
    // that writes through it.
    assert_eq!(
        flagged(
            "fn b(p: ptr<function, f32>) { *p = 1.0; }
             fn g(c: u32) { var a: array<f32, 4>; b(&a[c]); }"
        ),
        ["g"]
    );
    // A private array.
    assert_eq!(
        flagged("var<private> a: array<f32, 4>; fn f(c: u32) { a[c] = 1.0; }"),
        ["f"]
    );
    // A vector component in workgroup memory, which FXC may not compile either.
    assert_eq!(
        flagged("var<workgroup> w: array<vec4<f32>, 4>; fn f(c: u32) { w[0][c] = 1.0; }"),
        ["f"]
    );
    // Allowed: an array element in workgroup memory, and anything in a storage buffer,
    // which naga writes at a computed offset.
    assert!(
        flagged("var<workgroup> w: array<vec4<f32>, 4>; fn f(c: u32) { w[c] = vec4<f32>(1.0); }")
            .is_empty()
    );
    assert!(
        flagged(
            "@group(0) @binding(0) var<storage, read_write> b: array<vec4<f32>>; fn f(c: u32) { b[c][c] = 1.0; }"
        )
        .is_empty()
    );
    // Allowed: a write at a constant index, and reads at a run-time index. naga turns
    // `v[1]` into an `AccessIndex`; a `let` keeps the index an expression of its own.
    assert!(
        flagged(
            "fn f(c: u32) -> f32 { var v = vec3<f32>(0.0); let k = 1u; v[k] = 2.0; v += vec3<f32>(select(0.0, 1.0, c == 0u)); return v[c]; }"
        )
        .is_empty()
    );
    // Allowed: a by-value argument read at a run-time index and passed to a function.
    assert!(
        flagged(
            "fn h(x: f32) -> f32 { return x; } fn g(v: vec3<f32>, c: u32) -> f32 { return h(v[c]); }"
        )
        .is_empty()
    );
}

/// The engine's own kernels, which belong to no operation.
fn engine_kernels() -> Vec<&'static Kernel> {
    pipeline::KERNELS.iter().collect()
}

/// A kernel's module and naga's analysis of it, or a panic with naga's message.
fn validate(kernel: &Kernel) -> (Module, ModuleInfo) {
    let module = naga::front::wgsl::parse_str(kernel.wgsl)
        .unwrap_or_else(|e| panic!("{}: {}", kernel.name, e.emit_to_string(kernel.wgsl)));
    let info = Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}: {}", kernel.name, e.emit_to_string(kernel.wgsl)));
    (module, info)
}

/// The functions of a module, entry points included, that write at a run-time index
/// where FXC fails or may fail: a vector component of a function-local or private
/// variable (CI's failures), an array element of one, or a vector component in
/// workgroup memory (both untested), directly or through a pointer argument.
fn writes_at_run_time_index(module: &Module, info: &ModuleInfo) -> Vec<String> {
    let functions = module
        .functions
        .iter()
        .map(|(handle, function)| (function, &info[handle]));
    let entry_points = module
        .entry_points
        .iter()
        .enumerate()
        .map(|(i, entry)| (&entry.function, info.get_entry_point(i)));
    functions
        .chain(entry_points)
        .filter(|&(function, info)| {
            let body = Body {
                module,
                function,
                info,
            };
            let mut pointers = Vec::new();
            body.pointers_written(&function.body, &mut pointers);
            pointers
                .into_iter()
                .any(|pointer| body.writes_at_run_time_index(pointer))
        })
        .map(|(function, _)| function.name.clone().unwrap_or_default())
        .collect()
}

/// A function, with naga's analysis of it for its expressions' types.
struct Body<'a> {
    module: &'a Module,
    function: &'a Function,
    info: &'a FunctionInfo,
}

impl Body<'_> {
    fn ty(&self, expression: Handle<Expression>) -> &TypeInner {
        self.info[expression].ty.inner_with(&self.module.types)
    }

    fn is_pointer(&self, expression: Handle<Expression>) -> bool {
        matches!(
            self.ty(expression),
            TypeInner::Pointer { .. } | TypeInner::ValuePointer { .. }
        )
    }

    /// Every pointer a block stores through, or passes to a function that may.
    fn pointers_written(&self, block: &Block, pointers: &mut Vec<Handle<Expression>>) {
        for statement in block.iter() {
            match statement {
                Statement::Store { pointer, .. } => pointers.push(*pointer),
                Statement::Call { arguments, .. } => pointers.extend(
                    arguments
                        .iter()
                        .copied()
                        .filter(|&argument| self.is_pointer(argument)),
                ),
                Statement::Block(block) => self.pointers_written(block, pointers),
                Statement::If { accept, reject, .. } => {
                    self.pointers_written(accept, pointers);
                    self.pointers_written(reject, pointers);
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        self.pointers_written(&case.body, pointers);
                    }
                }
                Statement::Loop {
                    body, continuing, ..
                } => {
                    self.pointers_written(body, pointers);
                    self.pointers_written(continuing, pointers);
                }
                _ => {}
            }
        }
    }

    /// Whether a write through `pointer` indexes, with an index that is not a constant,
    /// what FXC may not compile in the memory it reaches: an array element or a vector
    /// component in function-local or private memory, a vector component in workgroup
    /// memory. Storage buffers take anything.
    fn writes_at_run_time_index(&self, pointer: Handle<Expression>) -> bool {
        let mut pointer = pointer;
        let (mut element, mut component) = (false, false);
        loop {
            match self.function.expressions[pointer] {
                Expression::Access { base, index } => {
                    let constant = matches!(
                        self.function.expressions[index],
                        Expression::Literal(_) | Expression::Constant(_)
                    );
                    if !constant {
                        if self.indexes_array(base) {
                            element = true;
                        } else {
                            component = true;
                        }
                    }
                    pointer = base;
                }
                Expression::AccessIndex { base, .. } => pointer = base,
                Expression::LocalVariable(_) => return element || component,
                Expression::GlobalVariable(global) => {
                    let space = self.module.global_variables[global].space;
                    return unsupported(space, element, component);
                }
                Expression::FunctionArgument(_) => {
                    return match *self.ty(pointer) {
                        TypeInner::Pointer { space, .. } => unsupported(space, element, component),
                        _ => false,
                    };
                }
                _ => return false,
            }
        }
    }

    /// Whether `pointer` points to an array, rather than a vector or a matrix.
    fn indexes_array(&self, pointer: Handle<Expression>) -> bool {
        match *self.ty(pointer) {
            TypeInner::Pointer { base, .. } => matches!(
                self.module.types[base].inner,
                TypeInner::Array { .. } | TypeInner::BindingArray { .. }
            ),
            _ => false,
        }
    }
}

/// Whether FXC fails, or may fail, on a write at a run-time index into an array element
/// or a vector component in `space`. Workgroup arrays are addressable, and storage
/// buffers are written at a computed offset.
fn unsupported(space: AddressSpace, element: bool, component: bool) -> bool {
    match space {
        AddressSpace::Function | AddressSpace::Private => element || component,
        AddressSpace::WorkGroup => component,
        _ => false,
    }
}
