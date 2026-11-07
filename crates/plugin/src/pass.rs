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

//! The kernel every operation's passes run, and the header of every pass's uniform
//! block.

/// A kernel's WGSL, generated from its Slang source by `tools/build-kernels.sh`.
#[derive(Debug, Clone, Copy)]
pub struct Kernel {
    /// An operation's ID, darktable's name for the module it ports, or the name of one
    /// of the engine's own kernels.
    pub name: &'static str,
    pub wgsl: &'static str,
    /// Whether it exports `<name>_backward` and `<name>_forward_diff`, as every
    /// differentiable operation's kernel does. The raw stage's kernels and the engine's
    /// own do not (decision 21).
    pub derivatives: bool,
}

/// The start of every pass's uniform block, as `slang/pass.slang` declares it. It says
/// which part of the image the pass covers, so maths that depends on a pixel's position
/// is the same in a tile, a region or the whole image (decision 12), and where the
/// windows of its images that the engine binds start (decision 14).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassHeader {
    /// The invocations this dispatch runs, one per pixel of the region the pass writes,
    /// or, for a pass in row strips, the strip's rows.
    pub width: u32,
    pub height: u32,
    /// Where the first invocation lies: after the raw stage, the corner of its
    /// operation's output region in its stage's image at the pipe's scale, a tile's or
    /// a region's (decision 16); for a pass in row strips (the raw stage's before
    /// demosaic, and the engine's `region` copy), the strip's first row in the pass's
    /// invocations; 0 in the passes of a viewport's full-resolution part, which run over
    /// the whole full-resolution image (decision 15).
    pub origin: [u32; 2],
    /// Its operation's stage's image at full resolution: the developed image, or after
    /// an operation that moves pixels, that operation's output (decision 16).
    pub full_width: u32,
    pub full_height: u32,
    /// Pixels at the pipe's scale per pixel at full resolution, at most 1; 1 in the
    /// passes of a viewport's full-resolution part (decision 15).
    pub scale: f32,
    /// How many times `main` applies the per-pixel function: 1, more only in benchmarks.
    pub repeat: u32,
    /// The 32-bit word of the image's whole buffer at which the window bound at binding
    /// 0, the input, or 1, the output, starts: 0 when the whole buffer is bound. A
    /// kernel that runs in strips indexes the whole image less it (decision 14).
    pub input_base: u32,
    pub output_base: u32,
}

impl PassHeader {
    /// Bytes in the header; the operation's parameters start here.
    pub const SIZE: usize = 48;

    /// A pass over a whole image at full resolution, run once.
    pub fn whole(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            origin: [0, 0],
            full_width: width,
            full_height: height,
            scale: 1.0,
            repeat: 1,
            input_base: 0,
            output_base: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let words = [
            self.width,
            self.height,
            self.origin[0],
            self.origin[1],
            self.full_width,
            self.full_height,
            self.scale.to_bits(),
            self.repeat,
            self.input_base,
            self.output_base,
            // Reserved, so the operation's parameters start on a 16-byte step.
            0,
            0,
        ];
        let mut bytes = [0; Self::SIZE];
        for (chunk, word) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(words) {
            *chunk = word.to_le_bytes();
        }
        bytes
    }

    /// The whole uniform block: the header, then `params`, padded to the 16-byte steps
    /// uniform blocks are sized in.
    pub fn uniform_block(&self, params: &[u8]) -> Vec<u8> {
        let mut block = Vec::with_capacity((Self::SIZE + params.len()).next_multiple_of(16));
        block.extend_from_slice(&self.to_bytes());
        block.extend_from_slice(params);
        block.resize(block.len().next_multiple_of(16), 0);
        block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_words_are_in_declaration_order() {
        let header = PassHeader {
            origin: [3, 4],
            scale: 0.5,
            repeat: 7,
            input_base: 9,
            output_base: 10,
            ..PassHeader::whole(1, 2)
        };
        let bytes = header.to_bytes();
        let word = |i: usize| u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap());
        assert_eq!([word(0), word(1), word(2), word(3)], [1, 2, 3, 4]);
        assert_eq!([word(4), word(5)], [1, 2]);
        assert_eq!(f32::from_bits(word(6)), 0.5);
        assert_eq!(word(7), 7);
        assert_eq!([word(8), word(9), word(10), word(11)], [9, 10, 0, 0]);
    }

    #[test]
    fn uniform_block_pads_to_16_bytes() {
        let block = PassHeader::whole(1, 1).uniform_block(&[1, 2, 3, 4, 5]);
        assert_eq!(block.len(), 64);
        assert_eq!(&block[48..53], &[1, 2, 3, 4, 5]);
        assert!(block[53..].iter().all(|&b| b == 0));
    }
}
