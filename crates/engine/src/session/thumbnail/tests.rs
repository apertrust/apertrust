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

/// The test's reading of `photo`: a smooth picture of 64 by 48, one of 511 by 383 for
/// `odd.ARW`, none for `none.ARW`, and for `broken.ARW` one whose pixels do not fill it,
/// which cannot be encoded.
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
        "odd.ARW" => Some(gradient_of(511, 383)),
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
    gradient_of(64, 48)
}

/// A smooth picture of `width` by `height`: red across, green down.
fn gradient_of(width: u32, height: u32) -> Thumbnail {
    let level = |at: u32, of: u32| (at * 255 / (of - 1).max(1)) as u8;
    let pixels: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).flat_map(move |x| [level(x, width), level(y, height), 128, 255]))
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
        self.cache
            .walk(&|| false)
            .unwrap()
            .unwrap()
            .of(THUMBNAILS)
            .entries
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

/// The largest difference of one channel between two pictures of one size.
fn worst(a: &Thumbnail, b: &Thumbnail) -> u8 {
    assert_eq!((a.width, a.height), (b.width, b.height));
    let pairs = a.pixels.iter().zip(b.pixels.iter());
    pairs.map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
}

/// The picture goes out before anything is written: the cache holds nothing as the
/// client is told, and the entry once the job has ended.
#[test]
fn the_picture_is_sent_before_it_is_written() {
    let bench = Bench::new("first", &["A.ARW"], Limit::Bytes(1 << 30));
    let told = Mutex::new(0);
    let cache = Arc::clone(&bench.cache);
    let tell = |_: LibraryEvent| {
        let entries = cache
            .walk(&|| false)
            .unwrap()
            .unwrap()
            .of(THUMBNAILS)
            .entries;
        assert_eq!(entries, 0, "nothing written before the picture is sent");
        *told.lock().unwrap() += 1;
    };
    let asked = Asked {
        open: 3,
        folder: &bench.folder,
        photo: "A.ARW",
        size: 256,
    };
    let tally = Tally::new(256);
    serve(
        Some((&bench.upkeep, &tally)),
        &asked,
        &tell,
        &|| false,
        read,
    );
    assert_eq!(*told.lock().unwrap(), 1);
    assert_eq!(bench.entries(), 1);
}

/// A cancel after the picture is read writes nothing.
#[test]
fn a_cancel_after_the_read_writes_nothing() {
    let bench = Bench::new("cancelled", &["A.ARW"], Limit::Bytes(1 << 30));
    let read_by_now = AtomicBool::new(false);
    let tell = |_: LibraryEvent| read_by_now.store(true, std::sync::atomic::Ordering::Relaxed);
    let cancel = || read_by_now.load(std::sync::atomic::Ordering::Relaxed);
    let asked = Asked {
        open: 3,
        folder: &bench.folder,
        photo: "A.ARW",
        size: 256,
    };
    let tally = Tally::new(256);
    serve(Some((&bench.upkeep, &tally)), &asked, &tell, &cancel, read);
    assert!(read_by_now.into_inner());
    assert_eq!(bench.entries(), 0);
    assert_eq!(tally.counts(), (0, 1, 0));
}

/// A picture of odd sides, which 4:2:2's halved chroma does not divide, comes back whole.
#[test]
fn an_odd_sized_picture_comes_back_whole() {
    let bench = Bench::new("odd", &["odd.ARW"], Limit::Bytes(1 << 30));
    let tally = Tally::new(512);
    let first = picture(&bench.serve("odd.ARW", &tally)).expect("a picture");
    let second = picture(&bench.serve("odd.ARW", &tally)).expect("a picture");
    assert_eq!(tally.counts(), (1, 1, 1));
    assert_eq!((second.width, second.height), (511, 383));
    let worst = worst(&first, &second);
    assert!(worst <= 6, "{worst}");
}

/// An entry at the right version that is not a JPEG, or one cut short, which zune-jpeg
/// would read without a word, is a miss: read again and written over.
#[test]
fn a_damaged_entry_is_read_again_and_written_over() {
    let bench = Bench::new("damaged", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let key = library::thumbnail_key(&bench.folder, "A.ARW", 256).expect("a key");
    let jpeg = encode(&gradient()).expect("it encodes");
    let cut = jpeg[..jpeg.len() * 2 / 3].to_vec();
    for (damage, entry) in [
        ("garbage", b"garbage at the right version".to_vec()),
        ("cut", cut),
    ] {
        let before = reads(&path);
        let cache = bench.upkeep.cache();
        cache
            .write(THUMBNAILS, &key.slot, &key.version, &entry)
            .unwrap();
        let tally = Tally::new(256);
        assert_eq!(
            picture(&bench.serve("A.ARW", &tally)),
            Some(gradient()),
            "{damage}"
        );
        assert_eq!(reads(&path), before + 1, "{damage}: read again");
        assert_eq!(tally.counts(), (0, 1, 1), "{damage}: written over");
        // Whole again: the next ask hits.
        bench.serve("A.ARW", &tally);
        assert_eq!(tally.counts(), (1, 1, 1), "{damage}");
    }
}

/// A tripwire: zune-jpeg, rawler's decoder, still misreads the export's own 4:2:0 with its
/// optimised tables, a scan for each component, which is why thumbnails are kept at 4:2:2.
/// Once it fails, zune-jpeg reads it right, and the thumbnails may move to 4:2:0.
#[test]
fn zune_jpeg_still_misreads_the_export_s_4_2_0_so_thumbnails_stay_4_2_2() {
    let image = gradient_of(64, 48);
    let pixels: Vec<[f32; 4]> = image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p.map(|v| f32::from(v) / 255.0))
        .collect();
    let mut jpeg = Vec::new();
    let options = Options {
        subsample: Subsample::S420,
        ..OPTIONS
    };
    let source = Image::new(&pixels, image.width, image.height).unwrap();
    jpeg::write(source, &options, &mut jpeg).unwrap();
    let misread = match apertrust_raw::read_jpeg(&jpeg) {
        Err(_) => true,
        Ok(read) => {
            let read = Thumbnail {
                width: read.width,
                height: read.height,
                pixels: read.pixels.into(),
            };
            worst(&image, &read) > 32
        }
    };
    assert!(
        misread,
        "zune-jpeg now reads the export's 4:2:0: thumbnails may use it"
    );
    // 4:2:2, as kept, reads right.
    let kept = apertrust_raw::read_jpeg(&encode(&image).unwrap()).unwrap();
    let kept = Thumbnail {
        width: kept.width,
        height: kept.height,
        pixels: kept.pixels.into(),
    };
    assert!(worst(&image, &kept) <= 6);
}
