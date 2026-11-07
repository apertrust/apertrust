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

//! The JPEG writer against what darktable sets in libjpeg (`src/imageio/format/jpeg.c`),
//! read back from the markers of the files it writes.

use apertrust_export::jpeg::{self, Options, Subsample};
use apertrust_export::{Error, Image, profile, quantise};

/// libjpeg's `std_luminance_quant_tbl` and `std_chrominance_quant_tbl` (`jcparam.c`),
/// Annex K's, in natural order.
const ANNEX_K: [[u32; 64]; 2] = [
    [
        16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69,
        56, 14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81,
        104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
    ],
    [
        17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99,
        99, 47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
        99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    ],
];

/// libjpeg's `jpeg_natural_order` (`jutils.c`): the natural index of each coefficient in
/// the zigzag order a DQT marker holds them in.
const NATURAL_ORDER: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// The two tables darktable-cli wrote at its default quality, 95, in their DQT
/// markers' order (`tests/profile.rs` says which export).
const DARKTABLE_95: [[u8; 64]; 2] = [
    [
        2, 1, 1, 1, 1, 1, 2, 1, 1, 1, 2, 2, 2, 2, 2, 4, 3, 2, 2, 2, 2, 5, 4, 4, 3, 4, 6, 5, 6, 6,
        6, 5, 6, 6, 6, 7, 9, 8, 6, 7, 9, 7, 6, 6, 8, 11, 8, 9, 10, 10, 10, 10, 10, 6, 8, 11, 12,
        11, 10, 12, 9, 10, 10, 10,
    ],
    [
        2, 2, 2, 2, 2, 2, 5, 3, 3, 5, 10, 7, 6, 7, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
    ],
];

/// libjpeg's `jpeg_set_quality(cinfo, quality, TRUE)`: `jpeg_quality_scaling`, then
/// `jpeg_add_quant_table` with baseline's limit, in zigzag order.
fn libjpeg_tables(quality: u32) -> [[u8; 64]; 2] {
    let scale = if quality < 50 {
        5000 / quality
    } else {
        200 - quality * 2
    };
    ANNEX_K.map(|table| {
        NATURAL_ORDER.map(|natural| ((table[natural] * scale + 50) / 100).clamp(1, 255) as u8)
    })
}

/// darktable's `write_image`: libjpeg's default 2×2 luma, then its changes by quality, then
/// its choice.
fn darktable_factors(subsample: Subsample, quality: u8) -> (u8, u8) {
    let (mut h, mut v) = (2, 2);
    if quality > 90 {
        v = 1;
    }
    if quality > 92 {
        h = 1;
    }
    match subsample {
        Subsample::Auto => (h, v),
        Subsample::S444 => (1, 1),
        Subsample::S440 => (1, 2),
        Subsample::S422 => (2, 1),
        Subsample::S420 => (2, 2),
    }
}

/// A smooth 37×23 render, its sides no multiple of a block, reaching outside 0 to 1.
fn render() -> (Vec<[f32; 4]>, u32, u32) {
    let (width, height) = (37, 23);
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let (u, v) = (
                    x as f32 / (width - 1) as f32,
                    y as f32 / (height - 1) as f32,
                );
                [1.2 * u - 0.1, v, 0.5 * (u + v), 1.0]
            })
        })
        .collect();
    (pixels, width, height)
}

fn encode(options: &Options) -> Vec<u8> {
    let (pixels, width, height) = render();
    let mut file = Vec::new();
    jpeg::write(
        Image::new(&pixels, width, height).unwrap(),
        options,
        &mut file,
    )
    .unwrap();
    file
}

/// A JPEG's segments before its scans: each marker and its data.
fn segments(file: &[u8]) -> Vec<(u8, &[u8])> {
    assert_eq!(&file[..2], [0xFF, 0xD8], "no SOI");
    let mut segments = Vec::new();
    let mut at = 2;
    while file[at + 1] != 0xDA {
        assert_eq!(file[at], 0xFF);
        let length = u16::from_be_bytes([file[at + 2], file[at + 3]]) as usize;
        segments.push((file[at + 1], &file[at + 4..at + 2 + length]));
        at += 2 + length;
    }
    segments
}

fn tables(file: &[u8]) -> Vec<(u8, Vec<u8>)> {
    segments(file)
        .into_iter()
        .filter(|&(marker, _)| marker == 0xDB)
        .flat_map(|(_, data)| data.chunks(65).map(|t| (t[0], t[1..].to_vec())))
        .collect()
}

/// The frame's marker and each component's sampling factors, across and down.
fn frame(file: &[u8]) -> (u8, Vec<(u8, u8)>) {
    let (marker, data) = segments(file)
        .into_iter()
        .find(|&(marker, _)| (0xC0..=0xC2).contains(&marker))
        .expect("a frame");
    let factors = data[6..]
        .chunks(3)
        .map(|c| (c[1] >> 4, c[1] & 15))
        .collect();
    (marker, factors)
}

/// The Huffman tables' code counts by length, 1 to 16 bits, each keyed by its class (0 DC,
/// 1 AC) and its table (0 luma, 1 chroma).
fn huffman_counts(file: &[u8]) -> Vec<((u8, u8), [u8; 16])> {
    let mut counts = Vec::new();
    for (_, data) in segments(file).into_iter().filter(|&(m, _)| m == 0xC4) {
        let mut at = 0;
        while at < data.len() {
            let table: [u8; 16] = data[at + 1..at + 17].try_into().unwrap();
            counts.push(((data[at] >> 4, data[at] & 15), table));
            at += 17 + table.iter().map(|&n| usize::from(n)).sum::<usize>();
        }
    }
    counts.sort();
    counts
}

/// Annex K's standard Huffman tables' counts (K.3, libjpeg's `bits_dc_luminance` and the
/// rest in `jcparam.c`), in `huffman_counts`' order.
const ANNEX_K_COUNTS: [((u8, u8), [u8; 16]); 4] = [
    ((0, 0), [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0]),
    ((0, 1), [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]),
    ((1, 0), [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d]),
    ((1, 1), [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77]),
];

#[test]
fn tables_are_libjpegs_at_every_quality() {
    for quality in 5..=100u8 {
        let options = Options {
            quality,
            ..Options::default()
        };
        let expected = libjpeg_tables(quality.into());
        assert_eq!(
            tables(&encode(&options)),
            [(0, expected[0].to_vec()), (1, expected[1].to_vec())],
            "quality {quality}"
        );
    }
    assert_eq!(libjpeg_tables(95), DARKTABLE_95);
}

#[test]
fn sampling_factors_are_darktables_for_each_quality_and_choice() {
    for subsample in Subsample::ALL {
        for quality in 5..=100 {
            let (marker, factors) = frame(&encode(&Options {
                quality,
                subsample,
                ..Options::default()
            }));
            assert_eq!(marker, 0xC0, "baseline");
            assert_eq!(
                factors,
                [darktable_factors(subsample, quality), (1, 1), (1, 1)],
                "{subsample:?} at quality {quality}"
            );
        }
    }
    // darktable's default, quality 95 and Auto, is 4:4:4.
    assert_eq!(frame(&encode(&Options::default())).1[0], (1, 1));
}

#[test]
fn huffman_tables_are_optimised_as_darktable_has_libjpeg_optimise_them() {
    // Each of the four tables is fitted to the image, so none counts its codes as Annex
    // K's does.
    for subsample in [Subsample::S444, Subsample::S420] {
        let counts = huffman_counts(&encode(&Options {
            subsample,
            ..Options::default()
        }));
        assert_eq!(
            counts.iter().map(|&(table, _)| table).collect::<Vec<_>>(),
            ANNEX_K_COUNTS.map(|(table, _)| table)
        );
        for ((table, ours), (_, standard)) in counts.iter().zip(ANNEX_K_COUNTS) {
            assert_ne!(
                *ours, standard,
                "{subsample:?}: table {table:?} is Annex K's"
            );
        }
    }
    // Without optimising, jpeg-encoder writes Annex K's tables, which the counts above
    // restate.
    let (pixels, width, height) = render();
    let mut file = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut file, 95);
    encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
    encoder
        .encode(
            &quantise::rgb8(&pixels),
            width as u16,
            height as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    assert_eq!(huffman_counts(&file), ANNEX_K_COUNTS);
}

#[test]
fn holds_the_density_and_darktables_profile() {
    let file = encode(&Options {
        dpi: 240,
        ..Options::default()
    });
    let markers = segments(&file);
    let (marker, jfif) = markers[0];
    assert_eq!((marker, &jfif[..5]), (0xE0, &b"JFIF\0"[..]));
    // Dots per inch, 240 across and down.
    assert_eq!(&jfif[7..12], [1, 0, 240, 0, 240]);
    let icc: Vec<_> = markers
        .iter()
        .filter(|&&(marker, data)| marker == 0xE2 && data.starts_with(b"ICC_PROFILE\0"))
        .collect();
    assert_eq!(icc.len(), 1);
    let chunk = icc[0].1;
    // The first of one marker.
    assert_eq!(chunk[12..14], [1, 1]);
    assert_eq!(&chunk[14..], profile::srgb());
    // darktable's `metadata/resolution` by default, 300.
    let file = encode(&Options::default());
    assert_eq!(&segments(&file)[0].1[7..12], [1, 1, 44, 1, 44]);
}

#[test]
fn decodes_at_every_choice() {
    let (pixels, ..) = render();
    let expected = quantise::rgb8(&pixels);
    for subsample in Subsample::ALL {
        for quality in [95, 50] {
            let file = encode(&Options {
                quality,
                subsample,
                ..Options::default()
            });
            let mut decoder = jpeg_decoder::Decoder::new(file.as_slice());
            let decoded = decoder.decode().unwrap();
            let info = decoder.info().unwrap();
            assert_eq!((info.width, info.height), (37, 23));
            assert_eq!(decoder.icc_profile().unwrap(), profile::srgb());
            let error = decoded
                .iter()
                .zip(&expected)
                .map(|(&a, &b)| f64::from(a.abs_diff(b)))
                .sum::<f64>()
                / expected.len() as f64;
            assert!(error < 4.0, "{subsample:?} at {quality}: {error:.2} levels");
        }
    }
}

#[test]
fn refuses_qualities_darktable_does_not_offer_and_sides_over_65535() {
    let (pixels, width, height) = render();
    let image = Image::new(&pixels, width, height).unwrap();
    for quality in [4, 101] {
        let options = Options {
            quality,
            ..Options::default()
        };
        assert!(matches!(
            jpeg::write(image, &options, Vec::new()),
            Err(Error::Quality(q)) if q == quality
        ));
    }
    let row = vec![[0.5; 4]; 65536];
    let wide = Image::new(&row, 65536, 1).unwrap();
    assert!(matches!(
        jpeg::write(wide, &Options::default(), Vec::new()),
        Err(Error::TooLarge { largest: 65535, .. })
    ));
}
