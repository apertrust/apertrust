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

//! Decodes the raw corpus, the raw.pixls.us samples in `corpus/files.sha256`, and
//! compares a fingerprint of each result with `corpus/expected.txt`, so that updating
//! rawler shows every change in decoding, including files that start or stop working.
//!
//! Fetch the corpus with `tools/fetch-raw-corpus.sh`, then run
//! `cargo test --release -p apertrust-raw -- --ignored`. After a deliberate change, run
//! it with `APERTRUST_BLESS=1` to rewrite the expectations, and review their diff.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use apertrust_raw::{Pixels, RawImage, Rect};

const LIST: &str = include_str!("corpus/files.sha256");

fn expected_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/expected.txt")
}

fn corpus_dir() -> PathBuf {
    std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    )
}

#[test]
#[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
fn corpus_decodes_as_expected() {
    let dir = corpus_dir();
    let mut actual = String::new();
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
        let result = match apertrust_raw::decode_file(&path) {
            Ok(raw) => fingerprint(&raw),
            Err(e) => format!(
                "error: {}",
                e.to_string().lines().next().unwrap_or_default()
            ),
        };
        writeln!(actual, "{name} | {result}").unwrap();
    }

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
        "{} file(s) decode differently (APERTRUST_BLESS=1 accepts the change):\n{}",
        changed.len(),
        changed.join("\n")
    );
}

/// The metadata read a photo's open makes before its decode (decision 77) gives what the
/// decode gives but the pixels, for every sample of the corpus, or fails where it fails.
#[test]
#[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
fn the_metadata_read_equals_the_decode_but_for_the_pixels() {
    let dir = corpus_dir();
    let mut differ = Vec::new();
    for name in LIST
        .lines()
        .filter_map(|line| line.split_once("  "))
        .map(|(_, name)| name)
    {
        let file = apertrust_raw::RawFile::open(dir.join(name))
            .unwrap_or_else(|e| panic!("{e}: run tools/fetch-raw-corpus.sh"));
        let read = file.metadata();
        let decoded = file.decode().map(|mut raw| {
            raw.pixels = match raw.pixels {
                Pixels::U16(_) => Pixels::U16(Vec::new()),
                Pixels::F32(_) => Pixels::F32(Vec::new()),
            };
            raw
        });
        // Every field, the lens's corrections and a DNG's tags among them.
        match (read, decoded) {
            (Ok(read), Ok(decoded)) if read == decoded => {}
            (Err(_), Err(_)) => {}
            (read, decoded) => {
                let (read, decoded) = (
                    read.map(|r| fingerprint(&r)),
                    decoded.map(|r| fingerprint(&r)),
                );
                differ.push(format!("{name}\n    read: {read:?}\n  decode: {decoded:?}"))
            }
        }
    }
    assert!(
        differ.is_empty(),
        "{} file(s) read otherwise than they decode:\n{}",
        differ.len(),
        differ.join("\n")
    );
}

/// Everything the decoder reports, on one line, with the pixels as a hash.
fn fingerprint(raw: &RawImage) -> String {
    let rect = |r: Rect| format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
    let (kind, hash) = match &raw.pixels {
        Pixels::U16(values) => ("u16", fnv1a(values.iter().map(|&v| u64::from(v)))),
        Pixels::F32(values) => ("f32", fnv1a(values.iter().map(|v| u64::from(v.to_bits())))),
    };
    let cfa = raw.cfa.as_ref().map_or("none".to_owned(), |cfa| {
        let colors: String = cfa
            .colors
            .iter()
            .map(|c| format!("{c:?}").remove(0))
            .collect();
        format!("{}x{} {colors}", cfa.width, cfa.height)
    });
    let numbers = |values: &[f32]| {
        values
            .iter()
            .map(|v| format!("{v:.4}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    let matrices: Vec<String> = raw
        .color_matrices
        .iter()
        .map(|m| {
            format!(
                "{:?}:{}",
                m.illuminant,
                numbers(m.xyz_to_camera.as_flattened())
            )
        })
        .collect();
    let black = &raw.black_level;
    format!(
        "{} {} | alias {} | iso {} | {}x{}x{} {kind} {hash:016x} | active {} crop {} masked {} | cfa {cfa} | black {}x{} {} | white {:?} | wb {} | matrices {} | {:?}",
        raw.camera.make,
        raw.camera.model,
        raw.camera.alias,
        raw.iso.map_or("none".to_owned(), |iso| iso.to_string()),
        raw.width,
        raw.height,
        raw.components,
        rect(raw.active_area),
        raw.default_crop.map_or("none".to_owned(), rect),
        raw.masked_areas.len(),
        black.width,
        black.height,
        numbers(&black.values),
        raw.white_levels,
        raw.white_balance
            .as_deref()
            .map_or("none".to_owned(), numbers),
        matrices.join(" "),
        raw.orientation,
    )
}

/// FNV-1a over the values, one value at a time: stable across platforms and releases.
fn fnv1a(values: impl Iterator<Item = u64>) -> u64 {
    values.fold(0xcbf2_9ce4_8422_2325, |hash, v| {
        (hash ^ v).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
