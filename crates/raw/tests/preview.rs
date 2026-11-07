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

//! Reads each corpus raw's embedded picture at 256 pixels, as the grid panel's thumbnails do,
//! and compares its size with `corpus/previews.txt`, so that updating rawler shows every
//! picture that changes, appears or goes. A raw rawler decodes is held to its picture: the
//! picture, turned upright, is portrait where the raw shown upright is.
//!
//! Run with the corpus as `corpus.rs` says; `APERTRUST_BLESS=1` rewrites the expectations.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use apertrust_raw::Orientation;

const LIST: &str = include_str!("corpus/files.sha256");

/// The longer side asked for: a Medium cell at twice the pixels per point.
const SIDE: u32 = 256;

fn expected_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/previews.txt")
}

fn corpus_dir() -> PathBuf {
    std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    )
}

#[test]
#[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
fn corpus_previews_read_as_expected_and_upright() {
    let dir = corpus_dir();
    let mut actual = String::new();
    let mut sideways = Vec::new();
    for name in LIST
        .lines()
        .filter_map(|line| line.split_once("  "))
        .map(|(_, name)| name)
    {
        let path = dir.join(name);
        assert!(
            path.exists(),
            "{} is missing: run tools/fetch-raw-corpus.sh",
            path.display()
        );
        let result = match apertrust_raw::read_preview_file(&path, SIDE) {
            Ok(Some(preview)) => {
                assert_eq!(
                    preview.pixels.len(),
                    preview.width as usize * preview.height as usize * 4,
                    "{name}"
                );
                assert!(preview.width.max(preview.height) <= SIDE, "{name}");
                if let Ok(raw) = apertrust_raw::decode_file(&path) {
                    let crop = raw.default_crop.unwrap_or(raw.active_area);
                    let turned = matches!(
                        raw.orientation,
                        Orientation::Transpose
                            | Orientation::Rotate90
                            | Orientation::Transverse
                            | Orientation::Rotate270
                    );
                    let portrait = (crop.height > crop.width) != turned;
                    if (preview.height > preview.width) != portrait {
                        sideways.push(name);
                    }
                }
                format!("{}x{}", preview.width, preview.height)
            }
            Ok(None) => "none".to_owned(),
            Err(e) => format!(
                "error: {}",
                e.to_string().lines().next().unwrap_or_default()
            ),
        };
        writeln!(actual, "{name} | {result}").unwrap();
    }
    assert!(sideways.is_empty(), "pictures not upright: {sideways:?}");

    if std::env::var_os("APERTRUST_BLESS").is_some() {
        std::fs::write(expected_path(), &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(expected_path()).unwrap_or_default();
    let changed: Vec<String> = actual
        .lines()
        .zip(expected.lines().chain(std::iter::repeat("")))
        .filter(|(a, e)| a != e)
        .map(|(a, e)| format!("expected: {e}\n  actual: {a}"))
        .collect();
    assert!(
        changed.is_empty() && actual.lines().count() == expected.lines().count(),
        "{} picture(s) read differently (APERTRUST_BLESS=1 accepts the change):\n{}",
        changed.len(),
        changed.join("\n")
    );
}
