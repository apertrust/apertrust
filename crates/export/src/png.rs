// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2025  darktable developers (ported from src/imageio/format/png.c)
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

//! darktable's PNG format (`src/imageio/format/png.c`), written by the png crate: RGB at
//! 8 or 16 bits, 16-bit samples most significant byte first; zlib's compression level, 0
//! to 9, through flate2's Rust backend, so the bytes differ from libpng's and the pixels
//! do not; darktable's sRGB profile in an iCCP chunk named by its description, then the
//! cICP chunk darktable writes for it. No pHYs, as darktable writes none.

use std::io::Write;

use ::png::{BitDepth as PngDepth, ColorType, DeflateCompression, Encoder, chunk};
use apertrust_base::settings::Choice;
use flate2::write::ZlibEncoder;

use crate::{Error, Image, profile, quantise};

/// The largest side a PNG can have, darktable's `dimension`.
pub const LARGEST: u32 = 2_147_483_647;

/// darktable's PNG options, with its defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub bit_depth: BitDepth,
    /// zlib's level, 0 to 9; darktable's 5.
    pub compression: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            bit_depth: BitDepth::Eight,
            compression: 5,
        }
    }
}

/// Bits a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    #[default]
    Eight,
    Sixteen,
}

/// The setting's options (`export.png.bit_depth`): darktable's own, the enum of 8 and 16
/// its `plugins/imageio/format/png/bpp` declares.
impl Choice for BitDepth {
    const OPTIONS: &'static [(Self, &'static str)] = &[(Self::Eight, "8"), (Self::Sixteen, "16")];
}

/// The cICP chunk darktable writes for its sRGB: BT.709's primaries, sRGB's transfer
/// function, RGB itself and full range (ITU-T H.273).
const CICP_SRGB: [u8; 4] = [1, 13, 0, 1];

/// Writes `image` to `out` as a PNG.
pub fn write(image: Image, options: &Options, out: impl Write) -> Result<(), Error> {
    let Options {
        bit_depth,
        compression,
    } = *options;
    if compression > 9 {
        return Err(Error::Compression(compression));
    }
    let (depth, data) = match bit_depth {
        BitDepth::Eight => (PngDepth::Eight, quantise::rgb8(image.pixels())),
        BitDepth::Sixteen => (
            PngDepth::Sixteen,
            image
                .pixels()
                .iter()
                .flat_map(|p| [p[0], p[1], p[2]])
                .flat_map(|v| quantise::sixteen_bit(v).to_be_bytes())
                .collect(),
        ),
    };
    let mut encoder = Encoder::new(out, image.width(), image.height());
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(depth);
    encoder.set_deflate_compression(DeflateCompression::Level(compression));
    let mut writer = encoder.write_header()?;
    writer.write_chunk(chunk::iCCP, &iccp(profile::srgb())?)?;
    writer.write_chunk(chunk::cICP, &CICP_SRGB)?;
    writer.write_image_data(&data)?;
    writer.finish()?;
    Ok(())
}

/// An iCCP chunk's data: the profile's name and a null, compression method 0, and the
/// profile, compressed by zlib. The png crate names a profile `_` (0.18.1).
fn iccp(profile: &[u8]) -> std::io::Result<Vec<u8>> {
    let name = format!("{}\0\0", profile::DESCRIPTION).into_bytes();
    let mut zlib = ZlibEncoder::new(name, flate2::Compression::default());
    zlib.write_all(profile)?;
    zlib.finish()
}
