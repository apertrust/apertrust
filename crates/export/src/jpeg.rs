// Apertrust, a free, non-destructive, cross-platform raw photo editor
// Copyright (C) 2026  Apertrust developers
// Copyright (C) 2010-2024  darktable developers (ported from src/imageio/format/jpeg.c)
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

//! darktable's JPEG format (`src/imageio/format/jpeg.c`), written by jpeg-encoder, a JPEG
//! encoder in Rust (decision 56), set as darktable sets libjpeg:
//!
//! - the quality, 5 to 100, through libjpeg's `jpeg_set_quality` tables, which
//!   jpeg-encoder computes the same way: Annex K's tables scaled by 5000 / q below 50 and
//!   by 200 − 2q from 50, rounded and held between 1 and 255;
//! - darktable's chroma subsampling ([`Subsample`]);
//! - optimised Huffman tables, a baseline frame, the JFIF density in dots per inch, and
//!   darktable's sRGB profile in APP2 markers.
//!
//! Not mirrored, since jpeg-encoder has neither: libjpeg's DCT by quality (floating point
//! above 95, the fast integer one below 50, and between them the accurate integer one,
//! which jpeg-encoder uses at every quality), and libjpeg's input smoothing below 80.
//! Subsampled chroma is the average of the pixels it covers, as in libjpeg.
//!
//! With optimised tables jpeg-encoder writes a scan for each component where libjpeg
//! writes one for all three. libjpeg and the Mac's ImageIO read both; zune-jpeg 0.5.15,
//! the image crate's decoder, misreads a scan for each component where chroma is
//! subsampled, below quality 93 at Auto.

use std::io::Write;

use apertrust_base::settings::Choice;
use jpeg_encoder::{ChromaSubsamplingMethod, ColorType, Encoder, PixelDensity, SamplingFactor};

use crate::{Error, Image, profile, quantise};

/// The largest side a JPEG can have, darktable's `dimension`.
pub const LARGEST: u32 = 65535;

/// darktable's JPEG options, with its defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// 5 to 100; darktable's 95.
    pub quality: u8,
    pub subsample: Subsample,
    /// The density the JFIF header gives, in dots per inch: darktable's
    /// `metadata/resolution`, 300.
    pub dpi: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            quality: 95,
            subsample: Subsample::Auto,
            dpi: 300,
        }
    }
}

/// darktable's chroma subsampling choices, in its order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Subsample {
    /// libjpeg's default, 4:2:0, with darktable's changes: 4:2:2 above quality 90 and
    /// 4:4:4 above 92.
    #[default]
    Auto,
    /// 4:4:4, no chroma subsampling.
    S444,
    /// 4:4:0, chroma halved vertically.
    S440,
    /// 4:2:2, chroma halved horizontally.
    S422,
    /// 4:2:0, chroma halved both ways.
    S420,
}

/// The setting's options (`export.jpeg.subsample`), by ids of their own, in darktable's
/// order, so the number darktable keeps for one (`dt_imageio_jpeg_subsample_t`) is its
/// place.
impl Choice for Subsample {
    const OPTIONS: &'static [(Self, &'static str)] = &[
        (Self::Auto, "auto"),
        (Self::S444, "444"),
        (Self::S440, "440"),
        (Self::S422, "422"),
        (Self::S420, "420"),
    ];
}

impl Subsample {
    pub const ALL: [Self; 5] = [Self::Auto, Self::S444, Self::S440, Self::S422, Self::S420];

    /// The luma's sampling factors, across and down, that darktable sets at `quality`.
    fn sampling(self, quality: u8) -> SamplingFactor {
        match self {
            Self::Auto if quality > 92 => SamplingFactor::F_1_1,
            Self::Auto if quality > 90 => SamplingFactor::F_2_1,
            Self::Auto => SamplingFactor::F_2_2,
            Self::S444 => SamplingFactor::F_1_1,
            Self::S440 => SamplingFactor::F_1_2,
            Self::S422 => SamplingFactor::F_2_1,
            Self::S420 => SamplingFactor::F_2_2,
        }
    }
}

/// Writes `image` to `out` as an 8-bit JPEG.
pub fn write(image: Image, options: &Options, out: impl Write) -> Result<(), Error> {
    let Options {
        quality,
        subsample,
        dpi,
    } = *options;
    if !(5..=100).contains(&quality) {
        return Err(Error::Quality(quality));
    }
    let (Ok(width), Ok(height)) = (u16::try_from(image.width()), u16::try_from(image.height()))
    else {
        return Err(Error::TooLarge {
            format: "JPEG",
            largest: LARGEST,
            width: image.width(),
            height: image.height(),
        });
    };
    let mut encoder = Encoder::new(out, quality);
    encoder.set_sampling_factor(subsample.sampling(quality));
    encoder.set_chroma_subsampling_method(ChromaSubsamplingMethod::Average);
    encoder.set_optimized_huffman_tables(true);
    encoder.set_density(PixelDensity::dpi(dpi));
    encoder.add_icc_profile(profile::srgb())?;
    encoder.encode(
        &quantise::rgb8(image.pixels()),
        width,
        height,
        ColorType::Rgb,
    )?;
    Ok(())
}
