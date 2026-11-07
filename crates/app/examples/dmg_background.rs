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

//! Renders the disk image's background, `packaging/macos/dmg/background.svg`, at 1x and 2x
//! into two PNGs, for `tools/build-dmg-background.sh`, which joins them into the TIFF the
//! image holds. The SVG's text is outlined and its logo linked by a relative path, so no
//! font is loaded and every machine draws the same pixels.
//!
//! `cargo run -p apertrust --example dmg_background -- SVG OUT_1X.png OUT_2X.png`

use std::fs;
use std::path::Path;

use resvg::{tiny_skia, usvg};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [svg, one, two] = args.as_slice() else {
        panic!("usage: dmg_background SVG OUT_1X.png OUT_2X.png");
    };
    let svg = Path::new(svg);
    let options = usvg::Options {
        resources_dir: svg.parent().map(Path::to_path_buf),
        ..usvg::Options::default()
    };
    let text = fs::read_to_string(svg).unwrap_or_else(|e| panic!("{}: {e}", svg.display()));
    let tree =
        usvg::Tree::from_str(&text, &options).unwrap_or_else(|e| panic!("{}: {e}", svg.display()));
    write(Path::new(one), &render(&tree, 1));
    write(Path::new(two), &render(&tree, 2));
}

/// The SVG at `scale` pixels a point, opaque, as a PNG with no chunk but the image's own.
fn render(tree: &usvg::Tree, scale: u32) -> Vec<u8> {
    let size = tree.size().to_int_size();
    let (width, height) = (size.width() * scale, size.height() * scale);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("a size above zero");
    let factor = scale as f32;
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(factor, factor),
        &mut pixmap.as_mut(),
    );
    // The ground fills the image, so every pixel is opaque and RGB alone keeps it.
    let rgb: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let pixel = pixel.demultiply();
            assert_eq!(pixel.alpha(), 255, "the background is opaque");
            [pixel.red(), pixel.green(), pixel.blue()]
        })
        .collect();

    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().expect("a PNG header");
    writer.write_image_data(&rgb).expect("the PNG's pixels");
    writer.finish().expect("the PNG's end");
    file
}

fn write(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
