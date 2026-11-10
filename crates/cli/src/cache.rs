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

//! `cache`: the disk cache's commands, run in this process on base's `cache` directly,
//! with no session and no GPU (decision 76; `docs/cache.md`, The CLI). The cache is opened
//! without creating anything: a folder that is missing is an empty cache.

use std::path::Path;

use anyhow::{Result, anyhow, bail};
use apertrust_base::cache::{self, Cache, KINDS, Kind, Limit, Policy, Removed, Sizes, Usage};
use apertrust_base::folders::Folders;
use apertrust_base::settings::{Choice, Store};
use apertrust_engine::settings::{self as engine, registry::CACHE_UNUSED_AFTER};
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show the cache folder and how it was chosen, the limit (cache.limit, or unlimited
    /// while cache.limited is off), the limited kinds' total, each kind's entries and
    /// size, and cache.unused_after. It walks the folder and writes nothing.
    Info,
    /// Remove the entries of the limited kinds named, thumbnails or pipelines, or of every
    /// limited kind when none is named, and print what was removed and what could not be:
    /// an entry that cannot be removed is passed over, and the exit status is then 1. The
    /// kept kinds, libraries and copies, are refused: their owners remove them.
    Clear {
        #[arg(value_name = "KIND", value_parser = limited_kind)]
        kinds: Vec<Kind>,
    },
}

/// Runs `command` on the cache folder `folders` chose, with the engine's settings in
/// `store`.
pub fn run(command: Command, folders: Option<&Folders>, store: &Store) -> Result<()> {
    let folders = folders.ok_or_else(|| {
        anyhow!("no cache folder: the system gives none; name one with --cache-dir")
    })?;
    match command {
        Command::Info => info(folders, store),
        Command::Clear { kinds } => clear(folders.cache(), &kinds),
    }
}

/// A limited kind by its id, as `cache clear` takes it.
fn limited_kind(id: &str) -> Result<Kind, String> {
    let limited = || {
        let ids = KINDS.iter().filter(|kind| kind.policy() == Policy::Limited);
        ids.map(|kind| kind.id()).collect::<Vec<_>>().join(", ")
    };
    match cache::kind(id) {
        Some(kind) if kind.policy() == Policy::Limited => Ok(kind),
        Some(kind) => Err(format!(
            "the cache's {} are kept: only their owner removes them; clear one of {}",
            kind.id(),
            limited()
        )),
        None => Err(format!("no such kind: clear one of {}", limited())),
    }
}

fn info(folders: &Folders, store: &Store) -> Result<()> {
    let folder = folders.cache();
    let sizes = Cache::open(folder)
        .walk(&|| false)
        .map_err(|e| anyhow!("cannot walk the cache in {}: {e}", folder.display()))?;
    // `cache.unused_after` by its id, as `settings.toml` holds it.
    let unused = store.get(&CACHE_UNUSED_AFTER);
    let mut options = cache::UnusedAfter::OPTIONS.iter();
    let unused = options
        .find(|(option, _)| *option == unused)
        .map_or("", |(_, id)| id);
    let chosen = folders.cache_chosen().to_string();
    let limit = engine::cache_limit(store);
    print!("{}", report(folder, &chosen, limit, &sizes, unused));
    Ok(())
}

/// `cache info`'s lines, one item to a line.
fn report(folder: &Path, chosen: &str, limit: Limit, sizes: &Sizes, unused: &str) -> String {
    let missing = if folder.exists() {
        ""
    } else {
        " (not created yet)"
    };
    let limit = match limit {
        Limit::Unlimited => "unlimited".to_owned(),
        Limit::Bytes(bytes) => format!("{} GB", bytes as f64 / 1e9),
    };
    let mut text = format!(
        "folder: {}{missing}\nchosen by: {chosen}\nlimit: {limit}\nlimited kinds: {}\n",
        folder.display(),
        usage(sizes.limited())
    );
    for kind in KINDS {
        text.push_str(&format!("{}: {}\n", kind.id(), usage(sizes.of(kind))));
    }
    text.push_str(&format!("cache.unused_after: {unused}\n"));
    text
}

/// Removes `kinds`' entries, every limited kind's where none is named, and prints each
/// kind's line; an error once all are done if anything could not be removed.
fn clear(folder: &Path, kinds: &[Kind]) -> Result<()> {
    let every: Vec<Kind> = KINDS
        .into_iter()
        .filter(|kind| kind.policy() == Policy::Limited)
        .collect();
    let kinds = if kinds.is_empty() { &every } else { kinds };
    let cache = Cache::open(folder);
    let mut failed = 0;
    for &kind in kinds {
        match cache.clear(kind, &|_, _| {}, &|| false) {
            Ok(removed) => {
                print!("{}", cleared(kind, &removed));
                failed += removed.failed;
            }
            Err(error) => {
                println!("{}: cannot be listed: {error}", kind.id());
                failed += 1;
            }
        }
    }
    if failed > 0 {
        bail!("the cache in {} is not fully cleared", folder.display());
    }
    Ok(())
}

/// A kind's lines once cleared: what it removed, then what it could not.
fn cleared(kind: Kind, removed: &Removed) -> String {
    let done = Usage {
        entries: removed.entries,
        bytes: removed.bytes,
    };
    let mut text = format!("{}: removed {}\n", kind.id(), usage(done));
    if removed.failed > 0 {
        text.push_str(&format!(
            "{}: could not remove {} {}\n",
            kind.id(),
            removed.failed,
            entries(removed.failed)
        ));
    }
    text
}

/// Entries and their size: "2104 entries, 138.4 MB".
fn usage(usage: Usage) -> String {
    format!(
        "{} {}, {}",
        usage.entries,
        entries(usage.entries),
        size(usage.bytes)
    )
}

fn entries(count: u64) -> &'static str {
    if count == 1 { "entry" } else { "entries" }
}

/// `bytes` in kB, MB or GB, of 10³, 10⁶ and 10⁹ bytes as disks count them, to one decimal.
fn size(bytes: u64) -> String {
    let (unit, scale) = match bytes {
        1_000_000_000.. => ("GB", 1e9),
        1_000_000.. => ("MB", 1e6),
        _ => ("kB", 1e3),
    };
    format!("{:.1} {unit}", bytes as f64 / scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kept_or_unknown_kind_is_refused_by_name() {
        assert_eq!(limited_kind("thumbnails"), Ok(cache::THUMBNAILS));
        assert_eq!(limited_kind("pipelines"), Ok(cache::PIPELINES));
        let kept = limited_kind("libraries").unwrap_err();
        assert!(kept.contains("libraries are kept"), "{kept}");
        assert!(kept.contains("thumbnails, pipelines"), "{kept}");
        assert!(limited_kind("copies").is_err());
        assert!(
            limited_kind("previews")
                .unwrap_err()
                .starts_with("no such kind")
        );
    }

    #[test]
    fn sizes_read_in_the_unit_they_fit() {
        assert_eq!(size(0), "0.0 kB");
        assert_eq!(size(10_090), "10.1 kB");
        assert_eq!(size(138_412_032), "138.4 MB");
        assert_eq!(size(4_000_000_000), "4.0 GB");
        assert_eq!(
            usage(Usage {
                entries: 1,
                bytes: 1009
            }),
            "1 entry, 1.0 kB"
        );
    }
}
