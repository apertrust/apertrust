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

//! Orientation's invariants (`docs/modules/orientation.md`, Invariants), one test per
//! row, each named after its row: F0 to F2, F4 and F5. F3, each option written as the
//! operation's choice, and F6, darktable's buttons, are unit tests in
//! `crates/modules/src/orientation.rs`. The small
//! raw and the synthetic sweep run on CI's software GPUs. Every row that renders reads
//! the frame, colorout's output before it is rounded to 8 bits.

#[expect(
    dead_code,
    reason = "Orientation's rows read neither the ramps nor a jump"
)]
mod common;

use apertrust_engine::edit::Edit;
use apertrust_engine::pipeline::{Instance, Pipe};
use apertrust_modules::orientation::{ORIENTATIONS, Orientation};
use apertrust_modules::{Base, ControlValue, Module, ModuleValues};
use apertrust_operations::OPERATIONS;

use common::{BEFORE_THE_WORKING_SPACE, Bench, exact, id, small_raw, sweep};

const ORIENTATION: &str = "orientation";

/// The frame, from exposure on for a synthetic image.
#[derive(Debug, Clone, Copy)]
struct Frame;

impl common::Measure for Frame {
    fn keep(self, instances: &mut Vec<Instance<'static>>, synthetic: bool) {
        instances.retain(|i| !(synthetic && BEFORE_THE_WORKING_SPACE.contains(&id(i))));
    }
}

const VIEWPORT: Pipe = Pipe::Viewport {
    width: 40,
    height: 30,
};

/// The small images on a bench through `pipe`: the small Bayer raw from its mosaic, and
/// the synthetic sweep in the working space.
fn benches(pipe: Pipe) -> Vec<Bench> {
    let (mosaic, image) = small_raw(-0.67);
    let (picture, swept) = sweep(-0.67);
    vec![
        Bench::mosaic(&mosaic, image, pipe),
        Bench::picture(&picture, swept, pipe),
    ]
}

fn chosen(option: &str) -> ControlValue {
    let index = ORIENTATIONS.iter().position(|&o| o == option).unwrap();
    ControlValue::Choice(index as u32)
}

/// darktable's flip bits of the option at `index`, darktable's value `index - 1`.
fn bits(index: usize) -> u32 {
    index as u32 - 1
}

/// darktable's `dt_imageio_flip_buffers` on a frame `width` by `height`: the columns
/// flipped (2), the rows (1), then the axes swapped (4).
fn flipped(pixels: &[[f32; 4]], [width, height]: [u32; 2], orientation: u32) -> Vec<[f32; 4]> {
    let swap = orientation & 4 != 0;
    let out_width = if swap { height } else { width };
    let mut out = vec![[0.0; 4]; pixels.len()];
    for j in 0..height {
        for i in 0..width {
            let x = if orientation & 2 != 0 {
                width - 1 - i
            } else {
                i
            };
            let y = if orientation & 1 != 0 {
                height - 1 - j
            } else {
                j
            };
            let (x, y) = if swap { (y, x) } else { (x, y) };
            out[(y * out_width + x) as usize] = pixels[(j * width + i) as usize];
        }
    }
    out
}

fn same_pixels(a: &[[f32; 4]], b: &[[f32; 4]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
}

/// F0: auto, written or kept, binds nothing and renders as no edit, its plan and keys
/// too, through the export and the viewport.
#[test]
fn f0_at_its_default_it_renders_as_no_edit() {
    for pipe in [Pipe::Export, VIEWPORT] {
        for mut bench in benches(pipe) {
            let none = bench.edit(&Edit::default(), Frame);
            let base = Base::new(OPERATIONS, &bench.image);
            let mut written = Edit::default();
            written
                .set_control(&Orientation, ORIENTATION, chosen("auto"), &bench.image)
                .unwrap();
            let mut kept = Edit::default();
            kept.keep(&Orientation, ORIENTATION, true).unwrap();
            for edit in [written, kept] {
                let values = ModuleValues::new();
                assert!(Orientation.bind(&values, &bench.image, &base).is_empty());
                assert!(exact(&bench.edit(&edit, Frame), &none), "{pipe:?}");
            }
        }
    }
}

/// F1: each orientation, set and then reset. In the export, the frame is the untouched
/// frame with darktable's buffer flip, bit for bit, every operation after orientation per
/// pixel; in the viewport it differs from the untouched frame. Reset by `ResetControl` or
/// `ResetModule`, it renders as no edit, through both. The session's tests hold the
/// viewport's frame upright, its sides swapped by a quarter turn.
#[test]
fn f1_each_orientation_turns_the_frame_and_its_reset_renders_as_no_edit() {
    for pipe in [Pipe::Export, VIEWPORT] {
        for mut bench in benches(pipe) {
            let none = bench.edit(&Edit::default(), Frame);
            let extent = [bench.image.width, bench.image.height];
            for (index, option) in ORIENTATIONS.iter().enumerate().skip(2) {
                let mut edit = Edit::default();
                edit.set_control(&Orientation, ORIENTATION, chosen(option), &bench.image)
                    .unwrap();
                let turned = bench.edit(&edit, Frame);
                assert!(!exact(&turned, &none), "{option}");
                if pipe == Pipe::Export {
                    let flip = flipped(&none.pixels, extent, bits(index));
                    assert!(same_pixels(&turned.pixels, &flip), "{option}");
                }
                let mut reset = edit.clone();
                reset.reset_control(&Orientation, ORIENTATION).unwrap();
                assert!(exact(&bench.edit(&reset, Frame), &none), "{option}");
                edit.reset_module(&Orientation);
                assert!(exact(&bench.edit(&edit, Frame), &none), "{option}");
            }
            // No rotation renders as a photo the right way up, auto's.
            let mut edit = Edit::default();
            edit.set_control(&Orientation, ORIENTATION, chosen("none"), &bench.image)
                .unwrap();
            assert!(same_pixels(&bench.edit(&edit, Frame).pixels, &none.pixels));
        }
    }
}

/// F2: auto on a raw its EXIF orientation turns, binding nothing, renders as that
/// orientation chosen on the same raw the right way up: the photo opens upright.
#[test]
fn f2_auto_turns_a_raw_as_its_exif_says() {
    let (mosaic, level) = small_raw(-0.67);
    for (index, option) in ORIENTATIONS.iter().enumerate().skip(2) {
        let mut turned = level.clone();
        turned.camera.as_mut().unwrap().orientation = bits(index) as u8;
        let base = Base::new(OPERATIONS, &turned);
        assert!(
            Orientation
                .bind(&ModuleValues::new(), &turned, &base)
                .is_empty()
        );
        let auto = Bench::mosaic(&mosaic, turned, Pipe::Export).edit(&Edit::default(), Frame);
        let mut bench = Bench::mosaic(&mosaic, level.clone(), Pipe::Export);
        let mut chosen_edit = Edit::default();
        chosen_edit
            .set_control(
                &Orientation,
                ORIENTATION,
                ControlValue::Choice(index as u32),
                &bench.image,
            )
            .unwrap();
        let chosen = bench.edit(&chosen_edit, Frame);
        assert!(same_pixels(&auto.pixels, &chosen.pixels), "{option}");
    }
}

/// F4: background only, with no Auto.
#[test]
fn f4_it_works_on_the_background_only() {
    let definition = Orientation.definition();
    assert!(!definition.layers);
    assert!(!definition.auto);
}

/// F5: darktable's orientations by their `$DESCRIPTION`s, in its enum's order from -1
/// (`src/common/image.h`), each the operation's choice of that value.
#[test]
fn f5_its_options_are_darktable_s_orientations() {
    let darktable = [
        (-1, "autodetect", "auto"),
        (0, "no rotation", "none"),
        (1, "flip vertically", "flip_vertically"),
        (2, "flip horizontally", "flip_horizontally"),
        (3, "rotate 180°", "rotate_180"),
        (4, "transpose", "transpose"),
        (5, "rotate -90°", "rotate_cw_90"),
        (6, "rotate 90°", "rotate_ccw_90"),
        (7, "transverse", "transverse"),
    ];
    for (index, (value, _, id)) in darktable.into_iter().enumerate() {
        assert_eq!(ORIENTATIONS[index], id);
        assert_eq!(value, index as i32 - 1);
    }
}
