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

//! Holds the lens corrections the decoder reads to what darktable reads (decision
//! 25): `corpus/lens-exiv2.txt` is Exiv2's dump of the tags, and darktable's checks and
//! arithmetic from `_check_lens_correction_data` are restated here over it, so rawler's
//! parse meets Exiv2's and the reader meets darktable's. Fujifilm's so far.
//!
//! Needs the raw corpus (`tools/fetch-raw-corpus.sh`) and darktable's integration
//! images (`APERTRUST_TEST_IMAGES`): `cargo test --release -p apertrust-raw -- --ignored`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use apertrust_raw::{FujifilmCorrection, LensCorrection};

const LIST: &str = include_str!("corpus/files.sha256");
const EXIV2: &str = include_str!("corpus/lens-exiv2.txt");

fn corpus_dir() -> PathBuf {
    std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    )
}

fn is_raf(name: &str) -> bool {
    name.to_lowercase().ends_with(".raf")
}

#[test]
#[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
fn the_corpus_reads_as_darktable_reads_it() {
    let dir = corpus_dir();
    let names: Vec<&str> = LIST
        .lines()
        .filter_map(|line| line.split_once("  "))
        .map(|(_, name)| name)
        .collect();
    for &name in &names {
        let path = dir.join(name);
        assert!(
            path.exists(),
            "{} is missing: run tools/fetch-raw-corpus.sh",
            path.display()
        );
        // A raw rawler cannot decode has nothing to read; none of them is a RAF.
        let Ok(raw) = apertrust_raw::decode_file(&path) else {
            assert!(!is_raf(name), "{name} does not decode");
            continue;
        };
        if is_raf(name) {
            check(name, raw.lens_correction);
        } else {
            // Sony's, a DNG's and Olympus's come with their parts of the port.
            assert_eq!(raw.lens_correction, None, "{name}");
        }
    }

    // The GFX 50S records its corrections in eight knots, a layout darktable does
    // not read.
    let gfx = "Fujifilm/GFX 50S/20170525_0036TEST.RAF";
    assert!(names.contains(&gfx));
    assert_eq!(tags(gfx)["GeometricDistortionParams"].len(), 17);
    assert_eq!(darktable(gfx), None);
}

/// darktable's tests 0145 and 0146 render `xtransIV.raf`, an X-T3; `mire1-xtrans.raf` is
/// an X-T4.
#[test]
#[ignore = "needs darktable's integration images (APERTRUST_TEST_IMAGES)"]
fn darktable_s_test_images_read_as_darktable_reads_them() {
    let dir = PathBuf::from(
        std::env::var_os("APERTRUST_TEST_IMAGES")
            .expect("APERTRUST_TEST_IMAGES names darktable's integration images"),
    );
    for file in ["xtransIV.raf", "mire1-xtrans.raf"] {
        let raw = apertrust_raw::decode_file(dir.join(file)).unwrap();
        let read = check(&format!("darktable/{file}"), raw.lens_correction);
        assert!(read.is_some(), "{file}");
    }
}

/// Holds a RAF's corrections to darktable's, and holds that its vignetting is never read
/// past its last knot; returns them.
fn check(name: &str, read: Option<LensCorrection>) -> Option<FujifilmCorrection> {
    let read = read.map(|correction| match correction {
        LensCorrection::Fujifilm(fuji) => fuji,
        other => panic!("{name}: {} corrections in a RAF", other.name()),
    });
    assert_eq!(read, darktable(name), "{name}");
    if let Some(fuji) = &read {
        // darktable's vignetting table holds zeros past the last knot and divides a pixel
        // there by 1e-4 (docs/ports/lens.md, Precision); every radius in the image is
        // below 1, the half-diagonal.
        let last = fuji.cropf * fuji.knots[fuji.nc as usize - 1];
        assert!(
            last >= 1.0,
            "{name}: the last vignetting knot lies at {last} of the half-diagonal, inside \
             the corner, where darktable divides by 1e-4: a deviation for the user to \
             decide (decision 25)"
        );
    }
    read
}

/// A file's tags in the dump, each its values as Exiv2 prints them.
fn tags(name: &str) -> HashMap<&'static str, Vec<&'static str>> {
    EXIV2
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(" | ").collect();
            (fields[0] == name).then(|| (fields[1], fields[3].split(' ').collect()))
        })
        .collect()
}

/// Exiv2's `toFloat` of a value as it prints it: a rational's numerator over its
/// denominator in f32, 0 for a zero denominator.
fn to_float(value: &str) -> f32 {
    match value.split_once('/') {
        Some((n, d)) => {
            let (n, d) = (n.parse::<i32>().unwrap(), d.parse::<i32>().unwrap());
            if d == 0 { 0.0 } else { n as f32 / d as f32 }
        }
        None => value.parse().unwrap(),
    }
}

/// What darktable's `_check_lens_correction_data` stores for a RAF's tags, restated
/// in its own indices.
fn darktable(name: &str) -> Option<FujifilmCorrection> {
    let tags = tags(name);
    let (posd, posc, posv) = (
        tags.get("GeometricDistortionParams")?,
        tags.get("ChromaticAberrationParams")?,
        tags.get("VignettingParams")?,
    );
    let d = |i: usize| to_float(posd[i]);
    let c = |i: usize| to_float(posc[i]);
    let v = |i: usize| to_float(posv[i]);
    let mut fuji = FujifilmCorrection::default();
    if (posd.len(), posc.len(), posv.len()) == (19, 29, 19) {
        fuji.nc = 9;
        for i in 0..9 {
            let (kd, kc, kv) = (d(i + 1), c(i + 1), v(i + 1));
            if kd != kc || kd != kv {
                return None;
            }
            fuji.knots[i] = kd;
            fuji.distortion[i] = d(i + 10);
            fuji.ca_r[i] = c(i + 10);
            fuji.ca_b[i] = c(i + 19);
            fuji.vignetting[i] = v(i + 10);
        }
    } else if (posd.len(), posc.len(), posv.len()) == (23, 31, 23) {
        fuji.nc = 11;
        for i in 0..11 {
            let kd = d(i + 1);
            let kc = if i != 0 { c(i) } else { 0.0 };
            let kv = v(i + 1);
            if kd != kc || kd != kv {
                return None;
            }
            fuji.knots[i] = kd;
            fuji.distortion[i] = d(i + 12);
            if i != 0 {
                fuji.ca_r[i] = c(i + 10);
                fuji.ca_b[i] = c(i + 20);
            }
            fuji.vignetting[i] = v(i + 12);
        }
    } else {
        return None;
    }
    let crop_mode = tags
        .get("CropMode")
        .map(|mode| mode[0].parse::<i64>().unwrap());
    fuji.cropf = if crop_mode == Some(2) || crop_mode == Some(4) {
        1.25
    } else {
        1.0
    };
    Some(fuji)
}
