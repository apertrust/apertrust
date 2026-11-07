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

//! What an export writes (decision 56), with no GPU: the engine renders, and this crate
//! writes the render in darktable's JPEG or PNG format, with that format's options,
//! quantised as darktable rounds and with darktable's sRGB output profile embedded.
//!
//! Built so far (`docs/gui.md`, The Export dialog): the writers, darktable's export size
//! ([`size`]), the export settings ([`settings`]), darktable's variables ([`variables`])
//! and the file-on-disk rules ([`disk`]).

// The generated settings name this crate's choice types by their paths in the schema,
// `apertrust_export::…`, as another crate would.
extern crate self as apertrust_export;

use std::io::Write;

use thiserror::Error;

pub mod disk;
pub mod jpeg;
pub mod png;
pub mod profile;
pub mod quantise;
pub mod settings;
pub mod size;
pub mod variables;

/// A rendered image: the export pipe's output, which colorout has encoded for darktable's
/// sRGB, row by row from the top, alpha ignored.
#[derive(Debug, Clone, Copy)]
pub struct Image<'a> {
    pixels: &'a [[f32; 4]],
    width: u32,
    height: u32,
}

impl<'a> Image<'a> {
    /// `pixels`, row by row; an error unless there are `width × height` of them.
    pub fn new(pixels: &'a [[f32; 4]], width: u32, height: u32) -> Result<Self, Error> {
        if pixels.len() as u64 != u64::from(width) * u64::from(height) {
            return Err(Error::Pixels {
                pixels: pixels.len(),
                width,
                height,
            });
        }
        Ok(Self {
            pixels,
            width,
            height,
        })
    }

    pub fn pixels(&self) -> &'a [[f32; 4]] {
        self.pixels
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

/// A format with its options, as darktable's format modules hold them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Jpeg(jpeg::Options),
    Png(png::Options),
}

impl Format {
    /// The format's name, for messages.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Jpeg(_) => "JPEG",
            Self::Png(_) => "PNG",
        }
    }

    /// The extension of the files the format writes, as darktable's format modules give
    /// it (`extension`), without the dot.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg(_) => "jpg",
            Self::Png(_) => "png",
        }
    }

    /// The largest side the format can have, darktable's `dimension`.
    pub fn largest(&self) -> u32 {
        match self {
            Self::Jpeg(_) => jpeg::LARGEST,
            Self::Png(_) => png::LARGEST,
        }
    }

    /// Writes `image` to `out` in this format, darktable's sRGB profile embedded.
    pub fn write(&self, image: Image, out: impl Write) -> Result<(), Error> {
        match self {
            Self::Jpeg(options) => jpeg::write(image, options, out),
            Self::Png(options) => png::write(image, options, out),
        }
    }
}

/// An export's settings as plain data, what a client sends the session's export job
/// (`docs/engine.md`, Commands and events): File on disk's path pattern and On conflict,
/// the format with its options, and the size. The dialog edits them, and the CLI reads
/// them from the `export.*` settings and its flags; the engine reads no settings itself
/// (decision 43).
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    /// File on disk's path, a pattern of darktable's variables without the extension
    /// ([`disk::Disk::new`]).
    pub path: String,
    pub on_conflict: settings::OnConflict,
    pub format: Format,
    pub size: size::Request,
}

/// The person's folders and name that darktable's variables give (`$(HOME)`,
/// `$(PICTURES.FOLDER)`, `$(DESKTOP.FOLDER)`, `$(USERNAME)`), sent beside an export's
/// settings by the client, which finds them on its own machine: the engine reads no
/// platform folder (decision 43). `None` for a folder the platform has not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Places {
    pub home: Option<std::path::PathBuf>,
    pub pictures: Option<std::path::PathBuf>,
    pub desktop: Option<std::path::PathBuf>,
    pub user_name: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{pixels} pixels for a {width}×{height} image")]
    Pixels {
        pixels: usize,
        width: u32,
        height: u32,
    },
    #[error("JPEG quality {0} is outside 5 to 100")]
    Quality(u8),
    #[error("PNG compression {0} is outside 0 to 9")]
    Compression(u8),
    #[error("{format} is at most {largest} px a side, and the image is {width}×{height}")]
    TooLarge {
        format: &'static str,
        largest: u32,
        width: u32,
        height: u32,
    },
    #[error("an export of {width}×{height} pixels has no image")]
    Empty { width: u32, height: u32 },
    #[error("cannot write the JPEG: {0}")]
    Jpeg(#[from] jpeg_encoder::EncodingError),
    #[error("cannot write the PNG: {0}")]
    Png(#[from] ::png::EncodingError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Whether the destination is at fault, the disk full, over quota or read-only, as
    /// [`disk::Error::destination`] says of a rename: the encoder writes the file as it
    /// encodes, so those errors surface here, as an I/O error of its own or inside the
    /// JPEG's or the PNG's, and an export stops at one, where any other error is this
    /// photo's.
    pub fn destination(&self) -> bool {
        self.io_kind().is_some_and(disk::destination_kind)
    }

    /// The kind of the I/O error the write met, bare or inside the JPEG's or the PNG's, so
    /// that a caller can say which of the destination's failures it was; `None` for an
    /// error of the image or the encoder.
    pub fn io_kind(&self) -> Option<std::io::ErrorKind> {
        let io = match self {
            Self::Io(error) => Some(error),
            Self::Jpeg(jpeg_encoder::EncodingError::IoError(error)) => Some(error),
            Self::Png(::png::EncodingError::IoError(error)) => Some(error),
            _ => None,
        };
        io.map(std::io::Error::kind)
    }
}
