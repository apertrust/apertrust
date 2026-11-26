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

//! Times the checked copy three ways (library.md, Move to… and Copy to…): the copy alone,
//! the copy with the source hashed as it is read (`Check::None`), and the copy, hash and
//! read-back past the system's cache (`Check::ReadBack`).
//!
//! ```sh
//! cargo run --release -p apertrust-library --example copy_timing -- [--cold WAY] SOURCE INTO [FILES]
//! ```
//!
//! The raws of 25 to 60 MB in `SOURCE`'s tree are taken in path order, repeated until there
//! are `FILES` (each once by default). Each is read once before the timing, so every way
//! meets the sources in the system's cache and the times are the destination's. With
//! `--cold WAY` (`alone`, `hash` or `read-back`) that way alone is timed, with no read
//! before, as a card just inserted is copied: each such run on a card inserted again, with
//! `FILES` raws on it. `INTO` is a folder that exists: each way copies into a new folder it
//! makes in it, `copy-timing-XXXXXX`, made only if no such name is there, each file into a
//! subfolder of its own so no name is taken; that folder, and nothing else, is removed once
//! the way is timed.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use apertrust_base::fs::{Retry, create, persist_noclobber, persist_retrying, sync};
use apertrust_library::copy::{CHUNK, Check, PhotoFiles, copy_photo};

/// The three ways, by their names and `--cold`'s.
const WAYS: [(&str, &str); 3] = [
    ("copy alone", "alone"),
    ("copy and hash", "hash"),
    ("copy, hash and read back", "read-back"),
];

const MIN: u64 = 25 << 20;
const MAX: u64 = 60 << 20;

/// A raw: its folder, its name and its size.
type Raw = (PathBuf, String, u64);

fn main() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1).peekable();
    let cold = if args.next_if(|a| a == "--cold").is_some() {
        let way = args.next().map(|w| w.to_string_lossy().into_owned());
        let Some(way) = WAYS
            .iter()
            .find(|(_, short)| Some(*short) == way.as_deref())
        else {
            eprintln!("--cold takes alone, hash or read-back");
            std::process::exit(2);
        };
        Some(way.0)
    } else {
        None
    };
    let (Some(source), Some(into)) = (args.next(), args.next()) else {
        eprintln!("usage: copy_timing [--cold WAY] SOURCE INTO [FILES]");
        std::process::exit(2);
    };
    let (source, into) = (PathBuf::from(source), PathBuf::from(into));
    let mut raws = Vec::new();
    find(&source, &mut raws)?;
    raws.sort();
    if raws.is_empty() {
        eprintln!("no raws of 25 to 60 MB in {}", source.display());
        std::process::exit(1);
    }
    let files: usize = args
        .next()
        .map_or(raws.len(), |n| n.to_string_lossy().parse().expect("FILES"));
    let bytes: u64 = (0..files).map(|i| raws[i % raws.len()].2).sum();
    let megabytes = bytes as f64 / 1e6;
    println!(
        "{files} files of {} raws, {megabytes:.0} MB, from {} into {}",
        raws.len(),
        source.display(),
        into.display()
    );
    if cold.is_none() {
        for (folder, name, _) in &raws {
            io::copy(&mut fs::File::open(folder.join(name))?, &mut io::sink())?;
        }
    }

    let mut alone = None;
    let ways = WAYS.map(|(way, _)| way);
    for way in ways
        .into_iter()
        .filter(|way| cold.is_none_or(|cold| cold == *way))
    {
        // Made by this run alone, so its removal removes nothing of anyone's.
        let scratch = tempfile::Builder::new()
            .prefix("copy-timing-")
            .tempdir_in(&into)?;
        let base = scratch.path();
        let start = Instant::now();
        for i in 0..files {
            let (folder, name, _) = &raws[i % raws.len()];
            let target = base.join(i.to_string());
            let check = match way {
                "copy alone" => {
                    plain_copy(&folder.join(name), &target, name)?;
                    continue;
                }
                "copy and hash" => Check::None,
                _ => Check::ReadBack,
            };
            let photo = PhotoFiles {
                folder: folder.clone(),
                main: name.clone(),
                companions: Vec::new(),
            };
            copy_photo(&photo, &target, check, &|| false, &mut |_| {}).map_err(io::Error::other)?;
        }
        let seconds = start.elapsed().as_secs_f64();
        let more = alone.map_or(String::new(), |alone: f64| {
            format!(
                ", +{:.1}% on the copy alone",
                (seconds / alone - 1.0) * 100.0
            )
        });
        alone.get_or_insert(seconds);
        println!(
            "{way}: {seconds:.2} s, {:.0} MB/s, {:.0} ms a file{more}",
            megabytes / seconds,
            seconds * 1000.0 / files as f64
        );
        scratch.close()?;
    }
    Ok(())
}

/// The raws of 25 to 60 MB in `folder`'s tree, hidden names left out.
fn find(folder: &Path, raws: &mut Vec<Raw>) -> io::Result<()> {
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type()?;
        if name.starts_with('.') {
        } else if kind.is_dir() {
            find(&entry.path(), raws)?;
        } else if kind.is_file() && (MIN..=MAX).contains(&entry.metadata()?.len()) {
            raws.push((folder.to_owned(), name, entry.metadata()?.len()));
        }
    }
    Ok(())
}

/// The checked copy's steps for one file but the hash and the read-back: streamed into a
/// hidden temporary, given the source's modified time, synced, renamed without replacing.
fn plain_copy(from: &Path, into: &Path, name: &str) -> io::Result<()> {
    let mut source = fs::File::open(from)?;
    let modified = source.metadata()?.modified()?;
    fs::create_dir_all(into)?;
    let mut temp = create(into, name)?;
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let got = source.read(&mut buffer)?;
        if got == 0 {
            break;
        }
        temp.write_all(&buffer[..got])?;
    }
    temp.as_file().set_modified(modified)?;
    sync(temp.as_file())?;
    persist_retrying(temp, &into.join(name), Retry::platform(), persist_noclobber)
        .map_err(|failed| failed.error)?;
    Ok(())
}
