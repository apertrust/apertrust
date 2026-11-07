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

//! Highlight reconstruction's invariants (`docs/modules/highlight-reconstruction.md`,
//! Invariants), one test per row, each named after its row: K0 to K3, the rows a
//! first-pass table has (decision 22) for a module over an operation every raw runs, so
//! with no first step. The small raw, which clips in a corner, runs on CI's software GPUs,
//! K3 on the CPU. The raw.pixls.us corpus runs locally, with `APERTRUST_RAW_CORPUS`
//! naming it:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test highlight_reconstruction -- --ignored --nocapture
//! ```
//!
//! The module acts on the raw stage, so every row renders from a mosaic, the small raw's
//! or a corpus raw's, never a synthetic image in the working space, and reads the frame,
//! colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Highlight reconstruction's rows read neither the synthetic images nor a jump"
)]
mod common;
mod corpus;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::highlight_reconstruction::{CLIP, HighlightReconstruction, threshold};
use apertrust_modules::raw_black_white_point::{RawBlackWhitePoint, WHITE};
use apertrust_modules::{Base, ControlValue, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::highlights::Highlights;
use apertrust_plugin::{Format, ImageContext, ImageDesc, Operation, ParamValues};

use common::{Bench, exact, id, small_raw};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, each_corpus_raw, export_bench};

const HIGHLIGHTS: &str = "highlights";

/// What every row reads: the frame, colorout's output, from the mosaic on.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, _: &mut Vec<Instance<'static>>, synthetic: bool) {
        assert!(!synthetic, "the raw stage needs a mosaic");
    }
}

/// An edit with these controls set on `image`, in this order: each a module and a
/// control of it.
fn edit_of(set: &[(&dyn Module, &str, f32)], image: &ImageContext) -> Edit {
    let mut edit = Edit::default();
    for &(module, control, value) in set {
        edit.set_control(module, control, ControlValue::Number(value), image)
            .unwrap();
    }
    edit
}

/// An edit setting only the clipping threshold, on `image`.
fn clip(value: f32, image: &ImageContext) -> Edit {
    edit_of(&[(&HighlightReconstruction, CLIP, value)], image)
}

// K0: every control at its default.

/// Edits that leave every control at its default for the photo: each written at it, and
/// each kept there.
fn untouched(image: &ImageContext) -> Vec<Edit> {
    let defaults = HighlightReconstruction.defaults(image);
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in HighlightReconstruction.definition().controls {
        let value = defaults.get(control.id).unwrap().clone();
        written
            .set_control(&HighlightReconstruction, control.id, value, image)
            .unwrap();
        kept.keep(&HighlightReconstruction, control.id, true)
            .unwrap();
    }
    vec![written, kept]
}

/// Whether the module's binding sets nothing for `edit` on the photo.
fn binds_nothing(edit: &Edit, image: &ImageContext) -> bool {
    let values = edit.module(HIGHLIGHTS).unwrap();
    let defaults = HighlightReconstruction.defaults(image);
    let base = Base::new(OPERATIONS, image);
    HighlightReconstruction
        .bind(&values.off_defaults(&defaults), image, &base)
        .is_empty()
}

/// The operation's defaults for the photo, every parameter resolved over its schema.
fn operation_defaults(image: &ImageContext) -> ParamValues {
    Highlights
        .schema()
        .resolve_over(&Highlights.defaults(image), &ParamValues::new())
        .unwrap()
}

/// Whether the module's defaults are the operation's for the photo: the threshold bit for
/// bit, and inpaint opposed.
fn defaults_are_the_operation_s(image: &ImageContext) -> bool {
    let ours = HighlightReconstruction.defaults(image);
    let theirs = operation_defaults(image);
    ours.number(CLIP).to_bits() == theirs.float(CLIP).to_bits()
        && ours.choice("mode") == 0
        && apertrust_operations::highlights::METHODS[theirs.choice("mode") as usize] == "opposed"
}

/// K0 on one pipeline, as the session renders: no edit, then the threshold one step off
/// its default, then each untouched edit, every frame as the first.
fn k0_holds(bench: &mut Bench) {
    assert!(defaults_are_the_operation_s(&bench.image));
    let none = bench.edit(&Edit::default(), Frame);
    let step = threshold(&bench.image) - 0.01;
    let stepped = clip(step, &bench.image);
    assert!(
        !exact(&none, &bench.edit(&stepped, Frame)),
        "{step} changes the frame"
    );
    for edit in untouched(&bench.image) {
        assert!(binds_nothing(&edit, &bench.image));
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "K0 on {:?}",
            bench.pipe
        );
    }
}

/// The pipes the small raw runs through: the export, and a viewport at about half its
/// size.
const PIPES: [Pipe; 2] = [
    Pipe::Export,
    Pipe::Viewport {
        width: 40,
        height: 30,
    },
];

#[test]
fn k0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in PIPES {
        let (mosaic, image) = small_raw(-0.67);
        k0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
}

/// K0 on every corpus raw: the module's defaults are the operation's, and the module at
/// them, written or kept, plans the very instances and cache keys no edit does, so the
/// same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn k0_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, input| {
        assert!(defaults_are_the_operation_s(&image), "K0 on {name}");
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
        for edit in untouched(&image) {
            assert!(binds_nothing(&edit, &image), "K0 on {name}");
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "K0 on {name}");
        }
        raws += 1;
    });
    println!("K0 holds on {raws} raws");
}

// K1: Clipping threshold moved and reset.

/// Moves of the threshold from the photo's default: one step down and one up, to 0.5,
/// where far more photosites count as clipped, and to 2, where none does. Each changes
/// the frame.
const MOVES: [f32; 4] = [-0.01, 0.01, -0.5, 1.0];

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline, which restarts at the
/// raw stage with each.
fn k1_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    for by in MOVES {
        let value = threshold(&bench.image) + by;
        let mut edit = clip(value, &bench.image);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{value} changes the frame");
        edit.reset_control(&HighlightReconstruction, CLIP).unwrap();
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{value}, reset the control"
        );
        let mut edit = clip(value, &bench.image);
        bench.edit(&edit, Frame);
        edit.reset_module(&HighlightReconstruction);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{value}, the module reset"
        );
    }
}

#[test]
fn k1_clipping_threshold_moved_and_reset_renders_as_no_edit() {
    for pipe in PIPES {
        let (mosaic, image) = small_raw(-0.67);
        k1_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
}

// K2: the order controls were set in.

/// Two orders of the same values, from the photo's defaults: the threshold and Raw
/// black/white point's White point, either first; and the threshold set through another
/// value first, against set at once, with the white point. The frames are the same,
/// byte for byte.
fn k2_holds(new: impl Fn() -> Bench) {
    let mut bench = new();
    let (threshold, white) = (
        threshold(&bench.image) - 0.05,
        RawBlackWhitePoint.defaults(&bench.image).number(WHITE) - 100.0,
    );
    let threshold_first = [
        (&HighlightReconstruction as &dyn Module, CLIP, threshold),
        (&RawBlackWhitePoint, WHITE, white),
    ];
    let white_first = [threshold_first[1], threshold_first[0]];
    let image = bench.image.clone();
    bench.edit(&edit_of(&threshold_first[..1], &image), Frame);
    let one = bench.edit(&edit_of(&threshold_first, &image), Frame);
    let mut other = new();
    other.edit(&edit_of(&white_first[..1], &image), Frame);
    let other = other.edit(&edit_of(&white_first, &image), Frame);
    assert!(exact(&one, &other), "K2: the threshold and the white point");
    let mut through = new();
    through.edit(&clip(threshold + 0.3, &image), Frame);
    let through = through.edit(
        &edit_of(
            &[
                (&HighlightReconstruction, CLIP, threshold + 0.3),
                (&RawBlackWhitePoint, WHITE, white),
                (&HighlightReconstruction, CLIP, threshold),
            ],
            &image,
        ),
        Frame,
    );
    assert!(
        exact(&one, &through),
        "K2: the threshold through another value"
    );
}

#[test]
fn k2_controls_render_the_same_whichever_came_first() {
    let (mosaic, image) = small_raw(-0.67);
    k2_holds(|| Bench::mosaic(&mosaic, image.clone(), Pipe::Export));
}

/// K1 and K2 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn k1_and_k2_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        k1_holds(&mut corpus_bench(NINETY_D, pipe));
        k2_holds(|| corpus_bench(NINETY_D, pipe));
    }
}

// K3: darktable's parameters.

/// darktable's `DT_IOP_HIGHLIGHTS_OPPOSED`, inpaint opposed's value in its enum.
const DT_OPPOSED: i32 = 5;

/// darktable's `highlights_clip_magics` for inpaint opposed.
const OPPOSED_MAGIC: f32 = 0.987;

/// K3: the threshold set to each value binds darktable's parameters: the edit renders
/// inpaint opposed, darktable's value 5, and the threshold bit for bit, so the history
/// entry `apertrust-golden raw-run` writes for it holds them at darktable's offsets, and
/// the kernel's clipping levels are darktable's `_process_opposed`'s, `0.987f × clip`
/// times each colour's white balance coefficient, in f32. Over a photo with no linear
/// response limit and one whose limit is 0.9, the default is darktable's `reload_defaults`
/// threshold, the lower of 1 and the limit.
#[test]
fn k3_the_threshold_binds_darktable_s_parameters() {
    for limit in [1.0, 0.9] {
        let mut image = small_raw(0.0).1;
        image.camera.as_mut().unwrap().linear_response_limit = limit;
        let default = HighlightReconstruction.defaults(&image).number(CLIP);
        assert_eq!(default.to_bits(), 1.0f32.min(limit).to_bits());
        for value in [0.0, 0.5, 0.9, 0.987, 1.0, 1.3, 2.0] {
            let edit = clip(value, &image);
            let instances = edit::instances(&edit, OPERATIONS, MODULES, &image)
                .all()
                .unwrap();
            let instance = instances.iter().find(|i| id(i) == HIGHLIGHTS).unwrap();
            let params = Highlights
                .schema()
                .resolve_over(&Highlights.defaults(&image), &instance.params)
                .unwrap();
            let (mode, threshold) = (params.choice("mode") as i32, params.float(CLIP));
            assert_eq!(
                (mode, threshold.to_bits()),
                (DT_OPPOSED, value.to_bits()),
                "{value}"
            );

            let stage = golden::raw_stage::RawStage {
                highlights: Some((mode, threshold)),
                ..golden::raw_stage::RawStage::passthrough(None, Some([1.0; 3]), false)
            };
            let xmp = golden::raw_stage::passthrough_xmp(&stage).unwrap();
            let written = golden::xmp::module_params(&xmp, HIGHLIGHTS).unwrap();
            let word = |at: usize| -> [u8; 4] { written.bytes[at..at + 4].try_into().unwrap() };
            assert_eq!(written.version, 4);
            assert_eq!(word(0), DT_OPPOSED.to_le_bytes(), "{value}: the method");
            assert_eq!(word(16), value.to_le_bytes(), "{value}: the threshold");

            let white_balance = [2.0, 1.0, 1.5];
            let input = ImageDesc {
                white_balance,
                maximum: white_balance,
                ..ImageDesc::new(84, 62, Format::Mosaic)
            };
            let prepared = Highlights.prepare(&params, &image, &input).unwrap();
            let bytes = &prepared.passes[0].params;
            let level = |c: usize| {
                let at = 32 + 4 * c;
                f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
            };
            for (c, coefficient) in white_balance.into_iter().enumerate() {
                let darktable = (OPPOSED_MAGIC * value) * coefficient;
                assert_eq!(
                    level(c).to_bits(),
                    darktable.to_bits(),
                    "{value}: colour {c}'s clipping level"
                );
            }
        }
    }
}

/// K3 on the corpus: every raw's default threshold is darktable's `reload_defaults`
/// one, the lower of 1 and its linear response limit, which the image context carries
/// from a DNG's LinearResponseLimit; each raw that records a limit is printed.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn k3_on_the_corpus() {
    let (mut raws, mut recorded) = (0, 0);
    each_corpus_raw(|name, image, _| {
        let limit = image.camera.as_ref().unwrap().linear_response_limit;
        let default = HighlightReconstruction.defaults(&image).number(CLIP);
        assert_eq!(default.to_bits(), 1.0f32.min(limit).to_bits(), "{name}");
        if limit != 1.0 {
            recorded += 1;
            println!("{name}: linear response limit {limit}, threshold {default}");
        }
        raws += 1;
    });
    println!("K3: the threshold is darktable's on {raws} raws, {recorded} with a limit below 1");
}
