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

//! The cache's tests, each in a temporary folder. Last use is set by hand, never waited
//! for.

use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

fn never() -> bool {
    false
}

fn quiet(_: u64, _: u64) {}

/// A cache in a folder of its own, the folder kept alive beside it.
fn cache() -> (tempfile::TempDir, Cache) {
    let folder = tempfile::tempdir().expect("a folder");
    let cache = Cache::open(folder.path().join("cache"));
    (folder, cache)
}

/// Every file under `folder`, its path from there, sorted.
fn files(folder: &Path) -> Vec<String> {
    fn under(base: &Path, folder: &Path, found: &mut Vec<String>) {
        let Ok(listing) = fs::read_dir(folder) else {
            return;
        };
        for item in listing {
            let path = item.unwrap().path();
            if path.is_dir() {
                under(base, &path, found);
            } else {
                let relative = path.strip_prefix(base).unwrap();
                found.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut found = Vec::new();
    under(folder, folder, &mut found);
    found.sort();
    found
}

fn used(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

fn set_used(path: &Path, time: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// How many subfolders `kinds` hold: the questions a listing asks its cancel before its
/// first removal.
fn subfolders(cache: &Cache, kinds: &[Kind]) -> u64 {
    kinds
        .iter()
        .map(|kind| cache.subfolders(*kind).unwrap().len() as u64)
        .sum()
}

/// A cancel that answers true from its `after`-th question on.
fn cancel_after(after: u64) -> impl Fn() -> bool + Sync {
    let asked = AtomicU64::new(0);
    move || asked.fetch_add(1, Ordering::Relaxed) >= after
}

#[test]
fn the_kinds_are_the_decided_ones_each_in_its_own_folder() {
    let ids: Vec<_> = KINDS.iter().map(|kind| kind.id()).collect();
    assert_eq!(ids, ["thumbnails", "pipelines", "libraries", "copies"]);
    for (i, kind) in KINDS.iter().enumerate() {
        assert_eq!(kind.index, i);
        assert!(
            KINDS[..i]
                .iter()
                .all(|other| other.folder() != kind.folder())
        );
    }
    assert_eq!(THUMBNAILS.policy(), Policy::Limited);
    assert_eq!(PIPELINES.policy(), Policy::Limited);
    assert_eq!(LIBRARIES.policy(), Policy::Kept);
    assert_eq!(COPIES.policy(), Policy::Kept);
}

/// A program that writes the cache creates its folder, as its first write, and leaves
/// nothing of its check in it; a folder that cannot be made is the error.
#[test]
fn a_cache_created_has_its_folder_and_one_that_cannot_be_made_is_an_error() {
    let folder = tempfile::tempdir().expect("a folder");
    let path = folder.path().join("deep").join("cache");
    let cache = Cache::create(&path).expect("the cache's folder is made");
    assert_eq!(cache.folder(), path);
    assert!(path.is_dir());
    assert_eq!(files(&path), Vec::<String>::new());
    // Again, on the folder that is there now.
    Cache::create(&path).expect("the same folder again");
    // A file where the folder should be, on every platform.
    let file = folder.path().join("a file");
    fs::write(&file, b"").unwrap();
    assert!(Cache::create(&file).is_err());
    assert!(Cache::create(file.join("cache")).is_err());
}

/// A folder that is there but cannot be written is the error too.
#[cfg(unix)]
#[test]
fn a_cache_folder_that_cannot_be_written_is_an_error() {
    let folder = tempfile::tempdir().expect("a folder");
    let path = folder.path().join("cache");
    fs::create_dir(&path).unwrap();
    with_mode(&path, 0o555, || assert!(Cache::create(&path).is_err()));
}

#[test]
fn a_slots_name_is_its_fnv_hash_in_a_subfolder_of_its_first_byte() {
    // FNV-1a's published values: the name never changes between builds.
    assert_eq!(hash(b""), 0x6c62272e07bb014262b821756295c58d);
    assert_eq!(hash(b"a"), 0xd228cb696f1a8caf78912b704e4a8964);
    // Its name, least significant byte first.
    assert_eq!(name(b"a"), "64894a4e702b9178af8c1a6f69cb28d2");
    let (_folder, cache) = cache();
    assert_eq!(
        cache.entry_path(THUMBNAILS, b"a"),
        cache
            .folder()
            .join("thumbnails")
            .join("64")
            .join("64894a4e702b9178af8c1a6f69cb28d2")
    );
    // Slots alike in all but their last byte each have a subfolder of their own.
    let subfolders: std::collections::HashSet<_> = (0..=255u8)
        .map(|last| {
            name(&[b"fingerprint, copy 0, size ".as_slice(), &[last]].concat())[..2].to_owned()
        })
        .collect();
    assert_eq!(subfolders.len(), 256);
}

#[test]
fn an_entry_written_reads_back() {
    let (_folder, cache) = cache();
    assert_eq!(cache.read(THUMBNAILS, b"photo", b"v1").unwrap(), None);
    cache.write(THUMBNAILS, b"photo", b"v1", b"pixels").unwrap();
    assert_eq!(
        cache.read(THUMBNAILS, b"photo", b"v1").unwrap().as_deref(),
        Some(b"pixels".as_slice())
    );
    // Another slot, or another kind's same slot, is not it.
    assert_eq!(cache.read(THUMBNAILS, b"other", b"v1").unwrap(), None);
    assert_eq!(cache.read(PIPELINES, b"photo", b"v1").unwrap(), None);
    // An empty entry is an entry.
    cache.write(PIPELINES, b"adapter", b"", b"").unwrap();
    assert_eq!(
        cache.read(PIPELINES, b"adapter", b"").unwrap().as_deref(),
        Some(b"".as_slice())
    );
}

#[test]
fn another_version_reads_as_none() {
    let (_folder, cache) = cache();
    cache
        .write(THUMBNAILS, b"photo", b"edit 1", b"pixels")
        .unwrap();
    assert_eq!(cache.read(THUMBNAILS, b"photo", b"edit 2").unwrap(), None);
    assert_eq!(cache.read(THUMBNAILS, b"photo", b"edit").unwrap(), None);
    assert_eq!(cache.read(THUMBNAILS, b"photo", b"edit 10").unwrap(), None);
    // A file that is no entry, or is cut short, is none too.
    let path = cache.entry_path(THUMBNAILS, b"photo");
    for broken in [
        &b""[..],
        b"APC1",
        b"APC1\x06\0\0\0edit",
        b"XXXX\x06\0\0\0edit 1px",
    ] {
        fs::write(&path, broken).unwrap();
        assert_eq!(cache.read(THUMBNAILS, b"photo", b"edit 1").unwrap(), None);
    }
}

#[test]
fn a_rewrite_replaces_the_stale_file() {
    let (folder, cache) = cache();
    cache
        .write(THUMBNAILS, b"photo", b"edit 1", b"old")
        .unwrap();
    cache
        .write(THUMBNAILS, b"photo", b"edit 2", b"newer")
        .unwrap();
    assert_eq!(cache.read(THUMBNAILS, b"photo", b"edit 1").unwrap(), None);
    assert_eq!(
        cache
            .read(THUMBNAILS, b"photo", b"edit 2")
            .unwrap()
            .as_deref(),
        Some(b"newer".as_slice())
    );
    // One file, and no temporary beside it.
    let name = name(b"photo");
    assert_eq!(
        files(folder.path()),
        [format!("cache/thumbnails/{}/{name}", &name[..2])]
    );
}

#[test]
fn a_kept_kind_has_no_entries_of_the_caches_own() {
    let (folder, cache) = cache();
    for kind in [LIBRARIES, COPIES] {
        let error = cache.write(kind, b"slot", b"v", b"x").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        let error = cache.read(kind, b"slot", b"v").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(cache.kind_folder(kind), cache.folder().join(kind.folder()));
    }
    assert!(files(folder.path()).is_empty());
}

#[test]
fn a_read_sets_last_use_at_most_once_a_day() {
    let (_folder, cache) = cache();
    cache.write(THUMBNAILS, b"photo", b"v", b"pixels").unwrap();
    let path = cache.entry_path(THUMBNAILS, b"photo");
    let now = SystemTime::now();

    // Used an hour ago: a read leaves it.
    let hour_ago = now - Duration::from_secs(60 * 60);
    set_used(&path, hour_ago);
    assert!(
        cache
            .read_at(THUMBNAILS, b"photo", b"v", now)
            .unwrap()
            .is_some()
    );
    assert_eq!(used(&path), hour_ago);

    // Just under a day: still left.
    let almost = now - (DAY - Duration::from_secs(60));
    set_used(&path, almost);
    assert!(
        cache
            .read_at(THUMBNAILS, b"photo", b"v", now)
            .unwrap()
            .is_some()
    );
    assert_eq!(used(&path), almost);

    // Two days ago: the read makes it now.
    set_used(&path, now - 2 * DAY);
    assert!(
        cache
            .read_at(THUMBNAILS, b"photo", b"v", now)
            .unwrap()
            .is_some()
    );
    assert_eq!(used(&path), now);

    // A read of another version uses nothing.
    set_used(&path, now - 2 * DAY);
    assert!(
        cache
            .read_at(THUMBNAILS, b"photo", b"w", now)
            .unwrap()
            .is_none()
    );
    assert_eq!(used(&path), now - 2 * DAY);
}

#[test]
fn a_write_sets_last_use_even_with_the_same_bytes() {
    let (_folder, cache) = cache();
    cache.write(THUMBNAILS, b"photo", b"v", b"pixels").unwrap();
    let path = cache.entry_path(THUMBNAILS, b"photo");
    let long_ago = SystemTime::now() - 30 * DAY;
    set_used(&path, long_ago);
    cache.write(THUMBNAILS, b"photo", b"v", b"pixels").unwrap();
    assert!(used(&path) > long_ago + 29 * DAY);
}

#[test]
fn the_walk_gives_each_kinds_entries_and_bytes() {
    let (_folder, cache) = cache();
    assert_eq!(cache.sizes(), None);
    // An empty or missing cache is nothing, and the walk makes no folder.
    assert_eq!(cache.walk(&never).unwrap(), Sizes::default());
    assert!(!cache.folder().exists());

    for (slot, bytes) in [(&b"a"[..], 10), (b"b", 20), (b"c", 30)] {
        cache
            .write(THUMBNAILS, slot, b"v", &vec![0; bytes])
            .unwrap();
    }
    cache.write(PIPELINES, b"adapter", b"v", &[0; 100]).unwrap();
    let library = cache.kind_folder(LIBRARIES).join("0f3a");
    fs::create_dir_all(library.join("index")).unwrap();
    fs::write(library.join("library.toml"), [0; 7]).unwrap();
    fs::write(library.join("index/terms"), [0; 1000]).unwrap();
    fs::create_dir_all(cache.kind_folder(COPIES)).unwrap();
    fs::write(cache.kind_folder(COPIES).join("raw"), [0; 5]).unwrap();

    // Temporaries, one left by a crash two days ago and one being written, are uncounted.
    let subfolder = cache
        .entry_path(THUMBNAILS, b"a")
        .parent()
        .unwrap()
        .to_owned();
    let stale = subfolder.join(".~abc.AbC123.tmp");
    let fresh = subfolder.join(".~abc.XyZ789.tmp");
    fs::write(&stale, [0; 50]).unwrap();
    fs::write(&fresh, [0; 50]).unwrap();
    set_used(&stale, SystemTime::now() - 2 * DAY);

    let header = (MAGIC.len() + 4 + 1) as u64;
    let sizes = cache.walk(&never).unwrap();
    assert_eq!(
        sizes.of(THUMBNAILS),
        Usage {
            entries: 3,
            bytes: 60 + 3 * header
        }
    );
    assert_eq!(
        sizes.of(PIPELINES),
        Usage {
            entries: 1,
            bytes: 100 + header
        }
    );
    assert_eq!(
        sizes.of(LIBRARIES),
        Usage {
            entries: 1,
            bytes: 1007
        }
    );
    assert_eq!(
        sizes.of(COPIES),
        Usage {
            entries: 1,
            bytes: 5
        }
    );
    assert_eq!(
        sizes.limited(),
        Usage {
            entries: 4,
            bytes: 160 + 4 * header
        }
    );
    assert_eq!(cache.sizes(), Some(sizes));

    // The walk writes nothing: the stale temporary stays until a tidy removes it.
    assert!(stale.exists());
    assert_eq!(cache.tidy(&never).unwrap(), 1);
    assert!(!stale.exists());
    assert!(fresh.exists());
    assert_eq!(cache.tidy(&never).unwrap(), 0);
    assert_eq!(cache.walk(&never).unwrap(), sizes);
}

#[test]
fn the_handle_counts_its_own_writes_and_removals_after_the_walk() {
    let (_folder, cache) = cache();
    let header = (MAGIC.len() + 4 + 1) as u64;
    // Before a walk nothing is counted: the walk finds it.
    cache.write(THUMBNAILS, b"a", b"v", &[0; 10]).unwrap();
    assert_eq!(cache.sizes(), None);
    cache.walk(&never).unwrap();
    let thumbnails = || cache.sizes().unwrap().of(THUMBNAILS);
    assert_eq!(
        thumbnails(),
        Usage {
            entries: 1,
            bytes: 10 + header
        }
    );

    // A new slot is an entry more; a rewrite only changes the bytes.
    cache.write(THUMBNAILS, b"b", b"v", &[0; 20]).unwrap();
    assert_eq!(
        thumbnails(),
        Usage {
            entries: 2,
            bytes: 30 + 2 * header
        }
    );
    cache.write(THUMBNAILS, b"a", b"w", &[0; 5]).unwrap();
    assert_eq!(
        thumbnails(),
        Usage {
            entries: 2,
            bytes: 25 + 2 * header
        }
    );
    assert_eq!(cache.sizes().unwrap().of(PIPELINES), Usage::default());

    // The counts agree with the next walk.
    let counted = cache.sizes().unwrap();
    assert_eq!(cache.walk(&never).unwrap(), counted);

    cache.clear(THUMBNAILS, &quiet, &never).unwrap();
    assert_eq!(thumbnails(), Usage::default());
}

/// Six limited entries of one size, thumbnails and a pipeline, each used a day apart, the
/// oldest first in `slots`, and a large kept library.
fn six_entries(cache: &Cache) -> (Vec<(Kind, &'static [u8])>, u64) {
    let slots: Vec<(Kind, &'static [u8])> = vec![
        (THUMBNAILS, b"1"),
        (PIPELINES, b"2"),
        (THUMBNAILS, b"3"),
        (THUMBNAILS, b"4"),
        (THUMBNAILS, b"5"),
        (THUMBNAILS, b"6"),
    ];
    for (kind, slot) in &slots {
        cache.write(*kind, slot, b"v", &[0; 100]).unwrap();
    }
    age(cache, &slots);
    let library = cache.kind_folder(LIBRARIES).join("0f3a");
    fs::create_dir_all(&library).unwrap();
    fs::write(library.join("index"), vec![0; 100_000]).unwrap();
    fs::create_dir_all(cache.kind_folder(COPIES)).unwrap();
    fs::write(cache.kind_folder(COPIES).join("raw"), vec![0; 100_000]).unwrap();
    let size = (MAGIC.len() + 4 + 1 + 100) as u64;
    (slots, size)
}

/// Sets the last use of those of `slots` still there a day apart, the first ten days ago,
/// as reads that set it to now leave them otherwise.
fn age(cache: &Cache, slots: &[(Kind, &'static [u8])]) {
    let now = SystemTime::now();
    for (days, (kind, slot)) in (0..).zip(slots) {
        let path = cache.entry_path(*kind, slot);
        if path.exists() {
            set_used(&path, now - (10 - days) * DAY);
        }
    }
}

fn present(cache: &Cache, slots: &[(Kind, &'static [u8])]) -> Vec<&'static [u8]> {
    slots
        .iter()
        .filter(|(kind, slot)| cache.read(*kind, slot, b"v").unwrap().is_some())
        .map(|(_, slot)| *slot)
        .collect()
}

#[test]
fn eviction_waits_for_the_limit_then_removes_the_oldest_across_kinds_to_ninety_percent() {
    let (_folder, cache) = cache();
    let (slots, size) = six_entries(&cache);
    cache.walk(&never).unwrap();

    // At the limit, or under it, nothing goes.
    for limit in [6 * size, 6 * size + 1, 10 * size] {
        assert_eq!(
            cache.evict(Limit::Bytes(limit), &never).unwrap(),
            Removed::default()
        );
    }
    assert_eq!(present(&cache, &slots).len(), 6);
    age(&cache, &slots);

    // A byte past it: the oldest goes, which leaves five under 90% of the limit.
    let removed = cache.evict(Limit::Bytes(6 * size - 1), &never).unwrap();
    assert_eq!(removed.entries, 1);
    assert_eq!(present(&cache, &slots), [b"2", b"3", b"4", b"5", b"6"]);
    age(&cache, &slots);

    // A limit of four entries: 90% of it holds three, so the two oldest go, the pipeline
    // among them.
    let removed = cache.evict(Limit::Bytes(4 * size), &never).unwrap();
    assert_eq!(
        removed,
        Removed {
            entries: 2,
            bytes: 2 * size,
            failed: 0,
            cancelled: false
        }
    );
    assert_eq!(present(&cache, &slots), [b"4", b"5", b"6"]);
    assert_eq!(
        cache.sizes().unwrap().limited(),
        Usage {
            entries: 3,
            bytes: 3 * size
        }
    );
}

#[test]
fn eviction_takes_its_removals_from_the_counts_and_keeps_writes_made_after() {
    let (_folder, cache) = cache();
    let (_, size) = six_entries(&cache);
    cache.walk(&never).unwrap();
    let removed = cache.evict(Limit::Bytes(4 * size), &never).unwrap();
    assert_eq!(removed.entries, 3);
    cache.write(THUMBNAILS, b"7", b"v", &[0; 100]).unwrap();
    assert_eq!(
        cache.sizes().unwrap().limited(),
        Usage {
            entries: 4,
            bytes: 4 * size
        }
    );
    assert_eq!(Some(cache.walk(&never).unwrap()), cache.sizes());
}

/// Gives `folder` the permissions `mode` while `test` runs, then the usual ones back; false,
/// and nothing run, where the process can write in it anyway, as root can.
#[cfg(unix)]
fn with_mode(folder: &Path, mode: u32, test: impl FnOnce()) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(folder, fs::Permissions::from_mode(mode)).unwrap();
    let probe = folder.join("probe");
    let refused = File::create(&probe).is_err();
    if refused {
        test();
    } else {
        let _ = fs::remove_file(&probe);
    }
    fs::set_permissions(folder, fs::Permissions::from_mode(0o755)).unwrap();
    refused
}

#[cfg(unix)]
#[test]
fn an_entry_that_cannot_be_removed_is_passed_over_and_still_counted() {
    let (_folder, cache) = cache();
    let (slots, size) = six_entries(&cache);
    cache.walk(&never).unwrap();
    let subfolder = cache
        .entry_path(THUMBNAILS, b"1")
        .parent()
        .unwrap()
        .to_owned();
    let alone = slots[1..]
        .iter()
        .all(|(kind, slot)| cache.entry_path(*kind, slot).parent() != Some(&subfolder));
    assert!(
        alone,
        "the test wants the oldest entry alone in its subfolder"
    );

    with_mode(&subfolder, 0o555, || {
        // The oldest stays and still counts, so three more go to reach 90% of four.
        let removed = cache.evict(Limit::Bytes(4 * size), &never).unwrap();
        assert_eq!(
            removed,
            Removed {
                entries: 3,
                bytes: 3 * size,
                failed: 1,
                cancelled: false
            }
        );
        assert_eq!(
            cache.sizes().unwrap().limited(),
            Usage {
                entries: 3,
                bytes: 3 * size
            }
        );
        // Clear passes over it too.
        let told = Mutex::new(Vec::new());
        let progress = |done, total| told.lock().unwrap().push((done, total));
        let removed = cache.clear(THUMBNAILS, &progress, &never).unwrap();
        assert_eq!(
            removed,
            Removed {
                entries: 2,
                bytes: 2 * size,
                failed: 1,
                cancelled: false
            }
        );
        assert_eq!(told.lock().unwrap().last(), Some(&(3, 3)));
        assert_eq!(
            cache.sizes().unwrap().of(THUMBNAILS),
            Usage {
                entries: 1,
                bytes: size
            }
        );
        assert_eq!(present(&cache, &slots), [b"1"]);
    });
}

#[cfg(unix)]
#[test]
fn an_unreadable_subfolder_is_passed_over() {
    let (_folder, cache) = cache();
    let (slots, size) = six_entries(&cache);
    let subfolder = cache
        .entry_path(THUMBNAILS, b"1")
        .parent()
        .unwrap()
        .to_owned();
    let elsewhere = slots
        .iter()
        .filter(|(kind, slot)| cache.entry_path(*kind, slot).parent() != Some(&subfolder))
        .count() as u64;
    with_mode(&subfolder, 0o000, || {
        let sizes = cache.walk(&never).unwrap();
        assert_eq!(
            sizes.limited(),
            Usage {
                entries: elsewhere,
                bytes: elsewhere * size
            }
        );
        assert_eq!(sizes.of(LIBRARIES).bytes, 100_000);
        assert_eq!(cache.tidy(&never).unwrap(), 0);
        let removed = cache.evict(Limit::Bytes(1), &never).unwrap();
        assert_eq!((removed.entries, removed.failed), (elsewhere, 0));
        assert_eq!(
            cache.clear(THUMBNAILS, &quiet, &never).unwrap(),
            Removed::default()
        );
    });
}

#[test]
fn two_handles_writing_evicting_and_clearing_at_once_never_read_a_part() {
    let folder = tempfile::tempdir().expect("a folder");
    let one = Cache::open(folder.path().join("cache"));
    let other = Cache::open(folder.path().join("cache"));
    let slots: [&[u8]; 3] = [b"a", b"b", b"c"];
    let bytes = |version: u8| vec![version; 20_000];
    let rounds = 40u8;
    std::thread::scope(|scope| {
        for handle in [&one, &other] {
            scope.spawn(move || {
                for version in 0..rounds {
                    for slot in slots {
                        let written = handle.write(THUMBNAILS, slot, &[version], &bytes(version));
                        // Windows may refuse a rename over a file another thread reads.
                        assert!(written.is_ok() || cfg!(windows), "{written:?}");
                    }
                }
            });
        }
        scope.spawn(|| {
            for round in 0..rounds {
                other.walk(&never).unwrap();
                other.evict(Limit::Bytes(30_000), &never).unwrap();
                if round % 10 == 0 {
                    other.clear(THUMBNAILS, &quiet, &never).unwrap();
                }
            }
        });
        scope.spawn(|| {
            for round in 0..u32::from(rounds) * 5 {
                let version = (round % u32::from(rounds)) as u8;
                for slot in slots {
                    match one.read(THUMBNAILS, slot, &[version]) {
                        Ok(Some(found)) => assert!(found == bytes(version), "a part read"),
                        Ok(None) => {}
                        // Windows may refuse to open a file another thread removes.
                        Err(error) if cfg!(windows) => drop(error),
                        Err(error) => panic!("{error}"),
                    }
                }
            }
        });
    });
}

#[test]
fn kept_kinds_are_never_evicted() {
    let (_folder, cache) = cache();
    let (slots, _) = six_entries(&cache);
    let removed = cache.evict(Limit::Bytes(1), &never).unwrap();
    assert_eq!(removed.entries, 6);
    assert!(present(&cache, &slots).is_empty());
    let sizes = cache.walk(&never).unwrap();
    assert_eq!(sizes.limited(), Usage::default());
    assert_eq!(
        sizes.of(LIBRARIES),
        Usage {
            entries: 1,
            bytes: 100_000
        }
    );
    assert_eq!(
        sizes.of(COPIES),
        Usage {
            entries: 1,
            bytes: 100_000
        }
    );
}

#[test]
fn an_unlimited_cache_evicts_nothing() {
    let (_folder, cache) = cache();
    let (slots, _) = six_entries(&cache);
    assert_eq!(
        cache.evict(Limit::Unlimited, &never).unwrap(),
        Removed::default()
    );
    assert_eq!(present(&cache, &slots).len(), 6);
}

#[test]
fn eviction_stops_at_a_cancel() {
    let (_folder, cache) = cache();
    let (slots, size) = six_entries(&cache);
    let listed = subfolders(&cache, &[THUMBNAILS, PIPELINES]);
    let removed = cache
        .evict(Limit::Bytes(1), &cancel_after(listed + 2))
        .unwrap();
    assert_eq!(
        removed,
        Removed {
            entries: 2,
            bytes: 2 * size,
            failed: 0,
            cancelled: true
        }
    );
    assert_eq!(present(&cache, &slots), [b"3", b"4", b"5", b"6"]);
}

#[test]
fn clear_removes_one_kinds_entries_and_tells_its_progress() {
    let (_folder, cache) = cache();
    let (slots, size) = six_entries(&cache);
    cache.walk(&never).unwrap();
    let told = Mutex::new(Vec::new());
    let progress = |done, total| told.lock().unwrap().push((done, total));
    let removed = cache.clear(THUMBNAILS, &progress, &never).unwrap();
    assert_eq!(
        removed,
        Removed {
            entries: 5,
            bytes: 5 * size,
            failed: 0,
            cancelled: false
        }
    );
    assert_eq!(
        *told.lock().unwrap(),
        [(0, 5), (1, 5), (2, 5), (3, 5), (4, 5), (5, 5)]
    );
    // The pipeline and the kept kinds are as they were.
    assert_eq!(present(&cache, &slots), [b"2"]);
    let sizes = cache.sizes().unwrap();
    assert_eq!(sizes.of(THUMBNAILS), Usage::default());
    assert_eq!(
        sizes.of(PIPELINES),
        Usage {
            entries: 1,
            bytes: size
        }
    );
    assert_eq!(sizes.of(LIBRARIES).bytes, 100_000);
    assert_eq!(cache.walk(&never).unwrap(), sizes);

    // A kept kind is not cleared here.
    let error = cache.clear(LIBRARIES, &quiet, &never).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert_eq!(cache.walk(&never).unwrap().of(LIBRARIES).entries, 1);
}

#[test]
fn clear_stops_at_a_cancel() {
    let (_folder, cache) = cache();
    let (slots, size) = six_entries(&cache);
    let removed = cache.clear(THUMBNAILS, &quiet, &cancel_after(0)).unwrap();
    assert_eq!(
        removed,
        Removed {
            entries: 0,
            bytes: 0,
            failed: 0,
            cancelled: true
        }
    );
    assert_eq!(present(&cache, &slots).len(), 6);
    let listed = subfolders(&cache, &[THUMBNAILS]);
    let removed = cache
        .clear(THUMBNAILS, &quiet, &cancel_after(listed + 3))
        .unwrap();
    assert_eq!(
        removed,
        Removed {
            entries: 3,
            bytes: 3 * size,
            failed: 0,
            cancelled: true
        }
    );
    assert_eq!(present(&cache, &slots).len(), 3);
}

#[test]
fn an_entry_removed_by_another_process_is_passed_over() {
    let (_folder, cache) = cache();
    cache.write(THUMBNAILS, b"a", b"v", b"x").unwrap();
    let path = cache.entry_path(THUMBNAILS, b"a");
    assert!(remove(&path).unwrap());
    assert!(!remove(&path).unwrap());
    assert_eq!(cache.read(THUMBNAILS, b"a", b"v").unwrap(), None);
}

/// Every listing stops at a cancel before its next subfolder: a cancelled walk keeps
/// nothing, and a cancelled tidy or scan removes nothing, so a program that quits waits for
/// none of them.
#[test]
fn a_walk_a_tidy_and_an_eviction_s_scan_stop_at_a_cancel() {
    let (_folder, cache) = cache();
    let (slots, _) = six_entries(&cache);
    let always = || true;
    let error = cache.walk(&always).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(cache.sizes(), None);
    let error = cache.tidy(&always).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    let removed = cache.evict(Limit::Bytes(1), &always).unwrap();
    assert_eq!(
        removed,
        Removed {
            cancelled: true,
            ..Removed::default()
        }
    );
    assert_eq!(present(&cache, &slots).len(), 6);
    // Stopped partway, after its first subfolder.
    let error = cache.walk(&cancel_after(1)).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(cache.sizes(), None);
    assert!(cache.walk(&never).is_ok());
}

/// The tidy removes the write checks a crash left in the cache folder, any process's,
/// and nothing else there.
#[test]
fn the_tidy_removes_write_checks_left_behind() {
    let (_folder, cache) = cache();
    fs::create_dir_all(cache.folder()).unwrap();
    let left = cache.folder().join(".apertrust-write-check-4242.tmp");
    fs::write(&left, b"").unwrap();
    let others = [
        ".apertrust-write-check-.tmp",
        ".apertrust-write-check-12a.tmp",
        "apertrust-write-check-1.tmp",
        "notes.tmp",
    ];
    for name in others {
        fs::write(cache.folder().join(name), b"").unwrap();
    }
    assert_eq!(cache.tidy(&never).unwrap(), 1);
    assert!(!left.exists());
    for name in others {
        assert!(cache.folder().join(name).exists(), "{name}");
    }
}
