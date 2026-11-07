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

//! Raw black/white point's invariants (`docs/modules/raw-black-white-point.md`,
//! Invariants), one test per row, each named after its row: R0 to R3, the rows a
//! first-pass table has (decision 22) for a module over an operation every raw runs, so
//! with no first step, and R4, a white point at or below a black level. The small raw runs
//! on CI's software GPUs, R3 on the CPU. The raw.pixls.us corpus runs locally, with
//! `APERTRUST_RAW_CORPUS` naming it, and R3's corpus row also reads darktable's test raws,
//! with `APERTRUST_TEST_IMAGES` naming them:
//!
//! ```sh
//! cargo test --release -p apertrust-modules --test raw_black_white_point -- --ignored --nocapture
//! ```
//!
//! The module acts on the raw stage, so every row renders from a mosaic, the small raw's
//! or a corpus raw's, never a synthetic image in the working space, and reads the frame,
//! colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Raw black/white point's rows read neither the synthetic images nor a jump"
)]
mod common;
mod corpus;

use std::path::PathBuf;

use apertrust_engine::edit::{self, Edit};
use apertrust_engine::pipeline::{self, Instance, Limits, Pipe};
use apertrust_modules::raw_black_white_point::{BLACK, RawBlackWhitePoint, WHITE};
use apertrust_modules::{Base, ControlValue, MODULES, Module};
use apertrust_operations::OPERATIONS;
use apertrust_operations::rawprepare::Rawprepare;
use apertrust_plugin::{
    Camera, Format, ImageContext, ImageDesc, Operation, ParamValues, Pattern, Stage,
};

use common::{Bench, exact, id, small_raw};
use corpus::{CORPUS_VIEWPORT, NINETY_D, corpus_bench, decoded, each_corpus_raw, export_bench};

const RAWPREPARE: &str = "rawprepare";

/// The levels, the four blacks then the white point.
const LEVELS: [&str; 5] = [BLACK[0], BLACK[1], BLACK[2], BLACK[3], WHITE];

/// What every row reads: the frame, colorout's output, from the mosaic on.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, _: &mut Vec<Instance<'static>>, synthetic: bool) {
        assert!(!synthetic, "the raw stage needs a mosaic");
    }
}

/// An edit with these controls set on `image`, in this order.
fn edit_of(set: &[(&str, f32)], image: &ImageContext) -> Edit {
    let mut edit = Edit::default();
    for &(control, value) in set {
        edit.set_control(
            &RawBlackWhitePoint,
            control,
            ControlValue::Number(value),
            image,
        )
        .unwrap();
    }
    edit
}

/// A level's default for the photo.
fn default(image: &ImageContext, level: &str) -> f32 {
    RawBlackWhitePoint.defaults(image).number(level)
}

/// `moves`, each a level and how far it moves from the photo's default, as values.
fn moved(image: &ImageContext, moves: &[(&'static str, f32)]) -> Vec<(&'static str, f32)> {
    moves
        .iter()
        .map(|&(level, by)| (level, default(image, level) + by))
        .collect()
}

impl Bench {
    fn set(&mut self, set: &[(&str, f32)]) -> common::Rendered {
        let edit = edit_of(set, &self.image);
        self.edit(&edit, Frame)
    }
}

// R0: every control at its default.

/// Edits that leave every control at its default for the photo: each written at it, and
/// each kept there.
fn untouched(image: &ImageContext) -> Vec<Edit> {
    let defaults = RawBlackWhitePoint.defaults(image);
    let mut written = Edit::default();
    let mut kept = Edit::default();
    for control in RawBlackWhitePoint.definition().controls {
        let value = defaults.get(control.id).unwrap().clone();
        written
            .set_control(&RawBlackWhitePoint, control.id, value, image)
            .unwrap();
        kept.keep(&RawBlackWhitePoint, control.id, true).unwrap();
    }
    vec![written, kept]
}

/// Whether the module's binding sets nothing for `edit` on the bench's photo.
fn binds_nothing(edit: &Edit, image: &ImageContext) -> bool {
    let values = edit.module(RAWPREPARE).unwrap();
    let defaults = RawBlackWhitePoint.defaults(image);
    let base = Base::new(OPERATIONS, image);
    RawBlackWhitePoint
        .bind(&values.off_defaults(&defaults), image, &base)
        .is_empty()
}

/// The operation's defaults for the photo, every parameter resolved over its schema.
fn operation_defaults(image: &ImageContext) -> ParamValues {
    Rawprepare
        .schema()
        .resolve_over(&Rawprepare.defaults(image), &ParamValues::new())
        .unwrap()
}

/// Whether each of the module's defaults is the operation's for the photo.
fn defaults_are_the_operation_s(image: &ImageContext) -> bool {
    let ours = RawBlackWhitePoint.defaults(image);
    let theirs = operation_defaults(image);
    LEVELS
        .iter()
        .all(|level| ours.number(level) as i64 == theirs.int(level))
        && ours.choice("flat_field") == theirs.choice("flat_field")
}

/// R0 on one pipeline, as the session renders: no edit, then a black level one step off
/// its default, then each untouched edit, every frame as the first.
fn r0_holds(bench: &mut Bench) {
    assert!(defaults_are_the_operation_s(&bench.image));
    let none = bench.edit(&Edit::default(), Frame);
    let step = moved(&bench.image, &[(BLACK[0], 1.0)]);
    assert!(
        !exact(&none, &bench.set(&step)),
        "{step:?} changes the frame"
    );
    for edit in untouched(&bench.image) {
        assert!(binds_nothing(&edit, &bench.image));
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "R0 on {:?}",
            bench.pipe
        );
    }
}

#[test]
fn r0_every_control_at_its_default_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        r0_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
}

/// R0 on every corpus raw: the module's defaults are the operation's, and the module at
/// them, written or kept, plans the very instances and cache keys no edit does, so the
/// same frame.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn r0_on_the_corpus() {
    let mut raws = 0;
    each_corpus_raw(|name, image, input| {
        assert!(defaults_are_the_operation_s(&image), "R0 on {name}");
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
            assert!(binds_nothing(&edit, &image), "R0 on {name}");
            assert_eq!(keys(&bench.instances(&edit, Frame)), none, "R0 on {name}");
        }
        raws += 1;
    });
    println!("R0 holds on {raws} raws");
}

// R1: each control moved, then reset.

/// Moves from the photo's defaults: each level by one step, Black level 0 and 3 up and the
/// white point down; every black by 16 together; the white point down by a twentieth of
/// its value; and every level at once. Each changes the frame.
fn moves(image: &ImageContext) -> Vec<Vec<(&'static str, f32)>> {
    let white = default(image, WHITE);
    [
        &[(BLACK[0], 1.0)][..],
        &[(BLACK[3], 1.0)],
        &[(WHITE, -1.0)],
        &[
            (BLACK[0], 16.0),
            (BLACK[1], 16.0),
            (BLACK[2], 16.0),
            (BLACK[3], 16.0),
        ],
        &[(WHITE, -(white / 20.0).round())],
        &[
            (BLACK[0], 4.0),
            (BLACK[1], 8.0),
            (BLACK[2], 12.0),
            (BLACK[3], 2.0),
            (WHITE, -(white / 10.0).round()),
        ],
    ]
    .iter()
    .map(|set| moved(image, set))
    .collect()
}

/// Each move, then reset by `ResetControl` (a double-click or Delete) and by
/// `ResetModule` (the header's reset), on the bench's one pipeline, which restarts at the
/// raw stage with each.
fn r1_holds(bench: &mut Bench) {
    let none = bench.edit(&Edit::default(), Frame);
    for set in moves(&bench.image) {
        let mut edit = edit_of(&set, &bench.image);
        let moved = bench.edit(&edit, Frame);
        assert!(!exact(&none, &moved), "{set:?} changes the frame");
        for (control, _) in &set {
            edit.reset_control(&RawBlackWhitePoint, control).unwrap();
        }
        assert_eq!(edit, Edit::default());
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, reset control by control"
        );
        let mut edit = edit_of(&set, &bench.image);
        bench.edit(&edit, Frame);
        edit.reset_module(&RawBlackWhitePoint);
        assert!(
            exact(&none, &bench.edit(&edit, Frame)),
            "{set:?}, the module reset"
        );
    }
}

#[test]
fn r1_a_control_moved_and_reset_renders_as_no_edit() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        r1_holds(&mut Bench::mosaic(&mosaic, image, pipe));
    }
}

// R2: the order controls were set in.

/// Pairs of moves from the photo's defaults: a black level and the white point, and two
/// black levels.
const PAIRS: [[(&str, f32); 2]; 2] = [
    [(BLACK[0], 8.0), (WHITE, -100.0)],
    [(BLACK[3], 4.0), (BLACK[1], 2.0)],
];

/// One pipeline sets `first`, then both; another sets `second`, then both: the two
/// frames are the same, byte for byte.
fn r2_holds(new: impl Fn() -> Bench, [first, second]: [(&'static str, f32); 2]) {
    let mut one = new();
    let [first, second] = [first, second].map(|(level, by)| moved(&one.image, &[(level, by)])[0]);
    one.set(&[first]);
    let one = one.set(&[first, second]);
    let mut other = new();
    other.set(&[second]);
    let other = other.set(&[second, first]);
    assert!(exact(&one, &other), "R2 with {first:?} and {second:?}");
}

#[test]
fn r2_controls_render_the_same_whichever_came_first() {
    let (mosaic, image) = small_raw(-0.67);
    for pair in PAIRS {
        r2_holds(|| Bench::mosaic(&mosaic, image.clone(), Pipe::Export), pair);
    }
}

/// R1 and R2 on the 90D, through the viewport's frame and the export's.
#[test]
#[ignore = "needs the raw.pixls.us corpus (APERTRUST_RAW_CORPUS)"]
fn r1_and_r2_on_the_corpus() {
    for pipe in [CORPUS_VIEWPORT, Pipe::Export] {
        r1_holds(&mut corpus_bench(NINETY_D, pipe));
        for pair in PAIRS {
            r2_holds(|| corpus_bench(NINETY_D, pipe), pair);
        }
    }
}

// R3: darktable's own levels, from `apertrust-golden raw-defaults`.

/// A raw whose levels darktable chooses otherwise than Apertrust: Apertrust's,
/// rawler's, and darktable's, as `apertrust-golden raw-defaults` recovers them from
/// darktable's render, each the four blacks by parity position and the white point
/// (`docs/ports/rawprepare.md`, Results, where each difference has its verdict).
struct Levels {
    /// The raw, under the corpus `<Make>/<Model>/<file>`, or darktable's test raw's name.
    raw: &'static str,
    ours: ([u16; 4], u16),
    darktable: ([u16; 4], u16),
}

/// Every raw on which `raw-defaults` finds a level of darktable's that is not Apertrust's,
/// as it prints them, but the Pixel 4a, whose difference is darktable's embedded gain maps,
/// which no level reproduces.
const DARKTABLE: [Levels; 7] = [
    Levels {
        raw: "mire1.cr2",
        ours: ([1023; 4], 13600),
        darktable: ([1030, 1030, 1026, 1026], 13600),
    },
    Levels {
        raw: "Canon/EOS M50/Canon_EOS_M50_CRAW_ISO_100_nocrop_nodual.CR3",
        ours: ([510, 511, 511, 511], 11892),
        darktable: ([510, 510, 511, 511], 11892),
    },
    Levels {
        raw: "Canon/EOS R6 Mark II/canon-r6m2-img_0346-raw-normal-full.cr3",
        ours: ([512; 4], 12735),
        darktable: ([511; 4], 14008),
    },
    Levels {
        raw: "Nikon/Z 9/Nikon_-_NIKON_Z_9_-_14bit_compressed_(Lossless).NEF",
        ours: ([1008; 4], 15520),
        darktable: ([1008; 4], 15892),
    },
    Levels {
        raw: "OM System/OM-1 Mark II/20mp-12bit.orf",
        ours: ([255, 254, 254, 255], 4095),
        darktable: ([255, 254, 254, 255], 4096),
    },
    Levels {
        raw: "Pentax/K-3 Mark III/IMGP9854.PEF",
        ours: ([256; 4], 16375),
        darktable: ([256; 4], 16315),
    },
    Levels {
        raw: "Samsung/NX1/2016-07-23-142101_sam_9364.srw",
        ours: ([0; 4], 16100),
        darktable: ([128; 4], 16100),
    },
];

/// A raw's context with Apertrust's levels for one of [`DARKTABLE`]'s raws, as a 2×2
/// black pattern over a small sensor with a crop.
fn context(levels: &Levels) -> ImageContext {
    let (black, white) = levels.ours;
    let mut image = small_raw(0.0).1;
    let camera: &mut Camera = image.camera.as_mut().unwrap();
    camera.black = Pattern {
        width: 2,
        height: 2,
        values: black.map(f32::from).to_vec(),
    };
    camera.white = f32::from(white);
    image
}

/// R3: the module's controls set to darktable's own levels for a raw bind them exactly:
/// the parameters the edit renders are darktable's history entry for its defaults with
/// Apertrust's crop, bit for bit, as `raw-run` writes one, and the kernel's inputs are
/// darktable's `commit_params`, black and white less black for each parity position. So
/// darktable's default render of each raw, up to its crop, is one edit away, through a
/// kernel `raw-run` holds to darktable's within 7.2e-7 at any parameters it is given.
#[test]
fn r3_darktable_s_levels_bind_darktable_s_parameters() {
    for levels in &DARKTABLE {
        let image = context(levels);
        let (black, white) = levels.ours;
        let defaults: Vec<f32> = LEVELS.iter().map(|level| default(&image, level)).collect();
        let ours: Vec<f32> = black
            .iter()
            .chain([&white])
            .map(|&l| f32::from(l))
            .collect();
        assert_eq!(defaults, ours, "{}: the defaults", levels.raw);

        let (black, white) = levels.darktable;
        let set: Vec<(&str, f32)> = LEVELS
            .into_iter()
            .zip(black.iter().chain([&white]))
            .map(|(level, &value)| (level, f32::from(value)))
            .collect();
        let instances = edit::instances(&edit_of(&set, &image), OPERATIONS, MODULES, &image)
            .all()
            .unwrap();
        let instance = instances.iter().find(|i| id(i) == RAWPREPARE).unwrap();
        let params = Rawprepare
            .schema()
            .resolve_over(&Rawprepare.defaults(&image), &instance.params)
            .unwrap();
        let margins = ["left", "top", "right", "bottom"].map(|m| params.int(m) as u32);
        let crop = operation_defaults(&image);
        assert_eq!(
            margins,
            ["left", "top", "right", "bottom"].map(|m| crop.int(m) as u32),
            "{}: the crop is the raw's",
            levels.raw
        );
        assert_eq!(
            golden::raw_stage::rawprepare_from(&params),
            golden::raw_stage::rawprepare_params(margins, black, white),
            "{}: darktable's history entry",
            levels.raw
        );

        let camera = image.camera.as_ref().unwrap();
        let sensor = ImageDesc::new(camera.width, camera.height, Format::Mosaic16);
        let prepared = Rawprepare.prepare(&params, &image, &sensor).unwrap();
        let bytes = &prepared.passes[0].params;
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let sub = [16, 20, 24, 28].map(float);
        let div = [32, 36, 40, 44].map(float);
        let theirs = black.map(f32::from);
        assert_eq!(sub, theirs, "{}: black", levels.raw);
        assert_eq!(
            div,
            theirs.map(|b| f32::from(white) - b),
            "{}: white less black",
            levels.raw
        );
    }
}

/// R3's table against the decoder: on each raw it names, the module's defaults are the
/// levels it gives as Apertrust's, which `raw-defaults` printed. darktable's test raws
/// come from `APERTRUST_TEST_IMAGES`, the corpus's from `APERTRUST_RAW_CORPUS`.
#[test]
#[ignore = "needs the raw.pixls.us corpus and darktable's test raws"]
fn r3_the_table_is_the_decoder_s() {
    let dir = |variable: &str| PathBuf::from(std::env::var_os(variable).expect(variable));
    for levels in &DARKTABLE {
        let path = if levels.raw.contains('/') {
            dir("APERTRUST_RAW_CORPUS").join(levels.raw)
        } else {
            dir("APERTRUST_TEST_IMAGES").join(levels.raw)
        };
        let (_, image, _) = decoded(&path);
        let (black, white) = levels.ours;
        let ours: Vec<f32> = black
            .iter()
            .chain([&white])
            .map(|&l| f32::from(l))
            .collect();
        let defaults: Vec<f32> = LEVELS.iter().map(|level| default(&image, level)).collect();
        assert_eq!(defaults, ours, "{}", levels.raw);
        println!("{}: {defaults:?}", levels.raw);
    }
}

// R4: a white point at or below a black level.

/// What R4 reads: the raw stage's output, demosaic's, in the camera's RGB, where an
/// infinity or NaN from rawprepare would still show; the frame's clamps would hide it.
#[derive(Debug, Clone, Copy)]
struct RawStage;

impl common::Measure for RawStage {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        assert!(!synthetic, "the raw stage needs a mosaic");
        instances.retain(|i| i.operation.manifest().slot.stage == Stage::Raw);
    }
}

/// R4: the white point moved to a black level, and below one, gives the raw stage finite
/// values through the export and the viewport, as the operation keeps each divisor at one
/// level (`docs/ports/rawprepare.md`, Deliberate differences), where darktable's would be
/// 0 or negative.
#[test]
fn r4_a_white_point_at_or_below_a_black_level_gives_finite_values() {
    for pipe in [
        Pipe::Export,
        Pipe::Viewport {
            width: 40,
            height: 30,
        },
    ] {
        let (mosaic, image) = small_raw(-0.67);
        let mut bench = Bench::mosaic(&mosaic, image, pipe);
        for set in [
            &[(WHITE, 0.0)][..],
            &[(BLACK[0], 100.0), (WHITE, 100.0)],
            &[(BLACK[0], 100.0), (BLACK[3], 200.0), (WHITE, 50.0)],
        ] {
            let edit = edit_of(set, &bench.image);
            let rendered = bench.edit(&edit, RawStage);
            assert!(
                rendered.pixels.iter().flatten().all(|v| v.is_finite()),
                "R4 with {set:?} on {pipe:?}"
            );
        }
    }
}
