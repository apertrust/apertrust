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

//! Thumbnails through the cache, with no GPU, each test in a folder of its own: a reading
//! of the test's own stands in for the library's, counting its calls by photo.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};

use apertrust_base::cache::{Cache, Limit};
use apertrust_base::jobs::Control;

use super::*;
use crate::session::cache::{Ask, CacheOptions};

const WAIT: Duration = Duration::from_secs(60);

/// How many times the test's reading read each photo, by its path.
static READS: Mutex<Option<HashMap<PathBuf, usize>>> = Mutex::new(None);

/// The test's reading of `photo`: a smooth picture of 64 by 48, none for `none.ARW`, and
/// for `broken.ARW` one whose pixels do not fill it, which cannot be encoded.
fn read(
    open: u64,
    folder: &Path,
    photo: &str,
    size: u32,
    observer: &(dyn Fn(LibraryEvent) + Send + Sync),
    _: &(dyn Fn() -> bool + Sync),
) {
    let mut reads = READS.lock().unwrap();
    *reads
        .get_or_insert_default()
        .entry(folder.join(photo))
        .or_default() += 1;
    drop(reads);
    let image = match photo {
        "none.ARW" => None,
        "broken.ARW" => Some(Thumbnail {
            width: 10,
            height: 10,
            pixels: vec![0; 4].into(),
        }),
        _ => Some(gradient()),
    };
    observer(LibraryEvent::Thumbnail {
        open,
        photo: photo.to_owned(),
        size,
        image,
    });
}

fn reads(path: &Path) -> usize {
    let reads = READS.lock().unwrap();
    reads
        .as_ref()
        .and_then(|r| r.get(path))
        .copied()
        .unwrap_or(0)
}

fn gradient() -> Thumbnail {
    let (width, height) = (64u32, 48u32);
    let pixels: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).flat_map(move |x| [(x * 4) as u8, (y * 5) as u8, 128, 255]))
        .collect();
    Thumbnail {
        width,
        height,
        pixels: pixels.into(),
    }
}

/// A folder of the test's own with a raw for each name, and a cache in it kept within
/// `limit`, its start's walk done.
struct Bench {
    folder: PathBuf,
    cache: Arc<Cache>,
    upkeep: Upkeep,
    asks: mpsc::Receiver<()>,
    _control: Control,
}

impl Bench {
    fn new(name: &str, photos: &[&str], limit: Limit) -> Bench {
        let folder =
            std::env::temp_dir().join(format!("apertrust-thumbnail-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        for photo in photos {
            std::fs::write(folder.join(photo), b"a raw").unwrap();
        }
        let cache = Arc::new(Cache::create(folder.join("cache")).unwrap());
        let control = Control::start(|_| {});
        let (asked, asks) = mpsc::channel();
        let ask: Ask = Arc::new(move || {
            let _ = asked.send(());
        });
        let options = CacheOptions {
            cache: Arc::clone(&cache),
            limit,
        };
        let upkeep = Upkeep::start(options, &control, ask, Arc::new(AtomicBool::new(false)));
        let deadline = Instant::now() + WAIT;
        while control.pending() > 0 {
            assert!(Instant::now() < deadline, "the start's walk ends in time");
            std::thread::sleep(Duration::from_millis(1));
        }
        Bench {
            folder,
            cache,
            upkeep,
            asks,
            _control: control,
        }
    }

    /// The events serving `photo` at 256 sends, through the cache with `tally`.
    fn serve(&self, photo: &str, tally: &Tally) -> Vec<LibraryEvent> {
        serve_with(Some((&self.upkeep, tally)), &self.folder, photo)
    }

    /// The thumbnails the cache holds.
    fn entries(&self) -> u64 {
        self.cache.walk(&|| false).unwrap().of(THUMBNAILS).entries
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

fn serve_with(cached: Option<(&Upkeep, &Tally)>, folder: &Path, photo: &str) -> Vec<LibraryEvent> {
    let events = Mutex::new(Vec::new());
    let tell = |event| events.lock().unwrap().push(event);
    let asked = Asked {
        open: 3,
        folder,
        photo,
        size: 256,
    };
    serve(cached, &asked, &tell, &|| false, read);
    events.into_inner().unwrap()
}

/// The one picture `events` holds.
fn picture(events: &[LibraryEvent]) -> Option<Thumbnail> {
    match events {
        [LibraryEvent::Thumbnail { image, .. }] => image.clone(),
        other => panic!("one thumbnail, not {other:?}"),
    }
}

/// A miss reads, sends, then writes; the next ask is a hit, not read again, its picture
/// the written JPEG's, within JPEG's loss of the one read.
#[test]
fn a_miss_is_written_and_the_next_ask_hits() {
    let bench = Bench::new("hit", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let tally = Tally::new(256);
    let first = picture(&bench.serve("A.ARW", &tally)).expect("a picture");
    assert_eq!(first, gradient());
    assert_eq!(reads(&path), 1);
    assert_eq!(tally.counts(), (0, 1, 1));
    assert_eq!(bench.entries(), 1);

    let second = picture(&bench.serve("A.ARW", &tally)).expect("a picture");
    assert_eq!(reads(&path), 1, "a hit reads nothing");
    assert_eq!(tally.counts(), (1, 1, 1));
    assert_eq!((second.width, second.height), (first.width, first.height));
    let jpeg = encode(&first).expect("it encodes");
    let decoded = apertrust_raw::read_jpeg(&jpeg).expect("it decodes");
    assert_eq!(
        *second.pixels, *decoded.pixels,
        "the JPEG written, read back"
    );
    let worst = first
        .pixels
        .iter()
        .zip(second.pixels.iter())
        .map(|(a, b)| a.abs_diff(*b))
        .max();
    assert!(worst <= Some(6), "{worst:?}");
}

/// A file changed has another version: the next ask misses, reads and writes again.
#[test]
fn a_changed_version_misses() {
    let bench = Bench::new("version", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let tally = Tally::new(256);
    bench.serve("A.ARW", &tally);
    std::fs::write(&path, b"the same raw, written again longer").unwrap();
    bench.serve("A.ARW", &tally);
    assert_eq!(reads(&path), 2);
    assert_eq!(tally.counts(), (0, 2, 2));
    // The slot's file replaced, not a second one beside it.
    assert_eq!(bench.entries(), 1);
}

/// With no cache, the library's reading alone, every time.
#[test]
fn with_no_cache_thumbnails_are_read_as_before() {
    let folder =
        std::env::temp_dir().join(format!("apertrust-thumbnail-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("A.ARW"), b"a raw").unwrap();
    for _ in 0..2 {
        assert_eq!(
            picture(&serve_with(None, &folder, "A.ARW")),
            Some(gradient())
        );
    }
    assert_eq!(reads(&folder.join("A.ARW")), 2);
    let items: Vec<_> = std::fs::read_dir(&folder).unwrap().flatten().collect();
    assert_eq!(items.len(), 1, "nothing written beside the raw");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A picture that cannot be encoded is sent and not kept, nor is no picture.
#[test]
fn what_cannot_be_encoded_or_is_none_is_sent_and_not_kept() {
    let bench = Bench::new("broken", &["broken.ARW", "none.ARW"], Limit::Bytes(1 << 30));
    let tally = Tally::new(256);
    let broken = picture(&bench.serve("broken.ARW", &tally)).expect("sent all the same");
    assert_eq!((broken.width, broken.height), (10, 10));
    assert_eq!(picture(&bench.serve("none.ARW", &tally)), None);
    assert_eq!(tally.counts(), (0, 2, 0));
    assert_eq!(bench.entries(), 0);
    // Read again each time, since nothing was kept.
    bench.serve("none.ARW", &tally);
    assert_eq!(reads(&bench.folder.join("none.ARW")), 2);
}

/// A write that takes the cache past its limit asks for an eviction; one within it does
/// not.
#[test]
fn a_write_past_the_limit_asks_for_an_eviction() {
    let within = Bench::new("within", &["A.ARW"], Limit::Bytes(1 << 30));
    within.serve("A.ARW", &Tally::new(256));
    assert!(within.asks.try_recv().is_err());

    let past = Bench::new("past", &["A.ARW"], Limit::Bytes(1000));
    past.serve("A.ARW", &Tally::new(256));
    assert_eq!(past.entries(), 1);
    assert!(past.asks.try_recv().is_ok(), "an eviction is asked for");
}

/// The cold walk's method (`docs/cache.md`, The walk's cost): the thumbnails of every raw
/// of the corpus written at 256, 512 and 1024 through the library's own reading, read back
/// as hits, their sizes told; then the cache filled to 100,000 entries and walked five
/// times. Run on its own, after `sudo purge` on a Mac for a cold first walk:
///
/// ```text
/// APERTRUST_CACHE_TIMING=DIR cargo test --release -p apertrust-engine --lib \
///     time_the_walk -- --ignored --nocapture
/// ```
///
/// `DIR` holds the cache, on the disk to time; the corpus is `APERTRUST_RAW_CORPUS` or
/// `test-images/raw.pixls.us`.
#[test]
#[ignore = "a measurement, on a corpus and a folder of the person's"]
fn time_the_walk() {
    const ENTRIES: u64 = 100_000;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let corpus = std::env::var_os("APERTRUST_RAW_CORPUS")
        .map_or_else(|| root.join("test-images/raw.pixls.us"), PathBuf::from);
    let folder = PathBuf::from(std::env::var_os("APERTRUST_CACHE_TIMING").expect("a folder"));
    let cache = folder.join("cache");
    if !cache.exists() {
        let bench_cache = Arc::new(Cache::create(&cache).unwrap());
        let control = Control::start(|_| {});
        let options = CacheOptions {
            cache: Arc::clone(&bench_cache),
            limit: Limit::Unlimited,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let upkeep = Upkeep::start(options, &control, Arc::new(|| {}), stop);
        let mut raws = Vec::new();
        let mut folders = vec![corpus];
        while let Some(folder) = folders.pop() {
            for item in std::fs::read_dir(&folder).unwrap().flatten() {
                let path = item.path();
                if path.is_dir() {
                    folders.push(path);
                } else if !path.extension().is_some_and(|e| e == "apxmp" || e == "xmp") {
                    raws.push(path);
                }
            }
        }
        let mut before = (0, 0);
        for size in [256, 512, 1024] {
            for pass in ["written", "read back"] {
                let tally = Tally::new(size);
                let started = Instant::now();
                for raw in &raws {
                    let (parent, name) = (raw.parent().unwrap(), raw.file_name().unwrap());
                    let asked = Asked {
                        open: 1,
                        folder: parent,
                        photo: name.to_str().unwrap(),
                        size,
                    };
                    let tell = |_| {};
                    serve(
                        Some((&upkeep, &tally)),
                        &asked,
                        &tell,
                        &|| false,
                        library::thumbnail,
                    );
                }
                let (hits, misses, written) = tally.counts();
                let took = started.elapsed().as_secs_f64();
                println!(
                    "{size}: {pass} in {took:.2} s: {hits} hits, {misses} read, {written} written"
                );
            }
            let sizes = bench_cache.walk(&|| false).unwrap().of(THUMBNAILS);
            let (entries, bytes) = (sizes.entries - before.0, sizes.bytes - before.1);
            before = (sizes.entries, sizes.bytes);
            let each = bytes as f64 / entries as f64 / 1e3;
            println!("{size}: {entries} entries, {each:.1} kB each");
        }
        drop(control);
        // Padded to the count, each a file of a byte in a subfolder of its own: a walk
        // reads no entry's contents.
        let have = bench_cache.walk(&|| false).unwrap().of(THUMBNAILS).entries;
        let kind = bench_cache.kind_folder(THUMBNAILS);
        for n in have..ENTRIES {
            let subfolder = kind.join(format!("{:02x}", n % 256));
            std::fs::create_dir_all(&subfolder).unwrap();
            std::fs::write(subfolder.join(format!("padding-{n}")), b"x").unwrap();
        }
    }
    let cache = Cache::open(&cache);
    for run in 1..=5 {
        let started = Instant::now();
        let sizes = cache.walk(&|| false).unwrap();
        let ms = started.elapsed().as_secs_f64() * 1e3;
        let entries = sizes.of(THUMBNAILS).entries;
        println!("walk {run}: {entries} entries in {ms:.1} ms");
    }
}
