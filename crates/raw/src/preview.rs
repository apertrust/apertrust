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

//! The camera's own picture of a raw, its embedded JPEG, which a thumbnail shows until
//! Apertrust's render of the photo arrives, as darktable shows it
//! (`dt_imageio_large_thumbnail`, then `dt_iop_flip_and_zoom_8` in `_init_8` of
//! `src/common/mipmap_cache.c`): read without decoding the mosaic, made smaller to the
//! size asked and turned upright by the photo's orientation, since the camera stores it as
//! the sensor sees it.
//!
//! rawler gives two pictures, a thumbnail (a DNG's) and a preview (most formats'); the
//! smaller that is at least the size asked is taken, and the larger where neither is.
//!
//! The disk cache keeps the pictures read here as JPEGs (decision 76): [`PREVIEW_VERSION`]
//! names how they were read, and [`read_jpeg`] reads one back, through rawler's own JPEG
//! decoder, so no decoder of its own is added.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use rawler::decoders::{Decoder, Orientation as RawlerOrientation, RawDecodeParams};
use rawler::decompressors::decompress_strips;
use rawler::decompressors::jpeg::JpegDecompressor;
use rawler::rawsource::RawSource;

use crate::{Error, Orientation, orientation_from};

/// A picture in 8-bit RGBA, rows top to bottom, four bytes a pixel, upright.
#[derive(Clone, PartialEq, Eq)]
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The pixels are too many to print.
        write!(f, "Preview({}×{})", self.width, self.height)
    }
}

/// How the embedded picture is read: rawler's version, which reads it, and this module's
/// own, raised whenever [`read_preview`] would give another picture for the same file. A
/// cached thumbnail of another reading is read again (decision 76). A test holds the first
/// to `Cargo.lock`, so updating rawler fails until it is raised.
pub const PREVIEW_VERSION: &str = "rawler 0.8.0, preview 1";

/// Reads the raw file's embedded picture, at most `longest` pixels on its longer side and
/// upright. `None` where the file holds none rawler reads; an error where the file cannot
/// be read or parsed.
pub fn read_preview_file(path: impl AsRef<Path>, longest: u32) -> Result<Option<Preview>, Error> {
    let path = path.as_ref();
    let source = RawSource::new(path).map_err(|source| Error::Read {
        path: path.to_owned(),
        source,
    })?;
    read_preview_source(&source, longest)
}

/// Reads a raw file's embedded picture from its contents, as [`read_preview_file`].
pub fn read_preview(file: &[u8], longest: u32) -> Result<Option<Preview>, Error> {
    read_preview_source(&RawSource::new_from_slice(file), longest)
}

fn read_preview_source(source: &RawSource, longest: u32) -> Result<Option<Preview>, Error> {
    let longest = longest.max(1);
    // A parser bug on an odd file must not take the application down.
    let read = catch_unwind(AssertUnwindSafe(|| {
        let decoder = rawler::get_decoder(source).map_err(|e| Error::Decode(e.to_string()))?;
        let params = RawDecodeParams::default();
        // darktable reads the orientation from the EXIF Orientation tag, as the decode does.
        let orientation = decoder
            .raw_metadata(source, &params)
            .ok()
            .and_then(|metadata| metadata.exif.orientation)
            .map_or(Orientation::Normal, |tag| {
                orientation_from(RawlerOrientation::from_u16(tag))
            });
        Ok::<_, Error>(
            embedded(&*decoder, source, &params, longest).map(|picture| (picture, orientation)),
        )
    }))
    .map_err(|_| Error::Decode("the decoder panicked reading the embedded picture".to_owned()))??;
    Ok(read.map(|(picture, orientation)| turn(&shrink(&picture, longest), orientation)))
}

/// The smaller of the file's thumbnail and preview that is at least `longest` on its longer
/// side, else the larger; a picture rawler fails to decode counts as none. The thumbnail is
/// the smaller by its nature, so the preview is decoded only where it is not enough.
fn embedded(
    decoder: &dyn Decoder,
    source: &RawSource,
    params: &RawDecodeParams,
    longest: u32,
) -> Option<Preview> {
    // rawler's pictures are the `image` crate's, read here without naming its types.
    macro_rules! rgba {
        ($picture:expr) => {
            $picture.ok().flatten().map(|picture| {
                let rgba = picture.to_rgba8();
                Preview {
                    width: rgba.width(),
                    height: rgba.height(),
                    pixels: rgba.into_raw(),
                }
            })
        };
    }
    let side = |p: &Preview| p.width.max(p.height);
    let thumbnail = rgba!(decoder.thumbnail_image(source, params)).filter(|p| side(p) > 0);
    if let Some(thumbnail) = thumbnail.as_ref().filter(|p| side(p) >= longest) {
        return Some(thumbnail.clone());
    }
    let preview = rgba!(decoder.preview_image(source, params)).filter(|p| side(p) > 0);
    match (thumbnail, preview) {
        (Some(thumbnail), Some(preview)) if side(&thumbnail) > side(&preview) => Some(thumbnail),
        (thumbnail, preview) => preview.or(thumbnail),
    }
}

/// `picture` at most `longest` pixels on its longer side, each pixel the mean of those
/// under it; never made larger.
fn shrink(picture: &Preview, longest: u32) -> Preview {
    let (width, height) = (picture.width, picture.height);
    let side = width.max(height);
    if side <= longest {
        return picture.clone();
    }
    let scale = f64::from(longest) / f64::from(side);
    let to = |n: u32| ((f64::from(n) * scale).round() as u32).clamp(1, longest);
    let (w, h) = (to(width), to(height));
    // The source columns and rows under each pixel: at least one each.
    let spans = |from: u32, to: u32| -> Vec<(usize, usize)> {
        (0..to)
            .map(|i| {
                let start = (u64::from(i) * u64::from(from) / u64::from(to)) as usize;
                let end = (u64::from(i + 1) * u64::from(from) / u64::from(to)) as usize;
                (start, end.max(start + 1))
            })
            .collect()
    };
    let (columns, rows) = (spans(width, w), spans(height, h));
    let stride = width as usize * 4;
    let mut pixels = Vec::with_capacity(w as usize * h as usize * 4);
    let mut sums = vec![0u64; w as usize * 4];
    for &(top, bottom) in &rows {
        sums.fill(0);
        for row in picture.pixels[top * stride..bottom * stride].chunks_exact(stride) {
            for (x, &(left, right)) in columns.iter().enumerate() {
                for pixel in row[left * 4..right * 4].as_chunks::<4>().0 {
                    for (sum, &value) in sums[x * 4..x * 4 + 4].iter_mut().zip(pixel) {
                        *sum += u64::from(value);
                    }
                }
            }
        }
        for (x, &(left, right)) in columns.iter().enumerate() {
            let count = ((right - left) * (bottom - top)) as u64;
            for &sum in &sums[x * 4..x * 4 + 4] {
                pixels.push(((sum + count / 2) / count) as u8);
            }
        }
    }
    Preview {
        width: w,
        height: h,
        pixels,
    }
}

/// Reads a baseline JPEG of three components, as the cache writes thumbnails, into 8-bit
/// RGBA, alpha opaque, through rawler's JPEG decoder; an error where it is none, another
/// kind, or does not end with its end marker, as one cut short does, which the decoder
/// would read without a word.
pub fn read_jpeg(jpeg: &[u8]) -> Result<Preview, Error> {
    if !jpeg.ends_with(&[0xFF, 0xD9]) {
        return Err(Error::Decode("a JPEG cut short".to_owned()));
    }
    let (width, height) =
        jpeg_size(jpeg).ok_or_else(|| Error::Decode("not a JPEG with a frame".to_owned()))?;
    let (w, h) = (width as usize, height as usize);
    // One strip of every row, so the picture is decoded once.
    let decode = || decompress_strips::<u16, _>(jpeg, w * 3, h, false, h, JpegDecompressor::new());
    let rgb = catch_unwind(AssertUnwindSafe(decode))
        .map_err(|_| Error::Decode("the JPEG decoder panicked".to_owned()))?
        .map_err(Error::Decode)?;
    let mut pixels = Vec::with_capacity(w * h * 4);
    for rgb in rgb.data.as_chunks::<3>().0 {
        // 8-bit samples, held in 16 bits unscaled.
        pixels.extend(rgb.iter().map(|&v| v.min(255) as u8));
        pixels.push(255);
    }
    Ok(Preview {
        width,
        height,
        pixels,
    })
}

/// A JPEG's width and height from its frame header, a baseline, extended or progressive
/// one's; `None` where there is none before the scan.
fn jpeg_size(jpeg: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    if jpeg.get(..2)? != [0xFF, 0xD8] {
        return None;
    }
    loop {
        let (&[0xFF, marker], rest) = jpeg.get(at..)?.split_first_chunk::<2>()? else {
            return None;
        };
        let length = usize::from(u16::from_be_bytes(*rest.first_chunk::<2>()?));
        if matches!(marker, 0xC0..=0xC2) {
            // Length, precision, then the height and the width.
            let height = u16::from_be_bytes(*rest.get(3..)?.first_chunk::<2>()?);
            let width = u16::from_be_bytes(*rest.get(5..)?.first_chunk::<2>()?);
            return (width > 0 && height > 0).then_some((width.into(), height.into()));
        }
        if marker == 0xDA {
            return None;
        }
        at += 2 + length;
    }
}

/// `picture` turned upright by `orientation`, the EXIF Orientation's transform from the
/// stored picture to the one shown.
fn turn(picture: &Preview, orientation: Orientation) -> Preview {
    let (width, height) = (picture.width as usize, picture.height as usize);
    if matches!(orientation, Orientation::Normal | Orientation::Unknown) {
        return picture.clone();
    }
    // A turn by a quarter swaps the sides.
    let quarter = matches!(
        orientation,
        Orientation::Transpose
            | Orientation::Rotate90
            | Orientation::Transverse
            | Orientation::Rotate270
    );
    let (w, h) = if quarter {
        (height, width)
    } else {
        (width, height)
    };
    // Each pixel shown at (x, y) is the stored one at `from(x, y)`.
    let from = |x: usize, y: usize| match orientation {
        Orientation::Normal | Orientation::Unknown => (x, y),
        Orientation::MirrorHorizontal => (width - 1 - x, y),
        Orientation::Rotate180 => (width - 1 - x, height - 1 - y),
        Orientation::MirrorVertical => (x, height - 1 - y),
        Orientation::Transpose => (y, x),
        // A quarter turn clockwise.
        Orientation::Rotate90 => (y, height - 1 - x),
        Orientation::Transverse => (width - 1 - y, height - 1 - x),
        // A quarter turn anticlockwise.
        Orientation::Rotate270 => (width - 1 - y, x),
    };
    let mut pixels = Vec::with_capacity(picture.pixels.len());
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = from(x, y);
            let at = (sy * width + sx) * 4;
            pixels.extend_from_slice(&picture.pixels[at..at + 4]);
        }
    }
    Preview {
        width: w as u32,
        height: h as u32,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture 3 wide and 2 high whose pixels are numbered 1 to 6 in reading order, each
    /// pixel's four bytes its number.
    ///
    /// ```text
    /// 1 2 3
    /// 4 5 6
    /// ```
    fn numbered() -> Preview {
        Preview {
            width: 3,
            height: 2,
            pixels: (1..=6u8).flat_map(|n| [n; 4]).collect(),
        }
    }

    /// The pixels' numbers in reading order, and the size.
    fn numbers(picture: &Preview) -> (u32, u32, Vec<u8>) {
        let numbers = picture
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p[0])
            .collect();
        (picture.width, picture.height, numbers)
    }

    #[test]
    fn each_orientation_turns_the_stored_picture_upright() {
        let cases = [
            (Orientation::Normal, (3, 2, vec![1, 2, 3, 4, 5, 6])),
            (Orientation::Unknown, (3, 2, vec![1, 2, 3, 4, 5, 6])),
            (
                Orientation::MirrorHorizontal,
                (3, 2, vec![3, 2, 1, 6, 5, 4]),
            ),
            (Orientation::Rotate180, (3, 2, vec![6, 5, 4, 3, 2, 1])),
            (Orientation::MirrorVertical, (3, 2, vec![4, 5, 6, 1, 2, 3])),
            // Across the main diagonal.
            (Orientation::Transpose, (2, 3, vec![1, 4, 2, 5, 3, 6])),
            // A quarter turn clockwise: the left column becomes the top row, read upwards.
            (Orientation::Rotate90, (2, 3, vec![4, 1, 5, 2, 6, 3])),
            // Across the other diagonal.
            (Orientation::Transverse, (2, 3, vec![6, 3, 5, 2, 4, 1])),
            // A quarter turn anticlockwise: the right column becomes the top row.
            (Orientation::Rotate270, (2, 3, vec![3, 6, 2, 5, 1, 4])),
        ];
        for (orientation, expected) in cases {
            assert_eq!(
                numbers(&turn(&numbered(), orientation)),
                expected,
                "{orientation:?}"
            );
        }
    }

    #[test]
    fn a_quarter_turn_and_its_opposite_give_the_picture_back() {
        let picture = numbered();
        let back = turn(
            &turn(&picture, Orientation::Rotate90),
            Orientation::Rotate270,
        );
        assert_eq!(back, picture);
    }

    #[test]
    fn a_picture_is_made_smaller_by_the_mean_of_the_pixels_under_each() {
        // 4 × 2, two grey levels on the left and right halves.
        let picture = Preview {
            width: 4,
            height: 2,
            pixels: [10u8, 10, 30, 30, 10, 10, 30, 30]
                .iter()
                .flat_map(|&v| [v, v, v, 255])
                .collect(),
        };

        let small = shrink(&picture, 2);

        assert_eq!((small.width, small.height), (2, 1));
        assert_eq!(small.pixels, [10, 10, 10, 255, 30, 30, 30, 255]);
        // Never larger, and a picture that fits stays as it is.
        assert_eq!(shrink(&picture, 8), picture);
        // A long thin one keeps a row.
        let thin = Preview {
            width: 100,
            height: 1,
            pixels: vec![7; 400],
        };
        let small = shrink(&thin, 10);
        assert_eq!((small.width, small.height), (10, 1));
        assert!(small.pixels.iter().all(|&v| v == 7));
    }

    #[test]
    fn a_file_that_is_no_raw_is_an_error() {
        assert!(matches!(
            read_preview(b"not a raw", 256),
            Err(Error::Decode(_))
        ));
    }

    /// The reading's version names the rawler that `Cargo.lock` holds, so updating rawler
    /// fails here until the version is raised and cached thumbnails are read again.
    #[test]
    fn the_preview_version_names_the_rawler_in_use() {
        let lock = include_str!("../../../Cargo.lock");
        let rawler = lock
            .split("\n[[package]]\n")
            .find_map(|package| package.strip_prefix("name = \"rawler\"\nversion = \""))
            .and_then(|rest| rest.split('"').next())
            .expect("rawler in Cargo.lock");
        assert!(
            PREVIEW_VERSION.starts_with(&format!("rawler {rawler}, ")),
            "{PREVIEW_VERSION} for rawler {rawler}: raise it"
        );
    }

    /// What is no JPEG, or one cut short, is an error, never a panic.
    #[test]
    fn what_is_no_jpeg_is_an_error() {
        assert!(read_jpeg(b"").is_err());
        assert!(read_jpeg(b"not a jpeg").is_err());
        // A start and a frame header of 2 by 1, and no scan.
        let header = [
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x01, 0x00, 0x02, 0x03,
        ];
        assert_eq!(jpeg_size(&header), Some((2, 1)));
        assert!(read_jpeg(&header).is_err());
        assert_eq!(jpeg_size(&header[..9]), None);
    }
}
