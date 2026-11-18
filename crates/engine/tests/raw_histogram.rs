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

//! The raw's histogram on a GPU adapter, hardware or software (decision 87): the counts
//! of a mosaic's 16-bit values over a region, against a count on the CPU, whole and in
//! row windows on a GPU that binds less than the mosaic at once (decision 14); and
//! counted only where an instance reads it.

use apertrust_engine::compute::GpuImage;
use apertrust_engine::gpu::{GpuContext, GpuOptions};
use apertrust_engine::pipeline::Instance;
use apertrust_engine::raw_histogram;
use apertrust_operations::exposure::Exposure;
use apertrust_plugin::{Camera, ImageContext, ParamValues, Pattern, RAW_HISTOGRAM_BINS, Value};

/// An odd width, so rows start in the middle of a 32-bit word.
const WIDTH: u32 = 2001;
const HEIGHT: u32 = 403;
/// x, y, width and height, starting on an odd column.
const REGION: [u32; 4] = [3, 5, 1990, 390];

/// Values over the whole 16-bit range, 0 and 65535 included, many repeated.
fn mosaic() -> Vec<u16> {
    let mut state = 0x2545_f491_u32;
    (0..WIDTH * HEIGHT)
        .map(|i| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            match i % 97 {
                0 => 0,
                1 => u16::MAX,
                // A narrow band, as a raw's values cluster.
                n if n < 60 => 1000 + (state % 300) as u16,
                _ => state as u16,
            }
        })
        .collect()
}

fn on_the_cpu(values: &[u16], [x, y, width, height]: [u32; 4]) -> Vec<u32> {
    let mut counts = vec![0; RAW_HISTOGRAM_BINS];
    for row in y..y + height {
        for column in x..x + width {
            counts[usize::from(values[(row * WIDTH + column) as usize])] += 1;
        }
    }
    counts
}

fn gpu(options: GpuOptions) -> GpuContext {
    pollster::block_on(GpuContext::new(&options)).expect("a GPU adapter, hardware or software")
}

#[test]
fn the_counts_are_the_cpu_s_whole_and_in_row_windows() {
    let values = mosaic();
    let expected = on_the_cpu(&values, REGION);
    // Whole, and at 256 KiB a binding, the counts' own size, which WebGPU's least, 128
    // MiB, exceeds: 65 rows a window, 6 windows.
    let small = GpuOptions {
        max_storage_binding: Some(1 << 18),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    };
    for options in [GpuOptions::default(), small] {
        let gpu = gpu(options);
        let input = GpuImage::upload_mosaic(&gpu, WIDTH, HEIGHT, &values).unwrap();
        let histogram = pollster::block_on(raw_histogram::count(&gpu, &input, REGION)).unwrap();
        assert_eq!(histogram.pixels(), 1990 * 390);
        assert!(histogram.counts() == expected.as_slice());
    }
}

/// A GPU that binds less than the counts' 256 KiB refuses the count, never the
/// validation error binding them would raise.
#[test]
fn a_gpu_binding_less_than_the_counts_refuses_the_count() {
    use apertrust_engine::compute::ComputeError;
    let gpu = gpu(GpuOptions {
        max_storage_binding: Some(1 << 17),
        storage_offset_alignment: Some(256),
        ..GpuOptions::default()
    });
    let values = vec![1000u16; 64 * 8];
    let input = GpuImage::upload_mosaic(&gpu, 64, 8, &values).unwrap();
    let counted = pollster::block_on(raw_histogram::count(&gpu, &input, [0, 0, 64, 8]));
    assert!(
        matches!(
            counted,
            Err(ComputeError::Binding {
                operation: "raw_histogram",
                bytes: 262_144,
                limit: 131_072,
                ..
            })
        ),
        "{counted:?}"
    );
}

#[test]
fn it_is_counted_over_the_declared_image_only_where_an_instance_reads_it() {
    let gpu = gpu(GpuOptions::default());
    let values = mosaic();
    let input = GpuImage::upload_mosaic(&gpu, WIDTH, HEIGHT, &values).unwrap();
    let image = ImageContext {
        camera: Some(Camera {
            make: String::new(),
            model: String::new(),
            alias: String::new(),
            width: WIDTH,
            height: HEIGHT,
            cfa: Some(Pattern {
                width: 2,
                height: 2,
                values: vec![0, 1, 1, 2],
            }),
            black: Pattern {
                width: 1,
                height: 1,
                values: vec![0.0],
            },
            white: 65535.0,
            white_balance: None,
            xyz_to_camera: None,
            embedded_matrix: false,
            crop: [1, 1, 1999, 401],
            declared: REGION,
            exposure_bias: 0.0,
            iso: 0.0,
            linear_response_limit: 1.0,
            lens_correction: None,
            highlight_preservation: 0.0,
            orientation: 0,
        }),
        ..ImageContext::full(1990, 390)
    };
    let manual = [Instance {
        operation: &Exposure,
        params: ParamValues::new(),
    }];
    let counted = raw_histogram::for_instances(&gpu, &manual, &image, &input);
    assert_eq!(pollster::block_on(counted).unwrap(), None);

    let mut automatic = ParamValues::new();
    automatic.set("mode", Value::Choice(1));
    let automatic = [Instance {
        operation: &Exposure,
        params: automatic,
    }];
    let mut filled = image.clone();
    pollster::block_on(raw_histogram::fill(&gpu, &automatic, &mut filled, &input)).unwrap();
    let histogram = filled.raw_histogram.clone().expect("counted");
    assert!(histogram.counts() == on_the_cpu(&values, REGION).as_slice());
    // Counted once: an image that holds it is not counted again.
    let again = raw_histogram::for_instances(&gpu, &automatic, &filled, &input);
    assert_eq!(pollster::block_on(again).unwrap(), None);
    // An image that is not a raw has none to count.
    let plain = ImageContext::full(1990, 390);
    let none = raw_histogram::for_instances(&gpu, &automatic, &plain, &input);
    assert_eq!(pollster::block_on(none).unwrap(), None);
    // Nor does any instance read it there, so nothing waits for it.
    assert!(!raw_histogram::read_by(&automatic, &plain));
    assert!(raw_histogram::read_by(&automatic, &image));
}
