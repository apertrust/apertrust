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
//! it; and [`Rows`], the bodies of rows 0 to 3 that each such tool's table has the same
//! way, which its tests call under their own names. Each switched tool's test file
//! includes it with `mod switched;`.

use std::path::PathBuf;

use apertrust_engine::compute::GpuImage;
use apertrust_engine::edit::{self, Edit};
use apertrust_engine::export::{BUDGET, Tiling, export};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_engine::sidecar;
use apertrust_operations::OPERATIONS;
use apertrust_plugin::{ImageContext, ParamValues};
use apertrust_tools::{ControlValue, ProcessingTool, TOOLS, check};

use crate::common::{
    self, BEFORE_THE_WORKING_SPACE, Bench, Jump, Picture, Rendered, exact, gpu, id, jump,
    same_pixels, small_raw, sweep,
};
use crate::corpus::{
    CORPUS_VIEWPORT, NINETY_D, corpus_bench, decoded, each_corpus_raw, export_bench,
};

/// Edits that leave the switched `tool` off: none, and every control kept at its
/// default, which never turns it on.
pub fn off(tool: &dyn ProcessingTool) -> Vec<Edit> {
    let mut kept = Edit::default();
    for control in tool.definition().controls {
        kept.keep(tool, control.id, true).unwrap();
    }
    assert!(!kept.is_on(tool));
    vec![Edit::default(), kept]
}

/// The tool on at darktable's defaults: its first control written at its default, as
/// a click that leaves a value where it was turns darktable's module on.
pub fn on_at_defaults(tool: &dyn ProcessingTool, image: &ImageContext) -> Edit {
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
pub fn on_at_defaults_holds(tool: &dyn ProcessingTool, operation: &str) {
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
        <apertrust:tools><rdf:Seq><rdf:li>
        <rdf:Description apertrust:tool="{tool}" apertrust:toolVersion="1"{attributes}/>
        </rdf:li></rdf:Seq></apertrust:tools></rdf:Description></rdf:RDF></x:xmpmeta>"#,
        sidecar::NAMESPACE,
        sidecar::CONTROL_NAMESPACE,
    );
    let loaded = sidecar::from_xml(&xml, OPERATIONS, TOOLS, &[]).unwrap();
    assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    assert_eq!(loaded.newer, None);
    loaded.edit
}

/// What every row reads: the frame, colorout's output, from the working space on for a
/// synthetic image.
#[derive(Debug, Clone, Copy)]
pub struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

/// A switched tool's rows 0 to 3, as every such tool's table has them: 0, off renders as
/// no edit; 1, the first step, on darktable's chart and the corpus (each tool measures
/// it on the ramp and the sweep itself); 2, a control moved and reset; 3, the order
/// controls were set in.
pub struct Rows {
    pub tool: &'static dyn ProcessingTool,
    /// The letter the tool's rows are named by, `"P"` for RGB primaries'.
    pub letter: &'static str,
}

impl Rows {
    /// An edit with these controls set, in this order: a number as itself, a toggle on
    /// unless 0, a choice by its option's index.
    pub fn edit_of(&self, set: &[(&str, f32)]) -> Edit {
        let mut edit = Edit::default();
        for &(control, value) in set {
            let value = match self.control(control).default() {
                ControlValue::Bool(_) => ControlValue::Bool(value != 0.0),
                ControlValue::Choice(_) => ControlValue::Choice(value as u32),
                _ => ControlValue::Number(value),
            };
            edit.set_control(self.tool, control, value, &common::photo())
                .unwrap();
        }
        edit
    }

    fn control(&self, id: &str) -> &'static apertrust_tools::Control {
        self.tool
            .definition()
            .controls
            .iter()
            .find(|control| control.id == id)
            .unwrap_or_else(|| panic!("{id} is none of the tool's controls"))
    }

    /// The tool on at darktable's defaults: its operation planned at its defaults,
    /// darktable's new instance.
    pub fn on_at_defaults(&self, bench: &Bench) -> Vec<Instance<'static>> {
        bench.instances(&on_at_defaults(self.tool, &bench.image), Frame)
    }

    /// The bench's frame with these controls set ([`Rows::edit_of`]).
    pub fn set(&self, bench: &mut Bench, set: &[(&str, f32)]) -> Rendered {
        bench.edit(&self.edit_of(set), Frame)
    }

    /// Row 0 on one pipeline, as the session renders: no edit, then the tool's operation
    /// joining the plan, on at darktable's defaults, and leaving it, then each edit that
    /// leaves the tool off, every frame as the first.
    fn off_holds(&self, bench: &mut Bench) {
        let none = bench.edit(&Edit::default(), Frame);
        let joined = self.on_at_defaults(bench);
        bench.render(&joined);
        for edit in off(self.tool) {
            assert!(
                exact(&none, &bench.edit(&edit, Frame)),
                "{}0 on {:?}",
                self.letter,
                bench.pipe
            );
        }
    }

    /// Row 0 on a small raw through the export and the viewport, and on `ramp`, the
    /// tool's ramp, and the sweep through the export.
    pub fn off_renders_as_no_edit(&self, ramp: (Picture, ImageContext)) {
        for pipe in [
            Pipe::Export,
            Pipe::Viewport {
                width: 40,
                height: 30,
            },
        ] {
            let (mosaic, image) = small_raw(-0.67);
            self.off_holds(&mut Bench::mosaic(&mosaic, image, pipe));
        }
        for (picture, image) in [ramp, sweep(-0.67)] {
            self.off_holds(&mut Bench::picture(&picture, image, Pipe::Export));
        }
    }

    /// Row 0 on every corpus raw: the tool off, untouched or kept, plans the very
    /// instances and cache keys no edit does, so the same frame.
    pub fn off_on_the_corpus(&self) {
        let mut raws = 0;
        each_corpus_raw(|name, image, input| {
            let bench = export_bench(image.clone(), input.clone());
            let keys = |instances: &[Instance]| -> Vec<(&str, u64)> {
                pipeline::plan(instances, &image, input.desc(), Pipe::Export, &Limits::NONE)
                    .unwrap()
                    .operations
                    .iter()
                    .map(|op| (op.id, op.key))
                    .collect()
            };
            let none = keys(&bench.instances(&Edit::default(), Frame));
            for edit in off(self.tool) {
                assert_eq!(
                    keys(&bench.instances(&edit, Frame)),
                    none,
                    "{}0 on {name}",
                    self.letter
                );
            }
            raws += 1;
        });
        println!("{}0 holds on {raws} raws", self.letter);
    }

    /// The first step's jump on an export of `image`, tiled when the raw does not fit.
    fn first_step_on(&self, image: &ImageContext, input: &GpuImage) -> Jump {
        let export_of = |instances: &[Instance]| {
            let tiling = Tiling::Auto { budget: BUDGET };
            pollster::block_on(export(gpu(), instances, image, input.clone(), tiling))
                .unwrap()
                .pixels
        };
        let bench = export_bench(image.clone(), input.clone());
        let before = export_of(&bench.instances(&Edit::default(), Frame));
        let after = export_of(&self.on_at_defaults(&bench));
        jump(&before, &after, |_| true)
    }

    /// Row 1 on darktable's chart, `mire1.cr2`.
    pub fn first_step_on_the_chart(&self) {
        let dir = PathBuf::from(
            std::env::var_os("APERTRUST_TEST_IMAGES")
                .expect("APERTRUST_TEST_IMAGES names darktable's integration images"),
        );
        let (_, image, input) = decoded(&dir.join("mire1.cr2"));
        let step = self.first_step_on(&image, &input);
        println!("{}1 on the chart: {step}", self.letter);
        assert!(step.mean.is_finite() && step.max.is_finite(), "{step}");
    }

    /// Row 1 on every corpus raw the pipeline takes.
    pub fn first_step_on_the_corpus(&self) {
        let mut jumps = Vec::new();
        each_corpus_raw(|name, image, input| {
            let step = self.first_step_on(&image, &input);
            println!("{name}: {step}");
            assert!(
                step.mean.is_finite() && step.max.is_finite(),
                "{name}: {step}"
            );
            jumps.push(step);
        });
        let worst_mean = jumps.iter().map(|j| j.mean).fold(0.0, f64::max);
        let worst_max = jumps.iter().map(|j| j.max).fold(0.0, f64::max);
        let worst_share = jumps.iter().map(Jump::share_over_one).fold(0.0, f64::max);
        println!(
            "{}1 on {} raws: mean at most {worst_mean:.4}, max at most {worst_max:.3}, at most {:.3}% over 1.0",
            self.letter,
            jumps.len(),
            100.0 * worst_share
        );
    }

    /// Row 2: each of `moves` changes the frame's pixels; reset by `ResetControl` (a
    /// double-click or Delete), control by control, it leaves the tool on at darktable's
    /// defaults, and by `ResetTool` (the header's reset), which turns it off, as no
    /// edit; on the bench's one pipeline, which the tool's operation joins with each move
    /// and leaves with each tool's reset.
    fn moved_and_reset_holds(&self, bench: &mut Bench, moves: &[&[(&str, f32)]]) {
        let none = bench.edit(&Edit::default(), Frame);
        let on = self.on_at_defaults(bench);
        let on = bench.render(&on);
        for set in moves {
            let mut edit = self.edit_of(set);
            let moved = bench.edit(&edit, Frame);
            assert!(!same_pixels(&none, &moved), "{set:?} changes the frame");
            for (control, _) in *set {
                edit.reset_control(self.tool, control).unwrap();
            }
            assert!(edit.is_on(self.tool), "{set:?}, reset control by control");
            assert!(
                exact(&on, &bench.edit(&edit, Frame)),
                "{set:?}, reset control by control"
            );
            let mut edit = self.edit_of(set);
            bench.edit(&edit, Frame);
            edit.reset_tool(self.tool);
            assert_eq!(edit, Edit::default());
            assert!(
                exact(&none, &bench.edit(&edit, Frame)),
                "{set:?}, the tool reset"
            );
        }
    }

    /// Row 2 on the sweep through the export and a small raw through the viewport.
    pub fn moved_and_reset(&self, moves: &[&[(&str, f32)]]) {
        let (picture, image) = sweep(-0.67);
        self.moved_and_reset_holds(&mut Bench::picture(&picture, image, Pipe::Export), moves);
        let (mosaic, image) = small_raw(-0.67);
        let viewport = Pipe::Viewport {
            width: 40,
            height: 30,
        };
        self.moved_and_reset_holds(&mut Bench::mosaic(&mosaic, image, viewport), moves);
    }

    /// Row 3: one pipeline sets `first`, then both; another sets `second`, then both: the
    /// two frames are the same, byte for byte.
    fn order_holds(&self, new: impl Fn() -> Bench, [first, second]: [(&str, f32); 2]) {
        let mut one = new();
        self.set(&mut one, &[first]);
        let one = self.set(&mut one, &[first, second]);
        let mut other = new();
        self.set(&mut other, &[second]);
        let other = self.set(&mut other, &[second, first]);
        assert!(
            exact(&one, &other),
            "{}3 with {first:?} and {second:?}",
            self.letter
        );
    }

    /// Row 3 for each of `pairs` on `picture` through the export.
    pub fn order(&self, pairs: &[[(&str, f32); 2]], (picture, image): (Picture, ImageContext)) {
        for &pair in pairs {
            self.order_holds(
                || Bench::picture(&picture, image.clone(), Pipe::Export),
                pair,
            );
        }
    }

    /// Rows 2 and 3 on the 90D, through the viewport's frame and the export's.
    pub fn moved_and_order_on_the_corpus(
        &self,
        moves: &[&[(&str, f32)]],
        pairs: &[[(&str, f32); 2]],
    ) {
        for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
            self.moved_and_reset_holds(&mut corpus_bench(NINETY_D, pipe), moves);
            for &pair in pairs {
                self.order_holds(|| corpus_bench(NINETY_D, pipe), pair);
            }
        }
    }
}
