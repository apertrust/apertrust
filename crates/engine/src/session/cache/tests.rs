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
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::SystemTime;

use apertrust_base::cache::THUMBNAILS;

use super::*;

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

    /// The limited kinds' entries as a walk finds them now.
    fn entries(&self) -> u64 {
        self.cache.walk(&never).unwrap().limited().entries
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
