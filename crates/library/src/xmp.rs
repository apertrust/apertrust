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

//! The XMP layer (decision 50): any valid XMP packet read into a tree of properties and
//! written back in a canonical compact form, for the `.apxmp` ([`sidecar`]), darktable's
//! `.xmp` and, later, the files an export writes. It is XMP and nothing about what any
//! property means, and it depends on `quick-xml` alone.
//!
//! [`Xmp::parse`] reads what a packet can be:
//!
//! - an optional `<?xpacket?>` wrapper, a byte order mark and trailing padding;
//! - `x:xmpmeta` around the RDF, or a bare `rdf:RDF`;
//! - several `rdf:Description`s, merged in order;
//! - a property as an attribute or as an element; a struct as `rdf:parseType="Resource"`,
//!   as a nested `rdf:Description` or as attributes alone; `rdf:resource`;
//! - `rdf:Bag`, `rdf:Seq` and `rdf:Alt`, their items with `xml:lang` and `x-default`;
//! - qualifiers, as `rdf:value` with the qualifiers beside it.
//!
//! Every form gives the same [`Node`], and every property of any namespace, the ones
//! Apertrust does not know too, is kept in its place with its prefix. An error names the
//! line, and what RDF has beyond XMP's subset is an error, not a guess.
//!
//! [`Xmp::to_xml`] writes the canonical form, the file a sidecar is; [`Xmp::to_packet`]
//! wraps it as a file embeds it. Both are the same bytes for the same tree, and a packet
//! this layer wrote is read back to the tree it was written from, but for these exceptions.
//! A read keeps only the first description's `rdf:about`, the others merged away, and
//! ignores `rdf:ID`, `rdf:nodeID` and `rdf:datatype`. A write declares the prefixes a tree
//! built by hand needs, which the read then lists in `namespaces`, and drops from a value
//! any character XML 1.0 cannot hold (a control character other than tab, newline and
//! carriage return, U+FFFE and U+FFFF), naming the property in a `tracing` warning. The
//! layer does not merge two packets or
//! decide which properties belong to which program: a caller [`Xmp::set`]s and
//! [`Xmp::remove`]s the properties it owns and leaves the rest as they were read, as
//! darktable's `dt_exif_xmp_write` does with `dt_xmp_keys`.
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Apertrust 0.4.0">
//!  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
//!   <rdf:Description rdf:about=""
//!     xmlns:xmp="http://ns.adobe.com/xap/1.0/"
//!     xmlns:dc="http://purl.org/dc/elements/1.1/"
//!     xmp:Rating="3">
//!    <dc:subject>
//!     <rdf:Bag>
//!      <rdf:li>harbour</rdf:li>
//!     </rdf:Bag>
//!    </dc:subject>
//!   </rdf:Description>
//!  </rdf:RDF>
//! </x:xmpmeta>
//! ```
//!
//! [`sidecar`]: crate::sidecar

mod model;
mod read;
mod write;

#[cfg(test)]
mod tests;

use thiserror::Error;

pub use model::{
    ArrayKind, DEFAULT_LANGUAGE, META, Name, Namespace, Node, Property, Qualifier, RDF, Value, XML,
    Xmp,
};

/// A packet that cannot be read, with the line it fails on (the first line is 1).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line}: {message}")]
pub struct Error {
    line: usize,
    message: String,
}

impl Error {
    pub(crate) fn at(line: usize, message: impl Into<String>) -> Self {
        Error {
            line,
            message: message.into(),
        }
    }

    /// The line the packet fails on, counting from 1.
    pub fn line(&self) -> usize {
        self.line
    }

    /// What is wrong, without the line.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// The line holding the byte `position` of `text`.
fn line_of(text: &str, position: usize) -> usize {
    let end = position.min(text.len());
    1 + text.as_bytes()[..end]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
}

impl Xmp {
    /// Reads a packet from text. A leading byte order mark is skipped.
    pub fn parse(xml: &str) -> Result<Xmp, Error> {
        read::parse(xml)
    }

    /// Reads a packet from bytes: UTF-8, or UTF-16 as XMP allows, found by its byte order mark
    /// or, as the XML specification says, by a first `<?` in two bytes; a byte order mark is
    /// skipped.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Xmp, Error> {
        let utf16 = |rest: &[u8], big_endian: bool| -> Result<String, Error> {
            let (pairs, odd) = rest.as_chunks::<2>();
            if !odd.is_empty() {
                return Err(Error::at(1, "UTF-16 text of an odd length"));
            }
            let units = pairs.iter().map(|&pair| {
                if big_endian {
                    u16::from_be_bytes(pair)
                } else {
                    u16::from_le_bytes(pair)
                }
            });
            char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .map_err(|_| Error::at(1, "invalid UTF-16 text"))
        };
        let text = match bytes {
            [0xFE, 0xFF, rest @ ..] => utf16(rest, true)?,
            [0xFF, 0xFE, rest @ ..] => utf16(rest, false)?,
            // No byte order mark, as in a packet whose only one is the `xpacket`'s: the
            // XML specification's detection, from the first `<?` in either byte order.
            [b'<', 0, b'?', 0, ..] => utf16(bytes, false)?,
            [0, b'<', 0, b'?', ..] => utf16(bytes, true)?,
            _ => std::str::from_utf8(bytes)
                .map_err(|e| {
                    Error::at(
                        1 + bytes[..e.valid_up_to()]
                            .iter()
                            .filter(|&&b| b == b'\n')
                            .count(),
                        "not UTF-8 text",
                    )
                })?
                .to_owned(),
        };
        read::parse(&text)
    }

    /// The packet as a sidecar file holds it: an XML declaration, then the packet with no
    /// `xpacket` wrapper.
    pub fn to_xml(&self) -> String {
        write::to_xml(self)
    }

    /// The packet as a file embeds it: in the `xpacket` wrapper, with `padding` bytes of
    /// whitespace for an edit in place to grow into.
    pub fn to_packet(&self, padding: usize) -> String {
        write::to_packet(self, padding)
    }
}
