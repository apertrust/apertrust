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

//! A tool's kernel uniforms against a golden case's, which the tests of the tools with
//! golden cases share: the uniforms as words, and how far two words are apart.

/// Little-endian bytes as four-byte words.
pub fn words(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

/// How far a uniform word of ours is from the golden case's: 0 for the same bits, else
/// the relative difference of the two as floats.
pub fn apart(ours: [u8; 4], theirs: [u8; 4]) -> f32 {
    let (a, b) = (f32::from_le_bytes(ours), f32::from_le_bytes(theirs));
    if ours == theirs {
        0.0
    } else {
        (a - b).abs() / b.abs()
    }
}
