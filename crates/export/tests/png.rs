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

//! The PNG writer: a decoded file equal to the render quantised as darktable rounds it, at
//! 8 and 16 bits, and the chunks darktable's `png.c` writes.

use std::io::Read;

use apertrust_export::png::{self, BitDepth, Options};
use apertrust_export::{Error, Image, profile, quantise};

/// A 37×23 render: every channel ramps across, down or both, from below 0 to above 1,
/// past values that round either way at 8 and at 16 bits.
fn render() -> (Vec<[f32; 4]>, u32, u32) {
    let (width, height) = (37, 23);
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let (u, v) = (
                    x as f32 / (width - 1) as f32,
                    y as f32 / (height - 1) as f32,
                );
                [1.2 * u - 0.1, 1.1 * v - 0.05, 0.37 * u + 0.61 * v, 0.5]
            })
        })
        .collect();
    (pixels, width, height)
}

fn encode(options: &Options) -> Vec<u8> {
    let (pixels, width, height) = render();
    let mut file = Vec::new();
    png::write(
        Image::new(&pixels, width, height).unwrap(),
        options,
        &mut file,
    )
    .unwrap();
    file
}

/// A PNG's chunks, each its type and data.
fn chunks(file: &[u8]) -> Vec<([u8; 4], &[u8])> {
    assert_eq!(&file[..8], b"\x89PNG\r\n\x1a\n");
    let mut chunks = Vec::new();
    let mut at = 8;
    while at < file.len() {
        let length = u32::from_be_bytes(file[at..at + 4].try_into().unwrap()) as usize;
        let kind = file[at + 4..at + 8].try_into().unwrap();
        chunks.push((kind, &file[at + 8..at + 8 + length]));
        at += 12 + length;
    }
    chunks
}

#[test]
fn decodes_to_the_quantised_render_at_8_and_16_bits() {
    let (pixels, ..) = render();
    let channels = || pixels.iter().flat_map(|p| [p[0], p[1], p[2]]);
    for (bit_depth, expected) in [
        (
            BitDepth::Eight,
            channels().map(quantise::eight_bit).collect::<Vec<_>>(),
        ),
        (
            BitDepth::Sixteen,
            channels()
                .flat_map(|v| quantise::sixteen_bit(v).to_be_bytes())
                .collect(),
        ),
    ] {
        let file = encode(&Options {
            bit_depth,
            ..Options::default()
        });
        let mut decoder = ::png::Decoder::new(std::io::Cursor::new(&file));
        decoder.set_transformations(::png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        let frame = reader.next_frame(&mut decoded).unwrap();
        assert_eq!((frame.width, frame.height), (37, 23));
        assert_eq!(frame.color_type, ::png::ColorType::Rgb);
        assert_eq!(
            frame.bit_depth,
            match bit_depth {
                BitDepth::Eight => ::png::BitDepth::Eight,
                BitDepth::Sixteen => ::png::BitDepth::Sixteen,
            }
        );
        assert_eq!(decoded[..frame.buffer_size()], expected, "{bit_depth:?}");
    }
}

#[test]
fn holds_darktables_profile_then_its_cicp_before_the_pixels() {
    let file = encode(&Options::default());
    let chunks = chunks(&file);
    let kinds: Vec<&[u8; 4]> = chunks.iter().map(|(kind, _)| kind).collect();
    assert_eq!(kinds[..3], [b"IHDR", b"iCCP", b"cICP"]);
    assert_eq!(kinds[kinds.len() - 1], b"IEND");
    assert!(kinds[3..kinds.len() - 1].iter().all(|&k| k == b"IDAT"));
    // RGB, 8 bits a channel.
    assert_eq!(chunks[0].1[8..10], [8, 2]);

    // Named by its description, compression method 0.
    let iccp = chunks[1].1;
    let name = format!("{}\0\0", profile::DESCRIPTION);
    assert_eq!(&iccp[..name.len()], name.as_bytes());
    let mut embedded = Vec::new();
    flate2::read::ZlibDecoder::new(&iccp[name.len()..])
        .read_to_end(&mut embedded)
        .unwrap();
    assert_eq!(embedded, profile::srgb());

    // BT.709's primaries, sRGB's transfer, RGB itself, full range.
    assert_eq!(chunks[2].1, [1, 13, 0, 1]);
}

#[test]
fn takes_zlibs_levels_0_to_9() {
    let (pixels, width, height) = render();
    let image = Image::new(&pixels, width, height).unwrap();
    let mut sizes = Vec::new();
    for compression in 0..=9 {
        let mut file = Vec::new();
        png::write(
            image,
            &Options {
                compression,
                ..Options::default()
            },
            &mut file,
        )
        .unwrap();
        sizes.push(file.len());
    }
    // Level 0 stores; the others compress.
    assert!(sizes[1..].iter().all(|&size| size < sizes[0]), "{sizes:?}");
    let options = Options {
        compression: 10,
        ..Options::default()
    };
    assert!(matches!(
        png::write(image, &options, Vec::new()),
        Err(Error::Compression(10))
    ));
}
