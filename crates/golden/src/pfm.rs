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

//! Portable float map (PFM): the float image format darktable-cli exports.
//!
//! A colour PFM is a text header (`PF`, width, height, scale) followed by float32 RGB
//! samples. Rows are stored bottom to top; a negative scale means little-endian.

use std::path::Path;

use anyhow::{Context, Result, bail, ensure};

/// An RGB float32 image with rows top to bottom.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[f32; 3]>,
}

impl Image {
    pub fn new(width: usize, height: usize, pixels: Vec<[f32; 3]>) -> Result<Self> {
        ensure!(
            pixels.len() == width * height,
            "{} pixels for a {width}×{height} image",
            pixels.len()
        );
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// A 16-bit RGB or RGBA PNG, its values scaled to 0 to 1 as they are, still encoded
    /// for the file's profile.
    pub fn read_png16(path: &Path) -> Result<Self> {
        let file =
            std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info()?;
        let mut bytes = vec![0; reader.output_buffer_size().context("PNG too large")?];
        let info = reader.next_frame(&mut bytes)?;
        ensure!(
            info.bit_depth == png::BitDepth::Sixteen,
            "{}: {:?} bits, expected 16",
            path.display(),
            info.bit_depth
        );
        let channels = match info.color_type {
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            other => bail!("{}: {other:?}, expected RGB", path.display()),
        };
        let value =
            |i: usize| f32::from(u16::from_be_bytes([bytes[2 * i], bytes[2 * i + 1]])) / 65535.0;
        let (width, height) = (info.width as usize, info.height as usize);
        let pixels = (0..width * height)
            .map(|p| [0, 1, 2].map(|c| value(p * channels + c)))
            .collect();
        Self::new(width, height, pixels)
    }

    pub fn read(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Self::decode(&bytes).with_context(|| format!("decoding {}", path.display()))
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.encode()).with_context(|| format!("writing {}", path.display()))
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut header = Header { bytes, pos: 0 };
        match header.token()? {
            "PF" => {}
            "Pf" => bail!("greyscale PFM is not supported"),
            other => bail!("not a PFM file (magic {other:?})"),
        }
        let width: usize = header.token()?.parse().context("width")?;
        let height: usize = header.token()?.parse().context("height")?;
        let scale: f32 = header.token()?.parse().context("scale")?;
        // Exactly one whitespace byte separates the header from the samples.
        let data = &bytes[header.pos + 1..];

        let row_bytes = width * 3 * 4;
        ensure!(
            data.len() == row_bytes * height,
            "expected {} sample bytes for {width}×{height}, found {}",
            row_bytes * height,
            data.len()
        );
        let sample = |b: &[u8; 4]| {
            if scale < 0.0 {
                f32::from_le_bytes(*b)
            } else {
                f32::from_be_bytes(*b)
            }
        };

        let mut pixels = Vec::with_capacity(width * height);
        for row in data.chunks_exact(row_bytes).rev() {
            let (samples, _) = row.as_chunks::<4>();
            let (rgb, _) = samples.as_chunks::<3>();
            pixels.extend(
                rgb.iter()
                    .map(|[r, g, b]| [sample(r), sample(g), sample(b)]),
            );
        }
        Self::new(width, height, pixels)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = format!("PF\n{} {}\n-1.0\n", self.width, self.height).into_bytes();
        out.reserve(self.pixels.len() * 12);
        for row in self.pixels.chunks_exact(self.width).rev() {
            for value in row.iter().flatten() {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }
}

struct Header<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Header<'a> {
    /// Next whitespace-delimited header token; leaves `pos` on the byte after it.
    fn token(&mut self) -> Result<&'a str> {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
        let start = self.pos;
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| !b.is_ascii_whitespace())
        {
            self.pos += 1;
        }
        ensure!(self.pos < self.bytes.len(), "truncated PFM header");
        std::str::from_utf8(&self.bytes[start..self.pos]).context("PFM header is not text")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_values_and_row_order() {
        let image = Image::new(
            2,
            2,
            vec![[0.0, 0.5, 1.0], [2.0, -0.25, 3.5], [4.0; 3], [5.0; 3]],
        )
        .unwrap();
        assert_eq!(Image::decode(&image.encode()).unwrap(), image);
    }

    #[test]
    fn rows_are_stored_bottom_to_top() {
        let image = Image::new(1, 2, vec![[1.0; 3], [2.0; 3]]).unwrap();
        let bytes = image.encode();
        let first_stored = f32::from_le_bytes(bytes[bytes.len() - 24..][..4].try_into().unwrap());
        assert_eq!(first_stored, 2.0);
    }

    #[test]
    fn reads_a_16_bit_png() {
        let dir = std::env::temp_dir().join(format!("apertrust-golden-png-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Two rows of two pixels, written as darktable's PNG writer does: big-endian.
        let values: [u16; 12] = [0, 32768, 65535, 1, 2, 3, 65534, 0, 4096, 100, 200, 300];
        let write = |path: &Path, color: png::ColorType, depth: png::BitDepth, bytes: &[u8]| {
            let file = std::fs::File::create(path).unwrap();
            let mut encoder = png::Encoder::new(file, 2, 2);
            encoder.set_color(color);
            encoder.set_depth(depth);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(bytes)
                .unwrap();
        };
        let path = dir.join("rgb16.png");
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
        write(&path, png::ColorType::Rgb, png::BitDepth::Sixteen, &bytes);
        let image = Image::read_png16(&path).unwrap();
        let expected: Vec<[f32; 3]> = values
            .as_chunks::<3>()
            .0
            .iter()
            .map(|rgb| rgb.map(|v| f32::from(v) / 65535.0))
            .collect();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.pixels, expected);

        // An 8-bit PNG is refused rather than read at the wrong scale.
        let path = dir.join("rgb8.png");
        write(&path, png::ColorType::Rgb, png::BitDepth::Eight, &[0; 12]);
        assert!(Image::read_png16(&path).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_big_endian() {
        let mut bytes = b"PF\n1 1\n1.0\n".to_vec();
        for v in [1.0f32, 2.0, 3.0] {
            bytes.extend_from_slice(&v.to_be_bytes());
        }
        assert_eq!(Image::decode(&bytes).unwrap().pixels, vec![[1.0, 2.0, 3.0]]);
    }
}
