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

//! An editing session on the engine thread (decision 17): one photo, its edit and the
//! viewport pipe (decision 12). A client sends [`Command`]s and reads [`Event`]s; it
//! never waits for the GPU, a decode or a file.
//!
//! Clients speak modules (decision 28): they set, reset and keep module controls, and the
//! coordinates of lossless views as controls, and apply the options of preset rows
//! (decision 2), and the session turns the edit into operation parameters through the
//! modules' bindings ([`crate::edit::instances`]). Only parameters no module owns yet, of
//! operations an unedited photo runs, are set at operation level. A module whose binding
//! misbehaves is reported and left out of the render; the rest of the edit still renders.
//!
//! The thread takes every command that has arrived, applies them in order and renders
//! only the latest state, so a slider dragged faster than frames render costs one render
//! per frame. Each frame is copied into whichever of two display buffers was not
//! published last, then published: a client draws the latest frame it was sent, and the
//! queue the two threads share orders its drawing after the copy it names.
//!
//! The edit saves itself, beside the photo: after each completed change (a control's
//! `done`, a reset, a keep, a preset's option), and, for a change never completed, when
//! another photo opens and when the session ends, the commands still queued when the
//! client drops its handle included. A command that changes nothing writes nothing. A
//! writer thread of the session's own writes the edit, so a slow disk never delays a
//! frame, and reports each write as [`Event::Saved`] or a failure; a failed write leaves
//! the edit in memory, and the next completed change, or closing the photo, writes it
//! again. Each edit records the build that wrote it, which the client hands the session.
//!
//! A gesture, the run of `done: false` changes on one module, can be cancelled, as a
//! slider popup's is ([`Command::CancelGesture`], decision 42): the session keeps the
//! edit and the held coordinates as they stood before its first change, puts both back,
//! leaving no history step, and writes nothing while the file holds that edit.
//!
//! A photo whose edit a newer Apertrust wrote opens for viewing only (decision 43,
//! [`sidecar::ViewOnly`]): it renders what this build reads, [`Photo::view_only`] says
//! which build wrote the edit, no control is reported available, every command that would
//! change the edit is refused, and its file is never written.
//!
//! Once a client asks for the scopes ([`Command::Scopes`], decision 19), each frame is
//! counted on the GPU ([`crate::scopes`]) right after its copy into the display buffer,
//! and its counts are read back through two staging buffers mapped without waiting: the
//! thread looks at the GPU between commands while a reading is on its way, and sends it
//! as [`Event::Scopes`] when it arrives. The totals, what clips, with every count; the
//! histogram and the waveform when asked. A scopes failure never costs a frame: it is
//! reported once, and the session goes on without them.
//!
//! The session runs background jobs on darktable's control (decision 57,
//! [`apertrust_base::jobs`]), one control per session: a command starts a job, whose
//! body runs on a worker and hands its GPU work to the engine thread ([`crate::jobs`]).
//! After the commands and the live frame, the thread steps one waiting task a turn, so a
//! change waits for one step at most. Jobs with a progress are reported as [`Event::Job`],
//! and [`Command::CancelJob`] cancels one that can be cancelled; one that failed and
//! offers it is tried again by [`Command::RetryJob`], which runs what the job left the
//! session (decision 72). Dropping the handle shuts the control down: cancellable jobs
//! stop, the rest finish, their GPU tasks still stepped until they do.
//!
//! Every thread of the session tells the client through one outbox (decision 59),
//! which wakes it once for each burst of events. Each event is a fact, the whole current
//! state of its topic, superseding the last of that topic, or a notice, something that
//! happened once ([`Event::topic`]); an event that answers commands names them by the
//! numbers [`Session::send`] gives them, a refused command as
//! [`SessionError::Refused`].
//!
//! These types stand in for the session protocol's until `apertrust-protocol` exists.

mod outbox;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use apertrust_base::jobs::Control;
pub use apertrust_export::{ExportSettings, Places};
/// The library's commands and events and what they carry (decision 54, layer 4).
pub use apertrust_library::ops as library;
pub use apertrust_library::ops::{LibraryCommand, LibraryEvent};
use apertrust_modules::check::{self, ModuleError};
use apertrust_modules::{Base, ControlKind, Controls, Module};
pub use apertrust_modules::{ControlValue, Mark, Range, Setting};
pub use apertrust_plugin::Value;
use apertrust_plugin::{ImageContext, ImageDesc, Operation, ParamValues};
use thiserror::Error;

use crate::compute::{ComputeError, GpuImage, POLL_INTERVAL, Timings};
use crate::edit::{self, Edit, EditError, Held, panic_message};
use crate::export_job::{self, Live, Snapshot};
pub use crate::export_job::{Cause, ExportStatus, Failure, PhotoResult, Summary, Why};
use crate::export_preview;
pub use crate::export_preview::ExportPreview;
use crate::gpu::{GpuContext, SubmitGate};
use crate::jobs::{self, Background, Queue, Tasks, Waiting};
pub use crate::jobs::{JobEvent, JobId};
use crate::names::Shot;
use crate::pipeline::{self, Limits, Pipe, Pipeline, PlanError};
use crate::scopes::Meter;
pub use crate::scopes::{Graphs, Reading};
pub use crate::sidecar::ViewOnly;
use crate::{raw, sidecar};
use outbox::Drain;
pub(crate) use outbox::Outbox;

/// What a client asks of the session. Plain data, so it can move into the protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Open a raw, with the edit saved beside it or none.
    Open(PathBuf),
    /// The most physical pixels the photo may be shown in.
    Resize { width: u32, height: u32 },
    /// Set a module's control, which keeps it. `done` on the last change of a gesture (a
    /// drag released, a click, a typed value), which saves the edit.
    ///
    /// `control` may name a coordinate of one of the module's lossless views (decision
    /// 2): the view's controls then take the values that give it `value` while the view's
    /// other coordinates hold, each kept, as one change. Its other coordinates are held
    /// from the gesture's start for as long as the controls hold what the session wrote,
    /// so a drag leaves them exactly where they were. A value outside the coordinate's
    /// range, or one that would put a control outside its own, is refused.
    SetControl {
        module: String,
        control: String,
        value: ControlValue,
        done: bool,
    },
    /// A control back to its default for the photo, no longer kept. A view's coordinate
    /// goes back to its value at the module's defaults, holding the others; once all are
    /// there, the view's controls are reset exactly.
    ResetControl { module: String, control: String },
    /// Every control of a module back to its default: the header's reset.
    ResetModule { module: String },
    /// Cancel the gesture under way on a module, as a slider popup's cancel does
    /// (decision 42). A gesture is a run of `SetControl { done: false }` on one module;
    /// any other command on the edit, refused or not, ends it as completed, while a
    /// resize, the scopes and a cancel naming another module leave it under way. The
    /// module goes back exactly as it was before the gesture's first change: every
    /// control's value and mark, follow writes and clamps included, and the coordinates
    /// its views held. It is no history step, and nothing is written while the file holds
    /// that edit, though the writer reports it [`Event::Saved`] with the cancel's number.
    /// Naming a module no gesture is under way on, it changes nothing, and is reported
    /// saved alike while the file holds the edit.
    CancelGesture { module: String },
    /// Keep a control, or release it, leaving its value; a view's coordinate keeps or
    /// releases the view's controls.
    Keep {
        module: String,
        control: String,
        kept: bool,
    },
    /// Apply an option of a module's preset row (decision 2), as White balance's Mode:
    /// the controls it writes take its values for the photo as one change, each kept, and
    /// the module's other controls stay as they are. An option the photo cannot have
    /// ([`PresetState::available`]), or one the row lacks, is refused. It completes a
    /// change, which saves the edit.
    ApplyPreset {
        module: String,
        preset: String,
        option: String,
    },
    /// Set a parameter no module owns yet, of an operation an unedited photo runs; any
    /// other is refused. It leaves the protocol before the first release, and is saved
    /// with the next completed change.
    Set {
        operation: String,
        param: String,
        value: Value,
    },
    /// The whole edit back to nothing.
    Reset,
    /// Write the open photo's edit again after its last write failed, as a client's Try
    /// again does; otherwise nothing. It changes no value and leaves a gesture under way,
    /// and is reported [`Event::Saved`] or failed as any write is.
    RetrySave,
    /// Count each frame for the scopes, the totals and these graphs, or, with `None`, as
    /// until a client first asks, count nothing. Asking for what the frame shown was not
    /// counted for counts that frame again, without rendering.
    Scopes(Option<Graphs>),
    /// Cancel a job, as its progress's cancel does (decision 57): a queued job never
    /// runs, and a running one stops at its body's next check, its GPU task dropped
    /// before its next step. Refused for a job that cannot be cancelled
    /// ([`SessionError::NotCancellable`]); one that has ended changes nothing.
    CancelJob { job: JobId },
    /// Try a failed job again, as its row's Try again does (decision 72): runs the command
    /// the job left the session as it offered [`jobs::Offer::Retry`], as if it were sent
    /// now, so an export takes each photo's edit as it is at this moment (D12). Its job's
    /// `Added` names this command and `retries: Some(job)`. A retry is used once; one not
    /// kept, used already or older than the latest [`RETRIES_KEPT`], is refused
    /// ([`SessionError::NotRetryable`]).
    RetryJob { job: JobId },
    /// Export `photos` with `settings` (decision 56): a job, darktable's "export images",
    /// one at a time, a later one queued behind it, its `Added` naming this command. Each
    /// photo's edit is taken as this command is handled (D12): the open photo's as the
    /// session holds it, every other's from its sidecar once the edits handed to the
    /// writer before are written, so a change sent after Export is absent from its file.
    /// Its progress comes as [`Event::Job`], each photo's start and result and the job's
    /// counts as [`Event::Export`]; [`Command::CancelJob`] cancels it.
    Export {
        photos: Vec<PathBuf>,
        settings: ExportSettings,
        /// The person's folders and name, which the variables give; the client's, since the
        /// engine reads no platform folder (decision 43).
        places: Places,
    },
    /// What the Export dialog shows under its rows for these `photos` and `settings`,
    /// answered as [`Event::ExportPreview`] (decision 56): a `UserForeground` job with
    /// no progress, on a job worker, that writes and makes nothing. The preview before it
    /// is cancelled and sends nothing, so the answer to the latest command is the one a
    /// client keeps. `places` are the person's folders and name, as for
    /// [`Command::Export`].
    PreviewExport {
        photos: Vec<PathBuf>,
        settings: ExportSettings,
        places: Places,
    },
    /// A library command (decision 54): opening a folder, closing it, or asking for its
    /// photos' thumbnails. An open is a cancellable job on darktable's `UserBackground`
    /// queue, its progress as [`Event::Job`], its `Added` naming this command, and what it
    /// finds as [`Event::Library`], each naming this command's number; it cancels the
    /// reading before it. A close cancels it and answers [`LibraryEvent::Closed`]. Opening a
    /// folder opens no photo: [`Command::Open`] does.
    ///
    /// Each thumbnail asked for is a job on darktable's `SystemForeground` queue, as
    /// darktable's thumbnail loads: a stack, the newest asked first, deduplicated by photo
    /// and size, so one asked for again while queued moves to the top and one being read is
    /// not read twice; past 30 the oldest is dropped, and a client asks again for a photo
    /// still on screen. It has no progress, and its picture comes as
    /// [`LibraryEvent::Thumbnail`], naming the open of its folder. A new open or a close
    /// cancels those not yet sent; with no folder open nothing is read.
    Library(LibraryCommand),
    /// A test's synthetic GPU job, started as a command starts a job.
    #[cfg(test)]
    Synthetic(crate::jobs::testing::Synthetic),
    /// Holds the first render step of each photo of the next export for a test.
    #[cfg(test)]
    HoldExport(export_job::testing::Hold),
    /// Holds the next export preview before it looks, for a test.
    #[cfg(test)]
    HoldPreview(export_job::testing::Hold),
    /// Holds the next folder's reading before it begins, for a test.
    #[cfg(test)]
    HoldFolder(export_job::testing::Hold),
    /// Holds each thumbnail the next [`LibraryCommand::Thumbnails`] asks for before it is
    /// read, for a test.
    #[cfg(test)]
    HoldThumbnails(export_job::testing::Hold),
}

/// Whether `command` acts on the open photo's edit, as every command does but these: the
/// values are reported after every command on the edit, and such a command ends a
/// gesture under way and is refused on a photo open for viewing only.
fn on_edit(command: &Command) -> bool {
    match command {
        Command::Open(_)
        | Command::Resize { .. }
        | Command::Scopes(_)
        | Command::CancelJob { .. }
        | Command::RetryJob { .. }
        | Command::Export { .. }
        | Command::PreviewExport { .. }
        | Command::Library(_) => false,
        #[cfg(test)]
        Command::Synthetic(_)
        | Command::HoldExport(_)
        | Command::HoldPreview(_)
        | Command::HoldFolder(_)
        | Command::HoldThumbnails(_) => false,
        _ => true,
    }
}

/// What reaches the engine thread, in the order sent: the client's commands, the GPU
/// tasks its jobs' bodies hand it ([`crate::jobs::Tasks`]), and, last, the client's
/// handle dropping.
pub(crate) enum Inbox {
    Command(Instant, Box<Command>),
    Task(Waiting),
    Stop,
}

/// The extensions of the files [`Command::Open`] can decode, upper case and without the
/// dot: what a client's open dialog offers.
pub fn raw_extensions() -> &'static [&'static str] {
    apertrust_raw::extensions()
}

/// What the session tells its client.
#[derive(Debug)]
pub enum Event {
    Opened(Photo),
    /// Every module's controls and the unowned parameters, after an open or a change.
    Values(Values),
    /// A new frame to show.
    Frame(Frame),
    /// What a frame holds, counted for the scopes, about a frame after it; `through`
    /// names the frame by the commands it includes.
    Scopes(Reading),
    /// The edit was written to `path`, as it stood after the first `through` commands
    /// ([`Session::send`] numbers them).
    Saved {
        path: PathBuf,
        through: u64,
    },
    /// What a job with a progress did (decision 57), everything its row shows (decision
    /// 72), in order for each job: added, with its queue, title, whether it can be
    /// cancelled and the rest of its row as it starts; then its states, progress, title,
    /// detail and destination, each a fact by job; then its end, with its outcome,
    /// failure and offers. `command`, on `Added`, is the number [`Session::send`] gave the
    /// command that started the job, so a client knows its own; `None` on the rest.
    Job {
        event: JobEvent,
        command: Option<u64>,
    },
    /// What an export job (decision 56) did with its photos, in order: each photo's start
    /// and result, then the job's counts. `job` is the job its `Event::Job` names.
    Export {
        job: JobId,
        status: ExportStatus,
    },
    /// The Export dialog's feedback for [`Command::PreviewExport`], the command numbered
    /// `through`. A client keeps the answer with the highest `through`: a preview
    /// cancelled in time sends nothing, but one that had finished sends its answer.
    ExportPreview {
        through: u64,
        preview: Box<ExportPreview>,
    },
    /// What a [`Command::Library`] found or did, each naming the command it answers: a
    /// client keeps the number of its latest open or close and drops the rest.
    Library(LibraryEvent),
    Failed(SessionError),
}

/// What an event is about (decision 59, `docs/engine.md`, Events): a fact, the whole
/// current state of its topic, which supersedes the last fact of the same topic, so the
/// latest is enough; or a notice, something that happened once, which a client takes in
/// every time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Topic {
    Fact(Fact),
    Notice,
}

/// The topic a fact states the whole of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fact {
    /// Every module's controls: [`Event::Values`].
    Values,
    /// The frame to show: [`Event::Frame`].
    Frame,
    /// What the frame shown holds: [`Event::Scopes`].
    Scopes,
    /// The Export dialog's feedback: [`Event::ExportPreview`], the latest answer kept.
    ExportPreview,
    /// A job's progress, by job.
    Progress(JobId),
    /// A job's title, darktable's message, by job.
    Title(JobId),
    /// The line under a job's bar, by job.
    Detail(JobId),
    /// Where a job's results go, by job.
    Destination(JobId),
    /// A job's state while it lives, by job.
    State(JobId),
}

impl Event {
    /// Whether this is a fact, and of which topic, or a notice.
    pub fn topic(&self) -> Topic {
        let fact = match self {
            Event::Values(_) => Fact::Values,
            Event::Frame(_) => Fact::Frame,
            Event::Scopes(_) => Fact::Scopes,
            Event::ExportPreview { .. } => Fact::ExportPreview,
            // Each of a job's values is a topic of its own, since each event carries only
            // its own (decision 72); its adding and its end each happen once.
            Event::Job { event, .. } => match *event {
                JobEvent::Progress { job, .. } => Fact::Progress(job),
                JobEvent::Title { job, .. } => Fact::Title(job),
                JobEvent::Detail { job, .. } => Fact::Detail(job),
                JobEvent::Destination { job, .. } => Fact::Destination(job),
                JobEvent::State { job, .. } => Fact::State(job),
                JobEvent::Added { .. } | JobEvent::Ended { .. } => return Topic::Notice,
            },
            // The library's events are about one open, and each comes once: a folder's
            // opening and closing, its subfolders as they were read, a chunk of its
            // photos, which is part of them, its failure, its photos' capture times, and a
            // photo's thumbnail, which answers one ask.
            Event::Opened(_)
            | Event::Saved { .. }
            | Event::Export { .. }
            | Event::Library(_)
            | Event::Failed(_) => {
                return Topic::Notice;
            }
        };
        Topic::Fact(fact)
    }
}

/// The photo the session opened.
#[derive(Debug, Clone, PartialEq)]
pub struct Photo {
    pub path: PathBuf,
    /// The number [`Session::send`] gave the [`Command::Open`] that opened it, so a client
    /// tells it from an earlier open's.
    pub open: u64,
    /// The make and model, as the decoder names them.
    pub camera: String,
    /// The developed image at full resolution.
    pub width: u32,
    pub height: u32,
    pub decode_ms: f64,
    pub upload_ms: f64,
    /// Set when a newer Apertrust wrote the photo's edit: the photo is open for viewing
    /// only, and this says which build wrote it.
    pub view_only: Option<ViewOnly>,
    /// The shooting data the raw's EXIF records, for the caption under the photo: the
    /// exposure time in seconds, the f-number and the focal length in mm; `None` where it
    /// records none or could not be read. The ISO is in `image`'s camera.
    pub exposure_time: Option<f32>,
    pub aperture: Option<f32>,
    pub focal_length: Option<f32>,
    /// The photo's image context at full resolution, the one every binding sees, so a
    /// client shows a module's view and availability of its pending values as the session
    /// will (decision 27): White balance's temperature and tint read the camera's matrix.
    /// Boxed, as the largest part of the photo the session opened.
    pub image: Box<ImageContext>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Values {
    /// Every module's controls, in the modules' order.
    pub modules: Vec<ModuleState>,
    /// Every parameter no module owns, of the operations an unedited photo runs: the
    /// edit's values over the photo's defaults.
    pub settings: Vec<Setting>,
    /// Whether anything differs from its default for the photo.
    pub edited: bool,
    /// How many commands had been applied: every one [`Session::send`] numbered up to
    /// this, so a client knows which of its changes the values include.
    pub through: u64,
    /// The photo as the edit shows it, at full resolution: the developed image, turned and
    /// mirrored where its orientation says (decision 16), which the viewport fits.
    pub extent: [u32; 2],
}

impl Values {
    /// The size, in pixels, of the frame the viewport renders the photo at for `bounds`,
    /// as `Command::Resize` gives them: the largest with the aspect of the photo as the
    /// edit shows it that fits, never larger than the photo. A client shows a frame
    /// rendered for other bounds at this size until the next arrives.
    pub fn viewport_size(&self, bounds: (u32, u32)) -> (u32, u32) {
        crate::pipeline::fit((self.extent[0], self.extent[1]), bounds)
    }
}

/// One module's controls.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleState {
    pub module: &'static str,
    /// Whether any control differs from its default for the photo.
    pub edited: bool,
    /// Every control, in the definition's order.
    pub controls: Vec<ControlState>,
    /// Every preset row, in the definition's order.
    pub presets: Vec<PresetState>,
    /// Every coordinate of every lossless view, in the definition's order.
    pub views: Vec<CoordinateState>,
}

/// One coordinate of a lossless view (decision 2), which a client sets, resets and keeps
/// as a control, and shows as the session reports it, computing nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateState {
    pub view: &'static str,
    pub coordinate: &'static str,
    /// The coordinate held through a gesture, else the module's view of the controls'
    /// values; it may lie outside its range, where the controls put it.
    pub value: f32,
    /// Its value at the module's defaults for the photo, which a reset goes back to.
    pub default: f32,
    /// The mark its view's controls share, `None` where they differ: marks belong to the
    /// stored controls, which views and presets set together.
    pub mark: Option<Mark>,
    /// Whether it can change the photo: some control it moves can, and the module says the
    /// coordinate itself can, as White balance's temperature and tint cannot without the
    /// camera's matrix (decision 30). Never on a photo open for viewing only.
    pub available: bool,
}

/// A preset row's state (decision 2): which of its options the controls hold, and
/// which the photo can have.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetState {
    pub preset: &'static str,
    /// The option whose values the controls hold (`Module::matching`); `None` for Custom.
    pub matching: Option<&'static str>,
    /// The options this photo can have, in the row's order, none while it is open for
    /// viewing only; a panel draws the others disabled, and [`Command::ApplyPreset`]
    /// refuses them.
    pub available: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlState {
    pub control: &'static str,
    pub value: ControlValue,
    /// Its untouched value for the photo, which a reset goes back to.
    pub default: ControlValue,
    pub mark: Option<Mark>,
    /// Whether it can change anything on this photo while the module's controls hold the
    /// values reported with it (`Module::available`, asked again after every change, so a
    /// control another's value disables follows it; decision 30), never for a control
    /// not ported (`Control::unported`) nor on a photo open for viewing only
    /// ([`Photo::view_only`]), nor for a choice whose photo lists only the option it holds
    /// (decision 30); a panel draws one that cannot as disabled, its value kept.
    pub available: bool,
    /// For a choice, the options this photo lists, by id, in the definition's order
    /// (`Module::options`, decision 30), computed once as the photo opened: every option
    /// where the module's code panics or gives a list that does not hold together. The
    /// session refuses any other; `value` may still hold one, from another photo's edit.
    /// Empty for a control that is no choice.
    pub options: Vec<&'static str>,
    /// For a number, its range while the module's controls hold the values reported with
    /// it (`Module::range`, decision 30): what a value may hold, what the track spans at
    /// rest and what a key moves, asked again after every change, so a range another
    /// control moves follows it; the definition's where the module's code panics or gives
    /// one that does not hold, warned of once for the photo. The session refuses a value
    /// outside it, and clamps one a change leaves outside. `None` for a control that is no
    /// number.
    pub range: Option<Range>,
}

/// A rendered frame: RGBA `f32` in the output profile's encoding, rows top to bottom,
/// in a storage buffer the session never writes again while it is the latest frame.
#[derive(Debug, Clone)]
pub struct Frame {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
    /// How many commands it includes, as [`Values::through`] counts them; its scopes'
    /// reading names it so.
    pub through: u64,
    pub timings: Timings,
    /// From the first command it answers reaching the session to its publication.
    pub latency_ms: f64,
}

#[derive(Debug, Error)]
pub enum SessionError {
    /// The [`Command::Open`] that [`Session::send`] numbered `open` failed, as `error`
    /// says: the photo before it is closed, its edit saved, and none is open.
    #[error("{error}")]
    Open { open: u64, error: Box<SessionError> },
    /// The command that [`Session::send`] numbered `command` was refused, as `error` says,
    /// and changed nothing: an edit refused, a change on a photo open for viewing only, a
    /// job that cannot be cancelled.
    #[error("{error}")]
    Refused {
        command: u64,
        error: Box<SessionError>,
    },
    #[error(transparent)]
    Decode(#[from] apertrust_raw::Error),
    #[error("only 16-bit mosaics can be developed so far")]
    NotAMosaic,
    #[error("reading the edit: {0}")]
    Sidecar(#[from] sidecar::SidecarError),
    /// A change refused: out of range, of another kind, unknown, or a parameter a module
    /// owns.
    #[error(transparent)]
    Edit(#[from] EditError),
    /// A change refused because the photo is open for viewing only: a newer Apertrust
    /// wrote its edit ([`Photo::view_only`]).
    #[error(transparent)]
    ViewOnly(#[from] ViewOnly),
    /// A module that does not hold, left out of the session; or one whose binding failed,
    /// left out of the render until it binds again.
    #[error("the module {module} is left out: {source}")]
    Module {
        module: &'static str,
        source: ModuleError,
    },
    /// The edit as it stood after the first `through` commands could not be written to
    /// `path`; it stays in memory, and the next completed change, or closing the photo,
    /// writes it again.
    #[error("saving the edit to {}: {source}", path.display())]
    Save {
        path: PathBuf,
        through: u64,
        source: io::Error,
    },
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Compute(#[from] ComputeError),
    /// The GPU ran out of memory or refused a command; the frame is not shown.
    #[error("the GPU: {0}")]
    Gpu(String),
    /// The scopes could not be counted or read back; the frames go on without them for
    /// the rest of the session.
    #[error("the scopes are off: {0}")]
    Scopes(String),
    /// [`Command::CancelJob`] named a job that cannot be cancelled: it goes on.
    #[error("job {job} cannot be cancelled")]
    NotCancellable { job: JobId },
    /// [`Command::RetryJob`] named a job with no retry kept: one that offered none, whose
    /// retry was used already, or one older than the latest [`RETRIES_KEPT`].
    #[error("job {job} cannot be tried again")]
    NotRetryable { job: JobId },
    /// A bug in the engine: what it was doing is dropped, the photo with it.
    #[error("the engine failed: {0}")]
    Panicked(String),
    /// The engine thread has ended, so nothing more will render.
    #[error("the engine stopped")]
    Stopped,
}

/// Called by the [`Outbox`] when events have arrived that the client may not have seen,
/// from the engine thread, the writer's or a job's.
pub(crate) type Wake = Arc<dyn Fn() + Send + Sync>;

/// A photo decoded and uploaded: what [`Engine::open`] and an export's photo start from.
pub(crate) struct Decoded {
    pub(crate) camera: String,
    pub(crate) image: ImageContext,
    pub(crate) input: GpuImage,
    /// What an export's variables read of the raw beside its EXIF.
    pub(crate) shot: Shot,
    pub(crate) decode_ms: f64,
    pub(crate) upload_ms: f64,
}

/// How the engine gets a photo from its path: [`decode`], or a synthetic photo in tests.
pub(crate) type Load = fn(&GpuContext, &Path) -> Result<Decoded, SessionError>;

/// The options each choice of `modules` lists on `image`, by module and control
/// (decision 30): [`edit::listed`], every option where a module's code panics or gives a
/// list that does not hold together, warned of once for the photo.
fn listed_options(
    modules: &[&'static dyn Module],
    image: &ImageContext,
) -> BTreeMap<(&'static str, &'static str), Vec<&'static str>> {
    let mut lists = BTreeMap::new();
    for &module in modules {
        let id = module.definition().id;
        let choices = module.definition().controls.iter();
        for control in choices.filter(|c| matches!(c.kind, ControlKind::Choice { .. })) {
            let (listed, failed) = edit::listed(module, control, image);
            if let Some(message) = failed {
                let control = control.id;
                tracing::warn!(target: "pipe", "module {id}'s {control} lists every option: {message}");
            }
            lists.insert((id, control.id), listed);
        }
    }
    lists
}

/// Decodes the raw at `path` and uploads its mosaic.
fn decode(gpu: &GpuContext, path: &Path) -> Result<Decoded, SessionError> {
    let start = Instant::now();
    let decoded = apertrust_raw::decode_file(path)?;
    let decode_ms = start.elapsed().as_secs_f64() * 1e3;
    let start = Instant::now();
    let input = raw::upload(gpu, &decoded).ok_or(SessionError::NotAMosaic)??;
    let upload_ms = start.elapsed().as_secs_f64() * 1e3;
    Ok(Decoded {
        // The name the camera has, an alias's own.
        camera: format!("{} {}", decoded.camera.make, decoded.camera.alias),
        image: raw::image_context(&decoded),
        input,
        shot: shot_of(&decoded, path),
        decode_ms,
        upload_ms,
    })
}

/// What the variables and the photo's caption read of the raw at `path`: its decoded image
/// and its EXIF, the metadata alone, which, if it cannot be read, leaves the shooting data
/// `None` and the photo open.
fn shot_of(decoded: &apertrust_raw::RawImage, path: &Path) -> Shot {
    let shot = Shot::of(decoded);
    match apertrust_raw::read_exif_file(path) {
        Ok(exif) => shot.with_exif(&exif),
        Err(error) => {
            tracing::warn!(target: "raw", "{}: no shooting data: {error}", path.display());
            shot
        }
    }
}

/// A client's handle on the session; dropping it ends the thread, once the edit is saved.
pub struct Session {
    commands: Option<mpsc::Sender<Inbox>>,
    /// How many commands have been sent.
    sent: AtomicU64,
    events: Drain,
    /// Set when the handle drops, so the thread renders nothing more.
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// The engine's submit gate, released while the thread is joined.
    gate: SubmitGate,
}

impl Session {
    /// Starts the engine thread on `gpu` with `operations`, the pipeline's operations,
    /// and `modules`, what clients edit with. A module that does not hold against the
    /// operations or the modules before it ([`check::accept`]) is reported
    /// [`SessionError::Module`] and left out. Every edit the session writes records
    /// `build` as the build that wrote it: the client's version (decision 44). Every photo
    /// is planned within `budget` bytes of GPU memory, the engine's memory budget
    /// (decision 12), which the client reads from its settings (`engine.memory_budget`);
    /// the session keeps it while it runs, so a change applies at the next start. `wake`
    /// is called, from the session's threads, when events have arrived and the client has
    /// not looked since the last call, once for a burst: the client looks at them with
    /// [`Session::events`], a window asking for a repaint, and the next event after that
    /// look wakes it again.
    pub fn start(
        gpu: GpuContext,
        operations: &'static [&'static dyn Operation],
        modules: &'static [&'static dyn Module],
        build: &str,
        budget: u64,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Session {
        Self::spawn(
            gpu,
            operations,
            modules,
            build,
            budget,
            Arc::new(wake),
            decode,
        )
    }

    pub(crate) fn spawn(
        gpu: GpuContext,
        operations: &'static [&'static dyn Operation],
        modules: &'static [&'static dyn Module],
        build: &str,
        budget: u64,
        wake: Wake,
        load: Load,
    ) -> Session {
        let build = build.to_owned();
        let (commands, received) = mpsc::channel();
        // The jobs' bodies hand their GPU tasks through the same inbox, so a task wakes
        // the engine thread as a command does.
        let tasks = commands.clone();
        let (outbox, events) = outbox::channel(wake);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let gate = gpu.gate.clone();
        let thread = std::thread::Builder::new()
            .name("engine".to_owned())
            .spawn(move || {
                let (mut engine, rejected) = Engine::new(
                    gpu,
                    operations,
                    modules,
                    build,
                    budget,
                    Writer::start(outbox.clone()),
                    Jobs::start(outbox.clone(), tasks),
                );
                // Dropped before the engine, after a panic too: the tasks on their way
                // go with it, so their bodies hear the engine stopped, and the control's
                // drop, which waits for its workers, never waits for them.
                let received = received;
                engine.load = load;
                outbox.send_all(rejected);
                // Once the handle has dropped, an Open still queued, and what follows it,
                // was for a photo that never opens.
                let mut past_open = false;
                loop {
                    // Blocks for the next command, then takes every other that has
                    // arrived; while a scopes reading or a GPU task's step is on its way,
                    // or a task waits for one, looks at the GPU between them, as often as
                    // `GpuContext::wait` does.
                    let polling = engine.reading() || engine.jobs.background.waiting();
                    let first = if polling && !stopping.load(Ordering::Relaxed) {
                        match received.recv_timeout(POLL_INTERVAL) {
                            Ok(first) => Some(first),
                            Err(mpsc::RecvTimeoutError::Timeout) => None,
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match received.recv() {
                            Ok(first) => Some(first),
                            Err(mpsc::RecvError) => break,
                        }
                    };
                    let mut batch = Vec::new();
                    let mut stop = false;
                    for item in first.into_iter().chain(received.try_iter()) {
                        match item {
                            Inbox::Command(at, command) => batch.push((at, *command)),
                            Inbox::Task(waiting) => engine.jobs.background.add(waiting),
                            // The handle's last word: the tasks after it are the end's.
                            Inbox::Stop => {
                                stop = true;
                                break;
                            }
                        }
                    }
                    if stop || stopping.load(Ordering::Relaxed) {
                        // The client has gone, but what it changed still counts: its edit
                        // commands apply, with nothing rendered, for the end to save.
                        if !past_open {
                            match catch_unwind(AssertUnwindSafe(|| engine.apply_edits(batch))) {
                                Ok(reached_open) => past_open = reached_open,
                                Err(_) => {
                                    engine.close();
                                    past_open = true;
                                }
                            }
                        }
                        if stop {
                            break;
                        }
                        continue;
                    }
                    let work = || {
                        let mut events = if batch.is_empty() {
                            Vec::new()
                        } else {
                            engine.apply(batch)
                        };
                        // What arrived meanwhile, during the batch's render included.
                        events.extend(engine.scopes());
                        events
                    };
                    let events = match catch_unwind(AssertUnwindSafe(work)) {
                        Ok(events) => events,
                        Err(panic) => {
                            // Whatever it was doing may be half done: start again empty,
                            // without saving what may be half an edit.
                            engine.close();
                            let message = panic_message(panic.as_ref());
                            vec![Event::Failed(SessionError::Panicked(message))]
                        }
                    };
                    // The batch's events are one burst, one wake.
                    outbox.send_all(events);
                    // Then, the frame sent, one step of one waiting GPU task, once the
                    // last is done.
                    engine.jobs.step(&engine.gpu);
                }
                engine.finish(received);
            })
            .expect("the engine thread starts");
        Session {
            commands: Some(commands),
            sent: AtomicU64::new(0),
            events,
            stop,
            thread: Some(thread),
            gate,
        }
    }

    /// Sends `command` and returns its number: 1 for the first, then up by one each.
    pub fn send(&self, command: Command) -> u64 {
        if let Some(commands) = &self.commands {
            // The thread outlives the handle, so this only fails after it panicked,
            // which its join reports.
            let _ = commands.send(Inbox::Command(Instant::now(), Box::new(command)));
        }
        self.sent.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// The events that have arrived, oldest first; never waits. Every event comes once, in
    /// the order its thread sent it; once the thread has ended they end with one
    /// [`SessionError::Stopped`]. The next event after this look wakes the client again.
    pub fn events(&self) -> Vec<Event> {
        self.events.events()
    }
}

impl Drop for Session {
    /// Ends the thread once its current batch is done: the edit commands still queued
    /// apply, up to one that opens another photo, nothing more renders, and the edit is
    /// saved. Then the jobs end, as [`Control::shutdown`] ends them, cancellable ones
    /// cancelled and the rest finished, the thread stepping their GPU tasks until they
    /// have. The batch and the tasks may submit, so the thread is joined with the submit
    /// gate free, should the dropping thread hold it, as a window's toolkit does.
    fn drop(&mut self) {
        // No one will look: the session's threads send nothing more, nor wake.
        self.events.close();
        self.stop.store(true, Ordering::Relaxed);
        if let Some(commands) = self.commands.take() {
            // The jobs' bodies keep the inbox open, so the thread is told.
            let _ = commands.send(Inbox::Stop);
        }
        if let Some(thread) = self.thread.take()
            && self.gate.released(|| thread.join()).is_err()
        {
            tracing::error!(target: "pipe", "engine thread panicked");
        }
    }
}

/// Writes edits off the engine thread: only the latest of each file it has been handed,
/// each through a temporary file renamed into place, then tells the client. It keeps, per
/// file, how many edits are still to write and whether the last write failed, so the
/// engine can wait for one file and write a failed one again. It also reads sidecars for an
/// export's snapshot (D12), in order with the writes: every edit handed over before the
/// read is written first.
struct Writer {
    jobs: Option<mpsc::Sender<Order>>,
    state: Arc<(Mutex<Writes>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

/// What the writer is asked, in order.
enum Order {
    Write(Job),
    /// Read these sidecars, once the writes before are done, and answer with them.
    Read {
        paths: Vec<PathBuf>,
        answer: mpsc::Sender<Vec<Snapshot>>,
    },
}

/// One edit to write: its file, its XMP, and how many commands it includes.
struct Job {
    path: PathBuf,
    xml: String,
    through: u64,
    /// Whether the file holds it already, as the edit a cancelled gesture puts back
    /// (decision 42): reported saved, and written only after a failed write.
    held: bool,
}

#[derive(Default)]
struct Writes {
    /// Edits handed over and not yet written, by file.
    pending: BTreeMap<PathBuf, usize>,
    /// Files whose last write failed.
    failed: BTreeSet<PathBuf>,
}

impl Writer {
    fn start(outbox: Outbox) -> Writer {
        let (jobs, received) = mpsc::channel::<Order>();
        let state = Arc::new((Mutex::new(Writes::default()), Condvar::new()));
        let shared = Arc::clone(&state);
        let thread = std::thread::Builder::new()
            .name("edit writer".to_owned())
            .spawn(move || {
                while let Ok(first) = received.recv() {
                    // The latest edit of each file, and how many it stands for, since the
                    // last read.
                    let mut writes: Vec<(Job, usize)> = Vec::new();
                    for order in std::iter::once(first).chain(received.try_iter()) {
                        let (paths, answer) = match order {
                            Order::Write(job) => {
                                let queued = writes.iter_mut().find(|(q, _)| q.path == job.path);
                                match queued {
                                    Some((queued, count)) => {
                                        // A held edit in place of one still to write is
                                        // written.
                                        let held = queued.held && job.held;
                                        *queued = Job { held, ..job };
                                        *count += 1;
                                    }
                                    None => writes.push((job, 1)),
                                }
                                continue;
                            }
                            Order::Read { paths, answer } => (paths, answer),
                        };
                        // Every edit handed over before the read is on disk first (D12).
                        write_edits(std::mem::take(&mut writes), &shared, &outbox);
                        let read = paths.iter().map(|path| Snapshot::read(path)).collect();
                        // A job that has gone hears nothing.
                        let _ = answer.send(read);
                    }
                    write_edits(writes, &shared, &outbox);
                }
            })
            .expect("the edit writer starts");
        Writer {
            jobs: Some(jobs),
            state,
            thread: Some(thread),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Writes> {
        self.state.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Hands over the edit to write to `path`, which `xml` holds, as it stood after the
    /// first `through` commands; `held` where the file holds it already, as the edit a
    /// cancelled gesture puts back, reported saved and written only should the last write
    /// of `path` have failed by then.
    fn write(&self, path: PathBuf, xml: String, through: u64, held: bool) {
        let Some(jobs) = &self.jobs else {
            return;
        };
        *self.lock().pending.entry(path.clone()).or_default() += 1;
        let _ = jobs.send(Order::Write(Job {
            path,
            xml,
            through,
            held,
        }));
    }

    /// Reads the sidecars at `paths` for an export's snapshot, once every edit handed over
    /// before is written (D12); what this returns hears them in their order, or that the
    /// writer has gone.
    fn read(&self, paths: Vec<PathBuf>) -> mpsc::Receiver<Vec<Snapshot>> {
        let (answer, answered) = mpsc::channel();
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(Order::Read { paths, answer });
        }
        answered
    }

    /// Waits until every edit of `path` handed over is written or has failed; or, with
    /// `None`, every edit of every file.
    fn flush(&self, path: Option<&Path>) {
        let mut state = self.lock();
        let waiting = |state: &Writes| match path {
            Some(path) => state.pending.contains_key(path),
            None => !state.pending.is_empty(),
        };
        while waiting(&state) {
            // A writer that has ended writes nothing more.
            if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
                return;
            }
            state = self
                .state
                .1
                .wait_timeout(state, Duration::from_millis(50))
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// Whether the last write of `path` failed.
    fn failed(&self, path: &Path) -> bool {
        self.lock().failed.contains(path)
    }
}

/// Writes the latest edit of each file in `writes`, with how many edits each stands for,
/// telling the client of each and the engine, which may wait for one, once it is done.
fn write_edits(writes: Vec<(Job, usize)>, shared: &(Mutex<Writes>, Condvar), outbox: &Outbox) {
    for (
        Job {
            path,
            xml,
            through,
            held,
        },
        count,
    ) in writes
    {
        let start = Instant::now();
        // An edit the file holds already is written only after a failure.
        let write = !held
            || shared
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .failed
                .contains(&path);
        let result = if write {
            sidecar::write_xml(&path, &xml)
        } else {
            Ok(())
        };
        let failed = result.is_err();
        let event = match result {
            Ok(()) => {
                let name = sidecar::file_name(&path);
                if write {
                    let ms = start.elapsed().as_secs_f64() * 1e3;
                    tracing::debug!(target: "pipe", "saved {name} in {ms:.1} ms");
                } else {
                    tracing::debug!(target: "pipe", "{name} holds the edit");
                }
                Event::Saved {
                    path: path.clone(),
                    through,
                }
            }
            Err(source) => {
                tracing::warn!(target: "pipe", "cannot save {}: {source}", path.display());
                Event::Failed(SessionError::Save {
                    path: path.clone(),
                    through,
                    source,
                })
            }
        };
        // Told before the engine stops waiting, so what it waited for has been reported.
        outbox.send(event);
        let (lock, written) = shared;
        let mut state = lock.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(left) = state.pending.get_mut(&path) {
            *left = left.saturating_sub(count);
            if *left == 0 {
                state.pending.remove(&path);
            }
        }
        if failed {
            state.failed.insert(path);
        } else {
            state.failed.remove(&path);
        }
        written.notify_all();
    }
}

impl Drop for Writer {
    /// Writes what it still has, then ends.
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!(target: "pipe", "edit writer panicked");
        }
    }
}

/// The engine thread's state.
struct Engine {
    gpu: GpuContext,
    operations: &'static [&'static dyn Operation],
    /// The modules that hold, in order.
    modules: Vec<&'static dyn Module>,
    /// The ids of the modules Apertrust has retired, whose values a saved edit loses
    /// without making the photo view-only: `apertrust_modules::RETIRED`.
    retired: &'static [&'static str],
    /// The build every edit written records.
    build: String,
    limits: Limits,
    load: Load,
    size: Option<(u32, u32)>,
    open: Option<Open>,
    /// Whether a live frame shares the GPU with the export jobs, a photo open and the
    /// viewport's size given: beside one, a job's operations before the tiles take a step
    /// per pass, in row strips; with none, as in the CLI's session, one step.
    live_frame: Arc<AtomicBool>,
    /// The two display buffers and the one published last.
    display: Option<Display>,
    /// How many commands have been applied.
    applied: u64,
    writer: Writer,
    /// What the scopes count of each frame: nothing, or the totals and these graphs.
    graphs: Option<Graphs>,
    /// The scopes' count and readback, made with the first frame counted.
    meter: Option<Meter>,
    /// Whether the scopes failed, which turns them off for the session.
    scopes_failed: bool,
    /// The session's jobs and the GPU tasks they hand it (decision 57).
    jobs: Jobs,
    /// The latest export preview's job, which the next one cancels.
    preview: Option<JobId>,
    /// The job reading the folder last opened, which the next open or a close cancels.
    folder: Option<JobId>,
    /// The folder last opened, with its open's number, whose thumbnails are asked for.
    shown: Option<(u64, PathBuf)>,
    /// The thumbnail jobs asked for of that folder that may not have ended, which the next
    /// open or a close cancels.
    thumbnails: Vec<JobId>,
    /// Holds the next export's renders for a test.
    #[cfg(test)]
    export_hold: Option<export_job::testing::Hold>,
    /// Holds the next export preview for a test.
    #[cfg(test)]
    preview_hold: Option<export_job::testing::Hold>,
    /// Holds the next folder's reading for a test.
    #[cfg(test)]
    folder_hold: Option<export_job::testing::Hold>,
    /// Holds the next thumbnails asked for, for a test.
    #[cfg(test)]
    thumbnails_hold: Option<export_job::testing::Hold>,
}

/// The session's jobs (decision 57): its control, and the GPU tasks their bodies hand
/// the engine thread.
struct Jobs {
    /// Before the control, so that, dropped after a panic, the tasks go first and their
    /// bodies hear the engine stopped before the control's drop waits for them.
    background: Background,
    control: Control,
    /// The engine thread's inbox, which each job's [`Tasks`] hands its tasks to.
    inbox: mpsc::Sender<Inbox>,
    /// The client's outbox, for what a job's body reports itself, as an export's photos.
    outbox: Outbox,
    /// The number of the command whose job is being added, for its `Added`; 0 between.
    starting: Arc<AtomicU64>,
    /// The jobs shown, those with a progress, from their `Added` to their end, and
    /// whether each can be cancelled.
    shown: Arc<Mutex<BTreeMap<JobId, bool>>>,
    /// What the jobs that offer Try again left to run for it (decision 72).
    retries: Retries,
    /// Makes [`Jobs::finish`]'s stepping panic, for the test of what follows.
    #[cfg(test)]
    panic_in_finish: bool,
}

impl Jobs {
    /// Starts the control, its events going to the client through `outbox` as
    /// [`Event::Job`], and the tasks its jobs hand going into `inbox`.
    fn start(outbox: Outbox, inbox: mpsc::Sender<Inbox>) -> Jobs {
        let starting = Arc::new(AtomicU64::new(0));
        let shown: Arc<Mutex<BTreeMap<JobId, bool>>> = Arc::default();
        let observer = {
            let (starting, shown) = (Arc::clone(&starting), Arc::clone(&shown));
            let outbox = outbox.clone();
            // Under the control's lock, so quick: a map's entry and a channel's send.
            move |event: JobEvent| {
                let mut jobs = shown.lock().unwrap_or_else(PoisonError::into_inner);
                let command = match &event {
                    JobEvent::Added {
                        job, cancellable, ..
                    } => {
                        jobs.insert(*job, *cancellable);
                        // Added on the engine thread, inside `Jobs::add`.
                        Some(starting.load(Ordering::Relaxed)).filter(|&command| command != 0)
                    }
                    JobEvent::Ended { job, .. } => {
                        jobs.remove(job);
                        None
                    }
                    _ => None,
                };
                drop(jobs);
                outbox.send(Event::Job { event, command });
            }
        };
        Jobs {
            background: Background::default(),
            control: Control::start(observer),
            inbox,
            outbox,
            starting,
            shown,
            retries: Retries::default(),
            #[cfg(test)]
            panic_in_finish: false,
        }
    }

    /// Adds the job `job` makes with its way to the engine thread, to `queue`, for the
    /// command numbered `command`, which its `Added` names.
    fn add(&mut self, command: u64, queue: Queue, job: impl FnOnce(Tasks) -> jobs::Job) -> JobId {
        let job = job(Tasks::new(self.inbox.clone(), queue));
        self.starting.store(command, Ordering::Relaxed);
        let id = self.control.add(queue, job);
        self.starting.store(0, Ordering::Relaxed);
        id
    }

    /// [`Command::CancelJob`]: a job shown that can be cancelled is; one that cannot is
    /// refused; one not shown, ended or never with a progress, changes nothing.
    fn cancel(&self, job: JobId) -> Result<(), SessionError> {
        // Looked up first: the control tells the observer, which takes the map.
        let cancellable = self
            .shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&job)
            .copied();
        match cancellable {
            Some(true) => {
                self.control.cancel(job);
                Ok(())
            }
            Some(false) => Err(SessionError::NotCancellable { job }),
            None => Ok(()),
        }
    }

    /// One step of one waiting GPU task, after the live frame.
    fn step(&mut self, gpu: &GpuContext) {
        self.background.step(gpu, &self.control);
    }

    /// Shuts the control down (difference J9): queued jobs discarded, cancellable ones
    /// cancelled, the rest finished, the tasks they hand meanwhile stepped until every
    /// worker has ended, so a body waiting for the engine thread never holds up the
    /// session's end. Should that stepping panic, the inbox and the tasks go at once, so
    /// their bodies hear the engine stopped and the workers end, and the panic goes on
    /// once they have.
    fn finish(&mut self, gpu: &GpuContext, inbox: mpsc::Receiver<Inbox>) {
        #[cfg(test)]
        let panics = self.panic_in_finish;
        let Jobs {
            background,
            control,
            ..
        } = self;
        let control = &*control;
        let panicked = std::thread::scope(|scope| {
            // `shutdown` joins the workers, so it waits on a thread of its own while this
            // one steps what they hand it.
            let stopping = std::thread::Builder::new()
                .name("jobs shutdown".to_owned())
                .spawn_scoped(scope, || control.shutdown())
                .expect("the jobs shutdown thread starts");
            let served = catch_unwind(AssertUnwindSafe(|| {
                while !stopping.is_finished() {
                    #[cfg(test)]
                    assert!(!panics, "the jobs' end panicked, as a test asked");
                    // The session sends nothing after its `Stop`; the tasks still come.
                    if let Ok(Inbox::Task(waiting)) = inbox.recv_timeout(POLL_INTERVAL) {
                        background.add(waiting);
                    }
                    for item in inbox.try_iter() {
                        if let Inbox::Task(waiting) = item {
                            background.add(waiting);
                        }
                    }
                    background.step(gpu, control);
                }
            }));
            // Before the scope waits for the shutdown, which waits for the workers.
            drop(inbox);
            served
                .err()
                .inspect(|_| *background = Background::default())
        });
        if let Some(panic) = panicked {
            std::panic::resume_unwind(panic);
        }
    }
}

/// How many retries the session keeps: the latest jobs' that offered Try again, for its
/// life (decision 72).
pub const RETRIES_KEPT: usize = 64;

/// What the session's jobs leave it to run for Try again (decision 72): each job that
/// offers [`jobs::Offer::Retry`] keeps, before its body returns, the command its retry
/// runs, so the command is kept before its `Ended` reaches a client. The latest
/// [`RETRIES_KEPT`], for the session's life; [`Command::RetryJob`] takes one, so each is
/// used once.
#[derive(Clone, Default)]
pub(crate) struct Retries(Arc<Mutex<VecDeque<(JobId, Command)>>>);

impl Retries {
    /// Keeps `command` as what trying `job` again runs, the oldest kept dropped past
    /// [`RETRIES_KEPT`].
    pub(crate) fn keep(&self, job: JobId, command: Command) {
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.len() == RETRIES_KEPT
            && let Some((old, _)) = kept.pop_front()
        {
            tracing::debug!(target: "control", "dropped the retry of job {old}, past {RETRIES_KEPT}");
        }
        kept.push_back((job, command));
        tracing::debug!(target: "control", "kept the retry of job {job}");
    }

    /// What trying `job` again runs, taken: none where none is kept.
    fn take(&self, job: JobId) -> Option<Command> {
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let at = kept.iter().position(|(own, _)| *own == job)?;
        kept.remove(at).map(|(_, command)| command)
    }
}

struct Open {
    /// The edit's file, beside the photo.
    sidecar: PathBuf,
    image: ImageContext,
    input: GpuImage,
    sensor: ImageDesc,
    /// What an export's variables read of the raw, so that its export needs no decode.
    shot: Shot,
    edit: Edit,
    /// Set when a newer Apertrust wrote the edit: no change applies, and nothing is ever
    /// written.
    view_only: Option<ViewOnly>,
    /// Whether the edit has changed since it was last handed to the writer.
    unsaved: bool,
    /// The modules whose binding failed at the last render, each reported once.
    failing: BTreeSet<&'static str>,
    /// The modules whose preset rows panicked as the values were reported, each warned
    /// of once for the photo.
    presets_failed: BTreeSet<&'static str>,
    /// The modules whose availability panicked as the values were reported, each warned
    /// of once for the photo (decision 30).
    available_failed: BTreeSet<&'static str>,
    /// The modules whose ranges failed as the values were reported, each warned of once
    /// for the photo (decision 30).
    ranges_failed: BTreeSet<&'static str>,
    /// The options each choice lists on the photo, by module and control, computed once as
    /// it opened (decision 30).
    options: BTreeMap<(&'static str, &'static str), Vec<&'static str>>,
    /// The coordinates of lossless views held through gestures (decision 2).
    held: Held,
    /// The gesture under way, which [`Command::CancelGesture`] puts back (decision 42).
    gesture: Option<Gesture>,
    /// The modules whose views failed as the values were reported, each warned of once
    /// for the photo.
    views_failed: BTreeSet<&'static str>,
    pipeline: Option<Pipeline>,
}

/// A gesture under way (decision 42): the run of `SetControl { done: false }` on one
/// module, and what its cancel puts back, the photo's edit as it stood before its first
/// change. The whole edit, so whatever a change writes beside its own control, follow
/// writes and clamps, and a switched module's state once decision 32 gives the edit one,
/// comes back with it.
struct Gesture {
    module: String,
    edit: Edit,
    held: Held,
    /// Whether `edit` differs from the edit handed to the writer last, so that putting it
    /// back leaves something to write.
    unsaved: bool,
}

struct Display {
    buffers: [wgpu::Buffer; 2],
    /// The frames' width and height: a client draws a buffer with the size of the frame
    /// it was sent, so a buffer holds frames of one size only.
    size: (u32, u32),
    published: usize,
    /// The commands the published frame includes.
    through: u64,
}

/// What an edit command did: whether the image may have changed, whether it completes a
/// change, which saves the edit, and whether it is a cancel, which the writer reports
/// saved, without writing, where the file holds the edit.
#[derive(Debug, Default)]
struct Effect {
    render: bool,
    save: bool,
    cancelled: bool,
}

impl Engine {
    /// The engine, planning within `budget` bytes, and an event for each module it
    /// leaves out.
    fn new(
        gpu: GpuContext,
        operations: &'static [&'static dyn Operation],
        modules: &'static [&'static dyn Module],
        build: String,
        budget: u64,
        writer: Writer,
        jobs: Jobs,
    ) -> (Self, Vec<Event>) {
        let limits = Limits {
            budget,
            ..Limits::of(&gpu)
        };
        let (modules, rejected) = check::accept(modules, operations);
        let rejected = rejected
            .into_iter()
            .map(|rejected| {
                tracing::error!(target: "pipe", "module {} left out: {}", rejected.module, rejected.error);
                Event::Failed(SessionError::Module {
                    module: rejected.module,
                    source: rejected.error,
                })
            })
            .collect();
        let engine = Engine {
            gpu,
            operations,
            modules,
            retired: apertrust_modules::RETIRED,
            build,
            limits,
            load: decode,
            size: None,
            open: None,
            live_frame: Arc::new(AtomicBool::new(false)),
            display: None,
            applied: 0,
            writer,
            graphs: None,
            meter: None,
            scopes_failed: false,
            jobs,
            preview: None,
            folder: None,
            shown: None,
            thumbnails: Vec::new(),
            #[cfg(test)]
            export_hold: None,
            #[cfg(test)]
            preview_hold: None,
            #[cfg(test)]
            folder_hold: None,
            #[cfg(test)]
            thumbnails_hold: None,
        };
        (engine, rejected)
    }

    /// Applies a batch of commands, then renders once if any of them changed the image.
    fn apply(&mut self, batch: Vec<(Instant, Command)>) -> Vec<Event> {
        self.guarded(|engine| engine.apply_batch(batch))
    }

    /// The scopes' readings that have arrived, never waiting.
    fn scopes(&mut self) -> Vec<Event> {
        if !self.reading() {
            return Vec::new();
        }
        let mut readings = Vec::new();
        let (polled, error) = self.caught(|engine| match &mut engine.meter {
            Some(meter) => meter.poll(&engine.gpu, &mut readings),
            None => Ok(()),
        });
        let mut events: Vec<Event> = readings.into_iter().map(Event::Scopes).collect();
        if let Some(error) = polled.err().map(|e| e.to_string()).or(error) {
            self.scopes_fail(error, &mut events);
        }
        events
    }

    /// Whether a scopes reading is on its way, so the thread looks at the GPU between
    /// commands.
    fn reading(&self) -> bool {
        self.meter.as_ref().is_some_and(Meter::pending)
    }

    /// Runs `work`, the GPU's errors on this thread becoming events rather than the
    /// panics wgpu makes of them.
    fn guarded(&mut self, work: impl FnOnce(&mut Self) -> Vec<Event>) -> Vec<Event> {
        let (mut events, error) = self.caught(work);
        if let Some(error) = error {
            // A frame rendered under the error is not shown, nor counted; the next render
            // starts over.
            events.retain(|event| !matches!(event, Event::Frame(_)));
            if let Some(open) = &mut self.open {
                open.pipeline = None;
            }
            self.display = None;
            if let Some(meter) = &mut self.meter {
                meter.discard();
            }
            events.push(Event::Failed(SessionError::Gpu(error)));
        }
        events
    }

    /// Runs `work` inside error scopes of its own, and returns what it returned and the
    /// first GPU error it caused, rather than the panic wgpu makes of one.
    fn caught<T>(&mut self, work: impl FnOnce(&mut Self) -> T) -> (T, Option<String>) {
        let memory = self
            .gpu
            .device
            .push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let validation = self
            .gpu
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let result = work(self);
        let errors = [validation.pop(), memory.pop()].map(pollster::block_on);
        let error = errors.into_iter().flatten().next();
        (result, error.map(|error| error.to_string()))
    }

    /// Counts the frame in `buffer`, of `size` pixels, for the scopes, if a client asks
    /// for them, in a submission of its own after the frame's copy. A meter that cannot be
    /// made, or a count the GPU refuses, turns the scopes off, never the frame.
    fn count(
        &mut self,
        buffer: &wgpu::Buffer,
        size: [u32; 2],
        through: u64,
        events: &mut Vec<Event>,
    ) {
        let (Some(graphs), false) = (self.graphs, self.scopes_failed) else {
            return;
        };
        let (counted, error) = self.caught(|engine| -> Result<(), ComputeError> {
            if engine.meter.is_none() {
                engine.meter = Some(pollster::block_on(Meter::new(&engine.gpu))?);
            }
            if let Some(meter) = &mut engine.meter {
                meter.measure_buffer(&engine.gpu, buffer, size, graphs, through);
            }
            Ok(())
        });
        if let Some(error) = counted.err().map(|e| e.to_string()).or(error) {
            self.scopes_fail(error, events);
        }
    }

    /// Turns the scopes off for the rest of the session, reporting why once: the frames go
    /// on without them.
    fn scopes_fail(&mut self, error: String, events: &mut Vec<Event>) {
        tracing::error!(target: "pipe", "scopes off: {error}");
        self.meter = None;
        self.scopes_failed = true;
        events.push(Event::Failed(SessionError::Scopes(error)));
    }

    /// Drops the photo and everything rendered from it, unsaved, with the readings of its
    /// frames still on their way.
    fn close(&mut self) {
        self.open = None;
        self.tell_live_frame();
        self.display = None;
        if let Some(meter) = &mut self.meter {
            meter.discard();
        }
    }

    /// Tells the export jobs whether a live frame now shares the GPU with them
    /// ([`Self::live_frame`]), after the photo or the viewport's size changes.
    fn tell_live_frame(&self) {
        let live = self.open.is_some() && self.size.is_some();
        self.live_frame.store(live, Ordering::Relaxed);
    }

    /// Saves what the photo's edit still needs, then drops the photo.
    fn close_saving(&mut self) {
        self.save_unsaved();
        self.close();
    }

    /// The session's end: saves a change never completed, then ends the jobs, stepping
    /// the GPU tasks that still come through `inbox` until they have ended, and waits for
    /// the writer. A panic saving the edit still ends the jobs first, `inbox` going before
    /// the control waits for its workers, and goes on once they have.
    fn finish(mut self, inbox: mpsc::Receiver<Inbox>) {
        let saved = catch_unwind(AssertUnwindSafe(|| self.close_saving()));
        self.jobs.finish(&self.gpu, inbox);
        if let Err(panic) = saved {
            std::panic::resume_unwind(panic);
        }
    }

    fn apply_batch(&mut self, batch: Vec<(Instant, Command)>) -> Vec<Event> {
        let Some(&(since, _)) = batch.first() else {
            return Vec::new();
        };
        let mut events = Vec::new();
        let (mut values, mut render, mut save, mut cancelled) = (false, false, false, false);
        // Whether the scopes now count a graph the published frame was not counted for.
        let mut recount = false;
        for (_, command) in batch {
            self.applied += 1;
            // Values follow every change asked for, failed or not, so a client's own
            // value is replaced by the engine's either way.
            values |= on_edit(&command);
            let result = match command {
                Command::Open(path) => {
                    // Its number, which its `Opened` or its failure carries.
                    let open = self.applied;
                    self.open(&path, open)
                        .map(|photo| {
                            events.push(Event::Opened(photo));
                            (values, render, save, cancelled) = (true, true, false, false);
                        })
                        .map_err(|error| SessionError::Open {
                            open,
                            error: Box::new(error),
                        })
                }
                Command::Resize { width, height } => {
                    render |= self.size != Some((width, height));
                    self.size = Some((width, height));
                    self.tell_live_frame();
                    Ok(())
                }
                Command::Scopes(graphs) => {
                    recount |= match (graphs, self.graphs) {
                        (Some(asked), Some(counted)) => asked.adds_to(counted),
                        (asked, _) => asked.is_some(),
                    };
                    self.graphs = graphs;
                    Ok(())
                }
                Command::CancelJob { job } => self.jobs.cancel(job),
                Command::RetryJob { job } => self.retry(job),
                Command::Export {
                    photos,
                    settings,
                    places,
                } => {
                    self.export(photos, settings, places, None);
                    Ok(())
                }
                #[cfg(test)]
                Command::Synthetic(synthetic) => {
                    let command = self.applied;
                    self.jobs
                        .add(command, synthetic.queue, |tasks| synthetic.job(tasks));
                    Ok(())
                }
                Command::PreviewExport {
                    photos,
                    settings,
                    places,
                } => {
                    self.preview_export(photos, settings, places);
                    Ok(())
                }
                Command::Library(command) => {
                    self.library(command, &mut events);
                    Ok(())
                }
                #[cfg(test)]
                Command::HoldExport(hold) => {
                    self.export_hold = Some(hold);
                    Ok(())
                }
                #[cfg(test)]
                Command::HoldPreview(hold) => {
                    self.preview_hold = Some(hold);
                    Ok(())
                }
                #[cfg(test)]
                Command::HoldFolder(hold) => {
                    self.folder_hold = Some(hold);
                    Ok(())
                }
                #[cfg(test)]
                Command::HoldThumbnails(hold) => {
                    self.thumbnails_hold = Some(hold);
                    Ok(())
                }
                command => self.apply_edit(command).map(|effect| {
                    render |= effect.render;
                    save |= effect.save;
                    cancelled |= effect.cancelled;
                }),
            };
            match result {
                Ok(()) => {}
                // An open's failure names its open already.
                Err(error @ SessionError::Open { .. }) => events.push(Event::Failed(error)),
                Err(error) => events.push(Event::Failed(SessionError::Refused {
                    command: self.applied,
                    error: Box::new(error),
                })),
            }
        }
        if let Some(open) = &mut self.open
            && values
        {
            events.push(Event::Values(open.values(
                self.operations,
                &self.modules,
                self.applied,
            )));
        }
        let needs_save = self.needs_save();
        if save && needs_save {
            self.save();
        } else if cancelled && !needs_save {
            // The writer has the edit a cancel put back or left as it was, and reports it
            // saved without writing it, unless its last write has failed by then.
            self.save_held();
        }
        if render {
            match self.render(since, &mut events) {
                Ok(Some(frame)) => events.push(Event::Frame(frame)),
                Ok(None) => {}
                Err(error) => events.push(Event::Failed(error)),
            }
        } else if recount && let Some(display) = &self.display {
            // The frame shown, counted again for what it was not counted for.
            let buffer = display.buffers[display.published].clone();
            let (size, through) = ([display.size.0, display.size.1], display.through);
            self.count(&buffer, size, through, &mut events);
        }
        events
    }

    /// Applies the edit commands of a batch that arrived after the client dropped its
    /// handle, rendering nothing, up to one that opens another photo; whether it reached
    /// one.
    fn apply_edits(&mut self, batch: Vec<(Instant, Command)>) -> bool {
        for (_, command) in batch {
            if matches!(command, Command::Open(_)) {
                return true;
            }
            self.applied += 1;
            // A refused change is refused as it would have been; no one is told.
            let _ = self.apply_edit(command);
        }
        false
    }

    /// Applies one command that changes the edit; on a photo open for viewing only, refuses
    /// it.
    fn apply_edit(&mut self, command: Command) -> Result<Effect, SessionError> {
        let edits = on_edit(&command);
        if let Some(view_only) = self.open.as_ref().and_then(|open| open.view_only.as_ref())
            && edits
        {
            return Err(SessionError::ViewOnly(view_only.clone()));
        }
        if let Some(open) = &mut self.open
            && edits
        {
            open.gesture_through(&command);
        }
        let (changed, completes) = match command {
            Command::SetControl {
                module,
                control,
                value,
                done,
            } => {
                let changed = self.change_holding(|edit, held, modules, _, image| {
                    let module = edit::module(modules, &module)?;
                    let before = edit.clone();
                    if module.definition().coordinate(&control).is_some() {
                        edit.set_coordinate(held, module, &control, value, image)?;
                    } else {
                        edit.set_control(module, &control, value, image)?;
                    }
                    edit.follow(&before, module, edit::written(module, &control), image)
                })?;
                (changed, done)
            }
            Command::ResetControl { module, control } => {
                // A reset is a change to the default, with its follow writes.
                let changed = self.change_holding(|edit, held, modules, _, image| {
                    let module = edit::module(modules, &module)?;
                    let before = edit.clone();
                    if module.definition().coordinate(&control).is_some() {
                        edit.reset_coordinate(held, module, &control, image)?;
                    } else {
                        edit.reset_control(module, &control)?;
                    }
                    edit.follow(&before, module, edit::written(module, &control), image)
                })?;
                (changed, true)
            }
            Command::ResetModule { module } => {
                let changed = self.change(|edit, modules, _, _| {
                    edit.reset_module(edit::module(modules, &module)?);
                    Ok(())
                })?;
                (changed, true)
            }
            Command::Keep {
                module,
                control,
                kept,
            } => {
                self.change(|edit, modules, _, _| {
                    let module = edit::module(modules, &module)?;
                    if module.definition().coordinate(&control).is_some() {
                        edit.keep_coordinate(module, &control, kept)
                    } else {
                        edit.keep(module, &control, kept)
                    }
                })?;
                // A mark changes no pixel.
                return Ok(Effect {
                    render: false,
                    save: true,
                    cancelled: false,
                });
            }
            Command::CancelGesture { module } => {
                let open = self.open.as_mut();
                let Some(changed) = open.and_then(|open| open.cancel(&module)) else {
                    // No gesture under way on it: nothing to put back, nor to save.
                    return Ok(Effect {
                        render: false,
                        save: false,
                        cancelled: true,
                    });
                };
                // No history step: the edit is as it was before the gesture.
                return Ok(Effect {
                    render: changed,
                    save: true,
                    cancelled: true,
                });
            }
            Command::ApplyPreset {
                module,
                preset,
                option,
            } => {
                // An option's values are the module's own, every control it describes,
                // so they take no follow writes, as darktable's `_preset_tune_callback`
                // writes its parameters without `gui_changed`.
                let changed = self.change(|edit, modules, _, image| {
                    edit.apply_preset(edit::module(modules, &module)?, &preset, &option, image)
                })?;
                (changed, true)
            }
            Command::Set {
                operation,
                param,
                value,
            } => {
                let changed = self.change(|edit, modules, operations, _| {
                    edit.set_param(operations, modules, &operation, &param, value)
                })?;
                (changed, false)
            }
            Command::RetrySave => {
                // Only an edit whose last write failed is written again.
                let failed =
                    (self.open.as_ref()).is_some_and(|open| self.writer.failed(&open.sidecar));
                return Ok(Effect {
                    render: false,
                    save: failed,
                    cancelled: false,
                });
            }
            Command::Reset => {
                let changed = self.change(|edit, _, _, _| {
                    *edit = Edit::default();
                    Ok(())
                })?;
                (changed, true)
            }
            Command::Open(_)
            | Command::Resize { .. }
            | Command::Scopes(_)
            | Command::CancelJob { .. }
            | Command::RetryJob { .. }
            | Command::Export { .. }
            | Command::PreviewExport { .. }
            | Command::Library(_) => {
                return Ok(Effect::default());
            }
            #[cfg(test)]
            Command::Synthetic(_)
            | Command::HoldExport(_)
            | Command::HoldPreview(_)
            | Command::HoldFolder(_)
            | Command::HoldThumbnails(_) => {
                return Ok(Effect::default());
            }
        };
        Ok(Effect {
            render: changed,
            save: completes,
            cancelled: false,
        })
    }

    /// Changes the open photo's edit, given its image context, and says whether it
    /// changed; `false`, changing nothing, when no photo is open.
    fn change(
        &mut self,
        change: impl FnOnce(
            &mut Edit,
            &[&dyn Module],
            &[&dyn Operation],
            &ImageContext,
        ) -> Result<(), EditError>,
    ) -> Result<bool, SessionError> {
        self.change_holding(|edit, _, modules, operations, image| {
            change(edit, modules, operations, image)
        })
    }

    /// [`Self::change`], given the coordinates held through gestures on lossless views
    /// too. Whatever the change, the coordinates of a view whose controls it changed but
    /// through the view are dropped: its gesture is over.
    fn change_holding(
        &mut self,
        change: impl FnOnce(
            &mut Edit,
            &mut Held,
            &[&dyn Module],
            &[&dyn Operation],
            &ImageContext,
        ) -> Result<(), EditError>,
    ) -> Result<bool, SessionError> {
        let Some(open) = &mut self.open else {
            return Ok(false);
        };
        let before = open.edit.clone();
        let changed = change(
            &mut open.edit,
            &mut open.held,
            &self.modules,
            self.operations,
            &open.image,
        );
        // A number a moved range leaves outside comes back inside, in the same change
        // (decision 30).
        let changed = changed.and_then(|()| open.edit.clamp(&before, &self.modules, &open.image));
        open.held.prune(&open.edit);
        changed?;
        let changed = open.edit != before;
        open.unsaved |= changed;
        Ok(changed)
    }

    /// Whether the open photo's edit differs from what was last written: changed since,
    /// or its last write failed. Never for an edit a newer Apertrust wrote, whose file
    /// stays as it was.
    fn needs_save(&self) -> bool {
        self.open.as_ref().is_some_and(|open| {
            open.view_only.is_none() && (open.unsaved || self.writer.failed(&open.sidecar))
        })
    }

    /// Hands the open photo's edit to the writer, unless a newer Apertrust wrote it.
    fn save(&mut self) {
        self.hand_over(false);
    }

    /// Hands the open photo's edit to the writer as one the file holds already, as after a
    /// cancel that leaves the edit handed over last: reported saved, and written only
    /// should that write have failed.
    fn save_held(&mut self) {
        self.hand_over(true);
    }

    fn hand_over(&mut self, held: bool) {
        if let Some(open) = &mut self.open
            && open.view_only.is_none()
        {
            let xml = sidecar::to_xml(&open.edit, self.operations, &self.modules, &self.build);
            self.writer
                .write(open.sidecar.clone(), xml, self.applied, held);
            open.unsaved = false;
            // A batch that completes a change and begins a gesture hands over the gesture's
            // first change: the edit before the gesture is the writer's only where the two
            // are the same.
            if let Some(gesture) = &mut open.gesture {
                gesture.unsaved = gesture.edit != open.edit;
            }
        }
    }

    /// Before the photo closes: saves a change never completed, and writes again an edit
    /// whose last write failed, once any write still under way is done.
    fn save_unsaved(&mut self) {
        let Some(open) = &self.open else {
            return;
        };
        self.writer.flush(Some(&open.sidecar));
        if self.needs_save() {
            self.save();
        }
    }

    /// Opens the raw at `path` for the [`Command::Open`] numbered `open`.
    fn open(&mut self, path: &Path, open: u64) -> Result<Photo, SessionError> {
        // The old photo's edit is saved, and its buffers go before the new one's arrive.
        self.close_saving();
        let Decoded {
            camera,
            image,
            input,
            shot,
            decode_ms,
            upload_ms,
        } = (self.load)(&self.gpu, path)?;
        let sidecar = sidecar::path_for(path);
        // An edit of this photo still being written is read back whole.
        self.writer.flush(Some(&sidecar));
        let (edit, view_only) = if sidecar.exists() {
            tracing::debug!(target: "pipe", "read {}", sidecar::file_name(&sidecar));
            let loaded = sidecar::read(&sidecar, self.operations, &self.modules, self.retired)?;
            let view_only = loaded.view_only();
            if let Some(view_only) = &view_only {
                let (name, by) = (
                    sidecar::file_name(&sidecar),
                    sidecar::writer(&view_only.build),
                );
                tracing::warn!(target: "pipe", "view only: {name}, by {by}");
            }
            let mut edit = loaded.edit;
            // A number outside the range its module's values give, as only a hand-written
            // file holds, is clamped as it is read (decisions 21 and 30); an edit a newer
            // Apertrust wrote is shown as it was.
            if view_only.is_none() {
                for warning in edit.clamp_read(&self.modules, &image) {
                    tracing::warn!(target: "pipe", "{warning}");
                }
            }
            (edit, view_only)
        } else {
            (Edit::default(), None)
        };
        let photo = Photo {
            path: path.to_owned(),
            open,
            camera,
            width: image.width,
            height: image.height,
            decode_ms,
            upload_ms,
            view_only: view_only.clone(),
            exposure_time: shot.exposure_time,
            aperture: shot.aperture,
            focal_length: shot.focal_length,
            image: Box::new(image.clone()),
        };
        let options = listed_options(&self.modules, &image);
        self.open = Some(Open {
            sidecar,
            image,
            sensor: input.desc(),
            input,
            shot,
            edit,
            view_only,
            unsaved: false,
            failing: BTreeSet::new(),
            presets_failed: BTreeSet::new(),
            available_failed: BTreeSet::new(),
            ranges_failed: BTreeSet::new(),
            options,
            held: Held::default(),
            gesture: None,
            views_failed: BTreeSet::new(),
            pipeline: None,
        });
        self.tell_live_frame();
        Ok(photo)
    }

    /// [`Command::Export`], the command numbered [`Self::applied`]: a `UserExport` job of
    /// the session's control, its `Added` naming the job it `retries` where it is one's
    /// retry. Each photo's edit is taken now (D12): the open photo's from memory, and every
    /// other's read by the edit writer after the writes handed to it so far, the job
    /// waiting for them on its worker, so nothing here waits for a disk.
    fn export(
        &mut self,
        photos: Vec<PathBuf>,
        settings: ExportSettings,
        places: Places,
        retries: Option<JobId>,
    ) {
        let live = self.live_among(&photos);
        let sidecars = photos.iter().map(|path| sidecar::path_for(path)).collect();
        let run = export_job::Run {
            gpu: self.gpu.clone(),
            operations: self.operations,
            modules: self.modules.clone(),
            retired: self.retired,
            limits: self.limits,
            load: self.load,
            photos,
            settings,
            places,
            now: crate::names::now(),
            build: self.build.clone(),
            live,
            live_frame: Arc::clone(&self.live_frame),
            snapshot: self.writer.read(sidecars),
            outbox: self.jobs.outbox.clone(),
            retries: self.jobs.retries.clone(),
            #[cfg(test)]
            hold: self.export_hold.take(),
        };
        let command = self.applied;
        self.jobs.add(command, Queue::UserExport, |tasks| {
            let job = export_job::job(run, tasks);
            match retries {
                Some(old) => job.retries(old),
                None => job,
            }
        });
    }

    /// [`Command::RetryJob`], the command numbered [`Self::applied`]: the command `job`
    /// left, taken, so it runs once, and run as if it were sent now; refused where none is
    /// kept.
    fn retry(&mut self, job: JobId) -> Result<(), SessionError> {
        match self.jobs.retries.take(job) {
            Some(Command::Export {
                photos,
                settings,
                places,
            }) => {
                let count = photos.len();
                tracing::debug!(target: "control", "trying job {job} again: {count} photos");
                self.export(photos, settings, places, Some(job));
                Ok(())
            }
            // Only an export leaves a retry.
            _ => Err(SessionError::NotRetryable { job }),
        }
    }

    /// The open photo as an export of `photos` takes it, if it is one of them: the raw is
    /// held only for an export of it.
    fn live_among(&self, photos: &[PathBuf]) -> Option<Live> {
        let exported = |open: &&Open| photos.iter().any(|p| sidecar::path_for(p) == open.sidecar);
        self.open.as_ref().filter(exported).map(|open| Live {
            sidecar: open.sidecar.clone(),
            edit: open.edit.clone(),
            newer: open.view_only.is_some(),
            image: open.image.clone(),
            input: open.input.clone(),
            shot: open.shot.clone(),
            unsaved: open.unsaved,
        })
    }

    /// [`Command::PreviewExport`], the command numbered [`Self::applied`]: the preview
    /// before it is cancelled and this one's job added, to `UserForeground`.
    fn preview_export(&mut self, photos: Vec<PathBuf>, settings: ExportSettings, places: Places) {
        if let Some(older) = self.preview.take() {
            self.jobs.control.cancel(older);
        }
        let run = export_preview::Run {
            operations: self.operations,
            modules: self.modules.clone(),
            limits: self.limits,
            active: self
                .live_among(&photos)
                .as_ref()
                .map(export_preview::Active::of),
            photos,
            settings,
            places,
            now: crate::names::now(),
            build: self.build.clone(),
            through: self.applied,
            outbox: self.jobs.outbox.clone(),
            #[cfg(test)]
            hold: self.preview_hold.take(),
        };
        let command = self.applied;
        let job = self
            .jobs
            .add(command, Queue::UserForeground, |_| export_preview::job(run));
        self.preview = Some(job);
    }

    /// [`Command::Library`], the command numbered [`Self::applied`]. An open cancels the
    /// folder's reading before it and reads the new one as a job on darktable's
    /// `UserBackground` queue, a folder's reading as darktable's film import (decision 57),
    /// cancellable, its events going out as [`Event::Library`]. A close cancels the reading
    /// and answers at once. Either cancels the thumbnails asked for before it.
    fn library(&mut self, command: LibraryCommand, events: &mut Vec<Event>) {
        let source = match command {
            LibraryCommand::Thumbnails { photos, size } => {
                self.thumbnails(photos, size);
                return;
            }
            LibraryCommand::Open(source) => Some(source),
            LibraryCommand::Close => None,
        };
        if let Some(older) = self.folder.take() {
            self.jobs.control.cancel(older);
        }
        for job in self.thumbnails.drain(..) {
            self.jobs.control.cancel(job);
        }
        let open = self.applied;
        let Some(source) = source else {
            self.shown = None;
            events.push(Event::Library(LibraryEvent::Closed { open }));
            return;
        };
        let library::Source::Folder(path) = &source;
        self.shown = Some((open, path.clone()));
        let message = jobs::Message::new("job-folder").with("folder", path.as_path());
        let description = format!("reading {}", path.display());
        let outbox = self.jobs.outbox.clone();
        #[cfg(test)]
        let hold = self.folder_hold.take();
        let job = self.jobs.add(open, Queue::UserBackground, |_| {
            jobs::Job::new("folder", description, move |context| {
                #[cfg(test)]
                if let Some(hold) = &hold {
                    hold.wait();
                }
                let tell = |event| outbox.send(Event::Library(event));
                library::open(open, &source, &tell, &|| context.cancelled());
                Ok(())
            })
            .progress(message, true)
        });
        self.folder = Some(job);
    }

    /// [`LibraryCommand::Thumbnails`]: each of the folder's `photos` a job on darktable's
    /// `SystemForeground` queue, as darktable's thumbnail loads (decision 57), with no
    /// progress, deduplicated by the open, the photo and the size; added from the last, so
    /// the first, on the stack's top, is read first. Its picture goes out as
    /// [`Event::Library`]. Nothing with no folder open.
    fn thumbnails(&mut self, photos: Vec<String>, size: u32) {
        let Some((open, folder)) = self.shown.clone() else {
            return;
        };
        // Those that have ended are forgotten, so the list holds what a cancel can reach.
        let control = &self.jobs.control;
        self.thumbnails.retain(|&job| control.state(job).is_some());
        #[cfg(test)]
        let hold = self.thumbnails_hold.take();
        for photo in photos.into_iter().rev() {
            let key = format!("{open}/{size}/{photo}").into_bytes();
            let description = format!("thumbnail of {photo} at {size}");
            let (outbox, folder) = (self.jobs.outbox.clone(), folder.clone());
            #[cfg(test)]
            let hold = hold.clone();
            let job = self.jobs.add(self.applied, Queue::SystemForeground, |_| {
                jobs::Job::new("thumbnail", description, move |context| {
                    #[cfg(test)]
                    if let Some(hold) = &hold {
                        hold.wait();
                    }
                    let tell = |event| outbox.send(Event::Library(event));
                    library::thumbnail(open, &folder, &photo, size, &tell, &|| context.cancelled());
                    Ok(())
                })
                .dedup(key)
            });
            self.thumbnails.push(job);
        }
    }

    /// Renders the photo at the viewport's size and publishes it, once both are known. A
    /// module whose binding fails is left out of it and reported in `events`, once until
    /// it binds again.
    fn render(
        &mut self,
        since: Instant,
        events: &mut Vec<Event>,
    ) -> Result<Option<Frame>, SessionError> {
        let (Some(open), Some((width, height))) = (&mut self.open, self.size) else {
            return Ok(None);
        };
        let pipe = Pipe::Viewport { width, height };
        let bound = edit::instances(&open.edit, self.operations, &self.modules, &open.image);
        let failing: BTreeSet<&'static str> = bound.failed.iter().map(|(id, _)| *id).collect();
        for (module, source) in bound.failed {
            if !open.failing.contains(module) {
                tracing::error!(target: "pipe", "module {module} left out: {source}");
                events.push(Event::Failed(SessionError::Module { module, source }));
            }
        }
        open.failing = failing;
        let plan = pipeline::plan(
            &bound.instances,
            &open.image,
            open.sensor,
            pipe,
            &self.limits,
        )?;
        let pipeline = match open.pipeline.take() {
            Some(pipeline) => pollster::block_on(pipeline.update(&self.gpu, plan))?,
            None => pollster::block_on(Pipeline::new(&self.gpu, plan, open.input.clone()))?,
        };
        let pipeline = open.pipeline.insert(pipeline);
        let output = pipeline.output();
        let desc = output.desc();
        let bytes = desc.buffer_size();

        // Into the display buffer the client was not sent last, copied in the render's
        // submission, which is the frame's one turn at the submit gate.
        let display = match self.display.take() {
            Some(display) if display.size == (desc.width, desc.height) => display,
            old => {
                drop(old);
                let buffer = |label| {
                    self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(label),
                        size: bytes,
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::COPY_DST
                            | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    })
                };
                Display {
                    buffers: [buffer("display 0"), buffer("display 1")],
                    size: (desc.width, desc.height),
                    published: 1,
                    through: 0,
                }
            }
        };
        let display = self.display.insert(display);
        let next = 1 - display.published;
        let timings = pipeline.render_then(&self.gpu, |encoder| {
            encoder.copy_buffer_to_buffer(output.buffer(), 0, &display.buffers[next], 0, bytes);
        })?;
        display.published = next;
        display.through = self.applied;
        let buffer = display.buffers[next].clone();
        // Counted after the copy, if a client asks; the reading comes later.
        self.count(&buffer, [desc.width, desc.height], self.applied, events);
        Ok(Some(Frame {
            buffer,
            width: desc.width,
            height: desc.height,
            through: self.applied,
            timings,
            latency_ms: since.elapsed().as_secs_f64() * 1e3,
        }))
    }
}

impl Open {
    /// Follows the gesture under way through `command`, a change to the edit about to
    /// apply (decision 42): a `SetControl { done: false }` goes on with the gesture on
    /// its module, or begins one there from the edit and the held coordinates as they
    /// stand; any other change ends it as completed. A cancel is left to [`Self::cancel`].
    fn gesture_through(&mut self, command: &Command) {
        match command {
            Command::SetControl {
                module,
                done: false,
                ..
            } => {
                if self.gesture.as_ref().is_none_or(|g| g.module != *module) {
                    self.gesture = Some(Gesture {
                        module: module.clone(),
                        edit: self.edit.clone(),
                        held: self.held.clone(),
                        unsaved: self.unsaved,
                    });
                }
            }
            Command::CancelGesture { .. } | Command::RetrySave => {}
            _ => self.gesture = None,
        }
    }

    /// Puts the edit, the held coordinates and whether the writer has that edit back as
    /// they stood before the gesture under way on `module`, and says whether the edit
    /// changed; `None`, changing nothing, where no gesture is under way on it.
    fn cancel(&mut self, module: &str) -> Option<bool> {
        let gesture = self.gesture.take_if(|gesture| gesture.module == module)?;
        let changed = self.edit != gesture.edit;
        (self.edit, self.held, self.unsaved) = (gesture.edit, gesture.held, gesture.unsaved);
        Some(changed)
    }

    fn values(
        &mut self,
        operations: &[&dyn Operation],
        modules: &[&dyn Module],
        through: u64,
    ) -> Values {
        let mut edited = false;
        // On a photo open for viewing only, no control can change anything.
        let editable = self.view_only.is_none();
        let modules_state = modules
            .iter()
            .map(|module| {
                let definition = module.definition();
                let set = self.edit.modules.get(definition.id);
                // Every control's value: the edit's over its default for the photo.
                let over = |defaults: &Controls| {
                    let mut values = Controls::new();
                    for control in definition.controls {
                        let value = set
                            .and_then(|set| set.get(control.id))
                            .or_else(|| defaults.get(control.id))
                            .cloned()
                            .unwrap_or_else(|| control.default());
                        values.set(control.id, value);
                    }
                    values
                };
                // A module's code may panic, as its binding is guarded against. Should
                // its defaults panic, its controls show the definition's, live; the
                // render, which asks for them too, leaves the module out and reports it
                // while the edit holds values of it.
                let defaults = catch_unwind(AssertUnwindSafe(|| module.defaults(&self.image)))
                    .unwrap_or_else(|_| definition.defaults());
                let values = over(&defaults);
                // A control not ported can change nothing on any photo; the others, as
                // the module says while the controls hold what they hold now, so a
                // control another's value disables follows it (decision 30).
                let ported = || definition.controls.iter().map(|control| !control.unported);
                let available = catch_unwind(AssertUnwindSafe(|| {
                    let available = definition.controls.iter().zip(ported()).map(
                        |(control, ported)| {
                            ported && module.available(control.id, &values, &self.image)
                        },
                    );
                    available.collect::<Vec<bool>>()
                }));
                // Should it panic, which a change can reach, the photo's defaults stand,
                // as the render, which never asks, binds over them, and every control
                // ported shows live; a warning says so once for the photo.
                let available = available.unwrap_or_else(|panic| {
                    let id = definition.id;
                    if self.available_failed.insert(id) {
                        let message = panic_message(panic.as_ref());
                        tracing::warn!(target: "pipe", "module {id}'s controls all live: {message}");
                    }
                    ported().collect()
                });
                let controls: Vec<ControlState> = definition
                    .controls
                    .iter()
                    .zip(available)
                    .map(|(control, available)| {
                        let default = defaults
                            .get(control.id)
                            .cloned()
                            .unwrap_or_else(|| control.default());
                        let value = values
                            .get(control.id)
                            .cloned()
                            .unwrap_or_else(|| default.clone());
                        let options = self
                            .options
                            .get(&(definition.id, control.id))
                            .cloned()
                            .unwrap_or_default();
                        // A choice whose photo lists only the option it holds can change
                        // nothing there (decision 30).
                        let alone = check::lists_only(control, &options, &value);
                        // A number's range for the values reported (decision 30).
                        let range = edit::range(*module, control.id, &values).map(
                            |(range, failed)| {
                                let id = definition.id;
                                if let Some(message) = failed
                                    && self.ranges_failed.insert(id)
                                {
                                    tracing::warn!(target: "pipe", "module {id}'s ranges are its definition's: {message}");
                                }
                                range
                            },
                        );
                        ControlState {
                            control: control.id,
                            value,
                            default,
                            mark: set.and_then(|set| set.mark(control.id)),
                            available: available && editable && !alone,
                            options,
                            range,
                        }
                    })
                    .collect();
                let module_edited = controls.iter().any(|c| c.value != c.default);
                edited |= module_edited;
                let presets = self.presets(*module, &controls);
                let views = self.views(*module, &controls, &values);
                ModuleState {
                    module: definition.id,
                    edited: module_edited,
                    controls,
                    presets,
                    views,
                }
            })
            .collect();
        let base = Base::new(operations, &self.image);
        let mut settings = Vec::new();
        for operation in operations {
            let manifest = operation.manifest();
            let (Some(defaults), true) = (base.params(manifest.id), manifest.default_enabled)
            else {
                continue;
            };
            let empty = ParamValues::new();
            let set = self.edit.params.get(manifest.id).unwrap_or(&empty);
            // Every value was checked when it was set or read.
            let Ok(resolved) = operation.schema().resolve_over(defaults, set) else {
                continue;
            };
            for param in operation.schema().params {
                if check::owner(modules, manifest.id, param.name).is_some() {
                    continue;
                }
                if let Some(value) = resolved.get(param.name) {
                    edited |= defaults.get(param.name) != Some(value);
                    settings.push(Setting::new(manifest.id, param.name, value));
                }
            }
        }
        // The edit's instances, every module's that binds, as the render plans them.
        let instances = edit::instances(&self.edit, operations, modules, &self.image).instances;
        let extent = pipeline::extent(&instances, &self.image)
            .unwrap_or([self.image.width, self.image.height]);
        Values {
            modules: modules_state,
            settings,
            edited,
            through,
            extent,
        }
    }

    /// Each of the module's preset rows while its controls hold `controls`: the option
    /// they match and the options the photo can have, none while it is open for viewing
    /// only. Should the module's code panic, every row reads Custom, with no option to
    /// apply, and a warning says so once for the photo.
    fn presets(&mut self, module: &dyn Module, controls: &[ControlState]) -> Vec<PresetState> {
        let rows = module.definition().presets;
        let editable = self.view_only.is_none();
        let mut values = Controls::new();
        for control in controls {
            values.set(control.control, control.value.clone());
        }
        let states = catch_unwind(AssertUnwindSafe(|| {
            rows.iter()
                .map(|row| PresetState {
                    preset: row.id,
                    matching: module.matching(row.id, &values, &self.image),
                    available: row
                        .options
                        .iter()
                        .copied()
                        .filter(|option| {
                            editable && module.preset(row.id, option, &self.image).is_some()
                        })
                        .collect(),
                })
                .collect()
        }));
        match states {
            Ok(states) => states,
            Err(panic) => {
                let id = module.definition().id;
                // The values are reported after every change: once is enough.
                if self.presets_failed.insert(id) {
                    let message = panic_message(panic.as_ref());
                    tracing::warn!(target: "pipe", "module {id}'s presets read Custom: {message}");
                }
                rows.iter()
                    .map(|row| PresetState {
                        preset: row.id,
                        matching: None,
                        available: Vec::new(),
                    })
                    .collect()
            }
        }
    }

    /// Every coordinate of the module's lossless views: the held coordinates through a
    /// gesture, else the module's view of the values, each available while a control of
    /// `controls` it moves is and the module's `available` holds for it on `values`. A view
    /// the module fails to give, by a panic or with no coordinates, is left out, and a
    /// warning says so once for the photo.
    fn views(
        &mut self,
        module: &dyn Module,
        controls: &[ControlState],
        values: &Controls,
    ) -> Vec<CoordinateState> {
        let id = module.definition().id;
        let mut states = Vec::new();
        for view in module.definition().views {
            let image = &self.image;
            let viewed = self
                .edit
                .coordinates(&self.held, module, view, image)
                .and_then(|values| {
                    let defaults = edit::default_coordinates(module, view, image)?;
                    Ok((values, defaults))
                });
            let (viewed, defaults) = match viewed {
                Ok(viewed) => viewed,
                Err(error) => {
                    // The values are reported after every change: once is enough.
                    if self.views_failed.insert(id) {
                        tracing::warn!(target: "pipe", "module {id}'s views left out: {error}");
                    }
                    continue;
                }
            };
            let set = self.edit.module(id);
            let marks: Vec<Option<Mark>> = view
                .controls
                .iter()
                .map(|control| set.and_then(|values| values.mark(control)))
                .collect();
            let first = marks.first().copied().flatten();
            let mark = first.filter(|_| marks.iter().all(|m| *m == first));
            let moves = controls
                .iter()
                .any(|shown| shown.available && view.controls.contains(&shown.control));
            let coordinates = view.coordinates.iter().zip(viewed).zip(defaults);
            for ((coordinate, value), default) in coordinates {
                // A module's code may panic, as its binding is guarded against: the
                // coordinate then follows its controls.
                let own = catch_unwind(AssertUnwindSafe(|| {
                    module.available(coordinate.id, values, image)
                }))
                .unwrap_or(true);
                states.push(CoordinateState {
                    view: view.id,
                    coordinate: coordinate.id,
                    value,
                    default,
                    mark,
                    available: moves && own,
                });
            }
        }
        states
    }
}

/// How long a test waits for any one thing a session does: on WARP, which compiles every
/// kernel through FXC on a CPU the tests running beside it share, one step can take
/// minutes.
#[cfg(test)]
pub(crate) const TEST_WAIT: Duration = Duration::from_secs(600);

#[cfg(test)]
mod tests {
    use std::sync::mpsc::Receiver;

    use apertrust_modules::testing::{self, Defaultless, Faulty, Pushing, Sample};
    use apertrust_modules::{
        ChoiceStyle, Control, ControlError, ControlKind, Coordinate, Definition, ModuleValues,
        Owned, PresetRow, Row, Slider, Tab, View,
    };
    use apertrust_operations::OPERATIONS;

    use super::*;
    use crate::compute::read_buffer;
    use crate::gpu::GpuOptions;
    use crate::jobs::testing::{Fault, Synthetic, Told};
    use crate::jobs::{Destination, Message, Offer, State, TaskError};
    use crate::pipeline::Instance;

    fn gpu() -> GpuContext {
        pollster::block_on(GpuContext::new(&GpuOptions::default()))
            .expect("a GPU adapter, hardware or software")
    }

    /// The build the tests' sessions record in the edits they write.
    const BUILD: &str = "0.4.0+12~g1a2b3c4";

    /// Set once the engine has begun to open `slow.ARW`.
    static SLOW_OPEN: AtomicBool = AtomicBool::new(false);

    /// `testing::image`'s photo, whatever the path; one called `slow.ARW` takes a third of
    /// a second to arrive, so commands queue behind it, and one called `bias.ARW` records
    /// an exposure bias of −0.7 EV.
    fn synthetic(gpu: &GpuContext, path: &Path) -> Result<Decoded, SessionError> {
        if path.ends_with("slow.ARW") {
            SLOW_OPEN.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(300));
        }
        let mut image = testing::image();
        if let (true, Some(camera)) = (path.ends_with("bias.ARW"), &mut image.camera) {
            camera.exposure_bias = -0.7;
        }
        // A photo taken upright, its EXIF orientation rotate 270° clockwise, darktable's
        // rotate 90° anticlockwise.
        if let (true, Some(camera)) = (path.ends_with("portrait.ARW"), &mut image.camera) {
            camera.orientation = 6;
        }
        Ok(Decoded {
            camera: "Synthetic".to_owned(),
            image,
            input: GpuImage::upload_mosaic(gpu, 96, 72, &testing::mosaic()).expect("it uploads"),
            shot: Shot::default(),
            decode_ms: 0.0,
            upload_ms: 0.0,
        })
    }

    fn start_with(
        gpu: &GpuContext,
        modules: &'static [&'static dyn Module],
        load: Load,
    ) -> (Session, Receiver<()>) {
        let (woken, wakes) = mpsc::channel();
        let wake = move || {
            let _ = woken.send(());
        };
        let session = Session::spawn(
            gpu.clone(),
            OPERATIONS,
            modules,
            BUILD,
            pipeline::BUDGET,
            Arc::new(wake),
            load,
        );
        (session, wakes)
    }

    fn start(gpu: &GpuContext, modules: &'static [&'static dyn Module]) -> (Session, Receiver<()>) {
        start_with(gpu, modules, decode)
    }

    /// The events up to the first that `last` accepts, waiting at most [`TEST_WAIT`] for
    /// each wake.
    fn until(session: &Session, wakes: &Receiver<()>, last: impl Fn(&Event) -> bool) -> Vec<Event> {
        let mut events = Vec::new();
        loop {
            wakes
                .recv_timeout(TEST_WAIT)
                .expect("the session answers in time");
            events.extend(session.events());
            if events.iter().any(&last) {
                return events;
            }
        }
    }

    /// The synthetic photo with the shooting data a decode gives it.
    fn synthetic_shot(gpu: &GpuContext, path: &Path) -> Result<Decoded, SessionError> {
        let mut decoded = synthetic(gpu, path)?;
        decoded.shot = decoded.shot.with_exif(&apertrust_raw::Exif {
            exposure_time: Some(1.0 / 250.0),
            aperture: Some(5.6),
            focal_length: Some(35.0),
            ..Default::default()
        });
        Ok(decoded)
    }

    /// A client shows the exposure time, the f-number and the focal length in the photo's
    /// caption, and has them from the photo the session reports, not from the file.
    #[test]
    fn an_opened_photo_carries_the_shooting_data_its_decode_read() {
        let gpu = gpu();
        let (session, wakes) = start_with(&gpu, &[], synthetic_shot);
        session.send(Command::Open(PathBuf::from("photo.ARW")));
        let events = until(&session, &wakes, |e| matches!(e, Event::Opened(_)));
        let Some(Event::Opened(photo)) = events.iter().find(|e| matches!(e, Event::Opened(_)))
        else {
            panic!("{events:?}");
        };
        assert_eq!(
            (photo.exposure_time, photo.aperture, photo.focal_length),
            (Some(1.0 / 250.0), Some(5.6), Some(35.0))
        );
        // A photo whose EXIF could not be read carries none.
        let (session, wakes) = start_with(&gpu, &[], synthetic);
        session.send(Command::Open(PathBuf::from("photo.ARW")));
        let events = until(&session, &wakes, |e| matches!(e, Event::Opened(_)));
        let Some(Event::Opened(photo)) = events.iter().find(|e| matches!(e, Event::Opened(_)))
        else {
            panic!("{events:?}");
        };
        assert_eq!(
            (photo.exposure_time, photo.aperture, photo.focal_length),
            (None, None, None)
        );
    }

    /// The real decode reads them from the file's EXIF.
    #[test]
    #[ignore = "needs the raw.pixls.us corpus: tools/fetch-raw-corpus.sh"]
    fn a_decoded_raw_carries_its_exif_shooting_data() {
        let gpu = gpu();
        let decoded = decode(&gpu, &corpus_raw()).expect("the raw decodes");
        let shot = decoded.shot;
        assert!(shot.exposure_time.is_some_and(|t| t > 0.0), "{shot:?}");
        assert!(shot.aperture.is_some_and(|f| f > 0.0), "{shot:?}");
        assert!(shot.focal_length.is_some_and(|f| f > 0.0), "{shot:?}");
    }

    #[test]
    fn a_frame_has_the_size_its_photo_gives_for_the_bounds_it_was_rendered_for() {
        let gpu = gpu();
        // A photo the right way up, then one its orientation turns upright.
        for (file, extent, small, large) in [
            ("photo.ARW", [84, 62], (40, 30), (84, 62)),
            ("portrait.ARW", [62, 84], (30, 40), (62, 84)),
        ] {
            let (session, wakes) = start_with(&gpu, &[], synthetic);
            session.send(Command::Open(PathBuf::from(file)));
            let mut values = None;
            // Smaller than the photo, then larger.
            for bounds in [(40, 40), (300, 200)] {
                let sent = session.send(Command::Resize {
                    width: bounds.0,
                    height: bounds.1,
                });
                let events = until(
                    &session,
                    &wakes,
                    |e| matches!(e, Event::Frame(frame) if frame.through >= sent),
                );
                values = events
                    .iter()
                    .rev()
                    .find_map(|e| match e {
                        Event::Values(values) => Some(values.clone()),
                        _ => None,
                    })
                    .or(values);
                let values = values.as_ref().expect("the photo's values");
                let frame = events
                    .iter()
                    .rev()
                    .find_map(|e| match e {
                        Event::Frame(frame) => Some(frame),
                        _ => None,
                    })
                    .expect("a frame");
                assert_eq!((frame.width, frame.height), values.viewport_size(bounds));
            }
            let values = values.expect("the photo's values");
            assert_eq!(values.extent, extent, "{file}");
            assert_eq!(values.viewport_size((40, 40)), small, "{file}");
            assert_eq!(values.viewport_size((300, 200)), large, "{file}");
        }
    }

    /// A photo whose EXIF orientation turns it shows upright: at full size, its frame is
    /// the frame of the same photo the right way up, turned 90° anticlockwise, pixel
    /// for pixel, as darktable's flip turns its buffer.
    #[test]
    fn a_portrait_photo_shows_upright() {
        let gpu = gpu();
        let frame_of = |file: &str| {
            let (session, wakes) = start_with(&gpu, &[], synthetic);
            session.send(Command::Open(PathBuf::from(file)));
            let sent = session.send(Command::Resize {
                width: 300,
                height: 300,
            });
            let events = until(
                &session,
                &wakes,
                |e| matches!(e, Event::Frame(frame) if frame.through >= sent),
            );
            let frame = events
                .iter()
                .rev()
                .find_map(|e| match e {
                    Event::Frame(frame) => Some(frame),
                    _ => None,
                })
                .expect("a frame");
            let bytes = read_buffer(&gpu, &frame.buffer, frame.buffer.size()).expect("it reads");
            let pixels: Vec<[f32; 4]> = bytemuck::pod_collect_to_vec(&bytes);
            ((frame.width, frame.height), pixels)
        };
        let ((width, height), level) = frame_of("photo.ARW");
        let (turned_size, turned) = frame_of("portrait.ARW");
        assert_eq!((width, height), (84, 62));
        assert_eq!(turned_size, (height, width));
        let pixel = |frame: &[[f32; 4]], width: u32, x: u32, y: u32| {
            frame[(y * width + x) as usize].map(f32::to_bits)
        };
        // Anticlockwise: input pixel (x, y) lands at (y, width - 1 - x).
        for y in 0..height {
            for x in 0..width {
                assert_eq!(
                    pixel(&turned, height, y, width - 1 - x),
                    pixel(&level, width, x, y),
                    "pixel ({x}, {y})"
                );
            }
        }
    }

    /// A window's toolkit holds the submit gate in its callbacks, inside which the app
    /// drops its session, while the engine may be blocked on the gate in a submission.
    #[test]
    fn a_session_ends_while_the_thread_dropping_it_holds_the_submit_gate() {
        let gpu = gpu();
        let (session, _wakes) = start_with(&gpu, &[], synthetic);
        let (ended, done) = mpsc::channel();
        let gate = gpu.gate.clone();
        std::thread::spawn(move || {
            let _held = gate.hold();
            session.send(Command::Resize {
                width: 64,
                height: 48,
            });
            session.send(Command::Open(PathBuf::from("photo.ARW")));
            // Long enough for the engine to reach a submission and wait at the gate.
            std::thread::sleep(Duration::from_millis(500));
            drop(session);
            let _ = ended.send(());
        });
        done.recv_timeout(TEST_WAIT)
            .expect("the session ends with the gate held by the thread dropping it");
    }

    /// While a window resizes, its toolkit holds the submit gate in callback after
    /// callback with no pause between them. A frame takes one turn at the gate, so it
    /// waits for the callback under way and comes before the next one ends. The test's
    /// thread is the toolkit, holding the gate from one change to the next.
    #[test]
    fn a_frame_waits_for_one_hold_of_the_submit_gate_at_most_while_it_is_held_again_and_again() {
        let gpu = gpu();
        let path = scratch("gate");
        let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
        session.send(Command::Resize {
            width: 64,
            height: 48,
        });
        session.send(Command::Open(path.clone()));
        let frame = |sent: u64| move |e: &Event| matches!(e, Event::Frame(f) if f.through >= sent);
        // The first frame opens the photo and compiles the kernels, the gate free.
        let sent = session.send(set("brightness", ControlValue::Number(0.5), false));
        until(&session, &wakes, frame(sent));
        let mut held = gpu.gate.hold();
        for value in [-0.5, 1.0, -1.0, 0.25, -0.25, 0.75, -0.75] {
            let sent = session.send(set("brightness", ControlValue::Number(value), false));
            // The callback under way, and the frame's turn: the engine waits at the gate.
            gpu.gate.until_asked(2);
            assert!(
                !session.events().iter().any(frame(sent)),
                "a frame came while the gate was held"
            );
            // The next callback asks at once, after the frame, and holds the gate until
            // the frame comes: a frame of two turns, or a gate that let the toolkit in
            // first, would never come.
            drop(held);
            held = gpu.gate.hold();
            until(&session, &wakes, frame(sent));
        }
        drop(held);
        drop(session);
        remove(&path);
    }

    #[test]
    fn a_session_without_a_photo_renders_nothing_and_reports_what_fails() {
        let gpu = gpu();
        let (session, wakes) = start(&gpu, &[&Sample]);
        session.send(Command::Resize {
            width: 640,
            height: 480,
        });
        session.send(Command::SetControl {
            module: "sample".to_owned(),
            control: "brightness".to_owned(),
            value: ControlValue::Number(1.0),
            done: true,
        });
        let sent = session.send(Command::Open(PathBuf::from("no such raw.CR2")));
        let events = until(&session, &wakes, |e| matches!(e, Event::Failed(_)));
        assert!(
            matches!(&events[..], [Event::Failed(SessionError::Open { open, error })]
                if *open == sent && matches!(**error, SessionError::Decode(_))),
            "{events:?}"
        );
    }

    #[test]
    fn each_photo_opened_carries_the_number_its_open_was_sent_with() {
        let gpu = gpu();
        let (session, wakes) = start_with(&gpu, &[], synthetic);
        let first = session.send(Command::Open(PathBuf::from("first.ARW")));
        let second = session.send(Command::Open(PathBuf::from("second.ARW")));
        let events = until(
            &session,
            &wakes,
            |e| matches!(e, Event::Opened(photo) if photo.open == second),
        );
        let opened: Vec<(u64, &Path)> = events
            .iter()
            .filter_map(|e| match e {
                Event::Opened(photo) => Some((photo.open, photo.path.as_path())),
                _ => None,
            })
            .collect();
        let expected = [(first, "first.ARW"), (second, "second.ARW")];
        assert_eq!(opened, expected.map(|(n, p)| (n, Path::new(p))));
    }

    /// Owns exposure's exposure too, which [`Sample`] has.
    struct Rival;

    static RIVAL: Definition = Definition {
        id: "rival",
        version: 1,
        layers: true,
        auto: false,
        controls: &[Control {
            id: "amount",
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        }],
        tabs: &[Tab {
            id: None,
            rows: &[Row::Slider(Slider::new("amount"))],
        }],
        before: &[],
        after: &[],
        presets: &[],
        views: &[],
    };

    static RIVAL_OWNS: [Owned; 1] = [Owned::new("exposure", "exposure")];

    impl Module for Rival {
        fn definition(&self) -> &'static Definition {
            &RIVAL
        }

        fn owns(&self) -> &'static [Owned] {
            &RIVAL_OWNS
        }

        fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
            Vec::new()
        }
    }

    #[test]
    fn a_faulty_module_is_reported_and_left_out() {
        let gpu = gpu();
        let (session, wakes) = start(&gpu, &[&Sample, &Rival]);
        let events = until(&session, &wakes, |e| matches!(e, Event::Failed(_)));
        assert!(
            matches!(
                events[..],
                [Event::Failed(SessionError::Module {
                    module: "rival",
                    source: ModuleError::OwnedTwice {
                        owner: "sample",
                        ..
                    }
                })]
            ),
            "{events:?}"
        );

        // Left out: its commands are refused, and the values have no place for it.
        let mut driven = Driven::new(&[&Sample, &Rival]);
        driven.open(&scratch("rival"));
        let events = driven.run([Command::SetControl {
            module: "rival".to_owned(),
            control: "amount".to_owned(),
            value: ControlValue::Number(0.5),
            done: true,
        }]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::UnknownModule(_))]
            ),
            "{events:?}"
        );
        let modules: Vec<&str> = values(&events).modules.iter().map(|m| m.module).collect();
        assert_eq!(modules, ["sample"]);
    }

    /// A session's engine, driven directly, its photos the synthetic one, with the
    /// writer's events in `saved`.
    struct Driven {
        gpu: GpuContext,
        engine: Engine,
        saved: Drain,
        /// The engine thread's inbox, which its jobs hand their tasks to.
        inbox: Receiver<Inbox>,
    }

    impl Driven {
        fn new(modules: &'static [&'static dyn Module]) -> Self {
            Driven::within(modules, pipeline::BUDGET)
        }

        /// An engine that plans within `budget` bytes.
        fn within(modules: &'static [&'static dyn Module], budget: u64) -> Self {
            let gpu = gpu();
            let (sent, saved) = outbox::channel(Arc::new(|| {}));
            let (tasks, inbox) = mpsc::channel();
            let (mut engine, _) = Engine::new(
                gpu.clone(),
                OPERATIONS,
                modules,
                BUILD.to_owned(),
                budget,
                Writer::start(sent.clone()),
                Jobs::start(sent, tasks),
            );
            engine.load = synthetic;
            Driven {
                gpu,
                engine,
                saved,
                inbox,
            }
        }

        /// Opens the synthetic photo as `path`, as a client does, and sizes the viewport.
        fn open(&mut self, path: &Path) -> Vec<Event> {
            let events = self.run([
                Command::Open(path.to_owned()),
                Command::Resize {
                    width: 40,
                    height: 40,
                },
            ]);
            assert!(
                matches!(events.first(), Some(Event::Opened(photo)) if (photo.width, photo.height) == (84, 62)),
                "{events:?}"
            );
            events
        }

        fn run(&mut self, commands: impl IntoIterator<Item = Command>) -> Vec<Event> {
            let batch = commands.into_iter().map(|c| (Instant::now(), c)).collect();
            self.engine.apply(batch)
        }

        /// What the writer has reported since, once it is done.
        fn writes(&self) -> Vec<Event> {
            self.engine.writer.flush(None);
            self.saved.events()
        }
    }

    fn set(control: &str, value: ControlValue, done: bool) -> Command {
        Command::SetControl {
            module: "sample".to_owned(),
            control: control.to_owned(),
            value,
            done,
        }
    }

    fn reset(control: &str) -> Command {
        Command::ResetControl {
            module: "sample".to_owned(),
            control: control.to_owned(),
        }
    }

    fn frame(gpu: &GpuContext, events: &[Event]) -> Vec<u8> {
        let frame = events
            .iter()
            .find_map(|e| match e {
                Event::Frame(frame) => Some(frame),
                _ => None,
            })
            .unwrap_or_else(|| panic!("a frame in {events:?}"));
        read_buffer(gpu, &frame.buffer, frame.buffer.size()).expect("it reads back")
    }

    fn values(events: &[Event]) -> &Values {
        events
            .iter()
            .rev()
            .find_map(|e| match e {
                Event::Values(values) => Some(values),
                _ => None,
            })
            .unwrap_or_else(|| panic!("values in {events:?}"))
    }

    fn failures(events: &[Event]) -> Vec<&SessionError> {
        events
            .iter()
            .filter_map(|e| match e {
                // A refused command's error, as the command was refused.
                Event::Failed(SessionError::Refused { error, .. }) => Some(&**error),
                Event::Failed(error) => Some(error),
                _ => None,
            })
            .collect()
    }

    /// The files each write in `writes` saved, or failed to.
    fn written(writes: &[Event]) -> Vec<(&Path, bool)> {
        writes
            .iter()
            .filter_map(|e| match e {
                Event::Saved { path, .. } => Some((path.as_path(), true)),
                Event::Failed(SessionError::Save { path, .. }) => Some((path.as_path(), false)),
                _ => None,
            })
            .collect()
    }

    fn control<'v>(values: &'v Values, module: &str, control: &str) -> &'v ControlState {
        values
            .modules
            .iter()
            .find(|m| m.module == module)
            .and_then(|m| m.controls.iter().find(|c| c.control == control))
            .unwrap()
    }

    /// The viewport pipe's render of the synthetic photo through the operations an
    /// unedited photo runs, decision 22's eight and colour calibration (decision 27),
    /// named here rather than taken from an edit, and `extra`, with `settings`: as the
    /// engine rendered before modules.
    fn direct(gpu: &GpuContext, extra: &[&str], settings: &[(&str, &str, Value)]) -> Vec<u8> {
        let ids = [
            "rawprepare",
            "temperature",
            "highlights",
            "demosaic",
            "exposure",
            "colorin",
            "channelmixerrgb",
            "agx",
            "colorout",
        ];
        let instances: Vec<Instance> = ids
            .iter()
            .chain(extra)
            .map(|id| {
                let operation = apertrust_operations::operation(id).expect("an operation");
                let mut params = ParamValues::new();
                for &(op, name, value) in settings {
                    if op == *id {
                        params.set(name, value);
                    }
                }
                Instance { operation, params }
            })
            .collect();
        let input = GpuImage::upload_mosaic(gpu, 96, 72, &testing::mosaic()).unwrap();
        let pipe = Pipe::Viewport {
            width: 40,
            height: 40,
        };
        let plan = pipeline::plan(
            &instances,
            &testing::image(),
            input.desc(),
            pipe,
            &Limits::of(gpu),
        )
        .expect("a plan");
        let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, input)).expect("it builds");
        pipeline.render(gpu).expect("it renders");
        bytemuck::cast_slice(&pipeline.output().download(gpu).expect("it reads back")).to_vec()
    }

    /// A photo's path in a folder of the test's own, emptied first.
    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("apertrust-session-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        directory.join("synthetic.ARW")
    }

    /// Removes `path`'s folder, trying again for a while should a write still under way,
    /// the edit writer's temporary file beside the edit, land in it meanwhile.
    fn remove(path: &Path) {
        let folder = path.parent().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match std::fs::remove_dir_all(folder) {
                Ok(()) => return,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return,
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("removing {}: {error}", folder.display()),
            }
        }
    }

    thread_local! {
        /// How many times [`Lists`] has been asked for its `counted` list on this thread: a
        /// `Driven` engine runs on its test's, so each test counts only its own asks.
        static COUNTED_ASKED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    /// How many times [`Lists`] has been asked for its `counted` list on this thread.
    fn counted_asked() -> u64 {
        COUNTED_ASKED.with(std::cell::Cell::get)
    }

    /// A module with two choices for the session's lists (decision 30): `mode`, which
    /// every photo lists only at its default, one, as darktable lists a monochrome
    /// sensor's one demosaic method; and `counted`, whose list panics, counting the times
    /// it is asked. It owns and binds nothing.
    struct Lists;

    static LISTS: Definition = Definition {
        id: "lists",
        version: 1,
        layers: false,
        auto: false,
        controls: &[
            Control {
                id: "mode",
                kind: ControlKind::Choice {
                    options: &["one", "two"],
                    default: 0,
                    unported: &[],
                },
                unit: None,
                unported: false,
            },
            Control {
                id: "counted",
                kind: ControlKind::Choice {
                    options: &["a", "b"],
                    default: 0,
                    unported: &[],
                },
                unit: None,
                unported: false,
            },
        ],
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Choice {
                    control: "mode",
                    style: ChoiceStyle::List,
                },
                Row::Choice {
                    control: "counted",
                    style: ChoiceStyle::List,
                },
            ],
        }],
        before: &[],
        after: &[],
        presets: &[],
        views: &[],
    };

    impl Module for Lists {
        fn definition(&self) -> &'static Definition {
            &LISTS
        }

        fn owns(&self) -> &'static [Owned] {
            &[]
        }

        fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
            Vec::new()
        }

        fn options(&self, control: &str, _: &ImageContext) -> Vec<&'static str> {
            if control == "counted" {
                COUNTED_ASKED.with(|asked| asked.set(asked.get() + 1));
                panic!("no list for counted");
            }
            vec!["one"]
        }
    }

    fn method(option: u32) -> Command {
        set("method", ControlValue::Choice(option), true)
    }

    /// Sample's method lists camera only on a photo that records an exposure bias.
    const CAMERA: u32 = 3;

    #[test]
    fn a_choice_reports_the_options_its_photo_lists_and_refuses_any_other() {
        let path = scratch("listed");
        let mut driven = Driven::new(&[&Sample]);
        let events = driven.open(&path);
        let reported = values(&events);
        let method = control(reported, "sample", "method");
        assert_eq!(method.options, ["even", "biased", "film"]);
        assert!(method.available);
        assert_eq!(control(reported, "sample", "size").options.len(), 3);
        assert!(control(reported, "sample", "grain").options.is_empty());
        // Camera is refused, the edit as it was.
        let events = driven.run([self::method(CAMERA)]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::NotListed {
                    module: "sample",
                    control: "method",
                    option: "camera",
                })]
            ),
            "{events:?}"
        );
        assert!(driven.engine.open.as_ref().unwrap().edit.is_empty());
        // On a photo that records a bias, it is listed and taken.
        let biased = path.with_file_name("bias.ARW");
        let events = driven.open(&biased);
        let method = control(values(&events), "sample", "method");
        assert_eq!(method.options, ["even", "biased", "film", "camera"]);
        let events = driven.run([self::method(CAMERA)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let method = control(values(&events), "sample", "method");
        assert_eq!(method.value, ControlValue::Choice(CAMERA));
        driven.writes();
        remove(&path);
    }

    #[test]
    fn an_option_from_another_photo_s_edit_is_kept_rendered_and_saved_as_it_is() {
        let path = scratch("unlisted");
        let mut edit = Edit::default();
        let mut biased = testing::image();
        if let Some(camera) = &mut biased.camera {
            camera.exposure_bias = -0.7;
        }
        let camera = ControlValue::Choice(CAMERA);
        edit.set_control(&Sample, "method", camera.clone(), &biased)
            .unwrap();
        let xml = sidecar::to_xml(&edit, OPERATIONS, &[&Sample], BUILD);
        std::fs::write(sidecar::path_for(&path), &xml).unwrap();
        // Opened on a photo that records no bias, which does not list camera: kept, the
        // photo editable, its frame rendered with nothing failing.
        let mut driven = Driven::new(&[&Sample]);
        let events = driven.open(&path);
        assert!(
            matches!(events.first(), Some(Event::Opened(photo)) if photo.view_only.is_none()),
            "{events:?}"
        );
        assert!(failures(&events).is_empty(), "{events:?}");
        frame(&driven.gpu, &events);
        let method = control(values(&events), "sample", "method");
        assert_eq!(method.value, camera);
        assert_eq!(method.options, ["even", "biased", "film"]);
        assert!(method.available);
        // Saved as it is after another change.
        driven.run([set("grain", ControlValue::Number(20.0), true)]);
        driven.writes();
        let saved = std::fs::read_to_string(sidecar::path_for(&path)).unwrap();
        assert!(saved.contains(r#"control:method="camera""#), "{saved}");
        // Choosing another option leaves it, and it cannot come back.
        driven.run([self::method(0)]);
        let events = driven.run([self::method(CAMERA)]);
        assert_eq!(failures(&events).len(), 1, "{events:?}");
        driven.writes();
        remove(&path);
    }

    #[test]
    fn a_choice_whose_photo_lists_only_the_option_it_holds_is_unavailable() {
        let path = scratch("alone");
        let mut driven = Driven::new(&[&Sample, &Lists]);
        let events = driven.open(&path);
        let values = values(&events);
        let mode = control(values, "lists", "mode");
        assert_eq!(
            (mode.options.as_slice(), mode.available),
            (&["one"][..], false)
        );
        // A list with unported options beside the one held stays live, as Exposure's Mode.
        assert!(control(values, "sample", "method").available);
        let events = driven.run([Command::SetControl {
            module: "lists".to_owned(),
            control: "mode".to_owned(),
            value: ControlValue::Choice(1),
            done: true,
        }]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::NotListed {
                    option: "two",
                    ..
                })]
            ),
            "{events:?}"
        );
        remove(&path);
    }

    #[test]
    fn a_list_that_panics_lists_every_option_asked_once_as_the_photo_opens() {
        let path = scratch("panicking-list");
        let mut driven = Driven::new(&[&Sample, &Lists]);
        let before = counted_asked();
        let events = driven.open(&path);
        let counted = control(values(&events), "lists", "counted");
        assert_eq!(
            (counted.options.as_slice(), counted.available),
            (&["a", "b"][..], true)
        );
        assert_eq!(counted_asked(), before + 1);
        // Reported again after a change, it is not asked again.
        let events = driven.run([set("grain", ControlValue::Number(20.0), true)]);
        assert_eq!(
            control(values(&events), "lists", "counted").options,
            ["a", "b"]
        );
        assert_eq!(counted_asked(), before + 1);
        driven.writes();
        remove(&path);
    }

    #[test]
    fn a_control_that_cannot_act_on_the_photo_is_reported_unavailable() {
        use apertrust_modules::exposure::Exposure;
        let path = scratch("available");
        let mut driven = Driven::new(&[&Exposure]);
        let events = driven.open(&path);
        let values = values(&events);
        // The synthetic raw records no exposure bias, so there is none to undo.
        assert!(!control(values, "exposure", "camera_bias").available);
        assert!(control(values, "exposure", "exposure").available);
        assert!(control(values, "exposure", "black_level").available);
        // A control not ported can act on no photo.
        assert!(!control(values, "exposure", "compensate_hilite_pres").available);
        remove(&path);
    }

    /// Sample's coarseness acts only on coarse grain that is on, as darktable makes
    /// vignetting's width/height ratio insensitive while automatic ratio is on (decision
    /// 30): every kind of change moves it, a set, a reset, a preset's option, a view's
    /// coordinate and a held gesture, and a keep leaves it.
    #[test]
    fn a_control_another_s_value_disables_follows_it_after_each_change_keeping_its_value() {
        let path = scratch("follows");
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        assert!(!control(values(&opened), "sample", "coarseness").available);
        // Each change's report: coarseness's availability and value, and the pair's
        // difference, with every other control available.
        let mut run = |command: Command| {
            let events = driven.run([command]);
            assert!(failures(&events).is_empty(), "{events:?}");
            let values = values(&events);
            let sample = &values.modules[0];
            let mut others = sample.controls.iter().filter(|c| c.control != "coarseness");
            assert!(others.all(|c| c.available), "{sample:?}");
            let state = control(values, "sample", "coarseness");
            let ControlValue::Number(value) = state.value else {
                unreachable!()
            };
            (state.available, value, pair(values)[1].0)
        };
        let keep = |control: &str, kept| Command::Keep {
            module: "sample".to_owned(),
            control: control.to_owned(),
            kept,
        };
        let (number, coarse) = (ControlValue::Number, ControlValue::Choice(2));
        // Unavailable, it still takes a value, and keeps it throughout.
        assert_eq!(
            run(set("coarseness", number(150.0), true)),
            (false, 150.0, -100.0)
        );
        assert_eq!(run(set("grain", number(20.0), true)), (false, 150.0, -80.0));
        // A set of size to coarse makes it available; a keep leaves it so.
        assert_eq!(run(set("size", coarse.clone(), true)), (true, 150.0, -80.0));
        assert_eq!(run(keep("size", false)), (true, 150.0, -80.0));
        assert_eq!(run(keep("grain", true)), (true, 150.0, -80.0));
        // A reset of size to medium takes that back.
        assert_eq!(run(reset("size")), (false, 150.0, -80.0));
        // So do a preset's options: grainy writes coarse grain on, untouched medium and off.
        assert_eq!(run(apply("grainy")), (true, 150.0, -50.0));
        assert_eq!(run(apply("untouched")), (false, 150.0, -100.0));
        // Coarse but off: the pair's sum moves grain on, then off again.
        assert_eq!(run(set("size", coarse, true)), (false, 150.0, -100.0));
        assert_eq!(run(set("bias", number(50.0), true)), (false, 150.0, -50.0));
        assert_eq!(run(set("sum", number(70.0), true)), (true, 150.0, -50.0));
        // A drag of the sum, the difference held at −50 %, follows each step.
        assert_eq!(run(set("sum", number(50.0), false)), (false, 150.0, -50.0));
        assert_eq!(run(set("sum", number(90.0), false)), (true, 150.0, -50.0));
        assert_eq!(run(set("sum", number(50.0), true)), (false, 150.0, -50.0));
        remove(&path);
    }

    /// Sample's coarseness reaches up to 100 % plus the bias (decision 30): the session
    /// reports each number's range for the values it reports, refuses a value outside it,
    /// and clamps one a change leaves outside in that change, saved once with it; an edit
    /// read with one outside is clamped as the photo opens.
    #[test]
    fn a_range_that_follows_another_control_is_reported_and_its_clamp_is_one_change() {
        let path = scratch("ranges");
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        let range = |values: &Values| control(values, "sample", "coarseness").range.unwrap();
        assert_eq!(range(values(&opened)).max, 200.0);
        assert_eq!(control(values(&opened), "sample", "size").range, None);
        let events = driven.run([set("coarseness", ControlValue::Number(150.0), true)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        driven.writes();
        // A bias of 20 % moves the range to 50 to 120 %, and clamps coarseness with it.
        let events = driven.run([set("bias", ControlValue::Number(20.0), true)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let reported = values(&events);
        let coarseness = control(reported, "sample", "coarseness");
        assert_eq!(
            (coarseness.value.clone(), coarseness.mark),
            (ControlValue::Number(120.0), Some(Mark::Kept))
        );
        assert_eq!(
            (range(reported).max, range(reported).soft),
            (120.0, (50.0, 120.0))
        );
        assert_eq!(driven.writes().len(), 1, "the clamp saved with its change");
        // A value outside the range the values give now is refused.
        let events = driven.run([set("coarseness", ControlValue::Number(150.0), true)]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::Control {
                    source: ControlError::OutOfRange { .. },
                    ..
                })]
            ),
            "{events:?}"
        );
        remove(&path);

        // An edit read with a number outside its range is clamped as the photo opens.
        let path = scratch("ranges-read");
        let mut edit = Edit::default();
        let image = testing::image();
        edit.set_control(&Sample, "coarseness", ControlValue::Number(150.0), &image)
            .unwrap();
        edit.set_control(&Sample, "bias", ControlValue::Number(20.0), &image)
            .unwrap();
        let xml = sidecar::to_xml(&edit, OPERATIONS, &[&Sample], "test");
        std::fs::write(sidecar::path_for(&path), xml).unwrap();
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        let coarseness = control(values(&opened), "sample", "coarseness");
        assert_eq!(coarseness.value, ControlValue::Number(120.0));
        remove(&path);
    }

    /// Each change's clamp asks every module (decision 30) and passes over one whose
    /// defaults for the photo panic: another module's changes apply with their clamps, and
    /// the photo stays open and saves them.
    #[test]
    fn a_module_whose_defaults_panic_leaves_the_others_changes_to_apply() {
        let path = scratch("defaultless");
        let mut driven = Driven::new(&[&Defaultless, &Sample]);
        driven.open(&path);
        // A bias of 20 % clamps coarseness to 120 %.
        for (id, value, coarseness) in [("coarseness", 150.0, 150.0), ("bias", 20.0, 120.0)] {
            let events = driven.run([set(id, ControlValue::Number(value), true)]);
            assert!(failures(&events).is_empty(), "{id}: {events:?}");
            assert_eq!(
                control(values(&events), "sample", "coarseness").value,
                ControlValue::Number(coarseness)
            );
        }
        assert!(!driven.writes().is_empty());
        let mut again = Driven::new(&[&Defaultless, &Sample]);
        let opened = again.open(&path);
        let coarseness = control(values(&opened), "sample", "coarseness");
        assert_eq!(coarseness.value, ControlValue::Number(120.0));
        remove(&path);
    }

    /// A drag of a view's coordinate whose move of a range clamps another control
    /// (decision 30): each step clamps in the same change, and the coordinates it holds
    /// survive the clamp, the difference held exactly where the module's view of the
    /// values would round it.
    #[test]
    fn a_coordinate_s_drag_clamps_what_its_range_leaves_outside_holding_the_others() {
        let path = scratch("view-clamp");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        driven.run([set("coarseness", ControlValue::Number(180.0), true)]);
        // The difference up, the sum held at 100: a bias of 70.15 %, so coarseness reaches
        // only 170.15 % and is clamped there, kept.
        let events = driven.run([set("difference", ControlValue::Number(-40.3), false)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let [_, ControlValue::Number(bias)] = unviewed([100.0, -40.3]) else {
            unreachable!()
        };
        let coarseness = control(values(&events), "sample", "coarseness");
        assert_eq!(
            (coarseness.value.clone(), coarseness.mark),
            (ControlValue::Number(100.0 + bias), Some(Mark::Kept))
        );
        // Then the sum, the difference held through the clamp.
        let events = driven.run([set("sum", ControlValue::Number(103.1), true)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let shown = values(&events);
        assert_eq!(coordinate(shown, "difference").value, -40.3);
        assert_ne!(viewed(&grain_and_bias(shown))[1], -40.3);
        driven.writes();
        remove(&path);
    }

    /// A drag step whose clamp would rewrite what the step itself wrote is the module's
    /// failure (decision 30): [`Pushing`]'s bias writes a coarseness of 190 % as it
    /// moves, which a bias under 90 % leaves outside. The step is refused, the edit as the
    /// step before left it, and the gesture's held coordinates dropped: the coordinates are
    /// the module's view of the values again.
    #[test]
    fn a_drag_step_whose_clamp_is_refused_changes_nothing_and_ends_the_gesture() {
        let path = scratch("view-clamp-refused");
        let mut driven = Driven::new(&[&Pushing]);
        driven.open(&path);
        // A bias of 95.15 %, then 96.7 %, coarseness written at 190 %, inside; then
        // coarseness set to 150 %, which leaves the pair's coordinates held.
        driven.run([set("difference", ControlValue::Number(-90.3), false)]);
        driven.run([set("sum", ControlValue::Number(103.1), false)]);
        let events = driven.run([set("coarseness", ControlValue::Number(150.0), false)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let held = values(&events);
        assert_eq!(pair(held).map(|(value, ..)| value), [103.1, -90.3]);
        let stepped = grain_and_bias(held);
        // A bias of 86.7 % writes coarseness at 190 %, outside, and the clamp would
        // rewrite it.
        let events = driven.run([set("difference", ControlValue::Number(-70.3), false)]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::ClampFailed {
                    module: "sample",
                    control: "coarseness"
                })]
            ),
            "{events:?}"
        );
        let refused = values(&events);
        assert_eq!(grain_and_bias(refused), stepped);
        assert_eq!(
            control(refused, "sample", "coarseness").value,
            ControlValue::Number(150.0)
        );
        let shown = pair(refused).map(|(value, ..)| value);
        assert_eq!(shown[..], viewed(&stepped)[..]);
        assert_ne!(shown[0], 103.1);
        driven.writes();
        remove(&path);
    }

    /// The session plans each photo within the budget its client hands it, the settings'
    /// `engine.memory_budget`: one smaller than demosaic's full-resolution image tiles the
    /// viewport's demosaic, to the same frame (decision 14).
    #[test]
    fn a_session_plans_within_the_budget_it_is_given() {
        let path = scratch("budget");
        let mut driven = Driven::new(&[&Sample]);
        let events = driven.open(&path);
        let whole = frame(&driven.gpu, &events);
        let tiled = |driven: &Driven| {
            let open = driven.engine.open.as_ref().expect("a photo open");
            open.pipeline
                .as_ref()
                .expect("a pipeline")
                .plan()
                .tiled
                .is_some()
        };
        assert!(!tiled(&driven));
        // Demosaic's image of the 96×72 mosaic takes 108 KiB.
        let mut small = Driven::within(&[&Sample], 64 << 10);
        assert_eq!(small.engine.limits.budget, 64 << 10);
        let events = small.open(&path);
        assert!(tiled(&small));
        assert!(frame(&small.gpu, &events) == whole);
        remove(&path);
    }

    #[test]
    fn an_untouched_module_renders_what_the_engine_did_before_modules() {
        let path = scratch("untouched");
        let mut driven = Driven::new(&[&Sample]);
        let events = driven.open(&path);
        let untouched = frame(&driven.gpu, &events);
        assert!(untouched == direct(&driven.gpu, &[], &[]));

        // Set to their defaults and kept, the controls change nothing.
        let events = driven.run([
            set("brightness", ControlValue::Number(0.0), true),
            set("grain", ControlValue::Number(0.0), true),
            set("size", ControlValue::Choice(1), true),
        ]);
        assert!(frame(&driven.gpu, &events) == untouched);
        let values = values(&events);
        assert!(!values.edited && !values.modules[0].edited);
        let brightness = control(values, "sample", "brightness");
        assert_eq!(
            (&brightness.value, &brightness.default, brightness.mark),
            (
                &ControlValue::Number(0.0),
                &ControlValue::Number(0.0),
                Some(Mark::Kept)
            )
        );
        // Sample owns exposure's exposure, so it is no unowned setting; grain is optional.
        assert!(
            values
                .settings
                .iter()
                .any(|s| (s.operation, s.param) == ("exposure", "black"))
        );
        assert!(
            !values
                .settings
                .iter()
                .any(|s| (s.operation, s.param) == ("exposure", "exposure")
                    || s.operation == "grain")
        );
        remove(&path);
    }

    #[test]
    fn module_commands_render_the_parameters_their_binding_derives() {
        let path = scratch("derived");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let events = driven.run([set("brightness", ControlValue::Number(0.5), false)]);
        let exposure = ("exposure", "exposure", Value::Float(0.7 + 0.5));
        assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &[], &[exposure]));
        assert!(values(&events).edited);

        // Grain joins the plan once its control leaves 0, at its own pipeline position.
        let events = driven.run([
            set("grain", ControlValue::Number(40.0), false),
            set("size", ControlValue::Choice(2), true),
        ]);
        let scale = Base::new(OPERATIONS, &testing::image()).float("grain", "scale");
        let grain = [
            exposure,
            ("grain", "strength", Value::Float(40.0)),
            ("grain", "scale", Value::Float(2.0 * scale)),
        ];
        assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &["grain"], &grain));
        remove(&path);
    }

    #[test]
    fn a_control_moved_and_reset_renders_the_untouched_frame() {
        let path = scratch("reset");
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        let untouched = frame(&driven.gpu, &opened);
        let moved = driven.run([
            set("brightness", ControlValue::Number(-1.0), false),
            set("grain", ControlValue::Number(60.0), true),
        ]);
        assert!(frame(&driven.gpu, &moved) != untouched);
        let events = driven.run([reset("grain"), reset("brightness")]);
        assert!(frame(&driven.gpu, &events) == untouched);
        let values = values(&events);
        assert!(!values.edited);
        assert_eq!(control(values, "sample", "grain").mark, None);

        driven.run([set("grain", ControlValue::Number(60.0), true)]);
        let events = driven.run([Command::ResetModule {
            module: "sample".to_owned(),
        }]);
        assert!(frame(&driven.gpu, &events) == untouched);
        driven.run([set("brightness", ControlValue::Number(2.0), true)]);
        let events = driven.run([Command::Reset]);
        assert!(frame(&driven.gpu, &events) == untouched);
        remove(&path);
    }

    #[test]
    fn values_out_of_range_and_unknown_controls_are_refused() {
        let path = scratch("refused");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        driven.run([set("brightness", ControlValue::Number(1.0), true)]);
        let events = driven.run([
            set("brightness", ControlValue::Number(9.0), true),
            set("size", ControlValue::Number(1.0), true),
            set("method", ControlValue::Choice(2), true),
            set("contrast", ControlValue::Number(1.0), true),
            Command::Keep {
                module: "exposure".to_owned(),
                control: "exposure".to_owned(),
                kept: true,
            },
        ]);
        let failures = failures(&events);
        assert!(
            matches!(
                failures[..],
                [
                    SessionError::Edit(EditError::Control {
                        source: ControlError::OutOfRange { .. },
                        ..
                    }),
                    SessionError::Edit(EditError::Control {
                        source: ControlError::WrongKind { .. },
                        ..
                    }),
                    // An option darktable lists that the module does not port.
                    SessionError::Edit(EditError::Control {
                        source: ControlError::Unported { option: "film", .. },
                        ..
                    }),
                    SessionError::Edit(EditError::UnknownControl { .. }),
                    SessionError::Edit(EditError::UnknownModule(_)),
                ]
            ),
            "{failures:?}"
        );
        // The values that follow keep the old one.
        let values = values(&events);
        assert_eq!(
            control(values, "sample", "brightness").value,
            ControlValue::Number(1.0)
        );
        assert_eq!(values.through, driven.engine.applied);
        remove(&path);
    }

    /// Applies `option` of [`Sample`]'s look.
    fn apply(option: &str) -> Command {
        Command::ApplyPreset {
            module: "sample".to_owned(),
            preset: "look".to_owned(),
            option: option.to_owned(),
        }
    }

    /// The state of [`Sample`]'s look.
    fn look(values: &Values) -> &PresetState {
        values
            .modules
            .iter()
            .find(|m| m.module == "sample")
            .and_then(|m| m.presets.iter().find(|p| p.preset == "look"))
            .unwrap()
    }

    #[test]
    fn a_preset_s_option_sets_its_controls_as_one_change_each_kept_and_leaves_the_rest() {
        let path = scratch("preset");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        let untouched = frame(&driven.gpu, &opened);
        // The synthetic raw records no exposure bias, so it cannot have the camera's look.
        let state = look(values(&opened));
        assert_eq!(state.matching, Some("untouched"));
        assert_eq!(state.available, ["untouched", "grainy"]);

        // Controls the look does not write: one set, one released.
        driven.run([
            set("method", ControlValue::Choice(1), false),
            set("bias", ControlValue::Number(30.0), true),
            Command::Keep {
                module: "sample".to_owned(),
                control: "bias".to_owned(),
                kept: false,
            },
        ]);
        driven.writes();
        let events = driven.run([apply("grainy")]);
        let applied = values(&events);
        for (id, value) in [
            ("brightness", ControlValue::Number(0.0)),
            ("grain", ControlValue::Number(50.0)),
            ("size", ControlValue::Choice(2)),
        ] {
            let shown = control(applied, "sample", id);
            assert_eq!(
                (&shown.value, shown.mark),
                (&value, Some(Mark::Kept)),
                "{id}"
            );
        }
        let method = control(applied, "sample", "method");
        assert_eq!(
            (&method.value, method.mark),
            (&ControlValue::Choice(1), Some(Mark::Kept))
        );
        let bias = control(applied, "sample", "bias");
        assert_eq!(
            (&bias.value, bias.mark),
            (&ControlValue::Number(30.0), None)
        );
        assert_eq!(look(applied).matching, Some("grainy"));
        // One change: it renders what its bindings derive, and saves once.
        let scale = Base::new(OPERATIONS, &testing::image()).float("grain", "scale");
        let grain = [
            ("grain", "strength", Value::Float(50.0)),
            ("grain", "scale", Value::Float(2.0 * scale)),
            ("grain", "midtones_bias", Value::Float(30.0)),
        ];
        assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &["grain"], &grain));
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);

        // A control it writes, moved off the option's value, is Custom.
        let events = driven.run([set("grain", ControlValue::Number(49.0), true)]);
        assert_eq!(look(values(&events)).matching, None);
        driven.run([apply("grainy")]);
        driven.writes();
        // The option the controls already hold changes nothing, so renders and writes
        // nothing.
        let events = driven.run([apply("grainy")]);
        assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
        assert!(driven.writes().is_empty());
        // Back to the defaults, kept: the photo as first shown.
        let events = driven.run([apply("untouched")]);
        assert!(frame(&driven.gpu, &events) == untouched);
        assert_eq!(look(values(&events)).matching, Some("untouched"));
        remove(&path);
    }

    #[test]
    fn a_preset_s_option_the_photo_cannot_have_or_the_row_lacks_is_refused() {
        let path = scratch("preset-refused");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        driven.run([set("grain", ControlValue::Number(20.0), true)]);
        driven.writes();
        let other = |module: &str, preset: &str| Command::ApplyPreset {
            module: module.to_owned(),
            preset: preset.to_owned(),
            option: "grainy".to_owned(),
        };
        let events = driven.run([
            apply("camera"),
            apply("film"),
            other("sample", "tone"),
            other("exposure", "look"),
        ]);
        let failures = failures(&events);
        assert!(
            matches!(
                failures[..],
                [
                    SessionError::Edit(EditError::Unavailable {
                        option: "camera",
                        ..
                    }),
                    SessionError::Edit(EditError::UnknownOption { .. }),
                    SessionError::Edit(EditError::UnknownPreset { .. }),
                    SessionError::Edit(EditError::UnknownModule(_)),
                ]
            ),
            "{failures:?}"
        );
        // Nothing changed: no frame, no write, the values as they were.
        assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
        assert!(driven.writes().is_empty());
        let values = values(&events);
        assert_eq!(
            control(values, "sample", "grain").value,
            ControlValue::Number(20.0)
        );
        assert_eq!(look(values).matching, None);
        remove(&path);
    }

    /// The state of the coordinate `id` of [`Sample`]'s pair.
    fn coordinate<'v>(values: &'v Values, id: &str) -> &'v CoordinateState {
        values
            .modules
            .iter()
            .find(|m| m.module == "sample")
            .and_then(|m| m.views.iter().find(|c| c.coordinate == id))
            .unwrap()
    }

    /// The pair's sum and difference as the values report them: value, default and mark.
    fn pair(values: &Values) -> [(f32, f32, Option<Mark>); 2] {
        ["sum", "difference"].map(|id| {
            let shown = coordinate(values, id);
            (shown.value, shown.default, shown.mark)
        })
    }

    /// Grain and bias as the values report them, with their marks.
    fn grain_and_bias(values: &Values) -> [(ControlValue, Option<Mark>); 2] {
        ["grain", "bias"].map(|id| {
            let shown = control(values, "sample", id);
            (shown.value.clone(), shown.mark)
        })
    }

    /// The pair's coordinates for grain and bias, the module's view of them.
    fn viewed(grain_and_bias: &[(ControlValue, Option<Mark>); 2]) -> Vec<f32> {
        let mut values = Sample.defaults(&testing::image());
        for (id, (value, _)) in ["grain", "bias"].into_iter().zip(grain_and_bias) {
            values.set(id, value.clone());
        }
        Sample.view("pair", &values, &testing::image()).unwrap()
    }

    /// Grain and bias for the pair's `coordinates`, the module's inverse of its view.
    fn unviewed(coordinates: [f32; 2]) -> [ControlValue; 2] {
        let values = Sample
            .unview("pair", &coordinates, &testing::image())
            .unwrap();
        ["grain", "bias"].map(|id| values.get(id).unwrap().clone())
    }

    #[test]
    fn a_coordinate_s_drag_writes_its_view_s_controls_as_one_change_holding_the_others() {
        let path = scratch("view");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        // Untouched, grain 0 and bias 100: their sum 100, their difference −100.
        assert_eq!(
            pair(values(&opened)),
            [(100.0, 100.0, None), (-100.0, -100.0, None)]
        );

        // The difference, the sum held at 100; then a drag of the sum, the difference held.
        driven.run([set("difference", ControlValue::Number(-40.3), false)]);
        for sum in [101.7, 103.1, 104.9] {
            let events = driven.run([set("sum", ControlValue::Number(sum), false)]);
            assert_eq!(coordinate(values(&events), "difference").value, -40.3);
        }
        assert!(driven.writes().is_empty(), "nothing saved until done");
        let events = driven.run([set("sum", ControlValue::Number(110.1), true)]);
        let shown = values(&events);
        // Grain and bias are what the module's inverse gives the sum and the difference
        // held, each kept, as though the drag had been one step.
        let [grain, bias] = unviewed([110.1, -40.3]);
        let kept = Some(Mark::Kept);
        assert_eq!(grain_and_bias(shown), [(grain.clone(), kept), (bias, kept)]);
        assert_eq!(pair(shown), [(110.1, 100.0, kept), (-40.3, -100.0, kept)]);
        // The module's view of them gives the difference back only to a rounding: the
        // session's hold is what keeps it exact.
        assert_ne!(viewed(&grain_and_bias(shown))[1], -40.3);
        // One change, saved once, rendering what the binding derives.
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
        let ControlValue::Number(strength) = grain else {
            unreachable!()
        };
        let grain = [("grain", "strength", Value::Float(strength))];
        assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &["grain"], &grain));
        remove(&path);
    }

    #[test]
    fn another_change_to_a_view_s_controls_drops_its_held_coordinates() {
        let path = scratch("view-dropped");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        for other in [
            set("grain", ControlValue::Number(30.0), true),
            apply("grainy"),
            reset("bias"),
        ] {
            driven.run([
                Command::Reset,
                set("difference", ControlValue::Number(-40.3), true),
            ]);
            // A change to grain or bias but through the pair: the coordinates are the
            // module's view of the values again, ...
            let events = driven.run([other.clone()]);
            let viewed = viewed(&grain_and_bias(values(&events)));
            let shown = pair(values(&events)).map(|(value, ..)| value);
            assert_eq!(shown[..], viewed[..], "{other:?}");
            assert_ne!(shown[1], -40.3, "{other:?}");
            // ... and the next coordinate set starts from them.
            let events = driven.run([set("sum", ControlValue::Number(101.7), true)]);
            let [grain, bias] = unviewed([101.7, viewed[1]]);
            let kept = Some(Mark::Kept);
            assert_eq!(
                grain_and_bias(values(&events)),
                [(grain, kept), (bias, kept)],
                "{other:?}"
            );
        }
        // The last change saved before the folder goes.
        driven.writes();
        remove(&path);
    }

    #[test]
    fn a_coordinate_resets_holding_the_others_and_all_back_leave_the_module_untouched() {
        let path = scratch("view-reset");
        let mut driven = Driven::new(&[&Sample]);
        let opened = driven.open(&path);
        let untouched = frame(&driven.gpu, &opened);
        // Untouched, a reset changes nothing: no frame, no file.
        let events = driven.run([reset("sum"), reset("difference")]);
        assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
        assert!(driven.writes().is_empty());

        let moved = [
            set("difference", ControlValue::Number(-40.0), false),
            set("sum", ControlValue::Number(120.0), true),
        ];
        let kept = Some(Mark::Kept);
        driven.run(moved.clone());
        // The sum back to its default, the difference held: grain and bias written, kept.
        let events = driven.run([reset("sum")]);
        let shown = values(&events);
        assert_eq!(pair(shown), [(100.0, 100.0, kept), (-40.0, -100.0, kept)]);
        let [grain, bias] = unviewed([100.0, -40.0]);
        assert_eq!(grain_and_bias(shown), [(grain, kept), (bias, kept)]);
        // The difference back too: grain and bias are untouched again, values and marks.
        let events = driven.run([reset("difference")]);
        let edit = |driven: &Driven| driven.engine.open.as_ref().unwrap().edit.clone();
        assert_eq!(edit(&driven), Edit::default());
        assert!(frame(&driven.gpu, &events) == untouched);
        assert!(!values(&events).edited);
        assert_eq!(
            pair(values(&events)),
            [(100.0, 100.0, None), (-100.0, -100.0, None)]
        );

        // Both in one batch, then the module's own reset: the same.
        driven.run(moved.clone());
        let events = driven.run([reset("sum"), reset("difference")]);
        assert_eq!(edit(&driven), Edit::default());
        assert!(frame(&driven.gpu, &events) == untouched);
        driven.run(moved);
        let events = driven.run([Command::ResetModule {
            module: "sample".to_owned(),
        }]);
        assert_eq!(edit(&driven), Edit::default());
        assert_eq!(
            pair(values(&events)),
            [(100.0, 100.0, None), (-100.0, -100.0, None)]
        );
        // The last change saved before the folder goes.
        driven.writes();
        remove(&path);
    }

    #[test]
    fn a_coordinate_outside_its_range_or_putting_a_control_outside_its_own_is_refused() {
        let path = scratch("view-refused");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        // Grain 40, bias 80.
        driven.run([
            set("difference", ControlValue::Number(-40.0), false),
            set("sum", ControlValue::Number(120.0), true),
        ]);
        driven.writes();
        let events = driven.run([
            set("sum", ControlValue::Number(250.0), true),
            set("sum", ControlValue::Bool(true), true),
            // Bias at 115 %.
            set("sum", ControlValue::Number(190.0), true),
            // The difference back to its default while the sum is off its own: bias at
            // 110 %.
            reset("difference"),
            set("ratio", ControlValue::Number(1.0), true),
        ]);
        let failures = failures(&events);
        assert!(
            matches!(
                failures[..],
                [
                    SessionError::Edit(EditError::Control {
                        source: ControlError::OutOfRange { control: "sum", .. },
                        ..
                    }),
                    SessionError::Edit(EditError::Control {
                        source: ControlError::WrongKind { control: "sum", .. },
                        ..
                    }),
                    SessionError::Edit(EditError::Coordinate {
                        coordinate: "sum",
                        source: ControlError::OutOfRange {
                            control: "bias",
                            ..
                        },
                        ..
                    }),
                    SessionError::Edit(EditError::Coordinate {
                        coordinate: "difference",
                        source: ControlError::OutOfRange {
                            control: "bias",
                            ..
                        },
                        ..
                    }),
                    SessionError::Edit(EditError::UnknownControl { .. }),
                ]
            ),
            "{failures:?}"
        );
        // Nothing changed: no frame, no write, the values as they were.
        assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
        assert!(driven.writes().is_empty());
        let kept = Some(Mark::Kept);
        let shown = values(&events);
        assert_eq!(pair(shown), [(120.0, 100.0, kept), (-40.0, -100.0, kept)]);
        let [grain, bias] = unviewed([120.0, -40.0]);
        assert_eq!(grain_and_bias(shown), [(grain, kept), (bias, kept)]);
        remove(&path);
    }

    #[test]
    fn keeping_a_coordinate_keeps_its_view_s_controls() {
        let path = scratch("view-kept");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let keep = |control: &str, kept| Command::Keep {
            module: "sample".to_owned(),
            control: control.to_owned(),
            kept,
        };
        let events = driven.run([keep("sum", true)]);
        let kept = Some(Mark::Kept);
        // Held at their defaults, grain 0 and bias 100.
        assert_eq!(
            grain_and_bias(values(&events)),
            [
                (ControlValue::Number(0.0), kept),
                (ControlValue::Number(100.0), kept)
            ]
        );
        assert_eq!(
            pair(values(&events)),
            [(100.0, 100.0, kept), (-100.0, -100.0, kept)]
        );
        // A mark changes no pixel, and saves.
        assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
        assert_eq!(written(&driven.writes()).len(), 1);
        let events = driven.run([keep("difference", false)]);
        assert_eq!(pair(values(&events))[0].2, None);
        assert_eq!(driven.engine.open.as_ref().unwrap().edit, Edit::default());
        // Set to the value it has, a coordinate writes no value but keeps the controls,
        // as setting a control keeps it.
        driven.run([set("sum", ControlValue::Number(100.0), true)]);
        let values_of = driven
            .engine
            .open
            .as_ref()
            .unwrap()
            .edit
            .module("sample")
            .cloned();
        let values_of = values_of.expect("the pair's controls kept");
        assert_eq!(
            (values_of.get("grain"), values_of.mark("grain")),
            (None, kept)
        );
        assert_eq!(
            (values_of.get("bias"), values_of.mark("bias")),
            (None, kept)
        );
        // One of its controls kept, the other released: the pair's mark is none.
        let events = driven.run([keep("bias", false)]);
        assert_eq!(pair(values(&events))[1].2, None);
        // The last change saved before the folder goes.
        driven.writes();
        remove(&path);
    }

    /// A preset row whose options panic, and a lossless view whose coordinates do, as a
    /// module's bug may make them.
    struct Brittle;

    static BRITTLE: Definition = Definition {
        id: "brittle",
        version: 1,
        layers: false,
        auto: false,
        controls: &[Control {
            id: "amount",
            kind: ControlKind::Number {
                min: 0.0,
                max: 1.0,
                step: 0.01,
                default: 0.0,
            },
            unit: None,
            unported: false,
        }],
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Preset {
                    preset: "look",
                    style: ChoiceStyle::List,
                },
                Row::Slider(Slider::new("amount")),
                Row::Slider(Slider::new("percent")),
            ],
        }],
        before: &[],
        after: &[],
        presets: &[PresetRow {
            id: "look",
            options: &["full"],
            controls: &["amount"],
        }],
        views: &[View {
            id: "scaled",
            controls: &["amount"],
            coordinates: &[Coordinate {
                id: "percent",
                min: 0.0,
                max: 100.0,
                step: 1.0,
                unit: None,
            }],
        }],
    };

    impl Module for Brittle {
        fn definition(&self) -> &'static Definition {
            &BRITTLE
        }

        fn owns(&self) -> &'static [Owned] {
            &[]
        }

        fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
            Vec::new()
        }

        fn preset(&self, _: &str, _: &str, _: &ImageContext) -> Option<Controls> {
            panic!("a bug in the look");
        }

        fn view(&self, _: &str, _: &Controls, _: &ImageContext) -> Option<Vec<f32>> {
            panic!("a bug in the view");
        }
    }

    #[test]
    fn a_preset_row_whose_module_panics_reads_custom_and_refuses_its_options() {
        let path = scratch("brittle");
        let mut driven = Driven::new(&[&Sample, &Brittle]);
        let events = driven.open(&path);
        let brittle = &values(&events).modules[1];
        let state = PresetState {
            preset: "look",
            matching: None,
            available: Vec::new(),
        };
        assert_eq!(brittle.presets, [state]);
        let events = driven.run([
            Command::ApplyPreset {
                module: "brittle".to_owned(),
                preset: "look".to_owned(),
                option: "full".to_owned(),
            },
            set("grain", ControlValue::Number(20.0), true),
        ]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::PresetFailed {
                    module: "brittle",
                    preset: "look",
                    option: "full",
                    message,
                })] if message == "a bug in the look"
            ),
            "{events:?}"
        );
        // The photo stays open, and the rest of the edit renders.
        assert!(events.iter().any(|e| matches!(e, Event::Frame(_))));
        // Warned of once for the photo, though the values were reported twice.
        let open = driven.engine.open.as_ref().expect("the photo is open");
        assert_eq!(open.presets_failed, BTreeSet::from(["brittle"]));
        remove(&path);
    }

    #[test]
    fn a_view_whose_module_panics_is_left_out_and_refuses_its_coordinates() {
        let path = scratch("brittle-view");
        let mut driven = Driven::new(&[&Sample, &Brittle]);
        let events = driven.open(&path);
        let modules = &values(&events).modules;
        assert!(modules[1].views.is_empty());
        assert_eq!(modules[0].views.len(), 2, "Sample's pair still shows");
        let events = driven.run([
            Command::SetControl {
                module: "brittle".to_owned(),
                control: "percent".to_owned(),
                value: ControlValue::Number(50.0),
                done: true,
            },
            set("grain", ControlValue::Number(20.0), true),
        ]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::ViewFailed {
                    module: "brittle",
                    view: "scaled",
                    message,
                })] if message == "a bug in the view"
            ),
            "{events:?}"
        );
        // The photo stays open, the rest of the edit renders, and the warning came once.
        assert!(events.iter().any(|e| matches!(e, Event::Frame(_))));
        let open = driven.engine.open.as_ref().expect("the photo is open");
        assert_eq!(open.views_failed, BTreeSet::from(["brittle"]));
        // The last change saved before the folder goes.
        driven.writes();
        remove(&path);
    }

    /// Colour calibration's CAT tab (decision 27): choosing daylight writes the light it
    /// describes into `x` and `y`, kept, in the same change, rendered and saved once, and
    /// hue and chroma follow; the temperature it reads is the stored one.
    #[test]
    fn a_control_s_follow_writes_are_one_change_with_it() {
        use apertrust_color::illuminants::{self, Illuminant};
        use apertrust_modules::colour_calibration::ColourCalibration;
        let path = scratch("follow");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&ColourCalibration]);
        let opened = driven.open(&path);
        let untouched = frame(&driven.gpu, &opened);
        let shown = |values: &Values, id: &str| {
            let state = control(values, "channelmixerrgb", id);
            (state.value.clone(), state.mark)
        };
        let number = |values: &Values, id: &str| match shown(values, id).0 {
            ControlValue::Number(value) => value,
            other => panic!("{id}: {other:?}"),
        };
        let temperature = number(values(&opened), "temperature");
        let events = driven.run([Command::SetControl {
            module: "channelmixerrgb".to_owned(),
            control: "illuminant".to_owned(),
            value: ControlValue::Choice(2),
            done: true,
        }]);
        let daylight = values(&events);
        let [x, y] = illuminants::illuminant_to_xy(Illuminant::D, temperature, 2, 4, None).unwrap();
        assert_eq!(
            shown(daylight, "x"),
            (ControlValue::Number(x), Some(Mark::Kept))
        );
        assert_eq!(
            shown(daylight, "y"),
            (ControlValue::Number(y), Some(Mark::Kept))
        );
        // Daylight reads the temperature, which it leaves as it was.
        assert_eq!(shown(daylight, "temperature").1, None);
        assert_eq!(number(daylight, "temperature"), temperature);
        // Hue and chroma, a view of x and y, follow.
        let views = &daylight.modules[0].views;
        let viewed = ColourCalibration.view(
            "hue_chroma",
            &{
                let mut values = ColourCalibration.defaults(&testing::image());
                values.set("x", ControlValue::Number(x));
                values.set("y", ControlValue::Number(y));
                values
            },
            &testing::image(),
        );
        let reported: Vec<f32> = views.iter().map(|c| c.value).collect();
        assert_eq!(Some(reported), viewed);
        // One change: one frame, the photo's daylight, and one write.
        assert!(frame(&driven.gpu, &events) != untouched);
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
        // As shot in camera forces CAT16: the adaptation is reported unavailable while it is
        // chosen (decision 30), and available again once it is left.
        assert!(control(daylight, "channelmixerrgb", "adaptation").available);
        let choose = |option| Command::SetControl {
            module: "channelmixerrgb".to_owned(),
            control: "illuminant".to_owned(),
            value: ControlValue::Choice(option),
            done: true,
        };
        let events = driven.run([choose(8)]);
        assert!(!control(values(&events), "channelmixerrgb", "adaptation").available);
        let events = driven.run([choose(7)]);
        assert!(control(values(&events), "channelmixerrgb", "adaptation").available);
        driven.writes();
        remove(&path);
    }

    /// `testing::image`'s photo with no colour matrix, as a camera darktable lacks one for.
    fn synthetic_without_a_matrix(gpu: &GpuContext, path: &Path) -> Result<Decoded, SessionError> {
        let mut decoded = synthetic(gpu, path)?;
        if let Some(camera) = decoded.image.camera.as_mut() {
            camera.xyz_to_camera = None;
        }
        Ok(decoded)
    }

    /// White balance (decision 27): the photo carries its image context; a coefficient set
    /// writes user modified, kept, in the same change, as darktable's `gui_changed`; a
    /// Settings option writes its values as they are, with no follow writes, as
    /// `_preset_tune_callback`; and temperature and tint are unavailable on a camera with no
    /// colour matrix, the coefficients still live.
    #[test]
    fn a_white_balance_coefficient_writes_user_modified_and_a_setting_writes_none() {
        use apertrust_modules::white_balance::{self, WhiteBalance};
        let path = scratch("white-balance");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&WhiteBalance]);
        let opened = driven.open(&path);
        assert!(
            matches!(opened.first(), Some(Event::Opened(photo)) if *photo.image == testing::image())
        );
        let settings = |values: &Values| {
            let module = &values.modules[0];
            let row = module.presets.iter().find(|p| p.preset == "settings");
            (
                row.unwrap().matching,
                control(values, "temperature", "preset").value.clone(),
                control(values, "temperature", "preset").mark,
            )
        };
        let preset = |option| {
            let index = white_balance::SETTINGS.iter().position(|o| *o == option);
            ControlValue::Choice(index.unwrap() as u32)
        };
        assert_eq!(
            settings(values(&opened)),
            (Some("d65_late"), preset("d65_late"), None)
        );
        let views = &values(&opened).modules[0].views;
        assert!(
            views.len() == 2 && views.iter().all(|c| c.available),
            "{views:?}"
        );

        let events = driven.run([Command::SetControl {
            module: "temperature".to_owned(),
            control: "red".to_owned(),
            value: ControlValue::Number(2.5),
            done: true,
        }]);
        assert_eq!(
            settings(values(&events)),
            (Some("user"), preset("user"), Some(Mark::Kept))
        );
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);

        // Camera reference writes the camera's D65 coefficients, which a follow write
        // would have turned into user modified.
        let events = driven.run([Command::ApplyPreset {
            module: "temperature".to_owned(),
            preset: "settings".to_owned(),
            option: "d65".to_owned(),
        }]);
        assert_eq!(
            settings(values(&events)),
            (Some("d65"), preset("d65"), Some(Mark::Kept))
        );
        driven.writes();
        remove(&path);

        let path = scratch("white-balance-no-matrix");
        driven.engine.load = synthetic_without_a_matrix;
        let opened = driven.open(&path);
        let values = values(&opened);
        assert_eq!(settings(values), (Some("as_shot"), preset("as_shot"), None));
        let views = &values.modules[0].views;
        assert!(
            views.len() == 2 && views.iter().all(|c| !c.available),
            "{views:?}"
        );
        assert!(control(values, "temperature", "red").available);
        driven.writes();
        remove(&path);
    }

    #[test]
    fn a_follow_write_its_module_gets_wrong_refuses_the_change() {
        use apertrust_modules::testing::{Follow, Following};
        let path = scratch("follow-wrong");
        let mut driven = Driven::new(&[&Following(Follow::OutOfRange)]);
        driven.open(&path);
        driven.writes();
        let events = driven.run([set("grain", ControlValue::Number(60.0), true)]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Edit(EditError::FollowFailed {
                    module: "sample",
                    control: "grain",
                    ..
                })]
            ),
            "{events:?}"
        );
        // Nothing changed: no frame, no write, the grain as it was.
        assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
        assert!(driven.writes().is_empty());
        let grain = control(values(&events), "sample", "grain");
        assert_eq!(
            (&grain.value, grain.mark),
            (&ControlValue::Number(0.0), None)
        );
        remove(&path);
    }

    /// [`Sample`] whose grain writes its size as [`testing::Following`]'s does: follow
    /// writes, a range that clamps (coarseness up to 100 % plus the bias) and a lossless
    /// view on one module, for the tests of a gesture's cancel.
    struct Followed;

    impl Module for Followed {
        fn definition(&self) -> &'static Definition {
            Sample.definition()
        }

        fn owns(&self) -> &'static [Owned] {
            Sample.owns()
        }

        fn bind(&self, values: &ModuleValues, image: &ImageContext, base: &Base) -> Vec<Setting> {
            Sample.bind(values, image, base)
        }

        fn available(&self, control: &str, values: &Controls, image: &ImageContext) -> bool {
            Sample.available(control, values, image)
        }

        fn range(&self, control: &str, values: &Controls) -> Option<Range> {
            Sample.range(control, values)
        }

        fn options(&self, control: &str, image: &ImageContext) -> Vec<&'static str> {
            Sample.options(control, image)
        }

        fn preset(&self, preset: &str, option: &str, image: &ImageContext) -> Option<Controls> {
            Sample.preset(preset, option, image)
        }

        fn view(&self, view: &str, values: &Controls, image: &ImageContext) -> Option<Vec<f32>> {
            Sample.view(view, values, image)
        }

        fn unview(
            &self,
            view: &str,
            coordinates: &[f32],
            image: &ImageContext,
        ) -> Option<Controls> {
            Sample.unview(view, coordinates, image)
        }

        fn follow(&self, control: &str, values: &Controls, image: &ImageContext) -> Controls {
            testing::Following(testing::Follow::Size).follow(control, values, image)
        }
    }

    fn cancel() -> Command {
        Command::CancelGesture {
            module: "sample".to_owned(),
        }
    }

    /// The values as reported, but for how many commands they include.
    fn reported(values: &Values) -> Values {
        Values {
            through: 0,
            ..values.clone()
        }
    }

    /// The open photo's edit.
    fn edit_of(driven: &Driven) -> Edit {
        driven
            .engine
            .open
            .as_ref()
            .expect("a photo open")
            .edit
            .clone()
    }

    /// A slider popup's cancel (decision 42): a drag of several `done: false` changes on
    /// one module, with a change to another of its controls, follow writes and a clamp, is
    /// put back exactly, every value and mark and the coordinate the pair held, and the
    /// frame before it renders again. It is no history step and leaves nothing to write:
    /// the writer reports it saved with the cancel's number and writes nothing, neither
    /// then nor as the photo closes.
    #[test]
    fn a_cancelled_gesture_puts_its_module_back_as_it_was_and_writes_nothing() {
        let path = scratch("cancel");
        let other = scratch("cancel-other");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Followed]);
        driven.open(&path);
        // Coarseness at 150 %, then the difference at −40.3 %, the sum held at 100 %:
        // grain at 29.85 %, which makes the size fine, and bias at 70.15 %.
        driven.run([set("coarseness", ControlValue::Number(150.0), true)]);
        let events = driven.run([set("difference", ControlValue::Number(-40.3), true)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let before = values(&events).clone();
        assert_eq!(
            control(&before, "sample", "size").value,
            ControlValue::Choice(0)
        );
        let shown = frame(&driven.gpu, &events);
        let edit = edit_of(&driven);
        driven.writes();
        // Taken away, the file shows whether anything writes it again.
        std::fs::remove_file(&sidecar).unwrap();

        // Brightness, then the difference up, the sum held: grain over 50 %, which makes
        // the size coarse, and bias down to 35 %, which clamps coarseness to 135 %.
        let mut events = Vec::new();
        for step in [
            set("brightness", ControlValue::Number(0.5), false),
            set("difference", ControlValue::Number(-10.0), false),
            set("difference", ControlValue::Number(20.0), false),
            set("difference", ControlValue::Number(30.0), false),
        ] {
            events = driven.run([step]);
            assert!(failures(&events).is_empty(), "{events:?}");
        }
        let dragged = values(&events);
        let shown_as = |values: &Values, id: &str| {
            let state = control(values, "sample", id);
            (state.value.clone(), state.mark)
        };
        let kept = Some(Mark::Kept);
        assert_eq!(shown_as(dragged, "size"), (ControlValue::Choice(2), kept));
        assert_eq!(
            shown_as(dragged, "coarseness"),
            (ControlValue::Number(135.0), kept)
        );
        assert_eq!(
            shown_as(dragged, "brightness"),
            (ControlValue::Number(0.5), kept)
        );

        let events = driven.run([cancel()]);
        let through = driven.engine.applied;
        assert!(failures(&events).is_empty(), "{events:?}");
        let cancelled = values(&events);
        assert_eq!(reported(cancelled), reported(&before));
        assert_eq!(cancelled.through, through);
        // The difference as the pair held it, where the module's view of grain and bias
        // rounds it.
        assert_eq!(coordinate(cancelled, "difference").value, -40.3);
        assert_ne!(viewed(&grain_and_bias(cancelled))[1], -40.3);
        assert_eq!(edit_of(&driven), edit);
        assert!(frame(&driven.gpu, &events) == shown);
        let writes = driven.writes();
        assert!(
            matches!(&writes[..], [Event::Saved { path, through: t }] if *path == sidecar && *t == through),
            "{writes:?}"
        );
        assert!(!sidecar.exists(), "the edit was written");
        driven.open(&other);
        assert!(driven.writes().is_empty());
        assert!(
            !sidecar.exists(),
            "the edit was written as the photo closed"
        );
        remove(&path);
        remove(&other);
    }

    /// Where the batch that completed a change began the gesture too, the writer was handed
    /// its first change, whether that completed change changed the edit or not: the cancel
    /// writes the edit it puts back.
    #[test]
    fn a_cancel_writes_back_the_edit_where_the_writer_was_handed_part_of_its_gesture() {
        let path = scratch("cancel-handed");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let grain = |sidecar: &Path| {
            let read = sidecar::read(sidecar, OPERATIONS, &[&Sample], &[]).unwrap();
            read.edit.module("sample").and_then(|v| v.number("grain"))
        };
        // Grain to 30 %, then again at 30 %, which changes nothing.
        for _ in 0..2 {
            driven.run([
                set("grain", ControlValue::Number(30.0), true),
                set("grain", ControlValue::Number(60.0), false),
            ]);
            driven.writes();
            assert_eq!(grain(&sidecar), Some(60.0));
            driven.run([cancel()]);
            assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
            assert_eq!(grain(&sidecar), Some(30.0));
        }
        remove(&path);
    }

    /// With no gesture under way on the module it names, a cancel changes nothing: no
    /// frame, no write, the values as they were, and the gesture under way on another
    /// module still under way. Like every cancel (decision 42), it is reported saved with
    /// its number while the file holds the edit; while the file does not, as after a step
    /// of a gesture or an unowned parameter's `Set`, it is not, and the edit is saved when
    /// it would have been.
    #[test]
    fn a_cancel_with_no_gesture_under_way_on_its_module_changes_nothing() {
        let path = scratch("cancel-nothing");
        let other = scratch("cancel-nothing-other");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        /// Runs `command`, which changes nothing, and gives what the writer reported.
        fn nothing(driven: &mut Driven, command: Command, before: &Values) -> Vec<Event> {
            let events = driven.run([command]);
            assert!(failures(&events).is_empty(), "{events:?}");
            assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
            assert_eq!(reported(values(&events)), reported(before));
            driven.writes()
        }
        let saved = |driven: &Driven, writes: &[Event]| {
            let through = driven.engine.applied;
            matches!(writes, [Event::Saved { path, through: t }] if *path == sidecar && *t == through)
        };
        // A photo just opened, with no file beside it, then after a completed change, its
        // file taken away to show that nothing writes it again.
        let opened = driven.open(&path);
        let writes = nothing(&mut driven, cancel(), values(&opened));
        assert!(saved(&driven, &writes), "{writes:?}");
        assert!(!sidecar.exists());
        let events = driven.run([set("grain", ControlValue::Number(30.0), true)]);
        driven.writes();
        std::fs::remove_file(&sidecar).unwrap();
        let writes = nothing(&mut driven, cancel(), values(&events));
        assert!(saved(&driven, &writes), "{writes:?}");
        // Another module named, while a gesture is under way on this one.
        let events = driven.run([set("grain", ControlValue::Number(60.0), false)]);
        let elsewhere = Command::CancelGesture {
            module: "other".to_owned(),
        };
        let writes = nothing(&mut driven, elsewhere, values(&events));
        assert!(writes.is_empty(), "{writes:?}");
        let events = driven.run([cancel()]);
        assert_eq!(
            control(values(&events), "sample", "grain").value,
            ControlValue::Number(30.0)
        );
        let writes = driven.writes();
        assert!(saved(&driven, &writes), "{writes:?}");
        assert!(!sidecar.exists());
        // After a `Set`, which only a completed change or the photo's closing saves.
        let events = driven.run([Command::Set {
            operation: "exposure".to_owned(),
            param: "black".to_owned(),
            value: Value::Float(0.01),
        }]);
        let writes = nothing(&mut driven, cancel(), values(&events));
        assert!(writes.is_empty(), "{writes:?}");
        driven.open(&other);
        driven.writes();
        let read = sidecar::read(&sidecar, OPERATIONS, &[&Sample], &[]).unwrap();
        let black = read.edit.params("exposure").and_then(|p| p.get("black"));
        assert_eq!(black, Some(Value::Float(0.01)));
        remove(&path);
        remove(&other);
    }

    /// A gesture that begins with a change no write includes yet, the end of a gesture on
    /// another module or an unowned parameter's `Set`, starts unsaved: its cancel writes
    /// the edit it puts back, which the file then holds, and holds still once another
    /// photo opens.
    #[test]
    fn a_cancel_writes_back_a_change_before_its_gesture_that_no_write_included() {
        let level = Command::SetControl {
            module: "touchy".to_owned(),
            control: "level".to_owned(),
            value: ControlValue::Number(10.0),
            done: false,
        };
        let black = Command::Set {
            operation: "exposure".to_owned(),
            param: "black".to_owned(),
            value: Value::Float(0.01),
        };
        let held = |edit: &Edit| {
            let level = edit.module("touchy").and_then(|v| v.number("level"));
            let black = edit.params("exposure").and_then(|p| p.get("black"));
            let grain = edit.module("sample").and_then(|v| v.number("grain"));
            (level, black, grain)
        };
        let cases = [
            ("level", level, (Some(10.0), None, None)),
            ("black", black, (None, Some(Value::Float(0.01)), None)),
        ];
        for (name, first, expected) in cases {
            let path = scratch(&format!("cancel-unsaved-{name}"));
            let other = scratch(&format!("cancel-unsaved-{name}-other"));
            let sidecar = sidecar::path_for(&path);
            let read = || {
                let read = sidecar::read(&sidecar, OPERATIONS, &[&Sample, &Touchy], &[]);
                read.unwrap_or_else(|e| panic!("{name}: {e}")).edit
            };
            let mut driven = Driven::new(&[&Sample, &Touchy]);
            driven.open(&path);
            driven.run([first]);
            driven.run([set("grain", ControlValue::Number(20.0), false)]);
            driven.run([cancel()]);
            assert_eq!(held(&edit_of(&driven)), expected, "{name}");
            driven.writes();
            assert_eq!(held(&read()), expected, "{name}");
            driven.open(&other);
            driven.writes();
            assert_eq!(held(&read()), expected, "{name}");
            remove(&path);
            remove(&other);
        }
    }

    /// Any other change to the edit ends a gesture as completed (decision 42), so a cancel
    /// after it changes nothing, and a gesture on another module begins where the first
    /// ended; a resize and the scopes, which change no edit, leave it under way.
    #[test]
    fn another_change_ends_a_gesture_as_completed_but_a_resize_does_not() {
        let path = scratch("cancel-ended");
        let mut driven = Driven::new(&[&Sample, &Touchy]);
        driven.open(&path);
        let level = Command::SetControl {
            module: "touchy".to_owned(),
            control: "level".to_owned(),
            value: ControlValue::Number(10.0),
            done: false,
        };
        let size = |kept| Command::Keep {
            module: "sample".to_owned(),
            control: "size".to_owned(),
            kept,
        };
        let others = [
            set("grain", ControlValue::Number(40.0), true),
            reset("size"),
            size(true),
            apply("grainy"),
            Command::ResetModule {
                module: "sample".to_owned(),
            },
            Command::Set {
                operation: "exposure".to_owned(),
                param: "black".to_owned(),
                value: Value::Float(0.01),
            },
            Command::Reset,
            level,
        ];
        for other in others {
            // Brightness completed, then a gesture on grain.
            driven.run([
                Command::Reset,
                set("brightness", ControlValue::Number(1.0), true),
                set("grain", ControlValue::Number(20.0), false),
            ]);
            driven.run([other.clone()]);
            let ended = edit_of(&driven);
            let events = driven.run([cancel()]);
            assert!(failures(&events).is_empty(), "{other:?}: {events:?}");
            assert!(!events.iter().any(|e| matches!(e, Event::Frame(_))));
            assert_eq!(edit_of(&driven), ended, "{other:?}");
        }
        // The gesture on Touchy's level began with grain at 20 %, which its cancel keeps.
        driven.run([Command::CancelGesture {
            module: "touchy".to_owned(),
        }]);
        let edit = edit_of(&driven);
        let sample = edit.module("sample").unwrap();
        assert_eq!(sample.number("grain"), Some(20.0));
        assert!(edit.module("touchy").is_none_or(ModuleValues::is_empty));

        driven.run([
            Command::Reset,
            set("brightness", ControlValue::Number(1.0), true),
        ]);
        let before = edit_of(&driven);
        driven.run([
            set("grain", ControlValue::Number(20.0), false),
            Command::Resize {
                width: 60,
                height: 60,
            },
            Command::Scopes(None),
        ]);
        driven.run([cancel()]);
        assert_eq!(edit_of(&driven), before);
        driven.writes();
        remove(&path);
    }

    /// A drag of White balance's temperature, a lossless view's coordinate (decisions 2
    /// and 27), cancelled: the coefficients it moved, the user modified its follow writes
    /// made Settings, their marks and the tint it held are as before, the untouched frame
    /// renders again, and no file is written beside the untouched photo, though the writer
    /// reports the cancel saved.
    #[test]
    fn a_cancelled_drag_of_white_balance_s_temperature_leaves_the_photo_untouched() {
        use apertrust_modules::white_balance::WhiteBalance;
        let path = scratch("cancel-white-balance");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&WhiteBalance]);
        let opened = driven.open(&path);
        let untouched = frame(&driven.gpu, &opened);
        let before = values(&opened).clone();
        let coordinate = |values: &Values, id: &str| {
            let views = &values.modules[0].views;
            views.iter().find(|c| c.coordinate == id).unwrap().value
        };
        let settings = |values: &Values| {
            let module = &values.modules[0];
            let row = module.presets.iter().find(|p| p.preset == "settings");
            (
                row.unwrap().matching,
                control(values, "temperature", "preset").mark,
            )
        };
        assert_eq!(settings(&before), (Some("d65_late"), None));
        let temperature = coordinate(&before, "temperature");
        let mut events = Vec::new();
        for step in [300.0, 600.0, 900.0] {
            events = driven.run([Command::SetControl {
                module: "temperature".to_owned(),
                control: "temperature".to_owned(),
                value: ControlValue::Number(temperature - step),
                done: false,
            }]);
            assert!(failures(&events).is_empty(), "{events:?}");
        }
        let dragged = values(&events);
        assert_eq!(settings(dragged), (Some("user"), Some(Mark::Kept)));
        assert_eq!(coordinate(dragged, "tint"), coordinate(&before, "tint"));

        let events = driven.run([Command::CancelGesture {
            module: "temperature".to_owned(),
        }]);
        let through = driven.engine.applied;
        assert!(failures(&events).is_empty(), "{events:?}");
        assert_eq!(reported(values(&events)), reported(&before));
        assert_eq!(edit_of(&driven), Edit::default());
        assert!(frame(&driven.gpu, &events) == untouched);
        let writes = driven.writes();
        assert!(
            matches!(&writes[..], [Event::Saved { path, through: t }] if *path == sidecar && *t == through),
            "{writes:?}"
        );
        assert!(!sidecar.exists());
        remove(&path);
    }

    /// A module whose `available` panics while its `trigger` is on, as a module's bug may
    /// make it, so a change reaches the bug (decision 30). Its `level` defaults to the
    /// photo's width, which its preset row's one option writes, and `later` is not ported.
    struct Touchy;

    static TOUCHY: Definition = Definition {
        id: "touchy",
        version: 1,
        layers: false,
        auto: false,
        controls: &[
            Control {
                id: "level",
                kind: ControlKind::Number {
                    min: 0.0,
                    max: 1000.0,
                    step: 1.0,
                    default: 0.0,
                },
                unit: None,
                unported: false,
            },
            Control {
                id: "trigger",
                kind: ControlKind::Bool { default: false },
                unit: None,
                unported: false,
            },
            Control {
                id: "later",
                kind: ControlKind::Bool { default: false },
                unit: None,
                unported: true,
            },
        ],
        tabs: &[Tab {
            id: None,
            rows: &[
                Row::Preset {
                    preset: "look",
                    style: ChoiceStyle::List,
                },
                Row::Slider(Slider::new("level")),
                Row::Toggle { control: "trigger" },
                Row::Toggle { control: "later" },
            ],
        }],
        before: &[],
        after: &[],
        presets: &[PresetRow {
            id: "look",
            options: &["photo"],
            controls: &["level"],
        }],
        views: &[],
    };

    /// [`Touchy`]'s level for `image`: its width.
    fn photo_level(image: &ImageContext) -> Controls {
        let mut values = Controls::new();
        values.set("level", ControlValue::Number(image.width as f32));
        values
    }

    impl Module for Touchy {
        fn definition(&self) -> &'static Definition {
            &TOUCHY
        }

        fn defaults(&self, image: &ImageContext) -> Controls {
            let mut defaults = TOUCHY.defaults();
            defaults.set("level", photo_level(image).get("level").unwrap().clone());
            defaults
        }

        fn owns(&self) -> &'static [Owned] {
            &[]
        }

        fn bind(&self, _: &ModuleValues, _: &ImageContext, _: &Base) -> Vec<Setting> {
            Vec::new()
        }

        fn preset(&self, _: &str, option: &str, image: &ImageContext) -> Option<Controls> {
            (option == "photo").then(|| photo_level(image))
        }

        fn available(&self, _: &str, values: &Controls, _: &ImageContext) -> bool {
            assert!(!values.flag("trigger"), "a bug in availability");
            true
        }
    }

    #[test]
    fn an_availability_that_panics_keeps_the_photo_s_defaults_and_shows_the_controls_live() {
        let path = scratch("touchy");
        let mut driven = Driven::new(&[&Sample, &Touchy]);
        let touchy = |events: &[Event]| {
            assert!(failures(events).is_empty(), "{events:?}");
            values(events).modules[1].clone()
        };
        let untouched = touchy(&driven.open(&path));
        assert!(!untouched.edited);
        // Trigger on reaches the bug, and the values are reported twice while it is.
        let on = Command::SetControl {
            module: "touchy".to_owned(),
            control: "trigger".to_owned(),
            value: ControlValue::Bool(true),
            done: true,
        };
        let reports = [
            touchy(&driven.run([on])),
            touchy(&driven.run([set("grain", ControlValue::Number(20.0), true)])),
        ];
        for module in &reports {
            let state = |id| module.controls.iter().find(|c| c.control == id).unwrap();
            // The photo's defaults stand: level at its width, its preset's option matched,
            // and nothing but the trigger edited.
            let level = state("level");
            assert_eq!(level.value, ControlValue::Number(84.0));
            assert_eq!(level.default, ControlValue::Number(84.0));
            assert_eq!(module.presets[0].matching, Some("photo"));
            assert!(module.edited);
            let edited: Vec<&str> = module
                .controls
                .iter()
                .filter(|c| c.value != c.default)
                .map(|c| c.control)
                .collect();
            assert_eq!(edited, ["trigger"]);
            // Every control ported shows live, the one not ported does not.
            assert!(level.available && state("trigger").available);
            assert!(!state("later").available);
        }
        // Warned of once for the photo.
        let open = driven.engine.open.as_ref().expect("the photo is open");
        assert_eq!(open.available_failed, BTreeSet::from(["touchy"]));
        // Trigger off, it answers again, and nothing is edited.
        let off = Command::ResetControl {
            module: "touchy".to_owned(),
            control: "trigger".to_owned(),
        };
        let module = touchy(&driven.run([off]));
        assert_eq!(module, untouched);
        driven.writes();
        remove(&path);
    }

    #[test]
    fn set_takes_only_unowned_parameters_of_operations_an_unedited_photo_runs() {
        let path = scratch("set");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let set_param = |operation: &str, param: &str, value| Command::Set {
            operation: operation.to_owned(),
            param: param.to_owned(),
            value,
        };
        let events = driven.run([
            set_param("exposure", "exposure", Value::Float(1.0)),
            set_param("grain", "strength", Value::Float(10.0)),
            set_param("exposure", "black", Value::Float(0.01)),
        ]);
        let failures = failures(&events);
        assert!(
            matches!(
                failures[..],
                [
                    SessionError::Edit(EditError::Owned {
                        module: "sample",
                        ..
                    }),
                    SessionError::Edit(EditError::Optional { operation: "grain" }),
                ]
            ),
            "{failures:?}"
        );
        let black = ("exposure", "black", Value::Float(0.01));
        assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &[], &[black]));
        let values = values(&events);
        assert!(values.edited);
        assert!(
            values
                .settings
                .contains(&Setting::new("exposure", "black", Value::Float(0.01)))
        );
        remove(&path);
    }

    #[test]
    fn done_saves_the_edit_and_it_reads_back() {
        let path = scratch("saved");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        // A drag saves nothing until it is done, then once, saying how far it got.
        driven.run([set("grain", ControlValue::Number(10.0), false)]);
        driven.run([set("grain", ControlValue::Number(20.0), false)]);
        assert!(driven.writes().is_empty());
        let edited = driven.run([set("grain", ControlValue::Number(30.0), true)]);
        let writes = driven.writes();
        let through = driven.engine.applied;
        assert!(
            matches!(&writes[..], [Event::Saved { path, through: t }] if *path == sidecar && *t == through),
            "{writes:?}"
        );
        let xml = std::fs::read_to_string(&sidecar).unwrap();
        assert!(xml.contains(r#"control:grain="30""#), "{xml}");
        // It records the build the client handed the session.
        assert!(
            xml.contains(&format!(r#"x:xmptk="Apertrust {BUILD}""#)),
            "{xml}"
        );
        // A keep saves.
        driven.run([Command::Keep {
            module: "sample".to_owned(),
            control: "size".to_owned(),
            kept: true,
        }]);
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);

        // Opened again, the photo has its edit back, and renders as it did.
        let mut again = Driven::new(&[&Sample]);
        let events = again.open(&path);
        assert!(frame(&again.gpu, &events) == frame(&driven.gpu, &edited));
        let values = values(&events);
        let grain = control(values, "sample", "grain");
        assert_eq!(
            (&grain.value, grain.mark),
            (&ControlValue::Number(30.0), Some(Mark::Kept))
        );
        assert_eq!(control(values, "sample", "size").mark, Some(Mark::Kept));
        remove(&path);
    }

    /// An edit [`Sample`]'s brightness at 0.5 sets, as `build` writes it.
    fn brightened(build: &str) -> String {
        let mut edit = Edit::default();
        edit.set_control(
            &Sample,
            "brightness",
            ControlValue::Number(0.5),
            &testing::image(),
        )
        .unwrap();
        sidecar::to_xml(&edit, OPERATIONS, &[&Sample], build)
    }

    /// `xml` with a module entry for `lens`, which no build here has, before the others.
    fn with_lens(xml: &str) -> String {
        xml.replacen(
            "<rdf:Seq>\n",
            "<rdf:Seq>\n<rdf:li><rdf:Description apertrust:module=\"lens\" apertrust:moduleVersion=\"1\" control:amount=\"1\"/></rdf:li>\n",
            1,
        )
    }

    /// Every command that changes an edit, each refused on a photo open for viewing only.
    fn every_edit() -> Vec<Command> {
        vec![
            set("brightness", ControlValue::Number(1.0), true),
            set("grain", ControlValue::Number(20.0), false),
            reset("brightness"),
            Command::ResetModule {
                module: "sample".to_owned(),
            },
            Command::Keep {
                module: "sample".to_owned(),
                control: "size".to_owned(),
                kept: true,
            },
            apply("grainy"),
            Command::Set {
                operation: "exposure".to_owned(),
                param: "black".to_owned(),
                value: Value::Float(0.01),
            },
            Command::Reset,
            cancel(),
        ]
    }

    #[test]
    fn a_newer_apertrust_s_edit_opens_for_viewing_only_and_is_never_written() {
        let brightened_by = brightened("0.9.0");
        let cases = [
            (
                "file",
                brightened_by.replace(r#"apertrust:version="1""#, r#"apertrust:version="2""#),
                sidecar::Newer::Version(2),
            ),
            (
                "module",
                brightened_by.replace(
                    r#"apertrust:moduleVersion="1""#,
                    r#"apertrust:moduleVersion="2""#,
                ),
                sidecar::Newer::ModuleVersion {
                    module: "sample".to_owned(),
                    version: "2".to_owned(),
                    has: 1,
                },
            ),
            (
                "unreadable",
                brightened_by.replace(
                    r#"apertrust:moduleVersion="1""#,
                    r#"apertrust:moduleVersion="2.0""#,
                ),
                sidecar::Newer::ModuleVersion {
                    module: "sample".to_owned(),
                    version: "2.0".to_owned(),
                    has: 1,
                },
            ),
            (
                "unknown",
                with_lens(&brightened_by),
                sidecar::Newer::Module("lens".to_owned()),
            ),
        ];
        for (name, xml, newer) in cases {
            let path = scratch(&format!("newer-{name}"));
            let sidecar = sidecar::path_for(&path);
            std::fs::write(&sidecar, &xml).unwrap();
            let mut driven = Driven::new(&[&Sample]);
            let opened = driven.open(&path);
            let Some(Event::Opened(photo)) = opened.first() else {
                unreachable!()
            };
            assert_eq!(
                photo.view_only,
                Some(ViewOnly {
                    build: Some("0.9.0".to_owned()),
                    newer
                }),
                "{name}"
            );
            // What this build reads renders, and no control can change it.
            let exposure = ("exposure", "exposure", Value::Float(0.7 + 0.5));
            assert!(frame(&driven.gpu, &opened) == direct(&driven.gpu, &[], &[exposure]));
            let values = values(&opened);
            let brightness = control(values, "sample", "brightness");
            assert_eq!(brightness.value, ControlValue::Number(0.5), "{name}");
            let sample = &values.modules[0];
            assert!(sample.controls.iter().all(|c| !c.available), "{name}");
            assert!(look(values).available.is_empty(), "{name}");

            // Every change refused, nothing written, then nothing as the photo closes,
            // nor as the session ends.
            let events = driven.run(every_edit());
            let failures = failures(&events);
            assert_eq!(failures.len(), every_edit().len(), "{name}: {events:?}");
            assert!(
                failures
                    .iter()
                    .all(|f| matches!(f, SessionError::ViewOnly(v) if v.build.as_deref() == Some("0.9.0"))),
                "{name}: {failures:?}"
            );
            assert!(driven.writes().is_empty(), "{name}");
            assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), xml, "{name}");
            let other = scratch(&format!("newer-{name}-other"));
            driven.open(&other);
            assert!(driven.writes().is_empty(), "{name}");
            driven.open(&path);
            let Driven {
                engine,
                saved,
                inbox,
                ..
            } = driven;
            engine.finish(inbox);
            // Nothing but the end of the session's threads.
            let ended = |e: &Event| matches!(e, Event::Failed(SessionError::Stopped));
            assert!(saved.events().iter().all(ended), "{name}");
            assert_eq!(std::fs::read_to_string(&sidecar).unwrap(), xml, "{name}");
            remove(&path);
            remove(&other);
        }
    }

    #[test]
    fn a_retired_module_is_left_out_and_the_photo_stays_editable() {
        let path = scratch("retired");
        let sidecar = sidecar::path_for(&path);
        std::fs::write(&sidecar, with_lens(&brightened("0.3.0"))).unwrap();
        let mut driven = Driven::new(&[&Sample]);
        driven.engine.retired = &["lens"];
        let opened = driven.open(&path);
        let Some(Event::Opened(photo)) = opened.first() else {
            unreachable!()
        };
        assert_eq!(photo.view_only, None);
        assert!(control(values(&opened), "sample", "grain").available);
        driven.run([set("grain", ControlValue::Number(30.0), true)]);
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
        // Saved as this build writes it, without the retired module's values.
        let xml = std::fs::read_to_string(&sidecar).unwrap();
        assert!(!xml.contains("lens"), "{xml}");
        assert!(xml.contains(r#"control:brightness="0.5""#), "{xml}");
        assert!(xml.contains(BUILD), "{xml}");
        remove(&path);
    }

    #[test]
    fn a_reset_writes_and_a_change_that_changes_nothing_does_not() {
        let path = scratch("resets");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        // Resetting what is untouched leaves no file beside a photo never edited.
        driven.run([
            Command::ResetModule {
                module: "sample".to_owned(),
            },
            reset("grain"),
            Command::Reset,
        ]);
        assert!(driven.writes().is_empty());
        assert!(!sidecar.exists());

        driven.run([set("grain", ControlValue::Number(30.0), true)]);
        // The same value again, done, changes nothing, so nothing is written.
        driven.run([set("grain", ControlValue::Number(30.0), true)]);
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
        // A reset writes.
        driven.run([Command::Reset]);
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
        let xml = std::fs::read_to_string(&sidecar).unwrap();
        assert!(!xml.contains("apertrust:module="), "{xml}");
        driven.run([set("brightness", ControlValue::Number(1.0), true)]);
        driven.run([reset("brightness")]);
        assert_eq!(written(&driven.writes()).len(), 2);
        remove(&path);
    }

    #[test]
    fn a_change_never_done_is_saved_when_another_photo_opens_or_the_session_ends() {
        let first = scratch("unsaved-first");
        let second = scratch("unsaved-second");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&first);
        driven.run([set("brightness", ControlValue::Number(0.3), false)]);
        driven.open(&second);
        let writes = driven.writes();
        assert_eq!(
            written(&writes),
            [(sidecar::path_for(&first).as_path(), true)]
        );
        driven.run([Command::Set {
            operation: "exposure".to_owned(),
            param: "black".to_owned(),
            value: Value::Float(0.01),
        }]);
        assert!(driven.writes().is_empty());
        let Driven {
            engine,
            saved,
            inbox,
            ..
        } = driven;
        engine.finish(inbox);
        let writes: Vec<Event> = saved.events();
        assert_eq!(
            written(&writes),
            [(sidecar::path_for(&second).as_path(), true)]
        );
        assert!(
            std::fs::read_to_string(sidecar::path_for(&second))
                .unwrap()
                .contains(r#"param:black="0.01""#)
        );
        remove(&first);
        remove(&second);
    }

    #[test]
    fn commands_queued_when_the_client_drops_its_handle_are_saved() {
        let path = scratch("dropped").with_file_name("slow.ARW");
        let other = scratch("dropped-other");
        let gpu = gpu();
        let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
        // The slow open keeps the engine busy while the rest queues behind it and the
        // handle drops.
        session.send(Command::Open(path.clone()));
        let waiting = Instant::now();
        while !SLOW_OPEN.load(Ordering::SeqCst) {
            assert!(waiting.elapsed() < TEST_WAIT, "the open never began");
            std::thread::yield_now();
        }
        session.send(Command::Resize {
            width: 40,
            height: 40,
        });
        session.send(set("grain", ControlValue::Number(25.0), false));
        session.send(set("brightness", ControlValue::Number(0.5), true));
        // A photo opened after the drop never opens, and nothing after it applies.
        session.send(Command::Open(other.clone()));
        session.send(set("brightness", ControlValue::Number(-1.0), true));
        drop(session);
        drop(wakes);
        let saved = sidecar::read(&sidecar::path_for(&path), OPERATIONS, &[&Sample], &[])
            .expect("the edit was saved")
            .edit;
        let values = saved.module("sample").expect("the module's values");
        assert_eq!(values.number("grain"), Some(25.0));
        assert_eq!(values.number("brightness"), Some(0.5));
        assert!(!sidecar::path_for(&other).exists());
        remove(&path);
        remove(&other);
    }

    #[test]
    fn a_failed_write_is_reported_and_written_again() {
        let folder = scratch("missing").with_file_name("not yet");
        let path = folder.join("synthetic.ARW");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let events = driven.run([set("brightness", ControlValue::Number(0.4), true)]);
        let writes = driven.writes();
        assert!(
            matches!(&writes[..], [Event::Failed(SessionError::Save { through, .. })] if *through == driven.engine.applied),
            "{writes:?}"
        );
        // The edit is still there.
        assert_eq!(
            control(values(&events), "sample", "brightness").value,
            ControlValue::Number(0.4)
        );
        // Once the folder exists, closing the photo writes the edit again, changed or not.
        std::fs::create_dir_all(&folder).unwrap();
        driven.open(&scratch("missing-other"));
        assert_eq!(written(&driven.writes()), [(sidecar.as_path(), true)]);
        let saved = sidecar::read(&sidecar, OPERATIONS, &[&Sample], &[])
            .unwrap()
            .edit;
        assert_eq!(
            saved.module("sample").and_then(|v| v.number("brightness")),
            Some(0.4)
        );
        std::fs::remove_dir_all(folder.parent().unwrap()).unwrap();
        remove(&scratch("missing-other"));
    }

    /// Try again (`RetrySave`) writes the edit again after a failed write, and nothing
    /// otherwise; a gesture under way goes on.
    #[test]
    fn retry_save_writes_again_only_after_a_failed_write() {
        let folder = scratch("retry").with_file_name("not yet");
        let path = folder.join("synthetic.ARW");
        let sidecar = sidecar::path_for(&path);
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        driven.run([set("brightness", ControlValue::Number(0.4), true)]);
        assert!(matches!(
            &driven.writes()[..],
            [Event::Failed(SessionError::Save { .. })]
        ));
        // Still failing, it fails again; once the folder exists, it is written.
        driven.run([Command::RetrySave]);
        assert!(matches!(
            &driven.writes()[..],
            [Event::Failed(SessionError::Save { .. })]
        ));
        std::fs::create_dir_all(&folder).unwrap();
        driven.run([Command::RetrySave]);
        let through = driven.engine.applied;
        assert!(
            matches!(&driven.writes()[..], [Event::Saved { path, through: t }] if *path == sidecar && *t == through)
        );
        // Written, it writes nothing more, and leaves a gesture under way.
        driven.run([
            set("brightness", ControlValue::Number(0.6), false),
            Command::RetrySave,
        ]);
        assert!(driven.writes().is_empty());
        assert!(driven.engine.open.as_ref().unwrap().gesture.is_some());
        std::fs::remove_dir_all(folder.parent().unwrap()).unwrap();
    }

    /// What the writer tests write: the writer writes whatever it is given.
    const XML: &str = "<x:xmpmeta/>";

    fn job(path: &Path, through: u64, held: bool) -> Job {
        Job {
            path: path.to_owned(),
            xml: XML.to_owned(),
            through,
            held,
        }
    }

    /// An edit the file is to hold already, as a cancel hands over (decision 42), is
    /// written after the file's last write failed.
    #[test]
    fn a_held_edit_is_written_after_a_failed_write() {
        let folder = scratch("held-failed").with_file_name("not yet");
        let path = folder.join("synthetic.ARW.apxmp");
        let (sent, _saved) = outbox::channel(Arc::new(|| {}));
        let writer = Writer::start(sent);
        writer.write(path.clone(), XML.to_owned(), 1, false);
        writer.flush(None);
        assert!(writer.failed(&path));
        std::fs::create_dir_all(&folder).unwrap();
        writer.write(path.clone(), XML.to_owned(), 2, true);
        writer.flush(None);
        assert!(path.exists());
        assert!(!writer.failed(&path));
        std::fs::remove_dir_all(folder.parent().unwrap()).unwrap();
    }

    /// The writer writes only the latest of a file's edits waiting, and a held one in place
    /// of one still to write is written (decision 42). Held up in another file's write by
    /// its state, which the test holds, the writer finds both of `p`'s waiting together.
    #[test]
    fn a_held_edit_in_place_of_one_still_to_write_is_written() {
        let photo = scratch("held-merged");
        let (q, p) = (
            photo.with_file_name("q.apxmp"),
            photo.with_file_name("p.apxmp"),
        );
        let (woken, wakes) = mpsc::channel();
        let (sent, _saved) = outbox::channel(Arc::new(move || {
            let _ = woken.send(());
        }));
        let writer = Writer::start(sent);
        let jobs = writer.jobs.as_ref().expect("the writer runs");
        {
            let mut state = writer.lock();
            state.pending.insert(p.clone(), 2);
            jobs.send(Order::Write(job(&q, 1, false))).unwrap();
            // Told of q's write, the writer waits for its state.
            wakes.recv_timeout(TEST_WAIT).expect("q is written");
            jobs.send(Order::Write(job(&p, 2, false))).unwrap();
            jobs.send(Order::Write(job(&p, 3, true))).unwrap();
        }
        writer.flush(Some(&p));
        assert!(p.exists());
        remove(&photo);
    }

    #[test]
    fn a_module_whose_binding_fails_is_left_out_and_the_rest_renders() {
        let path = scratch("faulty");
        let mut driven = Driven::new(&[&Faulty, &Sample]);
        let opened = driven.open(&path);
        let untouched = frame(&driven.gpu, &opened);
        let faulty = |control: &str| Command::SetControl {
            module: "faulty".to_owned(),
            control: control.to_owned(),
            value: ControlValue::Bool(true),
            done: true,
        };
        let events = driven.run([faulty("panic")]);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Module {
                    module: "faulty",
                    source: ModuleError::Panicked(_)
                }]
            ),
            "{events:?}"
        );
        assert!(frame(&driven.gpu, &events) == untouched);
        // Reported once: the next change renders Sample's part, and says nothing more.
        let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
        assert!(failures(&events).is_empty(), "{events:?}");
        let exposure = ("exposure", "exposure", Value::Float(0.7 + 0.5));
        assert!(frame(&driven.gpu, &events) == direct(&driven.gpu, &[], &[exposure]));
        let events = driven.run([faulty("stray")]);
        assert!(failures(&events).is_empty(), "{events:?}");

        // The saved edit still opens, the module reported again, the rest rendered.
        driven.writes();
        let mut again = Driven::new(&[&Faulty, &Sample]);
        let events = again.open(&path);
        assert!(
            matches!(
                failures(&events)[..],
                [SessionError::Module {
                    module: "faulty",
                    ..
                }]
            ),
            "{events:?}"
        );
        assert!(frame(&again.gpu, &events) == direct(&again.gpu, &[], &[exposure]));
        remove(&path);
    }

    /// The commands the frame in `events` includes.
    fn frame_through(events: &[Event]) -> u64 {
        events
            .iter()
            .find_map(|e| match e {
                Event::Frame(frame) => Some(frame.through),
                _ => None,
            })
            .unwrap_or_else(|| panic!("a frame in {events:?}"))
    }

    /// The frame in `events`, its pixels read back, which waits for the GPU, and the
    /// commands it includes.
    fn frame_pixels(gpu: &GpuContext, events: &[Event]) -> (Vec<[f32; 4]>, u64) {
        let through = frame_through(events);
        (bytemuck::pod_collect_to_vec(&frame(gpu, events)), through)
    }

    fn scopes(events: &[Event]) -> Vec<&Reading> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Scopes(reading) => Some(reading),
                _ => None,
            })
            .collect()
    }

    impl Driven {
        /// The scopes' readings still on their way, picked up as the thread picks them up,
        /// looking at the GPU between commands.
        fn readings(&mut self) -> Vec<Reading> {
            let start = Instant::now();
            let mut readings = Vec::new();
            loop {
                for event in self.engine.scopes() {
                    match event {
                        Event::Scopes(reading) => readings.push(reading),
                        event => panic!("{event:?}"),
                    }
                }
                if !self.engine.reading() {
                    return readings;
                }
                assert!(start.elapsed() < TEST_WAIT, "no reading");
                std::thread::sleep(POLL_INTERVAL);
            }
        }
    }

    /// Whether `reading` counts `pixels` as the kernel does, but for luminance, whose
    /// power functions round the CPU's own way: the totals, and each colour channel's
    /// histogram where there is one.
    fn counts(reading: &Reading, pixels: &[[f32; 4]]) -> bool {
        let bin = |v: f32| (v * 256.0).clamp(0.0, 255.0) as usize;
        let mut totals = crate::scopes::Totals::default();
        let mut bins = [[0u32; 256]; 3];
        for pixel in pixels {
            totals.pixels += 1;
            totals.shadows += u32::from(pixel[..3].iter().any(|&v| v <= 0.0));
            totals.highlights += u32::from(pixel[..3].iter().any(|&v| v >= 1.0));
            for c in 0..3 {
                bins[c][bin(pixel[c])] += 1;
            }
        }
        reading.totals == totals
            && reading.histogram.as_ref().is_none_or(|histogram| {
                [&histogram.red, &histogram.green, &histogram.blue]
                    .iter()
                    .zip(&bins)
                    .all(|(read, counted)| read[..] == counted[..])
                    && histogram.luminance.iter().sum::<u32>() == totals.pixels
            })
    }

    /// Whether the reading of the frame `through` names is still to arrive, as it is when
    /// the frame was published without waiting for it.
    fn awaits(driven: &mut Driven, through: u64) -> bool {
        driven
            .engine
            .meter
            .as_mut()
            .is_some_and(|meter| meter.awaits(through))
    }

    #[test]
    fn nothing_is_counted_until_asked_then_every_frame_as_asked() {
        let path = scratch("scopes");
        let mut driven = Driven::new(&[&Sample]);
        let events = driven.open(&path);
        assert!(driven.engine.meter.is_none() && !driven.engine.reading());
        let (pixels, through) = frame_pixels(&driven.gpu, &events);

        // Asking counts the frame shown at once, rendering nothing.
        let events = driven.run([Command::Scopes(Some(Graphs::default()))]);
        assert!(events.is_empty(), "{events:?}");
        let readings = driven.readings();
        let [reading] = &readings[..] else {
            panic!("{readings:?}");
        };
        assert_eq!(
            (reading.through, reading.width, reading.height),
            (through, 40, 30)
        );
        assert!(reading.histogram.is_none() && reading.waveform.is_none());
        assert!(counts(reading, &pixels), "{reading:?}");

        // So does asking for the histogram, which the frame shown was not counted for.
        let histogram = Graphs {
            histogram: true,
            waveform: false,
            vectorscope: false,
        };
        let events = driven.run([Command::Scopes(Some(histogram))]);
        assert!(events.is_empty(), "{events:?}");
        let readings = driven.readings();
        let [reading] = &readings[..] else {
            panic!("{readings:?}");
        };
        assert_eq!(reading.through, through);
        assert!(reading.histogram.is_some() && counts(reading, &pixels));

        // From then on every frame brings it, published before its reading arrives: before
        // anything else looks at the GPU, its mapping has not completed.
        let events = driven.run([set("brightness", ControlValue::Number(0.5), true)]);
        assert!(awaits(&mut driven, frame_through(&events)));
        let (pixels, through) = frame_pixels(&driven.gpu, &events);
        let readings = driven.readings();
        assert!(
            matches!(&readings[..], [r] if r.through == through && r.histogram.is_some() && counts(r, &pixels)),
            "{readings:?}"
        );

        // Back to the totals alone, which counts nothing again; then nothing at all.
        driven.run([Command::Scopes(Some(Graphs::default()))]);
        assert!(!driven.engine.reading());
        let events = driven.run([set("brightness", ControlValue::Number(0.2), true)]);
        let (pixels, through) = frame_pixels(&driven.gpu, &events);
        let readings = driven.readings();
        assert!(
            matches!(&readings[..], [r] if r.through == through && r.histogram.is_none() && counts(r, &pixels)),
            "{readings:?}"
        );
        driven.run([Command::Scopes(None)]);
        let events = driven.run([set("brightness", ControlValue::Number(0.3), true)]);
        frame_pixels(&driven.gpu, &events);
        assert!(!driven.engine.reading());
        remove(&path);
    }

    #[test]
    fn a_drag_never_waits_for_its_counts_and_the_last_reading_arrives() {
        let path = scratch("scopes-drag");
        let mut driven = Driven::new(&[&Sample]);
        driven.run([Command::Scopes(Some(Graphs {
            histogram: true,
            waveform: true,
            vectorscope: false,
        }))]);
        let opened = driven.open(&path);
        let mut frames = vec![frame_pixels(&driven.gpu, &opened)];
        // A drag, each frame published before its reading arrives and while the readings
        // of those before it are still to be picked up: the first two take the staging
        // buffers, and the counts of the others wait in the counts buffer, each replacing
        // the last.
        for i in 1..=4 {
            let events = driven.run([set(
                "brightness",
                ControlValue::Number(0.1 * i as f32),
                false,
            )]);
            assert!(awaits(&mut driven, frame_through(&events)), "frame {i}");
            frames.push(frame_pixels(&driven.gpu, &events));
        }
        let readings = driven.readings();
        let through: Vec<u64> = readings.iter().map(|r| r.through).collect();
        let shown: Vec<u64> = frames.iter().map(|(_, through)| *through).collect();
        assert_eq!(through, [shown[0], shown[1], shown[4]]);
        for reading in &readings {
            let (pixels, _) = frames
                .iter()
                .find(|(_, through)| *through == reading.through)
                .unwrap();
            assert!(
                counts(reading, pixels) && reading.waveform.is_some(),
                "{reading:?}"
            );
        }

        // Opening another photo drops the readings of the last one's frames.
        driven.run([set("brightness", ControlValue::Number(1.0), true)]);
        let events = driven.open(&scratch("scopes-other"));
        let (_, through) = frame_pixels(&driven.gpu, &events);
        let readings = driven.readings();
        assert!(
            matches!(&readings[..], [r] if r.through == through),
            "{readings:?}"
        );
        remove(&path);
        remove(&scratch("scopes-other"));
    }

    #[test]
    fn a_failure_of_the_scopes_is_reported_once_and_the_frames_go_on() {
        let path = scratch("scopes-failed");
        let mut driven = Driven::new(&[&Sample]);
        driven.run([Command::Scopes(Some(Graphs::default()))]);
        driven.open(&path);
        driven.readings();
        // A count the GPU refuses: a frame larger than its buffer.
        let small = driven.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let mut events = Vec::new();
        driven.engine.count(&small, [40, 30], 0, &mut events);
        assert!(
            matches!(failures(&events)[..], [SessionError::Scopes(_)]),
            "{events:?}"
        );
        // The frames go on, counted no more, with nothing more reported.
        for value in [0.4, 0.6] {
            let events = driven.run([set("brightness", ControlValue::Number(value), true)]);
            assert!(failures(&events).is_empty(), "{events:?}");
            frame_pixels(&driven.gpu, &events);
            assert!(driven.engine.meter.is_none() && !driven.engine.reading());
        }
        remove(&path);
    }

    #[test]
    fn a_session_sends_a_reading_with_no_command_after_its_frame() {
        let path = scratch("scopes-session");
        let gpu = gpu();
        let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
        session.send(Command::Scopes(Some(Graphs::default())));
        session.send(Command::Open(path.clone()));
        session.send(Command::Resize {
            width: 40,
            height: 40,
        });
        // No command follows the frame: the thread looks for its reading by itself.
        let events = until(&session, &wakes, |e| matches!(e, Event::Scopes(_)));
        let Some(Event::Frame(frame)) = events.iter().find(|e| matches!(e, Event::Frame(_))) else {
            panic!("{events:?}");
        };
        let readings = scopes(&events);
        assert!(
            matches!(readings[..], [r] if r.through == frame.through && r.totals.pixels == frame.width * frame.height && r.histogram.is_none()),
            "{events:?}"
        );
        drop(session);
        remove(&path);
    }

    /// The raw.pixls.us corpus's Canon EOS 5D Mark IV sample, where
    /// `tools/fetch-raw-corpus.sh` puts it or under `APERTRUST_RAW_CORPUS`.
    fn corpus_raw() -> PathBuf {
        let corpus = std::env::var_os("APERTRUST_RAW_CORPUS").map_or_else(
            || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-images/raw.pixls.us"),
            PathBuf::from,
        );
        corpus.join("Canon/EOS 5D Mark IV/B13A0729.CR2")
    }

    /// What the viewport pipe renders for the raw at `exposure`, rendered directly.
    fn rendered(gpu: &GpuContext, path: &Path, exposure: Option<f32>) -> Vec<u8> {
        let decoded = apertrust_raw::decode_file(path).expect("the raw decodes");
        let input = raw::upload(gpu, &decoded)
            .expect("a mosaic")
            .expect("it uploads");
        let image = raw::image_context(&decoded);
        let mut edit = Edit::default();
        if let Some(ev) = exposure {
            edit.set_param(OPERATIONS, &[], "exposure", "exposure", Value::Float(ev))
                .unwrap();
        }
        let instances = edit::instances(&edit, OPERATIONS, &[], &image)
            .all()
            .unwrap();
        let pipe = Pipe::Viewport {
            width: 640,
            height: 480,
        };
        let plan = pipeline::plan(&instances, &image, input.desc(), pipe, &Limits::of(gpu))
            .expect("a plan");
        let mut pipeline = pollster::block_on(Pipeline::new(gpu, plan, input)).expect("it builds");
        pipeline.render(gpu).expect("it renders");
        bytemuck::cast_slice(&pipeline.output().download(gpu).expect("it reads back")).to_vec()
    }

    fn pixels(gpu: &GpuContext, frame: &Frame) -> Vec<u8> {
        read_buffer(gpu, &frame.buffer, frame.buffer.size()).expect("it reads back")
    }

    #[test]
    #[ignore = "needs the raw.pixls.us corpus: tools/fetch-raw-corpus.sh"]
    fn a_session_renders_what_the_pipe_does_and_coalesces_a_drag() {
        // A link to the raw in a folder of the test's own, so the edit the session saves
        // at its end is not left beside the corpus's raw, nor one there read.
        let path = scratch("corpus").with_extension("CR2");
        std::fs::hard_link(corpus_raw(), &path)
            .or_else(|_| std::fs::copy(corpus_raw(), &path).map(|_| ()))
            .expect("the corpus raw");
        let gpu = gpu();
        // No modules, so exposure's exposure is unowned and `Set` drives it, as the app's
        // spike slider does.
        let (session, wakes) = start(&gpu, &[]);
        session.send(Command::Open(path.clone()));
        session.send(Command::Resize {
            width: 640,
            height: 480,
        });
        let events = until(&session, &wakes, |e| matches!(e, Event::Frame(_)));
        let Some(Event::Opened(photo)) = events.first() else {
            panic!("{events:?}");
        };
        assert_eq!(photo.camera, "Canon EOS 5D Mark IV");
        let Some(Event::Frame(frame)) = events.last() else {
            unreachable!()
        };
        assert!(frame.width <= 640 && frame.height <= 480);
        assert!(pixels(&gpu, frame) == rendered(&gpu, &path, None));

        // A drag: every value sent before the session wakes, so far fewer renders.
        let drag: Vec<f32> = (1..=30).map(|i| 0.7 + 0.05 * i as f32).collect();
        for &ev in &drag {
            session.send(Command::Set {
                operation: "exposure".to_owned(),
                param: "exposure".to_owned(),
                value: Value::Float(ev),
            });
        }
        let last = *drag.last().expect("a drag");
        let is_last = |e: &Event| match e {
            Event::Values(values) => values.settings.iter().any(|s| {
                (s.operation, s.param, s.value) == ("exposure", "exposure", Value::Float(last))
            }),
            _ => false,
        };
        let mut events = until(&session, &wakes, is_last);
        let at = events.iter().position(is_last).expect("the last value");
        if !events[at..].iter().any(|e| matches!(e, Event::Frame(_))) {
            events.extend(until(&session, &wakes, |e| matches!(e, Event::Frame(_))));
        }
        let frames: Vec<&Frame> = events
            .iter()
            .filter_map(|e| match e {
                Event::Frame(frame) => Some(frame),
                _ => None,
            })
            .collect();
        assert!(frames.len() < drag.len(), "{} renders", frames.len());
        let Some(Event::Values(values)) = events.iter().rfind(|e| matches!(e, Event::Values(_)))
        else {
            unreachable!()
        };
        assert!(values.edited);
        // Open, Resize and the drag.
        assert_eq!(values.through, 2 + drag.len() as u64);
        let frame = frames.last().expect("a frame");
        assert!(pixels(&gpu, frame) == rendered(&gpu, &path, Some(last)));

        // A value out of range is refused, and the values that follow keep the old one.
        let refused = session.send(Command::Set {
            operation: "exposure".to_owned(),
            param: "exposure".to_owned(),
            value: Value::Float(99.0),
        });
        let events = until(&session, &wakes, |e| matches!(e, Event::Values(_)));
        assert!(matches!(
            &events[0],
            Event::Failed(SessionError::Refused { command, error })
                if *command == refused && matches!(**error, SessionError::Edit(EditError::Param { .. }))
        ));
        let Some(Event::Values(values)) = events.last() else {
            unreachable!()
        };
        assert_eq!(values.through, refused);
        assert!(is_last(&events[1]));
        // Dropping the session saves the drag, which was never done.
        drop(session);
        assert!(sidecar::path_for(&path).exists());
        remove(&path);
    }

    // The session's jobs (decision 57), their test caller a synthetic GPU job beside a
    // viewport. Nothing here races on time: a step that must hold the engine thread waits
    // for the test's permit, a body that must wait for its go waits through its context,
    // and the test waits for what a job or the session tells it.

    /// What synthetic jobs have told the test so far, waited for by what it needs.
    struct Heard {
        receiver: Receiver<Told>,
        seen: Vec<Told>,
    }

    impl Heard {
        fn start() -> (mpsc::Sender<Told>, Heard) {
            let (told, receiver) = mpsc::channel();
            let seen = Vec::new();
            (told, Heard { receiver, seen })
        }

        /// Waits until `told` has been heard.
        fn until(&mut self, told: &Told) {
            while !self.seen.contains(told) {
                let next = self.receiver.recv_timeout(TEST_WAIT).unwrap_or_else(|_| {
                    panic!("still waiting for {told:?}, heard {:?}", self.seen)
                });
                self.seen.push(next);
            }
        }

        /// Every step begun so far, in order.
        fn steps(&mut self) -> Vec<(&'static str, usize)> {
            self.seen.extend(self.receiver.try_iter());
            self.seen
                .iter()
                .filter_map(|told| match told {
                    Told::Began(label, step) => Some((*label, *step)),
                    _ => None,
                })
                .collect()
        }

        /// How many of `label`'s steps have begun so far.
        fn began(&mut self, label: &str) -> usize {
            self.steps().iter().filter(|(own, _)| *own == label).count()
        }

        /// Waits for `count` ends of `label`'s tasks, and returns them in the order told.
        fn ends(&mut self, label: &str, count: usize) -> Vec<Result<usize, TaskError>> {
            let ends = |seen: &[Told]| -> Vec<Result<usize, TaskError>> {
                seen.iter()
                    .filter_map(|told| match told {
                        Told::Ended(own, end) if *own == label => Some(end.clone()),
                        _ => None,
                    })
                    .collect()
            };
            while ends(&self.seen).len() < count {
                let next = self
                    .receiver
                    .recv_timeout(TEST_WAIT)
                    .unwrap_or_else(|_| panic!("still waiting for {label}, heard {:?}", self.seen));
                self.seen.push(next);
            }
            ends(&self.seen)
        }
    }

    fn job_of(event: &JobEvent) -> JobId {
        event.job()
    }

    /// The job the command numbered `command` started, and its events so far, in order.
    fn of_job(events: &[Event], command: u64) -> Option<(JobId, Vec<JobEvent>)> {
        let job = events.iter().find_map(|e| match e {
            Event::Job {
                event: JobEvent::Added { job, .. },
                command: Some(started),
            } if *started == command => Some(*job),
            _ => None,
        })?;
        let own = events.iter().filter_map(|e| match e {
            Event::Job { event, .. } if job_of(event) == job => Some(event.clone()),
            _ => None,
        });
        Some((job, own.collect()))
    }

    /// Reads the session's events into `events` until the job the command numbered
    /// `command` started has sent one that `last` accepts; that job and its events.
    fn follow(
        session: &Session,
        wakes: &Receiver<()>,
        events: &mut Vec<Event>,
        command: u64,
        last: impl Fn(&JobEvent) -> bool,
    ) -> (JobId, Vec<JobEvent>) {
        loop {
            if let Some((job, own)) = of_job(events, command)
                && own.iter().any(&last)
            {
                return (job, own);
            }
            wakes
                .recv_timeout(TEST_WAIT)
                .expect("the session answers in time");
            events.extend(session.events());
        }
    }

    fn ended(event: &JobEvent) -> bool {
        matches!(event, JobEvent::Ended { .. })
    }

    fn frame_after(sent: u64) -> impl Fn(&Event) -> bool {
        move |e: &Event| matches!(e, Event::Frame(frame) if frame.through >= sent)
    }

    /// A session with the synthetic photo open beside its jobs, its first frame shown.
    fn beside_a_viewport(gpu: &GpuContext, path: &Path) -> (Session, Receiver<()>) {
        let (session, wakes) = start_with(gpu, &[&Sample], synthetic);
        session.send(Command::Resize {
            width: 64,
            height: 48,
        });
        session.send(Command::Open(path.to_owned()));
        let sent = session.send(set("brightness", ControlValue::Number(0.5), false));
        until(&session, &wakes, frame_after(sent));
        (session, wakes)
    }

    #[test]
    fn a_change_to_the_live_photo_waits_for_one_step_of_a_gpu_task_at_most() {
        let gpu = gpu();
        let path = scratch("one-step");
        let (session, wakes) = beside_a_viewport(&gpu, &path);
        let (told, mut heard) = Heard::start();
        let (long, permit) = Synthetic::new("long", Queue::UserBackground, 50, &told).gated(50);
        session.send(Command::Synthetic(long));
        for (step, value) in [(1, -0.5), (2, 1.0), (3, -1.0), (4, 0.25)] {
            // The engine thread is in the step, as in a long one, when the change comes.
            heard.until(&Told::Began("long", step));
            let sent = session.send(set("brightness", ControlValue::Number(value), false));
            permit.send(()).unwrap();
            // Its frame comes with the next step held for its permit, so after this step
            // and before any other: one that waited for that step would never come.
            until(&session, &wakes, frame_after(sent));
        }
        // The rest of the steps run on their own.
        drop(permit);
        heard.until(&Told::Ended("long", Ok(50)));
        drop(session);
        remove(&path);
    }

    #[test]
    fn a_cancel_takes_effect_before_the_task_s_next_step() {
        let gpu = gpu();
        let path = scratch("cancel-job");
        let (session, wakes) = beside_a_viewport(&gpu, &path);
        let (told, mut heard) = Heard::start();
        let (job, permit) = Synthetic::new("cancelled", Queue::UserBackground, 50, &told)
            .cancellable()
            .gated(50);
        let started = session.send(Command::Synthetic(job));
        heard.until(&Told::Began("cancelled", 1));
        let mut events = Vec::new();
        let (job, _) = follow(&session, &wakes, &mut events, started, |e| {
            matches!(
                e,
                JobEvent::State {
                    state: State::Running,
                    ..
                }
            )
        });
        // Sent while the engine thread is in the first step, then that step ends.
        session.send(Command::CancelJob { job });
        permit.send(()).unwrap();
        let (_, own) = follow(&session, &wakes, &mut events, started, ended);
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Cancelled,
                    failure: None,
                    ..
                })
            ),
            "{own:?}"
        );
        heard.until(&Told::Ended("cancelled", Err(TaskError::Cancelled)));
        // The engine goes on: a resize's frame comes, which a second step, held for its
        // permit, would hold back.
        let sent = session.send(Command::Resize {
            width: 80,
            height: 60,
        });
        until(&session, &wakes, frame_after(sent));
        assert_eq!(heard.began("cancelled"), 1);
        // Cancelling a job that has ended changes nothing.
        let sent = session.send(Command::CancelJob { job });
        session.send(Command::Resize {
            width: 64,
            height: 48,
        });
        let events = until(&session, &wakes, frame_after(sent));
        assert!(failures(&events).is_empty(), "{events:?}");
        drop(permit);
        drop(session);
        remove(&path);
    }

    #[test]
    fn waiting_tasks_step_in_darktable_s_order() {
        let gpu = gpu();
        let (session, _wakes) = start_with(&gpu, &[], synthetic);
        let (told, mut heard) = Heard::start();
        let (foreground, go) = Synthetic::new("fg", Queue::UserForeground, 8, &told).held();
        let (background, permit) = Synthetic::new("bg", Queue::UserBackground, 3, &told).gated(1);
        session.send(Command::Synthetic(foreground));
        session.send(Command::Synthetic(background));
        // The background task's first step holds the engine thread while the foreground's
        // arrives, so both wait for the next turn.
        heard.until(&Told::Began("bg", 1));
        go.send(()).unwrap();
        heard.until(&Told::Handed("fg"));
        permit.send(()).unwrap();
        heard.until(&Told::Ended("fg", Ok(8)));
        heard.until(&Told::Ended("bg", Ok(3)));
        // darktable's rule, each step added to its queue as a job: a foreground head
        // starts at priority 4 and a background one at 0, which gains one each turn it
        // loses, a tie going to the earlier queue, so it wins once at 5.
        let expected = [
            ("bg", 1),
            ("fg", 1),
            ("fg", 2),
            ("fg", 3),
            ("fg", 4),
            ("fg", 5),
            ("bg", 2),
            ("fg", 6),
            ("fg", 7),
            ("fg", 8),
            ("bg", 3),
        ];
        assert_eq!(heard.steps(), expected);
    }

    /// The session keeps the latest [`RETRIES_KEPT`] retries, the oldest dropped past
    /// them, and each is taken once (decision 72).
    #[test]
    fn the_latest_64_retries_are_kept_and_each_is_taken_once() {
        // Job numbers come only from a control.
        let control = apertrust_base::jobs::Control::start(|_| {});
        let ids: Vec<JobId> = (0..=RETRIES_KEPT)
            .map(|_| {
                let job = jobs::Job::new("test", "a job to number", |_| Ok(()));
                control.add(Queue::SystemBackground, job)
            })
            .collect();
        let retries = Retries::default();
        for &job in &ids {
            retries.keep(job, Command::Reset);
        }
        assert_eq!(RETRIES_KEPT, 64);
        assert_eq!(retries.take(ids[0]), None, "the oldest is dropped");
        assert_eq!(retries.take(ids[1]), Some(Command::Reset));
        assert_eq!(retries.take(ids[1]), None, "used once");
        assert_eq!(retries.take(ids[RETRIES_KEPT]), Some(Command::Reset));
    }

    #[test]
    fn a_job_s_events_reach_the_client_in_order_its_added_naming_its_command() {
        let gpu = gpu();
        let (session, wakes) = start_with(&gpu, &[], synthetic);
        let (told, _heard) = Heard::start();
        // The first reports its whole row (decision 72), the second its title and
        // progress alone.
        let jobs = [
            ("first", Queue::UserBackground, 3, true, true),
            ("second", Queue::UserForeground, 2, false, false),
        ];
        let started = jobs.map(|(label, queue, steps, cancellable, reporting)| {
            let job = Synthetic::new(label, queue, steps, &told);
            let job = if cancellable { job.cancellable() } else { job };
            let job = if reporting { job.reporting() } else { job };
            session.send(Command::Synthetic(job))
        });
        let mut events = Vec::new();
        for ((label, queue, steps, cancellable, reporting), command) in
            jobs.into_iter().zip(started)
        {
            let (job, own) = follow(&session, &wakes, &mut events, command, ended);
            let steps = Message::new("job-synthetic-steps").with("steps", steps);
            let mut expected = vec![
                JobEvent::Added {
                    job,
                    kind: "synthetic",
                    queue: Some(queue),
                    title: Message::new("job-synthetic"),
                    cancellable,
                    destination: reporting.then(|| Destination::Folder(label.into())),
                    detail: reporting.then(|| Message::new("job-synthetic-queued")),
                    progress: jobs::Progress {
                        done: 0,
                        total: reporting.then_some(1),
                    },
                    retries: None,
                },
                JobEvent::State {
                    job,
                    state: State::Queued,
                },
                JobEvent::State {
                    job,
                    state: State::Running,
                },
            ];
            if reporting {
                expected.extend([
                    JobEvent::Detail {
                        job,
                        detail: Some(Message::new("job-synthetic-detail")),
                    },
                    JobEvent::Destination {
                        job,
                        destination: Destination::File(Path::new(label).join("done")),
                    },
                ]);
            }
            expected.extend([
                JobEvent::Title { job, title: steps },
                JobEvent::Progress {
                    job,
                    done: 1,
                    total: Some(1),
                },
                JobEvent::Ended {
                    job,
                    state: State::Finished,
                    outcome: reporting.then(|| Message::new("job-synthetic-done")),
                    failure: None,
                    at: std::time::SystemTime::UNIX_EPOCH,
                    offers: if reporting {
                        vec![Offer::Retry { items: 1 }]
                    } else {
                        Vec::new()
                    },
                },
            ]);
            // The end's time is the control's clock: compared apart.
            let mut own = own;
            if let Some(JobEvent::Ended { at, .. }) = own.last_mut() {
                *at = std::time::SystemTime::UNIX_EPOCH;
            }
            assert_eq!(own, expected);
        }
        // Only `Added` names a command.
        for event in &events {
            if let Event::Job { event, command } = event {
                assert_eq!(command.is_some(), matches!(event, JobEvent::Added { .. }));
            }
        }
    }

    /// The observer's map of the jobs shown: each from its `Added` to its `Ended`, with
    /// whether it can be cancelled, the rest of its row leaving the map as it was; so a
    /// cancel stops one that can be, is refused for one that cannot, and changes nothing
    /// once it has ended.
    #[test]
    fn the_observer_keeps_each_job_shown_from_its_added_to_its_end() {
        let (woken, wakes) = mpsc::channel();
        let (outbox, drain) = outbox::channel(Arc::new(move || {
            let _ = woken.send(());
        }));
        let (inbox, _tasks) = mpsc::channel();
        let mut jobs = Jobs::start(outbox, inbox);
        let shown = |jobs: &Jobs| -> Vec<(JobId, bool)> {
            let shown = jobs.shown.lock().unwrap_or_else(PoisonError::into_inner);
            shown
                .iter()
                .map(|(job, cancellable)| (*job, *cancellable))
                .collect()
        };
        let mut events = Vec::new();
        let mut until = |job: JobId, last: fn(&JobEvent) -> bool| {
            while !events
                .iter()
                .any(|e| matches!(e, Event::Job { event, .. } if event.job() == job && last(event)))
            {
                wakes
                    .recv_timeout(TEST_WAIT)
                    .expect("the job's events come in time");
                events.extend(drain.events());
            }
        };
        let (told, _heard) = Heard::start();
        // Each held before it hands its task, its detail and destination reported.
        let (first, _go) = Synthetic::new("first", Queue::UserBackground, 1, &told)
            .cancellable()
            .reporting()
            .held();
        let (second, go) = Synthetic::new("second", Queue::UserBackground, 1, &told)
            .reporting()
            .held();
        let one = jobs.add(1, first.queue, |tasks| first.job(tasks));
        let two = jobs.add(2, second.queue, |tasks| second.job(tasks));
        assert_eq!(shown(&jobs), [(one, true), (two, false)]);
        until(one, |e| matches!(e, JobEvent::Destination { .. }));
        assert_eq!(shown(&jobs), [(one, true), (two, false)]);

        // Cancelled as it waits, it ends and goes.
        assert!(jobs.cancel(one).is_ok());
        until(one, ended);
        assert_eq!(shown(&jobs), [(two, false)]);
        assert!(matches!(
            jobs.cancel(two),
            Err(SessionError::NotCancellable { job }) if job == two
        ));
        // Its go gone, the second ends without handing its task.
        drop(go);
        until(two, ended);
        assert_eq!(shown(&jobs), []);
        assert!(jobs.cancel(two).is_ok());
    }

    #[test]
    fn a_job_that_panics_or_fails_ends_failed_and_the_session_goes_on() {
        let gpu = gpu();
        let path = scratch("failed-job");
        let (session, wakes) = beside_a_viewport(&gpu, &path);
        let (told, mut heard) = Heard::start();
        let faults = [
            ("body", Fault::BodyPanics),
            ("panics", Fault::StepPanics(2)),
            ("fails", Fault::StepFails(1)),
            ("refused", Fault::StepInvalid(2)),
            // Its last step, which hands the body its result.
            ("refused-last", Fault::StepInvalid(3)),
        ];
        let mut events = Vec::new();
        for (label, fault) in faults {
            let job = Synthetic::new(label, Queue::UserBackground, 3, &told).fault(fault);
            let command = session.send(Command::Synthetic(job));
            let (_, own) = follow(&session, &wakes, &mut events, command, ended);
            let Some(JobEvent::Ended {
                state: State::Finished,
                failure: Some(failure),
                ..
            }) = own.last()
            else {
                panic!("{label} did not end failed: {own:?}");
            };
            let cause = match fault {
                Fault::BodyPanics => jobs::Failure::PANICKED,
                _ => "synthetic",
            };
            assert_eq!(failure.cause, cause, "{label}: {failure:?}");
        }
        // The tasks the GPU refused told their bodies why, before the jobs ended, the one
        // refused in its last step too, never its result.
        heard.steps();
        for label in ["refused", "refused-last"] {
            let refused = |told: &Told| matches!(told, Told::Ended(l, Err(TaskError::Failed(_))) if *l == label);
            assert!(heard.seen.iter().any(refused), "{label}: {:?}", heard.seen);
            let answered = |told: &Told| matches!(told, Told::Ended(l, Ok(_)) if *l == label);
            assert!(
                !heard.seen.iter().any(answered),
                "{label}: {:?}",
                heard.seen
            );
        }
        // The session goes on: a change renders, and a job runs to its end.
        let sent = session.send(set("brightness", ControlValue::Number(-0.5), false));
        until(&session, &wakes, frame_after(sent));
        let after = Synthetic::new("after", Queue::UserBackground, 3, &told);
        session.send(Command::Synthetic(after));
        heard.until(&Told::Ended("after", Ok(3)));
        drop(session);
        remove(&path);
    }

    #[test]
    fn a_session_ends_with_a_gpu_job_that_cannot_be_cancelled_in_flight() {
        let gpu = gpu();
        let path = scratch("job-at-drop");
        let (session, wakes) = beside_a_viewport(&gpu, &path);
        let (told, mut heard) = Heard::start();
        let (stubborn, go) = Synthetic::new("stubborn", Queue::UserBackground, 40, &told).held();
        let stubborn = session.send(Command::Synthetic(stubborn));
        let yielding = Synthetic::new("yielding", Queue::UserBackground, 100_000, &told);
        session.send(Command::Synthetic(yielding.cancellable()));
        heard.until(&Told::Began("yielding", 1));
        let mut events = Vec::new();
        let (job, _) = follow(&session, &wakes, &mut events, stubborn, |e| {
            matches!(
                e,
                JobEvent::State {
                    state: State::Running,
                    ..
                }
            )
        });
        // It cannot be cancelled from its progress.
        let cancel = session.send(Command::CancelJob { job });
        until(&session, &wakes, |e| {
            matches!(e, Event::Failed(SessionError::Refused { command, error })
                if *command == cancel && matches!(**error, SessionError::NotCancellable { job: refused } if refused == job))
        });

        let (ended, done) = mpsc::channel();
        std::thread::spawn(move || {
            drop(session);
            let _ = ended.send(());
        });
        // The session's end cancels the job that can be, and finishes the other, whose
        // body hands its task only now, as the control shuts down.
        heard.until(&Told::Ended("yielding", Err(TaskError::Cancelled)));
        go.send(()).unwrap();
        heard.until(&Told::Ended("stubborn", Ok(40)));
        done.recv_timeout(TEST_WAIT)
            .expect("the session ends once the job that cannot be cancelled has finished");
        remove(&path);
    }

    #[test]
    fn a_task_that_has_stepped_outlasts_the_stack_s_limit_of_thirty() {
        let gpu = gpu();
        let (session, _wakes) = start_with(&gpu, &[], synthetic);
        let (told, mut heard) = Heard::start();
        let newer = Synthetic::new("newer", Queue::SystemForeground, 1, &told).tasks(31);
        let (newer, go) = newer.held();
        let (started, permit) =
            Synthetic::new("started", Queue::SystemForeground, 2, &told).gated(1);
        session.send(Command::Synthetic(newer));
        session.send(Command::Synthetic(started));
        // The first step holds the engine thread while 31 newer tasks arrive, so the stack
        // holds the stepped task under 31 that have not stepped.
        heard.until(&Told::Began("started", 1));
        go.send(()).unwrap();
        heard.until(&Told::Handed("newer"));
        permit.send(()).unwrap();
        heard.until(&Told::Ended("started", Ok(2)));
        // Past 30 the oldest that had not stepped went, the first two handed.
        let ends = heard.ends("newer", 31);
        assert_eq!(
            ends[..2],
            [Err(TaskError::Stopped), Err(TaskError::Stopped)]
        );
        assert!(ends[2..].iter().all(|end| *end == Ok(1)), "{ends:?}");
    }

    #[test]
    fn a_panic_ending_the_jobs_still_lets_the_session_end() {
        let mut driven = Driven::new(&[]);
        let (told, mut heard) = Heard::start();
        // It cannot be cancelled, so the control's end waits for its body, which waits
        // for a task no one has stepped yet.
        let waiting = Synthetic::new("waiting", Queue::UserBackground, 3, &told);
        driven.run([Command::Synthetic(waiting)]);
        heard.until(&Told::Handed("waiting"));
        let Driven {
            mut engine, inbox, ..
        } = driven;
        engine.jobs.panic_in_finish = true;
        let (ended, done) = mpsc::channel();
        std::thread::spawn(move || {
            let panicked = catch_unwind(AssertUnwindSafe(|| engine.finish(inbox))).is_err();
            let _ = ended.send(panicked);
        });
        let panicked = done
            .recv_timeout(TEST_WAIT)
            .expect("the jobs end after the panic");
        assert!(panicked, "the panic goes on once the jobs have ended");
        heard.until(&Told::Ended("waiting", Err(TaskError::Stopped)));
    }

    /// Every event's kind, stated again by a match with no catch-all, so a new variant
    /// stops this test from building until its kind is decided (decision 59, rule 1).
    fn kind(event: &Event) -> Topic {
        match event {
            Event::Opened(_) => Topic::Notice,
            Event::Values(_) => Topic::Fact(Fact::Values),
            Event::Frame(_) => Topic::Fact(Fact::Frame),
            Event::Scopes(_) => Topic::Fact(Fact::Scopes),
            Event::Saved { .. } => Topic::Notice,
            Event::Job { event, .. } => match *event {
                JobEvent::Added { .. } | JobEvent::Ended { .. } => Topic::Notice,
                JobEvent::Progress { job, .. } => Topic::Fact(Fact::Progress(job)),
                JobEvent::Title { job, .. } => Topic::Fact(Fact::Title(job)),
                JobEvent::Detail { job, .. } => Topic::Fact(Fact::Detail(job)),
                JobEvent::Destination { job, .. } => Topic::Fact(Fact::Destination(job)),
                JobEvent::State { job, .. } => Topic::Fact(Fact::State(job)),
            },
            Event::Export { status, .. } => match status {
                ExportStatus::Started { .. }
                | ExportStatus::Done { .. }
                | ExportStatus::Finished(_) => Topic::Notice,
            },
            Event::ExportPreview { .. } => Topic::Fact(Fact::ExportPreview),
            Event::Library(event) => match event {
                LibraryEvent::Opened { .. }
                | LibraryEvent::Subfolders { .. }
                | LibraryEvent::Listed { .. }
                | LibraryEvent::Failed { .. }
                | LibraryEvent::Taken { .. }
                | LibraryEvent::Closed { .. }
                | LibraryEvent::Thumbnail { .. } => Topic::Notice,
            },
            Event::Failed(_) => Topic::Notice,
        }
    }

    #[test]
    fn every_event_states_whether_it_is_a_fact_or_a_notice() {
        let gpu = gpu();
        let path = scratch("topics");
        let (session, wakes) = start_with(&gpu, &[&Sample], synthetic);
        session.send(Command::Resize {
            width: 64,
            height: 48,
        });
        session.send(Command::Open(path.clone()));
        session.send(Command::Scopes(Some(Graphs::default())));
        let sent = session.send(set("brightness", ControlValue::Number(0.2), true));
        let mut events = until(&session, &wakes, |e| matches!(e, Event::Scopes(_)));
        let saved = |e: &Event| matches!(e, Event::Saved { through, .. } if *through >= sent);
        if !events.iter().any(saved) {
            events.extend(until(&session, &wakes, saved));
        }
        // Two jobs, whose facts are their own, each reporting its whole row.
        let (told, _heard) = Heard::start();
        let mut ids = Vec::new();
        for label in ["first", "second"] {
            let job = Synthetic::new(label, Queue::UserBackground, 2, &told).reporting();
            let command = session.send(Command::Synthetic(job));
            ids.push(follow(&session, &wakes, &mut events, command, ended).0);
        }
        drop(session);
        let one = ids[0];
        events.extend([
            Event::Export {
                job: one,
                status: ExportStatus::Started {
                    index: 0,
                    source: path.clone(),
                },
            },
            Event::Export {
                job: one,
                status: ExportStatus::Done {
                    index: 0,
                    result: PhotoResult::Failed(Failure {
                        cause: Cause::Render,
                        file: None,
                        message: "no".to_owned(),
                    }),
                },
            },
            Event::Export {
                job: one,
                status: ExportStatus::Finished(Summary::default()),
            },
            Event::ExportPreview {
                through: 1,
                preview: Box::new(ExportPreview {
                    problem: None,
                    sequence_added: false,
                    not_here: Vec::new(),
                    names: Vec::new(),
                    size: None,
                    folders: Vec::new(),
                }),
            },
            Event::Failed(SessionError::Stopped),
        ]);
        let folder = path.parent().unwrap().to_owned();
        events.extend(
            [
                LibraryEvent::Opened {
                    open: 1,
                    name: "topics".to_owned(),
                    folder,
                },
                LibraryEvent::Subfolders {
                    open: 1,
                    names: Vec::new(),
                },
                LibraryEvent::Listed {
                    open: 1,
                    photos: Vec::new(),
                    done: true,
                },
                LibraryEvent::Failed {
                    open: 1,
                    reason: "gone".to_owned(),
                },
                LibraryEvent::Taken {
                    open: 1,
                    times: Vec::new(),
                },
                LibraryEvent::Closed { open: 2 },
                LibraryEvent::Thumbnail {
                    open: 1,
                    photo: "A.ARW".to_owned(),
                    size: 256,
                    image: None,
                },
            ]
            .map(Event::Library),
        );
        let mut seen = BTreeSet::new();
        let mut jobs = BTreeSet::new();
        for event in &events {
            assert_eq!(event.topic(), kind(event), "{event:?}");
            seen.insert(format!("{:?}", std::mem::discriminant(event)));
            // `-d event` writes each with its kind, a job's with its name and job first.
            let line = outbox::line(event);
            if let Event::Job { event, .. } = event {
                jobs.insert(format!("{:?}", std::mem::discriminant(event)));
                let name = match event {
                    JobEvent::Added { .. } => "Job Added",
                    JobEvent::Progress { .. } => "Job Progress",
                    JobEvent::Title { .. } => "Job Title",
                    JobEvent::Detail { .. } => "Job Detail",
                    JobEvent::Destination { .. } => "Job Destination",
                    JobEvent::State { .. } => "Job State",
                    JobEvent::Ended { .. } => "Job Ended",
                };
                let start = format!("{name}, job {}", event.job());
                assert!(line.starts_with(&start), "{line}");
            }
            let kind = match event.topic() {
                Topic::Fact(_) => ", a fact",
                Topic::Notice => ", a notice",
            };
            assert!(line.ends_with(kind), "{line}");
        }
        // Every variant, the ones a session sent among them, and every job event.
        assert_eq!(seen.len(), 10, "{events:?}");
        assert_eq!(jobs.len(), 7, "{jobs:?}");
        // Each job's progress, title, detail, destination and state are facts of their
        // own.
        let topics: BTreeSet<Topic> = events.iter().map(Event::topic).collect();
        for job in ids {
            for fact in [
                Fact::Progress(job),
                Fact::Title(job),
                Fact::Detail(job),
                Fact::Destination(job),
                Fact::State(job),
            ] {
                assert!(topics.contains(&Topic::Fact(fact)), "{topics:?}");
            }
        }
        remove(&path);
    }

    /// Rule 5 within a batch: a refused command's failure, naming its number, comes before
    /// the values after it, and the values before the batch's frame.
    #[test]
    fn a_batch_s_refusal_comes_before_its_values_and_its_values_before_its_frame() {
        let path = scratch("batch-order");
        let mut driven = Driven::new(&[&Sample]);
        driven.open(&path);
        let first = driven.engine.applied + 1;
        let events = driven.run([
            set("nothing", ControlValue::Number(1.0), false),
            set("brightness", ControlValue::Number(0.4), false),
        ]);
        let through = driven.engine.applied;
        assert!(
            matches!(
                &events[..],
                [
                    Event::Failed(SessionError::Refused { command, error }),
                    Event::Values(values),
                    Event::Frame(frame),
                ] if *command == first
                    && matches!(**error, SessionError::Edit(EditError::UnknownControl { .. }))
                    && values.through == through
                    && frame.through == through
            ),
            "{events:?}"
        );
        remove(&path);
    }

    /// Over a drag, the values, the frames, their scopes readings and the saves each name
    /// a number that never decreases, and the last of each includes the drag's end.
    #[test]
    fn through_never_decreases_over_a_drag() {
        let gpu = gpu();
        let path = scratch("drag-through");
        let (session, wakes) = beside_a_viewport(&gpu, &path);
        session.send(Command::Scopes(Some(Graphs::default())));
        let mut last = 0;
        for step in 0..=40 {
            let value = -1.0 + 0.05 * step as f32;
            last = session.send(set("brightness", ControlValue::Number(value), step == 40));
        }
        let counted = |e: &Event| matches!(e, Event::Scopes(r) if r.through >= last);
        let saved = |e: &Event| matches!(e, Event::Saved { through, .. } if *through >= last);
        let mut events = Vec::new();
        while !(events.iter().any(counted) && events.iter().any(saved)) {
            events.extend(until(&session, &wakes, |e| counted(e) || saved(e)));
        }
        let mut seen: BTreeMap<&str, Vec<u64>> = BTreeMap::new();
        for event in &events {
            let (kind, through) = match event {
                Event::Values(values) => ("values", values.through),
                Event::Frame(frame) => ("frame", frame.through),
                Event::Scopes(reading) => ("scopes", reading.through),
                Event::Saved { through, .. } => ("saved", *through),
                _ => continue,
            };
            seen.entry(kind).or_default().push(through);
        }
        assert_eq!(seen.len(), 4, "{seen:?}");
        for (kind, throughs) in &seen {
            assert!(throughs.is_sorted(), "{kind}: {throughs:?}");
            assert_eq!(throughs.last(), Some(&last), "{kind}: {throughs:?}");
        }
        drop(session);
        remove(&path);
    }

    /// A folder of its own holding two raws and a subfolder, and a path in it, which
    /// [`remove`] takes.
    fn photo_folder(name: &str) -> (PathBuf, PathBuf) {
        let path = scratch(name);
        let folder = path.parent().unwrap().to_owned();
        for raw in ["B.ARW", "A.ARW"] {
            std::fs::write(folder.join(raw), [0u8; 4]).unwrap();
        }
        std::fs::create_dir(folder.join("Day 1")).unwrap();
        (folder, path)
    }

    fn open_folder(folder: &Path) -> Command {
        Command::Library(LibraryCommand::Open(library::Source::Folder(
            folder.to_owned(),
        )))
    }

    /// The library's events among `events` about the open numbered `open`.
    fn library_of(events: &[Event], open: u64) -> Vec<&LibraryEvent> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Library(event) if event.open() == open => Some(event),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_folder_opens_as_a_job_whose_events_name_the_open() {
        let gpu = gpu();
        let (folder, path) = photo_folder("library-open");
        let (session, wakes) = start_with(&gpu, &[], synthetic);

        let open = session.send(open_folder(&folder));
        let mut events = Vec::new();
        let (job, own) = follow(&session, &wakes, &mut events, open, ended);

        assert_eq!(
            own.first(),
            Some(&JobEvent::Added {
                job,
                kind: "folder",
                queue: Some(Queue::UserBackground),
                title: Message::new("job-folder").with("folder", folder.as_path()),
                cancellable: true,
                destination: None,
                detail: None,
                progress: jobs::Progress::default(),
                retries: None,
            })
        );
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Finished,
                    failure: None,
                    ..
                })
            ),
            "{own:?}"
        );
        // The open's events, all before its job's end, sent by the same worker.
        let library = library_of(&events, open);
        let all = events.iter().filter(|e| matches!(e, Event::Library(_)));
        assert_eq!(all.count(), library.len());
        let [
            LibraryEvent::Opened {
                name,
                folder: opened,
                ..
            },
            LibraryEvent::Subfolders { names, .. },
            LibraryEvent::Listed {
                photos, done: true, ..
            },
            LibraryEvent::Taken { times, .. },
        ] = library.as_slice()
        else {
            panic!("{library:?}");
        };
        assert_eq!(opened, &folder);
        assert_eq!(
            Some(name.as_str()),
            folder.file_name().and_then(|n| n.to_str())
        );
        assert_eq!(names, &["Day 1"]);
        let listed: Vec<&str> = photos.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(listed, ["A.ARW", "B.ARW"]);
        assert!(photos.iter().all(|p| p.edit == library::EditState::None));
        assert_eq!(times.len(), photos.len());
        // Opening a folder opens no photo.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::Opened(_) | Event::Values(_)))
        );
        drop(session);
        remove(&path);
    }

    #[test]
    fn a_second_open_cancels_the_first_and_a_close_the_second() {
        let gpu = gpu();
        let (folder, path) = photo_folder("library-again");
        let (session, wakes) = start_with(&gpu, &[], synthetic);
        let mut events = Vec::new();

        // The first reading is held as it begins, so the second open finds it running.
        let (hold, begun, permit) = export_job::testing::Hold::new();
        session.send(Command::HoldFolder(hold));
        let first = session.send(open_folder(&folder));
        begun
            .recv_timeout(TEST_WAIT)
            .expect("the first reading begins");
        let second = session.send(open_folder(&folder));
        let (_, own) = follow(&session, &wakes, &mut events, second, ended);
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Finished,
                    ..
                })
            ),
            "{own:?}"
        );
        drop(permit);
        let (_, own) = follow(&session, &wakes, &mut events, first, ended);
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Cancelled,
                    ..
                })
            ),
            "{own:?}"
        );
        assert!(library_of(&events, first).is_empty());
        let listed = library_of(&events, second);
        assert!(
            matches!(listed.last(), Some(LibraryEvent::Taken { .. })),
            "{listed:?}"
        );

        // A close answers at once, and the reading it cancels sends nothing.
        let (hold, begun, permit) = export_job::testing::Hold::new();
        session.send(Command::HoldFolder(hold));
        let third = session.send(open_folder(&folder));
        begun
            .recv_timeout(TEST_WAIT)
            .expect("the third reading begins");
        let close = session.send(Command::Library(LibraryCommand::Close));
        let closed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Closed { open }) if *open == close);
        events.extend(until(&session, &wakes, closed));
        drop(permit);
        let (_, own) = follow(&session, &wakes, &mut events, third, ended);
        assert!(
            matches!(
                own.last(),
                Some(JobEvent::Ended {
                    state: State::Cancelled,
                    ..
                })
            ),
            "{own:?}"
        );
        assert!(library_of(&events, third).is_empty());
        assert_eq!(
            library_of(&events, close),
            [&LibraryEvent::Closed { open: close }]
        );
        drop(session);
        remove(&path);
    }

    fn thumbnails(photos: &[String]) -> Command {
        Command::Library(LibraryCommand::Thumbnails {
            photos: photos.to_vec(),
            size: 256,
        })
    }

    /// The photos the thumbnails among `events` of the open numbered `open` are of, in the
    /// order they came; none has a picture, the raws being no raws.
    fn thumbnails_of(events: &[Event], open: u64) -> Vec<String> {
        library_of(events, open)
            .into_iter()
            .filter_map(|event| match event {
                LibraryEvent::Thumbnail {
                    photo, size, image, ..
                } => {
                    assert_eq!((*size, image), (256, &None), "{photo}");
                    Some(photo.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Thumbnails are darktable's: a job each on the `SystemForeground` stack, the newest
    /// asked first, one asked again while queued moving to the top and one being read not
    /// read twice; a close drops those not yet sent.
    #[test]
    fn thumbnails_are_read_newest_first_once_each_and_none_after_a_close() {
        let gpu = gpu();
        let path = scratch("library-thumbnails");
        let folder = path.parent().unwrap().to_owned();
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
        let workers = apertrust_base::jobs::workers(threads);
        let names = |prefix: &str, count: usize| -> Vec<String> {
            (0..count).map(|i| format!("{prefix}{i:02}.ARW")).collect()
        };
        let (held, queued) = (names("P", workers), names("Q", 3));
        for raw in held.iter().chain(&queued) {
            std::fs::write(folder.join(raw), [0u8; 4]).unwrap();
        }
        let (session, wakes) = start_with(&gpu, &[], synthetic);
        let mut events = Vec::new();
        let open = session.send(open_folder(&folder));
        // The reading has ended, so every worker is free.
        follow(&session, &wakes, &mut events, open, ended);

        // Every worker held by a thumbnail of its own, then three more queued, the first
        // asked on top; asked again, the last moves above them, and one being read is not
        // read again.
        let (hold, begun, permit) = export_job::testing::Hold::new();
        session.send(Command::HoldThumbnails(hold));
        session.send(thumbnails(&held));
        for _ in 0..workers {
            begun
                .recv_timeout(TEST_WAIT)
                .expect("each worker begins a thumbnail");
        }
        session.send(thumbnails(&queued));
        session.send(thumbnails(&[queued[2].clone(), held[0].clone()]));
        // A job's `Added` comes as the session adds it, so the asks before it are in. It
        // waits behind the thumbnails, of a queue of a lower priority.
        let (told, _heard) = Heard::start();
        let marker = Synthetic::new("marker", Queue::UserBackground, 1, &told);
        let marker = session.send(Command::Synthetic(marker));
        follow(&session, &wakes, &mut events, marker, |e| {
            matches!(e, JobEvent::Added { .. })
        });
        // One worker let go reads its own, then the stack from its top, alone.
        permit.send(()).unwrap();
        let a_thumbnail = |e: &Event| thumbnails_of(std::slice::from_ref(e), open).len() == 1;
        while thumbnails_of(&events, open).len() < 4 {
            events.extend(until(&session, &wakes, a_thumbnail));
        }
        let order = thumbnails_of(&events, open);
        assert!(held.contains(&order[0]), "{order:?}");
        assert_eq!(
            order[1..4],
            [&queued[2], &queued[0], &queued[1]].map(Clone::clone)
        );
        drop(permit);
        while thumbnails_of(&events, open).len() < workers + 3 {
            events.extend(until(&session, &wakes, a_thumbnail));
        }
        let mut each = thumbnails_of(&events, open);
        each.sort();
        let mut all: Vec<String> = held.iter().chain(&queued).cloned().collect();
        all.sort();
        assert_eq!(each, all, "each photo once");

        // A close drops those held and those queued: none is told.
        let (hold, begun, permit) = export_job::testing::Hold::new();
        session.send(Command::HoldThumbnails(hold));
        let reopen = session.send(open_folder(&folder));
        follow(&session, &wakes, &mut events, reopen, ended);
        let every: Vec<String> = held.iter().chain(&queued).cloned().collect();
        session.send(thumbnails(&every));
        for _ in 0..workers {
            begun
                .recv_timeout(TEST_WAIT)
                .expect("each worker begins a thumbnail");
        }
        let close = session.send(Command::Library(LibraryCommand::Close));
        let closed = |e: &Event| matches!(e, Event::Library(LibraryEvent::Closed { open }) if *open == close);
        events.extend(until(&session, &wakes, closed));
        drop(permit);
        // The bodies let go find themselves cancelled at once; given time, none has told.
        std::thread::sleep(Duration::from_millis(500));
        events.extend(session.events());
        assert!(thumbnails_of(&events, reopen).is_empty());
        // With no folder open, nothing is read.
        session.send(thumbnails(&every[..1]));
        std::thread::sleep(Duration::from_millis(200));
        events.extend(session.events());
        let told = |e: &Event| matches!(e, Event::Library(LibraryEvent::Thumbnail { .. }));
        assert_eq!(events.iter().filter(|e| told(e)).count(), workers + 3);
        drop(session);
        remove(&path);
    }
}
