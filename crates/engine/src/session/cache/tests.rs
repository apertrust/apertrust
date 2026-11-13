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

//! The cache's jobs on a control of their own, with no GPU, each in a folder of its own:
//! the test answers the workers' asks as the engine thread does.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;

use apertrust_base::cache::{THUMBNAILS, Usage};
use apertrust_library::ops::{LibraryEvent, RawThumbnails};

use super::*;
use crate::session::thumbnail::{Asked, Tally, serve};

/// How long a test waits for the cache's jobs.
const WAIT: Duration = Duration::from_secs(60);

/// The bytes of each entry's file: its mark, the version's length, a one-byte version and
/// a thousand bytes.
const ENTRY: u64 = 4 + 4 + 1 + 1000;

/// A folder of the test's own, emptied first, removed when the test ends.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let path =
            std::env::temp_dir().join(format!("apertrust-cache-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("a folder");
        Folder(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A session's cache on a control of its own, and the asks its workers send.
struct Session {
    cache: Arc<Cache>,
    upkeep: Upkeep,
    control: Control,
    asks: mpsc::Receiver<()>,
    /// The session's end, for the cache's jobs.
    stop: Arc<AtomicBool>,
    /// A stale temporary left before the start, for its tidy.
    temporary: PathBuf,
    _folder: Folder,
}

impl Session {
    /// The cache in a folder named after `name`, holding `entries` thumbnails, kept
    /// within `limit` from its start.
    fn start(name: &str, entries: u64, limit: Limit) -> Session {
        let session = Session::new(name, entries, limit);
        session.upkeep.start_walk(&session.control);
        session
    }

    /// [`Session::start`] before the start's walk is added, so a test can hold it first.
    fn new(name: &str, entries: u64, limit: Limit) -> Session {
        let folder = Folder::new(name);
        let cache = Arc::new(Cache::open(folder.0.join("cache")));
        write(&cache, 0..entries);
        let temporary = stale_temporary(&cache);
        let control = Control::start(|_| {});
        let (asked, asks) = mpsc::channel();
        let ask: Ask = Arc::new(move || {
            let _ = asked.send(());
        });
        let options = CacheOptions {
            cache: Arc::clone(&cache),
            limit,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let upkeep = Upkeep::new(options, ask, Arc::clone(&stop));
        Session {
            cache,
            upkeep,
            control,
            asks,
            stop,
            temporary,
            _folder: folder,
        }
    }

    /// Answers the workers' asks as the engine thread does until no job is left and none
    /// asks.
    fn settle(&self) {
        let deadline = Instant::now() + WAIT;
        loop {
            // Idle before the asks are taken: a job asks before it ends.
            let idle = self.control.pending() == 0;
            let mut asked = false;
            for () in self.asks.try_iter() {
                asked = true;
                self.upkeep.evict(&self.control);
            }
            if idle && !asked {
                return;
            }
            assert!(Instant::now() < deadline, "the cache's jobs end in time");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Waits until eviction stands at `eviction`.
    fn until(&self, eviction: Eviction) {
        let deadline = Instant::now() + WAIT;
        while self.eviction() != eviction {
            assert!(
                Instant::now() < deadline,
                "eviction reaches {eviction:?} in time"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn set_limit(&self, limit: Limit) {
        self.upkeep.set_limit(limit, &self.control);
    }

    fn steps(&self) -> Vec<Step> {
        self.upkeep.lock().steps.clone()
    }

    /// How many eviction jobs were added.
    fn added(&self) -> usize {
        self.upkeep.lock().added
    }

    fn eviction(&self) -> Eviction {
        self.upkeep.lock().eviction
    }

    /// The limited kinds' bytes as the handle counts them.
    fn held(&self) -> u64 {
        self.cache.sizes().expect("walked").limited().bytes
    }

    /// Holds the walk for the sizes before each subfolder: what it tells as it waits, and
    /// its permit, which, dropped, lets it go on unheld.
    fn hold_walk(&self) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (hold, begun, permit) = crate::export_job::testing::Hold::new();
        *self.upkeep.walk_hold.lock().unwrap() = Some(hold);
        (begun, permit)
    }

    /// Asks for the sizes as the `Sizes` numbered `command`.
    fn sizes(&self, command: u64, tell: &Tell) {
        self.upkeep
            .walk_for_sizes(command, &self.control, Arc::clone(tell));
    }

    /// The limited kinds' entries as a walk finds them now.
    fn entries(&self) -> u64 {
        self.cache.walk(&never).unwrap().unwrap().limited().entries
    }
}

fn never() -> bool {
    false
}

/// Thumbnails for the slots `slots`, each [`ENTRY`] bytes.
fn write(cache: &Cache, slots: std::ops::Range<u64>) {
    for slot in slots {
        let bytes = [0; 1000];
        cache
            .write(THUMBNAILS, &slot.to_le_bytes(), b"v", &bytes)
            .expect("an entry is written");
    }
}

/// A stale temporary, as a write a crash cut short leaves, in a thumbnails subfolder.
fn stale_temporary(cache: &Cache) -> PathBuf {
    let subfolder = cache.kind_folder(THUMBNAILS).join("00");
    fs::create_dir_all(&subfolder).unwrap();
    let path = subfolder.join(".~entry.AbC123.tmp");
    fs::write(&path, b"half").unwrap();
    let two_days_ago = SystemTime::now() - Duration::from_secs(2 * 24 * 60 * 60);
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(two_days_ago)
        .unwrap();
    path
}

/// The start's job tidies, then walks, then evicts where the limited kinds are past the
/// limit, to 90% of it, in the same job; within the limit it evicts nothing.
#[test]
fn the_start_tidies_then_walks_then_evicts_past_the_limit() {
    let session = Session::start("start-past", 10, Limit::Bytes(5 * ENTRY));
    session.settle();
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk, Step::Evict]);
    assert!(!session.temporary.exists(), "the start's tidy removes it");
    assert_eq!(session.added(), 0, "the start's job evicts itself");
    assert!(session.held() <= 5 * ENTRY * 9 / 10, "{}", session.held());
    assert_eq!(session.entries() * ENTRY, session.held());
    assert_eq!(session.eviction(), Eviction::Idle);

    let within = Session::start("start-within", 10, Limit::Bytes(10 * ENTRY));
    within.settle();
    assert_eq!(within.steps(), [Step::Tidy, Step::Walk]);
    assert_eq!(within.added(), 0);
    assert_eq!(within.held(), 10 * ENTRY);
    assert_eq!(within.eviction(), Eviction::Idle);
}

/// The start's walk counts as an eviction running: a limit lowered during it is folded
/// in, evicted once the walk is done, with no job beside it.
#[test]
fn a_limit_lowered_during_the_start_s_walk_is_folded_into_it() {
    let session = Session::new("during-walk", 10, Limit::Bytes(20 * ENTRY));
    let hold = session.upkeep.hold.lock().unwrap();
    session.upkeep.start_walk(&session.control);
    session.set_limit(Limit::Bytes(5 * ENTRY));
    assert_eq!(session.added(), 0);
    assert_eq!(session.eviction(), Eviction::Running { again: true });
    drop(hold);
    session.settle();
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk, Step::Evict]);
    assert_eq!(session.added(), 0);
    assert!(session.held() <= 5 * ENTRY * 9 / 10, "{}", session.held());
    assert_eq!(session.entries() * ENTRY, session.held());
    assert_eq!(session.eviction(), Eviction::Idle);
}

/// A limit lowered evicts at once; one raised, or the same again, does not.
#[test]
fn a_lowered_limit_evicts_and_a_raised_one_does_not() {
    let session = Session::start("lowered", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk]);

    session.set_limit(Limit::Bytes(40 * ENTRY));
    session.settle();
    assert_eq!(session.added(), 0);

    session.set_limit(Limit::Bytes(5 * ENTRY));
    session.settle();
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk, Step::Evict]);
    assert!(session.held() <= 5 * ENTRY * 9 / 10, "{}", session.held());

    session.set_limit(Limit::Bytes(5 * ENTRY));
    session.set_limit(Limit::Bytes(6 * ENTRY));
    session.settle();
    assert_eq!(session.added(), 1);
}

/// While the cache is unlimited nothing asks for an eviction or adds one, past any
/// size; turned on, it evicts.
#[test]
fn an_unlimited_cache_is_never_evicted_until_its_limit_is_turned_on() {
    let session = Session::start("unlimited", 10, Limit::Unlimited);
    session.settle();
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk]);
    session.upkeep.check();
    session.upkeep.evict(&session.control);
    session.set_limit(Limit::Unlimited);
    session.settle();
    assert_eq!(session.added(), 0);
    assert_eq!(session.held(), 10 * ENTRY);

    // A write past what would be the limit asks for nothing either.
    write(&session.cache, 10..20);
    session.upkeep.check();
    session.settle();
    assert_eq!(session.added(), 0);

    session.set_limit(Limit::Bytes(5 * ENTRY));
    session.settle();
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk, Step::Evict]);
    assert!(session.held() <= 5 * ENTRY * 9 / 10, "{}", session.held());

    // Turned off again, nothing more.
    session.set_limit(Limit::Unlimited);
    write(&session.cache, 20..30);
    session.upkeep.check();
    session.settle();
    assert_eq!(session.added(), 1);
}

/// A write that takes the limited kinds past the limit asks for an eviction, as 3b's
/// thumbnails will.
#[test]
fn a_write_past_the_limit_asks_for_an_eviction() {
    let session = Session::start("written", 4, Limit::Bytes(5 * ENTRY));
    session.settle();
    assert_eq!(session.added(), 0);
    write(&session.cache, 4..5);
    session.upkeep.check();
    session.settle();
    assert_eq!(session.added(), 0, "at the limit, not past it");
    write(&session.cache, 5..6);
    session.upkeep.check();
    session.settle();
    assert_eq!(session.added(), 1);
    assert!(session.held() <= 5 * ENTRY * 9 / 10, "{}", session.held());
}

/// Requests while an eviction runs are folded into it: one job, which runs once more.
#[test]
fn requests_while_an_eviction_runs_are_folded_into_it() {
    let session = Session::start("folded", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    let hold = session.upkeep.hold.lock().unwrap();
    session.set_limit(Limit::Bytes(5 * ENTRY));
    session.until(Eviction::Running { again: false });
    session.upkeep.evict(&session.control);
    session.set_limit(Limit::Bytes(3 * ENTRY));
    session.upkeep.evict(&session.control);
    assert_eq!(session.eviction(), Eviction::Running { again: true });
    drop(hold);
    session.settle();
    assert_eq!(session.added(), 1);
    assert_eq!(
        session.steps(),
        [Step::Tidy, Step::Walk, Step::Evict, Step::Evict]
    );
    // The second pass took the limit as it stood then.
    assert!(session.held() <= 3 * ENTRY * 9 / 10, "{}", session.held());
}

/// A request while an eviction is queued is that one's: no second job.
#[test]
fn a_request_while_an_eviction_is_queued_adds_none() {
    let session = Session::start("queued", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    session.upkeep.lock().eviction = Eviction::Queued;
    session.upkeep.evict(&session.control);
    session.set_limit(Limit::Bytes(5 * ENTRY));
    assert_eq!(session.added(), 0);
    assert_eq!(session.eviction(), Eviction::Queued);
}

/// An eviction that panics leaves eviction free: the next request adds a job that runs.
#[test]
fn an_eviction_that_panics_leaves_the_next_one_free() {
    let session = Session::start("panicked", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    session.upkeep.lock().panic = true;
    session.set_limit(Limit::Bytes(5 * ENTRY));
    session.settle();
    assert_eq!(session.eviction(), Eviction::Idle);
    assert_eq!(session.entries(), 10, "the panicking pass removed nothing");
    session.set_limit(Limit::Bytes(3 * ENTRY));
    session.settle();
    assert_eq!(session.added(), 2);
    assert_eq!(session.steps(), [Step::Tidy, Step::Walk, Step::Evict]);
    assert!(session.held() <= 3 * ENTRY * 9 / 10, "{}", session.held());
}

/// Quitting during the start's job stops it before its next subfolder: no walk is kept,
/// and the job ends at once.
#[test]
fn quitting_stops_the_start_s_walk() {
    let session = Session::new("quit-walk", 10, Limit::Bytes(5 * ENTRY));
    let hold = session.upkeep.hold.lock().unwrap();
    session.upkeep.start_walk(&session.control);
    session.stop.store(true, Ordering::Relaxed);
    drop(hold);
    let started = Instant::now();
    session.settle();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(session.steps(), [Step::Tidy]);
    assert_eq!(session.cache.sizes(), None, "no walk is kept");
    assert_eq!(session.eviction(), Eviction::Idle);
    assert_eq!(session.entries(), 10);
}

/// As the session ends, an eviction under way stops: it removes less than a full pass.
#[test]
fn an_eviction_stops_as_the_session_ends() {
    let session = Session::start("stopped", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    let hold = session.upkeep.hold.lock().unwrap();
    session.set_limit(Limit::Bytes(5 * ENTRY));
    session.until(Eviction::Running { again: false });
    session.stop.store(true, Ordering::Relaxed);
    drop(hold);
    session.settle();
    assert_eq!(session.eviction(), Eviction::Idle);
    // A full pass leaves 4.
    assert!(session.entries() > 4, "{}", session.entries());
}

/// What [`Upkeep::walk_for_sizes`] tells, through a channel.
fn told() -> (Tell, mpsc::Receiver<Event>) {
    let (sent, told) = mpsc::channel();
    let sent = Mutex::new(sent);
    let tell: Tell = Arc::new(move |event| {
        let _ = sent.lock().unwrap().send(event);
    });
    (tell, told)
}

/// The thumbnails each sizes' answer among `answers` tells.
fn thumbnails(answers: &[Event]) -> Vec<u64> {
    let told = answers.iter().filter_map(|event| match event {
        Event::Cache(CacheEvent::Sizes { sizes, .. }) => Some(sizes.of(THUMBNAILS).entries),
        _ => None,
    });
    told.collect()
}

/// Before any walk the handle counts nothing, so `Sizes` has no answer at once: its walk
/// gives the first, and the handle keeps it.
#[test]
fn sizes_before_any_walk_are_walked_first() {
    let session = Session::new("sizes-before", 10, Limit::Bytes(20 * ENTRY));
    assert_eq!(session.upkeep.counted(), None);
    let (tell, told) = told();
    session.sizes(1, &tell);
    session.settle();
    let answers: Vec<_> = told.try_iter().collect();
    let [Event::Cache(CacheEvent::Sizes { sizes, limit, .. })] = answers.as_slice() else {
        panic!("one walk answers: {answers:?}");
    };
    assert_eq!(sizes.of(THUMBNAILS).entries, 10);
    assert_eq!(*limit, Limit::Bytes(20 * ENTRY));
    assert_eq!(
        session.upkeep.counted(),
        Some(CacheEvent::sizes(*sizes, *limit))
    );
}

/// A `Sizes` that comes while the walk runs is answered by a walk begun after it, one pass
/// more, which answers every `Sizes` that came meanwhile; then the walk is free again.
#[test]
fn a_sizes_while_its_walk_runs_gets_a_walk_of_its_own() {
    let session = Session::new("sizes-again", 10, Limit::Bytes(20 * ENTRY));
    let (tell, told) = told();
    let (begun, permit) = session.hold_walk();
    session.sizes(1, &tell);
    begun.recv_timeout(WAIT).expect("the walk begins");
    // A thumbnail more, written after the walk began, which only the next pass sees.
    write(&session.cache, 10..11);
    session.sizes(2, &tell);
    session.sizes(3, &tell);
    drop(permit);
    session.settle();
    let answers: Vec<_> = told.try_iter().collect();
    assert_eq!(thumbnails(&answers).last(), Some(&11), "{answers:?}");
    assert_eq!(thumbnails(&answers).len(), 2, "two passes: {answers:?}");
    assert!(!session.upkeep.lock().sizing.job);

    session.sizes(4, &tell);
    session.settle();
    assert_eq!(thumbnails(&told.try_iter().collect::<Vec<_>>()), [11]);
}

/// A walk for the sizes that the session's own Clear overlaps neither keeps nor tells
/// what it saw before the Clear: it walks again, and tells the cleared kind empty.
#[test]
fn a_sizes_walk_a_clear_overlaps_tells_what_the_clear_left() {
    let session = Session::start("sizes-clear", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    let (tell, told) = told();
    let (begun, permit) = session.hold_walk();
    session.sizes(1, &tell);
    // Past the first subfolder, whose entries it has counted.
    begun.recv_timeout(WAIT).expect("the walk begins");
    permit.send(()).unwrap();
    begun.recv_timeout(WAIT).expect("the walk goes on");
    let removed = session.cache.clear(THUMBNAILS, &|_, _| {}, &never).unwrap();
    assert_eq!(removed.entries, 10);
    drop(permit);
    session.settle();
    let answers: Vec<_> = told.try_iter().collect();
    assert_eq!(thumbnails(&answers), [0], "{answers:?}");
    let counted = session.cache.sizes().unwrap();
    assert_eq!(counted.of(THUMBNAILS), Usage::default());
}

/// A walk that fails names each `Sizes` its pass answers.
#[cfg(unix)]
#[test]
fn a_failed_walk_names_each_command_it_answers() {
    use apertrust_base::cache::PIPELINES;
    use std::os::unix::fs::PermissionsExt;
    let session = Session::new("sizes-failed", 10, Limit::Bytes(20 * ENTRY));
    session
        .cache
        .write(PIPELINES, b"gpu", b"v", &[0; 10])
        .unwrap();
    let pipelines = session.cache.kind_folder(PIPELINES);
    let mode = |mode| fs::set_permissions(&pipelines, fs::Permissions::from_mode(mode)).unwrap();
    mode(0o000);
    // Where the process reads it anyway, as root can, there is nothing to test.
    if fs::read_dir(&pipelines).is_ok() {
        mode(0o755);
        return;
    }
    let (tell, told) = told();
    let (begun, permit) = session.hold_walk();
    session.sizes(1, &tell);
    begun.recv_timeout(WAIT).expect("the walk begins");
    session.sizes(2, &tell);
    session.sizes(3, &tell);
    drop(permit);
    session.settle();
    mode(0o755);
    let named: Vec<_> = told
        .try_iter()
        .filter_map(|event| match event {
            Event::Failed(SessionError::CacheWalk { command, .. }) => Some(command),
            _ => None,
        })
        .collect();
    assert_eq!(named, [1, 2, 3]);
}

/// A Clear that cannot remove some entries fails, `cache-remove`, removes the rest, and
/// still tells the sizes, with what stayed.
#[cfg(unix)]
#[test]
fn a_clear_that_cannot_remove_some_fails_and_still_tells_the_sizes() {
    use std::os::unix::fs::PermissionsExt;
    let session = Session::start("clear-failed", 10, Limit::Bytes(20 * ENTRY));
    session.settle();
    let thumbnails_folder = session.cache.kind_folder(THUMBNAILS);
    // A subfolder holding entries, whichever the listing, in the file system's order,
    // gives first: the stale temporary's, which the start's tidy emptied, holds none.
    let (subfolder, kept) = fs::read_dir(&thumbnails_folder)
        .unwrap()
        .flatten()
        .map(|item| item.path())
        .filter(|path| !path.file_name().unwrap().to_string_lossy().starts_with('.'))
        .map(|path| {
            let kept = fs::read_dir(&path).unwrap().count() as u64;
            (path, kept)
        })
        .find(|&(_, kept)| kept > 0)
        .expect("a subfolder with entries");
    let mode = |mode| fs::set_permissions(&subfolder, fs::Permissions::from_mode(mode)).unwrap();
    mode(0o555);
    let probe = subfolder.join("probe");
    if fs::write(&probe, b"").is_ok() {
        let _ = fs::remove_file(&probe);
        mode(0o755);
        return;
    }
    let (tell, told) = told();
    let job = session.upkeep.clear_job(THUMBNAILS, tell, None);
    let ended = session.control.run_now(job);
    mode(0o755);
    let failure = ended.expect_err("the Clear fails");
    assert_eq!(failure.cause, CACHE_REMOVE);
    let answers: Vec<_> = told.try_iter().collect();
    assert_eq!(thumbnails(&answers), [kept], "{answers:?}");
    assert_eq!(session.entries(), kept);
}

/// A size as a job's message tells it: kB, MB or GB of disks' powers of ten, one decimal
/// below 10, written so, and none from there.
#[test]
fn a_size_is_told_in_the_unit_it_reads_best_in() {
    let told = |bytes| {
        let message = with_size(Message::new("size"), bytes);
        let size = message.argument("size").cloned();
        (size, message.argument("unit").cloned())
    };
    let number = |value: f64| {
        let at_least = u8::from(value < 10.0);
        Some(apertrust_base::jobs::Argument::Decimals { value, at_least })
    };
    let unit = |u: &str| Some(apertrust_base::jobs::Argument::Text(u.to_owned()));
    assert_eq!(told(0), (number(0.0), unit("kb")));
    assert_eq!(told(2_018), (number(2.0), unit("kb")));
    assert_eq!(told(10_090), (number(10.0), unit("kb")));
    assert_eq!(told(4_250_000), (number(4.3), unit("mb")));
    assert_eq!(told(138_412_032), (number(138.0), unit("mb")));
    assert_eq!(told(12_500_000_000), (number(13.0), unit("gb")));
    // The unit chosen after rounding: just below a unit reads in it.
    assert_eq!(told(49), (number(0.0), unit("kb")));
    assert_eq!(told(9_960), (number(10.0), unit("kb")));
    assert_eq!(told(999_400), (number(999.0), unit("kb")));
    assert_eq!(told(999_800), (number(1.0), unit("mb")));
    assert_eq!(told(999_600_000), (number(1.0), unit("gb")));
    assert_eq!(told(2_000_000_000_000), (number(2000.0), unit("gb")));
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
                } else if !path.extension().is_some_and(|e| e == "apxmp" || e == "xmp")
                    && !path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with('.'))
                {
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
                        raw: RawThumbnails::Never,
                    };
                    let tell = |_: LibraryEvent| {};
                    serve(
                        Some((&upkeep, &tally)),
                        &asked,
                        &tell,
                        &|| false,
                        apertrust_library::ops::thumbnail,
                        crate::session::thumbnail::raw_facts,
                    );
                }
                let (hits, misses, written) = tally.counts();
                let took = started.elapsed().as_secs_f64();
                println!(
                    "{size}: {pass} in {took:.2} s: {hits} hits, {misses} read, {written} written"
                );
            }
            let sizes = bench_cache.walk(&|| false).unwrap().unwrap().of(THUMBNAILS);
            let (entries, bytes) = (sizes.entries - before.0, sizes.bytes - before.1);
            before = (sizes.entries, sizes.bytes);
            let each = bytes as f64 / entries as f64 / 1e3;
            println!("{size}: {entries} entries, {each:.1} kB each");
        }
        drop(control);
        // Padded to the count, each a file of a byte in a subfolder of its own: a walk
        // reads no entry's contents.
        let have = bench_cache
            .walk(&|| false)
            .unwrap()
            .unwrap()
            .of(THUMBNAILS)
            .entries;
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
        let sizes = cache.walk(&|| false).unwrap().unwrap();
        let ms = started.elapsed().as_secs_f64() * 1e3;
        let entries = sizes.of(THUMBNAILS).entries;
        println!("walk {run}: {entries} entries in {ms:.1} ms");
    }
}
