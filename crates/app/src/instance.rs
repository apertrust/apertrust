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

//! One app instance per user data folder (decision 89; `docs/gui.md`, Instances and
//! windows): a start that finds the app running on its folder hands it what it was asked
//! to open and exits, on Windows and Linux; on macOS it is refused.
//!
//! - **The lock** ([`start`]): `instance.lock` in the user data folder, by its canonical
//!   path, held with `File::try_lock` for the whole run, the file holding nothing; the
//!   system releases it as the process ends, a crash included. The start that takes it is
//!   the [`Instance`].
//! - **The channel,** on Windows and Linux alone: a named pipe, `\\.\pipe\apertrust-<hash>`,
//!   the first instance of its name and open to the user's own account alone
//!   (`apertrust-display`), or an abstract Unix socket, `apertrust-<uid>-<hash>`, which has
//!   no file. `<hash>` is the first 16 hex digits of BLAKE3 over the canonical path. Each
//!   end checks that the other runs as its own user: the instance drops another user's
//!   connection unanswered, and a second start sends nothing to another user's server.
//! - **A second start** sends a [`HandOff`]: the format, its arguments, its paths made
//!   absolute, its working folder, its activation token and its flags for one run. The
//!   answer is [`Answer::Taken`] or [`Answer::Refused`] with the reason; another format is
//!   refused, saying another version is running. A start that cannot connect while the
//!   lock is held retries for 2 s, then gives up.
//! - **On macOS** there is no channel: Finder's opens reach the running app through
//!   LaunchServices, and a start that finds the lock held is refused.
//! - **The joins** ([`Join`]): hand-offs arriving before the window shows join its own
//!   command line into one open; on Windows, where Explorer starts the app once per file,
//!   those arriving within [`JOIN`] of the one before join too.
//!
//! The instance's thread answers each hand-off with the [`Handler`] it is given, which
//! decides the answer and posts what opens to the window ([`Post`], [`Inbox`]).

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

/// The lock's file in the user data folder.
pub const LOCK: &str = "instance.lock";

/// On Windows, the longest gap between two hand-offs that open together.
pub const JOIN: Duration = Duration::from_millis(500);

/// A second start's activation token, which lets the instance come forward: Wayland's
/// `XDG_ACTIVATION_TOKEN` and X11's `DESKTOP_STARTUP_ID`, as the launcher set them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Token {
    pub wayland: Option<String>,
    pub x11: Option<String>,
}

impl Token {
    /// The token this process was started with.
    pub fn from_environment() -> Token {
        let read = |name| std::env::var(name).ok().filter(|token| !token.is_empty());
        Token {
            wayland: read("XDG_ACTIVATION_TOKEN"),
            x11: read("DESKTOP_STARTUP_ID"),
        }
    }
}

/// What a second start hands the instance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HandOff {
    /// Its command line after the program's name, its paths made absolute from its own
    /// working folder, so a drive-relative one on Windows is its own drive's.
    pub arguments: Vec<OsString>,
    /// Its working folder, which its relative paths are taken from.
    pub folder: PathBuf,
    pub token: Token,
    /// Its flags for one run but the cache folder, as given: `--safe-mode`, `-d DOMAIN`,
    /// `--conf KEY=VALUE`.
    pub one_run: Vec<OsString>,
    /// The cache folder its `--cache-dir` names, absolute, where it gave one.
    pub cache: Option<PathBuf>,
}

/// The instance's answer to a hand-off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// What it was asked to open opens there, its window forward.
    Taken,
    /// Refused, saying why, for the second start's stderr.
    Refused(String),
}

/// What the instance's thread does with each hand-off of its format: its answer.
pub type Handler = Arc<dyn Fn(HandOff) -> Answer + Send + Sync>;

/// What a start found on its user data folder.
pub enum Start {
    /// It took the lock: it is the instance.
    Instance(Instance),
    /// The lock can be neither taken nor checked, as on a file system without locks: the
    /// app runs as it would with no instance, saying why.
    Alone(String),
    /// The instance took its hand-off: it exits.
    #[cfg_attr(
        not(any(windows, target_os = "linux")),
        allow(dead_code, reason = "macOS hands nothing off")
    )]
    Taken,
    /// The hand-off ended without being taken, or on macOS the start was refused, saying
    /// why: it exits with status 1.
    Ended(String),
}

/// Takes the lock on the user data folder `user_data` and, on Windows and Linux, binds the
/// channel, its hand-offs answered by `handler`; or, where an instance holds it, hands it
/// `request()` and says how that went, or on macOS that it runs.
pub fn start(user_data: &Path, request: impl FnOnce() -> HandOff, handler: Handler) -> Start {
    start_as(user_data, request, handler, own_user())
}

/// The user the process runs as, whose instance alone a start hands off to on Linux.
#[cfg(target_os = "linux")]
fn own_user() -> u32 {
    apertrust_display::instance::user_id()
}

#[cfg(not(target_os = "linux"))]
fn own_user() -> u32 {
    0
}

/// What a try at the lock found.
enum Lock {
    Taken(File),
    /// Another start holds it.
    Held,
    Unusable(String),
}

/// Tries the lock at `path` once.
fn try_lock(path: &Path) -> Lock {
    let opened = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path);
    let lock = match opened {
        Ok(lock) => lock,
        Err(e) => return Lock::Unusable(format!("{}: {e}", path.display())),
    };
    match lock.try_lock() {
        Ok(()) => Lock::Taken(lock),
        Err(TryLockError::WouldBlock) => Lock::Held,
        Err(TryLockError::Error(e)) => {
            Lock::Unusable(format!("{}: not locked: {e}", path.display()))
        }
    }
}

/// [`start`], handing off only to an instance running as `user` (on Linux).
#[cfg(any(windows, target_os = "linux"))]
fn start_as(
    user_data: &Path,
    request: impl FnOnce() -> HandOff,
    handler: Handler,
    user: u32,
) -> Start {
    let canonical = match std::fs::canonicalize(user_data) {
        Ok(canonical) => canonical,
        Err(e) => return Start::Alone(format!("{}: {e}", user_data.display())),
    };
    let path = canonical.join(LOCK);
    let deadline = Instant::now() + channel::RETRY;
    let mut request = Some(request);
    let mut hand_off = None;
    loop {
        // Tried again each time, so a start whose instance has quit meanwhile takes over.
        match try_lock(&path) {
            Lock::Taken(lock) => {
                return Start::Instance(Instance::begin(lock, &canonical, handler, user));
            }
            Lock::Held => {}
            Lock::Unusable(why) => return Start::Alone(why),
        }
        let hand_off = hand_off.get_or_insert_with(|| (request.take().expect("asked once"))());
        match channel::attempt(&canonical, hand_off, channel::FORMAT, user, deadline) {
            channel::Attempt::Answered(Answer::Taken) => return Start::Taken,
            channel::Attempt::Answered(Answer::Refused(why)) => return Start::Ended(why),
            channel::Attempt::NotOwn => {
                return Start::Ended(format!(
                    "the channel of the instance on {} is another user's: nothing sent",
                    canonical.display()
                ));
            }
            channel::Attempt::Unconfirmed(error) => {
                return Start::Ended(format!(
                    "the hand-off reached the instance on {}, but its answer did not come \
                     ({error}): what was asked may or may not open there, and is not sent again",
                    canonical.display()
                ));
            }
            channel::Attempt::Unreached => {}
        }
        if Instant::now() >= deadline {
            return Start::Ended(format!(
                "Apertrust runs on the user data folder {}, but is not answering",
                canonical.display()
            ));
        }
        std::thread::sleep(channel::RETRY_STEP);
    }
}

/// [`start`] on macOS, which has no channel: a start that finds the lock held is refused.
#[cfg(not(any(windows, target_os = "linux")))]
fn start_as(
    user_data: &Path,
    _request: impl FnOnce() -> HandOff,
    handler: Handler,
    user: u32,
) -> Start {
    let canonical = match std::fs::canonicalize(user_data) {
        Ok(canonical) => canonical,
        Err(e) => return Start::Alone(format!("{}: {e}", user_data.display())),
    };
    match try_lock(&canonical.join(LOCK)) {
        Lock::Taken(lock) => Start::Instance(Instance::begin(lock, &canonical, handler, user)),
        Lock::Held => Start::Ended(running_words(&canonical)),
        Lock::Unusable(why) => Start::Alone(why),
    }
}

/// What a start refused on macOS says on stderr: the app runs on the folder `folder`.
#[cfg(not(any(windows, target_os = "linux")))]
fn running_words(folder: &Path) -> String {
    format!(
        "Apertrust is already running with the user data folder {}",
        folder.display()
    )
}

/// The running instance: the lock held and, on Windows and Linux, the channel bound and
/// served, until it is released or dropped.
pub struct Instance {
    lock: Option<File>,
    #[cfg(any(windows, target_os = "linux"))]
    channel: Option<channel::Served>,
}

impl Instance {
    /// The instance on `canonical` once its `lock` is taken, its channel served by `handler`
    /// for `user`. A channel that cannot be made leaves the instance without one, with a
    /// warning: a second start then finds it not answering.
    fn begin(lock: File, canonical: &Path, handler: Handler, user: u32) -> Instance {
        #[cfg(any(windows, target_os = "linux"))]
        let channel = match channel::Served::bind(canonical, handler, user) {
            Ok(served) => Some(served),
            Err(e) => {
                let folder = canonical.display();
                tracing::warn!(target: "control", "no channel for a second start on {folder}: {e}");
                None
            }
        };
        #[cfg(not(any(windows, target_os = "linux")))]
        let _ = (canonical, handler, user);
        Instance {
            lock: Some(lock),
            #[cfg(any(windows, target_os = "linux"))]
            channel,
        }
    }

    /// Releases the channel and the lock, so the next start becomes the instance: Restart
    /// now's, before it starts it (decision 82). Dropping the instance does the same.
    pub fn release(mut self) {
        self.close();
    }

    fn close(&mut self) {
        #[cfg(any(windows, target_os = "linux"))]
        if let Some(channel) = self.channel.take() {
            channel.stop();
        }
        drop(self.lock.take());
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.close();
    }
}

/// The flags of `hand_off` that the instance refuses, with its own cache folder `cache`:
/// every flag for one run, and `--cache-dir` where it names another folder. None where it
/// is taken.
pub fn refused(hand_off: &HandOff, cache: &Path) -> Option<Vec<OsString>> {
    let mut flags = hand_off.one_run.clone();
    if let Some(given) = &hand_off.cache
        && !same_folder(given, cache)
    {
        flags.extend([
            apertrust_base::folders::CACHE_FLAGS[0].into(),
            given.clone().into_os_string(),
        ]);
    }
    (!flags.is_empty()).then_some(flags)
}

/// Whether `a` and `b` are one folder: by their canonical paths where both exist, else as
/// written.
pub fn same_folder(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The flags `flags` as one line, as they were typed.
pub fn flags_line(flags: &[OsString]) -> String {
    let flags: Vec<_> = flags.iter().map(|flag| flag.to_string_lossy()).collect();
    flags.join(" ")
}

/// What a refused start says on stderr: the instance runs on the folder `folder`, and
/// must be quit before a start with `flags`.
pub fn refused_words(folder: &Path, flags: &[OsString]) -> String {
    format!(
        "Apertrust is already running with the user data folder {}: quit it first, then start \
         again with {}",
        folder.display(),
        flags_line(flags)
    )
}

/// What a start is told when the instance can no longer open what it hands over, its
/// window gone as it quits.
pub fn closing_words(folder: &Path) -> String {
    format!(
        "Apertrust is quitting on the user data folder {}: start again once it has",
        folder.display()
    )
}

/// What reached the instance, for its window.
#[derive(Clone, Debug, PartialEq)]
pub enum Arrival {
    /// A hand-off taken: its paths, absolute, none for a start with none, which only
    /// brings the window forward; when it came, and its token.
    Open {
        paths: Vec<PathBuf>,
        at: Instant,
        token: Token,
    },
    /// A start refused for its flags for one run: the alert says so.
    Refused { flags: Vec<OsString>, token: Token },
}

/// The instance's thread's end of what it hands the window.
#[derive(Clone)]
pub struct Post {
    sender: mpsc::Sender<Arrival>,
    wake: Waker,
}

/// The window's end: what arrived, and how its thread wakes it.
pub struct Inbox {
    receiver: mpsc::Receiver<Arrival>,
    wake: Waker,
}

/// How the window is woken, and whether it still takes arrivals: one lock for both, so an
/// arrival is either delivered before the inbox closes or refused after.
type Waker = Arc<Mutex<Door>>;

#[derive(Default)]
struct Door {
    wake: Option<Box<dyn Fn() + Send>>,
    closed: bool,
}

/// A post and the inbox it delivers to.
pub fn mailbox() -> (Post, Inbox) {
    let (sender, receiver) = mpsc::channel();
    let wake = Waker::default();
    let post = Post {
        sender,
        wake: Arc::clone(&wake),
    };
    (post, Inbox { receiver, wake })
}

impl Post {
    /// Delivers `arrival`, waking the window where it has said how; false where it cannot
    /// be, the inbox closed or gone as the app ends.
    pub fn send(&self, arrival: Arrival) -> bool {
        let Ok(door) = self.wake.lock() else {
            return false;
        };
        if door.closed || self.sender.send(arrival).is_err() {
            return false;
        }
        if let Some(wake) = door.wake.as_ref() {
            wake();
        }
        true
    }
}

impl Inbox {
    /// Wakes the window with `wake` as each arrival comes from now on.
    pub fn wake_with(&self, wake: impl Fn() + Send + 'static) {
        if let Ok(mut door) = self.wake.lock() {
            door.wake = Some(Box::new(wake));
        }
    }

    /// Takes no more arrivals, as the app commits to quitting: each later [`Post::send`]
    /// is refused. Returns what arrived since the last [`Inbox::take`], which can no longer
    /// open.
    pub fn close(&self) -> Vec<Arrival> {
        if let Ok(mut door) = self.wake.lock() {
            door.closed = true;
        }
        self.take()
    }

    /// Everything that arrived since the last call, in order.
    pub fn take(&self) -> Vec<Arrival> {
        self.receiver.try_iter().collect()
    }
}

/// The hand-offs' joins into opens: each open the paths of the hand-offs it joined, each
/// hand-off's apart, in the order they came, for the window to open as `dropped`'s
/// `joined` has it, so that raws handed in after a folder are not lost to it.
///
/// - **While the window has not shown** every hand-off joins the instance's own command
///   line, however long the start takes, and nothing opens.
/// - **Once it shows,** a hand-off arriving within `interval` of the one before joins it,
///   and the joined paths open once none has arrived for `interval`: on Windows [`JOIN`],
///   since Explorer starts the app once per file; elsewhere none, so each opens at once.
/// - A hand-off with no paths joins nothing; it only brings the window forward.
/// - **Every open comes,** in order: a hand-off that joins none comes after the open waiting
///   before it, however many arrive before the window takes them.
#[derive(Debug)]
pub struct Join {
    interval: Duration,
    shown: bool,
    batch: Vec<Vec<PathBuf>>,
    last: Instant,
    /// Opens that no later hand-off can join, oldest first, each due at once.
    ready: VecDeque<Vec<Vec<PathBuf>>>,
}

impl Join {
    /// The joins of an instance whose own command line opens `own`, started at `started`.
    pub fn new(own: Vec<PathBuf>, started: Instant, interval: Duration) -> Join {
        let mut join = Join {
            interval,
            shown: false,
            batch: Vec::new(),
            last: started,
            ready: VecDeque::new(),
        };
        join.arrive(own, started);
        join
    }

    /// The interval of this platform's joins: [`JOIN`] on Windows, none elsewhere.
    pub fn platform() -> Duration {
        if cfg!(windows) { JOIN } else { Duration::ZERO }
    }

    /// A hand-off's `paths`, arrived `at`.
    pub fn arrive(&mut self, paths: Vec<PathBuf>, at: Instant) {
        if paths.is_empty() {
            return;
        }
        let joins = !self.shown || at.saturating_duration_since(self.last) < self.interval;
        if !joins && !self.batch.is_empty() {
            self.ready.push_back(std::mem::take(&mut self.batch));
        }
        self.batch.push(paths);
        self.last = self.last.max(at);
    }

    /// The window has shown.
    pub fn show(&mut self) {
        self.shown = true;
    }

    pub fn shown(&self) -> bool {
        self.shown
    }

    /// Whether an open waits: the start window is not shown meanwhile.
    pub fn pending(&self) -> bool {
        !self.batch.is_empty() || !self.ready.is_empty()
    }

    /// The hand-offs of the oldest open due `now`, one an open; the next is taken once it
    /// has opened.
    pub fn take(&mut self, now: Instant) -> Option<Vec<Vec<PathBuf>>> {
        if !self.shown {
            return None;
        }
        if let Some(paths) = self.ready.pop_front() {
            return Some(paths);
        }
        if self.batch.is_empty() || now.saturating_duration_since(self.last) < self.interval {
            return None;
        }
        Some(std::mem::take(&mut self.batch))
    }

    /// How long until an open is due, for the window to wake then.
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        if !self.shown {
            return None;
        }
        if !self.ready.is_empty() {
            return Some(Duration::ZERO);
        }
        (!self.batch.is_empty()).then(|| {
            self.interval
                .saturating_sub(now.saturating_duration_since(self.last))
        })
    }
}

/// The channel of Windows and Linux: the second start's side, the instance's, and the
/// message between them.
#[cfg(any(windows, target_os = "linux"))]
mod channel {
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Read, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use super::{Answer, HandOff, Handler, Token};

    #[cfg(target_os = "linux")]
    pub use socket::{Listener, Stream, connect, wake};

    #[cfg(windows)]
    pub use pipe::{Listener, Stream, connect, wake};

    /// The message's format, which both ends must speak: raised whenever a message changes.
    pub const FORMAT: u32 = 1;

    /// How long a second start that cannot reach the instance keeps trying.
    pub const RETRY: Duration = Duration::from_secs(2);

    /// How long a start waits between tries.
    pub const RETRY_STEP: Duration = Duration::from_millis(50);

    /// The first field of every message, so a channel that is not the instance's is told.
    pub const MAGIC: &[u8] = b"apertrust-instance";

    /// The most fields a message has, and the most bytes in all, past which it is refused.
    pub const MOST_FIELDS: u32 = 1 << 16;
    const MOST_BYTES: usize = 16 << 20;

    /// The first 16 hex digits of BLAKE3 over `canonical`, the user data folder's canonical
    /// path: the channel's name for that folder.
    pub fn hash(canonical: &Path) -> String {
        let digest = blake3::hash(canonical.as_os_str().as_encoded_bytes());
        digest.to_hex()[..16].to_owned()
    }

    /// How one try to reach the instance went.
    #[derive(Debug)]
    pub enum Attempt {
        Answered(Answer),
        /// The server runs as another user: nothing was sent.
        NotOwn,
        /// No channel yet, or one that refused or broke off before the hand-off was sent
        /// whole: tried again.
        Unreached,
        /// The hand-off was sent whole, but no answer came: never sent again, since the
        /// instance may have taken it.
        Unconfirmed(io::Error),
    }

    /// One try to hand `hand_off` in format `format` to the instance on `canonical`, its
    /// server checked to run as `user` first, the whole exchange over by `deadline`.
    pub fn attempt(
        canonical: &Path,
        hand_off: &HandOff,
        format: u32,
        user: u32,
        deadline: Instant,
    ) -> Attempt {
        let mut stream = match connect(canonical, user) {
            Ok(Some(stream)) => stream,
            Ok(None) => return Attempt::NotOwn,
            Err(_) => return Attempt::Unreached,
        };
        let hand_off = hand_off.clone();
        let exchanged = within(deadline, move |turn| {
            exchange(&mut stream, &hand_off, format, turn)
        });
        match exchanged {
            Ok(answer) => Attempt::Answered(answer),
            Err(Failed::Unsent) => Attempt::Unreached,
            Err(Failed::Unconfirmed(error)) => Attempt::Unconfirmed(error),
        }
    }

    /// Runs `work`, an exchange, on a thread of its own, and waits for it no later than
    /// `deadline`: Windows' pipe has no timeouts, so an instance that never greets or never
    /// answers would hold a read for ever. Past the deadline the exchange is given up as
    /// unsent, `work` then never sending; or, once `work` has taken the [`Turn`] it is
    /// given to send, counts as sent without an answer. Its thread is left to end with the
    /// process.
    pub fn within(
        deadline: Instant,
        work: impl FnOnce(&Turn) -> Result<Answer, Failed> + Send + 'static,
    ) -> Result<Answer, Failed> {
        let turn = Arc::new(Turn(AtomicU8::new(Turn::WAITING)));
        let marked = Arc::clone(&turn);
        let (answer, answered) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("hand-off".into())
            .spawn(move || {
                let _ = answer.send(work(&marked));
            });
        if spawned.is_err() {
            return Err(Failed::Unsent);
        }
        match answered.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(exchanged) => exchanged,
            Err(_) if turn.give_up() => Err(Failed::Unsent),
            Err(_) => Err(Failed::Unconfirmed(io::ErrorKind::TimedOut.into())),
        }
    }

    /// An exchange's one turn, taken by whichever comes first: its thread, to send the
    /// hand-off, or the side waiting for it, to give it up at the deadline. So a hand-off
    /// is never sent once its start has said it was not.
    pub struct Turn(AtomicU8);

    impl Turn {
        const WAITING: u8 = 0;
        const SENDING: u8 = 1;
        const GIVEN_UP: u8 = 2;

        /// Takes the turn to send: false where the exchange was given up, nothing to be
        /// sent.
        pub fn send(&self) -> bool {
            let taken = self.0.compare_exchange(
                Turn::WAITING,
                Turn::SENDING,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            taken.is_ok()
        }

        /// Gives the exchange up, unless sending has begun: whether it was.
        fn give_up(&self) -> bool {
            let taken = self.0.compare_exchange(
                Turn::WAITING,
                Turn::GIVEN_UP,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            taken.is_ok()
        }
    }

    /// How an exchange failed: before the hand-off was sent whole, which the instance
    /// cannot have taken, since it reads only a whole message; or after, waiting for its
    /// answer.
    #[derive(Debug)]
    pub enum Failed {
        Unsent,
        Unconfirmed(io::Error),
    }

    /// A second start's side of the exchange on `stream`: the instance's greeting, the
    /// foreground right handed to its process on Windows, `hand_off` sent in format
    /// `format` once `turn` is taken, then the answer.
    fn exchange(
        stream: &mut Stream,
        hand_off: &HandOff,
        format: u32,
        turn: &Turn,
    ) -> Result<Answer, Failed> {
        let hello = read_frame(stream).map_err(|_| Failed::Unsent)?;
        let process = hello_process(&hello).map_err(|_| Failed::Unsent)?;
        // Before the hand-off, so the instance may come forward as it takes it.
        let _ = apertrust_display::instance::allow_foreground(process);
        // Before the first byte, so a deadline that passes while it is written never counts
        // as unsent, to be sent again; and never once the exchange was given up.
        if !turn.send() {
            return Err(Failed::Unsent);
        }
        write_frame(stream, &request_fields(hand_off, format)).map_err(|_| Failed::Unsent)?;
        stream.flush().map_err(Failed::Unconfirmed)?;
        let answer = read_frame(stream).map_err(Failed::Unconfirmed)?;
        answer_of(&answer).map_err(Failed::Unconfirmed)
    }

    /// The instance's channel, served on a thread of its own, each second start on a thread
    /// of its own: a client that connects and never writes holds only its own thread, never
    /// the next start's hand-off.
    pub struct Served {
        stop: Arc<AtomicBool>,
        thread: JoinHandle<()>,
        /// What a connection of its own goes to, to wake the thread for the stop.
        canonical: PathBuf,
        user: u32,
        serving: Arc<Mutex<Vec<JoinHandle<()>>>>,
    }

    impl Served {
        /// The channel of the instance on `canonical`, bound and served by `handler`, its
        /// connections from `user` alone.
        pub fn bind(canonical: &Path, handler: Handler, user: u32) -> io::Result<Served> {
            let mut listener = Listener::bind(canonical, user)?;
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let serving: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::default();
            let started = Arc::clone(&serving);
            let thread = std::thread::Builder::new()
                .name("instance".into())
                .spawn(move || {
                    loop {
                        let accepted = listener.accept();
                        if stopped.load(Ordering::Acquire) {
                            break;
                        }
                        let Ok(mut stream) = accepted else {
                            std::thread::sleep(RETRY_STEP);
                            continue;
                        };
                        let handler = Arc::clone(&handler);
                        let served = std::thread::Builder::new()
                            .name("instance start".into())
                            .spawn(move || {
                                let served = serve(&mut stream, &handler);
                                // Until the second start has read the answer.
                                #[cfg(windows)]
                                if served.is_ok() {
                                    let _ = stream.sync_all();
                                }
                                #[cfg(not(windows))]
                                let _ = served;
                            });
                        match served {
                            Ok(served) => {
                                if let Ok(mut threads) = started.lock() {
                                    threads.retain(|thread| !thread.is_finished());
                                    threads.push(served);
                                }
                            }
                            Err(e) => {
                                tracing::warn!(target: "control", "a second start not answered: no thread: {e}");
                            }
                        }
                    }
                })?;
            Ok(Served {
                stop,
                thread,
                canonical: canonical.to_owned(),
                user,
                serving,
            })
        }

        /// Stops serving: the thread woken by a connection of its own and joined, then each
        /// start's thread, for a while at most, so its end of the channel is closed and the
        /// next instance can bind it.
        pub fn stop(self) {
            self.stop.store(true, Ordering::Release);
            if wake(&self.canonical, self.user) {
                let _ = self.thread.join();
            }
            if let Ok(mut serving) = self.serving.lock() {
                wait_for(std::mem::take(&mut *serving), Instant::now() + RETRY);
            }
        }
    }

    /// Waits for `threads` until `deadline`, joining each that has ended; one still running
    /// then, as a Windows start that never writes holds its read, is left to the process's
    /// end.
    pub fn wait_for(threads: Vec<JoinHandle<()>>, deadline: Instant) {
        let mut running = threads;
        loop {
            let (ended, still): (Vec<_>, Vec<_>) =
                running.into_iter().partition(JoinHandle::is_finished);
            for thread in ended {
                let _ = thread.join();
            }
            if still.is_empty() || Instant::now() >= deadline {
                return;
            }
            running = still;
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The instance's side of one exchange on `stream`: its greeting with its process id,
    /// then the hand-off's answer, `handler`'s for one of its format.
    fn serve(stream: &mut Stream, handler: &Handler) -> io::Result<()> {
        write_frame(
            stream,
            &[
                MAGIC.to_vec(),
                FORMAT.to_le_bytes().to_vec(),
                std::process::id().to_le_bytes().to_vec(),
            ],
        )?;
        stream.flush()?;
        let answer = match hand_off_of(&read_frame(stream)?)? {
            Request::HandOff(hand_off) => handler(hand_off),
            Request::OtherFormat => Answer::Refused(ANOTHER_VERSION.to_owned()),
        };
        write_frame(stream, &answer_fields(&answer))?;
        stream.flush()
    }

    /// The refusal of a hand-off of another format.
    const ANOTHER_VERSION: &str =
        "another version of Apertrust is running with this user data folder: quit it first";

    /// What a second start's message turned out to be.
    pub enum Request {
        HandOff(HandOff),
        /// Another format, as another build sends: refused, saying another version runs.
        OtherFormat,
    }

    /// The fields of `hand_off` in format `format`: the magic and the format first, which
    /// every format keeps; then the folder, the tokens, the cache folder, the flags for one
    /// run counted, and the arguments, each an empty field where there is none.
    pub fn request_fields(hand_off: &HandOff, format: u32) -> Vec<Vec<u8>> {
        let text = |text: &Option<String>| text.clone().unwrap_or_default().into_bytes();
        let mut fields = vec![
            MAGIC.to_vec(),
            format.to_le_bytes().to_vec(),
            os_bytes(hand_off.folder.as_os_str()),
            text(&hand_off.token.wayland),
            text(&hand_off.token.x11),
            hand_off
                .cache
                .as_deref()
                .map(|cache| os_bytes(cache.as_os_str()))
                .unwrap_or_default(),
            (hand_off.one_run.len() as u32).to_le_bytes().to_vec(),
        ];
        fields.extend(hand_off.one_run.iter().map(|flag| os_bytes(flag)));
        fields.extend(hand_off.arguments.iter().map(|argument| os_bytes(argument)));
        fields
    }

    /// The hand-off `fields` hold, or another format's.
    pub fn hand_off_of(fields: &[Vec<u8>]) -> io::Result<Request> {
        if fields.first().map(Vec::as_slice) != Some(MAGIC) {
            return Err(garbled("not a hand-off"));
        }
        if fields.get(1).and_then(|format| number(format)) != Some(FORMAT) {
            return Ok(Request::OtherFormat);
        }
        let [_, _, folder, wayland, x11, cache, count, rest @ ..] = fields else {
            return Err(garbled("a hand-off's fields missing"));
        };
        let count = number(count).ok_or_else(|| garbled("the flags' count"))? as usize;
        if count > rest.len() {
            return Err(garbled("fewer flags than counted"));
        }
        let text = |field: &Vec<u8>| -> io::Result<Option<String>> {
            let text =
                String::from_utf8(field.clone()).map_err(|_| garbled("a token not UTF-8"))?;
            Ok((!text.is_empty()).then_some(text))
        };
        let strings = |fields: &[Vec<u8>]| -> io::Result<Vec<OsString>> {
            fields
                .iter()
                .map(|field| os_string(field.clone()))
                .collect()
        };
        let (one_run, arguments) = rest.split_at(count);
        Ok(Request::HandOff(HandOff {
            arguments: strings(arguments)?,
            folder: os_string(folder.clone())?.into(),
            token: Token {
                wayland: text(wayland)?,
                x11: text(x11)?,
            },
            one_run: strings(one_run)?,
            cache: (!cache.is_empty())
                .then(|| os_string(cache.clone()).map(PathBuf::from))
                .transpose()?,
        }))
    }

    /// The process id the instance's greeting gives, once it is told as the instance's.
    fn hello_process(fields: &[Vec<u8>]) -> io::Result<u32> {
        match fields {
            [magic, _format, process] if magic == MAGIC => {
                number(process).ok_or_else(|| garbled("the instance's process"))
            }
            _ => Err(garbled("not the instance's greeting")),
        }
    }

    pub fn answer_fields(answer: &Answer) -> Vec<Vec<u8>> {
        match answer {
            Answer::Taken => vec![MAGIC.to_vec(), b"taken".to_vec()],
            Answer::Refused(why) => {
                vec![MAGIC.to_vec(), b"refused".to_vec(), why.as_bytes().to_vec()]
            }
        }
    }

    pub fn answer_of(fields: &[Vec<u8>]) -> io::Result<Answer> {
        match fields {
            [magic, taken] if magic == MAGIC && taken == b"taken" => Ok(Answer::Taken),
            [magic, refused, why] if magic == MAGIC && refused == b"refused" => {
                Ok(Answer::Refused(String::from_utf8_lossy(why).into_owned()))
            }
            _ => Err(garbled("not an answer")),
        }
    }

    fn number(field: &[u8]) -> Option<u32> {
        Some(u32::from_le_bytes(field.try_into().ok()?))
    }

    fn garbled(what: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, what.to_owned())
    }

    /// Writes `fields` as one message: their count, then each field's length and bytes,
    /// every number four bytes little-endian.
    pub fn write_frame(stream: &mut impl Write, fields: &[Vec<u8>]) -> io::Result<()> {
        let mut out = (fields.len() as u32).to_le_bytes().to_vec();
        for field in fields {
            out.extend((field.len() as u32).to_le_bytes());
            out.extend(field);
        }
        stream.write_all(&out)
    }

    /// Reads one message [`write_frame`] wrote, refusing one past [`MOST_FIELDS`] or
    /// [`MOST_BYTES`].
    pub fn read_frame(stream: &mut impl Read) -> io::Result<Vec<Vec<u8>>> {
        let mut number = || -> io::Result<u32> {
            let mut bytes = [0; 4];
            stream.read_exact(&mut bytes)?;
            Ok(u32::from_le_bytes(bytes))
        };
        let count = number()?;
        if count > MOST_FIELDS {
            return Err(garbled("too many fields"));
        }
        let mut fields = Vec::with_capacity(count as usize);
        let mut total = 0;
        for _ in 0..count {
            let mut bytes = [0; 4];
            stream.read_exact(&mut bytes)?;
            let length = u32::from_le_bytes(bytes) as usize;
            total += length;
            if total > MOST_BYTES {
                return Err(garbled("too long"));
            }
            let mut field = vec![0; length];
            stream.read_exact(&mut field)?;
            fields.push(field);
        }
        Ok(fields)
    }

    /// `text`'s bytes as the platform keeps them: Unix's bytes, Windows' UTF-16 units.
    #[cfg(unix)]
    fn os_bytes(text: &OsStr) -> Vec<u8> {
        use std::os::unix::ffi::OsStrExt as _;
        text.as_bytes().to_vec()
    }

    #[cfg(unix)]
    fn os_string(bytes: Vec<u8>) -> io::Result<OsString> {
        use std::os::unix::ffi::OsStringExt as _;
        Ok(OsString::from_vec(bytes))
    }

    #[cfg(windows)]
    fn os_bytes(text: &OsStr) -> Vec<u8> {
        use std::os::windows::ffi::OsStrExt as _;
        text.encode_wide().flat_map(u16::to_le_bytes).collect()
    }

    #[cfg(windows)]
    fn os_string(bytes: Vec<u8>) -> io::Result<OsString> {
        use std::os::windows::ffi::OsStringExt as _;
        let (units, rest) = bytes.as_chunks::<2>();
        if !rest.is_empty() {
            return Err(garbled("an odd number of bytes"));
        }
        let units: Vec<u16> = units.iter().map(|unit| u16::from_le_bytes(*unit)).collect();
        Ok(OsString::from_wide(&units))
    }

    /// Linux's channel: an abstract Unix socket, which has no file, each end checking the
    /// other's user (`SO_PEERCRED`).
    #[cfg(target_os = "linux")]
    pub mod socket {
        use std::io;
        use std::os::linux::net::SocketAddrExt as _;
        use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
        use std::path::Path;

        use apertrust_display::instance::peer_user;

        use super::RETRY;

        pub type Stream = UnixStream;

        /// The socket's address for the folder `canonical` and `user`:
        /// `apertrust-<uid>-<hash>` in the abstract namespace.
        pub fn address(canonical: &Path, user: u32) -> io::Result<SocketAddr> {
            let name = format!("apertrust-{user}-{}", super::hash(canonical));
            SocketAddr::from_abstract_name(name)
        }

        /// Connects to the instance on `canonical` and checks that its server runs as
        /// `user`, before anything is sent: None where it does not.
        pub fn connect(canonical: &Path, user: u32) -> io::Result<Option<Stream>> {
            let stream = UnixStream::connect_addr(&address(canonical, user)?)?;
            Ok((peer_user(&stream)? == user).then_some(stream))
        }

        /// Wakes the instance's thread waiting for a connection: whether it could be.
        pub fn wake(canonical: &Path, user: u32) -> bool {
            address(canonical, user)
                .and_then(|address| UnixStream::connect_addr(&address))
                .is_ok()
        }

        /// The instance's end of the socket.
        pub struct Listener {
            listener: UnixListener,
            user: u32,
        }

        impl Listener {
            /// Binds the socket, failing where another process, of any user, has its name.
            pub fn bind(canonical: &Path, user: u32) -> io::Result<Listener> {
                let listener = UnixListener::bind_addr(&address(canonical, user)?)?;
                Ok(Listener { listener, user })
            }

            /// The next connection, from the user's own account, each read and write limited
            /// to [`RETRY`]; another user's is dropped unanswered, an error.
            pub fn accept(&mut self) -> io::Result<Stream> {
                let (stream, _) = self.listener.accept()?;
                if !peer_user(&stream).is_ok_and(|peer| peer == self.user) {
                    return Err(io::ErrorKind::PermissionDenied.into());
                }
                stream.set_read_timeout(Some(RETRY))?;
                stream.set_write_timeout(Some(RETRY))?;
                Ok(stream)
            }
        }
    }

    /// Windows' channel: a named pipe, open to the user's own account alone, its server
    /// checked to run as the same user (`apertrust-display`).
    #[cfg(windows)]
    mod pipe {
        use std::fs::File;
        use std::io;
        use std::path::Path;

        use apertrust_display::instance::{PipeListener, connect_pipe, pipe_server_is_own};

        pub type Stream = File;

        /// The pipe's name for the folder `canonical`: `\\.\pipe\apertrust-<hash>`.
        fn name(canonical: &Path) -> String {
            format!(r"\\.\pipe\apertrust-{}", super::hash(canonical))
        }

        /// Connects to the instance on `canonical` and checks that its server runs as this
        /// process's user: None where it does not.
        pub fn connect(canonical: &Path, _user: u32) -> io::Result<Option<Stream>> {
            let pipe = connect_pipe(&name(canonical))?;
            let (own, _) = pipe_server_is_own(&pipe)?;
            Ok(own.then_some(pipe))
        }

        /// Wakes the instance's thread waiting for a connection: whether it could be.
        pub fn wake(canonical: &Path, _user: u32) -> bool {
            connect_pipe(&name(canonical)).is_ok()
        }

        /// The instance's end of the pipe.
        pub struct Listener(PipeListener);

        impl Listener {
            /// Creates the pipe as the first instance of its name.
            pub fn bind(canonical: &Path, _user: u32) -> io::Result<Listener> {
                PipeListener::bind(&name(canonical)).map(Listener)
            }

            /// The next second start's end of the pipe.
            pub fn accept(&mut self) -> io::Result<Stream> {
                self.0.accept()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead as _, BufReader, Read as _};
    use std::process::{Child, ChildStdout, Command, Stdio};

    use super::*;

    /// A test's own user data folder, removed as it drops.
    struct Fixture {
        root: PathBuf,
        user_data: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let root = std::env::temp_dir().join(format!("ai-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let user_data = root.join("u");
            std::fs::create_dir_all(&user_data).unwrap();
            Fixture { root, user_data }
        }

        fn canonical(&self) -> PathBuf {
            std::fs::canonicalize(&self.user_data).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// A handler as the app's decides, with the user data folder's `cache` folder as the
    /// instance's: what it takes printed on stdout, for a test in another process.
    fn printing(cache: PathBuf, folder: PathBuf) -> Handler {
        Arc::new(move |hand_off| {
            if let Some(flags) = refused(&hand_off, &cache) {
                return Answer::Refused(refused_words(&folder, &flags));
            }
            println!("got {:?} in {:?}", hand_off.arguments, hand_off.folder);
            Answer::Taken
        })
    }

    fn taking() -> Handler {
        Arc::new(|_| Answer::Taken)
    }

    fn photo() -> HandOff {
        HandOff {
            arguments: vec!["IMG_2041.ARW".into()],
            folder: PathBuf::from("/photos"),
            ..HandOff::default()
        }
    }

    const ROLE: &str = "APERTRUST_INSTANCE_TEST_ROLE";
    const ROOT: &str = "APERTRUST_INSTANCE_TEST_ROOT";

    /// A process of the app's tests, the instance or the lock's holder, as another test
    /// starts it: nothing without its role, so it passes as a test of its own. Never a
    /// window: it takes the lock, serves or not, says it is ready and runs until its
    /// stdin closes.
    #[test]
    fn instance_process() {
        let (Some(role), Some(root)) = (std::env::var_os(ROLE), std::env::var_os(ROOT)) else {
            return;
        };
        let root = PathBuf::from(root);
        let fixture = Fixture {
            user_data: root.join("u"),
            root,
        };
        let held = if role == "serve" {
            let cache = fixture.user_data.join("cache");
            let handler = printing(cache, fixture.canonical());
            match start(&fixture.user_data, HandOff::default, handler) {
                Start::Instance(instance) => Some(instance),
                _ => panic!("not the instance"),
            }
        } else {
            let lock = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(fixture.user_data.join(LOCK))
                .unwrap();
            lock.try_lock().unwrap();
            std::mem::forget(lock);
            None
        };
        println!("ready");
        let _ = std::io::stdin().read_to_end(&mut Vec::new());
        drop(held);
        // The fixture is the starting test's to remove.
        std::mem::forget(fixture);
    }

    /// The test executable itself, as [`instance_process`] in `role` on `fixture`, once it
    /// has said it is ready: its stdout, to read what it took.
    fn spawn(role: &str, fixture: &Fixture) -> (Child, BufReader<ChildStdout>) {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "instance::tests::instance_process",
                "--nocapture",
            ])
            .args(["--test-threads", "1", "-q"])
            .env(ROLE, role)
            .env(ROOT, &fixture.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let ready = line(&mut out, "ready");
        assert_eq!(ready, "ready");
        (child, out)
    }

    /// The next line of `out` that starts with `start`.
    fn line(out: &mut BufReader<ChildStdout>, start: &str) -> String {
        loop {
            let mut line = String::new();
            assert_ne!(
                out.read_line(&mut line).unwrap(),
                0,
                "ended before {start:?}"
            );
            if let Some(found) = line.trim_end().find(start) {
                return line.trim_end()[found..].to_owned();
            }
        }
    }

    /// Ends a process [`spawn`] started, its stdin closed.
    fn end(mut child: Child) {
        drop(child.stdin.take());
        child.wait().unwrap();
    }

    fn second(fixture: &Fixture, hand_off: HandOff) -> Start {
        start(&fixture.user_data, || hand_off, taking())
    }

    /// The first start is the instance; a second, in another process, hands it its
    /// arguments and working folder, and is taken.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_second_start_is_taken() {
        let fixture = Fixture::new("taken");
        let (child, mut out) = spawn("serve", &fixture);
        assert!(matches!(second(&fixture, photo()), Start::Taken));
        assert_eq!(
            line(&mut out, "got"),
            r#"got ["IMG_2041.ARW"] in "/photos""#
        );
        end(child);
    }

    /// On macOS a second start, in another process, is refused: Apertrust is already
    /// running with that folder.
    #[cfg(not(any(windows, target_os = "linux")))]
    #[test]
    fn a_second_start_is_refused_on_macos() {
        let fixture = Fixture::new("macos");
        let (child, _out) = spawn("serve", &fixture);
        let Start::Ended(why) = second(&fixture, photo()) else {
            panic!("not refused");
        };
        assert!(why.contains("already running"), "{why}");
        end(child);
    }

    /// A start with `--conf` while the instance runs is refused, saying so; and one naming
    /// another cache folder; the instance's own cache folder is taken.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_start_with_flags_for_one_run_is_refused() {
        let fixture = Fixture::new("refused");
        let (child, mut out) = spawn("serve", &fixture);
        let conf = HandOff {
            one_run: vec!["--conf".into(), "ui.theme=light".into()],
            ..photo()
        };
        let Start::Ended(why) = second(&fixture, conf) else {
            panic!("not refused");
        };
        assert!(why.contains("already running"), "{why}");
        assert!(why.contains("--conf ui.theme=light"), "{why}");
        let other = HandOff {
            cache: Some(fixture.root.join("elsewhere")),
            ..photo()
        };
        let Start::Ended(why) = second(&fixture, other) else {
            panic!("not refused");
        };
        assert!(why.contains("--cache-dir"), "{why}");
        let own = HandOff {
            cache: Some(fixture.user_data.join("cache")),
            ..photo()
        };
        assert!(matches!(second(&fixture, own), Start::Taken));
        line(&mut out, "got");
        end(child);
    }

    /// A start that finds the lock held and no channel tries for two seconds, then says
    /// the instance is not answering.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_second_start_with_no_answer_gives_up() {
        let fixture = Fixture::new("silent");
        let (child, _out) = spawn("hold", &fixture);
        let began = Instant::now();
        let Start::Ended(why) = second(&fixture, photo()) else {
            panic!("not ended");
        };
        assert!(why.contains("not answering"), "{why}");
        assert!(began.elapsed() >= channel::RETRY, "{:?}", began.elapsed());
        end(child);
    }

    /// Another build's format is refused, saying another version runs.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn another_format_is_refused() {
        let fixture = Fixture::new("format");
        let (child, _out) = spawn("serve", &fixture);
        let attempted = channel::attempt(
            &fixture.canonical(),
            &photo(),
            channel::FORMAT + 1,
            own_user(),
            Instant::now() + channel::RETRY,
        );
        let channel::Attempt::Answered(Answer::Refused(why)) = attempted else {
            panic!("not refused: {attempted:?}");
        };
        assert!(why.contains("another version"), "{why}");
        end(child);
    }

    /// Restart now's release leaves the folder to the next start, which becomes the
    /// instance; until then a start on it is a second start, taken where there is a
    /// channel, refused on macOS.
    #[test]
    fn a_released_instance_leaves_the_folder_to_the_next_start() {
        let fixture = Fixture::new("release");
        let Start::Instance(first) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        let next = second(&fixture, photo());
        if cfg!(any(windows, target_os = "linux")) {
            assert!(matches!(next, Start::Taken));
        } else {
            assert!(matches!(next, Start::Ended(_)));
        }
        first.release();
        let Start::Instance(next) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        drop(next);
    }

    /// An instance killed leaves nothing behind but its lock file, which the system has
    /// unlocked: the next start is the instance, and serves at once.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_killed_instance_leaves_the_folder_to_the_next_start() {
        let fixture = Fixture::new("killed");
        let (mut child, _out) = spawn("serve", &fixture);
        child.kill().unwrap();
        child.wait().unwrap();
        let Start::Instance(instance) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        assert!(matches!(second(&fixture, photo()), Start::Taken));
        drop(instance);
    }

    /// The lock file holds nothing, the channel's name included.
    #[test]
    fn the_lock_file_holds_nothing() {
        let fixture = Fixture::new("empty");
        let Start::Instance(instance) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        let held = std::fs::read(fixture.user_data.join(LOCK)).unwrap();
        assert!(held.is_empty(), "{held:?}");
        drop(instance);
    }

    /// An instance of the test's own on `fixture`, in this process: the lock held, the
    /// socket of `user`'s name served by `serve` for each connection, what each served
    /// sent on the receiver it gives. Its thread runs until the test process ends.
    #[cfg(target_os = "linux")]
    fn fake_instance(
        fixture: &Fixture,
        user: u32,
        serve: fn(&mut channel::Stream) -> usize,
    ) -> (File, mpsc::Receiver<usize>) {
        let Lock::Taken(lock) = try_lock(&fixture.user_data.join(LOCK)) else {
            panic!("not locked");
        };
        let address = channel::socket::address(&fixture.canonical(), user).unwrap();
        let listener = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();
        let (served, each) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let _ = served.send(serve(&mut stream.unwrap()));
            }
        });
        (lock, each)
    }

    /// The instance's greeting, as it writes it.
    #[cfg(target_os = "linux")]
    fn greeting() -> Vec<u8> {
        let mut hello = Vec::new();
        let fields = [
            channel::MAGIC.to_vec(),
            channel::FORMAT.to_le_bytes().to_vec(),
            1u32.to_le_bytes().to_vec(),
        ];
        channel::write_frame(&mut hello, &fields).unwrap();
        hello
    }

    /// A hand-off sent whole whose answer never comes is not sent again: the start ends,
    /// saying it cannot tell whether it was taken.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_hand_off_sent_whole_is_never_sent_again() {
        use std::io::Write as _;
        let fixture = Fixture::new("unanswered");
        let (_lock, served) = fake_instance(&fixture, own_user(), |stream| {
            stream.write_all(&greeting()).unwrap();
            channel::read_frame(stream).map_or(0, |fields| fields.len())
        });
        let Start::Ended(why) = second(&fixture, photo()) else {
            panic!("not ended");
        };
        assert!(why.contains("answer did not come"), "{why}");
        assert!(served.recv_timeout(channel::RETRY).unwrap() > 0, "sent");
        assert!(served.try_recv().is_err(), "sent once");
    }

    /// The two seconds bound the whole hand-off, not each read: an instance that greets a
    /// byte at a time, each well inside the limit, is given up on once they have passed.
    #[cfg(target_os = "linux")]
    #[test]
    fn one_deadline_bounds_the_whole_hand_off() {
        use std::io::Write as _;
        let fixture = Fixture::new("slow");
        let (_lock, _served) = fake_instance(&fixture, own_user(), |stream| {
            for byte in greeting() {
                if stream.write_all(&[byte]).is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            0
        });
        let began = Instant::now();
        let Start::Ended(why) = second(&fixture, photo()) else {
            panic!("not ended");
        };
        assert!(why.contains("not answering"), "{why}");
        let took = began.elapsed();
        assert!(
            took >= channel::RETRY && took < channel::RETRY + Duration::from_secs(1),
            "{took:?}"
        );
    }

    /// A second start sends nothing to a server of another user: here one whose name is
    /// that user's, run by this one, which its credentials give away.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_second_start_sends_nothing_to_another_user() {
        let fixture = Fixture::new("other");
        let other = own_user().wrapping_add(1);
        let (_lock, served) = fake_instance(&fixture, other, |stream| {
            let mut received = Vec::new();
            let _ = stream.read_to_end(&mut received);
            received.len()
        });
        let Start::Ended(why) = start_as(&fixture.user_data, photo, taking(), other) else {
            panic!("sent");
        };
        assert!(why.contains("another user's"), "{why}");
        // Connected, then closed with nothing written.
        assert_eq!(served.recv_timeout(channel::RETRY), Ok(0));
    }

    /// The instance drops a connection from another user unanswered: here an instance that
    /// takes only another user's, reached by this one.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_instance_drops_another_user_s_connection() {
        let fixture = Fixture::new("peer");
        let other = own_user().wrapping_add(1);
        let asked = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let noted = Arc::clone(&asked);
        let handler: Handler = Arc::new(move |_| {
            noted.store(true, std::sync::atomic::Ordering::SeqCst);
            Answer::Taken
        });
        let Start::Instance(instance) = start_as(&fixture.user_data, photo, handler, other) else {
            panic!("not the instance");
        };
        let address = channel::socket::address(&fixture.canonical(), other).unwrap();
        let mut stream = std::os::unix::net::UnixStream::connect_addr(&address).unwrap();
        stream.set_read_timeout(Some(channel::RETRY)).unwrap();
        let mut greeting = Vec::new();
        assert_eq!(stream.read_to_end(&mut greeting).unwrap(), 0, "no greeting");
        assert!(!asked.load(std::sync::atomic::Ordering::SeqCst));
        drop(instance);
    }

    /// An exchange is waited for no later than its deadline, whatever its reads do, as on
    /// Windows' pipe, which has no timeouts: given up as unsent while nothing was being
    /// sent, its thread then sending nothing, or sent without an answer; a quick one gives
    /// its answer.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn an_exchange_is_waited_for_no_later_than_its_deadline() {
        use channel::{Failed, within};
        let slow = Duration::from_secs(5);
        let limit = Duration::from_millis(300);
        let began = Instant::now();
        let (late, told) = mpsc::channel();
        let greeting = within(began + limit, move |turn| {
            std::thread::sleep(limit * 2);
            let _ = late.send(turn.send());
            Ok(Answer::Taken)
        });
        assert!(matches!(greeting, Err(Failed::Unsent)), "{greeting:?}");
        assert!(began.elapsed() < limit + Duration::from_millis(250));
        let sent = told.recv_timeout(Duration::from_secs(2));
        assert_eq!(sent, Ok(false), "nothing sent once given up");
        let began = Instant::now();
        let answer = within(began + limit, move |turn| {
            assert!(turn.send());
            std::thread::sleep(slow);
            Ok(Answer::Taken)
        });
        assert!(matches!(answer, Err(Failed::Unconfirmed(_))), "{answer:?}");
        let quick = within(Instant::now() + limit, |_| Ok(Answer::Taken));
        assert!(matches!(quick, Ok(Answer::Taken)));
    }

    /// Release waits for the threads serving starts, for a while at most: those that end
    /// in time are joined, one that never would is left behind at the deadline.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn release_waits_for_the_starts_served_for_a_while() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ended = Arc::new(AtomicBool::new(false));
        let ending = Arc::clone(&ended);
        let quick = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            ending.store(true, Ordering::Release);
        });
        let stuck = std::thread::spawn(|| std::thread::sleep(Duration::from_secs(30)));
        let began = Instant::now();
        channel::wait_for(vec![quick, stuck], began + Duration::from_millis(400));
        assert!(ended.load(Ordering::Acquire), "the quick one waited for");
        let took = began.elapsed();
        assert!(
            took >= Duration::from_millis(400) && took < Duration::from_secs(2),
            "{took:?}"
        );
        let began = Instant::now();
        channel::wait_for(Vec::new(), began + Duration::from_secs(5));
        assert!(
            began.elapsed() < Duration::from_millis(100),
            "nothing to wait for"
        );
    }

    /// Every field of a hand-off comes through, a path that is not UTF-8 among them on
    /// Linux; another format is told by its first two fields alone.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn a_hand_off_comes_through_whole() {
        use channel::{
            MOST_FIELDS, Request, answer_fields, answer_of, hand_off_of, read_frame,
            request_fields, write_frame,
        };
        #[cfg(unix)]
        let odd: OsString = std::os::unix::ffi::OsStringExt::from_vec(vec![b'a', 0xff]);
        #[cfg(not(unix))]
        let odd: OsString = "ä".into();
        let hand_off = HandOff {
            arguments: vec!["-d".into(), "gpu".into(), odd.clone(), "".into()],
            folder: PathBuf::from(odd),
            token: Token {
                wayland: Some("token".into()),
                x11: None,
            },
            one_run: vec!["-d".into(), "gpu".into()],
            cache: Some(PathBuf::from("/cache")),
        };
        let mut wire = Vec::new();
        write_frame(&mut wire, &request_fields(&hand_off, channel::FORMAT)).unwrap();
        let fields = read_frame(&mut wire.as_slice()).unwrap();
        let Request::HandOff(read) = hand_off_of(&fields).unwrap() else {
            panic!("another format");
        };
        assert_eq!(read, hand_off);
        let other = request_fields(&hand_off, channel::FORMAT + 1);
        assert!(matches!(hand_off_of(&other[..2]), Ok(Request::OtherFormat)));
        assert!(hand_off_of(&[b"nonsense".to_vec()]).is_err());
        for answer in [Answer::Taken, Answer::Refused("why".into())] {
            assert_eq!(answer_of(&answer_fields(&answer)).unwrap(), answer);
        }
        let mut huge = (MOST_FIELDS + 1).to_le_bytes().to_vec();
        huge.extend([0; 8]);
        assert!(read_frame(&mut huge.as_slice()).is_err());
    }

    /// A closed inbox takes nothing more, and gives back what came before it closed.
    #[test]
    fn a_closed_inbox_takes_nothing_more() {
        let (post, inbox) = mailbox();
        let open = |name: &str| Arrival::Open {
            paths: paths(&[name]),
            at: Instant::now(),
            token: Token::default(),
        };
        assert!(post.send(open("a")));
        let left = inbox.close();
        assert!(
            matches!(&left[..], [Arrival::Open { paths: given, .. }] if *given == paths(&["a"])),
            "{left:?}"
        );
        assert!(!post.send(open("b")));
        assert!(inbox.take().is_empty());
    }

    /// The flags for one run are refused, all of them; the cache folder only where it is
    /// another than the instance's.
    #[test]
    fn the_flags_for_one_run_are_refused() {
        let cache = Path::new("/home/ana/.cache/apertrust");
        assert_eq!(refused(&photo(), cache), None);
        let safe = HandOff {
            one_run: vec!["--safe-mode".into()],
            ..photo()
        };
        assert_eq!(refused(&safe, cache), Some(vec!["--safe-mode".into()]));
        let same = HandOff {
            cache: Some(cache.to_owned()),
            ..photo()
        };
        assert_eq!(refused(&same, cache), None);
        let other = HandOff {
            cache: Some("/elsewhere".into()),
            one_run: vec!["-d".into(), "gpu".into()],
            ..photo()
        };
        let flags = refused(&other, cache).unwrap();
        assert_eq!(flags_line(&flags), "-d gpu --cache-dir /elsewhere");
    }

    /// The hash is BLAKE3's first 16 hex digits over the canonical path, the same for every
    /// way of writing the folder.
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn the_hash_is_of_the_canonical_path() {
        use channel::hash;
        let path = Path::new("/home/ana/.config/apertrust");
        let digest = blake3::hash(b"/home/ana/.config/apertrust").to_hex();
        assert_eq!(hash(path), digest[..16]);
        assert_eq!(hash(path).len(), 16);
        let fixture = Fixture::new("hash");
        let roundabout = fixture.user_data.join("..").join("u");
        let canonical = std::fs::canonicalize(roundabout).unwrap();
        assert_eq!(hash(&canonical), hash(&fixture.canonical()));
        #[cfg(unix)]
        {
            let link = fixture.root.join("link");
            std::os::unix::fs::symlink(&fixture.user_data, &link).unwrap();
            assert_eq!(
                hash(&std::fs::canonicalize(link).unwrap()),
                hash(&fixture.canonical())
            );
        }
    }

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    /// Hand-offs, each its paths.
    fn hand_offs(each: &[&[&str]]) -> Vec<Vec<PathBuf>> {
        each.iter().map(|names| paths(names)).collect()
    }

    /// While the window has not shown, every hand-off joins the instance's own command
    /// line, however long it takes, each kept apart; then on Windows those within 500 ms
    /// of the one before join, and open once none has come for 500 ms.
    #[test]
    fn hand_offs_join_while_starting_and_within_the_interval() {
        let started = Instant::now();
        let at = |ms| started + Duration::from_millis(ms);
        let mut join = Join::new(paths(&["a"]), started, JOIN);
        assert!(join.pending());
        join.arrive(paths(&["b"]), at(900));
        join.arrive(paths(&["a", "c"]), at(5000));
        assert_eq!(
            join.take(at(9000)),
            None,
            "nothing opens before the window shows"
        );
        join.show();
        assert_eq!(join.take(at(5400)), None, "within 500 ms of the last");
        assert_eq!(join.wait(at(5400)), Some(Duration::from_millis(100)));
        let joined = hand_offs(&[&["a"], &["b"], &["a", "c"]]);
        assert_eq!(join.take(at(5500)), Some(joined));
        assert!(!join.pending());
        // Running: within 500 ms of the one before, one open.
        join.arrive(paths(&["d"]), at(7000));
        join.arrive(paths(&["e"]), at(7400));
        join.arrive(paths(&["f"]), at(7800));
        assert_eq!(join.take(at(8100)), None);
        let joined = hand_offs(&[&["d"], &["e"], &["f"]]);
        assert_eq!(join.take(at(8300)), Some(joined));
        // Later than that, an open of its own; none for a start with no paths.
        join.arrive(paths(&["g"]), at(9000));
        join.arrive(Vec::new(), at(9100));
        assert_eq!(join.take(at(9500)), Some(hand_offs(&[&["g"]])));
        assert_eq!(join.take(at(9600)), None);
    }

    /// Hand-offs that join none, taken by the window together, each open in order, none
    /// lost: on Windows past the interval, elsewhere any two.
    #[test]
    fn every_open_comes_however_many_arrive_at_once() {
        let started = Instant::now();
        let at = |ms| started + Duration::from_millis(ms);
        let mut join = Join::new(Vec::new(), started, Duration::ZERO);
        join.show();
        join.arrive(paths(&["a"]), at(100));
        join.arrive(paths(&["b"]), at(100));
        join.arrive(paths(&["c"]), at(120));
        assert_eq!(join.wait(at(120)), Some(Duration::ZERO));
        assert_eq!(join.take(at(120)), Some(hand_offs(&[&["a"]])));
        assert_eq!(join.take(at(120)), Some(hand_offs(&[&["b"]])));
        assert!(join.pending());
        assert_eq!(join.take(at(120)), Some(hand_offs(&[&["c"]])));
        assert!(!join.pending());
        assert_eq!(join.take(at(120)), None);
        let mut windows = Join::new(Vec::new(), started, JOIN);
        windows.show();
        windows.arrive(paths(&["a"]), at(1000));
        windows.arrive(paths(&["b"]), at(1200));
        windows.arrive(paths(&["c"]), at(1800));
        let joined = hand_offs(&[&["a"], &["b"]]);
        assert_eq!(windows.take(at(1900)), Some(joined));
        assert_eq!(windows.take(at(1900)), None, "within 500 ms of c");
        assert_eq!(windows.take(at(2300)), Some(hand_offs(&[&["c"]])));
    }

    /// Elsewhere each hand-off once the window shows opens at once, alone; an instance with
    /// nothing of its own to open waits for nothing.
    #[test]
    fn hand_offs_open_at_once_off_windows() {
        let started = Instant::now();
        let at = |ms| started + Duration::from_millis(ms);
        let mut join = Join::new(Vec::new(), started, Duration::ZERO);
        assert!(!join.pending());
        join.arrive(paths(&["a"]), at(100));
        join.show();
        assert_eq!(join.take(at(100)), Some(hand_offs(&[&["a"]])));
        join.arrive(paths(&["b"]), at(150));
        assert_eq!(join.take(at(150)), Some(hand_offs(&[&["b"]])));
        assert_eq!(
            Join::platform(),
            if cfg!(windows) { JOIN } else { Duration::ZERO }
        );
    }
}
