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

//! A photo's facts (decision 110), over the raw corpus: the same EXIF as a photo's open
//! reads, and the capture time the folder's open reads; and the time the read takes
//! against the EXIF read alone, over a folder an environment variable names, for
//! `docs/library.md`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use apertrust_library::ops::Shot;
use apertrust_raw::{RawFile, read_exif_file};

fn corpus_dir() -> PathBuf {
    std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
        PathBuf::from,
    )
}

/// The files under `dir`, every level, sorted.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut dirs = vec![dir.to_owned()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if !path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[test]
#[ignore = "needs the raw corpus: tools/fetch-raw-corpus.sh"]
fn the_facts_hold_the_exif_a_photo_s_open_reads() {
    let paths = files(&corpus_dir());
    assert!(!paths.is_empty(), "no corpus at {}", corpus_dir().display());
    for path in &paths {
        let shot = Shot::read(path);
        let opened = RawFile::open(path).and_then(|file| file.metadata());
        let Ok((_, Some(exif))) = opened else {
            continue;
        };
        let shot = shot.unwrap_or_else(|| panic!("{}: no facts", path.display()));
        assert_eq!(shot.exif, exif, "{}", path.display());
        // The capture time as the EXIF read alone gives it.
        let alone = read_exif_file(path).ok().and_then(|exif| exif.taken);
        assert_eq!(shot.exif.taken, alone, "{}", path.display());
    }
}

/// Times the facts' read (`Shot::read`) against the EXIF read alone, file by file on one
/// thread, over every file under the folder `APERTRUST_FACTS_DIR` names, each read twice, the
/// first round with the EXIF read first, and prints both: run
/// with `cargo test --release -p apertrust-library --test facts -- --ignored --nocapture
/// times`. It only reads.
#[test]
#[ignore = "times reads over the folder APERTRUST_FACTS_DIR names"]
fn times_the_facts_against_the_exif_alone() {
    let Some(dir) = std::env::var_os("APERTRUST_FACTS_DIR") else {
        panic!("APERTRUST_FACTS_DIR names no folder");
    };
    let paths = files(Path::new(&dir));
    let time = |read: &dyn Fn(&Path)| {
        let start = Instant::now();
        for path in &paths {
            read(path);
        }
        start.elapsed().as_secs_f64() * 1e3
    };
    for round in ["first", "second"] {
        let exif = time(&|path| {
            let _ = read_exif_file(path);
        });
        // The read a summary makes of each photo, one at a time as the EXIF alone is.
        let facts = time(&|path| {
            let _ = Shot::read(path);
        });
        let n = paths.len().max(1) as f64;
        println!(
            "{round}: {} files; EXIF alone {exif:.0} ms ({:.2} ms a file); facts {facts:.0} ms \
             ({:.2} ms a file); facts / EXIF {:.2}",
            paths.len(),
            exif / n,
            facts / n,
            facts / exif.max(f64::MIN_POSITIVE)
        );
    }
}
