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

//! One app instance per user data folder (decision 89, step 1; `docs/gui.md`, Instances and
//! windows): a start that finds the app running on its folder hands it what it was asked
//! to open, and exits.
//!
//! - **The lock** ([`start`]): `instance.lock` in the user data folder, by its canonical
//!   path, held with `File::try_lock` for the whole run; the system releases it as the
//!   process ends, a crash included. The start that takes it is the [`Instance`].
//! - **The channel:** on macOS and Linux a Unix socket, `apertrust-<hash>.sock`, in
//!   `$XDG_RUNTIME_DIR` (Linux) or `$TMPDIR` (macOS), else in `/tmp/apertrust-<uid>/`,
//!   created `0700`; each directory used only when the user owns it, its mode is `0700` and
//!   no ACL grants another account a right (`refused_directory`). The socket is made
//!   `0600`, a connection from another user is dropped unanswered, and its absolute path is
//!   written into the lock file once bound, where a second start reads it, never from its
//!   own environment. On Windows a named pipe, `\\.\pipe\apertrust-<hash>`, the first
//!   instance of its name, open to the user's own account alone (`apertrust-display`).
//!   `<hash>` is the first 16 hex digits of BLAKE3 over the canonical path ([`hash`]).
//! - **A stale socket** the lock file still names, or one already at the instance's own
//!   path, is removed only when it is a socket of the user's own, named for this folder,
//!   in a directory that passes the check; any other path is left alone, which
//!   [`Instance::notes`] says, for stderr.
//! - **A second start** connects, checks that the server runs as its own user before it
//!   sends anything, and sends a [`HandOff`]: the format, its arguments as given, its
//!   working folder, its activation token and its flags for one run. The answer is
//!   [`Answer::Taken`] or [`Answer::Refused`] with the reason. Another format is refused,
//!   saying another version is running. A start that cannot connect while the lock is held
//!   retries for [`RETRY`], reading the lock file again each time, then gives up.
//! - **The joins** ([`Join`]): hand-offs arriving before the window shows join its own
//!   command line into one open; on Windows, where Explorer starts the app once per file,
//!   those arriving within [`JOIN`] of the one before join too.
//!
//! The instance's thread answers each hand-off with the [`Handler`] it is given, which
//! decides the answer and posts what opens to the window ([`Post`], [`Inbox`]).

use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The lock's file in the user data folder.
pub const LOCK: &str = "instance.lock";

/// The message's format, which both ends must speak: raised whenever a message changes.
pub const FORMAT: u32 = 1;

/// How long a second start that cannot reach the instance keeps trying.
pub const RETRY: Duration = Duration::from_secs(2);

/// How long it waits between tries.
const RETRY_STEP: Duration = Duration::from_millis(50);

/// On Windows, the longest gap between two hand-offs that open together.
pub const JOIN: Duration = Duration::from_millis(500);

/// The first field of every message, so a channel that is not the instance's is told.
const MAGIC: &[u8] = b"apertrust-instance";

/// The most fields a message has, and the most bytes in all, past which it is refused.
const MOST_FIELDS: u32 = 1 << 16;
const MOST_BYTES: usize = 16 << 20;

/// The longest socket path, with its terminator: `sun_path`'s 104 bytes on macOS, 108 on
/// Linux.
#[cfg(unix)]
const SOCKET_PATH: usize = if cfg!(target_os = "macos") { 104 } else { 108 };

/// The first 16 hex digits of BLAKE3 over `canonical`, the user data folder's canonical
/// path: the channel's name for that folder.
pub fn hash(canonical: &Path) -> String {
    let digest = blake3::hash(canonical.as_os_str().as_encoded_bytes());
    digest.to_hex()[..16].to_owned()
}

/// The socket's file name for the folder whose hash is `hash`.
#[cfg(unix)]
fn socket_name(hash: &str) -> String {
    format!("apertrust-{hash}.sock")
}

/// The pipe's name for the folder whose hash is `hash`.
#[cfg(windows)]
fn pipe_name(hash: &str) -> String {
    format!(r"\\.\pipe\apertrust-{hash}")
}

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
    /// Its command line after the program's name, as given.
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

/// Where the instance's socket may go: the runtime directory, `$XDG_RUNTIME_DIR` on Linux
/// or `$TMPDIR` on macOS, and the folder its fallback is made in, `/tmp`.
#[derive(Clone, Debug, Default)]
pub struct Places {
    #[cfg(unix)]
    runtime: Option<PathBuf>,
    #[cfg(unix)]
    temporary: PathBuf,
}

impl Places {
    /// The system's.
    pub fn system() -> Places {
        #[cfg(unix)]
        {
            let variable = if cfg!(target_os = "macos") {
                "TMPDIR"
            } else {
                "XDG_RUNTIME_DIR"
            };
            let runtime = std::env::var_os(variable).filter(|dir| !dir.is_empty());
            Places {
                runtime: runtime.map(PathBuf::from),
                temporary: PathBuf::from("/tmp"),
            }
        }
        #[cfg(not(unix))]
        Places::default()
    }

    /// The runtime directory `runtime` and the fallback's folder `temporary`, for tests.
    #[cfg(all(test, unix))]
    fn of(runtime: Option<PathBuf>, temporary: PathBuf) -> Places {
        Places { runtime, temporary }
    }
}

/// What a start found on its user data folder.
pub enum Start {
    /// It took the lock: it is the instance, its channel bound, unless a note says why not.
    Instance(Instance),
    /// The lock can be neither taken nor checked, as on a file system without locks: the
    /// app runs as it would with no instance, saying why.
    Alone(String),
    /// The instance took its hand-off: it exits.
    Taken,
    /// The hand-off ended without being taken, saying why: it exits with status 1.
    Ended(String),
}

/// Takes the lock on the user data folder `user_data` and binds the channel, its hand-offs
/// answered by `handler`; or, where an instance holds it, hands it `request()` and says how
/// that went. The socket goes in one of `places`.
pub fn start(
    user_data: &Path,
    places: &Places,
    request: impl FnOnce() -> HandOff,
    handler: Handler,
) -> Start {
    start_as(user_data, places, request, handler, own_user())
}

/// The user the process runs as, whose instance alone a start hands off to.
#[cfg(unix)]
fn own_user() -> u32 {
    apertrust_display::instance::user_id()
}

#[cfg(not(unix))]
fn own_user() -> u32 {
    0
}

/// [`start`], handing off only to an instance running as `user` (on macOS and Linux).
fn start_as(
    user_data: &Path,
    places: &Places,
    request: impl FnOnce() -> HandOff,
    handler: Handler,
    user: u32,
) -> Start {
    let canonical = match std::fs::canonicalize(user_data) {
        Ok(canonical) => canonical,
        Err(e) => return Start::Alone(format!("{}: {e}", user_data.display())),
    };
    let path = canonical.join(LOCK);
    let deadline = Instant::now() + RETRY;
    let mut request = Some(request);
    let mut hand_off = None;
    let mut handler = Some(handler);
    loop {
        let opened = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path);
        let lock = match opened {
            Ok(lock) => lock,
            Err(e) => return Start::Alone(format!("{}: {e}", path.display())),
        };
        match lock.try_lock() {
            Ok(()) => {
                let handler = handler.take().expect("the handler is used once");
                return Start::Instance(Instance::begin(lock, canonical, places, handler, user));
            }
            Err(TryLockError::WouldBlock) => {}
            Err(TryLockError::Error(e)) => {
                return Start::Alone(format!("{}: not locked: {e}", path.display()));
            }
        }
        let hand_off = hand_off.get_or_insert_with(|| (request.take().expect("asked once"))());
        match attempt(&lock, &canonical, hand_off, FORMAT, user) {
            Attempt::Answered(Answer::Taken) => return Start::Taken,
            Attempt::Answered(Answer::Refused(why)) => return Start::Ended(why),
            Attempt::NotOwn => {
                return Start::Ended(format!(
                    "the channel of the instance on {} is not your own: nothing sent",
                    canonical.display()
                ));
            }
            Attempt::Unreached => {}
        }
        drop(lock);
        if Instant::now() >= deadline {
            return Start::Ended(format!(
                "Apertrust runs on the user data folder {}, but is not answering",
                canonical.display()
            ));
        }
        std::thread::sleep(RETRY_STEP);
    }
}

/// How one try to reach the instance went.
#[derive(Debug, PartialEq)]
enum Attempt {
    Answered(Answer),
    /// The server runs as another user: nothing was sent.
    NotOwn,
    /// No channel yet, or one that refused or broke off: tried again.
    Unreached,
}

/// One try to hand `hand_off` in format `format` to the instance on `canonical`, whose
/// lock file `lock` names its socket on macOS and Linux, its server checked to run as
/// `user` first.
fn attempt(lock: &File, canonical: &Path, hand_off: &HandOff, format: u32, user: u32) -> Attempt {
    let mut stream = match connect(lock, canonical, user) {
        Ok(Some(stream)) => stream,
        Ok(None) => return Attempt::NotOwn,
        Err(_) => return Attempt::Unreached,
    };
    match exchange(&mut stream, hand_off, format) {
        Ok(answer) => Attempt::Answered(answer),
        Err(_) => Attempt::Unreached,
    }
}

/// The instance's end of its channel, as a second start reaches it.
#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Stream = File;

/// Connects to the socket `lock` names, the file read again, and checks that its server
/// runs as `user`: None where it does not.
#[cfg(unix)]
fn connect(lock: &File, _canonical: &Path, user: u32) -> io::Result<Option<Stream>> {
    let socket = named_socket(lock)?.ok_or(io::ErrorKind::NotFound)?;
    let stream = Stream::connect(&socket)?;
    if apertrust_display::instance::peer_user(&stream)? != user {
        return Ok(None);
    }
    stream.set_read_timeout(Some(RETRY))?;
    stream.set_write_timeout(Some(RETRY))?;
    Ok(Some(stream))
}

/// Connects to the pipe the folder's hash names, and checks that its server runs as this
/// process's user: None where it does not.
#[cfg(windows)]
fn connect(_lock: &File, canonical: &Path, _user: u32) -> io::Result<Option<Stream>> {
    let pipe = apertrust_display::instance::connect_pipe(&pipe_name(&hash(canonical)))?;
    let (own, _) = apertrust_display::instance::pipe_server_is_own(&pipe)?;
    Ok(own.then_some(pipe))
}

/// The socket path the lock file names, read from its start: None while it names none.
#[cfg(unix)]
fn named_socket(mut lock: &File) -> io::Result<Option<PathBuf>> {
    use std::io::Seek as _;
    use std::os::unix::ffi::OsStringExt as _;
    lock.seek(io::SeekFrom::Start(0))?;
    let mut text = Vec::new();
    lock.take(4096).read_to_end(&mut text)?;
    while text.last().is_some_and(|byte| *byte == b'\n') {
        text.pop();
    }
    Ok((!text.is_empty()).then(|| PathBuf::from(OsString::from_vec(text))))
}

/// A second start's side of the exchange on `stream`: the instance's greeting, the
/// foreground right handed to its process on Windows, `hand_off` sent in format `format`,
/// then the answer.
fn exchange(stream: &mut Stream, hand_off: &HandOff, format: u32) -> io::Result<Answer> {
    let hello = read_frame(stream)?;
    let process = hello_process(&hello)?;
    // Before the hand-off, so the instance may come forward as it takes it.
    let _ = apertrust_display::instance::allow_foreground(process);
    write_frame(stream, &request_fields(hand_off, format))?;
    stream.flush()?;
    answer_of(&read_frame(stream)?)
}

/// The running instance: the lock held, the channel bound and served, until it is
/// released or dropped.
pub struct Instance {
    lock: Option<File>,
    canonical: PathBuf,
    channel: Option<Channel>,
    notes: Vec<String>,
}

/// The channel and the thread that serves it.
struct Channel {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    #[cfg(unix)]
    socket: PathBuf,
}

impl Instance {
    /// The instance on `canonical` once its `lock` is taken: the channel bound, a stale
    /// socket removed first, and served by `handler`. A channel that cannot be made leaves
    /// the instance without one, a note saying why: a second start then finds it not
    /// answering.
    fn begin(
        lock: File,
        canonical: PathBuf,
        places: &Places,
        handler: Handler,
        user: u32,
    ) -> Instance {
        let mut instance = Instance {
            lock: Some(lock),
            canonical,
            channel: None,
            notes: Vec::new(),
        };
        match instance.bind(places, handler, user) {
            Ok(channel) => instance.channel = Some(channel),
            Err(why) => instance.notes.push(format!(
                "no channel for a second start on {}: {why}",
                instance.canonical.display()
            )),
        }
        instance
    }

    #[cfg(unix)]
    fn bind(&mut self, places: &Places, handler: Handler, user: u32) -> Result<Channel, String> {
        use std::io::Seek as _;
        use std::os::unix::fs::PermissionsExt as _;
        use std::os::unix::net::UnixListener;
        let name = socket_name(&hash(&self.canonical));
        let stale = self
            .lock
            .as_ref()
            .and_then(|lock| named_socket(lock).ok().flatten());
        let (dir, notes) = socket_dir(places, user, &name)?;
        self.notes.extend(notes);
        let socket = dir.join(&name);
        for path in stale
            .iter()
            .filter(|stale| **stale != socket)
            .chain([&socket])
        {
            if let Stale::LeftAlone(why) = remove_stale(path, &name, user) {
                self.notes
                    .push(format!("{}: left alone: {why}", path.display()));
            }
        }
        let listener =
            UnixListener::bind(&socket).map_err(|e| format!("{}: {e}", socket.display()))?;
        // Whatever the umask: the directory keeps others out already.
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("{}: {e}", socket.display()))?;
        let lock = self.lock.as_mut().expect("held while binding");
        let written = lock
            .set_len(0)
            .and_then(|()| lock.seek(io::SeekFrom::Start(0)))
            .and_then(|_| {
                use std::os::unix::ffi::OsStrExt as _;
                lock.write_all(socket.as_os_str().as_bytes())
            })
            .and_then(|()| lock.write_all(b"\n"))
            .and_then(|()| lock.sync_data());
        if let Err(e) = written {
            let _ = std::fs::remove_file(&socket);
            return Err(format!("{}: not written: {e}", LOCK));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("instance".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut stream) = stream else {
                        std::thread::sleep(RETRY_STEP);
                        continue;
                    };
                    // Another user's connection is dropped unanswered.
                    match apertrust_display::instance::peer_user(&stream) {
                        Ok(peer) if peer == user => {}
                        _ => continue,
                    }
                    let _ = stream.set_read_timeout(Some(RETRY));
                    let _ = stream.set_write_timeout(Some(RETRY));
                    let _ = serve(&mut stream, &handler);
                }
            })
            .map_err(|e| format!("no thread: {e}"))?;
        Ok(Channel {
            stop,
            thread: Some(thread),
            socket,
        })
    }

    #[cfg(windows)]
    fn bind(&mut self, _places: &Places, handler: Handler, _user: u32) -> Result<Channel, String> {
        let name = pipe_name(&hash(&self.canonical));
        let mut listener = apertrust_display::instance::PipeListener::bind(&name)
            .map_err(|e| format!("{name}: {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("instance".into())
            .spawn(move || {
                loop {
                    let accepted = listener.accept();
                    if stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut pipe) = accepted else {
                        std::thread::sleep(RETRY_STEP);
                        continue;
                    };
                    // Each start on a thread of its own: a pipe has no read timeout, so a
                    // client that connects and never writes holds only its own thread, never
                    // the next start's hand-off.
                    let handler = Arc::clone(&handler);
                    let served = std::thread::Builder::new()
                        .name("instance start".into())
                        .spawn(move || {
                            if serve(&mut pipe, &handler).is_ok() {
                                // Until the second start has read the answer.
                                let _ = pipe.sync_all();
                            }
                        });
                    if let Err(e) = served {
                        tracing::warn!(target: "control", "a second start not answered: no thread: {e}");
                    }
                }
            })
            .map_err(|e| format!("no thread: {e}"))?;
        Ok(Channel {
            stop,
            thread: Some(thread),
        })
    }

    /// What the start found worth saying on stderr: a path the lock file named left alone,
    /// a runtime directory refused, no channel at all.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// The socket the instance serves, which the lock file names.
    #[cfg(all(test, unix))]
    fn socket(&self) -> Option<&Path> {
        self.channel
            .as_ref()
            .map(|channel| channel.socket.as_path())
    }

    /// Releases the channel and the lock, so the next start becomes the instance: Restart
    /// now's, before it starts it (decision 82). Dropping the instance does the same.
    pub fn release(mut self) {
        self.close();
    }

    fn close(&mut self) {
        if let Some(mut channel) = self.channel.take() {
            channel.stop.store(true, Ordering::Release);
            // A connection of its own wakes the thread waiting for the next.
            #[cfg(unix)]
            let woken = Stream::connect(&channel.socket).is_ok();
            #[cfg(windows)]
            let woken =
                apertrust_display::instance::connect_pipe(&pipe_name(&hash(&self.canonical)))
                    .is_ok();
            if let Some(thread) = channel.thread.take()
                && woken
            {
                let _ = thread.join();
            }
            #[cfg(unix)]
            let _ = std::fs::remove_file(&channel.socket);
        }
        if let Some(lock) = self.lock.take() {
            // Names no socket once released, so the next start finds nothing stale.
            #[cfg(unix)]
            let _ = lock.set_len(0);
            drop(lock);
        }
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.close();
    }
}

/// The instance's side of one exchange on `stream`: its greeting with its process id,
/// then the hand-off's answer, `handler`'s for one of its format.
fn serve(stream: &mut (impl Read + Write), handler: &Handler) -> io::Result<()> {
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
enum Request {
    HandOff(HandOff),
    /// Another format, as another build sends: refused, saying another version runs.
    OtherFormat,
}

/// The fields of `hand_off` in format `format`: the magic and the format first, which
/// every format keeps; then the folder, the tokens, the cache folder, the flags for one
/// run counted, and the arguments, each an empty field where there is none.
fn request_fields(hand_off: &HandOff, format: u32) -> Vec<Vec<u8>> {
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
fn hand_off_of(fields: &[Vec<u8>]) -> io::Result<Request> {
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
        let text = String::from_utf8(field.clone()).map_err(|_| garbled("a token not UTF-8"))?;
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

fn answer_fields(answer: &Answer) -> Vec<Vec<u8>> {
    match answer {
        Answer::Taken => vec![MAGIC.to_vec(), b"taken".to_vec()],
        Answer::Refused(why) => vec![MAGIC.to_vec(), b"refused".to_vec(), why.as_bytes().to_vec()],
    }
}

fn answer_of(fields: &[Vec<u8>]) -> io::Result<Answer> {
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

/// Writes `fields` as one message: their count, then each field's length and bytes, every
/// number four bytes little-endian.
fn write_frame(stream: &mut impl Write, fields: &[Vec<u8>]) -> io::Result<()> {
    let mut out = (fields.len() as u32).to_le_bytes().to_vec();
    for field in fields {
        out.extend((field.len() as u32).to_le_bytes());
        out.extend(field);
    }
    stream.write_all(&out)
}

/// Reads one message [`write_frame`] wrote, refusing one past [`MOST_FIELDS`] or
/// [`MOST_BYTES`].
fn read_frame(stream: &mut impl Read) -> io::Result<Vec<Vec<u8>>> {
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
fn same_folder(a: &Path, b: &Path) -> bool {
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

type Waker = Arc<Mutex<Option<Box<dyn Fn() + Send>>>>;

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
    /// Delivers `arrival`, waking the window where it has said how.
    pub fn send(&self, arrival: Arrival) {
        let _ = self.sender.send(arrival);
        if let Ok(wake) = self.wake.lock()
            && let Some(wake) = wake.as_ref()
        {
            wake();
        }
    }
}

impl Inbox {
    /// Wakes the window with `wake` as each arrival comes from now on.
    pub fn wake_with(&self, wake: impl Fn() + Send + 'static) {
        if let Ok(mut waker) = self.wake.lock() {
            *waker = Some(Box::new(wake));
        }
    }

    /// Everything that arrived since the last call, in order.
    pub fn take(&self) -> Vec<Arrival> {
        self.receiver.try_iter().collect()
    }
}

/// The hand-offs' joins into opens: each open the paths of the hand-offs it joined, in the
/// order they came, each path once.
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
    batch: Vec<PathBuf>,
    last: Instant,
    due: bool,
    /// Opens that no later hand-off can join, oldest first, each due at once.
    ready: std::collections::VecDeque<Vec<PathBuf>>,
}

impl Join {
    /// The joins of an instance whose own command line opens `own`, started at `started`.
    pub fn new(own: Vec<PathBuf>, started: Instant, interval: Duration) -> Join {
        Join {
            interval,
            shown: false,
            due: !own.is_empty(),
            batch: own,
            last: started,
            ready: std::collections::VecDeque::new(),
        }
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
        if !joins {
            let before = std::mem::take(&mut self.batch);
            if self.due {
                self.ready.push_back(before);
            }
        }
        for path in paths {
            if !self.batch.contains(&path) {
                self.batch.push(path);
            }
        }
        self.last = self.last.max(at);
        self.due = true;
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
        self.due || !self.ready.is_empty()
    }

    /// The paths of the oldest open due `now`, one an open; the next is taken once it has
    /// opened.
    pub fn take(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        if !self.shown {
            return None;
        }
        if let Some(paths) = self.ready.pop_front() {
            return Some(paths);
        }
        if !self.due || now.saturating_duration_since(self.last) < self.interval {
            return None;
        }
        self.due = false;
        Some(self.batch.clone())
    }

    /// How long until an open is due, for the window to wake then.
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        if !self.shown {
            return None;
        }
        if !self.ready.is_empty() {
            return Some(Duration::ZERO);
        }
        self.due.then(|| {
            self.interval
                .saturating_sub(now.saturating_duration_since(self.last))
        })
    }
}

/// What a directory for the socket is, as [`refused_directory`] judges it.
#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
struct Facts {
    link: bool,
    directory: bool,
    owner: u32,
    mode: u32,
    acl: bool,
}

/// Why a directory with `facts` may not hold the socket of `user`, or None where it may:
/// it is a directory of `user`'s own, not a link, its mode `0700`, and no ACL grants
/// another account a right on it.
#[cfg(unix)]
fn refused_directory(facts: Facts, user: u32) -> Option<&'static str> {
    if facts.link {
        Some("a symbolic link")
    } else if !facts.directory {
        Some("not a directory")
    } else if facts.owner != user {
        Some("another account owns it")
    } else if facts.mode & 0o777 != 0o700 {
        Some("its mode is not 0700")
    } else if facts.acl {
        Some("an access-control list grants another account a right")
    } else {
        None
    }
}

/// [`refused_directory`] for the directory at `path`, asked of the disk.
#[cfg(unix)]
fn check_directory(path: &Path, user: u32) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt as _;
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let acl = match metadata.is_dir() {
        true => apertrust_display::instance::acl_grants_others(path).map_err(|e| e.to_string())?,
        false => false,
    };
    let facts = Facts {
        link: metadata.file_type().is_symlink(),
        directory: metadata.is_dir(),
        owner: metadata.uid(),
        mode: metadata.mode(),
        acl,
    };
    refused_directory(facts, user).map_or(Ok(()), |why| Err(why.to_owned()))
}

/// The directory the socket `name` of `user` goes in: the runtime directory where it
/// passes the check and the path fits, else `apertrust-<uid>` in the temporary folder,
/// made `0700` where missing, under the same check; with a note where a runtime
/// directory was refused.
#[cfg(unix)]
fn socket_dir(places: &Places, user: u32, name: &str) -> Result<(PathBuf, Vec<String>), String> {
    use std::os::unix::fs::DirBuilderExt as _;
    let fits = |dir: &Path| dir.join(name).as_os_str().len() < SOCKET_PATH;
    let mut notes = Vec::new();
    if let Some(runtime) = places.runtime.as_ref().filter(|dir| dir.is_absolute()) {
        match check_directory(runtime, user) {
            Ok(()) if fits(runtime) => return Ok((runtime.clone(), notes)),
            Ok(()) => {}
            Err(why) => notes.push(format!(
                "{}: not used for the socket: {why}",
                runtime.display()
            )),
        }
    }
    let fallback = places.temporary.join(format!("apertrust-{user}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&fallback) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("{}: {e}", fallback.display())),
    }
    check_directory(&fallback, user).map_err(|why| format!("{}: {why}", fallback.display()))?;
    if !fits(&fallback) {
        return Err(format!(
            "{}: the socket's path is too long",
            fallback.display()
        ));
    }
    Ok((fallback, notes))
}

/// What became of a socket file a crash may have left.
#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
enum Stale {
    Removed,
    Missing,
    /// Not removed, and why: it is not certainly a socket of this folder's.
    LeftAlone(String),
}

/// Removes the socket file at `path`, which the lock, held, says no instance serves: only
/// a socket owned by `user`, named `name`, in a directory that passes the check.
#[cfg(unix)]
fn remove_stale(path: &Path, name: &str, user: u32) -> Stale {
    use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _};
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Stale::Missing,
        Err(e) => return Stale::LeftAlone(e.to_string()),
    };
    if path.file_name() != Some(OsStr::new(name)) {
        return Stale::LeftAlone("not named for this folder's socket".into());
    }
    if !metadata.file_type().is_socket() {
        return Stale::LeftAlone("not a socket".into());
    }
    if metadata.uid() != user {
        return Stale::LeftAlone("another account owns it".into());
    }
    let Some(dir) = path.parent() else {
        return Stale::LeftAlone("in no directory".into());
    };
    if let Err(why) = check_directory(dir, user) {
        return Stale::LeftAlone(format!("its directory: {why}"));
    }
    match std::fs::remove_file(path) {
        Ok(()) => Stale::Removed,
        Err(e) => Stale::LeftAlone(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead as _, BufReader};
    use std::process::{Child, ChildStdout, Command, Stdio};

    use super::*;

    /// A test's own user data folder, with on macOS and Linux a runtime directory of its
    /// own, `0700`, and the fallback's folder, all removed as it drops. Short names, so the
    /// socket's path fits macOS's limit under its long `$TMPDIR`.
    struct Fixture {
        root: PathBuf,
        user_data: PathBuf,
        #[cfg_attr(
            not(unix),
            allow(dead_code, reason = "Windows' pipe needs no directory")
        )]
        runtime: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let root = std::env::temp_dir().join(format!("ai-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            let user_data = root.join("u");
            let runtime = root.join("r");
            std::fs::create_dir_all(&user_data).unwrap();
            std::fs::create_dir_all(&runtime).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                let private = std::fs::Permissions::from_mode(0o700);
                std::fs::set_permissions(&runtime, private).unwrap();
            }
            Fixture {
                root,
                user_data,
                runtime,
            }
        }

        fn places(&self) -> Places {
            #[cfg(unix)]
            return Places::of(Some(self.runtime.clone()), self.root.clone());
            #[cfg(not(unix))]
            Places::default()
        }

        fn canonical(&self) -> PathBuf {
            std::fs::canonicalize(&self.user_data).unwrap()
        }

        fn lock(&self) -> PathBuf {
            self.user_data.join(LOCK)
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
            runtime: root.join("r"),
            root,
        };
        let held = if role == "serve" {
            let cache = fixture.user_data.join("cache");
            let handler = printing(cache, fixture.canonical());
            match start(
                &fixture.user_data,
                &fixture.places(),
                HandOff::default,
                handler,
            ) {
                Start::Instance(instance) => Some(instance),
                _ => panic!("not the instance"),
            }
        } else {
            let lock = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(fixture.lock())
                .unwrap();
            lock.try_lock().unwrap();
            std::mem::forget(lock);
            None
        };
        println!("ready");
        let _ = io::stdin().read_to_end(&mut Vec::new());
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
        start(&fixture.user_data, &fixture.places(), || hand_off, taking())
    }

    /// The first start is the instance; a second, in another process, hands it its
    /// arguments and working folder, and is taken.
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

    /// A start with `--conf` while the instance runs is refused, saying so; and one naming
    /// another cache folder; the instance's own cache folder is taken.
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
    #[test]
    fn a_second_start_with_no_answer_gives_up() {
        let fixture = Fixture::new("silent");
        let (child, _out) = spawn("hold", &fixture);
        let began = Instant::now();
        let Start::Ended(why) = second(&fixture, photo()) else {
            panic!("not ended");
        };
        assert!(why.contains("not answering"), "{why}");
        assert!(began.elapsed() >= RETRY, "{:?}", began.elapsed());
        end(child);
    }

    /// Another build's format is refused, saying another version runs.
    #[test]
    fn another_format_is_refused() {
        let fixture = Fixture::new("format");
        let (child, _out) = spawn("serve", &fixture);
        let lock = File::open(fixture.lock()).unwrap();
        let attempted = attempt(
            &lock,
            &fixture.canonical(),
            &photo(),
            FORMAT + 1,
            own_user(),
        );
        let Attempt::Answered(Answer::Refused(why)) = attempted else {
            panic!("not refused: {attempted:?}");
        };
        assert!(why.contains("another version"), "{why}");
        end(child);
    }

    /// Restart now's release leaves the folder to the next start, which becomes the
    /// instance; until then a start on it is a second start.
    #[test]
    fn a_released_instance_leaves_the_folder_to_the_next_start() {
        let fixture = Fixture::new("release");
        let Start::Instance(first) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        assert!(matches!(second(&fixture, photo()), Start::Taken));
        first.release();
        let Start::Instance(next) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        drop(next);
    }

    /// The instance's socket is in its runtime directory, which the lock file names; a
    /// second start reads it there whatever its own `$XDG_RUNTIME_DIR` or `$TMPDIR`, which
    /// is this test process's and not the instance's.
    #[cfg(unix)]
    #[test]
    fn the_lock_file_names_the_socket_for_another_environment() {
        let fixture = Fixture::new("named");
        let (child, mut out) = spawn("serve", &fixture);
        let named = std::fs::read_to_string(fixture.lock()).unwrap();
        let name = socket_name(&hash(&fixture.canonical()));
        assert_eq!(
            named.trim_end(),
            fixture.runtime.join(&name).to_str().unwrap()
        );
        assert_ne!(
            Places::system().runtime.as_deref(),
            Some(fixture.runtime.as_path())
        );
        assert!(matches!(second(&fixture, photo()), Start::Taken));
        line(&mut out, "got");
        end(child);
    }

    /// An instance killed leaves its socket, which the lock file still names: the next
    /// start removes it and binds its own there.
    #[cfg(unix)]
    #[test]
    fn a_killed_instance_leaves_a_socket_the_next_start_removes() {
        let fixture = Fixture::new("killed");
        let (mut child, _out) = spawn("serve", &fixture);
        child.kill().unwrap();
        child.wait().unwrap();
        let socket = fixture
            .runtime
            .join(socket_name(&hash(&fixture.canonical())));
        assert!(socket.exists(), "the killed instance's socket");
        let Start::Instance(instance) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        assert_eq!(instance.notes(), [] as [String; 0]);
        assert_eq!(instance.socket(), Some(socket.as_path()));
        assert!(matches!(second(&fixture, photo()), Start::Taken));
        drop(instance);
        assert!(!socket.exists(), "removed as the instance ends");
    }

    /// A crash between binding and writing the path leaves a socket at the instance's own
    /// path that the lock file does not name: the next start removes it too.
    #[cfg(unix)]
    #[test]
    fn a_socket_left_unnamed_is_removed() {
        let fixture = Fixture::new("unnamed");
        let socket = fixture
            .runtime
            .join(socket_name(&hash(&fixture.canonical())));
        drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
        assert!(socket.exists());
        let Start::Instance(instance) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        assert_eq!(instance.notes(), [] as [String; 0]);
        assert!(matches!(second(&fixture, photo()), Start::Taken));
    }

    /// A path the lock file names that is not this folder's socket, a file or another
    /// socket, is left alone, and the start says so.
    #[cfg(unix)]
    #[test]
    fn cleanup_leaves_alone_what_is_not_its_own_socket() {
        let fixture = Fixture::new("alone");
        let file = fixture.root.join("notes.txt");
        std::fs::write(&file, "keep").unwrap();
        let other = fixture.runtime.join("other.sock");
        drop(std::os::unix::net::UnixListener::bind(&other).unwrap());
        let name = socket_name(&hash(&fixture.canonical()));
        let impostor = fixture.root.join("d");
        std::fs::create_dir(&impostor).unwrap();
        std::fs::write(impostor.join(&name), "not a socket").unwrap();
        for named in [&file, &other, &impostor.join(&name)] {
            std::fs::write(fixture.lock(), named.as_os_str().as_encoded_bytes()).unwrap();
            let Start::Instance(instance) = second(&fixture, photo()) else {
                panic!("not the instance");
            };
            let notes = instance.notes().join("\n");
            assert!(notes.contains("left alone"), "{notes}");
            assert!(named.exists(), "{}", named.display());
        }
    }

    /// A start refuses to send anything to a server that runs as another user.
    #[cfg(unix)]
    #[test]
    fn a_second_start_sends_nothing_to_another_user() {
        let fixture = Fixture::new("other");
        let (child, _out) = spawn("serve", &fixture);
        let other = own_user().wrapping_add(1);
        let places = fixture.places();
        let Start::Ended(why) = start_as(&fixture.user_data, &places, photo, taking(), other)
        else {
            panic!("sent");
        };
        assert!(why.contains("not your own"), "{why}");
        end(child);
    }

    /// A directory is refused when another account owns it, its mode is open to others,
    /// an ACL grants another account a right, or it is a link; the fallback is used then.
    #[cfg(unix)]
    #[test]
    fn a_socket_directory_is_checked() {
        let fine = Facts {
            link: false,
            directory: true,
            owner: 501,
            mode: 0o40700,
            acl: false,
        };
        assert_eq!(refused_directory(fine, 501), None);
        let owner = refused_directory(fine, 502);
        assert_eq!(owner, Some("another account owns it"));
        let open = Facts {
            mode: 0o40755,
            ..fine
        };
        assert_eq!(refused_directory(open, 501), Some("its mode is not 0700"));
        let acl = Facts { acl: true, ..fine };
        let granted = refused_directory(acl, 501).unwrap();
        assert!(granted.contains("access-control list"));
        let link = Facts { link: true, ..fine };
        assert_eq!(refused_directory(link, 501), Some("a symbolic link"));
        // On the disk: the root's, another account's to a test run as a user; and one of
        // the user's own open to others, which sends the socket to the fallback.
        if own_user() != 0 {
            let why = check_directory(Path::new("/"), own_user()).unwrap_err();
            assert_eq!(why, "another account owns it");
        }
        let fixture = Fixture::new("open");
        use std::os::unix::fs::PermissionsExt as _;
        let open = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&fixture.runtime, open).unwrap();
        let Start::Instance(instance) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        let fallback = fixture.root.join(format!("apertrust-{}", own_user()));
        assert_eq!(
            instance.socket().and_then(Path::parent),
            Some(fallback.as_path())
        );
        assert!(
            instance.notes()[0].contains("its mode is not 0700"),
            "{:?}",
            instance.notes()
        );
    }

    /// The socket is `0600` whatever the umask.
    #[cfg(unix)]
    #[test]
    fn the_socket_is_the_users_alone() {
        use std::os::unix::fs::PermissionsExt as _;
        let fixture = Fixture::new("mode");
        let Start::Instance(instance) = second(&fixture, photo()) else {
            panic!("not the instance");
        };
        let socket = instance.socket().unwrap();
        let mode = std::fs::metadata(socket).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    /// Every field of a hand-off comes through, a path that is not UTF-8 among them on
    /// macOS and Linux; another format is told by its first two fields alone.
    #[test]
    fn a_hand_off_comes_through_whole() {
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
        write_frame(&mut wire, &request_fields(&hand_off, FORMAT)).unwrap();
        let fields = read_frame(&mut wire.as_slice()).unwrap();
        let Request::HandOff(read) = hand_off_of(&fields).unwrap() else {
            panic!("another format");
        };
        assert_eq!(read, hand_off);
        let other = request_fields(&hand_off, FORMAT + 1);
        assert!(matches!(hand_off_of(&other[..2]), Ok(Request::OtherFormat)));
        assert!(hand_off_of(&[b"nonsense".to_vec()]).is_err());
        for answer in [Answer::Taken, Answer::Refused("why".into())] {
            assert_eq!(answer_of(&answer_fields(&answer)).unwrap(), answer);
        }
        let mut huge = (MOST_FIELDS + 1).to_le_bytes().to_vec();
        huge.extend([0; 8]);
        assert!(read_frame(&mut huge.as_slice()).is_err());
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
    #[test]
    fn the_hash_is_of_the_canonical_path() {
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

    /// While the window has not shown, every hand-off joins the instance's own command
    /// line, however long it takes; then on Windows those within 500 ms of the one before
    /// join, and open once none has come for 500 ms.
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
        assert_eq!(join.take(at(5500)), Some(paths(&["a", "b", "c"])));
        assert!(!join.pending());
        // Running: within 500 ms of the one before, one open.
        join.arrive(paths(&["d"]), at(7000));
        join.arrive(paths(&["e"]), at(7400));
        join.arrive(paths(&["f"]), at(7800));
        assert_eq!(join.take(at(8100)), None);
        assert_eq!(join.take(at(8300)), Some(paths(&["d", "e", "f"])));
        // Later than that, an open of its own; none for a start with no paths.
        join.arrive(paths(&["g"]), at(9000));
        join.arrive(Vec::new(), at(9100));
        assert_eq!(join.take(at(9500)), Some(paths(&["g"])));
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
        assert_eq!(join.take(at(120)), Some(paths(&["a"])));
        assert_eq!(join.take(at(120)), Some(paths(&["b"])));
        assert!(join.pending());
        assert_eq!(join.take(at(120)), Some(paths(&["c"])));
        assert!(!join.pending());
        assert_eq!(join.take(at(120)), None);
        let mut windows = Join::new(Vec::new(), started, JOIN);
        windows.show();
        windows.arrive(paths(&["a"]), at(1000));
        windows.arrive(paths(&["b"]), at(1200));
        windows.arrive(paths(&["c"]), at(1800));
        assert_eq!(windows.take(at(1900)), Some(paths(&["a", "b"])));
        assert_eq!(windows.take(at(1900)), None, "within 500 ms of c");
        assert_eq!(windows.take(at(2300)), Some(paths(&["c"])));
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
        assert_eq!(join.take(at(100)), Some(paths(&["a"])));
        join.arrive(paths(&["b"]), at(150));
        assert_eq!(join.take(at(150)), Some(paths(&["b"])));
        assert_eq!(
            Join::platform(),
            if cfg!(windows) { JOIN } else { Duration::ZERO }
        );
    }
}
