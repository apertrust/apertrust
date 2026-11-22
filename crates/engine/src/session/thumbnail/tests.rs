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

/// How many times the test's facts were read, by the raw's path.
static FACT_READS: Mutex<Option<HashMap<PathBuf, usize>>> = Mutex::new(None);

/// The test's raws' facts for darktable's rule, of a raw of 6048 by 4024: the embedded
/// picture `read` gives, but 6000 by 4000 for `large.ARW`; none for `none.ARW`; and
/// `phase.ARW` is Phase One's.
fn facts(path: &Path) -> RawFacts {
    *FACT_READS
        .lock()
        .unwrap()
        .get_or_insert_default()
        .entry(path.to_owned())
        .or_default() += 1;
    let embedded = match path.file_name().and_then(|name| name.to_str()) {
        Some("none.ARW") => None,
        Some("large.ARW") => Some((6000, 4000)),
        Some("odd.ARW") => Some((511, 383)),
        _ => Some((64, 48)),
    };
    RawFacts {
        embedded,
        raw: (6048, 4024),
        phase_one: path.ends_with("phase.ARW"),
    }
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

/// A folder of the test's own with a raw for each name, each of its own bytes, and a cache
/// in it kept within `limit`, its start's walk done, with the session's memo of
/// fingerprints.
struct Bench {
    folder: PathBuf,
    cache: Arc<Cache>,
    upkeep: Upkeep,
    fingerprints: Fingerprints,
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
            std::fs::write(folder.join(photo), format!("a raw: {photo}")).unwrap();
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
            fingerprints: Fingerprints::default(),
            asks,
            _control: control,
        }
    }

    /// The events serving `photo` at 256 sends, through the cache with `tally`.
    fn serve(&self, photo: &str, tally: &Tally) -> Vec<LibraryEvent> {
        serve_with(self.cached(tally), &self.folder, photo)
    }

    /// The cache with `tally` and the memo, as a thumbnail job serves through them.
    fn cached<'a>(&'a self, tally: &'a Tally) -> Option<(&'a Upkeep, &'a Tally, &'a Fingerprints)> {
        Some((&self.upkeep, tally, &self.fingerprints))
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

/// The identity of `photo` of `folder` as a thumbnail job reads it now, the next session's,
/// with a memo of its own.
fn identity(folder: &Path, photo: &str) -> Identity {
    let path = folder.join(photo);
    let state = library::raw_state(&path).expect("the raw's state");
    Identity::read(&path, state, &Fingerprints::default()).expect("its identity")
}

/// The key of the embedded picture of `photo` of `folder` at 256, as a thumbnail job reads
/// it now.
fn key_of(folder: &Path, photo: &str) -> ThumbnailKey {
    library::thumbnail_key(&identity(folder, photo), 256)
}

/// The slot of the raw at `path` at `size`, as a thumbnail job reads it now.
fn slot_of(path: &Path, size: u32) -> Vec<u8> {
    let (folder, photo) = (path.parent().unwrap(), path.file_name().unwrap());
    identity(folder, photo.to_str().unwrap()).slot(size)
}

/// Sets the modified time of the file at `path` to `time`.
fn touch(path: &Path, time: std::time::SystemTime) {
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

type Cached<'a> = Option<(&'a Upkeep, &'a Tally, &'a Fingerprints)>;

fn serve_with(cached: Cached<'_>, folder: &Path, photo: &str) -> Vec<LibraryEvent> {
    serve_raw(cached, folder, photo, RawThumbnails::Never)
}

/// [`serve_with`] under the thumbnails' setting `raw`.
fn serve_raw(
    cached: Cached<'_>,
    folder: &Path,
    photo: &str,
    raw: RawThumbnails,
) -> Vec<LibraryEvent> {
    let events = Mutex::new(Vec::new());
    let tell = |event| events.lock().unwrap().push(event);
    let asked = Asked {
        open: 3,
        folder,
        photo,
        size: 256,
        raw,
    };
    serve(cached, &asked, &tell, &|| false, read, facts);
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

/// A file touched has another version: the next ask misses, reads and writes again.
#[test]
fn a_changed_version_misses() {
    let bench = Bench::new("version", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let tally = Tally::new(256);
    bench.serve("A.ARW", &tally);
    touch(
        &path,
        std::time::SystemTime::now() + Duration::from_secs(10),
    );
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

/// A cloud file not on the disk, as the library's test hook marks it, is sent as none
/// before anything else, with a cache or without: the reading is never called, its cached
/// picture never read, and nothing is written.
#[test]
fn a_file_not_on_the_disk_is_sent_as_none_and_never_read() {
    let bench = Bench::new("not-on-the-disk", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let tally = Tally::new(256);
    assert!(picture(&bench.serve("A.ARW", &tally)).is_some());
    assert_eq!((reads(&path), bench.entries()), (1, 1));

    apertrust_library::testing::evict(&path);
    assert_eq!(picture(&bench.serve("A.ARW", &tally)), None);
    assert_eq!(picture(&serve_with(None, &bench.folder, "A.ARW")), None);
    assert_eq!(reads(&path), 1, "not read again");
    assert_eq!(tally.counts(), (0, 1, 1), "neither a hit nor a miss");
    assert_eq!(bench.entries(), 1);
    apertrust_library::testing::restore(&path);
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
        raw: RawThumbnails::Never,
    };
    let tally = Tally::new(256);
    serve(bench.cached(&tally), &asked, &tell, &|| false, read, facts);
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
        raw: RawThumbnails::Never,
    };
    let tally = Tally::new(256);
    serve(bench.cached(&tally), &asked, &tell, &cancel, read, facts);
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
    let key = key_of(&bench.folder, "A.ARW");
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

/// Both the exact edit's render and an older render are preferable to the camera's
/// picture. The event needs no extra flags: either replaces the cell's pixels alike.
#[test]
fn rendered_entries_are_served_current_or_stale() {
    for (name, version) in [
        (
            "rendered-current",
            library::rendered_version(&apertrust_library::index::hash(b"edit")),
        ),
        (
            "rendered-stale",
            [library::RENDERED, b"old renderer and edit"].concat(),
        ),
    ] {
        let bench = Bench::new(name, &["A.ARW"], Limit::Bytes(1 << 30));
        let path = bench.folder.join("A.ARW");
        let slot = slot_of(&path, 256);
        let jpeg = encode(&gradient_of(32, 24)).unwrap();
        bench
            .cache
            .write(THUMBNAILS, &slot, &version, &jpeg)
            .unwrap();
        let tally = Tally::new(256);
        let image = picture(&bench.serve("A.ARW", &tally)).unwrap();
        assert_eq!((image.width, image.height), (32, 24));
        assert_eq!(reads(&path), 0);
        assert_eq!(tally.counts(), (1, 0, 0));
    }
}

#[test]
fn an_embedded_write_keeps_a_rendered_entry_that_arrived_meanwhile() {
    let bench = Bench::new("rendered-race", &["A.ARW"], Limit::Bytes(1 << 30));
    let embedded = key_of(&bench.folder, "A.ARW");
    let rendered = ThumbnailKey {
        slot: embedded.slot.clone(),
        version: library::rendered_version(&apertrust_library::index::hash(b"edit")),
    };
    assert!(write(
        &bench.upkeep,
        &rendered,
        &gradient_of(32, 24),
        "A.ARW"
    ));
    assert!(!write(&bench.upkeep, &embedded, &gradient(), "A.ARW"));
    let (version, _) = bench
        .cache
        .read_any(THUMBNAILS, &embedded.slot)
        .unwrap()
        .unwrap();
    assert_eq!(version, rendered.version);
}

#[test]
fn an_unreadable_rendered_entry_falls_back_to_the_embedded_picture() {
    let bench = Bench::new("rendered-broken", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let slot = slot_of(&path, 256);
    bench
        .cache
        .write(THUMBNAILS, &slot, library::RENDERED, b"broken jpeg")
        .unwrap();
    let image = picture(&bench.serve("A.ARW", &Tally::new(256))).unwrap();
    assert_eq!(image, gradient());
    assert_eq!(reads(&path), 1);
    assert!(
        bench
            .cache
            .read_any(THUMBNAILS, &slot)
            .unwrap()
            .unwrap()
            .0
            .starts_with(library::EMBEDDED)
    );
}

#[test]
fn a_render_arriving_after_a_failed_lookup_wins_over_the_fallback() {
    let bench = Bench::new("rendered-after-miss", &["A.ARW"], Limit::Bytes(1 << 30));
    let embedded = key_of(&bench.folder, "A.ARW");
    let old = library::rendered_version(&apertrust_library::index::hash(b"old"));
    bench
        .cache
        .write(THUMBNAILS, &embedded.slot, &old, b"broken jpeg")
        .unwrap();
    let newer = ThumbnailKey {
        slot: embedded.slot.clone(),
        version: library::rendered_version(&apertrust_library::index::hash(b"new")),
    };
    let sent = Mutex::new(None);
    let mut first = true;
    assert!(send_current_with(
        &bench.upkeep,
        &embedded.slot,
        None,
        Some(gradient()),
        &|| false,
        &|image| *sent.lock().unwrap() = image,
        || {
            if first {
                first = false;
                assert!(write(&bench.upkeep, &newer, &gradient_of(32, 24), "A.ARW"));
                None
            } else {
                rendered_hit(&bench.upkeep, &embedded.slot)
            }
        }
    ));
    let image = sent.into_inner().unwrap().unwrap();
    assert_eq!((image.width, image.height), (32, 24));
}

#[test]
fn reset_replaces_a_render_but_a_superseded_write_changes_nothing() {
    let bench = Bench::new("rendered-reset", &["A.ARW"], Limit::Bytes(1 << 30));
    let embedded = key_of(&bench.folder, "A.ARW");
    let rendered = ThumbnailKey {
        slot: embedded.slot.clone(),
        version: library::rendered_version(&apertrust_library::index::hash(b"edit")),
    };
    assert!(write(
        &bench.upkeep,
        &rendered,
        &gradient_of(32, 24),
        "A.ARW"
    ));
    assert!(!replace_with_embedded(
        &bench.upkeep,
        &embedded,
        &gradient(),
        "A.ARW",
        &|| true
    ));
    assert!(rendered_hit(&bench.upkeep, &embedded.slot).is_some());
    assert!(replace_with_embedded(
        &bench.upkeep,
        &embedded,
        &gradient(),
        "A.ARW",
        &|| false
    ));
    assert!(!write_if_current(
        &bench.upkeep,
        &rendered,
        &gradient_of(32, 24),
        "A.ARW",
        &|| true
    ));
    let (version, _) = bench
        .cache
        .read_any(THUMBNAILS, &embedded.slot)
        .unwrap()
        .unwrap();
    assert_eq!(version, embedded.version);
}

#[test]
fn a_reset_without_an_embedded_picture_removes_only_the_rendered_slot() {
    let bench = Bench::new("rendered-remove", &["none.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("none.ARW");
    let key = ThumbnailKey {
        slot: slot_of(&path, 256),
        version: library::rendered_version(&apertrust_library::index::hash(b"edit")),
    };
    assert!(write(&bench.upkeep, &key, &gradient(), "none.ARW"));
    assert_eq!(bench.entries(), 1);
    assert!(!remove_rendered_if_current(
        &bench.upkeep,
        &key.slot,
        &|| true
    ));
    assert!(remove_rendered_if_current(
        &bench.upkeep,
        &key.slot,
        &|| false
    ));
    assert_eq!(bench.entries(), 0);
    assert!(remove_rendered_if_current(
        &bench.upkeep,
        &key.slot,
        &|| false
    ));
}

#[test]
fn a_slow_embedded_read_does_not_downgrade_a_render_that_arrived() {
    fn render_during_read(
        open: u64,
        folder: &Path,
        photo: &str,
        size: u32,
        observer: &(dyn Fn(LibraryEvent) + Send + Sync),
        _: &(dyn Fn() -> bool + Sync),
    ) {
        let cache = Cache::open(folder.join("cache"));
        let slot = slot_of(&folder.join(photo), size);
        let version = library::rendered_version(&apertrust_library::index::hash(b"new edit"));
        let jpeg = encode(&gradient_of(32, 24)).unwrap();
        cache.write(THUMBNAILS, &slot, &version, &jpeg).unwrap();
        observer(LibraryEvent::Thumbnail {
            open,
            photo: photo.to_owned(),
            size,
            image: Some(gradient()),
        });
    }
    let bench = Bench::new("rendered-event-race", &["A.ARW"], Limit::Bytes(1 << 30));
    let events = Mutex::new(Vec::new());
    serve(
        bench.cached(&Tally::new(256)),
        &Asked {
            open: 3,
            folder: &bench.folder,
            photo: "A.ARW",
            size: 256,
            raw: RawThumbnails::Never,
        },
        &|event| events.lock().unwrap().push(event),
        &|| false,
        render_during_read,
        facts,
    );
    let image = picture(&events.into_inner().unwrap()).unwrap();
    assert_eq!((image.width, image.height), (32, 24));
}

/// A render of a photo with no edit is shown at 256, VGA's level of 720 by 450, as
/// darktable's rule shows it, else the camera's embedded picture takes its slot: never, the
/// embedded picture, but with none or for a Phase One raw; always, the render; auto and a
/// size at the cell's level, the render where the embedded picture is smaller than the
/// level (64 by 48 and 511 by 383, not 6000 by 4000); a size below it, the render.
#[test]
fn a_render_of_a_photo_with_no_edit_is_shown_as_the_setting_says() {
    use RawThumbnails::{Always, Auto, Never, Small, Vga};
    let cases = [
        (Never, "large.ARW", false),
        (Never, "none.ARW", true),
        (Never, "phase.ARW", true),
        (Always, "large.ARW", true),
        (Auto, "A.ARW", true),
        (Auto, "odd.ARW", true),
        (Auto, "large.ARW", false),
        (Small, "large.ARW", true),
        (Vga, "large.ARW", false),
        (Vga, "odd.ARW", true),
    ];
    for (raw, photo, shown) in cases {
        let name = format!("raw-{raw:?}-{photo}");
        let bench = Bench::new(&name, &[photo], Limit::Bytes(1 << 30));
        let slot = slot_of(&bench.folder.join(photo), 256);
        let version = library::rendered_version(&library::UNALTERED);
        let jpeg = encode(&gradient_of(32, 24)).unwrap();
        bench
            .cache
            .write(THUMBNAILS, &slot, &version, &jpeg)
            .unwrap();
        let tally = Tally::new(256);
        let image = picture(&serve_raw(bench.cached(&tally), &bench.folder, photo, raw));
        let size = image.map(|image| (image.width, image.height));
        assert_eq!(size == Some((32, 24)), shown, "{raw:?} {photo}");
        let (kept, _) = bench.cache.read_any(THUMBNAILS, &slot).unwrap().unwrap();
        if shown {
            assert_eq!(kept, version, "{raw:?} {photo}");
        } else {
            assert!(kept.starts_with(library::EMBEDDED), "{raw:?} {photo}");
        }
    }
}

/// A render of a photo with no edit that records its raw's facts is decided from them under
/// auto, with no look at the raw: here a 64 by 48 embedded picture, smaller than the level,
/// though the raw itself would say 6000 by 4000. Once the raw is touched, its facts are
/// read again: the embedded picture of 6000 by 4000 then takes the slot.
#[test]
fn a_render_that_records_its_facts_is_shown_with_no_read_of_the_raw() {
    let bench = Bench::new("raw-facts", &["large.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("large.ARW");
    let slot = slot_of(&path, 256);
    let recorded = RawFacts {
        embedded: Some((64, 48)),
        raw: (6048, 4024),
        phase_one: false,
    };
    let stamp = library::raw_stamp(&path).unwrap();
    let version = library::unaltered_version(&recorded, &stamp, library::Origin::Rule);
    let jpeg = encode(&gradient_of(32, 24)).unwrap();
    bench
        .cache
        .write(THUMBNAILS, &slot, &version, &jpeg)
        .unwrap();
    let tally = Tally::new(256);
    let serve = || {
        picture(&serve_raw(
            bench.cached(&tally),
            &bench.folder,
            "large.ARW",
            RawThumbnails::Auto,
        ))
    };
    let image = serve().unwrap();
    assert_eq!((image.width, image.height), (32, 24));
    let fact_reads = || {
        let reads = FACT_READS.lock().unwrap();
        reads.as_ref().and_then(|reads| reads.get(&path)).copied()
    };
    assert_eq!(fact_reads(), None, "the raw's facts are not read");
    assert_eq!(reads(&path), 0, "the raw is not read");
    // The raw touched: the facts recorded are of its earlier time.
    touch(
        &path,
        std::time::SystemTime::now() + Duration::from_secs(10),
    );
    let image = serve().unwrap();
    assert_eq!(
        (image.width, image.height),
        (64, 48),
        "the embedded picture"
    );
    assert_eq!(fact_reads(), Some(1), "the raw's facts are read again");
}

/// An edited photo's render is shown whatever the setting.
#[test]
fn an_edited_photo_s_render_is_shown_whatever_the_setting() {
    let bench = Bench::new("raw-edited", &["odd.ARW"], Limit::Bytes(1 << 30));
    let slot = slot_of(&bench.folder.join("odd.ARW"), 256);
    let version = library::rendered_version(&apertrust_library::index::hash(b"edit"));
    let jpeg = encode(&gradient_of(32, 24)).unwrap();
    bench
        .cache
        .write(THUMBNAILS, &slot, &version, &jpeg)
        .unwrap();
    for raw in [RawThumbnails::Never, RawThumbnails::Auto] {
        let tally = Tally::new(256);
        let events = serve_raw(bench.cached(&tally), &bench.folder, "odd.ARW", raw);
        let image = picture(&events).unwrap();
        assert_eq!((image.width, image.height), (32, 24), "{raw:?}");
    }
}

/// A raw renamed in its folder keeps its thumbnail: asked for under its new name, it is a
/// hit, its fingerprint the same.
#[test]
fn a_renamed_raw_finds_its_cached_thumbnail() {
    let bench = Bench::new("renamed", &["A.ARW"], Limit::Bytes(1 << 30));
    let tally = Tally::new(256);
    bench.serve("A.ARW", &tally);
    assert_eq!(tally.counts(), (0, 1, 1));
    let renamed = bench.folder.join("B.ARW");
    std::fs::rename(bench.folder.join("A.ARW"), &renamed).unwrap();
    let image = picture(&bench.serve("B.ARW", &tally)).expect("a picture");
    assert_eq!((image.width, image.height), (64, 48));
    assert_eq!(reads(&renamed), 0, "a hit reads nothing");
    assert_eq!(tally.counts(), (1, 1, 1));
    assert_eq!(bench.entries(), 1);
}

/// A raw replaced by other bytes of the same length and modified time, as FAT32's
/// 2-second times allow for a burst, has another fingerprint: the next session reads it
/// again, never taking the other raw's thumbnail.
#[test]
fn a_raw_replaced_with_the_same_size_and_time_misses() {
    let bench = Bench::new("replaced", &["A.ARW"], Limit::Bytes(1 << 30));
    let path = bench.folder.join("A.ARW");
    let tally = Tally::new(256);
    bench.serve("A.ARW", &tally);
    let before = library::raw_stamp(&path).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let length = std::fs::metadata(&path).unwrap().len() as usize;
    std::fs::write(&path, vec![b'x'; length]).unwrap();
    touch(&path, modified);
    assert_eq!(
        library::raw_stamp(&path),
        Some(before),
        "the same size and time"
    );
    // The next session, with a memo of its own.
    let next = Fingerprints::default();
    serve_with(Some((&bench.upkeep, &tally, &next)), &bench.folder, "A.ARW");
    assert_eq!(reads(&path), 2, "read again");
    assert_eq!(tally.counts(), (0, 2, 2));
}

/// The same raw in two folders, one with a sidecar and its render in the cache: the other,
/// with no sidecar, has no copy and so its own slot, and gets the camera's embedded picture.
#[test]
fn an_unedited_copy_of_an_edited_raw_never_shows_its_render() {
    use apertrust_library::sidecar::{Edits, VirtualCopy};
    let bench = Bench::new("copies", &["A.ARW"], Limit::Bytes(1 << 30));
    let edited = bench.folder.join("A.ARW");
    let mut edits = Edits::new("test");
    edits.copies.push(VirtualCopy {
        id: "1a2b3c4d".to_owned(),
        number: "0".to_owned(),
        name: String::new(),
        end: String::new(),
        clocks: Vec::new(),
        history: Vec::new(),
    });
    let sidecar = apertrust_library::sidecar::path_for(&edited);
    std::fs::write(&sidecar, edits.to_xml()).unwrap();
    let other = bench.folder.join("other");
    std::fs::create_dir(&other).unwrap();
    std::fs::copy(&edited, other.join("A.ARW")).unwrap();
    let photo = identity(&bench.folder, "A.ARW");
    assert_eq!(photo.copy.as_deref(), Some("1a2b3c4d"));
    assert_eq!(identity(&other, "A.ARW").fingerprint, photo.fingerprint);
    let slot = photo.slot(256);
    let version = library::rendered_version(&apertrust_library::index::hash(b"edit"));
    let jpeg = encode(&gradient_of(32, 24)).unwrap();
    bench
        .cache
        .write(THUMBNAILS, &slot, &version, &jpeg)
        .unwrap();
    let tally = Tally::new(256);
    let image = picture(&bench.serve("A.ARW", &tally)).unwrap();
    assert_eq!(
        (image.width, image.height),
        (32, 24),
        "the edited one's render"
    );
    let image = picture(&serve_with(bench.cached(&tally), &other, "A.ARW")).unwrap();
    assert_eq!(image, gradient(), "the other's embedded picture");
    assert_eq!(reads(&other.join("A.ARW")), 1);
    let (kept, _) = bench.cache.read_any(THUMBNAILS, &slot).unwrap().unwrap();
    assert_eq!(kept, version, "the render kept");
}

/// Writes a picture of 32 by 24 as the render of `photo` with no edit at 256, recording the
/// facts `facts` gives and the raw's stamp, made by `origin`: its version, and its slot.
fn unaltered(bench: &Bench, photo: &str, origin: Origin) -> (Vec<u8>, Vec<u8>) {
    let path = bench.folder.join(photo);
    let stamp = library::raw_stamp(&path).unwrap();
    let version = library::unaltered_version(&facts(&path), &stamp, origin);
    let jpeg = encode(&gradient_of(32, 24)).unwrap();
    let slot = identity(&bench.folder, photo).slot(256);
    bench
        .cache
        .write(THUMBNAILS, &slot, &version, &jpeg)
        .unwrap();
    (version, slot)
}

/// Under never, a render of a photo with no edit the rule made, of the raw as it is, is not
/// shown: the embedded picture of 64 by 48 is sent and written over it.
#[test]
fn a_rule_render_still_follows_the_setting() {
    let bench = Bench::new("rule-render", &["x.ARW"], Limit::Bytes(1 << 30));
    let (_, slot) = unaltered(&bench, "x.ARW", Origin::Rule);
    let tally = Tally::new(256);
    let events = serve_raw(
        bench.cached(&tally),
        &bench.folder,
        "x.ARW",
        RawThumbnails::Never,
    );
    let image = picture(&events).unwrap();
    assert_eq!((image.width, image.height), (64, 48));
    let (kept, _) = bench.cache.read_any(THUMBNAILS, &slot).unwrap().unwrap();
    assert!(kept.starts_with(library::EMBEDDED), "{kept:?}");
}

/// A render of a photo with no edit made from the live photo is sent whatever the setting,
/// never replaced by the embedded picture, while its raw is the one it records. Once the
/// raw's stamp changes, its bytes and so its slot the same, the rule decides: under never,
/// the embedded picture is sent and written over it.
#[test]
fn an_opened_render_is_sent_whatever_the_setting_until_its_raw_is_replaced() {
    let bench = Bench::new("opened-render", &["x.ARW"], Limit::Bytes(1 << 30));
    let (version, slot) = unaltered(&bench, "x.ARW", Origin::Opened);
    let path = bench.folder.join("x.ARW");
    let serve = |raw| {
        let tally = Tally::new(256);
        let events = serve_raw(bench.cached(&tally), &bench.folder, "x.ARW", raw);
        let image = picture(&events).unwrap();
        (image.width, image.height)
    };
    for raw in [
        RawThumbnails::Never,
        RawThumbnails::Auto,
        RawThumbnails::Always,
    ] {
        assert_eq!(serve(raw), (32, 24), "{raw:?}");
        let (kept, _) = bench.cache.read_any(THUMBNAILS, &slot).unwrap().unwrap();
        assert_eq!(kept, version, "{raw:?}");
    }
    assert_eq!(reads(&path), 0, "the embedded picture is not read");
    touch(&path, std::time::SystemTime::now() + Duration::from_secs(5));
    assert_eq!(identity(&bench.folder, "x.ARW").slot(256), slot);
    assert_eq!(serve(RawThumbnails::Never), (64, 48));
    let (kept, _) = bench.cache.read_any(THUMBNAILS, &slot).unwrap().unwrap();
    assert!(kept.starts_with(library::EMBEDDED), "{kept:?}");
}
