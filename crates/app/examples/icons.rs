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

//! Renders the app icon's platform files from its SVG sources in `assets/icons`
//! (its README lists them): the macOS iconsets and `apertrust.icns`, the Windows sizes and
//! `apertrust.ico`, the Linux hicolor tree named after the app id, and the mark's PNGs; and
//! the `.apxmp` edit's document icon on Windows, `apertrust-apxmp.ico`.
//! Each icon at 32 px and below is drawn from its `-small` SVG, the mark's simplified
//! drawing, but the edit's at 16, 20 and 24 px, each drawn from its own SVG (`-16`,
//! `-20`, `-24`) on that size's pixel grid.
//! Run by `tools/build-icons.sh`; the files it writes are committed.
//!
//! Every PNG is rendered by resvg at its own size, never scaled from another, and written
//! with no chunk but the image's own, so the files carry no metadata. The `.icns` and the
//! `.ico` hold those PNGs as they are: both formats take PNG entries (macOS 10.7 and
//! Windows Vista on), so no system tool is needed.

use std::fs;
use std::path::{Path, PathBuf};

use resvg::{tiny_skia, usvg};

/// The app's id, which names its Linux icons as its desktop file and metadata.
const APP_ID: &str = "org.apertrust.Apertrust";

/// macOS's iconset: each point size at 1x and 2x, with its `.icns` types.
const MACOS: [(u32, &[u8; 4], &[u8; 4]); 5] = [
    (16, b"icp4", b"ic11"),
    (32, b"icp5", b"ic12"),
    (128, b"ic07", b"ic13"),
    (256, b"ic08", b"ic14"),
    (512, b"ic09", b"ic10"),
];
/// Windows's sizes, every one in the `.ico`: the shell's views at 100% to 200%.
const WINDOWS: [u32; 10] = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256];
/// The freedesktop icon theme's sizes, in `hicolor/NxN/apps`.
const LINUX: [u32; 9] = [16, 22, 24, 32, 48, 64, 128, 256, 512];
/// The mark's heights, for the web and documents.
const MARK: [u32; 3] = [256, 512, 1024];
/// The largest size, in pixels, drawn from an icon's `-small` SVG.
const SMALL: u32 = 32;
/// The sizes of the edit's Windows icon drawn on their own pixel grids, each from its
/// `-<size>` SVG.
const PIXEL_GRID: [u32; 3] = [16, 20, 24];

fn main() {
    let icons = std::env::args_os().nth(1).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons"),
        PathBuf::from,
    );
    let svg = |name: &str| load(&icons.join("svg").join(name));
    // An icon's drawing at `size` pixels: its `-small` SVG at SMALL and below.
    let icon = |name: &str| {
        [
            svg(&format!("{name}.svg")),
            svg(&format!("{name}-small.svg")),
        ]
    };
    let at = |[full, small]: &[usvg::Tree; 2], size: u32| {
        render(if size <= SMALL { small } else { full }, size, size)
    };

    let light = icon("apertrust-macos");
    let dark = icon("apertrust-macos-dark");
    let mut icns = Vec::new();
    for (size, one, two) in MACOS {
        for (tree, set) in [(&light, "AppIcon"), (&dark, "AppIcon-dark")] {
            let folder = icons.join(format!("png/macos/{set}.iconset"));
            let at_1x = at(tree, size);
            let at_2x = at(tree, size * 2);
            write(&folder.join(format!("icon_{size}x{size}.png")), &at_1x);
            write(&folder.join(format!("icon_{size}x{size}@2x.png")), &at_2x);
            // An `.icns` holds one appearance: the dark set waits for an asset catalog.
            if set == "AppIcon" {
                icns.push((*one, at_1x));
                icns.push((*two, at_2x));
            }
        }
    }
    write(&icons.join("apertrust.icns"), &icns_file(&icns));

    let square = icon("apertrust-square");
    let mut ico = Vec::new();
    for size in WINDOWS {
        let png = at(&square, size);
        write(
            &icons.join(format!("png/windows/apertrust-{size}.png")),
            &png,
        );
        ico.push((size, png));
    }
    write(&icons.join("apertrust.ico"), &ico_file(&ico));

    // The edit's document icon on Windows, which has no page of the system's to draw on;
    // at 16, 20 and 24 px each from a drawing of its own, on that size's pixel grid.
    let document = icon("apertrust-apxmp-windows");
    let mut ico = Vec::new();
    for size in WINDOWS {
        let png = if PIXEL_GRID.contains(&size) {
            let own = svg(&format!("apertrust-apxmp-windows-{size}.svg"));
            render(&own, size, size)
        } else {
            at(&document, size)
        };
        let path = icons.join(format!("png/windows/apertrust-apxmp-{size}.png"));
        write(&path, &png);
        ico.push((size, png));
    }
    write(&icons.join("apertrust-apxmp.ico"), &ico_file(&ico));

    let linux = icon("apertrust-linux");
    let hicolor = icons.join("png/linux/hicolor");
    for size in LINUX {
        let path = hicolor.join(format!("{size}x{size}/apps/{APP_ID}.png"));
        write(&path, &at(&linux, size));
    }
    let scalable = hicolor.join(format!("scalable/apps/{APP_ID}.svg"));
    let source = icons.join("svg/apertrust-linux.svg");
    write(&scalable, &fs::read(&source).expect("the Linux icon's SVG"));

    let mark = svg("apertrust-mark.svg");
    let ratio = mark.size().width() / mark.size().height();
    for height in MARK {
        // As wide as the mark is at that height, rounded up so that no edge is cut.
        let width = (height as f32 * ratio).ceil() as u32;
        let path = icons.join(format!("png/mark/apertrust-mark-{height}.png"));
        write(&path, &render(&mark, width, height));
    }
}

fn load(path: &Path) -> usvg::Tree {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    usvg::Tree::from_str(&text, &usvg::Options::default())
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The SVG scaled to fit `width` by `height` pixels, centred, as a PNG file.
fn render(tree: &usvg::Tree, width: u32, height: u32) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("a size above zero");
    let size = tree.size();
    let scale = (width as f32 / size.width()).min(height as f32 / size.height());
    let fit = tiny_skia::Transform::from_scale(scale, scale).post_translate(
        (width as f32 - size.width() * scale) / 2.0,
        (height as f32 - size.height() * scale) / 2.0,
    );
    resvg::render(tree, fit, &mut pixmap.as_mut());
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let pixel = pixel.demultiply();
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
        })
        .collect();

    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().expect("a PNG header");
    writer.write_image_data(&rgba).expect("the PNG's pixels");
    writer.finish().expect("the PNG's end");
    file
}

/// An `.icns` of PNG entries: the magic and the file's length, then each entry's type,
/// its length with its eight bytes of header, and the PNG, all lengths big-endian.
fn icns_file(entries: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let length = 8 + entries.iter().map(|(_, png)| 8 + png.len()).sum::<usize>();
    let mut file = Vec::with_capacity(length);
    file.extend_from_slice(b"icns");
    file.extend_from_slice(&u32::try_from(length).expect("under 4 GB").to_be_bytes());
    for (kind, png) in entries {
        file.extend_from_slice(kind);
        file.extend_from_slice(&u32::try_from(8 + png.len()).expect("small").to_be_bytes());
        file.extend_from_slice(png);
    }
    file
}

/// An `.ico` of PNG entries: its six-byte header, a 16-byte entry for each image (its
/// width and height, 0 meaning 256, 32 bits a pixel, the PNG's length and offset, all
/// little-endian), then the PNGs.
fn ico_file(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(images.len()).expect("a few sizes");
    let mut file = Vec::new();
    file.extend_from_slice(&[0, 0, 1, 0]);
    file.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, png) in images {
        let side = if *size >= 256 { 0 } else { *size as u8 };
        file.extend_from_slice(&[side, side, 0, 0]);
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&u32::try_from(png.len()).expect("small").to_le_bytes());
        file.extend_from_slice(&u32::try_from(offset).expect("small").to_le_bytes());
        offset += png.len();
    }
    for (_, png) in images {
        file.extend_from_slice(png);
    }
    file
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).unwrap_or_else(|e| panic!("{}: {e}", folder.display()));
    }
    fs::write(path, bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
